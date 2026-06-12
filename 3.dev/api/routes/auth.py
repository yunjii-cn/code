"""鉴权路由 - 用户注册/登录/JWT 刷新/工作区管理

2026-06-09 TASK-4.4 引入：Phase 4 W14 多用户协作

Endpoints:
    POST /api/auth/register          - 注册新用户（自动创建 default 工作区）
    POST /api/auth/login             - 用户名密码登录 → access + refresh
    POST /api/auth/refresh           - 刷新 access token
    POST /api/auth/logout            - 撤销 refresh token
    GET  /api/auth/me                - 当前用户信息（Bearer token）
    GET  /api/auth/workspaces        - 当前用户的工作区列表
    POST /api/auth/workspaces        - 创建工作区
    POST /api/auth/workspaces/{id}/members  - 添加成员（仅 owner）
    DELETE /api/auth/workspaces/{id}/members/{user_id} - 移除成员
    POST /api/auth/devices/pair      - 配对码换取 token
"""
from typing import List, Optional

from fastapi import APIRouter, Depends, Header, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared.auth_core import (
    AuthError,
    InvalidCredentials,
    PermissionDenied,
    TokenExpired,
    TokenInvalid,
    UserNotFound,
    WorkspaceNotFound,
    get_auth_service,
    ROLE_OWNER,
    ROLE_EDITOR,
)


router = APIRouter(prefix="/api/auth", tags=["鉴权"])


# ──────────── 依赖项 ────────────


async def get_current_user(authorization: Optional[str] = Header(None)) -> str:
    """从 Authorization header 解析 user_id"""
    if not authorization or not authorization.startswith("Bearer "):
        raise HTTPException(401, "未提供 Authorization header")
    token = authorization[len("Bearer "):]
    auth = get_auth_service()
    try:
        payload = auth.verify_access_token(token)
        return payload["sub"]
    except TokenExpired:
        raise HTTPException(401, "token 已过期")
    except TokenInvalid as e:
        raise HTTPException(401, f"token 无效: {e}")


async def get_current_user_obj(authorization: Optional[str] = Header(None)):
    user_id = await get_current_user(authorization)
    auth = get_auth_service()
    user = auth.store.get_user(user_id)
    if not user:
        raise HTTPException(401, "用户不存在")
    return user


# ──────────── Pydantic 模型 ────────────


class RegisterRequest(BaseModel):
    username: str = Field(..., min_length=3, max_length=32)
    password: str = Field(..., min_length=8, max_length=128)
    email: str = Field("", max_length=128)
    display_name: str = Field("", max_length=64)


class LoginRequest(BaseModel):
    username: str
    password: str


class RefreshRequest(BaseModel):
    refresh_token: str


class CreateWorkspaceRequest(BaseModel):
    name: str = Field(..., min_length=1, max_length=64)
    description: str = Field("", max_length=256)


class AddMemberRequest(BaseModel):
    username: str = Field(..., description="用户名（不是 user_id）")
    role: str = Field("viewer", pattern="^(owner|editor|viewer)$")


class PairDeviceRequest(BaseModel):
    pair_code: str = Field(..., description="配对码（W13 daemon 颁发的）")
    device_name: str = Field("Unknown Device", max_length=64)


# ──────────── Auth Endpoints ────────────


@router.post("/register")
async def register(req: RegisterRequest):
    """注册新用户（首次启动时使用）"""
    auth = get_auth_service()
    try:
        user, access, refresh = auth.register(
            username=req.username,
            password=req.password,
            email=req.email,
            display_name=req.display_name,
        )
        # 自动创建一个 default 工作区
        ws = auth.store.create_workspace(
            name=f"{user.display_name} 的工作区",
            owner_id=user.id,
            description="默认工作区",
        )
        return {
            "ok": True,
            "data": {
                "user": user.to_dict(),
                "access_token": access,
                "refresh_token": refresh,
                "workspace_id": ws.id,
            },
        }
    except AuthError as e:
        raise HTTPException(400, str(e))


@router.post("/login")
async def login(req: LoginRequest):
    """用户名密码登录"""
    auth = get_auth_service()
    try:
        user, access, refresh = auth.login(req.username, req.password)
        return {
            "ok": True,
            "data": {
                "user": user.to_dict(),
                "access_token": access,
                "refresh_token": refresh,
            },
        }
    except InvalidCredentials as e:
        raise HTTPException(401, str(e))


@router.post("/refresh")
async def refresh(req: RefreshRequest):
    """用 refresh token 换新的 access token"""
    auth = get_auth_service()
    try:
        access, new_refresh = auth.refresh(req.refresh_token)
        return {
            "ok": True,
            "data": {
                "access_token": access,
                "refresh_token": new_refresh,
            },
        }
    except TokenInvalid as e:
        raise HTTPException(401, str(e))


@router.post("/logout")
async def logout(req: RefreshRequest):
    """登出：撤销 refresh token"""
    auth = get_auth_service()
    auth.store.revoke_refresh_token(req.refresh_token)
    return {"ok": True}


@router.get("/me")
async def me(user_id: str = Depends(get_current_user)):
    """当前用户信息"""
    auth = get_auth_service()
    user = auth.store.get_user(user_id)
    if not user:
        raise HTTPException(404, "用户不存在")
    return {"ok": True, "data": user.to_dict()}


# ──────────── Workspace Endpoints ────────────


@router.get("/workspaces")
async def list_workspaces(user_id: str = Depends(get_current_user)):
    """列出我的工作区"""
    auth = get_auth_service()
    items = []
    for ws, role in auth.store.list_user_workspaces(user_id):
        items.append({**ws.to_dict(), "role": role})
    return {"ok": True, "data": items, "total": len(items)}


@router.post("/workspaces")
async def create_workspace(req: CreateWorkspaceRequest, user_id: str = Depends(get_current_user)):
    """创建工作区"""
    auth = get_auth_service()
    ws = auth.store.create_workspace(req.name, user_id, req.description)
    return {"ok": True, "data": {**ws.to_dict(), "role": "owner"}}


@router.get("/workspaces/{workspace_id}/members")
async def list_members(workspace_id: str, user_id: str = Depends(get_current_user)):
    """列出工作区成员"""
    auth = get_auth_service()
    if not auth.store.check_permission(workspace_id, user_id, "viewer"):
        raise HTTPException(403, "无权访问该工作区")
    with auth.store._conn_lock:
        c = auth.store._conn()
        try:
            rows = c.execute(
                """SELECT * FROM workspace_members WHERE workspace_id = ?""",
                (workspace_id,),
            ).fetchall()
            return {
                "ok": True,
                "data": [
                    {
                        "user_id": r["user_id"],
                        "display_name": r["display_name"],
                        "role": r["role"],
                        "joined_at": r["joined_at"],
                    }
                    for r in rows
                ],
            }
        finally:
            c.close()


@router.post("/workspaces/{workspace_id}/members")
async def add_member(
    workspace_id: str, req: AddMemberRequest, user_id: str = Depends(get_current_user)
):
    """添加成员（仅 owner）"""
    auth = get_auth_service()
    if not auth.store.check_permission(workspace_id, user_id, "owner"):
        raise HTTPException(403, "仅 owner 可添加成员")
    target = auth.store.get_user_by_username(req.username)
    if not target:
        raise HTTPException(404, f"用户 {req.username} 不存在")
    member = auth.store.add_member(workspace_id, target.id, req.role)
    return {"ok": True, "data": member.to_dict()}


@router.delete("/workspaces/{workspace_id}/members/{member_user_id}")
async def remove_member(
    workspace_id: str, member_user_id: str, user_id: str = Depends(get_current_user)
):
    """移除成员（仅 owner）"""
    auth = get_auth_service()
    if not auth.store.check_permission(workspace_id, user_id, "owner"):
        raise HTTPException(403, "仅 owner 可移除成员")
    if member_user_id == user_id:
        raise HTTPException(400, "不能移除自己")
    ok = auth.store.remove_member(workspace_id, member_user_id)
    return {"ok": ok}


# ──────────── 设备配对（W13 兼容） ────────────


@router.post("/devices/pair")
async def pair_device(req: PairDeviceRequest):
    """用配对码换取设备 token + 提示登录

    L1 鉴权：配对码（设备级）
    成功后返回 device_id + 临时 token，提示用户登录以获取用户级 JWT
    """
    from platformkit.shared.auth_core import get_auth_service
    auth = get_auth_service()
    device = auth.store.get_device_by_token(req.pair_code)
    if not device or not device.enabled:
        raise HTTPException(404, "配对码无效")
    auth.store.touch_device(device.id)
    return {
        "ok": True,
        "data": {
            "device_id": device.id,
            "device_name": device.name,
            "user_id": device.user_id,  # 可能为空（未绑定）
            "paired": True,
        },
    }
