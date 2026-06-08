"""知识管理路由 - 包装 KnowledgeEngine 暴露给前端

2026-06-09 TASK-3.2 引入：Phase 3 W9 自进化知识系统 API

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/knowledge_core.py（无 FastAPI 依赖）
    - 所有路径前缀 /api/knowledge/*
    - 不超过 30 行业务代码

Endpoints:
    GET    /api/knowledge/list           - 列表（按 layer/strength/scope 过滤）
    GET    /api/knowledge/stats          - 统计
    GET    /api/knowledge/{id}           - 详情
    POST   /api/knowledge                - 添加（4 个 add_* 系列）
    DELETE /api/knowledge/{id}           - 删除
    POST   /api/knowledge/{id}/confirm   - 确认（提升强度）
    POST   /api/knowledge/{id}/deny      - 否定（重置为 weak）
    POST   /api/knowledge/{id}/promote   - 提升为全局
    POST   /api/knowledge/relevant       - 检索相关知识（按上下文）
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared.knowledge_core import (
    KnowledgeEngine,
    KnowledgeLayer,
    KnowledgeScope,
    KnowledgeStrength,
)

router = APIRouter(prefix="/api/knowledge", tags=["知识管理（自进化）"])


# ──────────── Pydantic 模型 ────────────


class AddKnowledgeRequest(BaseModel):
    content: str = Field(..., min_length=1, description="知识内容")
    layer: str = Field(..., description="层级 L1/L2/L3/L4")
    scope: str = Field("project", description="作用域 project/global")
    strength: str = Field("weak", description="强度 weak/medium/strong")
    source: Optional[str] = Field(None, description="来源标识 user/ai/error_correction 等")


class ConfirmRequest(BaseModel):
    force_strong: bool = Field(False, description="是否立即跳到 strong")


class RelevantRequest(BaseModel):
    context: str = Field(..., min_length=1, description="检索上下文")
    top_k: int = Field(5, ge=1, le=20, description="返回前 N 条")


# ──────────── 辅助：构造 engine ────────────


def _make_engine(workspace_path: Optional[str]) -> KnowledgeEngine:
    """根据 workspace_path 构造 KnowledgeEngine。
    workspace_path 缺省时仍可使用（仅操作全局知识）。"""
    if workspace_path:
        return KnowledgeEngine(project_root=workspace_path)
    return KnowledgeEngine()


def _serialize(k) -> dict:
    """Knowledge → 字典（前端可读）"""
    d = k.to_dict()
    return {
        "id": d["id"],
        "content": d["content"],
        "layer": d["layer"],
        "strength": d["strength"],
        "scope": d["scope"],
        "confirm_count": d["confirm_count"],
        "source": d.get("source"),
        "created_at": d["created_at"],
        "updated_at": d["updated_at"],
    }


# ──────────── 端点 ────────────


@router.get("/list")
async def list_knowledge(
    workspace_path: Optional[str] = Query(None, description="项目根目录（用于项目级知识）"),
    layer: Optional[str] = Query(None, description="按层级过滤 L1/L2/L3/L4"),
    strength: Optional[str] = Query(None, description="按强度过滤 weak/medium/strong"),
    scope: Optional[str] = Query(None, description="按作用域过滤 project/global"),
):
    engine = _make_engine(workspace_path)
    items = engine.list_all()
    if layer:
        try:
            items = [k for k in items if k.layer == KnowledgeLayer(layer)]
        except ValueError:
            raise HTTPException(400, f"invalid layer: {layer}")
    if strength:
        try:
            items = [k for k in items if k.strength == KnowledgeStrength(strength)]
        except ValueError:
            raise HTTPException(400, f"invalid strength: {strength}")
    if scope:
        try:
            items = [k for k in items if k.scope == KnowledgeScope(scope)]
        except ValueError:
            raise HTTPException(400, f"invalid scope: {scope}")
    return {"ok": True, "data": [_serialize(k) for k in items], "total": len(items)}


@router.get("/stats")
async def stats(workspace_path: Optional[str] = Query(None, description="项目根目录")):
    engine = _make_engine(workspace_path)
    return {"ok": True, "data": engine.stats()}


@router.post("/relevant")
async def relevant(req: RelevantRequest, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    items = engine.get_relevant(req.context, top_k=req.top_k)
    return {"ok": True, "data": [_serialize(k) for k in items], "total": len(items)}


@router.get("/{knowledge_id}")
async def get_knowledge(knowledge_id: str, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    k = engine.get(knowledge_id)
    if not k:
        raise HTTPException(404, f"knowledge not found: {knowledge_id}")
    return {"ok": True, "data": _serialize(k)}


@router.post("")
async def add_knowledge(req: AddKnowledgeRequest, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    try:
        layer_e = KnowledgeLayer(req.layer)
    except ValueError:
        raise HTTPException(400, f"invalid layer: {req.layer}")
    try:
        scope_e = KnowledgeScope(req.scope)
    except ValueError:
        raise HTTPException(400, f"invalid scope: {req.scope}")
    try:
        strength_e = KnowledgeStrength(req.strength)
    except ValueError:
        raise HTTPException(400, f"invalid strength: {req.strength}")

    if layer_e == KnowledgeLayer.CORRECTION:
        k = engine.add_correction(req.content, scope_e, strength_e, req.source)
    elif layer_e == KnowledgeLayer.PATTERN:
        k = engine.add_pattern(req.content, scope_e, strength_e, req.source)
    elif layer_e == KnowledgeLayer.FACT:
        k = engine.add_fact(req.content, scope_e, strength_e, req.source)
    elif layer_e == KnowledgeLayer.PREFERENCE:
        k = engine.add_preference(req.content, scope_e, strength_e, req.source)
    else:
        raise HTTPException(400, f"unsupported layer: {req.layer}")
    return {"ok": True, "data": _serialize(k)}


@router.delete("/{knowledge_id}")
async def delete_knowledge(knowledge_id: str, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    ok = engine.delete(knowledge_id)
    if not ok:
        raise HTTPException(404, f"knowledge not found: {knowledge_id}")
    return {"ok": True}


@router.post("/{knowledge_id}/confirm")
async def confirm_knowledge(knowledge_id: str, req: ConfirmRequest, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    try:
        k = engine.confirm(knowledge_id, force_strong=req.force_strong)
    except KeyError:
        raise HTTPException(404, f"knowledge not found: {knowledge_id}")
    return {"ok": True, "data": _serialize(k)}


@router.post("/{knowledge_id}/deny")
async def deny_knowledge(knowledge_id: str, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    try:
        k = engine.deny(knowledge_id)
    except KeyError:
        raise HTTPException(404, f"knowledge not found: {knowledge_id}")
    return {"ok": True, "data": _serialize(k)}


@router.post("/{knowledge_id}/promote")
async def promote_knowledge(knowledge_id: str, workspace_path: Optional[str] = Query(None)):
    engine = _make_engine(workspace_path)
    try:
        k = engine.promote_to_global(knowledge_id)
    except KeyError:
        raise HTTPException(404, f"knowledge not found: {knowledge_id}")
    return {"ok": True, "data": _serialize(k)}
