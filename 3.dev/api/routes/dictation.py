"""Dictation 路由 - 包装 whisper_core 暴露给前端

TASK-2.5 (2026-06-10) 语音听写

设计原则:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/whisper_core.py
    - 错误统一以 HTTPException 抛出

API 规范:
    POST /api/dictation/transcribe
        Content-Type: multipart/form-data
        - file: 音频文件
        - mime_type: 客户端 MIME（默认 audio/webm）
        - filename: 文件名（默认 audio.webm）
        - model: 模型名（默认 whisper-1）
        - language: 语言代码（zh/en/auto，默认 auto）
        - provider_id: 配置中预存的供应商 ID（从 ProviderConfig 读 api_base/api_key）
"""
from typing import Optional

from fastapi import APIRouter, File, Form, HTTPException, UploadFile

from platformkit.shared import whisper_core

router = APIRouter(prefix="/api/dictation", tags=["Dictation"])


@router.post("/transcribe")
async def transcribe(
    file: UploadFile = File(...),
    mime_type: str = Form("audio/webm"),
    filename: Optional[str] = Form(None),
    model: str = Form(whisper_core.DEFAULT_MODEL),
    language: str = Form("auto"),
    api_base: str = Form(""),
    api_key: str = Form(""),
    timeout_s: int = Form(whisper_core.DEFAULT_TIMEOUT_S),
):
    """接收音频文件 → 调用 Whisper API → 返回转写文本。

    如果前端传 api_base / api_key 为空，服务端可从 ProviderConfig 读取（待集成）。
    """
    try:
        audio_bytes = await file.read()
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"读取音频失败: {e}")
    if not audio_bytes:
        raise HTTPException(status_code=400, detail="音频数据为空")

    # 如果未传 api_base/api_key：尝试从 settings 读默认（lazy import 避免循环）
    if not api_base or not api_key:
        try:
            from platformkit.shared import config_core

            cfg = config_core.get_provider_config(provider_type="openai", id="whisper")
            if cfg:
                api_base = api_base or cfg.get("api_base", "")
                api_key = api_key or cfg.get("api_key", "")
        except Exception:
            pass

    if not api_base or not api_key:
        raise HTTPException(
            status_code=400,
            detail="未配置 Whisper API（请在 .env 中设置 WHISPER_API_BASE 和 WHISPER_API_KEY，或在请求中传 api_base/api_key）",
        )

    result = whisper_core.transcribe_audio(
        audio_bytes=audio_bytes,
        mime_type=mime_type,
        filename=filename or file.filename or "audio.webm",
        api_base=api_base,
        api_key=api_key,
        model=model,
        language=language,
        timeout_s=timeout_s,
    )
    return result
