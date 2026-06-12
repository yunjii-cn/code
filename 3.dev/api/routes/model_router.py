"""智能模型路由 HTTP 路由 (TASK-4.6)

**Endpoints**:
    GET  /api/model-router/rules              - 读取当前规则
    PUT  /api/model-router/rules              - 更新规则
    POST /api/model-router/reset              - 重置为默认
    POST /api/model-router/evaluate           - 评估复杂度（不路由）
    POST /api/model-router/decide             - 路由决策（评估 + 选 tier + 记录）
    GET  /api/model-router/history            - 历史记录
    DELETE /api/model-router/history          - 清空历史
    GET  /api/model-router/defaults           - 默认规则（用于"恢复默认"按钮）

**设计原则**:
    - 路由层只做参数解析 + 调 core
    - 业务逻辑在 platformkit/shared/model_router.py
"""
from typing import Optional, List

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared import model_router

router = APIRouter(prefix="/api/model-router", tags=["智能模型路由"])


# ── Pydantic 模型 ───────────────────────────────────────────────
class ModelRefDto(BaseModel):
    provider: str = Field(..., min_length=1, max_length=64)
    model: str = Field(..., min_length=1, max_length=128)


class RoutingTierDto(BaseModel):
    name: str = Field(..., min_length=1, max_length=32)
    max_complexity: float = Field(..., ge=0.0, le=1.5)
    primary: ModelRefDto
    fallback: Optional[ModelRefDto] = None


class RoutingRulesDto(BaseModel):
    tiers: List[RoutingTierDto] = Field(..., min_length=1, max_length=10)
    auto_fallback: bool = True
    history_limit: int = Field(200, ge=10, le=2000)


class EvaluateRequest(BaseModel):
    prompt: str = Field(..., min_length=1)
    history: Optional[List[dict]] = None
    has_images: bool = False


class DecideRequest(BaseModel):
    prompt: str = Field(..., min_length=1)
    history: Optional[List[dict]] = None
    has_images: bool = False
    record: bool = Field(True, description="是否记录到历史")


# ── 工具 ────────────────────────────────────────────────────────
def _core_to_response(res: dict) -> dict:
    if res.get("ok"):
        return {"ok": True, "data": res.get("data")}
    return {"ok": False, "error": res.get("error") or "未知错误"}


# ── 规则管理 ────────────────────────────────────────────────────
@router.get("/defaults")
async def get_defaults():
    """获取默认规则（用于界面"恢复默认"按钮）。"""
    return _core_to_response({"ok": True, "data": model_router.DEFAULT_RULES.to_dict()})


@router.get("/rules")
async def get_rules():
    """读取当前规则。"""
    return _core_to_response(model_router.load_rules())


@router.put("/rules")
async def put_rules(req: RoutingRulesDto):
    """更新规则（同时校验）。"""
    payload = req.model_dump()
    res = model_router.save_rules(payload)
    if not res["ok"]:
        raise HTTPException(status_code=400, detail=res.get("error"))
    return _core_to_response(res)


@router.post("/reset")
async def post_reset():
    """重置为默认规则。"""
    return _core_to_response(model_router.reset_rules())


# ── 评估 / 决策 ────────────────────────────────────────────────
@router.post("/evaluate")
async def post_evaluate(req: EvaluateRequest):
    """仅评估复杂度（不路由、不记录）。"""
    score = model_router.evaluate_complexity(
        prompt=req.prompt,
        history=req.history,
        has_images=req.has_images,
    )
    return _core_to_response({"ok": True, "data": score.to_dict()})


@router.post("/decide")
async def post_decide(req: DecideRequest):
    """路由决策（评估 + 选 tier）。

    自动加载持久化规则，可选是否记录到历史。
    """
    rules_res = model_router.load_rules()
    rules_dict = rules_res["data"] if rules_res["ok"] else model_router.DEFAULT_RULES.to_dict()
    rules = model_router.RoutingRules.from_dict(rules_dict)

    decision = model_router.decide(
        prompt=req.prompt,
        rules=rules,
        history=req.history,
        has_images=req.has_images,
    )
    if req.record:
        model_router.record_decision(decision, prompt_preview=req.prompt)
    return _core_to_response({"ok": True, "data": decision.to_dict()})


# ── 历史 ────────────────────────────────────────────────────────
@router.get("/history")
async def get_history(
    limit: int = Query(50, ge=1, le=500, description="返回条数（最新在前）"),
):
    """读取路由历史。"""
    return _core_to_response(model_router.get_history(limit=limit))


@router.delete("/history")
async def delete_history():
    """清空路由历史。"""
    return _core_to_response(model_router.clear_history())
