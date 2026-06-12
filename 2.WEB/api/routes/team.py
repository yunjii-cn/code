"""Team 路由 - 团队模式工作流编排 API

2026-06-09 TASK-3.9 引入：Phase 3 W12 团队模式路由

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 services/team_workflow.py
    - 不直接调 LLM（plan 阶段由前端传 text，本模块只解析）
    - tool 调度由前端发起（路由层 routes/system.py 已提供）

Endpoints:
    GET   /api/team/status                              - 团队模式状态（角色/工具/审查对）
    GET   /api/team/sessions                            - 列出会话
    GET   /api/team/sessions/{id}                       - 会话详情
    POST  /api/team/sessions                            - 创建会话 {requirement}
    DELETE /api/team/sessions/{id}                      - 删除会话
    POST  /api/team/sessions/{id}/tasks                 - 添加任务 {title, description, role, dependencies?}
    POST  /api/team/sessions/{id}/tasks/{task_id}/assign  {agent_id}
    POST  /api/team/sessions/{id}/tasks/{task_id}/start   - 开始执行
    POST  /api/team/sessions/{id}/tasks/{task_id}/complete  {output, file_paths?}
    POST  /api/team/sessions/{id}/tasks/{task_id}/fail     {error}
    POST  /api/team/sessions/{id}/approvals             - 触发审批 {task_id, approver_role, reason}
    POST  /api/team/sessions/{id}/approvals/{id}/decide  {approve, comment?}
    POST  /api/team/sessions/{id}/messages              - 团队消息 {role, content, type?, task_id?}
    POST  /api/team/sessions/{id}/context               - 共享上下文 {key, value, source}
    POST  /api/team/sessions/{id}/check-boundary        - 边界检查 {role, path, action?}
    POST  /api/team/sessions/{id}/close                 - 归档
    GET   /api/team/stats                               - 全局统计
"""
from pathlib import Path
from typing import List, Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from services.team_workflow import (
    TeamWorkflow,
    TeamSession,
    TeamStatus,
    TaskStatus,
    MessageType,
)
from platformkit.shared.team_core import (
    AgentRole,
    BoundaryViolationError,
    check_boundary,
    team_status as get_team_status,
    parse_role,
)


router = APIRouter(prefix="/api/team", tags=["团队模式"])


# ──────────── Workflow 缓存 ────────────


_workflows: dict = {}


def _workflow(workspace_path: Optional[str]) -> TeamWorkflow:
    key = workspace_path or "_default_"
    if key not in _workflows:
        if workspace_path:
            _workflows[key] = TeamWorkflow(workspace_path)
        else:
            _workflows[key] = TeamWorkflow(Path.cwd())
    return _workflows[key]


# ──────────── Pydantic 模型 ────────────


class CreateSessionRequest(BaseModel):
    requirement: str = Field(..., min_length=1, description="用户需求")


class AddTaskRequest(BaseModel):
    title: str = Field(..., min_length=1)
    description: str = ""
    role: str = Field(..., description="coordinator/architect/developer/tester/documenter")
    dependencies: List[str] = Field(default_factory=list)


class AssignTaskRequest(BaseModel):
    agent_id: str = Field(..., min_length=1, description="派发给具体的 agent_id")


class CompleteTaskRequest(BaseModel):
    output: Optional[str] = None
    file_paths: List[str] = Field(default_factory=list)


class FailTaskRequest(BaseModel):
    error: str = Field(..., min_length=1)


class RequestApprovalRequest(BaseModel):
    task_id: str
    approver_role: str = Field(..., description="coordinator/architect/developer/tester/documenter")
    reason: str = Field(..., min_length=1)


class DecideApprovalRequest(BaseModel):
    approve: bool
    comment: Optional[str] = None


class AddMessageRequest(BaseModel):
    role: str
    content: str = Field(..., min_length=1)
    type: str = "chat"
    task_id: Optional[str] = None


class UpdateContextRequest(BaseModel):
    key: str = Field(..., min_length=1)
    value: str = Field(..., min_length=1)
    source: str = Field(..., description="写入者角色")


class CheckBoundaryRequest(BaseModel):
    role: str
    path: str
    action: str = "write"


# ──────────── 辅助 ────────────


def _serialize(s: TeamSession) -> dict:
    return s.to_dict()


# ──────────── 端点 ────────────


@router.get("/status")
async def status():
    """团队模式状态（角色/工具/审查对）"""
    return {"ok": True, "data": get_team_status()}


@router.get("/sessions")
async def list_sessions(
    workspace_path: Optional[str] = Query(None),
    status: Optional[str] = Query(None, description="按状态过滤"),
):
    wf = _workflow(workspace_path)
    if status:
        try:
            sessions = wf.list_sessions(TeamStatus(status))
        except ValueError:
            raise HTTPException(400, f"invalid status: {status}")
    else:
        sessions = wf.list_sessions()
    return {"ok": True, "data": [_serialize(s) for s in sessions], "total": len(sessions)}


@router.get("/sessions/{session_id}")
async def get_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    s = wf.get_session(session_id)
    if not s:
        raise HTTPException(404, f"team session not found: {session_id}")
    return {"ok": True, "data": _serialize(s)}


@router.post("/sessions")
async def create_session(req: CreateSessionRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        s = wf.create_session(req.requirement)
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(s)}


@router.delete("/sessions/{session_id}")
async def delete_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    if not wf.delete_session(session_id):
        raise HTTPException(404, f"team session not found: {session_id}")
    return {"ok": True}


@router.post("/sessions/{session_id}/tasks")
async def add_task(session_id: str, req: AddTaskRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        task = wf.add_task(session_id, req.title, req.description, req.role, req.dependencies)
    except KeyError:
        raise HTTPException(404, f"team session not found: {session_id}")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/tasks/{task_id}/assign")
async def assign_task(session_id: str, task_id: str, req: AssignTaskRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        task = wf.assign_task(session_id, task_id, req.agent_id)
    except KeyError:
        raise HTTPException(404, f"team session or task not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/tasks/{task_id}/start")
async def start_task(session_id: str, task_id: str, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        task = wf.start_task(session_id, task_id)
    except KeyError:
        raise HTTPException(404, f"team session or task not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/tasks/{task_id}/complete")
async def complete_task(session_id: str, task_id: str, req: CompleteTaskRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        task = wf.complete_task(session_id, task_id, req.output, req.file_paths or None)
    except KeyError:
        raise HTTPException(404, f"team session or task not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    except BoundaryViolationError as e:
        raise HTTPException(403, f"边界违规: {e.reason}")
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/tasks/{task_id}/fail")
async def fail_task(session_id: str, task_id: str, req: FailTaskRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        task = wf.fail_task(session_id, task_id, req.error)
    except KeyError:
        raise HTTPException(404, f"team session or task not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/approvals")
async def request_approval(session_id: str, req: RequestApprovalRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        node = wf.request_approval(session_id, req.task_id, req.approver_role, req.reason)
    except KeyError:
        raise HTTPException(404, f"team session or task not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/approvals/{approval_id}/decide")
async def decide_approval(session_id: str, approval_id: str, req: DecideApprovalRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        node = wf.decide_approval(session_id, approval_id, req.approve, req.comment)
    except KeyError:
        raise HTTPException(404, f"team session or approval not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/messages")
async def add_message(session_id: str, req: AddMessageRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        msg = wf.add_message(session_id, req.role, req.content, req.type, req.task_id)
    except KeyError:
        raise HTTPException(404, f"team session not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/context")
async def update_context(session_id: str, req: UpdateContextRequest, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        ctx = wf.update_shared_context(session_id, req.key, req.value, req.source)
    except KeyError:
        raise HTTPException(404, f"team session not found")
    except ValueError as e:
        raise HTTPException(400, str(e))
    return {"ok": True, "data": _serialize(wf.get_session(session_id))}


@router.post("/sessions/{session_id}/check-boundary")
async def check_role_boundary(session_id: str, req: CheckBoundaryRequest, workspace_path: Optional[str] = Query(None)):
    """检查角色边界（路由层暴露给前端在工具调度前预检）"""
    wf = _workflow(workspace_path)
    try:
        ok = wf.check_role_boundary(req.role, req.path, req.action)
    except ValueError as e:
        raise HTTPException(400, str(e))
    except BoundaryViolationError as e:
        return {"ok": False, "data": {"violation": True, "reason": e.reason}}
    return {"ok": True, "data": {"violation": False, "allowed": ok}}


@router.post("/sessions/{session_id}/close")
async def close_session(session_id: str, workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    try:
        s = wf.close_session(session_id)
    except KeyError:
        raise HTTPException(404, f"team session not found")
    return {"ok": True, "data": _serialize(s)}


@router.get("/stats")
async def stats(workspace_path: Optional[str] = Query(None)):
    wf = _workflow(workspace_path)
    return {"ok": True, "data": wf.stats()}
