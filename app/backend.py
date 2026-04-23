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
import subprocess
import threading
import queue
import http.server
import socketserver
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


# ── Ollama 代理服务器 ──
class OllamaProxyHandler(http.server.BaseHTTPRequestHandler):
    """将 Anthropic Messages API 转换为 OpenAI Chat Completions API（通过 Ollama /v1/chat/completions）"""

    target_url = None
    proxy_model = "qwen3:8b"

    def log_message(self, format, *args):
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

        openai_messages = self._convert_messages(anthropic_body)
        openai_tools = self._convert_tools(anthropic_body)

        tool_result_count = sum(
            1 for msg in anthropic_body.get("messages", [])
            if msg.get("role") == "user" and isinstance(msg.get("content"), list)
            and any(b.get("type") == "tool_result" for b in msg.get("content", []))
        )
        if tool_result_count >= 3:
            openai_tools = None

        stream = anthropic_body.get("stream") is True
        openai_body = {
            "model": self.proxy_model,
            "messages": openai_messages,
            "stream": stream,
        }

        env_temp = os.environ.get("AI_TEMPERATURE", "").strip()
        env_max_tokens = os.environ.get("AI_MAX_TOKENS", "").strip()
        if env_temp:
            try:
                openai_body["temperature"] = float(env_temp)
            except ValueError:
                pass
        if env_max_tokens:
            try:
                openai_body["max_tokens"] = int(env_max_tokens)
            except ValueError:
                pass
        if openai_tools:
            openai_body["tools"] = openai_tools

        self._send_to_ollama(openai_body, stream)

    def _convert_messages(self, body: dict) -> list:
        openai_msgs = []

        if isinstance(body.get("system"), str) and body["system"].strip():
            openai_msgs.append({"role": "system", "content": body["system"]})
        elif isinstance(body.get("system"), list):
            sys_text = "\n".join(
                b.get("text", "") for b in body["system"] if b.get("type") == "text"
            )
            if sys_text.strip():
                openai_msgs.append({"role": "system", "content": sys_text})
        else:
            lang = os.environ.get("AI_LANGUAGE", "zh").strip().lower()
            if lang == "zh":
                openai_msgs.append({"role": "system", "content": "你是一个专业的AI编程助手。请始终使用中文回答。只有在用户明确要求执行编程任务时才使用工具。普通对话请直接回答。"})
            elif lang == "ja":
                openai_msgs.append({"role": "system", "content": "あなたはプロのAIプログラミングアシスタントです。日本語で回答してください。"})
            elif lang == "ko":
                openai_msgs.append({"role": "system", "content": "당신은 전문 AI 프로그래밍 어시스턴트입니다. 한국어로 답변해 주세요."})
            elif lang and lang != "en":
                openai_msgs.append({"role": "system", "content": f"You are a professional AI coding assistant. Please respond in {lang}."})

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
                    openai_msgs.append({
                        "role": "tool",
                        "tool_call_id": tr.get("tool_use_id", "unknown"),
                        "content": c or "(no output)",
                    })
                text = "\n".join(p for p in text_parts if p)
                if text:
                    openai_msgs.append({"role": "user", "content": text})

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
                assistant_msg = {"role": "assistant", "content": text or None}
                if tool_calls:
                    openai_tc = []
                    for tc in tool_calls:
                        args = tc.get("input", {})
                        if isinstance(args, dict):
                            args = json.dumps(args)
                        elif not isinstance(args, str):
                            args = json.dumps({})
                        openai_tc.append({
                            "id": tc.get("id", f"call_{len(openai_tc)}"),
                            "type": "function",
                            "function": {
                                "name": tc.get("name", ""),
                                "arguments": args,
                            }
                        })
                    assistant_msg["tool_calls"] = openai_tc
                openai_msgs.append(assistant_msg)

        return openai_msgs

    def _convert_tools(self, body: dict) -> list:
        openai_tools = []
        for tool in body.get("tools", []):
            if tool.get("type") == "custom":
                continue
            openai_tools.append({
                "type": "function",
                "function": {
                    "name": tool.get("name", ""),
                    "description": tool.get("description", ""),
                    "parameters": tool.get("input_schema", {"type": "object", "properties": {}}),
                },
            })
        return openai_tools if openai_tools else None

    def _send_to_ollama(self, body_obj: dict, stream: bool):
        body_str = json.dumps(body_obj).encode("utf-8")
        target = self.target_url

        req = urllib.request.Request(
            f"http://{target.hostname}:{target.port or 11434}/v1/chat/completions",
            data=body_str,
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        try:
            resp = urllib.request.urlopen(req, timeout=300)
        except urllib.error.HTTPError as e:
            err_body = ""
            try:
                err_body = e.read().decode("utf-8")
            except:
                pass
            if e.code == 400 and "does not support tools" in err_body and "tools" in body_obj:
                fallback_body = {k: v for k, v in body_obj.items() if k != "tools"}
                fallback_body_str = json.dumps(fallback_body).encode("utf-8")
                fallback_req = urllib.request.Request(
                    f"http://{target.hostname}:{target.port or 11434}/v1/chat/completions",
                    data=fallback_body_str,
                    headers={"Content-Type": "application/json"},
                    method="POST",
                )
                try:
                    resp = urllib.request.urlopen(fallback_req, timeout=300)
                except urllib.error.HTTPError as e2:
                    err_body2 = ""
                    try:
                        err_body2 = e2.read().decode("utf-8")
                    except:
                        pass
                    self.send_response(e2.code)
                    self.send_header("Content-Type", "application/json")
                    self.end_headers()
                    self.wfile.write(json.dumps({
                        "type": "error",
                        "error": {"type": "invalid_request_error", "message": f"Ollama error ({e2.code}): {err_body2}"},
                    }).encode())
                    return
                except Exception:
                    self.send_response(502)
                    self.send_header("Content-Type", "application/json")
                    self.end_headers()
                    self.wfile.write(json.dumps({
                        "type": "error",
                        "error": {"type": "api_error", "message": "Proxy error on retry without tools"},
                    }).encode())
                    return
            else:
                self.send_response(e.code)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({
                    "type": "error",
                    "error": {"type": "invalid_request_error", "message": f"Ollama error ({e.code}): {err_body}"},
                }).encode())
                return
        except Exception as e:
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({
                "type": "error",
                "error": {"type": "api_error", "message": f"Proxy error: {e}"},
            }).encode())
            return

        if stream:
            self._handle_stream_response(resp)
        else:
            self._handle_non_stream_response(resp)

    def _handle_stream_response(self, resp):
        msg_id = f"msg_{int(time.time())}"
        sent_start = False
        text_block_started = False
        text_block_closed = False
        tool_block_indices = {}
        next_content_index = 0
        prompt_tokens = 0
        completion_tokens = 0
        buffer = ""
        has_tool_calls = False
        text_content = ""

        while True:
            try:
                chunk = resp.read(4096)
                if not chunk:
                    break
                buffer += chunk.decode("utf-8", errors="ignore")

                while "\n" in buffer:
                    line, buffer = buffer.split("\n", 1)
                    line = line.strip()
                    if not line:
                        continue
                    if not line.startswith("data:"):
                        continue
                    data = line[5:].strip()
                    if data == "[DONE]":
                        if not sent_start:
                            self._send_empty_message(msg_id)
                        else:
                            if text_block_started and not text_block_closed:
                                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                            for oi, ai in tool_block_indices.items():
                                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": ai})
                            self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": completion_tokens}})
                            self._write_sse("message_stop", {"type": "message_stop"})
                        self.wfile.flush()
                        return

                    try:
                        parsed = json.loads(data)
                    except:
                        continue

                    if not parsed or not isinstance(parsed, dict):
                        continue

                    if parsed.get("usage"):
                        prompt_tokens = parsed["usage"].get("prompt_tokens", prompt_tokens)
                        completion_tokens = parsed["usage"].get("completion_tokens", completion_tokens)

                    choices = parsed.get("choices", [])
                    if not choices:
                        continue
                    choice = choices[0]
                    delta = choice.get("delta", {})
                    finish_reason = choice.get("finish_reason")

                    if not sent_start:
                        sent_start = True
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.end_headers()
                        self._write_sse("message_start", {
                            "type": "message_start",
                            "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [],
                                        "model": self.proxy_model, "stop_reason": None, "stop_sequence": None,
                                        "usage": {"input_tokens": prompt_tokens, "output_tokens": 0}},
                        })

                    if delta.get("tool_calls"):
                        has_tool_calls = True
                        if text_block_started and not text_block_closed:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                            text_block_closed = True

                    if delta.get("content") and not has_tool_calls:
                        text_content += delta["content"]
                        if not text_block_started:
                            text_block_started = True
                            text_idx = next_content_index
                            next_content_index += 1
                            self._write_sse("content_block_start", {"type": "content_block_start", "index": text_idx, "content_block": {"type": "text", "text": ""}})
                        self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": delta["content"]}})

                    for tc in delta.get("tool_calls", []):
                        oi = tc.get("index", 0)
                        if oi not in tool_block_indices:
                            ai = next_content_index
                            next_content_index += 1
                            tool_block_indices[oi] = ai
                            tc_id = tc.get("id", f"call_{oi}")
                            tc_name = (tc.get("function", {}) or {}).get("name", "")
                            self._write_sse("content_block_start", {"type": "content_block_start", "index": ai, "content_block": {"type": "tool_use", "id": tc_id, "name": tc_name, "input": ""}})
                        tc_args = (tc.get("function", {}) or {}).get("arguments", "")
                        if tc_args:
                            self._write_sse("content_block_delta", {"type": "content_block_delta", "index": tool_block_indices[oi], "delta": {"type": "input_json_delta", "partial_json": tc_args}})

                    if finish_reason:
                        if text_block_started and not text_block_closed:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                        for oi, ai in tool_block_indices.items():
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": ai})

                        stop_reason = "tool_use" if finish_reason == "tool_calls" else ("max_tokens" if finish_reason == "length" else "end_turn")
                        self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": None}, "usage": {"output_tokens": completion_tokens}})
                        self._write_sse("message_stop", {"type": "message_stop"})
                        self.wfile.flush()
                        return

            except:
                break

        if not sent_start:
            self._send_empty_message(msg_id)
        self.wfile.flush()

    def _handle_non_stream_response(self, resp):
        data = resp.read().decode("utf-8")
        try:
            result = json.loads(data)
        except:
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"type": "error", "error": {"type": "api_error", "message": "Invalid response from Ollama"}}).encode())
            return

        choice = (result.get("choices") or [{}])[0]
        message = choice.get("message", {})
        finish_reason = choice.get("finish_reason", "stop")
        content_text = message.get("content") or ""
        tool_calls = message.get("tool_calls", [])
        prompt_tokens = (result.get("usage") or {}).get("prompt_tokens", 0)
        completion_tokens = (result.get("usage") or {}).get("completion_tokens", 0)

        msg_id = f"msg_{int(time.time())}"
        content = []
        if content_text:
            content.append({"type": "text", "text": content_text})
        for tc in tool_calls:
            func = tc.get("function", {})
            args = func.get("arguments", "{}")
            if isinstance(args, str):
                try:
                    args = json.loads(args)
                except:
                    args = {}
            content.append({"type": "tool_use", "id": tc.get("id", f"toolu_{int(time.time())}"), "name": func.get("name", "unknown"), "input": args})
        if not content:
            content.append({"type": "text", "text": ""})

        stop_reason = "tool_use" if finish_reason == "tool_calls" else ("max_tokens" if finish_reason == "length" else "end_turn")

        response = {
            "id": msg_id,
            "type": "message",
            "role": "assistant",
            "content": content,
            "model": self.proxy_model,
            "stop_reason": stop_reason,
            "stop_sequence": None,
            "usage": {"input_tokens": prompt_tokens, "output_tokens": completion_tokens},
        }

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def _send_empty_message(self, msg_id):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
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
        self.wfile.write(f"event: {event}\ndata: {json.dumps(data)}\n\n".encode())


# ── Ollama 代理服务器管理 ──
class OllamaProxyServer:
    """管理 Ollama 代理的生命周期"""

    def __init__(self):
        self.server = None
        self.port = 0
        self.thread = None

    def start(self, target_base_url: str, model: str) -> int:
        if self.server:
            return self.port

        target_url = urllib.parse.urlparse(target_base_url)
        OllamaProxyHandler.target_url = target_url
        OllamaProxyHandler.proxy_model = model or "qwen3:8b"

        # 找可用端口
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
        _log(f"[CLI] MODEL_PROVIDER={env.get('MODEL_PROVIDER')} BASE_URL={env.get('ANTHROPIC_BASE_URL')} MODEL={env.get('ANTHROPIC_MODEL')}")

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
            has_stream_delta = False
            line_count = 0
            event_types = []
            idle_count = 0
            max_idle_after_exit = 30
            proc_exited = False

            stdout_queue = queue.Queue()

            def _read_stdout():
                try:
                    for line in proc.stdout:
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
                return {"ok": True, "text": last_text.strip(), "sessionId": session_id}
            else:
                if timed_out:
                    return {"ok": False, "error": f"CLI执行超时({cli_timeout}秒)，已自动终止", "sessionId": session_id}
                error = stderr_out.strip() or "Unknown CLI error."
                fallback = ""
                if error == "Unknown CLI error.":
                    fallback = last_result or last_text or stdout_log[:1200]
                return {"ok": False, "error": fallback or error, "sessionId": session_id}

        except Exception as e:
            return {"ok": False, "error": str(e), "sessionId": session_id}


# ── QWebChannel 桥接对象 ──
# 这个类在 main.py 中定义，因为需要访问 PyQt6 的 QWebChannel
# 这里只定义接口规范

BRIDGE_METHODS = [
    "getState", "newSession", "sendMessage", "stopMessage",
    "getWorkspace", "chooseWorkspace", "getSettings", "saveSettings",
    "clearModelSettings", "listModels", "detectHardware",
    "deleteModel", "recommendModels",
]

BRIDGE_SIGNALS = [
    "deltaReceived",   # chat:delta -> {text: str}
    "statusReceived",  # chat:status -> {busy: bool, ...}
]
