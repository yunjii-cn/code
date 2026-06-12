"""Agent 路由 - 独行模式 Code Agent 编排

2026-06-09 TASK-3.8 引入：Phase 3 W11 独行模式 Agent API

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 services/code_agent.py
    - 不直接调 LLM（plan 阶段由前端传 text，本模块只解析）
    - tool 调度由前端发起（路由层 routes/system.py 已提供）

Endpoints:
    GET   /api/agent/sessions                          - 列出会话
    GET   /api/agent/sessions/{id}                     - 会话详情
    POST  /api/agent/sessions                          - 创建会话 {requirement}
    DELETE /api/agent/sessions/{id}                    - 删除会话
    POST  /api/agent/sessions/{id}/plan                - 设置计划 {text}
    POST  /api/agent/sessions/{id}/append-step         - 追加步骤 {description}
    POST  /api/agent/sessions/{id}/steps/{step_id}/approve
    POST  /api/agent/sessions/{id}/steps/{step_id}/reject  {reason}
    POST  /api/agent/sessions/{id}/steps/{step_id}/start   - 开始执行
    POST  /api/agent/sessions/{id}/steps/{step_id}/tool    - 记录 tool 调用
    POST  /api/agent/sessions/{id}/steps/{step_id}/complete {result_summary}
    POST  /api/agent/sessions/{id}/steps/{step_id}/fail    {error}
    POST  /api/agent/sessions/{id}/diff               - 记录 diff
    POST  /api/agent/sessions/{id}/learnings          - 添加 learn {layer, content, step_id?}
    POST  /api/agent/sessions/{id}/close              - 归档
    GET   /api/agent/stats                             - 全局统计
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from services.code_agent import (
    CodeAgent,
    CodeSession,
    DiffEntry,
    LearningEntry,
    SessionStatus,
)
from platformkit.shared.knowledge_core import KnowledgeLayer

router = APIRouter(prefix="/api/agent", tags=["独行 Agent"])


# ──────────── Agent 缓存 ────────────


_agents: dict = {}


def _agent(workspace_path: Optional[str]) -> CodeAgent:
    key = workspace_path or "_default_"
    if key not in _agents:
        if workspace_path:
            from pathlib import Path
            _agents[key] = CodeAgent(workspace_path)
        else:
            from pathlib import Path
            _agents[key] = CodeAgent(Path.cwd())
    return _agents[key]


# ──────────── Pydantic 模型 ────────────


class CreateSessionRequest(BaseModel):
    requirement: str = Field(..., min_length=1, description="用户需求描述")


class PlanRequest(BaseModel):
    text: str = Field(..., min_length=1, description="计划文本（markdown 列表 / 数字列表 / 纯行）")


class AppendStepRequest(BaseModel):
    description: str = Field(..., min_length=1)


class RejectRequest(BaseModel):
    reason: Optional[str] = None


class ToolCallRequest(BaseModel):
    name: str = Field(..., min_length=1)
    args: dict = Field(default_factory=dict)
    result: str = ""


class CompleteRequest(BaseModel):
    result_summary: Optional[str] = None


class FailRequest(BaseModel):
    error: str = Field(..., min_length=1)


class DiffRequest(BaseModel):
    step_id: str
    file_path: str
    added_lines: int = 0
    removed_lines: int = 0
    summary: str = ""


class LearningRequest(BaseModel):
    layer: str  # L1/L2/L3/L4
    content: str = Field(..., min_length=1)
    step_id: Optional[str] = None


# ──────────── 辅助 ────────────


def _serialize(s: CodeSession) -> dict:
    return s.to_dict()


# ──────────── 端点 ────────────


@router.get("/sessions")
async def list_sessions(
    workspace_path: Optional[str] = Query(None),
    status: Optional[str] = Query(None, description="按状态过滤 draft/planned/.../closed"),
):
    agent = _agent(workspace_path)
    if status:
        try:
            sessions = agent.list_sessions(SessionStatus(status))
        except ValueError:
            raise HTTPException(400, f"invalid status: {status}")
    else:
        sessions = agent.list_sessions()
    return {"ok": True, "data": [_serialize(s) for s in sessions], "total": len(sessions)}


@router.get("/sessions/{session_id}")
async def get_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    s = agent.get_session(session_id)
    if not s:
        raise HTTPException(404, f"session not found: {session_id}")
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions")
async def create_session(req: CreateSessionRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.create_session(req.requirement)
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.delete("/sessions/{session_id}")
async def delete_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    """删除会话（直接 remove from _sessions）"""
    agent = _agent(workspace_path)
    s = agent.get_session(session_id)
    if not s:
        raise HTTPException(404, f"session not found: {session_id}")
    # 用 _save_sessions 把当前状态保存（删除直接操作内部 dict）
    agent._sessions.pop(session_id, None)
    agent._save_sessions()
    return {"ok": True}


@router.post("/sessions/{session_id}/plan")
async def set_plan(session_id: str, req: PlanRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    steps = CodeAgent.parse_plan_text(req.text)
    if not steps:
        raise HTTPException(400, "no valid steps parsed from text")
    try:
        s = agent.set_plan(session_id, steps)
    except KeyError:
        raise HTTPException(404, f"session not found: {session_id}")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/append-step")
async def append_step(session_id: str, req: AppendStepRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.append_step(session_id, req.description)
    except KeyError:
        raise HTTPException(404, f"session not found: {session_id}")
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/approve")
async def approve_step(session_id: str, step_id: str, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.approve_step(session_id, step_id)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/reject")
async def reject_step(session_id: str, step_id: str, req: RejectRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.reject_step(session_id, step_id, req.reason)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/start")
async def start_step(session_id: str, step_id: str, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.start_execute(session_id, step_id)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/tool")
async def record_tool(session_id: str, step_id: str, req: ToolCallRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.record_tool_call(session_id, step_id, req.name, req.args, req.result)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/complete")
async def complete_step(session_id: str, step_id: str, req: CompleteRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.complete_step(session_id, step_id, req.result_summary)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/steps/{step_id}/fail")
async def fail_step(session_id: str, step_id: str, req: FailRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.fail_step(session_id, step_id, req.error)
    except KeyError:
        raise HTTPException(404, f"session or step not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/diff")
async def record_diff(session_id: str, req: DiffRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    if not agent.get_session(session_id):
        raise HTTPException(404, f"session not found: {session_id}")
    entry = DiffEntry(
        step_id=req.step_id,
        file_path=req.file_path,
        added_lines=req.added_lines,
        removed_lines=req.removed_lines,
        summary=req.summary,
    )
    s = agent.record_diff(session_id, entry)
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/learnings")
async def add_learning(session_id: str, req: LearningRequest, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        layer_e = KnowledgeLayer(req.layer)
    except ValueError:
        raise HTTPException(400, f"invalid layer: {req.layer}")
    try:
        s = agent.add_learning(session_id, layer_e, req.content, req.step_id)
    except KeyError:
        raise HTTPException(404, f"session not found: {session_id}")
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions/{session_id}/close")
async def close_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    try:
        s = agent.close_session(session_id)
    except KeyError:
        raise HTTPException(404, f"session not found: {session_id}")
    return {"ok": True, "data": _serialize(s)}


@router.get("/stats")
async def stats(workspace_path: Optional[str] = Query(None)):
    agent = _agent(workspace_path)
    return {"ok": True, "data": agent.stats()}
