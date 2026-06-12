"""whisper_core 单元测试 - TASK-2.5 (2026-06-10)

覆盖:
    - _build_multipart: 正确生成 multipart 字节 + content-type
    - transcribe_audio: 空字节 / 缺 API / 成功 / HTTP 错误 / 网络错误
    - detect_supported_mime: 各种 blob type
"""
from __future__ import annotations

import json
from unittest.mock import patch, MagicMock

import pytest

from platformkit.shared import whisper_core


# ============ _build_multipart 测试 ============

def test_build_multipart_basic():
    body, ct = whisper_core._build_multipart(
        file_bytes=b"hello",
        filename="test.webm",
        mime_type="audio/webm",
        fields={"model": "whisper-1"},
    )
    assert b'hello' in body
    assert b'name="file"' in body
    assert b'filename="test.webm"' in body
    assert b'Content-Type: audio/webm' in body
    assert b'name="model"' in body
    assert b'whisper-1' in body
    assert 'multipart/form-data; boundary=' in ct


def test_build_multipart_skip_empty_fields():
    body, ct = whisper_core._build_multipart(
        file_bytes=b"x",
        filename="a.webm",
        mime_type="audio/webm",
        fields={"model": "whisper-1", "language": "", "prompt": None},
    )
    assert b'name="model"' in body
    assert b'name="language"' not in body
    assert b'name="prompt"' not in body


def test_build_multipart_multiple_fields():
    body, _ = whisper_core._build_multipart(
        file_bytes=b"x",
        filename="a.webm",
        mime_type="audio/webm",
        fields={"a": "1", "b": "2", "c": "3"},
    )
    assert b'name="a"' in body
    assert b'name="b"' in body
    assert b'name="c"' in body


# ============ transcribe_audio 测试 ============

def test_transcribe_audio_empty_bytes():
    res = whisper_core.transcribe_audio(b"", api_base="https://api.test", api_key="k")
    assert res["ok"] is False
    assert "为空" in res["error"]


def test_transcribe_audio_missing_api_base():
    res = whisper_core.transcribe_audio(b"x", api_base="", api_key="k")
    assert res["ok"] is False
    assert "API_BASE_URL" in res["error"]


def test_transcribe_audio_missing_api_key():
    res = whisper_core.transcribe_audio(b"x", api_base="https://api.test", api_key="")
    assert res["ok"] is False
    assert "API_KEY" in res["error"]


def test_transcribe_audio_strips_trailing_slash():
    """api_base 末尾的 / 应被去除。"""
    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = b'{"text": "hello"}'
        mock_resp.__enter__ = MagicMock(return_value=mock_resp)
        mock_resp.__exit__ = MagicMock(return_value=False)
        mock_urlopen.return_value = mock_resp

        res = whisper_core.transcribe_audio(
            b"x", api_base="https://api.test/", api_key="k"
        )
        assert res["ok"] is True
        assert res["text"] == "hello"
        # 验证 url 不含双斜杠
        called_url = mock_urlopen.call_args[0][0].full_url
        assert "//" not in called_url.replace("https://", "")


def test_transcribe_audio_keeps_v1_suffix():
    """api_base 已含 /v1 时不应重复加。"""
    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = b'{"text": "hi"}'
        mock_resp.__enter__ = MagicMock(return_value=mock_resp)
        mock_resp.__exit__ = MagicMock(return_value=False)
        mock_urlopen.return_value = mock_resp

        res = whisper_core.transcribe_audio(
            b"x", api_base="https://api.test/v1", api_key="k"
        )
        assert res["ok"] is True
        called_url = mock_urlopen.call_args[0][0].full_url
        assert called_url.endswith("/audio/transcriptions")
        assert "/v1/v1/" not in called_url


def test_transcribe_audio_success():
    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = b'{"text": "hello world"}'
        mock_resp.__enter__ = MagicMock(return_value=mock_resp)
        mock_resp.__exit__ = MagicMock(return_value=False)
        mock_urlopen.return_value = mock_resp

        res = whisper_core.transcribe_audio(
            b"audio bytes",
            api_base="https://api.test",
            api_key="sk-test",
        )
        assert res["ok"] is True
        assert res["text"] == "hello world"
        assert res["status"] == 200


def test_transcribe_audio_empty_text_in_response():
    """响应中无 text 字段 → 错误。"""
    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = b'{}'
        mock_resp.__enter__ = MagicMock(return_value=mock_resp)
        mock_resp.__exit__ = MagicMock(return_value=False)
        mock_urlopen.return_value = mock_resp

        res = whisper_core.transcribe_audio(b"x", api_base="https://api.test", api_key="k")
        assert res["ok"] is False
        assert "text" in res["error"]


def test_transcribe_audio_invalid_json():
    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = b'not json'
        mock_resp.__enter__ = MagicMock(return_value=mock_resp)
        mock_resp.__exit__ = MagicMock(return_value=False)
        mock_urlopen.return_value = mock_resp

        res = whisper_core.transcribe_audio(b"x", api_base="https://api.test", api_key="k")
        assert res["ok"] is False
        assert "JSON" in res["error"]


def test_transcribe_audio_http_401():
    """HTTP 401 → 错误信息含状态码。"""
    import urllib.error

    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        err = urllib.error.HTTPError(
            url="https://api.test/v1/audio/transcriptions",
            code=401,
            msg="Unauthorized",
            hdrs={},
            fp=MagicMock(read=MagicMock(return_value=b'{"error": {"message": "Invalid API key"}}')),
        )
        mock_urlopen.side_effect = err

        res = whisper_core.transcribe_audio(b"x", api_base="https://api.test", api_key="bad")
        assert res["ok"] is False
        assert res["status"] == 401
        assert "Invalid API key" in res["error"] or "401" in res["error"]


def test_transcribe_audio_network_error():
    import urllib.error

    with patch("platformkit.shared.whisper_core.urllib.request.urlopen") as mock_urlopen:
        mock_urlopen.side_effect = urllib.error.URLError("connection refused")
        res = whisper_core.transcribe_audio(b"x", api_base="https://api.test", api_key="k")
        assert res["ok"] is False
        assert "网络" in res["error"]


# ============ detect_supported_mime 测试 ============

def test_detect_supported_mime_webm():
    blob = MagicMock()
    blob.type = "audio/webm;codecs=opus"
    assert whisper_core.detect_supported_mime(blob) == "audio/webm;codecs=opus"


def test_detect_supported_mime_ogg():
    blob = MagicMock()
    blob.type = "audio/ogg"
    assert whisper_core.detect_supported_mime(blob) == "audio/ogg"


def test_detect_supported_mime_fallback():
    blob = MagicMock()
    blob.type = ""
    assert whisper_core.detect_supported_mime(blob) == "audio/webm"


def test_detect_supported_mime_ignores_non_audio():
    blob = MagicMock()
    blob.type = "video/mp4"
    assert whisper_core.detect_supported_mime(blob) == "audio/webm"
