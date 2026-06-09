"""Tailscale 路由 - 远程访问配置 API

2026-06-09 TASK-4.2 引入：Phase 4 W13 Tailscale 集成

Endpoints:
    GET  /api/tailscale/status           - 探测 Tailscale 状态（IP/hostname/账户/在线）
    GET  /api/tailscale/remote-hint      - 远程访问提示（推荐 URL + 备选 + 配对码）
    POST /api/tailscale/pairing/regenerate  - 重新生成配对码
    POST /api/tailscale/pairing/verify      - 校验配对码
    GET  /api/tailscale/summary          - 完整摘要（status + remote-hint + 平台信息）
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared.tailscale_core import (
    PairingCodeStore,
    TailscaleDetector,
    build_remote_hint,
    tailscale_summary,
)


router = APIRouter(prefix="/api/tailscale", tags=["远程访问"])


# ──────────── Pydantic ────────────


class VerifyPairingRequest(BaseModel):
    code: str = Field(..., min_length=1, description="配对码（6 位 hex 大写）")


# ──────────── 端点 ────────────


@router.get("/status")
async def status(
    use_cache: bool = Query(True, description="是否使用 5 秒缓存"),
):
    """Tailscale 状态探测"""
    info = TailscaleDetector.detect(use_cache=use_cache)
    return {"ok": True, "data": info.to_dict()}


@router.get("/remote-hint")
async def remote_hint(
    port: int = Query(18080, ge=1, le=65535, description="监听端口"),
    include_code: bool = Query(True, description="是否包含配对码"),
    regenerate: bool = Query(False, description="是否重新生成配对码"),
):
    """远程访问提示（含配对码）"""
    if regenerate:
        code = PairingCodeStore.instance().regenerate()
    else:
        code = PairingCodeStore.instance().get_or_generate()
    code_str = code.value if include_code else None
    hint = build_remote_hint(port=port, pairing_code=code_str)
    return {
        "ok": True,
        "data": {
            "hint": hint.to_dict(),
            "pairing_code": code.to_dict() if include_code else None,
        },
    }


@router.post("/pairing/regenerate")
async def regenerate_pairing(ttl_seconds: int = Query(3600, ge=60, le=86400)):
    """重新生成配对码"""
    code = PairingCodeStore.instance().regenerate(ttl_seconds=ttl_seconds)
    return {"ok": True, "data": code.to_dict()}


@router.post("/pairing/verify")
async def verify_pairing(req: VerifyPairingRequest):
    """校验配对码"""
    valid = PairingCodeStore.instance().verify(req.code)
    if not valid:
        raise HTTPException(401, "配对码无效或已过期")
    return {"ok": True, "data": {"valid": True}}


@router.get("/summary")
async def summary():
    """Tailscale 完整摘要（含 remote-hint）"""
    s = tailscale_summary()
    code = PairingCodeStore.instance().get_or_generate()
    hint = build_remote_hint(pairing_code=code.value)
    s["remote_hint"] = hint.to_dict()
    s["pairing_code"] = code.to_dict()
    return {"ok": True, "data": s}
