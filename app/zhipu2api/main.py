import os
import json
import time
import asyncio
import logging
import secrets
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, HTTPException, Header, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse, JSONResponse
from pydantic import BaseModel
from typing import Optional, List

from core import key_pool, ADMIN_KEY, BASE_URL, MODEL_MAP, PORT, _load_json, _save_json, API_KEYS_FILE, logger

import httpx


logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(message)s")


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info(f"zhipu2api started on port {PORT}, base_url={BASE_URL}")
    yield


app = FastAPI(title="zhipu2api", version="1.0.0", lifespan=lifespan)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])


def _check_admin(authorization: str = Header(default="")):
    token = authorization.replace("Bearer ", "").strip()
    if token != ADMIN_KEY:
        raise HTTPException(401, "Unauthorized")
    return token


def _check_api_key(authorization: str = Header(default="")):
    token = authorization.replace("Bearer ", "").strip()
    keys_data = _load_json(API_KEYS_FILE, {"keys": []})
    for k in keys_data.get("keys", []):
        if k.get("key") == token:
            return token
    if token == ADMIN_KEY:
        return token
    raise HTTPException(401, "Invalid API key")


@app.get("/healthz")
async def healthz():
    total, valid = await key_pool.count()
    return {"status": "ok", "total_keys": total, "valid_keys": valid}


@app.get("/v1/models")
async def list_models():
    from core import _load_json
    static_models = [
        {"id": "glm-5.1", "name": "GLM-5.1", "owned_by": "zhipu"},
        {"id": "glm-5", "name": "GLM-5", "owned_by": "zhipu"},
        {"id": "glm-4.7", "name": "GLM-4.7", "owned_by": "zhipu"},
        {"id": "glm-4.7-flash", "name": "GLM-4.7-Flash", "owned_by": "zhipu"},
        {"id": "glm-4-plus", "name": "GLM-4-Plus", "owned_by": "zhipu"},
        {"id": "glm-4-flash", "name": "GLM-4-Flash", "owned_by": "zhipu"},
        {"id": "glm-4-air", "name": "GLM-4-Air", "owned_by": "zhipu"},
        {"id": "glm-4-long", "name": "GLM-4-Long", "owned_by": "zhipu"},
        {"id": "glm-4v", "name": "GLM-4V", "owned_by": "zhipu"},
        {"id": "glm-4v-plus", "name": "GLM-4V-Plus", "owned_by": "zhipu"},
        {"id": "glm-4", "name": "GLM-4", "owned_by": "zhipu"},
        {"id": "glm-3-turbo", "name": "GLM-3-Turbo", "owned_by": "zhipu"},
    ]
    return {"object": "list", "data": [{"id": m["id"], "object": "model", "created": int(time.time()), "owned_by": m.get("owned_by", "zhipu")} for m in static_models]}


class ChatMessage(BaseModel):
    role: str
    content: str | list | None = None
    name: Optional[str] = None
    tool_calls: Optional[list] = None
    tool_call_id: Optional[str] = None


class ChatRequest(BaseModel):
    model: str = "glm-4-flash"
    messages: List[ChatMessage]
    temperature: Optional[float] = None
    top_p: Optional[float] = None
    max_tokens: Optional[int] = None
    stream: Optional[bool] = False
    tools: Optional[list] = None
    tool_choice: Optional[str | dict] = None


@app.post("/v1/chat/completions")
async def chat_completions(req: ChatRequest, api_key: str = Depends(_check_api_key)):
    acc = await key_pool.acquire()
    if not acc:
        raise HTTPException(503, "No available API keys in pool")

    model = MODEL_MAP.get(req.model, req.model)

    zhipu_messages = []
    for msg in req.messages:
        m = {"role": msg.role}
        if isinstance(msg.content, str):
            m["content"] = msg.content
        elif isinstance(msg.content, list):
            parts = []
            for part in msg.content:
                if isinstance(part, dict):
                    if part.get("type") == "text":
                        parts.append(part["text"])
                    elif part.get("type") == "image_url":
                        url = part.get("image_url", {}).get("url", "")
                        if url:
                            parts.append({"type": "image_url", "image_url": {"url": url}})
            if any(isinstance(p, dict) for p in parts):
                m["content"] = parts
            else:
                m["content"] = "\n".join(str(p) for p in parts)
        else:
            m["content"] = str(msg.content) if msg.content is not None else ""
        zhipu_messages.append(m)

    payload = {
        "model": model,
        "messages": zhipu_messages,
    }
    if req.temperature is not None:
        payload["temperature"] = req.temperature
    if req.top_p is not None:
        payload["top_p"] = req.top_p
    if req.max_tokens is not None:
        payload["max_tokens"] = req.max_tokens
    if req.tools:
        payload["tools"] = req.tools
    if req.tool_choice:
        payload["tool_choice"] = req.tool_choice

    upstream_key = acc["api_key"]
    headers = {
        "Authorization": f"Bearer {upstream_key}",
        "Content-Type": "application/json",
    }

    if req.stream:
        return await _stream_chat(payload, headers, model, upstream_key)
    else:
        return await _normal_chat(payload, headers, model, upstream_key)


async def _normal_chat(payload, headers, model, upstream_key):
    try:
        async with httpx.AsyncClient(timeout=120) as client:
            resp = await client.post(f"{BASE_URL}/chat/completions", json=payload, headers=headers)
            if resp.status_code == 429:
                await key_pool.mark_rate_limited(upstream_key, cooldown=60)
                raise HTTPException(429, "Rate limited, switching key")
            if resp.status_code == 401:
                await key_pool.mark_invalid(upstream_key)
                raise HTTPException(401, "API key invalid")
            if resp.status_code >= 400:
                raise HTTPException(resp.status_code, f"Upstream error: {resp.text[:200]}")
            return JSONResponse(content=resp.json(), status_code=200)
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Chat error: {e}")
        raise HTTPException(500, str(e))


async def _stream_chat(payload, headers, model, upstream_key):
    payload["stream"] = True

    async def generate():
        try:
            async with httpx.AsyncClient(timeout=120) as client:
                async with client.stream("POST", f"{BASE_URL}/chat/completions", json=payload, headers=headers) as resp:
                    if resp.status_code == 429:
                        await key_pool.mark_rate_limited(upstream_key, cooldown=60)
                        yield f"data: {json.dumps({'error': 'rate_limited'})}\n\n"
                        return
                    if resp.status_code == 401:
                        await key_pool.mark_invalid(upstream_key)
                        yield f"data: {json.dumps({'error': 'unauthorized'})}\n\n"
                        return
                    if resp.status_code >= 400:
                        body = await resp.aread()
                        yield f"data: {json.dumps({'error': body.decode()[:200]})}\n\n"
                        return
                    async for line in resp.aiter_lines():
                        if line:
                            yield line + "\n\n"
                    yield "data: [DONE]\n\n"
        except Exception as e:
            logger.error(f"Stream error: {e}")
            yield f"data: {json.dumps({'error': str(e)})}\n\n"

    return StreamingResponse(generate(), media_type="text/event-stream")


@app.get("/api/admin/accounts")
async def admin_list_accounts(admin=Depends(_check_admin)):
    accounts = await key_pool.list_all()
    safe = []
    for a in accounts:
        safe.append({
            "label": a.get("label", ""),
            "api_key": a["api_key"][:6] + "..." + a["api_key"][-4:] if len(a.get("api_key", "")) > 10 else "***",
            "full_key": a["api_key"],
            "valid": a.get("valid", True),
            "status": a.get("status", "valid"),
            "rate_limited_until": a.get("rate_limited_until", 0),
            "consecutive_failures": a.get("consecutive_failures", 0),
            "total_requests": a.get("total_requests", 0),
        })
    return {"accounts": safe}


@app.post("/api/admin/accounts")
async def admin_add_account(request: Request, admin=Depends(_check_admin)):
    body = await request.json()
    api_key = body.get("api_key", "").strip()
    label = body.get("label", "").strip()
    if not api_key:
        raise HTTPException(400, "api_key required")
    added = await key_pool.add(api_key, label)
    if not added:
        raise HTTPException(409, "Key already exists")
    return {"ok": True, "message": "Key added"}


@app.delete("/api/admin/accounts/{api_key:path}")
async def admin_delete_account(api_key: str, admin=Depends(_check_admin)):
    await key_pool.remove(api_key)
    return {"ok": True}


@app.post("/api/admin/accounts/{api_key:path}/validate")
async def admin_validate_account(api_key: str, admin=Depends(_check_admin)):
    try:
        async with httpx.AsyncClient(timeout=15) as client:
            resp = await client.get(
                f"{BASE_URL}/models",
                headers={"Authorization": f"Bearer {api_key}"},
            )
            if resp.status_code == 200:
                await key_pool.mark_valid(api_key)
                return {"ok": True, "valid": True}
            else:
                await key_pool.mark_invalid(api_key)
                return {"ok": True, "valid": False, "status_code": resp.status_code}
    except Exception as e:
        await key_pool.mark_invalid(api_key)
        return {"ok": True, "valid": False, "error": str(e)}


@app.get("/api/admin/keys")
async def admin_list_keys(admin=Depends(_check_admin)):
    data = _load_json(API_KEYS_FILE, {"keys": []})
    return data


@app.post("/api/admin/keys")
async def admin_create_key(admin=Depends(_check_admin)):
    data = _load_json(API_KEYS_FILE, {"keys": []})
    new_key = f"sk-zhipu-{secrets.token_hex(16)}"
    data["keys"].append({"key": new_key, "created": time.time()})
    _save_json(API_KEYS_FILE, data)
    return {"key": new_key}


@app.post("/api/admin/register")
async def admin_register(request: Request, admin=Depends(_check_admin)):
    body = await request.json()
    api_key = body.get("api_key", "").strip()
    label = body.get("label", "").strip()
    if not api_key:
        return {"ok": False, "error": "api_key is required"}
    added = await key_pool.add(api_key, label or api_key[:8] + "...")
    if added:
        valid = False
        try:
            async with httpx.AsyncClient(timeout=15) as client:
                resp = await client.get(
                    f"{BASE_URL}/models",
                    headers={"Authorization": f"Bearer {api_key}"},
                )
                if resp.status_code == 200:
                    valid = True
                    await key_pool.mark_valid(api_key)
                else:
                    await key_pool.mark_invalid(api_key)
        except Exception:
            pass
        return {"ok": True, "valid": valid, "message": "Key added" + (" and validated" if valid else " but validation failed")}
    return {"ok": False, "error": "Key already exists"}


@app.post("/api/admin/login")
async def admin_login(request: Request, admin=Depends(_check_admin)):
    body = await request.json()
    email = body.get("email", "").strip()
    password = body.get("password", "").strip()
    region = body.get("region", "international")

    if not email or not password:
        return {"ok": False, "error": "邮箱和密码不能为空"}

    if region == "china":
        return {"ok": False, "error": "国内版需手机验证码，请直接在浏览器登录后复制 API Key 添加"}

    login_urls = [
        "https://z.ai/api/auth/login",
        "https://z.ai/api/passport/login",
        "https://z.ai/api/user/login",
        "https://z.ai/api/v1/auth/login",
    ]

    token = None
    login_error = ""

    async with httpx.AsyncClient(timeout=20, follow_redirects=True) as client:
        for login_url in login_urls:
            try:
                resp = await client.post(
                    login_url,
                    json={"email": email, "password": password},
                    headers={
                        "Content-Type": "application/json",
                        "Accept": "application/json",
                        "Accept-Language": "en-US,en",
                        "Origin": "https://z.ai",
                        "Referer": "https://z.ai/chat",
                    },
                )
                if resp.status_code in (200, 201):
                    data = resp.json()
                    token = (
                        data.get("token")
                        or data.get("access_token")
                        or data.get("data", {}).get("token")
                        or data.get("data", {}).get("access_token")
                        or ""
                    )
                    if token:
                        break
                else:
                    try:
                        detail = resp.json()
                        msg = detail.get("message", detail.get("error", ""))
                        if msg:
                            login_error = f"HTTP {resp.status_code}: {msg}"
                    except Exception:
                        login_error = f"HTTP {resp.status_code}"
            except Exception as e:
                login_error = str(e)
                continue

        if not token:
            for cookie_name in ["token", "access_token", "session_id", "z_ai_token"]:
                if cookie_name in client.cookies:
                    token = client.cookies[cookie_name]
                    break

    if not token:
        return {"ok": False, "error": f"登录失败，Z.ai 未返回有效 Token。{login_error}。请尝试在浏览器登录后手动复制 API Key。"}

    api_key = None
    async with httpx.AsyncClient(timeout=15, follow_redirects=True) as client:
        key_urls = [
            "https://z.ai/api/apikeys",
            "https://z.ai/api/passport/apikeys",
            "https://z.ai/api/v1/apikeys",
        ]
        for key_url in key_urls:
            try:
                resp = await client.post(
                    key_url,
                    json={"name": "zhipu2api-auto"},
                    headers={
                        "Authorization": f"Bearer {token}",
                        "Content-Type": "application/json",
                        "Accept": "application/json",
                        "Origin": "https://z.ai",
                        "Referer": "https://z.ai/chat",
                    },
                )
                if resp.status_code in (200, 201):
                    data = resp.json()
                    api_key = (
                        data.get("key")
                        or data.get("api_key")
                        or data.get("data", {}).get("key")
                        or data.get("data", {}).get("api_key")
                        or ""
                    )
                    if api_key:
                        break
            except Exception:
                continue

        if not api_key:
            try:
                resp = await client.get(
                    "https://z.ai/api/apikeys",
                    headers={
                        "Authorization": f"Bearer {token}",
                        "Accept": "application/json",
                        "Origin": "https://z.ai",
                        "Referer": "https://z.ai/chat",
                    },
                )
                if resp.status_code == 200:
                    data = resp.json()
                    keys = data.get("data", data.get("keys", []))
                    if isinstance(keys, list) and keys:
                        api_key = keys[-1].get("key", keys[-1].get("api_key", ""))
            except Exception:
                pass

    if api_key:
        await key_pool.add(api_key, label=email)
        return {"ok": True, "api_key": api_key, "email": email, "message": "登录成功，API Key 已自动添加到账户池"}
    else:
        return {"ok": True, "token": token, "email": email, "api_key": None, "message": "登录成功但未能自动获取 API Key，请手动在 Z.ai 网站创建 Key 后添加", "manual_key_needed": True}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="0.0.0.0", port=PORT, workers=1)
