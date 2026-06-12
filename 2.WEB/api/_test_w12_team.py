"""W12 TASK-3.9 TeamWorkflow 核心集成测试"""
import sys
import time
from pathlib import Path

# 设置路径
ROOT = Path(r"E:\软件开发\云集智能编程工作站\dev\dev\app")
sys.path.insert(0, str(ROOT))

import os
os.environ["YUNJI_GLOBAL_ROOT"] = str(ROOT / "_yunji_test_w12_")

from platformkit.shared.team_core import (
    AgentRole, BoundaryViolationError, ROLE_BOUNDARIES, ROLE_PROMPTS,
    REVIEW_PAIRS, TOOL_VISIBILITY, check_boundary, can_use_tool,
    parse_role, list_roles, team_status,
)
from services.team_workflow import (
    TeamWorkflow, TeamSession, TeamStatus, TaskItem, TaskStatus,
    ApprovalNode, ApprovalStatus, Message, MessageType, SharedContext,
)


def test_boundary_basic():
    """基础边界检查"""
    # 主管无限制
    assert check_boundary(AgentRole.COORDINATOR, "any/path/file.py", "write")
    # 架构师只读
    try:
        check_boundary(AgentRole.ARCHITECT, "app/main.py", "write")
        assert False, "should raise"
    except BoundaryViolationError:
        pass
    # 架构师可读
    assert check_boundary(AgentRole.ARCHITECT, "app/main.py", "read")
    # 开发工程师可写 app/
    assert check_boundary(AgentRole.DEVELOPER, "app/services/foo.py", "write")
    # 开发工程师不能写 tests/
    try:
        check_boundary(AgentRole.DEVELOPER, "tests/test_foo.py", "write")
        assert False, "should raise"
    except BoundaryViolationError:
        pass
    # 测试工程师可写 tests/
    assert check_boundary(AgentRole.TESTER, "tests/test_foo.py", "write")
    # 测试工程师不能写 app/services/
    try:
        check_boundary(AgentRole.TESTER, "app/services/foo.py", "write")
        assert False, "should raise"
    except BoundaryViolationError:
        pass
    # 文档工程师可写 docs/
    assert check_boundary(AgentRole.DOCUMENTER, "docs/api.md", "write")
    # 文档工程师不能写 app/
    try:
        check_boundary(AgentRole.DOCUMENTER, "app/main.py", "write")
        assert False, "should raise"
    except BoundaryViolationError:
        pass
    print("  ✓ test_boundary_basic")


def test_role_parsing():
    """角色解析"""
    assert parse_role("coordinator") == AgentRole.COORDINATOR
    assert parse_role("developer") == AgentRole.DEVELOPER
    assert parse_role("主管") == AgentRole.COORDINATOR
    assert parse_role("开发") == AgentRole.DEVELOPER
    assert parse_role("测试") == AgentRole.TESTER
    assert parse_role("架构师") == AgentRole.ARCHITECT
    assert parse_role("文档") == AgentRole.DOCUMENTER
    assert parse_role("qa") == AgentRole.TESTER
    try:
        parse_role("unknown_role_xyz")
        assert False
    except ValueError:
        pass
    print("  ✓ test_role_parsing")


def test_tool_visibility():
    """工具可见性"""
    assert can_use_tool(AgentRole.DEVELOPER, "write_file")
    assert can_use_tool(AgentRole.TESTER, "run_test")
    assert not can_use_tool(AgentRole.DOCUMENTER, "run_test")  # 文档不能跑测试
    assert not can_use_tool(AgentRole.ARCHITECT, "write_file")  # 架构只读
    assert can_use_tool(AgentRole.COORDINATOR, "git_push")  # 主管可 push
    assert not can_use_tool(AgentRole.DEVELOPER, "git_push")  # 开发不能 push
    print("  ✓ test_tool_visibility")


def test_team_status():
    """team_status API"""
    status = team_status()
    assert "roles" in status
    assert len(status["roles"]) == 5
    assert "review_pairs" in status
    assert "tools" in status
    print("  ✓ test_team_status")


def test_workflow_create_session(tmp_workspace):
    """创建团队会话"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("实现用户登录功能")
    assert s.status == TeamStatus.PLANNING
    assert s.requirement == "实现用户登录功能"
    assert len(s.messages) >= 1  # 主管首条消息
    print("  ✓ test_workflow_create_session")


def test_workflow_add_task(tmp_workspace):
    """添加任务"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("实现登录")
    t1 = team.add_task(s.id, "设计 API", "RESTful", AgentRole.ARCHITECT)
    t2 = team.add_task(s.id, "写后端", "实现 /login", AgentRole.DEVELOPER, dependencies=[t1.id])
    t3 = team.add_task(s.id, "写测试", "单测 + 集成", AgentRole.TESTER, dependencies=[t2.id])
    t4 = team.add_task(s.id, "写文档", "API 文档", AgentRole.DOCUMENTER, dependencies=[t2.id])
    assert len(s.tasks) == 4
    assert t1.role == AgentRole.ARCHITECT
    assert t2.dependencies == [t1.id]
    # 角色分布
    dist = s.role_distribution()
    assert dist["architect"] == 1
    assert dist["developer"] == 1
    assert dist["tester"] == 1
    assert dist["documenter"] == 1
    print("  ✓ test_workflow_add_task")


def test_workflow_task_lifecycle(tmp_workspace):
    """任务完整生命周期"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("实现登录")
    t = team.add_task(s.id, "写后端", "实现 /login", AgentRole.DEVELOPER)
    # 派发
    team.assign_task(s.id, t.id, "dev-1")
    assert t.status == TaskStatus.IN_PROGRESS
    assert t.assigned_agent == "dev-1"
    assert s.status == TeamStatus.IN_PROGRESS
    # 完成
    team.complete_task(s.id, t.id, output="已实现", file_paths=["app/auth.py"])
    assert t.status == TaskStatus.DONE
    assert t.output == "已实现"
    assert t.file_paths == ["app/auth.py"]
    assert s.status == TeamStatus.DONE
    print("  ✓ test_workflow_task_lifecycle")


def test_workflow_boundary_violation(tmp_workspace):
    """边界违规：开发工程师写测试"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t = team.add_task(s.id, "写测试", "测试代码", AgentRole.DEVELOPER)  # 故意错配
    team.assign_task(s.id, t.id, "dev-1")
    try:
        team.complete_task(s.id, t.id, output="OK", file_paths=["tests/test_foo.py"])
        assert False, "should raise"
    except BoundaryViolationError:
        pass
    print("  ✓ test_workflow_boundary_violation")


def test_workflow_dependency_check(tmp_workspace):
    """依赖检查"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t1 = team.add_task(s.id, "T1", "first", AgentRole.DEVELOPER)
    t2 = team.add_task(s.id, "T2", "second", AgentRole.DEVELOPER, dependencies=[t1.id])
    # t2 不能直接派发（依赖未完成）
    try:
        team.assign_task(s.id, t2.id, "dev-1")
        assert False, "should raise"
    except ValueError:
        pass
    # 完成 t1 后再派发 t2
    team.assign_task(s.id, t1.id, "dev-1")
    team.complete_task(s.id, t1.id)
    team.assign_task(s.id, t2.id, "dev-1")
    assert t2.status == TaskStatus.IN_PROGRESS
    print("  ✓ test_workflow_dependency_check")


def test_workflow_approval(tmp_workspace):
    """审批节点 + 主管仲裁"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t = team.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
    team.assign_task(s.id, t.id, "dev-1")
    # 触发审批
    aid = team.request_approval(s.id, t.id, AgentRole.COORDINATOR, "需要主管确认")
    assert aid.status == ApprovalStatus.PENDING
    assert t.status == TaskStatus.AWAITING_APPROVAL
    # 主管批准
    team.decide_approval(s.id, aid.id, approve=True, comment="OK")
    assert aid.status == ApprovalStatus.APPROVED
    assert t.status == TaskStatus.APPROVED
    print("  ✓ test_workflow_approval")


def test_workflow_approval_reject(tmp_workspace):
    """审批拒绝"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t = team.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
    team.assign_task(s.id, t.id, "dev-1")
    aid = team.request_approval(s.id, t.id, AgentRole.COORDINATOR, "需要确认")
    team.decide_approval(s.id, aid.id, approve=False, comment="代码不通过")
    assert aid.status == ApprovalStatus.REJECTED
    assert t.status == TaskStatus.REJECTED
    print("  ✓ test_workflow_approval_reject")


def test_workflow_fail_task(tmp_workspace):
    """任务失败触发仲裁"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t = team.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
    team.assign_task(s.id, t.id, "dev-1")
    team.fail_task(s.id, t.id, "依赖缺失")
    assert t.status == TaskStatus.FAILED
    assert s.status == TeamStatus.ARBITRATING
    print("  ✓ test_workflow_fail_task")


def test_workflow_messages(tmp_workspace):
    """团队消息流"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    team.add_message(s.id, AgentRole.DEVELOPER, "我开始干活了", MessageType.CHAT)
    team.add_message(s.id, AgentRole.TESTER, "我准备好了", MessageType.CHAT)
    team.add_message(s.id, AgentRole.COORDINATOR, "辛苦了", MessageType.CHAT)
    assert len(s.messages) >= 4  # 3 + 1 主管首条
    roles = [m.role for m in s.messages]
    assert AgentRole.DEVELOPER in roles
    assert AgentRole.TESTER in roles
    assert AgentRole.COORDINATOR in roles
    print("  ✓ test_workflow_messages")


def test_workflow_shared_context(tmp_workspace):
    """共享上下文"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    team.update_shared_context(s.id, "api_url", "https://api.example.com", AgentRole.ARCHITECT)
    team.update_shared_context(s.id, "db_schema", "users(id, name, ...)", AgentRole.DEVELOPER)
    # 更新现有
    team.update_shared_context(s.id, "api_url", "https://api.v2.example.com", AgentRole.COORDINATOR)
    assert len(s.shared_context) == 2
    api_ctx = next(c for c in s.shared_context if c.key == "api_url")
    assert api_ctx.value == "https://api.v2.example.com"
    assert api_ctx.source == AgentRole.COORDINATOR
    print("  ✓ test_workflow_shared_context")


def test_workflow_cross_review(tmp_workspace):
    """交叉审查：完成任务自动触发"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    t = team.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
    team.assign_task(s.id, t.id, "dev-1")
    team.complete_task(s.id, t.id)
    # 状态应变为 REVIEWING（开发完成 → 测试自动审查）
    assert s.status in (TeamStatus.REVIEWING, TeamStatus.DONE)
    # 审查消息应该存在
    review_msgs = [m for m in s.messages if m.type == MessageType.REVIEW]
    assert len(review_msgs) >= 1
    print("  ✓ test_workflow_cross_review")


def test_workflow_persistence(tmp_workspace):
    """持久化"""
    team1 = TeamWorkflow(tmp_workspace)
    s = team1.create_session("持久化测试")
    team1.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
    # 重新加载
    team2 = TeamWorkflow(tmp_workspace)
    s2 = team2.get_session(s.id)
    assert s2 is not None
    assert s2.requirement == "持久化测试"
    assert len(s2.tasks) == 1
    assert s2.tasks[0].title == "T1"
    print("  ✓ test_workflow_persistence")


def test_workflow_stats(tmp_workspace):
    """全局统计"""
    team = TeamWorkflow(tmp_workspace)
    s1 = team.create_session("S1")
    team.add_task(s1.id, "T1", "做某事", AgentRole.DEVELOPER)
    s2 = team.create_session("S2")
    team.add_task(s2.id, "T2", "做某事", AgentRole.TESTER)
    stats = team.stats()
    assert stats["total_sessions"] == 2
    assert stats["total_tasks"] == 2
    assert stats["role_distribution"]["developer"] == 1
    assert stats["role_distribution"]["tester"] == 1
    print("  ✓ test_workflow_stats")


def test_workflow_list_sessions(tmp_workspace):
    """列出会话 + 状态过滤"""
    team = TeamWorkflow(tmp_workspace)
    s1 = team.create_session("S1")
    s2 = team.create_session("S2")
    s3 = team.create_session("S3")
    # 关闭 s3
    team.close_session(s3.id)
    all_sessions = team.list_sessions()
    assert len(all_sessions) == 3
    closed = team.list_sessions(TeamStatus.CLOSED)
    assert len(closed) == 1
    assert closed[0].id == s3.id
    planning = team.list_sessions(TeamStatus.PLANNING)
    assert len(planning) == 2
    print("  ✓ test_workflow_list_sessions")


def test_workflow_close_session(tmp_workspace):
    """关闭会话"""
    team = TeamWorkflow(tmp_workspace)
    s = team.create_session("测试")
    team.close_session(s.id)
    assert s.status == TeamStatus.CLOSED
    # 验证 memory.md 写入了
    mem = team.store.get_memory(max_entries=1)
    assert "团队会话" in mem
    print("  ✓ test_workflow_close_session")


def test_role_workflow_parallel():
    """4-5 个 Agent 并行场景"""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        ws = Path(tmp)
        team = TeamWorkflow(ws)
        s = team.create_session("完整功能实现")
        # 主管拆解为 4 个并行任务
        arch = team.add_task(s.id, "架构设计", "设计简报", AgentRole.ARCHITECT)
        dev = team.add_task(s.id, "开发实现", "写代码", AgentRole.DEVELOPER, dependencies=[arch.id])
        test = team.add_task(s.id, "测试覆盖", "单测+集成", AgentRole.TESTER, dependencies=[dev.id])
        doc = team.add_task(s.id, "文档编写", "API 文档", AgentRole.DOCUMENTER, dependencies=[dev.id])
        # 1. 架构师完成设计
        team.assign_task(s.id, arch.id, "arch-1")
        team.complete_task(s.id, arch.id, output="设计完成")
        # 2. 开发实现（依赖架构）
        team.assign_task(s.id, dev.id, "dev-1")
        team.complete_task(s.id, dev.id, output="代码完成", file_paths=["app/foo.py"])
        # 3. 测试和文档可以并行（都依赖 dev）
        team.assign_task(s.id, test.id, "test-1")
        team.assign_task(s.id, doc.id, "doc-1")
        # 验证状态
        assert test.status == TaskStatus.IN_PROGRESS
        assert doc.status == TaskStatus.IN_PROGRESS
        # 4. 测试完成
        team.complete_task(s.id, test.id, output="测试通过", file_paths=["tests/test_foo.py"])
        # 5. 文档完成
        team.complete_task(s.id, doc.id, output="文档已写", file_paths=["docs/foo.md"])
        # 验证全部完成
        assert s.status == TeamStatus.DONE
        progress = s.task_progress()
        assert progress["done"] == 4
        assert progress["total"] == 4
        print("  ✓ test_role_workflow_parallel")


def test_boundary_denial_paths():
    """边界：黑名单路径"""
    # 开发工程师不能写 .yunji/knowledge/
    try:
        check_boundary(AgentRole.DEVELOPER, ".yunji/knowledge/foo.md", "write")
        assert False
    except BoundaryViolationError:
        pass
    # 测试工程师不能改 app/services/
    try:
        check_boundary(AgentRole.TESTER, "app/services/foo.py", "write")
        assert False
    except BoundaryViolationError:
        pass
    # 文档工程师不能写 app/
    try:
        check_boundary(AgentRole.DOCUMENTER, "app/main.py", "write")
        assert False
    except BoundaryViolationError:
        pass
    print("  ✓ test_boundary_denial_paths")


def test_message_types():
    """消息类型完整性"""
    assert MessageType.CHAT
    assert MessageType.STATUS
    assert MessageType.TASK
    assert MessageType.APPROVAL
    assert MessageType.REVIEW
    assert MessageType.SYSTEM
    print("  ✓ test_message_types")


def main():
    print("\n=== W12 TASK-3.9 TeamWorkflow 核心集成测试 ===\n")
    # team_core 部分
    print("[team_core]")
    test_boundary_basic()
    test_role_parsing()
    test_tool_visibility()
    test_team_status()
    test_boundary_denial_paths()

    # team_workflow 部分（每个测试用独立子目录避免持久化串扰）
    import tempfile
    import uuid
    tmp_root = Path(tempfile.mkdtemp(prefix="w12_team_"))
    counter = [0]
    def fresh():
        counter[0] += 1
        p = tmp_root / f"ws_{counter[0]}_{uuid.uuid4().hex[:6]}"
        p.mkdir(parents=True, exist_ok=True)
        return p

    print("\n[team_workflow]")
    test_workflow_create_session(fresh())
    test_workflow_add_task(fresh())
    test_workflow_task_lifecycle(fresh())
    test_workflow_boundary_violation(fresh())
    test_workflow_dependency_check(fresh())
    test_workflow_approval(fresh())
    test_workflow_approval_reject(fresh())
    test_workflow_fail_task(fresh())
    test_workflow_messages(fresh())
    test_workflow_shared_context(fresh())
    test_workflow_cross_review(fresh())
    test_workflow_persistence(fresh())
    test_workflow_stats(fresh())
    test_workflow_list_sessions(fresh())
    test_workflow_close_session(fresh())

    # 不需要持久化的测试
    print("\n[集成场景]")
    test_role_workflow_parallel()
    test_message_types()

    print("\n✅ 所有 20 个测试通过\n")


if __name__ == "__main__":
    main()
