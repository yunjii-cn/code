"""Platform Shared - 团队模式角色系统 (Team Core)

2026-06-09 TASK-3.9 引入：Phase 3 W12 团队模式 4-5 Agent 并行

设计（按 plan）:
    5 个角色 + 边界治理：

    | 角色        | 写入范围           | 职责               |
    |------------|------------------|-------------------|
    | COORDINATOR | 集成和仲裁        | 任务分配、冲突解决、最终验证 |
    | ARCHITECT   | docs/design/     | 设计简报、风险热点、文件影响图 |
    | DEVELOPER   | app/             | 写生产代码，最小安全实现     |
    | TESTER      | tests/           | 测试覆盖率、回归保护         |
    | DOCUMENTER  | docs/            | API 文档、部署文档          |

    关键规则：
        1. 一个会话一个功能（plan 强制约束）
        2. 角色边界不可逾越（write_file 前校验角色权限）
        3. 交叉审查（开发完成 → 测试自动审查）
        4. 主管仲裁（角色分歧由 COORDINATOR 决定）
        5. 知识共享（一个角色学到的经验，所有角色共享）

本模块职责：
    - AgentRole 枚举
    - ROLE_BOUNDARIES 写入范围规则
    - BoundaryViolationError 异常
    - check_boundary(role, file_path, action) 边界检查
    - ROLE_PROMPTS 角色 prompt 模板（路由层/LLM 层用）

不负责：
    - TeamWorkflow 业务编排（在 services/team_workflow.py）
    - LLM 调度（路由层 routes/team.py 负责）
    - UI 渲染（前端 features/team/* 负责）

使用示例：
    from platformkit.shared.team_core import AgentRole, check_boundary

    try:
        check_boundary(AgentRole.DEVELOPER, "tests/test_login.py", "write")
    except BoundaryViolationError as e:
        print(f"边界违规: {e}")
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from typing import Dict, FrozenSet, List, Optional, Set


# ──────────── 角色枚举 ────────────


class AgentRole(str, Enum):
    """团队模式 5 种角色"""

    COORDINATOR = "coordinator"   # 🎯 主管
    ARCHITECT = "architect"       # 🏗️ 架构师（只读）
    DEVELOPER = "developer"       # 💻 开发工程师
    TESTER = "tester"             # 🧪 测试工程师
    DOCUMENTER = "documenter"     # 📝 文档工程师

    @property
    def emoji(self) -> str:
        return {
            AgentRole.COORDINATOR: "🎯",
            AgentRole.ARCHITECT: "🏗️",
            AgentRole.DEVELOPER: "💻",
            AgentRole.TESTER: "🧪",
            AgentRole.DOCUMENTER: "📝",
        }[self]

    @property
    def label(self) -> str:
        return {
            AgentRole.COORDINATOR: "主管",
            AgentRole.ARCHITECT: "架构师",
            AgentRole.DEVELOPER: "开发",
            AgentRole.TESTER: "测试",
            AgentRole.DOCUMENTER: "文档",
        }[self]

    @property
    def label_full(self) -> str:
        return f"{self.emoji} {self.label}"


# ──────────── 边界规则 ────────────


# 允许的目录 / 禁止的目录（前缀匹配）
@dataclass(frozen=True)
class BoundaryRule:
    """角色边界规则

    allowed_prefixes: 允许写入的路径前缀列表（None = 不限制）
    denied_prefixes: 禁止写入的路径前缀列表（最高优先级，匹配直接拒绝）
    read_only: 是否只读
    """

    allowed_prefixes: Optional[FrozenSet[str]] = None
    denied_prefixes: FrozenSet[str] = frozenset()
    read_only: bool = False


# 跨平台路径分隔符
def _norm(path: str) -> str:
    """统一路径分隔符为 /"""
    return path.replace("\\", "/").lstrip("/")


def _match_prefix(path: str, prefixes: FrozenSet[str]) -> bool:
    """检查 path 是否以 prefixes 中任一前缀开头（前缀匹配）

    规则：
        - 完全相等
        - 或 path 以前缀开始，且前缀结尾是 / 或者 path 的下一个字符是 /
    """
    np = _norm(path)
    for p in prefixes:
        np_p = _norm(p)
        if np == np_p:
            return True
        if np.startswith(np_p):
            # 前缀末尾必须是 /（避免 appx 匹配 app）
            if np_p.endswith("/"):
                return True
            # 或 path 的下一个字符是 /
            if len(np) > len(np_p) and np[len(np_p)] == "/":
                return True
    return False


# 5 个角色的边界规则
ROLE_BOUNDARIES: Dict[AgentRole, BoundaryRule] = {
    # 主管：可写任意位置（仲裁权）
    AgentRole.COORDINATOR: BoundaryRule(
        allowed_prefixes=None,  # None = 不限制
        denied_prefixes=frozenset(),
        read_only=False,
    ),
    # 架构师：只读（不能写）
    AgentRole.ARCHITECT: BoundaryRule(
        allowed_prefixes=None,
        denied_prefixes=frozenset(),  # 通过 read_only 限制
        read_only=True,
    ),
    # 开发工程师：可写 app/ 和 src/ 下的生产代码，禁止 tests/ 和 docs/
    AgentRole.DEVELOPER: BoundaryRule(
        allowed_prefixes=frozenset([
            "app/",
            "src/",
            "platformkit/",
            "services/",
            "routes/",
        ]),
        denied_prefixes=frozenset([
            "tests/",
            "test/",
            "__tests__/",
            "docs/",
            ".yunji/knowledge/",  # 知识由 KnowledgeEngine 负责
        ]),
        read_only=False,
    ),
    # 测试工程师：可写 tests/，禁止 app/ 下的生产代码
    AgentRole.TESTER: BoundaryRule(
        allowed_prefixes=frozenset([
            "tests/",
            "test/",
            "__tests__/",
            "e2e/",
            "fixtures/",
        ]),
        denied_prefixes=frozenset([
            "app/services/",  # 不能改生产服务
            "app/routes/",    # 不能改生产路由
            "docs/",          # 文档归 Documenter
        ]),
        read_only=False,
    ),
    # 文档工程师：可写 docs/，禁止代码
    AgentRole.DOCUMENTER: BoundaryRule(
        allowed_prefixes=frozenset([
            "docs/",
            "doc/",
            "README.md",
            "CHANGELOG.md",
        ]),
        denied_prefixes=frozenset([
            "app/",
            "src/",
            "tests/",
            "test/",
        ]),
        read_only=False,
    ),
}


# ──────────── 异常 ────────────


class BoundaryViolationError(Exception):
    """角色边界违规异常"""

    def __init__(
        self,
        role: AgentRole,
        action: str,
        path: str,
        reason: str,
    ) -> None:
        self.role = role
        self.action = action
        self.path = path
        self.reason = reason
        super().__init__(
            f"[{role.label_full}] 边界违规: {action} '{path}' — {reason}"
        )


# ──────────── 边界检查 ────────────


# 写操作 action 列表
WRITE_ACTIONS = frozenset({"write", "create", "edit", "delete", "remove", "rename"})
READ_ACTIONS = frozenset({"read", "view", "list", "search", "grep"})


def check_boundary(role: AgentRole, path: str, action: str = "write") -> bool:
    """检查角色 + 路径 + 操作的边界

    Args:
        role: 操作者角色
        path: 文件路径
        action: 操作类型（write / read / delete / ...）

    Returns:
        True 表示通过

    Raises:
        BoundaryViolationError: 边界违规
    """
    rule = ROLE_BOUNDARIES.get(role)
    if rule is None:
        raise BoundaryViolationError(
            role, action, path, f"未知角色: {role.value}"
        )

    action = action.lower().strip()

    # 1. 主管不受限制
    if role == AgentRole.COORDINATOR:
        return True

    # 2. 只读角色
    if rule.read_only and action in WRITE_ACTIONS:
        raise BoundaryViolationError(
            role, action, path, f"{role.label}是只读角色，不能{action}"
        )

    # 3. 黑名单前缀（最高优先级）
    if rule.denied_prefixes and _match_prefix(path, rule.denied_prefixes):
        raise BoundaryViolationError(
            role, action, path,
            f"路径 '{path}' 在 {role.label} 的禁止列表中（{sorted(rule.denied_prefixes)}）"
        )

    # 4. 白名单前缀（如果有，强制要求匹配）
    if rule.allowed_prefixes and action in WRITE_ACTIONS:
        if not _match_prefix(path, rule.allowed_prefixes):
            raise BoundaryViolationError(
                role, action, path,
                f"路径 '{path}' 不在 {role.label} 的允许列表中（{sorted(rule.allowed_prefixes)}）"
            )

    return True


# ──────────── 角色 prompt 模板 ────────────


ROLE_PROMPTS: Dict[AgentRole, str] = {
    AgentRole.COORDINATOR: (
        "你是团队主管（Coordinator）。你的职责：\n"
        "1. 拆解用户需求为可执行子任务\n"
        "2. 按角色派发任务（架构师/开发/测试/文档）\n"
        "3. 处理角色间冲突与分歧（仲裁权）\n"
        "4. 监控任务进度，识别阻塞\n"
        "5. 在所有任务完成后做最终集成验证\n"
        "原则：一次只做一件事、范围严格、阻塞立即升级。"
    ),
    AgentRole.ARCHITECT: (
        "你是架构师（Architect）。你的职责：\n"
        "1. 评估需求的技术风险与架构影响\n"
        "2. 输出设计简报（关键决策 + 文件影响图）\n"
        "3. 识别热点文件 / 改动半径\n"
        "4. 只读不改（你不能修改任何代码或文档）\n"
        "5. 给开发工程师提供最小安全实现建议\n"
        "原则：少即是多、最小变更、可回滚。"
    ),
    AgentRole.DEVELOPER: (
        "你是开发工程师（Developer）。你的职责：\n"
        "1. 按架构师简报实现生产代码（app/、src/、platformkit/、services/、routes/）\n"
        "2. 严格遵守角色边界：禁止写 tests/、docs/\n"
        "3. 最小安全实现 + 充分错误处理\n"
        "4. 每完成一个子任务立即报告主管\n"
        "5. 接受测试工程师的交叉审查\n"
        "原则：KISS、最小惊讶、向后兼容、留有测试钩子。"
    ),
    AgentRole.TESTER: (
        "你是测试工程师（Tester）。你的职责：\n"
        "1. 为开发工程师的实现写测试（tests/、test/、__tests__/、e2e/、fixtures/）\n"
        "2. 严格遵守角色边界：禁止改 app/ 下的生产代码\n"
        "3. 测试覆盖率 ≥ 80%，回归保护\n"
        "4. 开发完成后自动触发交叉审查\n"
        "5. 发现 bug 立即报告主管（不擅自修）\n"
        "原则：黑盒优先、边界条件、确定性、自动化。"
    ),
    AgentRole.DOCUMENTER: (
        "你是文档工程师（Documenter）。你的职责：\n"
        "1. 维护 docs/、doc/、README.md、CHANGELOG.md\n"
        "2. API 文档、部署文档、用户指南\n"
        "3. 严格遵守角色边界：禁止改 app/、src/、tests/\n"
        "4. 与开发工程师同步：API 变更要同步更新文档\n"
        "5. 文档质量优先：示例完整、术语统一\n"
        "原则：清晰、准确、示例驱动、与代码同步。"
    ),
}


# ──────────── 协作关系图 ────────────


# 谁可以审查谁（交叉审查）
REVIEW_PAIRS: Dict[AgentRole, List[AgentRole]] = {
    AgentRole.DEVELOPER: [AgentRole.TESTER, AgentRole.ARCHITECT],
    AgentRole.TESTER: [AgentRole.DEVELOPER, AgentRole.COORDINATOR],
    AgentRole.DOCUMENTER: [AgentRole.ARCHITECT, AgentRole.DEVELOPER],
    AgentRole.ARCHITECT: [AgentRole.COORDINATOR],
    AgentRole.COORDINATOR: [],  # 主管不需要被审查
}

# 谁可以仲裁谁
ARBITRATION_PAIRS: Dict[AgentRole, List[AgentRole]] = {
    AgentRole.ARCHITECT: [AgentRole.COORDINATOR],
    AgentRole.DEVELOPER: [AgentRole.COORDINATOR],
    AgentRole.TESTER: [AgentRole.COORDINATOR],
    AgentRole.DOCUMENTER: [AgentRole.COORDINATOR],
    AgentRole.COORDINATOR: [],
}


# ──────────── 工具定义 ────────────


# 工具对角色的可见性（哪些角色可以用哪些工具）
TOOL_VISIBILITY: Dict[str, Set[AgentRole]] = {
    # 代码类工具
    "write_file": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "edit_file": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "read_file": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "grep": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "search_files": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "list_files": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    # 终端类
    "run_command": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER},
    # 测试类
    "run_test": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER},
    # Git 类
    "git_diff": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "git_commit": {AgentRole.COORDINATOR, AgentRole.DEVELOPER},
    "git_push": {AgentRole.COORDINATOR},
    # 知识类（共享给所有角色）
    "knowledge_read": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "knowledge_write": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    # 团队协作类
    "send_message": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "request_approval": {AgentRole.COORDINATOR, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
    "update_shared_context": {AgentRole.COORDINATOR, AgentRole.ARCHITECT, AgentRole.DEVELOPER, AgentRole.TESTER, AgentRole.DOCUMENTER},
}


def can_use_tool(role: AgentRole, tool_name: str) -> bool:
    """检查角色是否可以使用某工具"""
    allowed = TOOL_VISIBILITY.get(tool_name)
    if allowed is None:
        # 未知工具默认允许（保守原则：未注册的视为通用）
        return True
    return role in allowed


# ──────────── 工具函数 ────────────


def parse_role(value: str) -> AgentRole:
    """解析角色字符串（支持多种格式）"""
    value = (value or "").strip().lower()
    # 直接匹配
    for role in AgentRole:
        if role.value == value:
            return role
    # 中文 / emoji
    aliases = {
        "主管": AgentRole.COORDINATOR,
        "协调": AgentRole.COORDINATOR,
        "架构": AgentRole.ARCHITECT,
        "架构师": AgentRole.ARCHITECT,
        "开发": AgentRole.DEVELOPER,
        "工程师": AgentRole.DEVELOPER,
        "测试": AgentRole.TESTER,
        "qa": AgentRole.TESTER,
        "文档": AgentRole.DOCUMENTER,
        "写文档": AgentRole.DOCUMENTER,
    }
    if value in aliases:
        return aliases[value]
    # 模糊匹配
    for keyword, role in aliases.items():
        if keyword in value:
            return role
    raise ValueError(f"unknown role: {value}")


def list_roles() -> List[AgentRole]:
    """列出所有角色（按主管→架构→开发→测试→文档）"""
    return [
        AgentRole.COORDINATOR,
        AgentRole.ARCHITECT,
        AgentRole.DEVELOPER,
        AgentRole.TESTER,
        AgentRole.DOCUMENTER,
    ]


# ──────────── 自检 ────────────


def team_status() -> Dict[str, any]:
    """团队模式状态（用于 /api/team/status 端点）"""
    return {
        "roles": [
            {
                "id": r.value,
                "label": r.label_full,
                "read_only": ROLE_BOUNDARIES[r].read_only,
                "allowed": sorted(ROLE_BOUNDARIES[r].allowed_prefixes) if ROLE_BOUNDARIES[r].allowed_prefixes else None,
                "denied": sorted(ROLE_BOUNDARIES[r].denied_prefixes),
            }
            for r in AgentRole
        ],
        "review_pairs": {
            r.value: [x.value for x in targets] for r, targets in REVIEW_PAIRS.items()
        },
        "arbitration_pairs": {
            r.value: [x.value for x in targets] for r, targets in ARBITRATION_PAIRS.items()
        },
        "tools": {
            tool: sorted(roles) for tool, roles in TOOL_VISIBILITY.items()
        },
    }
