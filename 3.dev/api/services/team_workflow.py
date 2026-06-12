"""Services - 团队模式工作流 (Team Workflow)

2026-06-09 TASK-3.9 引入：Phase 3 W12 团队模式 4-5 Agent 并行

设计（按 plan）:
    主管 → 分配 → 并行执行 → 交叉审查 → 仲裁

    团队会话（TeamSession）状态机：
        DRAFT        - 刚创建，未分配任务
        PLANNING     - 主管在拆解需求
        ASSIGNED     - 任务已分配给子 Agent
        IN_PROGRESS  - 至少一个任务执行中
        REVIEWING    - 交叉审查阶段
        ARBITRATING  - 主管仲裁分歧
        DONE         - 全部任务完成
        CLOSED       - 归档

    任务（TaskItem）状态机：
        PENDING          - 等待开始
        IN_PROGRESS      - 执行中
        AWAITING_APPROVAL- 等待审批
        APPROVED         - 已批准
        REJECTED         - 被拒绝
        DONE             - 完成
        FAILED           - 失败

    审批节点（ApprovalNode）状态机：
        PENDING    - 等待决策
        APPROVED   - 已批准
        REJECTED   - 被拒绝

本模块职责：
    - TeamSession / TaskItem / ApprovalNode / Message / SharedContext 数据模型
    - 任务分发 + 状态机
    - 共享上下文（SharedContext）
    - 团队消息流（Message）
    - 审批节点 + 主管仲裁
    - 角色边界（调用 platformkit.shared.team_core.check_boundary）
    - 持久化（DurableStore 联动 + .yunji/team_sessions.json）

不负责：
    - LLM 调用（路由层 routes/team.py 负责）
    - 实际工具执行（路由层负责）
    - 交叉审查算法（路由层负责发起）

使用示例：
    team = TeamWorkflow(workspace_path)

    # 1. 创建团队会话
    session = team.create_session("实现登录功能")

    # 2. 主管拆解任务（4-5 个 Agent 角色）
    t1 = team.add_task(session.id, "设计登录 API", "RESTful 设计 + 风险评估", AgentRole.ARCHITECT)
    t2 = team.add_task(session.id, "写登录后端", "实现 /login 端点", AgentRole.DEVELOPER, dependencies=[t1.id])
    t3 = team.add_task(session.id, "写登录测试", "单元测试 + 集成测试", AgentRole.TESTER, dependencies=[t2.id])
    t4 = team.add_task(session.id, "写登录文档", "API 文档 + 部署文档", AgentRole.DOCUMENTER, dependencies=[t2.id])

    # 3. 派发任务（主管→开发/测试/文档）
    team.assign_task(session.id, t2.id, "dev-agent-1")
    team.start_task(session.id, t2.id)
    team.complete_task(session.id, t2.id, output="实现完成")

    # 4. 触发审批节点
    aid = team.request_approval(session.id, t2.id, AgentRole.COORDINATOR, "需要主管审批合并")

    # 5. 主管仲裁
    team.decide_approval(session.id, aid, approve=True, comment="代码质量 OK，批准合并")
"""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from platformkit.shared.durable_store import DurableStore
from platformkit.shared.team_core import (
    AgentRole,
    BoundaryViolationError,
    REVIEW_PAIRS,
    check_boundary,
    parse_role,
)


# ──────────── 状态枚举 ────────────


class TeamStatus(str, Enum):
    """团队会话状态机"""

    DRAFT = "draft"                # 刚创建，未分配任务
    PLANNING = "planning"          # 主管在拆解需求
    ASSIGNED = "assigned"          # 任务已分配
    IN_PROGRESS = "in_progress"    # 至少一个任务执行中
    REVIEWING = "reviewing"        # 交叉审查阶段
    ARBITRATING = "arbitrating"    # 主管仲裁分歧
    DONE = "done"                  # 全部完成
    FAILED = "failed"              # 失败
    CLOSED = "closed"              # 归档


class TaskStatus(str, Enum):
    """任务状态机"""

    PENDING = "pending"                  # 等待开始
    IN_PROGRESS = "in_progress"          # 执行中
    AWAITING_APPROVAL = "awaiting_approval"  # 等待审批
    APPROVED = "approved"                # 已批准
    REJECTED = "rejected"                # 被拒绝
    DONE = "done"                        # 完成
    FAILED = "failed"                    # 失败
    SKIPPED = "skipped"                  # 跳过


class ApprovalStatus(str, Enum):
    """审批节点状态机"""

    PENDING = "pending"        # 等待决策
    APPROVED = "approved"      # 已批准
    REJECTED = "rejected"      # 被拒绝


class MessageType(str, Enum):
    """消息类型"""

    CHAT = "chat"              # 普通对话
    STATUS = "status"          # 状态变化
    TASK = "task"              # 任务相关
    APPROVAL = "approval"      # 审批相关
    REVIEW = "review"          # 审查相关
    SYSTEM = "system"          # 系统消息


# ──────────── 数据模型 ────────────


@dataclass
class TaskItem:
    """任务项"""

    id: str
    title: str
    description: str
    role: AgentRole
    dependencies: List[str] = field(default_factory=list)  # 依赖的其他 task id
    status: TaskStatus = TaskStatus.PENDING
    assigned_agent: Optional[str] = None
    output: Optional[str] = None
    error: Optional[str] = None
    file_paths: List[str] = field(default_factory=list)  # 涉及的文件路径
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    started_at: Optional[float] = None
    completed_at: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["role"] = self.role.value
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "TaskItem":
        return cls(
            id=d["id"],
            title=d["title"],
            description=d["description"],
            role=AgentRole(d.get("role", "developer")),
            dependencies=d.get("dependencies", []),
            status=TaskStatus(d.get("status", "pending")),
            assigned_agent=d.get("assigned_agent"),
            output=d.get("output"),
            error=d.get("error"),
            file_paths=d.get("file_paths", []),
            created_at=d.get("created_at", time.time()),
            updated_at=d.get("updated_at", time.time()),
            started_at=d.get("started_at"),
            completed_at=d.get("completed_at"),
        )


@dataclass
class ApprovalNode:
    """审批节点"""

    id: str
    task_id: str
    approver_role: AgentRole
    reason: str
    status: ApprovalStatus = ApprovalStatus.PENDING
    decision_comment: Optional[str] = None
    requested_at: float = field(default_factory=time.time)
    decided_at: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["approver_role"] = self.approver_role.value
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "ApprovalNode":
        return cls(
            id=d["id"],
            task_id=d["task_id"],
            approver_role=AgentRole(d.get("approver_role", "coordinator")),
            reason=d["reason"],
            status=ApprovalStatus(d.get("status", "pending")),
            decision_comment=d.get("decision_comment"),
            requested_at=d.get("requested_at", time.time()),
            decided_at=d.get("decided_at"),
        )


@dataclass
class SharedContext:
    """共享上下文（团队成员共享的事实）"""

    key: str
    value: str
    source: AgentRole  # 写入者角色
    updated_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["source"] = self.source.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "SharedContext":
        return cls(
            key=d["key"],
            value=d["value"],
            source=AgentRole(d.get("source", "coordinator")),
            updated_at=d.get("updated_at", time.time()),
        )


@dataclass
class Message:
    """团队消息流"""

    id: str
    role: AgentRole
    content: str
    type: MessageType = MessageType.CHAT
    task_id: Optional[str] = None  # 关联到某个任务
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["role"] = self.role.value
        d["type"] = self.type.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "Message":
        return cls(
            id=d["id"],
            role=AgentRole(d.get("role", "coordinator")),
            content=d["content"],
            type=MessageType(d.get("type", "chat")),
            task_id=d.get("task_id"),
            ts=d.get("ts", time.time()),
        )


@dataclass
class TeamSession:
    """团队会话"""

    id: str
    requirement: str
    status: TeamStatus = TeamStatus.DRAFT
    tasks: List[TaskItem] = field(default_factory=list)
    approvals: List[ApprovalNode] = field(default_factory=list)
    shared_context: List[SharedContext] = field(default_factory=list)
    messages: List[Message] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    started_at: Optional[float] = None
    completed_at: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["status"] = self.status.value
        d["tasks"] = [t.to_dict() for t in self.tasks]
        d["approvals"] = [a.to_dict() for a in self.approvals]
        d["shared_context"] = [s.to_dict() for s in self.shared_context]
        d["messages"] = [m.to_dict() for m in self.messages]
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "TeamSession":
        return cls(
            id=d["id"],
            requirement=d["requirement"],
            status=TeamStatus(d.get("status", "draft")),
            tasks=[TaskItem.from_dict(x) for x in d.get("tasks", [])],
            approvals=[ApprovalNode.from_dict(x) for x in d.get("approvals", [])],
            shared_context=[SharedContext.from_dict(x) for x in d.get("shared_context", [])],
            messages=[Message.from_dict(x) for x in d.get("messages", [])],
            created_at=d.get("created_at", time.time()),
            updated_at=d.get("updated_at", time.time()),
            started_at=d.get("started_at"),
            completed_at=d.get("completed_at"),
        )

    # ──────────── 派生状态 ────────────

    def task_progress(self) -> Dict[str, int]:
        """任务进度统计"""
        cnt = {s.value: 0 for s in TaskStatus}
        for t in self.tasks:
            cnt[t.status.value] += 1
        cnt["total"] = len(self.tasks)
        return cnt

    def role_distribution(self) -> Dict[str, int]:
        """角色分布"""
        cnt: Dict[str, int] = {r.value: 0 for r in AgentRole}
        for t in self.tasks:
            cnt[t.role.value] += 1
        return cnt

    def pending_approvals(self) -> List[ApprovalNode]:
        """待审批列表"""
        return [a for a in self.approvals if a.status == ApprovalStatus.PENDING]

    def is_ready_task(self, task: TaskItem) -> bool:
        """任务是否可启动（依赖全部完成）"""
        if not task.dependencies:
            return True
        # 已完成的状态（DONE 或 APPROVED 都算已完成且可被依赖）
        completed = {TaskStatus.DONE, TaskStatus.APPROVED, TaskStatus.SKIPPED}
        dep_map = {t.id: t for t in self.tasks}
        for dep_id in task.dependencies:
            dep = dep_map.get(dep_id)
            if not dep:
                return False
            if dep.status not in completed:
                return False
        return True


# ──────────── 工作流引擎 ────────────


# 持久化文件
_SESSIONS_FILE = "team_sessions.json"


class TeamWorkflow:
    """团队模式工作流引擎

    设计：4-5 个 Agent 角色并行工作，主管仲裁。
    """

    def __init__(
        self,
        workspace_path: Union[Path, str],
        store: Optional[DurableStore] = None,
    ) -> None:
        self.workspace_path: Path = Path(workspace_path).resolve()
        self.store: DurableStore = store or DurableStore(self.workspace_path)
        self._sessions: Dict[str, TeamSession] = {}
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
                session = TeamSession.from_dict(entry)
                self._sessions[session.id] = session
            except Exception:
                continue

    def _save_sessions(self) -> None:
        path = self.store.yunji_dir / _SESSIONS_FILE
        path.write_text(
            json.dumps(
                [s.to_dict() for s in self._sessions.values()],
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )

    def _touch(self, session: TeamSession) -> TeamSession:
        session.updated_at = time.time()
        self._save_sessions()
        return session

    # ──────────── 会话生命周期 ────────────

    def create_session(self, requirement: str) -> TeamSession:
        """创建团队会话（draft 状态）"""
        if not requirement.strip():
            raise ValueError("requirement cannot be empty")
        s = TeamSession(id=str(uuid.uuid4()), requirement=requirement.strip())
        s.status = TeamStatus.PLANNING  # 进入规划阶段
        self._sessions[s.id] = s
        # 主管发首条消息
        self.add_message(
            s.id,
            AgentRole.COORDINATOR,
            f"📋 新需求: {requirement}\n\n正在拆解为可执行子任务...",
            MessageType.SYSTEM,
        )
        return self._touch(s)

    def get_session(self, session_id: str) -> Optional[TeamSession]:
        return self._sessions.get(session_id)

    def list_sessions(self, status: Optional[TeamStatus] = None) -> List[TeamSession]:
        arr = list(self._sessions.values())
        if status:
            arr = [s for s in arr if s.status == status]
        return sorted(arr, key=lambda x: x.updated_at, reverse=True)

    def delete_session(self, session_id: str) -> bool:
        if session_id in self._sessions:
            self._sessions.pop(session_id)
            self._save_sessions()
            return True
        return False

    # ──────────── 任务管理 ────────────

    def add_task(
        self,
        session_id: str,
        title: str,
        description: str,
        role: Union[AgentRole, str],
        dependencies: Optional[List[str]] = None,
    ) -> TaskItem:
        """添加任务（主管拆解需求）"""
        s = self._get(session_id)
        if isinstance(role, str):
            role = parse_role(role)
        if not title.strip():
            raise ValueError("task title cannot be empty")
        task = TaskItem(
            id=str(uuid.uuid4())[:8],
            title=title.strip(),
            description=description.strip(),
            role=role,
            dependencies=dependencies or [],
        )
        s.tasks.append(task)
        self.add_message(
            session_id,
            AgentRole.COORDINATOR,
            f"📝 新增任务 [{task.id}] {role.emoji} {title}（{role.label}）",
            MessageType.TASK,
            task_id=task.id,
        )
        self._touch(s)
        return task

    def assign_task(self, session_id: str, task_id: str, agent_id: str) -> TaskItem:
        """派发任务到具体的 agent_id（主管→开发/测试/文档）"""
        s = self._get(session_id)
        task = self._find_task(s, task_id)
        if task.status not in (TaskStatus.PENDING, TaskStatus.IN_PROGRESS):
            raise ValueError(f"task {task_id} cannot be assigned from {task.status.value}")
        # 检查依赖
        if not s.is_ready_task(task):
            deps = ", ".join(task.dependencies)
            raise ValueError(f"task {task_id} has unfinished dependencies: {deps}")
        task.assigned_agent = agent_id
        task.status = TaskStatus.IN_PROGRESS
        task.started_at = time.time()
        if s.status == TeamStatus.PLANNING:
            s.status = TeamStatus.ASSIGNED
        if s.status in (TeamStatus.ASSIGNED, TeamStatus.DRAFT):
            s.status = TeamStatus.IN_PROGRESS
        if s.started_at is None:
            s.started_at = time.time()
        self.add_message(
            session_id,
            AgentRole.COORDINATOR,
            f"📤 派发任务 [{task_id}] → {agent_id}（{task.role.label}）",
            MessageType.TASK,
            task_id=task_id,
        )
        self._touch(s)
        return task

    def start_task(self, session_id: str, task_id: str) -> TaskItem:
        """子 Agent 开始执行（assign 后自动开始，也可单独调用）"""
        s = self._get(session_id)
        task = self._find_task(s, task_id)
        if task.status == TaskStatus.PENDING:
            task.status = TaskStatus.IN_PROGRESS
            task.started_at = time.time()
            if s.status in (TeamStatus.DRAFT, TeamStatus.PLANNING, TeamStatus.ASSIGNED):
                s.status = TeamStatus.IN_PROGRESS
            if s.started_at is None:
                s.started_at = time.time()
            self.add_message(
                session_id,
                task.role,
                f"▶️ 开始执行 [{task_id}] {task.title}",
                MessageType.STATUS,
                task_id=task_id,
            )
        return self._touch(s)._find_task(task_id)  # type: ignore

    def complete_task(
        self,
        session_id: str,
        task_id: str,
        output: Optional[str] = None,
        file_paths: Optional[List[str]] = None,
    ) -> TaskItem:
        """完成任务"""
        s = self._get(session_id)
        task = self._find_task(s, task_id)
        if task.status not in (TaskStatus.IN_PROGRESS, TaskStatus.APPROVED):
            raise ValueError(f"task {task_id} cannot be completed from {task.status.value}")
        # 边界检查（如有 file_paths）
        if file_paths:
            for fp in file_paths:
                check_boundary(task.role, fp, "write")
        task.status = TaskStatus.DONE
        task.output = output
        task.file_paths = file_paths or []
        task.completed_at = time.time()
        self.add_message(
            session_id,
            task.role,
            f"✅ 完成任务 [{task_id}] {task.title}" + (f"\n\n{output}" if output else ""),
            MessageType.TASK,
            task_id=task_id,
        )
        # 触发交叉审查（如果有审查对）
        reviewers = REVIEW_PAIRS.get(task.role, [])
        if reviewers and task.role != AgentRole.COORDINATOR:
            for reviewer in reviewers:
                if reviewer != AgentRole.COORDINATOR or True:  # 主管一定参与审查
                    self.add_message(
                        session_id,
                        reviewer,
                        f"🔍 收到交叉审查请求 [{task_id}] {task.title}",
                        MessageType.REVIEW,
                        task_id=task_id,
                    )
            if s.status == TeamStatus.IN_PROGRESS:
                s.status = TeamStatus.REVIEWING
        # 检查全部完成
        if all(t.status in (TaskStatus.DONE, TaskStatus.APPROVED, TaskStatus.SKIPPED, TaskStatus.REJECTED) for t in s.tasks):
            s.status = TeamStatus.DONE
            s.completed_at = time.time()
        return self._touch(s)._find_task(task_id)  # type: ignore

    def fail_task(self, session_id: str, task_id: str, error: str) -> TaskItem:
        """任务失败"""
        s = self._get(session_id)
        task = self._find_task(s, task_id)
        if task.status != TaskStatus.IN_PROGRESS:
            raise ValueError(f"task {task_id} not in progress")
        task.status = TaskStatus.FAILED
        task.error = error
        self.add_message(
            session_id,
            task.role,
            f"❌ 任务失败 [{task_id}] {task.title}\n\n{error}",
            MessageType.TASK,
            task_id=task_id,
        )
        # 升级到主管仲裁
        s.status = TeamStatus.ARBITRATING
        self.add_message(
            session_id,
            AgentRole.COORDINATOR,
            f"⚠️ 收到失败报告 [{task_id}]，启动仲裁流程",
            MessageType.SYSTEM,
            task_id=task_id,
        )
        return self._touch(s)._find_task(task_id)  # type: ignore

    # ──────────── 审批节点 ────────────

    def request_approval(
        self,
        session_id: str,
        task_id: str,
        approver_role: Union[AgentRole, str],
        reason: str,
    ) -> ApprovalNode:
        """触发审批节点"""
        s = self._get(session_id)
        task = self._find_task(s, task_id)
        if isinstance(approver_role, str):
            approver_role = parse_role(approver_role)
        # 如果 task 还在 IN_PROGRESS 状态，把状态切到 AWAITING_APPROVAL
        # 如果 task 已经 DONE（开发完成后申请合并），保持 DONE 不变
        if task.status == TaskStatus.IN_PROGRESS:
            task.status = TaskStatus.AWAITING_APPROVAL
        node = ApprovalNode(
            id=str(uuid.uuid4())[:8],
            task_id=task_id,
            approver_role=approver_role,
            reason=reason,
        )
        s.approvals.append(node)
        self.add_message(
            session_id,
            task.role,
            f"⏸ 触发审批 [{node.id}] → {approver_role.label_full}\n\n原因: {reason}",
            MessageType.APPROVAL,
            task_id=task_id,
        )
        if s.status not in (TeamStatus.ARBITRATING,):
            s.status = TeamStatus.REVIEWING
        return self._touch(s)._find_approval(node.id)  # type: ignore

    def decide_approval(
        self,
        session_id: str,
        approval_id: str,
        approve: bool,
        comment: Optional[str] = None,
    ) -> ApprovalNode:
        """主管/审查者做决策"""
        s = self._get(session_id)
        node = self._find_approval(s, approval_id)
        if node.status != ApprovalStatus.PENDING:
            raise ValueError(f"approval {approval_id} already decided")
        node.status = ApprovalStatus.APPROVED if approve else ApprovalStatus.REJECTED
        node.decision_comment = comment
        node.decided_at = time.time()
        # 更新关联任务状态
        task = self._find_task(s, node.task_id)
        if approve:
            # 如果 task 在 AWAITING_APPROVAL，批准后变为 APPROVED
            # 如果 task 已经是 DONE（开发完成后申请合并），保持 DONE
            if task.status == TaskStatus.AWAITING_APPROVAL:
                task.status = TaskStatus.APPROVED
        else:
            task.status = TaskStatus.REJECTED
        self.add_message(
            session_id,
            node.approver_role,
            f"{'✅' if approve else '❌'} 审批 [{approval_id}] → {'通过' if approve else '拒绝'}"
            + (f"\n\n{comment}" if comment else ""),
            MessageType.APPROVAL,
            task_id=node.task_id,
        )
        # 检查是否所有任务都已决策
        pending_approvals = [a for a in s.approvals if a.status == ApprovalStatus.PENDING]
        if not pending_approvals and s.status == TeamStatus.REVIEWING:
            # 没有更多待审批，回到 IN_PROGRESS 或 DONE
            if all(t.status in (TaskStatus.DONE, TaskStatus.APPROVED, TaskStatus.SKIPPED, TaskStatus.REJECTED) for t in s.tasks):
                s.status = TeamStatus.DONE
                s.completed_at = time.time()
            else:
                s.status = TeamStatus.IN_PROGRESS
        return self._touch(s)._find_approval(approval_id)  # type: ignore

    # ──────────── 团队消息流 ────────────

    def add_message(
        self,
        session_id: str,
        role: Union[AgentRole, str],
        content: str,
        type: Union[MessageType, str] = MessageType.CHAT,
        task_id: Optional[str] = None,
    ) -> Message:
        """添加团队消息"""
        s = self._get(session_id)
        if isinstance(role, str):
            role = parse_role(role)
        if isinstance(type, str):
            type = MessageType(type)
        msg = Message(
            id=str(uuid.uuid4())[:8],
            role=role,
            content=content,
            type=type,
            task_id=task_id,
        )
        s.messages.append(msg)
        return self._touch(s)._find_message(msg.id)  # type: ignore

    # ──────────── 共享上下文 ────────────

    def update_shared_context(
        self,
        session_id: str,
        key: str,
        value: str,
        source: Union[AgentRole, str],
    ) -> SharedContext:
        """更新共享上下文（key 不存在则新增，存在则覆盖）"""
        s = self._get(session_id)
        if isinstance(source, str):
            source = parse_role(source)
        # 查找现有 key
        for ctx in s.shared_context:
            if ctx.key == key:
                ctx.value = value
                ctx.source = source
                ctx.updated_at = time.time()
                self.add_message(
                    session_id,
                    source,
                    f"📌 更新共享上下文: {key} = {value[:80]}{'...' if len(value) > 80 else ''}",
                    MessageType.STATUS,
                )
                return self._touch(s)._find_context(key)  # type: ignore
        # 新增
        ctx = SharedContext(key=key, value=value, source=source)
        s.shared_context.append(ctx)
        self.add_message(
            session_id,
            source,
            f"📌 新增共享上下文: {key} = {value[:80]}{'...' if len(value) > 80 else ''}",
            MessageType.STATUS,
        )
        return self._touch(s)._find_context(key)  # type: ignore

    # ──────────── 边界检查（路由层调）────────────

    def check_role_boundary(
        self,
        role: Union[AgentRole, str],
        path: str,
        action: str = "write",
    ) -> bool:
        """检查角色边界（外部工具调度前必调）"""
        if isinstance(role, str):
            role = parse_role(role)
        return check_boundary(role, path, action)

    # ──────────── 关闭 / 统计 ────────────

    def close_session(self, session_id: str) -> TeamSession:
        """归档会话（写入 DurableStore memory）"""
        s = self._get(session_id)
        if s.status == TeamStatus.DONE:
            s.status = TeamStatus.CLOSED
        else:
            s.status = TeamStatus.CLOSED
        # 写一条 memory
        try:
            progress = s.task_progress()
            self.store.append_memory(
                f"## 团队会话 {s.id[:8]} 关闭\n\n"
                f"需求: {s.requirement}\n\n"
                f"状态: {s.status.value}\n\n"
                f"任务: {progress['total']} | 完成: {progress['done']} | 失败: {progress['failed']} | 拒绝: {progress['rejected']}\n\n"
                f"审批节点: {len(s.approvals)} 个\n\n"
                f"消息: {len(s.messages)} 条\n"
            )
        except Exception:
            pass
        return self._touch(s)

    def stats(self) -> Dict[str, Any]:
        """全局统计"""
        sessions = list(self._sessions.values())
        by_status: Dict[str, int] = {s.value: 0 for s in TeamStatus}
        for s in sessions:
            by_status[s.status.value] += 1
        total_tasks = sum(len(s.tasks) for s in sessions)
        total_approvals = sum(len(s.approvals) for s in sessions)
        total_messages = sum(len(s.messages) for s in sessions)
        # 角色分布
        role_dist: Dict[str, int] = {r.value: 0 for r in AgentRole}
        for s in sessions:
            for t in s.tasks:
                role_dist[t.role.value] += 1
        return {
            "total_sessions": len(sessions),
            "by_status": by_status,
            "total_tasks": total_tasks,
            "total_approvals": total_approvals,
            "total_messages": total_messages,
            "role_distribution": role_dist,
        }

    # ──────────── 内部辅助 ────────────

    def _get(self, session_id: str) -> TeamSession:
        s = self._sessions.get(session_id)
        if not s:
            raise KeyError(f"team session not found: {session_id}")
        return s

    def _find_task(self, session: TeamSession, task_id: str) -> TaskItem:
        for t in session.tasks:
            if t.id == task_id:
                return t
        raise KeyError(f"task not found: {task_id}")

    def _find_approval(self, session: TeamSession, approval_id: str) -> ApprovalNode:
        for a in session.approvals:
            if a.id == approval_id:
                return a
        raise KeyError(f"approval not found: {approval_id}")

    def _find_message(self, msg_id: str) -> Message:
        for s in self._sessions.values():
            for m in s.messages:
                if m.id == msg_id:
                    return m
        raise KeyError(f"message not found: {msg_id}")

    def _find_context(self, key: str) -> SharedContext:
        for s in self._sessions.values():
            for c in s.shared_context:
                if c.key == key:
                    return c
        raise KeyError(f"context not found: {key}")


# 给 TeamSession 加内部查找方法（为了链式 _touch()._find_xxx()）
def _team_session_find_task(self: TeamSession, task_id: str) -> TaskItem:
    for t in self.tasks:
        if t.id == task_id:
            return t
    raise KeyError(f"task not found: {task_id}")


def _team_session_find_approval(self: TeamSession, approval_id: str) -> ApprovalNode:
    for a in self.approvals:
        if a.id == approval_id:
            return a
    raise KeyError(f"approval not found: {approval_id}")


def _team_session_find_message(self: TeamSession, msg_id: str) -> Message:
    for m in self.messages:
        if m.id == msg_id:
            return m
    raise KeyError(f"message not found: {msg_id}")


def _team_session_find_context(self: TeamSession, key: str) -> SharedContext:
    for c in self.shared_context:
        if c.key == key:
            return c
    raise KeyError(f"context not found: {key}")


TeamSession._find_task = _team_session_find_task  # type: ignore
TeamSession._find_approval = _team_session_find_approval  # type: ignore
TeamSession._find_message = _team_session_find_message  # type: ignore
TeamSession._find_context = _team_session_find_context  # type: ignore
