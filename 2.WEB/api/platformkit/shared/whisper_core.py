"""Platform Shared - Whisper 云端转录核心

通过 multipart/form-data POST 调用 OpenAI 兼容的 /audio/transcriptions 端点
（如 OpenAI Whisper API、自托管兼容服务）。

**设计原则**:
    - 不依赖 FastAPI（路由层做参数解析 + 调本模块）
    - 不依赖第三方库（仅 stdlib urllib + uuid）
    - 统一返回 dict: {"ok": bool, "text": str, "error": str|None, "status": int}

**使用规则**:
    - ✅ 路由层: `from platformkit.shared import whisper_core`
    - ❌ 禁止: 在 routes/*.py 中直接构造 multipart

**支持的供应商**:
    - OpenAI (api.openai.com/v1/audio/transcriptions, model=whisper-1)
    - 自托管 OpenAI 兼容服务（Ollama whisper、本地 faster-whisper server 等）
    - 注: Ollama 官方不暴露 Whisper API，但部分第三方实现支持
"""
from __future__ import annotations

import json
import urllib.error
import urllib.request
import uuid
from typing import Optional


DEFAULT_MODEL = "whisper-1"
DEFAULT_TIMEOUT_S = 60  # 音频转录可能较慢


def _make_result(ok: bool, text: str = "", error: Optional[str] = None, status: int = 0, raw: str = "") -> dict:
    return {"ok": ok, "text": text, "error": error, "status": status, "raw": raw}


def _build_multipart(
    file_bytes: bytes,
    filename: str,
    mime_type: str,
    fields: dict,
) -> tuple[bytes, str]:
    """构造 multipart/form-data 字节内容 + Content-Type header 值。

    Args:
        file_bytes: 文件二进制
        filename: 文件名（服务端日志识别用）
        mime_type: 文件 MIME 类型（如 audio/webm）
        fields: 其他表单字段，如 {"model": "whisper-1", "language": "zh"}

    Returns:
        (body_bytes, content_type)
    """
    boundary = f"----YJWhisperBoundary{uuid.uuid4().hex}"
    lines: list[bytes] = []
    # 文件字段
    lines.append(f"--{boundary}\r\n".encode())
    lines.append(
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'.encode()
    )
    lines.append(f"Content-Type: {mime_type}\r\n\r\n".encode())
    lines.append(file_bytes)
    lines.append(b"\r\n")
    # 其他字段
    for k, v in fields.items():
        if v is None or v == "":
            continue
        lines.append(f"--{boundary}\r\n".encode())
        lines.append(f'Content-Disposition: form-data; name="{k}"\r\n\r\n'.encode())
        lines.append(str(v).encode())
        lines.append(b"\r\n")
    # 结束
    lines.append(f"--{boundary}--\r\n".encode())
    body = b"".join(lines)
    content_type = f"multipart/form-data; boundary={boundary}"
    return body, content_type


def transcribe_audio(
    audio_bytes: bytes,
    mime_type: str = "audio/webm",
    filename: str = "audio.webm",
    api_base: str = "",
    api_key: str = "",
    model: str = DEFAULT_MODEL,
    language: str = "",
    timeout_s: int = DEFAULT_TIMEOUT_S,
) -> dict:
    """调用 OpenAI 兼容的 /audio/transcriptions 端点。

    Args:
        audio_bytes: 音频二进制（webm/opus/mp3/wav/m4a 等）
        mime_type: 音频 MIME 类型
        filename: 文件名
        api_base: API base URL（如 https://api.openai.com/v1）
        api_key: Bearer Token
        model: 模型名（默认 whisper-1）
        language: 语言代码（zh/en/auto/""）
        timeout_s: 超时秒数

    Returns:
        {"ok", "text", "error"|None, "status", "raw"}
    """
    if not audio_bytes:
        return _make_result(ok=False, error="音频数据为空")
    if not api_base:
        return _make_result(ok=False, error="API_BASE_URL 未配置")
    if not api_key:
        return _make_result(ok=False, error="API_KEY 未配置")

    # 规范化 api_base（去掉尾部 / 和 /v1）
    base = api_base.strip().rstrip("/")
    if base.endswith("/v1"):
        url = f"{base}/audio/transcriptions"
    else:
        url = f"{base}/v1/audio/transcriptions"

    # 构造 multipart
    fields = {
        "model": model,
        "response_format": "json",
    }
    if language:
        fields["language"] = language
    body, content_type = _build_multipart(audio_bytes, filename, mime_type, fields)

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": content_type,
        "Content-Length": str(len(body)),
    }

    try:
        req = urllib.request.Request(url, data=body, method="POST", headers=headers)
        resp = urllib.request.urlopen(req, timeout=timeout_s)
        raw = resp.read().decode("utf-8")
        try:
            obj = json.loads(raw)
            text = obj.get("text", "") if isinstance(obj, dict) else ""
            if not text:
                return _make_result(
                    ok=False,
                    status=resp.status,
                    raw=raw,
                    error="响应中未包含 text 字段",
                )
            return _make_result(ok=True, text=text, status=resp.status, raw=raw)
        except json.JSONDecodeError:
            return _make_result(ok=False, status=resp.status, raw=raw, error="响应非 JSON")
    except urllib.error.HTTPError as e:
        try:
            text = e.read().decode("utf-8")
        except Exception:
            text = ""
        # 尝试解析 OpenAI 风格错误
        err_msg = f"Whisper API HTTP {e.code}"
        try:
            err_obj = json.loads(text)
            if isinstance(err_obj, dict):
                err_obj_err = err_obj.get("error") or err_obj.get("message")
                if err_obj_err:
                    err_msg = f"{err_msg}: {err_obj_err}"
        except Exception:
            if text:
                err_msg = f"{err_msg}: {text[:200]}"
        return _make_result(ok=False, status=e.code, raw=text, error=err_msg)
    except urllib.error.URLError as e:
        return _make_result(ok=False, error=f"网络错误: {e.reason}")
    except Exception as e:
        return _make_result(ok=False, error=str(e))


def detect_supported_mime(blob: "Blob") -> str:
    """从浏览器 MediaRecorder blob 推断 MIME 类型。

    Args:
        blob: 浏览器 MediaRecorder 产出的 Blob 对象

    Returns:
        标准 MIME 类型字符串
    """
    try:
        mt = blob.type
        if mt and mt.startswith("audio/"):
            return mt
    except Exception:
        pass
    return "audio/webm"


__all__ = [
    "transcribe_audio",
    "detect_supported_mime",
    "DEFAULT_MODEL",
]
