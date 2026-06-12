"""Y.js WebSocket 路由

2026-06-09 TASK-4.4 引入：Phase 4 W14 多用户协作

Endpoints:
    WS /api/yjs/{workspace_id}    - 实时协作（Y.js 协议）
        鉴权：query param ?token=<jwt> 或 Authorization header
        房间：workspace_id 决定房间（一个工作区一个 Y.Doc）

    GET /api/yjs/rooms            - 列出所有房间（管理用）
    GET /api/yjs/rooms/{id}       - 房间详情
"""
import asyncio
import json
import logging
from typing import Optional

from fastapi import APIRouter, Header, HTTPException, Query, WebSocket, WebSocketDisconnect
from pydantic import BaseModel

from platformkit.shared.yjs_server import (
    YjsAuthResult,
    YjsServer,
    get_yjs_server,
)
from platformkit.shared.auth_core import (
    AuthError,
    TokenInvalid,
    TokenExpired,
    get_auth_service,
)

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/yjs", tags=["实时协作 Y.js"])


# ──────────── 鉴权回调（注入到 YjsServer）────────────


def _auth_callback(token: str, room_id: str) -> YjsAuthResult:
    """从 JWT 解析用户信息并校验工作区权限

    规则：
      - 必须是有效 JWT
      - 用户必须存在于数据库
      - 用户必须是 room_id（workspace）的成员
      - role 决定写权限：
          * owner / editor: 允许读写
          * viewer: 仅读（仍可连接，只读状态）
    """
    if not token:
        return YjsAuthResult(allowed=False, reason="no token")

    auth = get_auth_service()
    try:
        payload = auth.verify_access_token(token)
    except TokenExpired:
        return YjsAuthResult(allowed=False, reason="token expired")
    except TokenInvalid as e:
        return YjsAuthResult(allowed=False, reason=f"invalid token: {e}")

    user_id = payload.get("sub", "")
    user = auth.store.get_user(user_id)
    if not user or not user.is_active:
        return YjsAuthResult(allowed=False, reason="user not found or inactive")

    # room_id = workspace_id
    member = auth.store.get_member(room_id, user_id)
    if not member:
        return YjsAuthResult(allowed=False, reason="not a member of this workspace")

    return YjsAuthResult(
        allowed=True,
        user_id=user_id,
        username=user.username,
        role=member.role,
        workspace_id=room_id,
    )


def _get_server() -> YjsServer:
    return get_yjs_server(auth_callback=_auth_callback)


# ──────────── WebSocket 端点 ────────────


@router.websocket("/{workspace_id}")
async def yjs_websocket(
    websocket: WebSocket,
    workspace_id: str,
    token: Optional[str] = Query(None),
):
    """Y.js WebSocket 端点

    握手：
      - 客户端通过 query param `?token=<jwt>` 提供 JWT
      - 也可后续在第一帧中发送 {"type":"auth","token":"..."}
      - 鉴权失败 → 1008 close

    协议：
      - 接收：Y.js binary 消息（首字节：0=sync, 1=awareness）
      - 发送：Y.js binary 消息
    """
    await websocket.accept()
    server = _get_server()

    # 鉴权（先看 query param）
    auth_token = token
    if not auth_token:
        # 等待客户端发送 auth 消息
        try:
            first_msg = await asyncio.wait_for(websocket.receive_text(), timeout=5.0)
            try:
                auth_msg = json.loads(first_msg)
                if auth_msg.get("type") == "auth":
                    auth_token = auth_msg.get("token")
            except json.JSONDecodeError:
                pass
        except Exception:
            pass

    # 接入房间
    connected = await server.on_connect(websocket, workspace_id, auth_token or "")
    if not connected:
        return

    try:
        while True:
            # 接收二进制（Y.js 协议）
            message = await websocket.receive_bytes()
            await server.on_message(websocket, workspace_id, message)
    except WebSocketDisconnect:
        pass
    except Exception as e:
        logger.exception(f"[Yjs] ws error: {e}")
    finally:
        await server.on_disconnect(websocket, workspace_id)


# ──────────── 管理端点 ────────────


@router.get("/rooms")
async def list_rooms(authorization: Optional[str] = Header(None)):
    """列出所有活跃房间（管理员）"""
    if not authorization or not authorization.startswith("Bearer "):
        raise HTTPException(401, "未授权")
    token = authorization[len("Bearer "):]
    auth = get_auth_service()
    try:
        auth.verify_access_token(token)
    except (TokenInvalid, TokenExpired):
        raise HTTPException(401, "token 无效")

    server = _get_server()
    return {"ok": True, "data": server.list_rooms()}


@router.get("/rooms/{room_id}")
async def room_detail(room_id: str, authorization: Optional[str] = Header(None)):
    """房间详情"""
    if not authorization or not authorization.startswith("Bearer "):
        raise HTTPException(401, "未授权")
    token = authorization[len("Bearer "):]
    auth = get_auth_service()
    try:
        payload = auth.verify_access_token(token)
        user_id = payload["sub"]
    except (TokenInvalid, TokenExpired):
        raise HTTPException(401, "token 无效")

    if not auth.store.check_permission(room_id, user_id, "viewer"):
        raise HTTPException(403, "无权访问该工作区")

    server = _get_server()
    room = server.get_room(room_id)
    return {
        "ok": True,
        "data": {
            "id": room_id,
            "client_count": room.client_count(),
            "last_save_at": room._last_save_at,
            "has_state": room.ydoc_state_path.exists(),
            "state_size": room.ydoc_state_path.stat().st_size if room.ydoc_state_path.exists() else 0,
        },
    }
