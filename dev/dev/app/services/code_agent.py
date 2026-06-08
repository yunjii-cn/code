"""Services - 独行模式 Code Agent (单人 / 单会话)

2026-06-09 TASK-3.8 引入：Phase 3 W11 独行模式 Agent

设计（按 plan）:
    4 阶段循环: Plan → Execute → Review → Learn

    Plan 阶段：
        1. 用户输入需求
        2. AI 生成开发计划（to-do list 形式）
        3. 用户逐条 approve / reject
    Execute 阶段：
        4. AI 按步骤执行（路由层做 tool 调度，本模块做状态机）
        5. 每步记录 tool 调用 + 结果
    Review 阶段：
        6. DiffView 查看变更（路由层做，本模块保存 diff 摘要）
    Learn 阶段：
        7. 自动提炼知识（KnowledgeEngine.add_*）

本模块职责：
    - CodeSession 状态机（draft/planned/approved/executing/done/failed）
    - PlanStep 状态机（pending/approved/rejected/executing/done/failed）
    - 持久化（DurableStore）
    - 知识吸收（KnowledgeEngine）
    - Diff 摘要记录

不负责：
    - LLM 调用（路由层 routes/agent.py 负责）
    - 实际 tool 执行（路由层负责，调用 system/terminal/run 等）
    - AI diff 渲染（前端 DiffView 负责）

使用示例：
    agent = CodeAgent(workspace_path)
    session = agent.create_session("实现登录功能")
    session = agent.set_plan(session.id, ["写 User 模型", "加 /login 端点", "写测试"])
    agent.approve_step(session.id, step_ids[0])
    result = session  # tool 调用路由层做，然后 execute_step 回调
"""

from __future__ import annotations

import json
import re
import time
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from platformkit.shared.durable_store import DurableStore
from platformkit.shared.knowledge_core import (
    KnowledgeEngine,
    KnowledgeLayer,
)


# ──────────── 状态枚举 ────────────


class SessionStatus(str, Enum):
    DRAFT = "draft"          # 刚创建，未生成计划
    PLANNED = "planned"      # 计划已生成，等待审批
    APPROVED = "approved"    # 至少一个步骤被批准
    EXECUTING = "executing"  # 正在执行
    DONE = "done"            # 全部步骤完成
    FAILED = "failed"        # 失败
    LEARNING = "learning"    # Learn 阶段
    CLOSED = "closed"        # 结束归档


class StepStatus(str, Enum):
    PENDING = "pending"      # 等待用户审批
    APPROVED = "approved"    # 用户批准
    REJECTED = "rejected"    # 用户拒绝
    EXECUTING = "executing"  # 正在执行
    DONE = "done"            # 执行完成
    FAILED = "failed"        # 执行失败
    SKIPPED = "skipped"      # 被跳过（被前序拒绝或用户主动跳过）


# ──────────── 数据模型 ────────────


@dataclass
class PlanStep:
    id: str
    description: str
    status: StepStatus = StepStatus.PENDING
    tool_calls: List[Dict[str, Any]] = field(default_factory=list)  # [{name, args, result}]
    result: Optional[str] = None
    reject_reason: Optional[str] = None
    approved_at: Optional[float] = None
    executed_at: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "PlanStep":
        return cls(
            id=d["id"],
            description=d["description"],
            status=StepStatus(d.get("status", "pending")),
            tool_calls=d.get("tool_calls", []),
            result=d.get("result"),
            reject_reason=d.get("reject_reason"),
            approved_at=d.get("approved_at"),
            executed_at=d.get("executed_at"),
        )


@dataclass
class DiffEntry:
    step_id: str
    file_path: str
    added_lines: int = 0
    removed_lines: int = 0
    summary: str = ""  # 简短摘要（路由层填）

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "DiffEntry":
        return cls(
            step_id=d["step_id"],
            file_path=d["file_path"],
            added_lines=d.get("added_lines", 0),
            removed_lines=d.get("removed_lines", 0),
            summary=d.get("summary", ""),
        )


@dataclass
class LearningEntry:
    layer: KnowledgeLayer  # L1-L4
    content: str
    step_id: Optional[str] = None
    source: str = "code_agent"  # 来源标识

    def to_dict(self) -> Dict[str, Any]:
        return {"layer": self.layer.value, "content": self.content, "step_id": self.step_id, "source": self.source}


@dataclass
class CodeSession:
    id: str
    requirement: str
    status: SessionStatus = SessionStatus.DRAFT
    plan: List[PlanStep] = field(default_factory=list)
    diffs: List[DiffEntry] = field(default_factory=list)
    learnings: List[LearningEntry] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    started_at: Optional[float] = None
    completed_at: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["status"] = self.status.value
        d["plan"] = [s.to_dict() for s in self.plan]
        d["diffs"] = [e.to_dict() for e in self.diffs]
        d["learnings"] = [e.to_dict() for e in self.learnings]
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "CodeSession":
        return cls(
            id=d["id"],
            requirement=d["requirement"],
            status=SessionStatus(d.get("status", "draft")),
            plan=[PlanStep.from_dict(x) for x in d.get("plan", [])],
            diffs=[DiffEntry.from_dict(x) for x in d.get("diffs", [])],
            learnings=[LearningEntry(**x) if x.get("layer") else None for x in d.get("learnings", [])],  # type: ignore
            created_at=d.get("created_at", time.time()),
            updated_at=d.get("updated_at", time.time()),
            started_at=d.get("started_at"),
            completed_at=d.get("completed_at"),
        )

    # ──────────── 派生状态 ────────────

    def progress(self) -> Dict[str, int]:
        """统计进度：{total, pending, approved, rejected, executing, done, failed, skipped}"""
        cnt = {s.value: 0 for s in StepStatus}
        for step in self.plan:
            cnt[step.status.value] += 1
        cnt["total"] = len(self.plan)
        return cnt


# ──────────── Agent ────────────


# 会话持久化文件
_SESSIONS_FILE = "code_sessions.json"


class CodeAgent:
    """独行模式 Agent

    单会话：一次专注完成一个功能（plan 强制约束：一个会话一个功能）。
    """

    def __init__(
        self,
        workspace_path: Union[Path, str],
        store: Optional[DurableStore] = None,
        knowledge: Optional[KnowledgeEngine] = None,
    ) -> None:
        self.workspace_path: Path = Path(workspace_path).resolve()
        self.store: DurableStore = store or DurableStore(self.workspace_path)
        # KnowledgeEngine 单独用 env YUNJI_GLOBAL_ROOT 路径
        self.knowledge: KnowledgeEngine = knowledge or KnowledgeEngine(
            project_root=self.workspace_path
        )
        # 会话缓存
        self._sessions: Dict[str, CodeSession] = {}
        self._load_sessions()

    # ──────────── 持久化 ────────────

    def _load_sessions(self) -> None:
        path = self.store.yunji_dir / _SESSIONS_FILE
        if not path.exists():
            return
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return
        if not isinstance(data, list):
            return
        for entry in data:
            try:
                session = CodeSession.from_dict(entry)
                self._sessions[session.id] = session
            except Exception:
                continue

    def _save_sessions(self) -> None:
        path = self.store.yunji_dir / _SESSIONS_FILE
        path.write_text(
            json.dumps([s.to_dict() for s in self._sessions.values()], ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

    def _touch(self, session: CodeSession) -> CodeSession:
        session.updated_at = time.time()
        self._save_sessions()
        return session

    # ──────────── 状态机 ────────────

    def create_session(self, requirement: str) -> CodeSession:
        """创建会话（draft 状态）"""
        if not requirement.strip():
            raise ValueError("requirement cannot be empty")
        s = CodeSession(id=str(uuid.uuid4()), requirement=requirement.strip())
        self._sessions[s.id] = s
        self._touch(s)
        return s

    def set_plan(self, session_id: str, step_descriptions: List[str]) -> CodeSession:
        """设置计划（draft → planned）

        step_descriptions: AI 生成的步骤描述列表
        """
        s = self._get(session_id)
        s.plan = [
            PlanStep(id=str(uuid.uuid4())[:8], description=desc.strip())
            for desc in step_descriptions
            if desc.strip()
        ]
        if not s.plan:
            raise ValueError("plan must contain at least one step")
        s.status = SessionStatus.PLANNED
        return self._touch(s)

    def append_step(self, session_id: str, description: str) -> CodeSession:
        """在 plan 末尾追加一个 step"""
        s = self._get(session_id)
        s.plan.append(PlanStep(id=str(uuid.uuid4())[:8], description=description.strip()))
        if s.status == SessionStatus.DRAFT:
            s.status = SessionStatus.PLANNED
        return self._touch(s)

    def approve_step(self, session_id: str, step_id: str) -> CodeSession:
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status not in (StepStatus.PENDING, StepStatus.REJECTED):
            raise ValueError(f"step {step_id} cannot be approved from {step.status.value}")
        step.status = StepStatus.APPROVED
        step.approved_at = time.time()
        step.reject_reason = None
        if s.status in (SessionStatus.DRAFT, SessionStatus.PLANNED):
            s.status = SessionStatus.APPROVED
        return self._touch(s)

    def reject_step(self, session_id: str, step_id: str, reason: Optional[str] = None) -> CodeSession:
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status not in (StepStatus.PENDING,):
            raise ValueError(f"step {step_id} cannot be rejected from {step.status.value}")
        step.status = StepStatus.REJECTED
        step.reject_reason = reason
        return self._touch(s)

    def start_execute(self, session_id: str, step_id: str) -> CodeSession:
        """开始执行某个已批准步骤"""
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status != StepStatus.APPROVED:
            raise ValueError(f"step {step_id} not approved")
        step.status = StepStatus.EXECUTING
        s.status = SessionStatus.EXECUTING
        if s.started_at is None:
            s.started_at = time.time()
        return self._touch(s)

    def record_tool_call(
        self, session_id: str, step_id: str, tool_name: str, args: Dict[str, Any], result: str
    ) -> CodeSession:
        """记录一次 tool 调用结果"""
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status != StepStatus.EXECUTING:
            raise ValueError(f"step {step_id} not executing")
        step.tool_calls.append({
            "name": tool_name,
            "args": args,
            "result": result,
            "ts": time.time(),
        })
        return self._touch(s)

    def complete_step(self, session_id: str, step_id: str, result_summary: Optional[str] = None) -> CodeSession:
        """完成步骤（executing → done）"""
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status != StepStatus.EXECUTING:
            raise ValueError(f"step {step_id} not executing")
        step.status = StepStatus.DONE
        step.executed_at = time.time()
        step.result = result_summary
        # 检查 session 是否全部完成
        if all(st.status in (StepStatus.DONE, StepStatus.SKIPPED, StepStatus.REJECTED) for st in s.plan):
            s.status = SessionStatus.DONE
            s.completed_at = time.time()
        return self._touch(s)

    def fail_step(self, session_id: str, step_id: str, error: str) -> CodeSession:
        s = self._get(session_id)
        step = self._find_step(s, step_id)
        if step.status != StepStatus.EXECUTING:
            raise ValueError(f"step {step_id} not executing")
        step.status = StepStatus.FAILED
        step.result = error
        s.status = SessionStatus.FAILED
        return self._touch(s)

    def record_diff(self, session_id: str, diff: DiffEntry) -> CodeSession:
        s = self._get(session_id)
        s.diffs.append(diff)
        return self._touch(s)

    # ──────────── Learn 阶段 ────────────

    def add_learning(
        self, session_id: str, layer: Union[KnowledgeLayer, str], content: str, step_id: Optional[str] = None
    ) -> CodeSession:
        s = self._get(session_id)
        if s.status != SessionStatus.DONE:
            # 自动设置 Learning 状态（仅一次）
            if s.status not in (SessionStatus.LEARNING,):
                s.status = SessionStatus.LEARNING
        layer_e = layer if isinstance(layer, KnowledgeLayer) else KnowledgeLayer(layer)
        entry = LearningEntry(layer=layer_e, content=content, step_id=step_id)
        s.learnings.append(entry)
        # 自动写入知识库
        try:
            self.knowledge._add(content, layer_e, "project", "weak", "code_agent")  # 弱关联
        except Exception:
            pass
        return self._touch(s)

    def close_session(self, session_id: str) -> CodeSession:
        """归档会话（写入 DurableStore memory）"""
        s = self._get(session_id)
        if s.status == SessionStatus.DONE:
            s.status = SessionStatus.LEARNING
        s.status = SessionStatus.CLOSED
        # 写一条 memory
        try:
            progress = s.progress()
            self.store.append_memory(
                f"## 会话 {s.id[:8]} 完成\n\n"
                f"需求: {s.requirement}\n\n"
                f"步骤: {progress['total']} | 已完成: {progress['done']} | 失败: {progress['failed']} | 拒绝: {progress['rejected']}\n\n"
                f"Learnings: {len(s.learnings)} 条\n"
            )
            # 更新 plan.md
            plan_md = f"# 当前计划\n\n## {s.requirement}\n\n"
            for i, step in enumerate(s.plan, 1):
                mark = "x" if step.status == StepStatus.DONE else " "
                plan_md += f"- [{mark}] {step.description}\n"
            self.store.update_plan(plan_md)
        except Exception:
            pass
        return self._touch(s)

    # ──────────── 查询 ────────────

    def get_session(self, session_id: str) -> Optional[CodeSession]:
        s = self._sessions.get(session_id)
        return s

    def list_sessions(self, status: Optional[SessionStatus] = None) -> List[CodeSession]:
        arr = list(self._sessions.values())
        if status:
            arr = [s for s in arr if s.status == status]
        return sorted(arr, key=lambda x: x.updated_at, reverse=True)

    def stats(self) -> Dict[str, Any]:
        """全局统计"""
        sessions = list(self._sessions.values())
        by_status: Dict[str, int] = {s.value: 0 for s in SessionStatus}
        for s in sessions:
            by_status[s.status.value] += 1
        total_learnings = sum(len(s.learnings) for s in sessions)
        total_diffs = sum(len(s.diffs) for s in sessions)
        return {
            "total_sessions": len(sessions),
            "by_status": by_status,
            "total_learnings": total_learnings,
            "total_diffs": total_diffs,
        }

    # ──────────── 内部辅助 ────────────

    def _get(self, session_id: str) -> CodeSession:
        s = self._sessions.get(session_id)
        if not s:
            raise KeyError(f"session not found: {session_id}")
        return s

    def _find_step(self, session: CodeSession, step_id: str) -> PlanStep:
        for step in session.plan:
            if step.id == step_id:
                return step
        raise KeyError(f"step not found: {step_id}")

    # ──────────── 解析辅助（路由层调）────────────

    @staticmethod
    def parse_plan_text(text: str) -> List[str]:
        """解析 AI 生成的 plan 文本（支持 markdown 列表 / 数字列表 / 换行分割）

        接受：
            "1. 写 User 模型\n2. 加 /login 端点\n3. 写测试"
            "- 写 User 模型\n- 加 /login 端点\n- 写测试"
            "写 User 模型\\n加 /login 端点\\n写测试"
        """
        items: List[str] = []
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            # 去掉 markdown 列表前缀
            line = re.sub(r"^[\-\*]\s+", "", line)
            line = re.sub(r"^\d+[\.\)]\s*", "", line)
            if line:
                items.append(line)
        return items
