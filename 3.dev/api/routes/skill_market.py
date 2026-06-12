"""技能市场路由 - 包装 skill_market 暴露给前端

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/skill_market.py（无 FastAPI 依赖）
    - 错误统一以 HTTPException 抛出
    - 所有路径前缀 /api/skill-market/*

Endpoints:
    GET  /api/skill-market/stats              - 总览统计
    GET  /api/skill-market/categories         - 分类列表
    GET  /api/skill-market/list               - 官方目录（支持 search/category/tag 过滤）
    GET  /api/skill-market/{skill_id}         - 技能详情（官方目录）
    GET  /api/skill-market/installed          - 已安装列表
    POST /api/skill-market/install            - 安装
    POST /api/skill-market/uninstall          - 卸载
    POST /api/skill-market/upload             - 上传自定义技能
    POST /api/skill-market/rate               - 评分
    POST /api/skill-market/apply              - 渲染 prompt 模板
    POST /api/skill-market/export             - 导出 zip（base64）
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared import skill_market

router = APIRouter(prefix="/api/skill-market", tags=["技能市场"])


# ── Helpers ──────────────────────────────────────────────────────
def _core_to_response(res: dict) -> dict:
    if res.get("ok"):
        return {"ok": True, "data": res.get("data")}
    return {"ok": False, "error": res.get("error") or "未知错误"}


# ── 目录（catalog） ───────────────────────────────────────────────
@router.get("/stats")
async def sm_stats():
    """市场总览统计。"""
    return _core_to_response(skill_market.get_stats())


@router.get("/categories")
async def sm_categories():
    """所有有效分类 + 各分类的技能数。"""
    return _core_to_response(skill_market.list_categories())


@router.get("/list")
async def sm_list(
    category: str = Query("", description="按分类过滤（空 = 全部）"),
    search: str = Query("", description="搜索关键字（id/name/desc/tags）"),
    tag: str = Query("", description="按标签过滤（空 = 全部）"),
    limit: int = Query(60, ge=1, le=200),
):
    """列出官方目录中的技能。"""
    res = skill_market.list_catalog(category=category, search=search, tag=tag)
    if not res["ok"]:
        return _core_to_response(res)
    return _core_to_response({**res, "data": (res.get("data") or [])[:limit]})


@router.get("/{skill_id}")
async def sm_get(skill_id: str):
    """获取官方目录中某技能的详情。"""
    res = skill_market.get_skill(skill_id)
    return _core_to_response(res)


# ── 安装 / 卸载 ──────────────────────────────────────────────────
class InstallRequest(BaseModel):
    skill_id: str = Field(..., min_length=1, max_length=64, description="技能 ID")
    scope: str = Field("global", pattern="^(global|project)$", description="安装范围")
    workspace_path: Optional[str] = Field(None, description="项目级安装需要")


@router.post("/install")
async def sm_install(req: InstallRequest):
    """从官方目录安装技能。"""
    res = skill_market.install_skill(
        skill_id=req.skill_id,
        scope=req.scope,
        workspace_path=req.workspace_path or "",
    )
    if not res["ok"]:
        raise HTTPException(status_code=400, detail=res.get("error"))
    return _core_to_response(res)


class UninstallRequest(BaseModel):
    skill_id: str = Field(..., min_length=1, max_length=64)
    scope: str = Field("global", pattern="^(global|project)$")
    workspace_path: Optional[str] = None


@router.post("/uninstall")
async def sm_uninstall(req: UninstallRequest):
    """卸载技能。"""
    res = skill_market.uninstall_skill(
        skill_id=req.skill_id,
        scope=req.scope,
        workspace_path=req.workspace_path or "",
    )
    if not res["ok"]:
        raise HTTPException(status_code=400, detail=res.get("error"))
    return _core_to_response(res)


@router.get("/installed")
async def sm_installed(
    scope: str = Query("global", pattern="^(global|project)$"),
    workspace_path: Optional[str] = Query(None, description="项目级查询需要"),
):
    """列出已安装的技能。"""
    res = skill_market.list_installed(
        scope=scope, workspace_path=workspace_path or "",
    )
    return _core_to_response(res)


# ── 上传（自定义技能） ───────────────────────────────────────────
class UploadRequest(BaseModel):
    meta: dict = Field(..., description="技能元数据")
    scope: str = Field("global", pattern="^(global|project)$")
    workspace_path: Optional[str] = None


@router.post("/upload")
async def sm_upload(req: UploadRequest):
    """上传（创建）自定义技能。"""
    res = skill_market.upload_skill(
        meta=req.meta,
        scope=req.scope,
        workspace_path=req.workspace_path or "",
    )
    if not res["ok"]:
        raise HTTPException(status_code=400, detail=res.get("error"))
    return _core_to_response(res)


# ── 评分 ────────────────────────────────────────────────────────
class RateRequest(BaseModel):
    skill_id: str = Field(..., min_length=1, max_length=64)
    score: float = Field(..., ge=1, le=5, description="1-5 星")


@router.post("/rate")
async def sm_rate(req: RateRequest):
    """给技能打分。"""
    res = skill_market.rate_skill(req.skill_id, req.score)
    return _core_to_response(res)


@router.get("/{skill_id}/rating")
async def sm_get_rating(skill_id: str):
    """获取某技能的用户评分聚合。"""
    res = skill_market.get_user_rating(skill_id)
    return _core_to_response(res)


# ── 应用（注入 prompt） ──────────────────────────────────────────
class ApplyRequest(BaseModel):
    skill_id: str = Field(..., min_length=1, max_length=64)
    variables: dict = Field(default_factory=dict, description="模板变量")
    scope: str = Field("global", pattern="^(global|project)$")
    workspace_path: Optional[str] = None


@router.post("/apply")
async def sm_apply(req: ApplyRequest):
    """渲染技能的 prompt 模板。"""
    res = skill_market.apply_skill(
        skill_id=req.skill_id,
        variables=req.variables,
        scope=req.scope,
        workspace_path=req.workspace_path or "",
    )
    if not res["ok"]:
        raise HTTPException(status_code=404, detail=res.get("error"))
    return _core_to_response(res)


# ── 导出 ────────────────────────────────────────────────────────
class ExportRequest(BaseModel):
    skill_id: str = Field(..., min_length=1, max_length=64)
    scope: str = Field("global", pattern="^(global|project)$")
    workspace_path: Optional[str] = None


@router.post("/export")
async def sm_export(req: ExportRequest):
    """把已安装的技能打包为 zip（base64），用于分享。"""
    res = skill_market.export_skill_zip(
        skill_id=req.skill_id,
        scope=req.scope,
        workspace_path=req.workspace_path or "",
    )
    return _core_to_response(res)
