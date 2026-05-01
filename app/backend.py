#!/usr/bin/env python3
"""
云集智能编程工作站 - 后端服务模块
替代 Electron main.cjs 的全部后端逻辑

包含:
- Ollama 代理服务器 (格式转换 + 自动降级重试)
- Claude CLI 子进程管理 (stream-json 解析)
- .env 配置读写
- 模型列表 API (OpenRouter / Anthropic / Ollama)
- QWebChannel 桥接对象 (替代 Electron IPC)
"""

import os
import sys
import json
import re
import uuid
import time
import shutil
import subprocess
import threading
import queue
import http.server
import http.client
import socketserver
import socket
import urllib.request
import urllib.error
from typing import Optional, Dict, List, Any, Callable
from pathlib import Path


# ── 常量 ──
MODEL_KEYS = [
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "OLLAMA_MODEL",
]

SETTINGS_KEYS = [
    "MODEL_PROVIDER",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "OLLAMA_BASE_URL",
    "OLLAMA_MODEL",
    "API_BASE_URL",
    "API_MODEL",
    "API_KEY",
    "API_TIMEOUT_MS",
    "DISABLE_TELEMETRY",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "AI_LANGUAGE",
    "AI_TEMPERATURE",
    "AI_MAX_TOKENS",
    "SYSTEM_PROMPT",
]

OLLAMA_AGENT_MAX_STEPS = 10
TOOL_TEXT_LIMIT = 12000
COMMAND_OUTPUT_LIMIT = 8000

TOOL_CALLING_MODEL_PATTERNS = [
    "qwen3", "qwen2.5", "qwen2-",
    "llama3.1", "llama3.2", "llama3.3", "llama4",
    "mistral", "mixtral",
    "command-r",
    "gemma2", "gemma3",
    "phi3", "phi4",
    "deepseek-r1", "deepseek-coder-v2", "deepseek-v3",
    "snowflake-arctic",
    "cogito",
    "devstral",
]


# ── .env 配置管理 ──
class EnvFileManager:
    """读写 .env 文件中的配置项"""

    def __init__(self, env_path: str):
        self.env_path = env_path

    def parse_env_lines(self, raw: str):
        lines = raw.splitlines()
        kv = {}
        for line in lines:
            if not line or line.strip().startswith("#"):
                continue
            idx = line.index("=") if "=" in line else -1
            if idx <= 0:
                continue
            kv[line[:idx].strip()] = line[idx + 1:]
        return lines, kv

    def read_settings(self) -> dict:
        if not os.path.exists(self.env_path):
            return {}
        with open(self.env_path, "r", encoding="utf-8") as f:
            raw = f.read()
        _, kv = self.parse_env_lines(raw)
        return {k: kv.get(k, "") for k in SETTINGS_KEYS}

    def write_settings(self, partial: dict) -> dict:
        partial = partial or {}
        existing = ""
        if os.path.exists(self.env_path):
            with open(self.env_path, "r", encoding="utf-8") as f:
                existing = f.read()
        lines, kv = self.parse_env_lines(existing)
        for key in SETTINGS_KEYS:
            if key in partial:
                kv[key] = str(partial[key] or "")

        used = set()
        next_lines = []
        for line in lines:
            idx = line.index("=") if "=" in line else -1
            if idx <= 0:
                next_lines.append(line)
                continue
            key = line[:idx].strip()
            if key not in SETTINGS_KEYS:
                next_lines.append(line)
                continue
            used.add(key)
            next_lines.append(f"{key}={kv.get(key, '')}")

        for key in SETTINGS_KEYS:
            if key not in used and key in kv:
                next_lines.append(f"{key}={kv[key]}")

        content = "\n".join(next_lines)
        content = re.sub(r"\n{3,}", "\n\n", content).strip() + "\n"
        with open(self.env_path, "w", encoding="utf-8") as f:
            f.write(content)
        return self.read_settings()

    def clear_model_settings(self) -> dict:
        reset = {k: "" for k in MODEL_KEYS}
        return self.write_settings(reset)


# ── 模型列表 API ──
def fetch_json_with_timeout(url: str, headers: dict = None, timeout_ms: int = 15000, method: str = "GET", body: bytes = None):
    """带超时的 HTTP 请求"""
    try:
        req = urllib.request.Request(url, method=method, headers=headers or {})
        if body:
            req.data = body
        resp = urllib.request.urlopen(req, timeout=timeout_ms / 1000)
        text = resp.read().decode("utf-8")
        data = None
        try:
            data = json.loads(text)
        except:
            pass
        return {"ok": True, "status": resp.status, "data": data, "text": text}
    except urllib.error.HTTPError as e:
        text = ""
        try:
            text = e.read().decode("utf-8")
        except:
            pass
        data = None
        try:
            data = json.loads(text)
        except:
            pass
        return {"ok": False, "status": e.code, "data": data, "text": text}
    except Exception as e:
        return {"ok": False, "status": 0, "data": None, "text": str(e)}


def normalize_model_entries(lst: list, provider: str) -> list:
    if not isinstance(lst, list):
        return []
    result = []
    for item in lst[:120]:
        mid = str(item.get("id", "") or "").strip()
        if not mid:
            continue
        name = str(item.get("name", "") or item.get("display_name", "") or mid).strip()
        result.append({"id": mid, "name": name, "provider": provider})
    return result


def guess_tool_support_by_name(name: str) -> bool:
    lower = (name or "").lower()
    if lower.endswith(":cloud"):
        return False
    return any(p in lower for p in TOOL_CALLING_MODEL_PATTERNS)


def fetch_ollama_capabilities(base_url: str, model_name: str, timeout_ms: int = 5000) -> Optional[dict]:
    """查询 Ollama 模型的 capabilities"""
    try:
        url = f"{base_url.rstrip('/')}/api/show"
        body = json.dumps({"name": model_name}).encode("utf-8")
        result = fetch_json_with_timeout(
            url,
            headers={"Content-Type": "application/json"},
            timeout_ms=min(timeout_ms, 5000),
            method="POST",
            body=body,
        )
        if not result["ok"]:
            return None
        data = result.get("data") or {}
        # 新版本在顶层 capabilities
        caps = data.get("capabilities")
        if caps and isinstance(caps, dict):
            return caps
        # 有些版本在 model_info.capabilities
        caps = (data.get("model_info") or {}).get("capabilities")
        if caps and isinstance(caps, dict):
            return caps
        return None
    except:
        return None


def list_openrouter_models(timeout_ms: int = 15000) -> dict:
    result = fetch_json_with_timeout("https://openrouter.ai/api/v1/models", timeout_ms=timeout_ms)
    if not result["ok"]:
        return {"ok": False, "error": f"OpenRouter API failed ({result['status']})"}
    models = normalize_model_entries((result.get("data") or {}).get("data", []), "openrouter")
    return {"ok": True, "models": models}


def list_anthropic_models(api_key: str, timeout_ms: int = 15000) -> dict:
    if not api_key or not api_key.strip():
        return {"ok": False, "error": "Anthropic API key is required."}
    result = fetch_json_with_timeout(
        "https://api.anthropic.com/v1/models",
        headers={
            "x-api-key": api_key.strip(),
            "anthropic-version": "2023-06-01",
        },
        timeout_ms=timeout_ms,
    )
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"Anthropic API failed ({result['status']}) {err}".strip()}
    models = normalize_model_entries((result.get("data") or {}).get("data", []), "anthropic")
    return {"ok": True, "models": models}


def check_ollama_model_health(base_url: str, model_name: str, timeout_ms: int = 30000) -> dict:
    """检测模型是否能正常加载和响应，返回健康状态"""
    result = {"loadable": True, "tool_support": None, "error": ""}
    try:
        url = f"{base_url.rstrip('/')}/v1/chat/completions"
        body = json.dumps({
            "model": model_name,
            "messages": [{"role": "user", "content": "hi"}],
            "stream": False,
            "max_tokens": 5,
        }).encode("utf-8")
        resp = fetch_json_with_timeout(
            url,
            headers={"Content-Type": "application/json"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if resp.get("ok"):
            result["loadable"] = True
        else:
            status = resp.get("status", 0)
            err_text = (resp.get("text", "") or "")[:300]
            if status == 500 and "unable to load" in err_text.lower():
                result["loadable"] = False
                result["error"] = "模型文件损坏或无法加载"
            elif status == 400 and "does not support tools" in err_text:
                result["loadable"] = True
                result["tool_support"] = False
            elif status >= 400:
                result["loadable"] = False
                result["error"] = f"API错误({status})"
            else:
                result["loadable"] = False
                result["error"] = err_text[:100] or "未知错误"
    except Exception as e:
        result["loadable"] = False
        result["error"] = str(e)[:100]
    return result


def list_ollama_models(base_url: str, timeout_ms: int = 15000, check_health: bool = False) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:11434"
    url = f"{base.rstrip('/')}/api/tags"
    result = fetch_json_with_timeout(url, timeout_ms=timeout_ms)
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"Ollama API failed ({result['status']}) {err}".strip()}
    raw_models = ((result.get("data") or {}).get("models", []))[:120]

    # 并行查询 capabilities
    cap_results = [None] * len(raw_models)
    health_results = [None] * len(raw_models) if check_health else []

    def _fetch_cap(idx, m):
        cap_results[idx] = fetch_ollama_capabilities(base, m.get("name", ""), timeout_ms)

    def _fetch_health(idx, m):
        health_results[idx] = check_ollama_model_health(base, m.get("name", ""), min(timeout_ms, 60000))

    threads = []
    for i, m in enumerate(raw_models):
        t = threading.Thread(target=_fetch_cap, args=(i, m), daemon=True)
        t.start()
        threads.append(t)
        if check_health:
            ht = threading.Thread(target=_fetch_health, args=(i, m), daemon=True)
            ht.start()
            threads.append(ht)
    for t in threads:
        t.join(timeout=120 if check_health else 10)

    models = []
    for i, m in enumerate(raw_models):
        tool_support = guess_tool_support_by_name(m.get("name", ""))
        cap = cap_results[i]
        cap_resolved = False
        if cap:
            if isinstance(cap.get("tool_calling"), bool):
                tool_support = cap["tool_calling"]
                cap_resolved = True
            elif isinstance(cap.get("input_tool"), bool):
                tool_support = cap["input_tool"]
                cap_resolved = True
            if not cap_resolved and len(cap) > 0:
                tool_support = False
        loadable = True
        health_error = ""
        if check_health and health_results:
            hr = health_results[i]
            if hr:
                loadable = hr.get("loadable", True)
                health_error = hr.get("error", "")
                if hr.get("tool_support") is False:
                    tool_support = False
        size_bytes = m.get("size", 0) or 0
        size_str = ""
        if size_bytes > 0:
            size_gb = size_bytes / (1024 * 1024 * 1024)
            if size_gb >= 1:
                size_str = f"{size_gb:.1f}GB"
            else:
                size_mb = size_bytes / (1024 * 1024)
                size_str = f"{size_mb:.0f}MB"
        models.append({
            "id": m.get("name", ""),
            "name": m.get("name", ""),
            "provider": "ollama",
            "toolSupport": tool_support,
            "size": size_str,
            "family": m.get("details", {}).get("family", "") if isinstance(m.get("details"), dict) else "",
            "paramCount": m.get("details", {}).get("parameter_size", "") if isinstance(m.get("details"), dict) else "",
            "loadable": loadable,
            "healthError": health_error,
        })
    return {"ok": True, "models": models}


def list_api_models(base_url: str, api_key: str = "", timeout_ms: int = 15000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    url = f"{base.rstrip('/')}/v1/models"
    headers = {"Content-Type": "application/json"}
    if api_key and api_key.strip():
        headers["Authorization"] = f"Bearer {api_key.strip()}"
    result = fetch_json_with_timeout(url, headers=headers, timeout_ms=timeout_ms)
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"API 服务连接失败 ({result['status']}) {err}".strip()}
    raw_models = ((result.get("data") or {}).get("data", []))[:120]
    models = []
    for m in raw_models:
        model_id = m.get("id", m.get("name", ""))
        models.append({
            "id": model_id,
            "name": m.get("name", model_id),
            "provider": "api",
            "toolSupport": True,
            "size": "",
            "family": m.get("owned_by", ""),
            "paramCount": "",
            "loadable": True,
            "healthError": "",
        })
    if not models:
        models.append({
            "id": "qwen3.6-plus",
            "name": "qwen3.6-plus",
            "provider": "api",
            "toolSupport": True,
            "size": "",
            "family": "qwen",
            "paramCount": "",
            "loadable": True,
            "healthError": "",
        })
    return {"ok": True, "models": models}


def check_api_service(base_url: str, timeout_ms: int = 5000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    try:
        result = fetch_json_with_timeout(f"{base.rstrip('/')}/healthz", timeout_ms=timeout_ms)
        if result["ok"]:
            info = {"ok": True, "running": True}
            try:
                acct_result = fetch_json_with_timeout(
                    f"{base.rstrip('/')}/api/admin/accounts",
                    headers={"Content-Type": "application/json", "Authorization": "Bearer admin"},
                    timeout_ms=timeout_ms,
                )
                if acct_result["ok"]:
                    accounts = (acct_result.get("data") or {}).get("accounts", [])
                    info["accountCount"] = len(accounts)
                    if len(accounts) == 0:
                        info["warning"] = "未添加上游账号，AI 对话将返回 500 错误。请在管理台添加 chat.qwen.ai 的账号 Token。"
            except Exception:
                pass
            return info
    except:
        pass
    return {"ok": True, "running": False}


def _ensure_api_service(base_url: str = "", admin_key: str = "admin") -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"

    def _check_service():
        import urllib.request
        try:
            urllib.request.urlopen(f"{base.rstrip('/')}/healthz", timeout=3)
            return True
        except Exception:
            pass
        try:
            from urllib.parse import urlparse
            parsed = urlparse(base)
            import socket
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.settimeout(2)
            s.connect((parsed.hostname or "127.0.0.1", parsed.port or 7777))
            s.close()
            return True
        except Exception:
            return False

    if _check_service():
        return {"ok": True, "was_running": True}
    start_result = start_qwen2api("", 7777, ak)
    if not start_result.get("ok"):
        return {"ok": False, "error": f"API 服务未运行且自动启动失败: {start_result.get('message', start_result.get('error', ''))}"}
    import time
    for _ in range(20):
        time.sleep(1)
        if _check_service():
            return {"ok": True, "was_running": False}
    return {"ok": False, "error": "API 服务启动超时"}


def add_qwen_account(base_url: str, token: str, admin_key: str = "", timeout_ms: int = 15000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"
    if not token or not token.strip():
        return {"ok": False, "error": "Token 不能为空"}

    try:
        body = json.dumps({"token": token.strip()}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/accounts",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                return {"ok": True, "email": data.get("email", "")}
            return {"ok": False, "error": data.get("error", "添加账户失败")}
        err_text = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"请求失败 ({result.get('status', '?')}) {err_text}".strip()}
    except Exception as e:
        return {"ok": False, "error": f"添加账户异常: {e}"}


def list_qwen_accounts(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"

    try:
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/accounts",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
        )
        if result["ok"]:
            data = result.get("data") or {}
            accounts = data.get("accounts", [])
            sticky = data.get("sticky_email") or ""
            return {"ok": True, "accounts": accounts, "count": len(accounts), "sticky_email": sticky}
        return {"ok": False, "error": "获取账户列表失败"}
    except Exception as e:
        return {"ok": False, "error": f"获取账户列表异常: {e}"}


def delete_qwen_account(base_url: str, email: str, admin_key: str = "") -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"
    if not email or not email.strip():
        return {"ok": False, "error": "邮箱不能为空"}
    try:
        import urllib.request
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/{urllib.parse.quote(email.strip())}",
            headers={"Authorization": f"Bearer {ak}"},
            method="DELETE",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return {"ok": data.get("ok", True)}
    except Exception as e:
        return {"ok": False, "error": f"删除账户失败: {e}"}

def set_sticky_account(base_url: str, email: str, admin_key: str = "") -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"
    if not email or not email.strip():
        return {"ok": False, "error": "邮箱不能为空"}
    try:
        import urllib.request
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/{urllib.parse.quote(email.strip())}/set-sticky",
            headers={"Authorization": f"Bearer {ak}", "Content-Type": "application/json"},
            method="POST",
            data=b"{}",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data
    except Exception as e:
        return {"ok": False, "error": f"设置优先账户失败: {e}"}

def clear_sticky_account(base_url: str, admin_key: str = "") -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"
    try:
        import urllib.request
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/clear-sticky",
            headers={"Authorization": f"Bearer {ak}", "Content-Type": "application/json"},
            method="POST",
            data=b"{}",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data
    except Exception as e:
        return {"ok": False, "error": f"清除优先账户失败: {e}"}

_login_state = {
    "busy": False,
    "done": False,
    "ok": False,
    "email": "",
    "error": "",
}

def _do_login_background(base_url: str, admin_key: str, email: str, password: str):
    global _login_state
    try:
        body = json.dumps({"email": email, "password": password}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/login",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {admin_key}"},
            timeout_ms=120000,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                _login_state["ok"] = True
                _login_state["email"] = data.get("email", "")
                _login_state["error"] = ""
            else:
                _login_state["ok"] = False
                _login_state["error"] = data.get("error", "登录失败")
        else:
            err_text = (result.get("text", "") or "")[:300]
            _login_state["ok"] = False
            _login_state["error"] = f"请求失败 ({result.get('status', '?')}) {err_text}".strip()
    except Exception as e:
        _login_state["ok"] = False
        _login_state["error"] = f"登录异常: {e}"
    finally:
        _login_state["done"] = True
        _login_state["busy"] = False

def start_qwen_login(base_url: str, email: str, password: str, admin_key: str = "") -> dict:
    global _login_state
    if _login_state["busy"]:
        return {"ok": False, "error": "登录正在进行中，请稍候"}
    if not email or not password:
        return {"ok": False, "error": "邮箱和密码不能为空"}
    ak = (admin_key or "").strip() or "admin"
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ensure = _ensure_api_service(base, ak)
    if not ensure.get("ok"):
        return {"ok": False, "error": ensure.get("error", "API 服务不可用")}
    _login_state = {
        "busy": True,
        "done": False,
        "ok": False,
        "email": "",
        "error": "",
    }
    t = threading.Thread(target=_do_login_background, args=(base, ak, email, password), daemon=True)
    t.start()
    return {"ok": True, "message": "登录已启动"}

def poll_qwen_login() -> dict:
    global _login_state
    return {
        "busy": _login_state["busy"],
        "done": _login_state["done"],
        "success": _login_state["ok"],
        "email": _login_state.get("email", ""),
        "error": _login_state.get("error", ""),
    }

_register_state = {
    "busy": False,
    "done": False,
    "ok": False,
    "email": "",
    "error": "",
    "log_offset": 0,
    "log_lines": [],
}


def _get_qwen2api_log_path() -> str:
    app_dir = _app_dir()
    qwen_dir = Path(app_dir) / "qwen2api"
    if qwen_dir.exists():
        return str(qwen_dir / "data" / "qwen2api.log")
    return ""


def _read_qwen2api_log_tail(n: int = 50, offset: int = 0) -> dict:
    log_path = _get_qwen2api_log_path()
    if not log_path or not os.path.isfile(log_path):
        return {"ok": True, "lines": [], "total": 0}
    try:
        with open(log_path, "r", encoding="utf-8", errors="replace") as f:
            all_lines = f.readlines()
        total = len(all_lines)
        start = max(offset, 0)
        selected = all_lines[start:start + n]
        return {"ok": True, "lines": [l.rstrip("\n\r") for l in selected], "total": total, "nextOffset": start + len(selected)}
    except Exception as e:
        return {"ok": False, "error": str(e), "lines": [], "total": 0}


def _do_register_background(base_url: str, admin_key: str, custom_email: str = "", custom_password: str = "", custom_username: str = ""):
    global _register_state
    try:
        reg_body = {}
        if custom_email:
            reg_body["email"] = custom_email
        if custom_password:
            reg_body["password"] = custom_password
        if custom_username:
            reg_body["username"] = custom_username
        body = json.dumps(reg_body).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/register",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {admin_key}"},
            timeout_ms=180000,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                _register_state["ok"] = True
                _register_state["email"] = data.get("email", "")
                _register_state["error"] = ""
            else:
                _register_state["ok"] = False
                _register_state["error"] = data.get("error", "自动注册失败")
        else:
            err_text = (result.get("text", "") or "")[:300]
            _register_state["ok"] = False
            _register_state["error"] = f"请求失败 ({result.get('status', '?')}) {err_text}".strip()
    except Exception as e:
        _register_state["ok"] = False
        _register_state["error"] = f"自动注册异常: {e}"
    finally:
        _register_state["done"] = True
        _register_state["busy"] = False


def start_qwen_register(base_url: str, admin_key: str = "", custom_email: str = "", custom_password: str = "", custom_username: str = "") -> dict:
    global _register_state
    if _register_state["busy"]:
        return {"ok": False, "error": "注册正在进行中，请稍候"}
    ak = (admin_key or "").strip() or "admin"
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ensure = _ensure_api_service(base, ak)
    if not ensure.get("ok"):
        return {"ok": False, "error": ensure.get("error", "API 服务不可用")}
    _register_state = {
        "busy": True,
        "done": False,
        "ok": False,
        "email": "",
        "error": "",
        "log_offset": 0,
        "log_lines": [],
    }
    log_path = _get_qwen2api_log_path()
    if log_path and os.path.isfile(log_path):
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                _register_state["log_offset"] = len(f.readlines())
        except Exception:
            _register_state["log_offset"] = 0
    t = threading.Thread(target=_do_register_background, args=(base, ak, custom_email, custom_password, custom_username), daemon=True)
    t.start()
    return {"ok": True, "message": "注册已启动"}


def poll_qwen_register() -> dict:
    global _register_state
    log_path = _get_qwen2api_log_path()
    new_lines = []
    if log_path and os.path.isfile(log_path):
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                all_lines = f.readlines()
            offset = _register_state.get("log_offset", 0)
            if offset < len(all_lines):
                new_lines = [l.rstrip("\n\r") for l in all_lines[offset:]]
                _register_state["log_offset"] = len(all_lines)
            _register_state["log_lines"].extend(new_lines)
        except Exception:
            pass
    register_lines = [l for l in new_lines if "[Register]" in l or "[注册]" in l or "register" in l.lower()]
    return {
        "ok": True,
        "busy": _register_state["busy"],
        "done": _register_state["done"],
        "success": _register_state["ok"],
        "email": _register_state.get("email", ""),
        "error": _register_state.get("error", ""),
        "newLines": register_lines,
        "allNewLines": new_lines,
    }


def fetch_api_key(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:7777"
    ak = (admin_key or "").strip() or "admin"

    try:
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/keys",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
        )
        if result["ok"]:
            data = result.get("data") or {}
            keys = data.get("keys", [])
            if keys:
                return {"ok": True, "key": keys[0]}
    except:
        pass

    try:
        body = json.dumps({}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/keys",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            key = data.get("key", "")
            if key:
                return {"ok": True, "key": key}
    except:
        pass

    admin_url = base.rstrip("/")
    return {"ok": False, "error": "未能自动获取 API Key，请确认 API 服务已启动且 ADMIN_KEY 正确。", "adminUrl": admin_url}


UV_PYTHON_VERSION = "3.12"

MIRROR_SOURCES = {
    "official": {
        "label": "🌐 官方源",
        "node": "https://nodejs.org/dist/",
        "github_proxy": "",
        "uv_python_mirror": "",
        "pypi_index": "https://pypi.org/simple/",
    },
    "china": {
        "label": "🇨🇳 国内镜像",
        "node": "https://npmmirror.com/mirrors/node/",
        "github_proxy": "https://gh-proxy.com/",
        "uv_python_mirror": "https://registry.npmmirror.com/-/binary/python-build-standalone/",
        "pypi_index": "https://pypi.tuna.tsinghua.edu.cn/simple/",
    },
}

def _app_dir() -> str:
    if getattr(sys, 'frozen', False):
        exe_dir = os.path.abspath(os.path.dirname(sys.executable))
        return os.path.join(exe_dir, "app")
    return str(Path(__file__).resolve().parent)

def _uv_exe() -> str:
    return os.path.join(_app_dir(), "uv", "uv.exe")

def _uv_python_dir() -> str:
    return os.path.join(_app_dir(), "python")

def _qwen2api_venv_python() -> str:
    venv_dir = os.path.join(_app_dir(), "scripts", ".venv")
    if os.name == "nt":
        return os.path.join(venv_dir, "Scripts", "python.exe")
    return os.path.join(venv_dir, "bin", "python")

def _qwen2api_venv_exists() -> bool:
    return os.path.isfile(_qwen2api_venv_python())

def _load_mirror_key() -> str:
    try:
        fp = os.path.join(_app_dir(), "mirror_source.json")
        if os.path.isfile(fp):
            with open(fp, "r", encoding="utf-8") as f:
                key = f.read().strip()
            if key in MIRROR_SOURCES:
                return key
    except Exception:
        pass
    return "china"

def _uv_env() -> dict:
    env = os.environ.copy()
    env["UV_PYTHON_INSTALL_DIR"] = _uv_python_dir()
    env["UV_PYTHON_DOWNLOADS"] = "auto"
    mirror = MIRROR_SOURCES.get(_load_mirror_key(), MIRROR_SOURCES["china"])
    python_mirror = mirror.get("uv_python_mirror", "")
    if python_mirror:
        env["UV_PYTHON_INSTALL_MIRROR"] = python_mirror
    pypi_index = mirror.get("pypi_index", "")
    if pypi_index:
        env["UV_INDEX_URL"] = pypi_index
        env["UV_DEFAULT_INDEX"] = pypi_index
    return env

def _check_qwen2api_deps() -> bool:
    if not _qwen2api_venv_exists():
        return False
    venv_python = _qwen2api_venv_python()
    try:
        r = subprocess.run(
            [venv_python, "-c",
             "import fastapi, uvicorn, httpx, pydantic_settings, tiktoken, curl_cffi, camoufox; print('ok')"],
            capture_output=True, text=True, timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        return r.returncode == 0 and "ok" in (r.stdout or "")
    except Exception:
        return False

_qwen2api_proc = None

def stop_qwen2api() -> dict:
    global _qwen2api_proc
    if _qwen2api_proc is None or _qwen2api_proc.poll() is not None:
        _qwen2api_proc = None
        return {"ok": True, "message": "API 服务未在运行"}
    try:
        _qwen2api_proc.terminate()
        try:
            _qwen2api_proc.wait(timeout=10)
        except Exception:
            _qwen2api_proc.kill()
            try:
                _qwen2api_proc.wait(timeout=5)
            except Exception:
                pass
        _qwen2api_proc = None
        return {"ok": True, "message": "API 服务已停止"}
    except Exception as e:
        return {"ok": False, "error": f"停止 API 服务失败: {e}"}

def start_qwen2api(project_dir: str = "", port: int = 7777, admin_key: str = "admin") -> dict:
    global _qwen2api_proc

    _debug_log = []
    _debug_log.append(f"_app_dir={_app_dir()}")
    _debug_log.append(f"frozen={getattr(sys, 'frozen', False)}")
    _debug_log.append(f"exe={getattr(sys, 'executable', '')}")
    _debug_log.append(f"__file__={__file__}")

    try:
        _debug_path = os.path.join(os.path.dirname(os.path.abspath(sys.executable if getattr(sys, 'frozen', False) else __file__)), "start_qwen2api_debug.log")
        with open(_debug_path, "w", encoding="utf-8") as _df:
            _df.write("\n".join(_debug_log) + "\n")
    except Exception:
        pass

    base = f"http://127.0.0.1:{port}"
    status = check_api_service(base)
    if status.get("running"):
        return {"ok": True, "message": "API 服务已在运行", "baseUrl": base}

    if _qwen2api_proc and _qwen2api_proc.poll() is None:
        return {"ok": True, "message": "API 服务正在启动中", "baseUrl": base}

    qwen_dir = project_dir.strip()
    if not qwen_dir:
        app_qwen_dir = Path(_app_dir()) / "qwen2api"
        _debug_log.append(f"app_qwen_dir={app_qwen_dir}")
        _debug_log.append(f"app_qwen_dir.exists={app_qwen_dir.exists()}")
        if app_qwen_dir.exists():
            qwen_dir = str(app_qwen_dir)
        else:
            debug_info = " | ".join(_debug_log)
            return {"ok": False, "error": f"未找到 qwen2api 目录: {app_qwen_dir} [{debug_info}]"}

    if not Path(qwen_dir).exists():
        return {"ok": False, "error": f"qwen2api 目录不存在: {qwen_dir}"}

    venv_python = _qwen2api_venv_python()
    _debug_log.append(f"venv_python={venv_python}")
    _debug_log.append(f"venv_python.exists={os.path.isfile(venv_python)}")
    venv_dir = os.path.join(_app_dir(), "scripts", ".venv")
    uv = _uv_exe()
    _debug_log.append(f"uv={uv}")
    _debug_log.append(f"uv.exists={os.path.isfile(uv)}")
    env = _uv_env()

    if not _check_qwen2api_deps():
        if not os.path.isfile(venv_python):
            if not os.path.isfile(uv):
                return {"ok": False, "error": "uv 未安装，请先在部署维护中安装 uv"}
            try:
                subprocess.check_call(
                    [uv, "venv", venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                    env=env,
                    timeout=300,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
            except Exception as e:
                return {"ok": False, "error": f"创建虚拟环境失败: {e}"}

        req_file = Path(qwen_dir) / "backend" / "requirements.txt"
        if req_file.exists():
            try:
                subprocess.check_call(
                    [uv, "pip", "install", "-r", str(req_file), "--python", venv_python],
                    env=env,
                    timeout=300,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
            except Exception as e:
                return {"ok": False, "error": f"安装 API 服务依赖失败: {e}"}

    env.update({
        "PYTHONPATH": str(qwen_dir),
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
        "PORT": str(port),
        "ADMIN_KEY": admin_key,
        "WORKERS": "1",
        "ENGINE_MODE": "httpx",
        "LOG_LEVEL": "WARNING",
        "ACCOUNTS_FILE": str(Path(qwen_dir) / "data" / "accounts.json"),
        "USERS_FILE": str(Path(qwen_dir) / "data" / "users.json"),
        "VIRTUAL_ENV": venv_dir,
    })

    data_dir = Path(qwen_dir) / "data"
    data_dir.mkdir(exist_ok=True)

    log_path = Path(qwen_dir) / "data" / "qwen2api.log"
    try:
        log_file = open(log_path, "a", encoding="utf-8")
    except Exception:
        log_file = subprocess.PIPE

    try:
        proc = subprocess.Popen(
            [venv_python, "-m", "uvicorn", "backend.main:app",
             "--host", "0.0.0.0", "--port", str(port), "--workers", "1"],
            cwd=qwen_dir,
            env=env,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        _qwen2api_proc = proc
    except Exception as e:
        if log_file != subprocess.PIPE:
            try: log_file.close()
            except: pass
        return {"ok": False, "error": f"启动 API 服务失败: {e} [{chr(124).join(_debug_log)}]"}

    import time as _time
    _time.sleep(2)
    if proc.poll() is not None:
        err_msg = "进程意外退出"
        if log_file != subprocess.PIPE:
            try:
                log_file.close()
                with open(log_path, "r", encoding="utf-8", errors="replace") as lf:
                    tail = lf.read()[-2000:]
                if tail.strip():
                    err_msg = tail.strip().split("\n")[-1][:200]
            except:
                pass
        _debug_log.append(f"RESULT=进程意外退出 err_msg={err_msg}")
        try:
            with open(_debug_path, "a", encoding="utf-8") as _df:
                _df.write("\n".join(_debug_log) + "\n")
        except: pass
        return {"ok": False, "error": f"API 服务启动失败: {err_msg}", "logPath": str(log_path), "debug": " | ".join(_debug_log)}

    _debug_log.append("RESULT=ok")
    try:
        with open(_debug_path, "a", encoding="utf-8") as _df:
            _df.write("\n".join(_debug_log) + "\n")
    except: pass
    return {"ok": True, "message": "API 服务正在启动，请稍候检查状态", "baseUrl": base, "pid": proc.pid, "logPath": str(log_path)}


# ── Ollama 代理服务器 ──
class OllamaProxyHandler(http.server.BaseHTTPRequestHandler):
    """将 Anthropic Messages API 转换为 Ollama /api/chat API（原生 NDJSON 格式）

    对齐 BAK 版本 (bin/claude-code-tudou) 的实现方式：
    - 使用 Ollama 原生 /api/chat 端点（而非 /v1/chat/completions）
    - NDJSON 格式流式响应（每行一个完整 JSON，而非 SSE data: 格式）
    - 通过 ollamaChunk.done 标记完成（而非 [DONE] 标记）
    - 使用 http.client 直连（而非 urllib，避免缓冲问题）
    """

    target_url = None
    proxy_model = "qwen3:8b"
    close_connection = True
    _proxy_log_file = None
    _proxy_log_dir = None

    def log_message(self, format, *args):
        pass

    @classmethod
    def _log_proxy(cls, msg):
        try:
            if cls._proxy_log_file is None:
                log_dir = cls._proxy_log_dir or _app_dir()
                os.makedirs(log_dir, exist_ok=True)
                cls._proxy_log_file = open(os.path.join(log_dir, "proxy_debug.log"), "a", encoding="utf-8")
            ts = time.strftime("%H:%M:%S")
            cls._proxy_log_file.write(f"[{ts}] {msg}\n")
            cls._proxy_log_file.flush()
        except Exception as e:
            try:
                sys.stderr.write(f"[proxy_log_err] {e}\n")
            except:
                pass

    def do_POST(self):
        if self.path.startswith("/v1/messages"):
            self._handle_messages()
        else:
            self.send_error(404)

    def _handle_messages(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body_bytes = self.rfile.read(content_length)
        try:
            anthropic_body = json.loads(body_bytes)
        except:
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": "Invalid JSON"}).encode())
            return

        msgs = anthropic_body.get("messages", [])
        msg_summary = []
        for m in msgs:
            role = m.get("role", "?")
            content = m.get("content", "")
            if isinstance(content, list):
                types = [b.get("type", "?") for b in content]
                msg_summary.append(f"{role}[{','.join(types)}]")
            elif isinstance(content, str):
                msg_summary.append(f"{role}({len(content)})")
            else:
                msg_summary.append(f"{role}(?)")
        self._log_proxy(f"REQUEST: msgs={len(msgs)} summary={msg_summary} stream={anthropic_body.get('stream')} tools={len(anthropic_body.get('tools', []))}")

        ollama_messages, system_prompt = self._convert_messages(anthropic_body)
        ollama_tools = self._convert_tools(anthropic_body)

        MAX_MESSAGES = 30
        if len(ollama_messages) > MAX_MESSAGES:
            self._log_proxy(f"TRUNCATE: {len(ollama_messages)} msgs > {MAX_MESSAGES}, keeping last {MAX_MESSAGES}")
            ollama_messages = ollama_messages[-MAX_MESSAGES:]

        stream = anthropic_body.get("stream") is True
        ollama_body = {
            "model": self.proxy_model,
            "messages": ollama_messages,
            "stream": stream,
            "options": {
                "num_ctx": 65536,
            },
        }
        if system_prompt:
            ollama_body["system"] = system_prompt
        if ollama_tools:
            ollama_body["tools"] = ollama_tools

        env_temp = os.environ.get("AI_TEMPERATURE", "").strip()
        env_max_tokens = os.environ.get("AI_MAX_TOKENS", "").strip()
        if env_temp:
            try:
                ollama_body["options"]["temperature"] = float(env_temp)
            except ValueError:
                pass
        if env_max_tokens:
            try:
                ollama_body["options"]["num_predict"] = int(env_max_tokens)
            except ValueError:
                pass

        self._send_to_ollama(ollama_body, stream)

    def _convert_messages(self, body: dict):
        ollama_msgs = []
        system_prompt = ""

        if isinstance(body.get("system"), str) and body["system"].strip():
            system_prompt = body["system"]
        elif isinstance(body.get("system"), list):
            system_prompt = "\n".join(
                b.get("text", "") for b in body["system"] if b.get("type") == "text"
            )
        else:
            lang = os.environ.get("AI_LANGUAGE", "zh").strip().lower()
            if lang == "zh":
                system_prompt = "你是一个专业的AI编程助手。请始终使用中文回答。只有在用户明确要求执行编程任务时才使用工具。普通对话请直接回答。"
            elif lang == "ja":
                system_prompt = "あなたはプロのAIプログラミングアシスタントです。日本語で回答してください。"
            elif lang == "ko":
                system_prompt = "당신은 전문 AI 프로그래밍 어시스턴트입니다. 한국어로 답변해 주세요."
            elif lang and lang != "en":
                system_prompt = f"You are a professional AI coding assistant. Please respond in {lang}."

        for msg in body.get("messages", []):
            if msg["role"] == "user":
                tool_results, text_parts = [], []
                content = msg.get("content", "")
                if isinstance(content, list):
                    for block in content:
                        if block.get("type") == "tool_result":
                            tool_results.append(block)
                        elif block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                elif isinstance(content, str):
                    text_parts.append(content)

                for tr in tool_results:
                    c = tr.get("content", "")
                    if isinstance(c, list):
                        c = "\n".join(b.get("text", "") for b in c if b.get("type") == "text")
                    elif not isinstance(c, str):
                        c = ""
                    ollama_msgs.append({
                        "role": "tool",
                        "name": tr.get("tool_use_id", "unknown"),
                        "content": c or "(no output)",
                    })
                text = "\n".join(p for p in text_parts if p)
                if text:
                    ollama_msgs.append({"role": "user", "content": text})

            elif msg["role"] == "assistant":
                tool_calls, text_parts = [], []
                content = msg.get("content", "")
                if isinstance(content, list):
                    for block in content:
                        if block.get("type") == "tool_use":
                            tool_calls.append(block)
                        elif block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                elif isinstance(content, str):
                    text_parts.append(content)

                text = "\n".join(p for p in text_parts if p)
                assistant_msg = {"role": "assistant", "content": text or ""}
                ollama_tool_calls = []
                for tc in tool_calls:
                    args = tc.get("input", {})
                    if isinstance(args, str):
                        try:
                            args = json.loads(args)
                        except:
                            args = {}
                    elif not isinstance(args, dict):
                        args = {}
                    ollama_tool_calls.append({
                        "function": {
                            "name": tc.get("name", ""),
                            "arguments": args,
                        },
                    })
                if ollama_tool_calls:
                    assistant_msg["tool_calls"] = ollama_tool_calls
                ollama_msgs.append(assistant_msg)

        self._log_proxy(f"CONVERT: {len(body.get('messages', []))} anthropic msgs -> {len(ollama_msgs)} ollama msgs")
        for i, m in enumerate(ollama_msgs):
            role = m.get("role", "?")
            tc = m.get("tool_calls", [])
            self._log_proxy(f"  msg[{i}]: role={role} has_tool_calls={len(tc) > 0} content_len={len(m.get('content', ''))}")

        return ollama_msgs, system_prompt

    def _convert_tools(self, body: dict) -> list:
        ollama_tools = []
        for tool in body.get("tools", []):
            if tool.get("type") == "custom":
                continue
            ollama_tools.append({
                "type": "function",
                "function": {
                    "name": tool.get("name", ""),
                    "description": tool.get("description", ""),
                    "parameters": tool.get("input_schema", {"type": "object", "properties": {}}),
                },
            })
        return ollama_tools if ollama_tools else None

    def _send_to_ollama(self, body_obj: dict, stream: bool):
        body_str = json.dumps(body_obj, ensure_ascii=False).encode("utf-8")
        target = self.target_url

        msg_count = len(body_obj.get("messages", []))
        has_tools = "yes" if body_obj.get("tools") else "no"
        self._log_proxy(f"OLLAMA: msgs={msg_count} tools={has_tools} stream={stream} model={body_obj.get('model')} body_len={len(body_str)}")
        if msg_count > 0:
            last_msg = body_obj["messages"][-1]
            self._log_proxy(f"OLLAMA: last_msg role={last_msg.get('role')} content_preview={str(last_msg.get('content', ''))[:200]}")

        conn = None
        try:
            conn = http.client.HTTPConnection(
                target.hostname,
                target.port or 11434,
                timeout=30,
            )
            conn.request("POST", "/api/chat", body=body_str, headers={"Content-Type": "application/json"})
            resp = conn.getresponse()
            self._log_proxy(f"OLLAMA: response status={resp.status}")

            try:
                if conn.sock:
                    conn.sock.settimeout(90)
                    self._log_proxy("OLLAMA: set read timeout=90s")
            except Exception as te:
                self._log_proxy(f"OLLAMA: settimeout failed: {te}")

            if resp.status != 200:
                err_body = resp.read().decode("utf-8", errors="ignore")
                self._log_proxy(f"OLLAMA ERROR: code={resp.status} body={err_body[:500]}")
                if resp.status == 400 and "does not support tools" in err_body and body_obj.get("tools"):
                    fallback_body = {k: v for k, v in body_obj.items() if k != "tools"}
                    ollama_tools = body_obj.get("tools", [])
                    if ollama_tools:
                        tool_descs = "\n".join(
                            f"- {t.get('function', {}).get('name', '')}: {t.get('function', {}).get('description', '')} Params: {json.dumps(t.get('function', {}).get('parameters', {}))}"
                            for t in ollama_tools
                        )
                        bt = "`" * 3
                        tool_prompt = (
                            f"\n\nYou have access to the following tools. To call a tool, output a JSON block enclosed in {bt} tags like this:\n"
                            f"{bt}\n"
                            f'{{"name": "tool_name", "arguments": {{"param": "value"}}}}\n'
                            f"{bt}\n"
                            f"You can call multiple tools. After each tool call, wait for the result in the next user message enclosed in <tool_result> tags.\n"
                            f"Available tools:\n{tool_descs}"
                        )
                        fallback_body["system"] = fallback_body.get("system", "") + tool_prompt
                    fallback_str = json.dumps(fallback_body).encode("utf-8")
                    conn2 = http.client.HTTPConnection(
                        target.hostname,
                        target.port or 11434,
                        timeout=300,
                    )
                    conn2.request("POST", "/api/chat", body=fallback_str, headers={"Content-Type": "application/json"})
                    resp2 = conn2.getresponse()
                    if resp2.status != 200:
                        err_body2 = resp2.read().decode("utf-8", errors="ignore")
                        self.send_response(resp2.status)
                        self.send_header("Content-Type", "application/json")
                        self.end_headers()
                        self.wfile.write(json.dumps({
                            "type": "error",
                            "error": {"type": "invalid_request_error", "message": f"Ollama error ({resp2.status}): {err_body2}"},
                        }).encode())
                        conn2.close()
                        conn.close()
                        return
                    if stream:
                        self._handle_ndjson_stream(resp2, is_retry=True)
                    else:
                        self._handle_non_stream_response(resp2, is_retry=True)
                    return
                else:
                    self.send_response(resp.status)
                    self.send_header("Content-Type", "application/json")
                    self.end_headers()
                    self.wfile.write(json.dumps({
                        "type": "error",
                        "error": {"type": "invalid_request_error", "message": f"Ollama error ({resp.status}): {err_body}"},
                    }).encode())
                    conn.close()
                    return

            if stream:
                self._handle_ndjson_stream(resp)
            else:
                self._handle_non_stream_response(resp)
            self._log_proxy("OLLAMA: stream handling completed")
            try:
                conn.close()
            except:
                pass

        except Exception as e:
            self._log_proxy(f"OLLAMA EXCEPTION: {e}")
            try:
                self.send_response(502)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({
                    "type": "error",
                    "error": {"type": "api_error", "message": f"Proxy error: {e}"},
                }).encode())
            except:
                pass
            if conn:
                try:
                    conn.close()
                except:
                    pass

    def _handle_ndjson_stream(self, resp, is_retry=False):
        msg_id = f"msg_{int(time.time())}"
        sent_start = False
        content_block_index = 0
        current_tool_use_index = 0
        has_tool_calls = False
        full_content = ""
        input_tokens = 0
        output_tokens = 0
        text_block_started = False
        text_block_closed = False

        if is_retry:
            resp_data = resp.read().decode("utf-8", errors="ignore")
            for line in resp_data.split("\n"):
                trimmed = line.strip()
                if not trimmed:
                    continue
                try:
                    chunk = json.loads(trimmed)
                    if chunk.get("message", {}).get("content"):
                        full_content += chunk["message"]["content"]
                    if chunk.get("prompt_eval_count"):
                        input_tokens = chunk["prompt_eval_count"]
                    if chunk.get("eval_count"):
                        output_tokens = chunk["eval_count"]
                except:
                    pass

            tool_calls = self._parse_tool_calls_from_text(full_content)
            clean_content = self._remove_tool_call_blocks_from_text(full_content)

            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.end_headers()
            self._write_sse("message_start", {
                "type": "message_start",
                "message": {
                    "id": msg_id, "type": "message", "role": "assistant",
                    "content": [], "model": self.proxy_model,
                    "stop_reason": None, "stop_sequence": None,
                    "usage": {"input_tokens": input_tokens, "output_tokens": 0},
                },
            })

            cb_index = 0
            if clean_content.strip():
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": clean_content}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                cb_index = 1

            for i, tc in enumerate(tool_calls):
                tool_use_id = f"toolu_{int(time.time())}_{i}"
                self._write_sse("content_block_start", {"type": "content_block_start", "index": cb_index, "content_block": {"type": "tool_use", "id": tool_use_id, "name": tc["name"], "input": {}}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": cb_index, "delta": {"type": "input_json_delta", "partial_json": json.dumps(tc["arguments"])}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": cb_index})
                cb_index += 1

            if cb_index == 0:
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})

            stop_reason = "tool_use" if tool_calls else "end_turn"
            self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
            self._write_sse("message_stop", {"type": "message_stop"})
            self.wfile.flush()
            return

        try:
            buffer = ""
            chunk_count = 0
            last_data_time = time.time()
            ollama_timeout = 120
            read_was_timeout = False
            while True:
                try:
                    chunk = resp.read(4096)
                except (socket.timeout, OSError) as read_err:
                    self._log_proxy(f"STREAM: read error after {chunk_count} chunks: {read_err}")
                    read_was_timeout = isinstance(read_err, socket.timeout)
                    break
                if not chunk:
                    self._log_proxy(f"STREAM: chunk empty after {chunk_count} chunks, connection closed by Ollama")
                    break
                chunk_count += 1
                last_data_time = time.time()
                buffer += chunk.decode("utf-8", errors="ignore")

                lines = buffer.split("\n")
                buffer = lines.pop() or ""

                for line in lines:
                    trimmed = line.strip()
                    if not trimmed:
                        continue

                    try:
                        ollama_chunk = json.loads(trimmed)
                    except:
                        self._log_proxy(f"STREAM: failed to parse chunk: {trimmed[:200]}")
                        continue

                    has_chunk_content = bool(ollama_chunk.get("message", {}).get("content"))
                    has_chunk_tools = bool(ollama_chunk.get("message", {}).get("tool_calls"))
                    is_done = ollama_chunk.get("done", False)
                    if is_done or has_chunk_content or has_chunk_tools or chunk_count <= 3:
                        self._log_proxy(f"STREAM chunk#{chunk_count}: content={has_chunk_content} tools={has_chunk_tools} done={is_done} eval_count={ollama_chunk.get('eval_count')} prompt_eval_count={ollama_chunk.get('prompt_eval_count')}")

                    if ollama_chunk.get("prompt_eval_count"):
                        input_tokens = ollama_chunk["prompt_eval_count"]
                    if ollama_chunk.get("eval_count"):
                        output_tokens = ollama_chunk["eval_count"]

                    if not sent_start:
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.send_header("Connection", "close")
                        self.end_headers()
                        self._write_sse("message_start", {
                            "type": "message_start",
                            "message": {
                                "id": msg_id, "type": "message", "role": "assistant",
                                "content": [], "model": self.proxy_model,
                                "stop_reason": None, "stop_sequence": None,
                                "usage": {"input_tokens": input_tokens, "output_tokens": 0},
                            },
                        })
                        self._write_sse("content_block_start", {
                            "type": "content_block_start", "index": 0,
                            "content_block": {"type": "text", "text": ""},
                        })
                        text_block_started = True
                        sent_start = True

                    if not ollama_chunk.get("message", {}).get("content") and not ollama_chunk.get("message", {}).get("tool_calls") and not ollama_chunk.get("done"):
                        try:
                            self.wfile.write(b": heartbeat\n\n")
                            self.wfile.flush()
                        except:
                            pass

                    if ollama_chunk.get("message", {}).get("content"):
                        text_content = ollama_chunk["message"]["content"]
                        if not has_tool_calls:
                            full_content += text_content
                            self._write_sse("content_block_delta", {
                                "type": "content_block_delta", "index": 0,
                                "delta": {"type": "text_delta", "text": text_content},
                            })

                    if ollama_chunk.get("message", {}).get("tool_calls") and len(ollama_chunk["message"]["tool_calls"]) > 0:
                        if text_block_started and not text_block_closed and not has_tool_calls:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                            text_block_closed = True
                            content_block_index = 1
                        elif not text_block_started:
                            content_block_index = 0

                        has_tool_calls = True

                        for tc in ollama_chunk["message"]["tool_calls"]:
                            tool_use_id = f"toolu_{int(time.time())}_{current_tool_use_index}"
                            tool_name = (tc.get("function", {}) or {}).get("name", "unknown")
                            tool_input = (tc.get("function", {}) or {}).get("arguments", {})

                            self._write_sse("content_block_start", {
                                "type": "content_block_start",
                                "index": content_block_index,
                                "content_block": {
                                    "type": "tool_use",
                                    "id": tool_use_id,
                                    "name": tool_name,
                                    "input": {},
                                },
                            })
                            self._write_sse("content_block_delta", {
                                "type": "content_block_delta",
                                "index": content_block_index,
                                "delta": {
                                    "type": "input_json_delta",
                                    "partial_json": json.dumps(tool_input),
                                },
                            })
                            self._write_sse("content_block_stop", {
                                "type": "content_block_stop", "index": content_block_index,
                            })

                            content_block_index += 1
                            current_tool_use_index += 1

                    if ollama_chunk.get("done"):
                        self._log_proxy(f"STREAM: ollama done, has_tool_calls={has_tool_calls} text_block_started={text_block_started} text_block_closed={text_block_closed}")
                        if not has_tool_calls and text_block_started and not text_block_closed:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})

                        done_reason = ollama_chunk.get("done_reason", "")
                        stop_reason = "tool_use" if has_tool_calls else ("max_tokens" if done_reason == "length" else "end_turn")

                        self._write_sse("message_delta", {
                            "type": "message_delta",
                            "delta": {"stop_reason": stop_reason, "stop_sequence": None},
                            "usage": {"output_tokens": output_tokens or len(full_content)},
                        })
                        self._write_sse("message_stop", {"type": "message_stop"})
                        self.wfile.flush()
                        return

        except Exception as stream_err:
            self._log_proxy(f"STREAM ERROR: sent_start={sent_start} text_block_started={text_block_started} err={stream_err}")
            try:
                if sent_start:
                    if not text_block_started and not has_tool_calls:
                        self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                        if full_content:
                            self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": full_content}})
                        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                    elif text_block_started and not text_block_closed:
                        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                    self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
                    self._write_sse("message_stop", {"type": "message_stop"})
                else:
                    self._send_empty_message(msg_id)
                self.wfile.flush()
            except:
                pass
            return

        self._log_proxy(f"STREAM END: sent_start={sent_start} text_block_started={text_block_started} text_block_closed={text_block_closed} has_tool_calls={has_tool_calls} text_len={len(full_content)} chunks={chunk_count} timeout={read_was_timeout}")
        if sent_start:
            try:
                if not text_block_started and not has_tool_calls:
                    self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                    timeout_msg = ""
                    if read_was_timeout and not full_content:
                        timeout_msg = "\n\n[代理超时] Ollama未在90秒内返回数据，对话可能因上下文过长而中断。请尝试开启新对话。"
                    if full_content or timeout_msg:
                        self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": full_content + timeout_msg}})
                    self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                elif text_block_started and not text_block_closed:
                    self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
                self._write_sse("message_stop", {"type": "message_stop"})
                self.wfile.flush()
                self._log_proxy("STREAM: Complete end events sent successfully")
            except Exception as e:
                self._log_proxy(f"STREAM END ERROR: {e}")
        else:
            self._send_empty_message(msg_id)

    def _handle_non_stream_response(self, resp, is_retry=False):
        resp_data = resp.read().decode("utf-8", errors="ignore")
        full_content = ""
        input_tokens = 0
        output_tokens = 0
        tool_calls = []

        for line in resp_data.split("\n"):
            trimmed = line.strip()
            if not trimmed:
                continue
            try:
                chunk = json.loads(trimmed)
                if chunk.get("message", {}).get("content"):
                    full_content += chunk["message"]["content"]
                if chunk.get("message", {}).get("tool_calls"):
                    for tc in chunk["message"]["tool_calls"]:
                        tool_calls.append(tc)
                if chunk.get("prompt_eval_count"):
                    input_tokens = chunk["prompt_eval_count"]
                if chunk.get("eval_count"):
                    output_tokens = chunk["eval_count"]
            except:
                pass

        if is_retry and not tool_calls and full_content:
            parsed_calls = self._parse_tool_calls_from_text(full_content)
            for pc in parsed_calls:
                tool_calls.append({"function": {"name": pc["name"], "arguments": pc["arguments"]}})
            if tool_calls:
                full_content = self._remove_tool_call_blocks_from_text(full_content)

        content = []
        if full_content:
            content.append({"type": "text", "text": full_content})

        tool_use_index = 0
        for tc in tool_calls:
            func = tc.get("function", {})
            args = func.get("arguments", {})
            if isinstance(args, str):
                try:
                    args = json.loads(args)
                except:
                    args = {}
            content.append({
                "type": "tool_use",
                "id": f"toolu_{int(time.time())}_{tool_use_index}",
                "name": func.get("name", "unknown"),
                "input": args,
            })
            tool_use_index += 1

        if not content:
            content.append({"type": "text", "text": ""})

        stop_reason = "tool_use" if tool_calls else "end_turn"

        response = {
            "id": f"msg_{int(time.time())}",
            "type": "message",
            "role": "assistant",
            "content": content,
            "model": self.proxy_model,
            "stop_reason": stop_reason,
            "stop_sequence": None,
            "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
        }

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def _parse_tool_calls_from_text(self, text):
        calls = []
        seen = set()
        patterns = [
            r'```(?:json)?\s*\n?([\s\S]*?)\n?```',
            r'\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\s*\}',
        ]
        for pattern in patterns:
            for match in re.finditer(pattern, text):
                json_str = match.group(1) if match.lastindex and match.lastindex >= 1 else match.group(0)
                try:
                    parsed = json.loads(json_str)
                    if isinstance(parsed, dict) and parsed.get("name") and isinstance(parsed.get("name"), str):
                        key = parsed["name"] + ":" + json.dumps(parsed.get("arguments", {}), sort_keys=True)
                        if key not in seen:
                            seen.add(key)
                            calls.append({"name": parsed["name"], "arguments": parsed.get("arguments", {})})
                except:
                    if match.lastindex and match.lastindex >= 1:
                        try:
                            inner = json.loads(match.group(1))
                            if isinstance(inner, dict) and inner.get("name") and isinstance(inner.get("name"), str):
                                key = inner["name"] + ":" + json.dumps(inner.get("arguments", {}), sort_keys=True)
                                if key not in seen:
                                    seen.add(key)
                                    calls.append({"name": inner["name"], "arguments": inner.get("arguments", {})})
                        except:
                            pass
        return calls

    def _remove_tool_call_blocks_from_text(self, text):
        result = re.sub(r'```(?:json)?\s*\n?[\s\S]*?\n?```', lambda m: "" if self._is_tool_call_json(m.group(0)) else m.group(0), text)
        result = re.sub(r'\{\s*"name"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:\s*\{[^}]*\}\s*\}', "", result)
        result = re.sub(r'\n{3,}', '\n\n', result).strip()
        return result

    def _is_tool_call_json(self, block):
        inner = re.sub(r'^```(?:json)?\s*\n?', '', block)
        inner = re.sub(r'\n?```$', '', inner)
        try:
            parsed = json.loads(inner)
            return isinstance(parsed, dict) and parsed.get("name") and parsed.get("arguments")
        except:
            return False

    def _send_empty_message(self, msg_id):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        self._write_sse("message_start", {
            "type": "message_start",
            "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [],
                        "model": self.proxy_model, "stop_reason": None, "stop_sequence": None,
                        "usage": {"input_tokens": 0, "output_tokens": 0}},
        })
        self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
        self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 0}})
        self._write_sse("message_stop", {"type": "message_stop"})

    def _write_sse(self, event: str, data: dict):
        payload = f"event: {event}\ndata: {json.dumps(data)}\n\n".encode()
        try:
            self.wfile.write(payload)
            self.wfile.flush()
            if event in ("message_start", "content_block_start", "content_block_stop", "message_delta", "message_stop"):
                self._log_proxy(f"SSE SENT: event={event} payload_len={len(payload)}")
        except Exception as e:
            self._log_proxy(f"SSE WRITE ERROR: event={event} err={e}")
            raise


# ── Ollama 代理服务器管理 ──
class OllamaProxyServer:
    """管理 Ollama 代理的生命周期"""

    def __init__(self):
        self.server = None
        self.port = 0
        self.thread = None
        self._target_base_url = ""
        self._model = ""

    def start(self, target_base_url: str, model: str, log_dir: str = None) -> int:
        self._target_base_url = target_base_url
        self._model = model or "qwen3:8b"

        if log_dir:
            OllamaProxyHandler._proxy_log_dir = log_dir
            OllamaProxyHandler._proxy_log_file = None

        if self.server and self.thread and self.thread.is_alive():
            OllamaProxyHandler.target_url = urllib.parse.urlparse(target_base_url)
            OllamaProxyHandler.proxy_model = self._model
            return self.port

        if self.server:
            try:
                self.server.server_close()
            except:
                pass
            self.server = None

        target_url = urllib.parse.urlparse(target_base_url)
        OllamaProxyHandler.target_url = target_url
        OllamaProxyHandler.proxy_model = self._model

        self.server = socketserver.TCPServer(("127.0.0.1", 0), OllamaProxyHandler)
        self.port = self.server.server_address[1]
        self.server.timeout = 1

        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()
        return self.port

    def _serve(self):
        while self.server:
            try:
                self.server.handle_request()
            except:
                if not self.server:
                    break

    def stop(self):
        if self.server:
            try:
                self.server.server_close()
            except:
                pass
            self.server = None
            self.port = 0


# ── Claude CLI 子进程管理 ──
class ClaudeCliRunner:
    """管理 claude-code-tudou CLI 子进程"""

    def __init__(self, project_root: str, node_dir: str, bun_dir: str):
        self.project_root = project_root
        self.node_dir = node_dir
        self.bun_dir = bun_dir
        self.cli_entry = os.path.join(project_root, "bin", "claude-code-tudou")

    def _find_node(self) -> str:
        local = os.path.join(self.node_dir, "node.exe")
        if os.path.exists(local):
            return local
        return "node"

    def _build_args(self, session_id: str, model: str, is_resuming: bool,
                    system_prompt: str = None) -> list:
        args = [
            "--env-file=.env",
            self.cli_entry,
            "-p",
            "--output-format", "stream-json",
            "--include-partial-messages",
            "--verbose",
            "--max-turns", "3",
        ]
        if is_resuming:
            args.extend(["--resume", session_id])
        else:
            args.extend(["--session-id", session_id])
        if model and model.strip():
            args.extend(["--model", model.strip()])
        if system_prompt and system_prompt.strip():
            args.extend(["--system-prompt", system_prompt.strip()])
        return args

    def run(self, prompt: str, session_id: str, model: str, is_resuming: bool,
            workspace_path: str, env_overrides: dict = None,
            on_delta: Callable = None, on_status: Callable = None,
            on_log: Callable = None, on_proc: Callable = None,
            system_prompt: str = None) -> dict:
        """运行 CLI，返回结果"""
        node_path = self._find_node()
        args = self._build_args(session_id, model, is_resuming, system_prompt)

        env = dict(os.environ)
        if os.path.exists(self.node_dir):
            env["PATH"] = self.node_dir + ";" + env.get("PATH", "")
        if os.path.exists(self.bun_dir):
            env["PATH"] = self.bun_dir + ";" + env.get("PATH", "")
        if env_overrides:
            env.update(env_overrides)

        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW

        def _log(msg, color="#888"):
            if on_log:
                on_log(msg, color)

        _log(f"[CLI] 启动: node_path={node_path} entry_exists={os.path.exists(self.cli_entry)}")
        _log(f"[CLI] MODEL_PROVIDER={env.get('MODEL_PROVIDER')} API_BASE_URL={env.get('API_BASE_URL')} OLLAMA_BASE_URL={env.get('OLLAMA_BASE_URL')} API_MODEL={env.get('API_MODEL')} OLLAMA_MODEL={env.get('OLLAMA_MODEL')}")

        _log_path = os.path.join(self.project_root, "cli_debug.log")
        _log_file = open(_log_path, "a", encoding="utf-8")
        _log_file.write(f"=== CLI Debug Log {time.strftime('%Y-%m-%d %H:%M:%S')} ===\n")
        _log_file.write(f"node_path={node_path}\n")
        _log_file.write(f"args={[node_path] + args}\n")
        _log_file.write(f"cwd={workspace_path or self.project_root}\n")
        _log_file.write(f"env_overrides={env_overrides}\n")
        _log_file.write(f"MODEL_PROVIDER={env.get('MODEL_PROVIDER')}\n")
        _log_file.write(f"ANTHROPIC_BASE_URL={env.get('ANTHROPIC_BASE_URL')}\n")
        _log_file.write(f"ANTHROPIC_MODEL={env.get('ANTHROPIC_MODEL')}\n")
        _log_file.write(f"node_exists={os.path.exists(node_path)}\n")
        _log_file.write(f"cli_entry_exists={os.path.exists(self.cli_entry)}\n")
        _log_file.flush()

        try:
            proc = subprocess.Popen(
                [node_path] + args,
                cwd=workspace_path or self.project_root,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=env,
                startupinfo=si,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )

            _log_file.write(f"proc_pid={proc.pid}\n")
            _log_file.flush()
            _log(f"[CLI] 进程已启动 pid={proc.pid}")

            if on_proc:
                on_proc(proc)

            proc.stdin.write(prompt)
            proc.stdin.close()

            cli_timeout = 300
            timed_out = False

            def _timeout_watcher():
                nonlocal timed_out
                time.sleep(cli_timeout)
                if proc.poll() is None:
                    timed_out = True
                    try:
                        proc.kill()
                    except:
                        pass

            timeout_thread = threading.Thread(target=_timeout_watcher, daemon=True)
            timeout_thread.start()

            stderr_lines = []
            def _read_stderr():
                try:
                    for line in proc.stderr:
                        stderr_lines.append(line)
                except:
                    pass

            stderr_thread = threading.Thread(target=_read_stderr, daemon=True)
            stderr_thread.start()

            last_text = ""
            last_result = ""
            stdout_log = ""
            cli_session_id = ""
            has_stream_delta = False
            line_count = 0
            event_types = []
            idle_count = 0
            max_idle_after_exit = 30
            proc_exited = False

            stdout_queue = queue.Queue()

            def _read_stdout():
                try:
                    while True:
                        line = proc.stdout.readline()
                        if not line:
                            break
                        stdout_queue.put(line)
                except:
                    pass
                finally:
                    stdout_queue.put(None)

            stdout_reader = threading.Thread(target=_read_stdout, daemon=True)
            stdout_reader.start()

            while True:
                try:
                    line = stdout_queue.get(timeout=0.5)
                except queue.Empty:
                    if proc.poll() is not None:
                        proc_exited = True
                        idle_count += 1
                        if idle_count > max_idle_after_exit:
                            _log_file.write(f"  [TIMEOUT] stdout pipe timeout after process exit (pid={proc.pid} rc={proc.returncode}), breaking\n")
                            _log_file.flush()
                            try:
                                proc.kill()
                            except:
                                pass
                            break
                    continue

                if line is None:
                    break

                if not line:
                    if proc.poll() is not None:
                        proc_exited = True
                        idle_count += 1
                        if idle_count > max_idle_after_exit:
                            _log_file.write(f"  [TIMEOUT] stdout pipe still open after process exit (pid={proc.pid} rc={proc.returncode}), breaking\n")
                            _log_file.flush()
                            try:
                                proc.kill()
                            except:
                                pass
                            break
                        time.sleep(0.1)
                        continue
                    time.sleep(0.05)
                    continue
                idle_count = 0
                trimmed = line.strip()
                if not trimmed:
                    continue
                line_count += 1
                stdout_log += trimmed + "\n"
                try:
                    parsed = json.loads(trimmed)
                except:
                    continue

                evt_type = parsed.get("type", "?")
                if evt_type not in event_types:
                    event_types.append(evt_type)
                    _log_file.write(f"  [NEW-TYPE] {evt_type}: {trimmed[:200]}\n")
                    _log_file.flush()
                    _log(f"[CLI] 事件类型: {evt_type}")

                if evt_type == "system" and parsed.get("subtype") == "init":
                    cli_session_id = parsed.get("session_id", "")
                    if cli_session_id:
                        _log(f"[CLI] 捕获session_id={cli_session_id}")
                        _log_file.write(f"  [SESSION] cli_session_id={cli_session_id}\n")
                        _log_file.flush()

                if evt_type == "stream_event":
                    sub_type = parsed.get("event", {}).get("type", "?")
                    delta_type = parsed.get("event", {}).get("delta", {}).get("type", "?")
                    if f"stream_event.{sub_type}.{delta_type}" not in event_types:
                        event_types.append(f"stream_event.{sub_type}.{delta_type}")
                        _log_file.write(f"  [STREAM-EVT] {sub_type}.{delta_type}\n")
                        _log_file.flush()

                # 流式文本 delta
                if (parsed.get("type") == "stream_event" and
                    parsed.get("event", {}).get("type") == "content_block_delta" and
                    parsed.get("event", {}).get("delta", {}).get("type") == "text_delta"):
                    text = parsed["event"]["delta"].get("text", "")
                    if text and on_delta:
                        on_delta(text)
                        has_stream_delta = True

                # assistant 消息
                if parsed.get("type") == "assistant":
                    msg = parsed.get("message", {})
                    if isinstance(msg.get("content"), list):
                        parts = [b.get("text", "") for b in msg["content"] if b.get("type") == "text" and isinstance(b.get("text"), str)]
                        if parts:
                            last_text = "\n".join(parts)
                            if not has_stream_delta and on_delta:
                                on_delta(last_text)
                                _log(f"[CLI] 从assistant消息补充文本 len={len(last_text)}", "#FF9800")

                # result
                if parsed.get("type") == "result" and isinstance(parsed.get("result"), str):
                    last_result = parsed["result"]
                    if not has_stream_delta and not last_text and on_delta:
                        on_delta(last_result)
                        has_stream_delta = True
                        _log(f"[CLI] 从result补充文本 len={len(last_result)}", "#FF9800")

            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                try:
                    proc.kill()
                except:
                    pass
                proc.wait(timeout=5)
            stderr_thread.join(timeout=5)
            stderr_out = "".join(stderr_lines)

            _log_file.write(f"\n=== RESULT ===\n")
            _log_file.write(f"lines={line_count} types={event_types} has_delta={has_stream_delta} last_text_len={len(last_text)} rc={proc.returncode}\n")
            if stderr_out:
                _log_file.write(f"stderr={stderr_out[:2000]}\n")
            if not has_stream_delta and not last_text and stdout_log:
                _log_file.write(f"stdout_sample={stdout_log[:3000]}\n")
            _log_file.flush()
            _log_file.close()

            _log(f"[CLI] 完成: lines={line_count} types={event_types} has_delta={has_stream_delta} rc={proc.returncode}",
                 "#4CAF50" if proc.returncode == 0 else "#F44336")
            if stderr_out and proc.returncode != 0:
                _log(f"[CLI] stderr: {stderr_out[:300]}", "#F44336")

            if proc.returncode == 0:
                return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
            else:
                if timed_out:
                    return {"ok": False, "error": f"CLI执行超时({cli_timeout}秒)，已自动终止", "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
                if has_stream_delta:
                    _log(f"[CLI] rc={proc.returncode} 但已收到流式文本(has_delta=True)，视为成功", "#FF9800")
                    return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
                if last_text.strip():
                    _log(f"[CLI] rc={proc.returncode} 但已收到文本内容(len={len(last_text.strip())})，视为成功", "#FF9800")
                    return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
                if last_result.strip():
                    _log(f"[CLI] rc={proc.returncode} 但已收到result内容(len={len(last_result.strip())})，视为成功", "#FF9800")
                    return {"ok": True, "text": last_result.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
                error = stderr_out.strip() or "Unknown CLI error."
                fallback = ""
                if error == "Unknown CLI error.":
                    for raw_line in stdout_log.split("\n"):
                        raw_line = raw_line.strip()
                        if not raw_line:
                            continue
                        try:
                            pj = json.loads(raw_line)
                            if pj.get("type") == "result" and pj.get("subtype") == "error":
                                fallback = pj.get("result", "") or pj.get("error", "")
                                if fallback:
                                    break
                            if pj.get("type") == "error":
                                fallback = pj.get("error", {}).get("message", "") if isinstance(pj.get("error"), dict) else str(pj.get("error", ""))
                                if fallback:
                                    break
                        except:
                            pass
                    if not fallback:
                        fallback = f"CLI异常退出(rc={proc.returncode})"
                return {"ok": False, "error": fallback or error, "sessionId": session_id, "cliSessionId": cli_session_id or session_id}

        except Exception as e:
            return {"ok": False, "error": str(e), "sessionId": session_id, "cliSessionId": cli_session_id if 'cli_session_id' in dir() else session_id}


# ── QWebChannel 桥接对象 ──
# 这个类在 main.py 中定义，因为需要访问 PyQt6 的 QWebChannel
# 这里只定义接口规范

BRIDGE_METHODS = [
    "getState", "newSession", "sendMessage", "stopMessage",
    "getWorkspace", "chooseWorkspace", "getSettings", "saveSettings",
    "clearModelSettings", "listModels", "detectHardware",
    "deleteModel", "recommendModels", "fetchApiKey",
    "startQwen2Api", "checkApiService",
    "listQwenAccounts", "deleteQwenAccount",
    "startQwenLogin", "pollQwenLogin",
    "startQwenRegister", "pollQwenRegister",
    "addQwenAccount",
]

BRIDGE_SIGNALS = [
    "deltaReceived",   # chat:delta -> {text: str}
    "statusReceived",  # chat:status -> {busy: bool, ...}
]
