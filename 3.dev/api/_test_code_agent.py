"""code_agent 集成测试 (TASK-3.8)"""
import sys
import os
import pathlib
import tempfile
import time

# 隔离环境
os.environ["YUNJI_GLOBAL_ROOT"] = str(pathlib.Path(tempfile.gettempdir()) / "yj_agent_test")
sys.path.insert(0, '.')

from platformkit.shared.durable_store import DurableStore
from platformkit.shared.knowledge_core import KnowledgeEngine
from services.code_agent import (
    CodeAgent, CodeSession, PlanStep, DiffEntry, LearningEntry,
    SessionStatus, StepStatus,
)

print('imports OK')

with tempfile.TemporaryDirectory() as tmp:
    workspace = pathlib.Path(tmp) / "demo"
    workspace.mkdir()
    # 创建 knowledge 目录用于 KnowledgeEngine
    (workspace / ".yunji").mkdir()

    # 隔离 KnowledgeEngine 项目目录
    proj_root = pathlib.Path(tmp) / "knowledge_proj"
    proj_root.mkdir()
    knowledge = KnowledgeEngine(project_root=proj_root)
    store = DurableStore(workspace)
    agent = CodeAgent(workspace, store=store, knowledge=knowledge)

    # 1) create_session
    s1 = agent.create_session("实现登录功能")
    print('session id:', s1.id[:8], 'status:', s1.status.value)
    assert s1.status == SessionStatus.DRAFT
    assert s1.requirement == "实现登录功能"

    # 2) set_plan (markdown 格式)
    plan_text = "- 写 User 模型\n- 加 /login 端点\n- 写测试"
    steps = CodeAgent.parse_plan_text(plan_text)
    print('parsed steps:', steps)
    assert len(steps) == 3
    s2 = agent.set_plan(s1.id, steps)
    assert s2.status == SessionStatus.PLANNED
    assert len(s2.plan) == 3
    assert all(st.status == StepStatus.PENDING for st in s2.plan)

    # 3) 解析多种格式
    s_num = CodeAgent.parse_plan_text("1. 步骤1\n2. 步骤2\n3. 步骤3")
    assert len(s_num) == 3
    s_plain = CodeAgent.parse_plan_text("步骤A\n步骤B")
    assert len(s_plain) == 2

    # 4) approve_step
    step1_id = s2.plan[0].id
    s3 = agent.approve_step(s1.id, step1_id)
    approved = [st for st in s3.plan if st.id == step1_id][0]
    assert approved.status == StepStatus.APPROVED
    assert approved.approved_at is not None
    assert s3.status == SessionStatus.APPROVED

    # 5) reject_step
    step2_id = s2.plan[1].id
    s4 = agent.reject_step(s1.id, step2_id, reason="用 OAuth 替代")
    rejected = [st for st in s4.plan if st.id == step2_id][0]
    assert rejected.status == StepStatus.REJECTED
    assert rejected.reject_reason == "用 OAuth 替代"

    # 6) start_execute
    s5 = agent.start_execute(s1.id, step1_id)
    exec_step = [st for st in s5.plan if st.id == step1_id][0]
    assert exec_step.status == StepStatus.EXECUTING
    assert s5.status == SessionStatus.EXECUTING
    assert s5.started_at is not None

    # 7) record_tool_call
    s6 = agent.record_tool_call(
        s1.id, step1_id,
        "write_file",
        {"path": "models/user.py", "content": "class User: pass"},
        "Wrote 10 bytes",
    )
    s7 = agent.record_tool_call(
        s1.id, step1_id,
        "run_command",
        {"cmd": "pytest tests/"},
        "1 passed",
    )
    exec_step2 = [st for st in s7.plan if st.id == step1_id][0]
    assert len(exec_step2.tool_calls) == 2
    assert exec_step2.tool_calls[1]["name"] == "run_command"

    # 8) record_diff
    s8 = agent.record_diff(s1.id, DiffEntry(
        step_id=step1_id,
        file_path="models/user.py",
        added_lines=10,
        removed_lines=0,
        summary="新增 User 模型",
    ))
    assert len(s8.diffs) == 1

    # 9) complete_step
    s9 = agent.complete_step(s1.id, step1_id, "User 模型完成")
    done_step = [st for st in s9.plan if st.id == step1_id][0]
    assert done_step.status == StepStatus.DONE
    assert done_step.result == "User 模型完成"
    # session 还有 2 个步骤未完成，session 状态仍为 EXECUTING
    assert s9.status == SessionStatus.EXECUTING

    # 10) add_learning (Learn 阶段)
    s10 = agent.add_learning(s1.id, "L1", "不要在 User 模型里写密码", step_id=step1_id)
    s11 = agent.add_learning(s1.id, "L3", "项目用 PostgreSQL")
    s12 = agent.add_learning(s1.id, "L4", "我喜欢用 snake_case")
    assert len(s12.learnings) == 3
    # 自动写入 KnowledgeEngine
    kb_items = knowledge.list_all()
    print('learnings in KB:', len(kb_items))
    assert len(kb_items) >= 3

    # 11) 把剩余 step 也完成
    step3_id = s2.plan[2].id
    s13 = agent.approve_step(s1.id, step3_id)
    s14 = agent.start_execute(s1.id, step3_id)
    s15 = agent.complete_step(s1.id, step3_id, "测试通过")
    print('session status after all:', s15.status.value)
    assert s15.status == SessionStatus.DONE
    assert s15.completed_at is not None

    # 12) progress（step1 done + step3 done + step2 rejected）
    progress = s15.progress()
    print('progress:', progress)
    assert progress["total"] == 3
    assert progress["done"] == 2
    assert progress["rejected"] == 1

    # 13) close_session
    s16 = agent.close_session(s1.id)
    assert s16.status == SessionStatus.CLOSED
    # 持久化到 memory.md
    mem = store.get_memory()
    assert "实现登录功能" in mem
    # plan.md 写入
    plan_back = store.get_plan()
    assert "User 模型" in plan_back

    # 14) 持久化一致性 + 列出
    agent2 = CodeAgent(workspace, store=DurableStore(workspace), knowledge=KnowledgeEngine(project_root=proj_root))
    s17 = agent2.get_session(s1.id)
    assert s17 is not None
    assert s17.status == SessionStatus.CLOSED
    assert len(s17.learnings) == 3

    sessions_list = agent2.list_sessions()
    assert len(sessions_list) == 1
    closed_list = agent2.list_sessions(status=SessionStatus.CLOSED)
    assert len(closed_list) == 1

    stats = agent2.stats()
    print('stats:', stats)
    assert stats["total_sessions"] == 1
    assert stats["total_learnings"] == 3
    assert stats["total_diffs"] == 1

    # 15) 错误处理
    try:
        agent.approve_step("not-exist", step1_id)
        assert False, "应该抛 KeyError"
    except KeyError:
        pass
    try:
        s_now = agent.get_session(s1.id)
        agent.start_execute(s1.id, step1_id)  # 已 CLOSED 状态
    except (ValueError, KeyError):
        pass

print('ALL OK')
