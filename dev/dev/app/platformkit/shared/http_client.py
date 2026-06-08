"""Platform Shared HTTP Client - HTTP 请求工具

跨进程共享的纯 HTTP 工具，**不依赖 FastAPI / aiohttp**。
使用标准库 urllib，支持 GET/POST、自定义 headers、body、超时控制。

**使用规则**:
    - ✅ 新代码: `from platformkit.shared import fetch_json_with_timeout`
    - ✅ 旧代码: `from backend import fetch_json_with_timeout`（仍工作）
    - ❌ 禁止: 在本模块中重新定义（避免双源漂移）

**历史来源**:
    - 2026-06-08 从 dev/app/backend.py L152-L179 提取（TASK-1.3 Phase 1 收尾）
    - 旧位置: backend.py 顶层函数
    - 兼容期: 至少 4 周
"""

import json
import urllib.error
import urllib.request
from typing import Optional


def fetch_json_with_timeout(
    url: str,
    headers: Optional[dict] = None,
    timeout_ms: int = 15000,
    method: str = "GET",
    body: Optional[bytes] = None,
) -> dict:
    """带超时的 HTTP 请求

    Args:
        url: 目标 URL
        headers: 自定义请求头
        timeout_ms: 超时（毫秒）
        method: HTTP 方法（GET/POST/PUT/DELETE）
        body: 请求体（bytes）

    Returns:
        dict: {
            "ok": bool,         # 是否成功
            "status": int,      # HTTP 状态码（0 = 网络错误）
            "data": dict|list|None,  # 解析后的 JSON（如响应是 JSON）
            "text": str,        # 原始响应文本
        }
    """
    try:
        req = urllib.request.Request(url, method=method, headers=headers or {})
        if body:
            req.data = body
        resp = urllib.request.urlopen(req, timeout=timeout_ms / 1000)
        text = resp.read().decode("utf-8")
        data = None
        try:
            data = json.loads(text)
        except Exception:
            pass
        return {"ok": True, "status": resp.status, "data": data, "text": text}
    except urllib.error.HTTPError as e:
        text = ""
        try:
            text = e.read().decode("utf-8")
        except Exception:
            pass
        data = None
        try:
            data = json.loads(text)
        except Exception:
            pass
        return {"ok": False, "status": e.code, "data": data, "text": text}
    except Exception as e:
        return {"ok": False, "status": 0, "data": None, "text": str(e)}


__all__ = ["fetch_json_with_timeout"]
