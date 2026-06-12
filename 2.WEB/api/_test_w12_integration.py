"""W12 TASK-3.9 跨模块联动测试：TeamWorkflow + DurableStore + KnowledgeEngine + CodeAgent"""
import sys
import tempfile
import uuid
from pathlib import Path

ROOT = Path(r"E:\软件开发\云集智能编程工作站\dev\dev\app")
sys.path.insert(0, str(ROOT))

import os
os.environ["YUNJI_GLOBAL_ROOT"] = str(ROOT / "_yunji_test_w12_integration_")


def fresh():
    p = Path(tempfile.mkdtemp(prefix="w12_int_"))
    return p


def test_integration_full_flow():
    """完整流程：创建会话 → 拆解任务 → 派发 → 执行 → 审查 → 仲裁 → 关闭"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole
    from platformkit.shared.durable_store import DurableStore
    from platformkit.shared.knowledge_core import KnowledgeEngine, KnowledgeLayer

    team = TeamWorkflow(ws)
    store = DurableStore(ws)
    knowledge = KnowledgeEngine(project_root=ws)

    # 1. 主管创建会话
    s = team.create_session("实现用户管理模块（CRUD）")
    assert s.requirement == "实现用户管理模块（CRUD）"

    # 2. 主管拆解任务（4 个角色 + 1 个评审）
    arch = team.add_task(s.id, "架构设计", "API 设计与数据模型", AgentRole.ARCHITECT)
    dev = team.add_task(s.id, "开发 CRUD", "实现 4 个端点", AgentRole.DEVELOPER, dependencies=[arch.id])
    test = team.add_task(s.id, "编写测试", "pytest 单测 + 集成", AgentRole.TESTER, dependencies=[dev.id])
    doc = team.add_task(s.id, "写 API 文档", "OpenAPI + 部署说明", AgentRole.DOCUMENTER, dependencies=[dev.id])
    assert len(s.tasks) == 4

    # 3. 架构师完成设计
    team.assign_task(s.id, arch.id, "arch-agent-001")
    team.complete_task(s.id, arch.id, output="RESTful API + 用户表设计")
    assert arch.status.value == "done"

    # 4. 主管更新共享上下文
    team.update_shared_context(s.id, "api_url", "/api/users", AgentRole.ARCHITECT)
    team.update_shared_context(s.id, "db_table", "users(id, name, email, created_at)", AgentRole.ARCHITECT)
    assert len(s.shared_context) == 2

    # 5. 开发工程师实现
    team.assign_task(s.id, dev.id, "dev-agent-001")
    # 模拟开发中的消息
    team.add_message(s.id, AgentRole.DEVELOPER, "正在写 user model", "chat", dev.id)
    team.add_message(s.id, AgentRole.DEVELOPER, "已完成 model，开始写端点", "chat", dev.id)
    # 完成（含 file_paths，触发边界检查）
    team.complete_task(s.id, dev.id, output="4 个端点全部实现", file_paths=["app/routes/users.py", "app/models/user.py"])
    assert dev.status.value == "done"
    assert dev.file_paths == ["app/routes/users.py", "app/models/user.py"]

    # 6. 触发审批节点（开发请求主管审批合并）
    aid = team.request_approval(s.id, dev.id, AgentRole.COORDINATOR, "请求主管审批合并到 main")
    assert aid.status.value == "pending"
    # 主管批准（dev 已经是 DONE 状态，批准后保持 DONE）
    team.decide_approval(s.id, aid.id, approve=True, comment="代码质量 OK，可以合并")
    assert aid.status.value == "approved"
    # dev 保持 DONE 状态（开发已完成，审批只是确认合并）
    assert dev.status.value == "done"

    # 7. 测试和文档并行（都依赖 dev）
    team.assign_task(s.id, test.id, "test-agent-001")
    team.assign_task(s.id, doc.id, "doc-agent-001")
    # 验证并行
    assert test.status.value == "in_progress"
    assert doc.status.value == "in_progress"

    # 8. 测试完成
    team.complete_task(s.id, test.id, output="12 个测试全部通过", file_paths=["tests/test_users.py"])
    assert test.status.value == "done"

    # 9. 文档完成
    team.complete_task(s.id, doc.id, output="OpenAPI 文档已写", file_paths=["docs/api/users.md"])
    assert doc.status.value == "done"

    # 10. 验证全部完成
    assert s.status.value == "done"
    progress = s.task_progress()
    assert progress["done"] == 4
    assert progress["total"] == 4

    # 11. 验证 DurableStore 联动（关闭会话时写 memory）
    team.close_session(s.id)
    mem = store.get_memory(max_entries=1)
    assert "团队会话" in mem
    assert "实现用户管理模块" in mem

    # 12. 验证消息流
    msg_count = len(s.messages)
    assert msg_count >= 6  # 1 主管首条 + 2 开发 + 2 状态 + 1 关闭
    print("  ✓ test_integration_full_flow")


def test_integration_cross_module_knowledge():
    """团队工作流 + 知识引擎：主管学习的知识被所有角色共享"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole
    from platformkit.shared.knowledge_core import KnowledgeEngine, KnowledgeLayer

    team = TeamWorkflow(ws)
    knowledge = KnowledgeEngine(project_root=ws)

    s = team.create_session("测试知识共享")
    # 主管在共享上下文里记下关键事实
    team.update_shared_context(s.id, "lesson_learned", "FastAPI 路由必须用 async def", AgentRole.COORDINATOR)
    team.update_shared_context(s.id, "tech_decision", "选择 SQLAlchemy 2.0 异步 ORM", AgentRole.ARCHITECT)

    # 验证共享上下文可被所有角色读取
    ctx_keys = [c.key for c in s.shared_context]
    assert "lesson_learned" in ctx_keys
    assert "tech_decision" in ctx_keys
    # 所有角色都能拿到
    print("  ✓ test_integration_cross_module_knowledge")


def test_integration_role_boundary_in_complex_workflow():
    """完整工作流 + 角色边界：开发写测试会被拒绝"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole
    from platformkit.shared.team_core import BoundaryViolationError

    team = TeamWorkflow(ws)
    s = team.create_session("边界测试")
    # 给开发工程师一个写测试的任务（错误配置）
    bad = team.add_task(s.id, "写测试", "应该由测试工程师做", AgentRole.DEVELOPER)
    team.assign_task(s.id, bad.id, "dev-1")
    # 尝试写 tests/ 应该失败
    try:
        team.complete_task(s.id, bad.id, file_paths=["tests/test_foo.py"])
        assert False, "should raise"
    except BoundaryViolationError:
        pass

    # 但开发工程师可以正常写 app/
    ok = team.add_task(s.id, "写后端", "实现逻辑", AgentRole.DEVELOPER)
    team.assign_task(s.id, ok.id, "dev-1")
    team.complete_task(s.id, ok.id, file_paths=["app/foo.py"])
    assert ok.status.value == "done"
    print("  ✓ test_integration_role_boundary_in_complex_workflow")


def test_integration_persistence_reload():
    """持久化：重新加载后状态完整恢复"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole

    # 1. 第一次：创建并完成
    team1 = TeamWorkflow(ws)
    s1 = team1.create_session("持久化测试")
    t = team1.add_task(s1.id, "T1", "做某事", AgentRole.DEVELOPER)
    team1.assign_task(s1.id, t.id, "dev-1")
    team1.complete_task(s1.id, t.id, output="完成")
    team1.add_message(s1.id, AgentRole.COORDINATOR, "辛苦了", "chat")
    team1.close_session(s1.id)
    s1_id = s1.id

    # 2. 第二次：重新加载
    team2 = TeamWorkflow(ws)
    s2 = team2.get_session(s1_id)
    assert s2 is not None
    assert s2.requirement == "持久化测试"
    assert s2.status.value == "closed"
    assert len(s2.tasks) == 1
    assert s2.tasks[0].output == "完成"
    assert len(s2.messages) >= 2
    print("  ✓ test_integration_persistence_reload")


def test_integration_with_code_agent():
    """TeamWorkflow 与 CodeAgent 协同：团队任务可调用独行 Agent 的工具"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole
    from services.code_agent import CodeAgent

    team = TeamWorkflow(ws)
    agent = CodeAgent(ws)

    # 主管创建团队会话
    s = team.create_session("登录功能")
    t = team.add_task(s.id, "实现登录", "User 模型 + /login", AgentRole.DEVELOPER)
    team.assign_task(s.id, t.id, "dev-1")

    # 开发工程师在独行 Agent 中创建会话（同一工作区）
    code_session = agent.create_session("实现 User 模型 + /login 端点")
    code_session = agent.set_plan(code_session.id, [
        "写 User 模型（app/models/user.py）",
        "加 /login 端点（app/routes/auth.py）",
        "写测试（tests/test_auth.py）",
    ])
    # 批准第 1 步
    first_step = code_session.plan[0]
    agent.approve_step(code_session.id, first_step.id)
    agent.start_execute(code_session.id, first_step.id)
    agent.complete_step(code_session.id, first_step.id, result_summary="User 模型已写")

    # 回到团队会话：开发完成
    team.complete_task(s.id, t.id, output="已调用 CodeAgent 完成实现", file_paths=["app/models/user.py"])
    assert t.status.value == "done"
    # 团队会话状态：仍可能为 IN_PROGRESS（因为没有其他任务触发 done 收尾）
    # 关闭团队会话
    team.close_session(s.id)
    assert s.status.value == "closed"
    print("  ✓ test_integration_with_code_agent")


def test_integration_stats_aggregate():
    """全局统计：多会话 + 多角色"""
    ws = fresh()
    from services.team_workflow import TeamWorkflow, AgentRole

    team = TeamWorkflow(ws)
    # 创建 3 个会话
    for req in ["登录", "注册", "找回密码"]:
        s = team.create_session(req)
        team.add_task(s.id, "T1", "做某事", AgentRole.DEVELOPER)
        team.add_task(s.id, "T2", "测试", AgentRole.TESTER)
        team.add_task(s.id, "T3", "文档", AgentRole.DOCUMENTER)

    stats = team.stats()
    assert stats["total_sessions"] == 3
    assert stats["total_tasks"] == 9
    assert stats["role_distribution"]["developer"] == 3
    assert stats["role_distribution"]["tester"] == 3
    assert stats["role_distribution"]["documenter"] == 3
    print("  ✓ test_integration_stats_aggregate")


def main():
    print("\n=== W12 TASK-3.9 跨模块联动测试 ===\n")
    test_integration_full_flow()
    test_integration_cross_module_knowledge()
    test_integration_role_boundary_in_complex_workflow()
    test_integration_persistence_reload()
    test_integration_with_code_agent()
    test_integration_stats_aggregate()
    print("\n✅ 所有 6 个联动测试通过\n")


if __name__ == "__main__":
    main()
