import os
import json
import time
import asyncio
import logging
import secrets
import uuid
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


async def _check_anthropic_key(x_api_key: str = Header(default="", alias="x-api-key"),
                         authorization: str = Header(default="")):
    token = x_api_key or authorization.replace("Bearer ", "").strip()
    if not token:
        raise HTTPException(401, "Missing API key")
    keys_data = _load_json(API_KEYS_FILE, {"keys": []})
    for k in keys_data.get("keys", []):
        if k.get("key") == token:
            return token
    if token == ADMIN_KEY:
        return token
    try:
        accounts = await key_pool.list_all()
        for acc in accounts:
            if acc.get("api_key") == token:
                return token
    except Exception:
        pass
    if len(token) > 20:
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

    # 429重试逻辑
    max_retries = 3
    upstream_key = None
    headers = None
    for attempt in range(max_retries + 1):
        acc = await key_pool.acquire()
        if not acc:
            if attempt < max_retries:
                await asyncio.sleep(2 * (attempt + 1))
                continue
            raise HTTPException(503, "No available API keys in pool")

        upstream_key = acc["api_key"]
        headers = {
            "Authorization": f"Bearer {upstream_key}",
            "Content-Type": "application/json",
        }

        if req.stream:
            break  # 流式模式直接交给stream函数处理

        # 非流式：试探请求，429时切换key重试
        try:
            async with httpx.AsyncClient(timeout=120) as client:
                resp = await client.post(f"{BASE_URL}/chat/completions", json=payload, headers=headers)
                if resp.status_code == 429 and attempt < max_retries:
                    await key_pool.mark_rate_limited(upstream_key, cooldown=15)
                    logger.warning(f"429 rate limited on key ...{upstream_key[-4:]}, retry {attempt+1}/{max_retries}")
                    await asyncio.sleep(1)
                    continue
                if resp.status_code == 429:
                    await key_pool.mark_rate_limited(upstream_key, cooldown=15)
                    raise HTTPException(429, "Rate limited after retries")
                if resp.status_code == 401:
                    await key_pool.mark_invalid(upstream_key)
                    raise HTTPException(401, "API key invalid")
                if resp.status_code >= 400:
                    raise HTTPException(resp.status_code, f"Upstream error: {resp.text[:200]}")
                await key_pool.mark_valid(upstream_key)
                return JSONResponse(content=resp.json(), status_code=200)
        except HTTPException:
            raise
        except Exception as e:
            logger.error(f"Chat error: {e}")
            raise HTTPException(500, str(e))

    if req.stream:
        return await _stream_chat(payload, headers, model, upstream_key)


async def _stream_chat(payload, headers, model, upstream_key):
    payload["stream"] = True

    async def generate():
        try:
            async with httpx.AsyncClient(timeout=120) as client:
                async with client.stream("POST", f"{BASE_URL}/chat/completions", json=payload, headers=headers) as resp:
                    if resp.status_code == 429:
                        await key_pool.mark_rate_limited(upstream_key, cooldown=15)
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


def _anthropic_to_openai_messages(req_data: dict) -> list:
    messages = []
    system_text = ""
    sys = req_data.get("system")
    if sys:
        if isinstance(sys, str):
            system_text = sys
        elif isinstance(sys, list):
            parts = []
            for block in sys:
                if isinstance(block, dict) and block.get("type") == "text":
                    parts.append(block.get("text", ""))
            system_text = "\n".join(parts)

    if system_text:
        messages.append({"role": "system", "content": system_text})

    for msg in req_data.get("messages", []):
        role = msg.get("role", "user")
        content = msg.get("content")

        if role == "user":
            if isinstance(content, str):
                messages.append({"role": "user", "content": content})
            elif isinstance(content, list):
                text_parts = []
                tool_results = []
                for block in content:
                    if isinstance(block, dict):
                        if block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                        elif block.get("type") == "tool_result":
                            tool_use_id = block.get("tool_use_id", "")
                            result_content = block.get("content", "")
                            if isinstance(result_content, list):
                                result_text = "\n".join(
                                    b.get("text", "") for b in result_content if isinstance(b, dict) and b.get("type") == "text"
                                )
                            else:
                                result_text = str(result_content)
                            tool_results.append({"tool_use_id": tool_use_id, "content": result_text})
                if tool_results:
                    combined = "\n".join(text_parts) if text_parts else ""
                    for tr in tool_results:
                        combined += f"\n[Tool Result {tr['tool_use_id']}]: {tr['content']}"
                    messages.append({"role": "user", "content": combined.strip()})
                elif text_parts:
                    messages.append({"role": "user", "content": "\n".join(text_parts)})

        elif role == "assistant":
            if isinstance(content, str):
                messages.append({"role": "assistant", "content": content})
            elif isinstance(content, list):
                text_parts = []
                tool_calls = []
                for block in content:
                    if isinstance(block, dict):
                        if block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                        elif block.get("type") == "tool_use":
                            tool_calls.append({
                                "id": block.get("id", f"call_{uuid.uuid4().hex[:8]}"),
                                "type": "function",
                                "function": {
                                    "name": block.get("name", ""),
                                    "arguments": json.dumps(block.get("input", {}), ensure_ascii=False),
                                },
                            })
                        elif block.get("type") == "thinking":
                            pass
                msg_dict = {"role": "assistant"}
                if text_parts:
                    msg_dict["content"] = "\n".join(text_parts)
                else:
                    msg_dict["content"] = None
                if tool_calls:
                    msg_dict["tool_calls"] = tool_calls
                messages.append(msg_dict)

        else:
            if isinstance(content, str):
                messages.append({"role": role, "content": content})
            elif isinstance(content, list):
                text_parts = []
                for block in content:
                    if isinstance(block, dict) and block.get("type") == "text":
                        text_parts.append(block.get("text", ""))
                messages.append({"role": role, "content": "\n".join(text_parts) if text_parts else ""})

    return messages


def _anthropic_tools_to_openai(tools: list) -> list:
    if not tools:
        return None
    openai_tools = []
    for tool in tools:
        if tool.get("type") == "custom" or tool.get("name"):
            openai_tools.append({
                "type": "function",
                "function": {
                    "name": tool.get("name", ""),
                    "description": tool.get("description", ""),
                    "parameters": tool.get("input_schema", tool.get("parameters", {"type": "object", "properties": {}})),
                },
            })
    return openai_tools if openai_tools else None


def _sse(event_type: str, data: dict) -> str:
    return f"event: {event_type}\ndata: {json.dumps(data, ensure_ascii=False)}\n\n"


@app.post("/anthropic/v1/messages")
@app.post("/v1/messages")
async def anthropic_messages(request: Request, api_key: str = Depends(_check_anthropic_key)):
    try:
        req_data = await request.json()
    except Exception:
        raise HTTPException(400, {"error": {"message": "Invalid JSON body", "type": "invalid_request_error"}})

    requested_model = req_data.get("model", "claude-3-5-sonnet-20241022")
    model = MODEL_MAP.get(requested_model, requested_model)

    openai_messages = _anthropic_to_openai_messages(req_data)
    openai_tools = _anthropic_tools_to_openai(req_data.get("tools", []))

    payload = {
        "model": model,
        "messages": openai_messages,
    }
    if req_data.get("max_tokens"):
        payload["max_tokens"] = req_data["max_tokens"]
    if req_data.get("temperature") is not None:
        payload["temperature"] = req_data["temperature"]
    if req_data.get("top_p") is not None:
        payload["top_p"] = req_data["top_p"]
    if openai_tools:
        payload["tools"] = openai_tools
    if req_data.get("tool_choice"):
        tc = req_data["tool_choice"]
        if isinstance(tc, dict) and tc.get("type") == "auto":
            payload["tool_choice"] = "auto"
        elif isinstance(tc, dict) and tc.get("type") == "any":
            payload["tool_choice"] = "auto"
        elif isinstance(tc, str):
            payload["tool_choice"] = tc

    msg_id = f"msg_{uuid.uuid4().hex[:24]}"
    is_stream = req_data.get("stream", False)

    # 429重试逻辑：尝试获取可用key，遇到429切换key重试
    max_retries = 3
    for attempt in range(max_retries + 1):
        acc = await key_pool.acquire()
        if acc:
            upstream_key = acc["api_key"]
        else:
            # key pool空，等待最短冷却期后重试
            if attempt < max_retries:
                await asyncio.sleep(2 * (attempt + 1))
                continue
            upstream_key = api_key

        headers = {
            "Authorization": f"Bearer {upstream_key}",
            "Content-Type": "application/json",
        }

        # 非流式：先试探请求，429时切换key重试
        if not is_stream:
            try:
                async with httpx.AsyncClient(timeout=120) as client:
                    resp = await client.post(f"{BASE_URL}/chat/completions", json=payload, headers=headers)
                    if resp.status_code == 429 and attempt < max_retries:
                        await key_pool.mark_rate_limited(upstream_key, cooldown=15)
                        logger.warning(f"429 rate limited on key ...{upstream_key[-4:]}, retry {attempt+1}/{max_retries}")
                        await asyncio.sleep(1)
                        continue
                    break
            except Exception as e:
                if attempt < max_retries:
                    logger.warning(f"Request error: {e}, retry {attempt+1}/{max_retries}")
                    await asyncio.sleep(1)
                    continue
                raise HTTPException(500, str(e))
        else:
            break  # 流式模式直接交给stream函数处理

    if is_stream:
        return await _anthropic_stream(payload, headers, model, upstream_key, msg_id, requested_model)
    else:
        if resp.status_code == 429:
            await key_pool.mark_rate_limited(upstream_key, cooldown=15)
            return JSONResponse(
                status_code=429,
                content={"type": "error", "error": {"type": "rate_limit_error", "message": "Rate limited after retries"}},
            )
        if resp.status_code == 401:
            await key_pool.mark_invalid(upstream_key)
            return JSONResponse(
                status_code=401,
                content={"type": "error", "error": {"type": "authentication_error", "message": "API key invalid"}},
            )
        if resp.status_code >= 400:
            err_text = resp.text[:500]
            return JSONResponse(
                status_code=resp.status_code,
                content={"type": "error", "error": {"type": "api_error", "message": f"Upstream error: {err_text}"}},
            )
        # 成功时重置key的失败计数
        await key_pool.mark_valid(upstream_key)
        return _anthropic_format_response(resp.json(), msg_id, requested_model)


def _anthropic_format_response(data, msg_id, requested_model):
    """将OpenAI格式的响应转换为Anthropic格式"""
    choice = data.get("choices", [{}])[0]
    message = choice.get("message", {})
    finish_reason = choice.get("finish_reason", "stop")

    content_blocks = []
    reasoning = message.get("reasoning_content", "")
    if reasoning:
        content_blocks.append({"type": "text", "text": reasoning})
    if message.get("content"):
        content_blocks.append({"type": "text", "text": message["content"]})

    if message.get("tool_calls"):
        for tc in message["tool_calls"]:
            func = tc.get("function", {})
            try:
                input_data = json.loads(func.get("arguments", "{}"))
            except Exception:
                input_data = {}
            content_blocks.append({
                "type": "tool_use",
                "id": tc.get("id", f"toolu_{uuid.uuid4().hex[:24]}"),
                "name": func.get("name", ""),
                "input": input_data,
            })

    if not content_blocks:
        content_blocks.append({"type": "text", "text": ""})

    stop_reason = "end_turn"
    if finish_reason == "tool_calls" or (message.get("tool_calls") and len(message["tool_calls"]) > 0):
        stop_reason = "tool_use"
    elif finish_reason == "length":
        stop_reason = "max_tokens"

    usage_data = data.get("usage", {})
    return JSONResponse(content={
        "id": msg_id,
        "type": "message",
        "role": "assistant",
        "model": requested_model,
        "content": content_blocks,
        "stop_reason": stop_reason,
        "stop_sequence": None,
        "usage": {
            "input_tokens": usage_data.get("prompt_tokens", 0),
            "output_tokens": usage_data.get("completion_tokens", 0),
        },
    })


async def _anthropic_stream(payload, headers, model, upstream_key, msg_id, requested_model):
    payload["stream"] = True

    async def generate():
        sent_message_start = False
        current_block_index = -1
        current_block_type = None
        open_tool_ids = set()
        total_output_tokens = 0
        has_content = False

        try:
            async with httpx.AsyncClient(timeout=180) as client:
                async with client.stream("POST", f"{BASE_URL}/chat/completions", json=payload, headers=headers) as resp:
                    if resp.status_code == 429:
                        await key_pool.mark_rate_limited(upstream_key, cooldown=15)
                        yield _sse("error", {"type": "error", "error": {"type": "rate_limit_error", "message": "Rate limited"}})
                        return
                    if resp.status_code == 401:
                        await key_pool.mark_invalid(upstream_key)
                        yield _sse("error", {"type": "error", "error": {"type": "authentication_error", "message": "API key invalid"}})
                        return
                    if resp.status_code >= 400:
                        body = await resp.aread()
                        yield _sse("error", {"type": "error", "error": {"type": "api_error", "message": f"Upstream error: {body.decode()[:200]}"}})
                        return

                    async for line in resp.aiter_lines():
                        if not line:
                            continue
                        if line.startswith("data: "):
                            data_str = line[6:]
                        elif line.startswith("data:"):
                            data_str = line[5:]
                        else:
                            continue

                        data_str = data_str.strip()
                        if data_str == "[DONE]":
                            break

                        try:
                            chunk = json.loads(data_str)
                        except Exception:
                            continue

                        choices = chunk.get("choices", [])
                        if not choices:
                            continue
                        delta = choices[0].get("delta", {})
                        finish_reason = choices[0].get("finish_reason")

                        if not sent_message_start:
                            usage_data = chunk.get("usage", {})
                            yield _sse("message_start", {
                                "type": "message_start",
                                "message": {
                                    "id": msg_id,
                                    "type": "message",
                                    "role": "assistant",
                                    "model": requested_model,
                                    "content": [],
                                    "stop_reason": None,
                                    "stop_sequence": None,
                                    "usage": {
                                        "input_tokens": usage_data.get("prompt_tokens", 0),
                                        "output_tokens": 0,
                                    },
                                },
                            })
                            sent_message_start = True

                        for content_field in ["reasoning_content", "content"]:
                            if delta.get(content_field) is not None:
                                text = delta[content_field]
                                if text:
                                    has_content = True
                                    if current_block_type != "text":
                                        if current_block_type is not None:
                                            yield _sse("content_block_stop", {"type": "content_block_stop", "index": current_block_index})
                                        current_block_index += 1
                                        current_block_type = "text"
                                        yield _sse("content_block_start", {
                                            "type": "content_block_start",
                                            "index": current_block_index,
                                            "content_block": {"type": "text", "text": ""},
                                        })
                                    total_output_tokens += 1
                                    yield _sse("content_block_delta", {
                                        "type": "content_block_delta",
                                        "index": current_block_index,
                                        "delta": {"type": "text_delta", "text": text},
                                    })

                        if delta.get("tool_calls"):
                            has_content = True
                            for tc in delta["tool_calls"]:
                                tc_id = tc.get("id", "")
                                tc_index = tc.get("index", 0)
                                func = tc.get("function", {})
                                tool_name = func.get("name", "")
                                partial_args = func.get("arguments", "")

                                if tc_id and tc_id not in open_tool_ids:
                                    if current_block_type is not None:
                                        yield _sse("content_block_stop", {"type": "content_block_stop", "index": current_block_index})
                                    current_block_index += 1
                                    current_block_type = "tool_use"
                                    open_tool_ids.add(tc_id)
                                    yield _sse("content_block_start", {
                                        "type": "content_block_start",
                                        "index": current_block_index,
                                        "content_block": {
                                            "type": "tool_use",
                                            "id": tc_id,
                                            "name": tool_name,
                                            "input": {},
                                        },
                                    })

                                if partial_args:
                                    yield _sse("content_block_delta", {
                                        "type": "content_block_delta",
                                        "index": current_block_index,
                                        "delta": {"type": "input_json_delta", "partial_json": partial_args},
                                    })

                        if finish_reason is not None:
                            if current_block_type is not None:
                                yield _sse("content_block_stop", {"type": "content_block_stop", "index": current_block_index})

                            if not has_content:
                                yield _sse("content_block_start", {
                                    "type": "content_block_start",
                                    "index": 0,
                                    "content_block": {"type": "text", "text": ""},
                                })
                                yield _sse("content_block_stop", {"type": "content_block_stop", "index": 0})

                            stop_reason = "end_turn"
                            if finish_reason == "tool_calls":
                                stop_reason = "tool_use"
                            elif finish_reason == "length":
                                stop_reason = "max_tokens"

                            yield _sse("message_delta", {
                                "type": "message_delta",
                                "delta": {"stop_reason": stop_reason, "stop_sequence": None},
                                "usage": {"output_tokens": total_output_tokens},
                            })
                            yield _sse("message_stop", {"type": "message_stop"})

        except Exception as e:
            logger.error(f"Anthropic stream error: {e}")
            if not sent_message_start:
                yield _sse("message_start", {
                    "type": "message_start",
                    "message": {
                        "id": msg_id, "type": "message", "role": "assistant",
                        "model": requested_model, "content": [], "stop_reason": None,
                        "stop_sequence": None, "usage": {"input_tokens": 0, "output_tokens": 0},
                    },
                })
            yield _sse("error", {"type": "error", "error": {"type": "api_error", "message": str(e)}})

    return StreamingResponse(generate(), media_type="text/event-stream", headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})


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
