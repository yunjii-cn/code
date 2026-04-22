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


def list_ollama_models(base_url: str, timeout_ms: int = 15000) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:11434"
    url = f"{base.rstrip('/')}/api/tags"
    result = fetch_json_with_timeout(url, timeout_ms=timeout_ms)
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"Ollama API failed ({result['status']}) {err}".strip()}
    raw_models = ((result.get("data") or {}).get("models", []))[:120]

    # 并行查询 capabilities
    cap_results = [None] * len(raw_models)

    def _fetch_cap(idx, m):
        cap_results[idx] = fetch_ollama_capabilities(base, m.get("name", ""), timeout_ms)

    threads = []
    for i, m in enumerate(raw_models):
        t = threading.Thread(target=_fetch_cap, args=(i, m), daemon=True)
        t.start()
        threads.append(t)
    for t in threads:
        t.join(timeout=10)

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
        models.append({
            "id": m.get("name", ""),
            "name": m.get("name", ""),
            "provider": "ollama",
            "toolSupport": tool_support,
        })
    return {"ok": True, "models": models}


# ── Ollama 代理服务器 ──
class OllamaProxyHandler(http.server.BaseHTTPRequestHandler):
    """将 Anthropic Messages API 转换为 Ollama Chat API"""

    # 类级变量，由 start_ollama_proxy 设置
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

        # 转换消息
        ollama_messages, system_prompt = self._convert_messages(anthropic_body)
        ollama_tools = self._convert_tools(anthropic_body)

        stream = anthropic_body.get("stream") is True
        ollama_body = {
            "model": self.proxy_model,
            "messages": ollama_messages,
            "stream": stream,
            "options": {"num_ctx": 32768},
        }
        if system_prompt:
            ollama_body["system"] = system_prompt
        if ollama_tools:
            ollama_body["tools"] = ollama_tools

        self._send_to_ollama(ollama_body, ollama_tools, stream, is_retry=False)

    def _convert_messages(self, body: dict):
        ollama_msgs = []
        system_prompt = ""
        if isinstance(body.get("system"), str):
            system_prompt = body["system"]
        elif isinstance(body.get("system"), list):
            system_prompt = "\n".join(
                b.get("text", "") for b in body["system"] if b.get("type") == "text"
            )

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

                assistant_msg = {"role": "assistant", "content": ""}
                text = "\n".join(p for p in text_parts if p)
                if text:
                    assistant_msg["content"] = text
                ollama_tc = []
                for tc in tool_calls:
                    args = tc.get("input", {})
                    if isinstance(args, str):
                        try:
                            args = json.loads(args)
                        except:
                            args = {}
                    ollama_tc.append({"function": {"name": tc.get("name", ""), "arguments": args}})
                if ollama_tc:
                    assistant_msg["tool_calls"] = ollama_tc
                ollama_msgs.append(assistant_msg)

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
        return ollama_tools

    def _send_to_ollama(self, body_obj: dict, ollama_tools: list, stream: bool, is_retry: bool):
        """发送请求到 Ollama，支持自动降级重试"""
        body_str = json.dumps(body_obj).encode("utf-8")
        target = self.target_url

        req = urllib.request.Request(
            f"http://{target.hostname}:{target.port or 11434}/api/chat",
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

            # 检测 "does not support tools" → 自动去掉 tools 重试
            if not is_retry and "does not support tools" in err_body:
                retry_body = {**body_obj}
                retry_body.pop("tools", None)
                if ollama_tools:
                    tool_descs = "\n".join(
                        f"- {t['function']['name']}: {t['function'].get('description', '')} Params: {json.dumps(t['function'].get('parameters', {}))}"
                        for t in ollama_tools
                    )
                    BT = "`" * 3
                    tool_prompt = (
                        f"\n\nYou have access to the following tools. To call a tool, output a JSON block enclosed in {BT} tags like this:\n"
                        f"{BT}\n"
                        f'{{"name": "tool_name", "arguments": {{"param": "value"}}}}\n'
                        f"{BT}\n"
                        f"You can call multiple tools. After each tool call, wait for the result in the next user message enclosed in <tool_result> tags.\n"
                        f"Available tools:\n{tool_descs}"
                    )
                    retry_body["system"] = (retry_body.get("system") or "") + tool_prompt
                self._send_to_ollama(retry_body, ollama_tools, stream, is_retry=True)
                return

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
            self._handle_stream_response(resp, is_retry)
        else:
            self._handle_non_stream_response(resp, is_retry)

    def _handle_stream_response(self, resp, is_retry: bool):
        """处理流式响应"""
        msg_id = f"msg_{int(time.time())}"
        input_tokens = 0
        output_tokens = 0

        if is_retry:
            # 降级重试：先收集完整响应再解析
            chunks = []
            while True:
                try:
                    chunk = resp.read(4096)
                    if not chunk:
                        break
                    chunks.append(chunk)
                except:
                    break

            full_content = ""
            for chunk in chunks:
                for line in chunk.decode("utf-8", errors="ignore").split("\n"):
                    trimmed = line.strip()
                    if not trimmed:
                        continue
                    try:
                        parsed = json.loads(trimmed)
                        if parsed.get("prompt_eval_count"):
                            input_tokens = parsed["prompt_eval_count"]
                        if parsed.get("eval_count"):
                            output_tokens = parsed["eval_count"]
                        if parsed.get("message", {}).get("content"):
                            full_content += parsed["message"]["content"]
                    except:
                        pass

            tool_calls = _parse_tool_calls_from_text(full_content)
            clean_content = _remove_tool_call_blocks_from_text(full_content)

            # 发送转换后的 SSE
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()

            self._write_sse("message_start", {
                "type": "message_start",
                "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [],
                            "model": self.proxy_model, "stop_reason": None, "stop_sequence": None,
                            "usage": {"input_tokens": input_tokens, "output_tokens": 0}},
            })

            idx = 0
            if clean_content.strip():
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": clean_content}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                idx = 1

            for i, tc in enumerate(tool_calls):
                tool_id = f"toolu_{int(time.time())}_{i}"
                self._write_sse("content_block_start", {"type": "content_block_start", "index": idx, "content_block": {"type": "tool_use", "id": tool_id, "name": tc["name"], "input": {}}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": idx, "delta": {"type": "input_json_delta", "partial_json": json.dumps(tc.get("arguments", {}))}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": idx})
                idx += 1

            if idx == 0:
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})

            stop_reason = "tool_use" if tool_calls else "end_turn"
            self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
            self._write_sse("message_stop", {"type": "message_stop"})
            self.wfile.flush()
            return

        # ── 正常流式模式 ──
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()

        sent_start = False
        sent_text_block_start = False
        block_idx = 0
        tool_idx = 0
        has_tool_calls = False
        full_content = ""
        buffer = ""
        chunk_count = 0
        ollama_event_count = 0

        while True:
            try:
                chunk = resp.read(4096)
                if not chunk:
                    break
                chunk_count += 1
                buffer += chunk.decode("utf-8", errors="ignore")
                lines = buffer.split("\n")
                buffer = lines.pop() or ""

                for line in lines:
                    trimmed = line.strip()
                    if not trimmed:
                        continue
                    try:
                        oc = json.loads(trimmed)
                    except:
                        continue
                    ollama_event_count += 1

                    if oc.get("prompt_eval_count"):
                        input_tokens = oc["prompt_eval_count"]
                    if oc.get("eval_count"):
                        output_tokens = oc["eval_count"]

                    if not sent_start:
                        self._write_sse("message_start", {
                            "type": "message_start",
                            "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [],
                                        "model": self.proxy_model, "stop_reason": None, "stop_sequence": None,
                                        "usage": {"input_tokens": input_tokens, "output_tokens": 0}},
                        })
                        sent_start = True

                    msg = oc.get("message", {})
                    if msg.get("content") and not has_tool_calls:
                        full_content += msg["content"]
                        if not sent_text_block_start:
                            self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                            sent_text_block_start = True
                        self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": msg["content"]}})

                    if msg.get("tool_calls"):
                        if sent_text_block_start and not has_tool_calls:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                            block_idx = 1
                        has_tool_calls = True
                        for tc in msg["tool_calls"]:
                            tool_id = f"toolu_{int(time.time())}_{tool_idx}"
                            func = tc.get("function", {})
                            self._write_sse("content_block_start", {"type": "content_block_start", "index": block_idx, "content_block": {"type": "tool_use", "id": tool_id, "name": func.get("name", "unknown"), "input": {}}})
                            self._write_sse("content_block_delta", {"type": "content_block_delta", "index": block_idx, "delta": {"type": "input_json_delta", "partial_json": json.dumps(func.get("arguments", {}))}})
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": block_idx})
                            block_idx += 1
                            tool_idx += 1

                    if oc.get("done"):
                        if not has_tool_calls and sent_text_block_start:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                        stop_reason = "tool_use" if has_tool_calls else ("max_tokens" if oc.get("done_reason") == "length" else "end_turn")
                        self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
                        self._write_sse("message_stop", {"type": "message_stop"})

            except:
                break

        if not sent_start:
            self._write_sse("message_start", {"type": "message_start", "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [{"type": "text", "text": ""}], "model": self.proxy_model, "stop_reason": "end_turn", "stop_sequence": None, "usage": {"input_tokens": 0, "output_tokens": 0}}})
        self.wfile.flush()

    def _handle_non_stream_response(self, resp, is_retry: bool):
        """处理非流式响应"""
        data = resp.read().decode("utf-8")
        full_content = ""
        input_tokens = 0
        output_tokens = 0
        tool_calls_raw = []

        for line in data.split("\n"):
            trimmed = line.strip()
            if not trimmed:
                continue
            try:
                chunk = json.loads(trimmed)
                if chunk.get("message", {}).get("content"):
                    full_content += chunk["message"]["content"]
                if chunk.get("message", {}).get("tool_calls"):
                    tool_calls_raw.extend(chunk["message"]["tool_calls"])
                if chunk.get("prompt_eval_count"):
                    input_tokens = chunk["prompt_eval_count"]
                if chunk.get("eval_count"):
                    output_tokens = chunk["eval_count"]
            except:
                pass

        if is_retry and not tool_calls_raw and full_content:
            parsed = _parse_tool_calls_from_text(full_content)
            for pc in parsed:
                tool_calls_raw.append({"function": {"name": pc["name"], "arguments": pc.get("arguments", {})}})
            if tool_calls_raw:
                full_content = _remove_tool_call_blocks_from_text(full_content)

        content = []
        if full_content:
            content.append({"type": "text", "text": full_content})
        for i, tc in enumerate(tool_calls_raw):
            func = tc.get("function", {})
            content.append({"type": "tool_use", "id": f"toolu_{int(time.time())}_{i}", "name": func.get("name", "unknown"), "input": func.get("arguments", {})})
        if not content:
            content.append({"type": "text", "text": ""})

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({
            "id": f"msg_{int(time.time())}", "type": "message", "role": "assistant", "content": content,
            "model": self.proxy_model, "stop_reason": "tool_use" if tool_calls_raw else "end_turn",
            "stop_sequence": None, "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
        }).encode())

    def _write_sse(self, event: str, data: dict):
        self.wfile.write(f"event: {event}\ndata: {json.dumps(data)}\n\n".encode())


# ── 工具调用解析（降级重试模式） ──
def _parse_tool_calls_from_text(text: str) -> list:
    calls = []
    seen = set()
    # 代码块中的 JSON
    for match in re.finditer(r"```(?:json)?\s*\n?([\s\S]*?)\n?```", text):
        try:
            parsed = json.loads(match.group(1))
            if parsed.get("name") and isinstance(parsed["name"], str):
                key = f"{parsed['name']}:{json.dumps(parsed.get('arguments', {}))}"
                if key not in seen:
                    seen.add(key)
                    calls.append({"name": parsed["name"], "arguments": parsed.get("arguments", {})})
        except:
            pass
    # 独立的 JSON 对象
    for match in re.finditer(r'\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\s*\}', text):
        name = match.group(1)
        try:
            args = json.loads(match.group(2))
            key = f"{name}:{json.dumps(args)}"
            if key not in seen:
                seen.add(key)
                calls.append({"name": name, "arguments": args})
        except:
            pass
    return calls


def _remove_tool_call_blocks_from_text(text: str) -> str:
    result = re.sub(r"```(?:json)?\s*\n?[\s\S]*?\n?```", lambda m: _try_remove_tool_block(m), text)
    result = re.sub(r'\{\s*"name"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:\s*\{[^}]*\}\s*\}', "", result)
    result = re.sub(r"\n{3,}", "\n\n", result).strip()
    return result


def _try_remove_tool_block(match) -> str:
    inner = re.sub(r"^```(?:json)?\s*\n?", "", match.group(0))
    inner = re.sub(r"\n?```$", "", inner)
    try:
        parsed = json.loads(inner)
        if parsed.get("name") and parsed.get("arguments"):
            return ""
    except:
        pass
    return match.group(0)


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

    def _build_args(self, session_id: str, model: str, is_resuming: bool) -> list:
        args = [
            "--env-file=.env",
            self.cli_entry,
            "-p",
            "--output-format", "stream-json",
            "--include-partial-messages",
            "--verbose",
        ]
        if is_resuming:
            args.extend(["--resume", session_id])
        else:
            args.extend(["--session-id", session_id])
        if model and model.strip():
            args.extend(["--model", model.strip()])
        return args

    def run(self, prompt: str, session_id: str, model: str, is_resuming: bool,
            workspace_path: str, env_overrides: dict = None,
            on_delta: Callable = None, on_status: Callable = None,
            on_log: Callable = None, on_proc: Callable = None) -> dict:
        """运行 CLI，返回结果"""
        node_path = self._find_node()
        args = self._build_args(session_id, model, is_resuming)

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
            max_idle_after_exit = 50

            while True:
                line = proc.stdout.readline()
                if not line:
                    if proc.poll() is not None:
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
    "clearModelSettings", "listModels",
]

BRIDGE_SIGNALS = [
    "deltaReceived",   # chat:delta -> {text: str}
    "statusReceived",  # chat:status -> {busy: bool, ...}
]
