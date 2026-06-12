"""durable_store 集成测试 (TASK-3.7)"""
import sys
import os
import pathlib
import tempfile
import time

# 隔离环境
os.environ["YUNJI_GLOBAL_ROOT"] = str(pathlib.Path(tempfile.gettempdir()) / "yj_store_test")
sys.path.insert(0, '.')

from platformkit.shared.durable_store import (
    DurableStore, SkillEntry, AgentEntry, CheckItem, StoreStatus,
)

print('imports OK')

with tempfile.TemporaryDirectory() as tmp:
    workspace = pathlib.Path(tmp) / "demo"
    workspace.mkdir()
    # 创建一个 dev/AGENTS.md 模拟 AGENTS 读取
    (workspace / "dev").mkdir()
    (workspace / "dev" / "AGENTS.md").write_text(
        "# AGENTS\n\n这是项目级协作者契约。\n", encoding="utf-8"
    )

    # 1) init
    store = DurableStore(workspace)
    print('yunji_dir:', store.yunji_dir.exists())
    assert store.yunji_dir.exists()
    assert (store.yunji_dir / "state.json").exists()
    assert (store.yunji_dir / "memory.md").exists()
    assert (store.yunji_dir / "plan.md").exists()
    assert (store.yunji_dir / "checklist.md").exists()
    assert (store.yunji_dir / "knowledge").exists()
    assert (store.yunji_dir / "skills").exists()

    # 2) state.json
    state = store.get_state()
    print('state version:', state["version"], "workspace:", state["workspace"])
    assert state["version"] == 1
    assert state["workspace"] == str(workspace)

    # mark session start
    state2 = store.mark_session_start()
    print('total_sessions:', state2["stats"]["total_sessions"])
    assert state2["stats"]["total_sessions"] == 1
    assert state2["last_session"]["started_at"] is not None
    assert state2["last_session"]["ended_at"] is None

    # update_state 浅合并
    state3 = store.update_state({"stats": {"total_messages": 5}})
    assert state3["stats"]["total_messages"] == 5
    assert state3["stats"]["total_sessions"] == 1  # 之前累计保留

    # mark session end with unfinished
    state4 = store.mark_session_end(unfinished_tasks=["完成 W10", "写文档"])
    assert state4["last_session"]["ended_at"] is not None
    assert state4["last_session"]["unfinished_tasks"] == ["完成 W10", "写文档"]

    # 3) memory.md append
    store.append_memory("今天完成了 TASK-3.4-3.6")
    store.append_memory("接下来要做 TASK-3.7")
    mem = store.get_memory()
    assert "TASK-3.4-3.6" in mem
    assert "TASK-3.7" in mem
    mem_recent = store.get_memory(max_entries=1)
    assert "TASK-3.7" in mem_recent
    assert "TASK-3.4-3.6" not in mem_recent  # 只取最近 1 条

    # 4) plan.md 覆盖
    plan = "# 当前计划\n\n- [x] W9\n- [ ] W10\n- [ ] W11\n"
    store.update_plan(plan)
    plan_back = store.get_plan()
    assert plan_back == plan

    # 5) checklist
    store.reset_checklist(["W9", "W10", "W11"])
    items = store.get_checklist()
    print('checklist items:', [i.title for i in items])
    assert len(items) == 3
    assert all(not i.done for i in items)

    # tick W10
    ok = store.tick_checklist("W10")
    assert ok
    items2 = store.get_checklist()
    w10 = [i for i in items2 if i.title == "W10"][0]
    print('W10 done:', w10.done)
    assert w10.done

    # tick 模糊匹配
    ok2 = store.tick_checklist("w11", done=False)  # 已经未完成，无变化但返回 False? -- 会变回未完成
    print('w11 tick(False) result:', ok2)
    # 因 W11 已经是 todo，tick(False) 会重新写 - [ ] W11，返回 True
    assert ok2

    # 6) knowledge 只读
    kb = store.read_knowledge()
    print('knowledge layers:', list(kb.keys()))
    assert set(kb.keys()) == {"L1", "L2", "L3", "L4"}
    # 写入 L1 测试
    (store.yunji_dir / "knowledge" / "l1.json").write_text(
        '[{"id": "k1", "content": "用 const", "layer": "L1", "strength": "weak", "scope": "project", "created_at": 1.0, "updated_at": 1.0, "confirm_count": 0}]',
        encoding="utf-8",
    )
    kb2 = store.read_knowledge()
    assert len(kb2["L1"]) == 1
    assert kb2["L1"][0]["content"] == "用 const"

    # 7) skills
    skill_path = store.write_skill("commit-style", "# Commit Style\n\n- 用 conventional commits\n")
    assert skill_path.exists()
    skills = store.read_skills()
    print('skills:', [s.name for s in skills])
    assert any(s.name == "commit-style.md" for s in skills)
    cs = [s for s in skills if s.name == "commit-style.md"][0]
    assert "conventional commits" in cs.content

    # 8) agents
    agents = store.read_agents()
    print('agents:', [(a.name, a.path) for a in agents])
    assert any(a.name == "AGENTS" for a in agents)
    ag = [a for a in agents if a.name == "AGENTS"][0]
    assert "协作者契约" in ag.content

    # 9) status
    s = store.status()
    print('status:', s)
    assert s.yunji_dir_exists
    assert all(s.files.values())
    assert s.last_updated_at is not None
    assert s.last_updated_at > 0

    # 10) 持久化一致性（重新加载）
    store2 = DurableStore(workspace)
    state5 = store2.get_state()
    assert state5["stats"]["total_sessions"] == 1
    assert state5["stats"]["total_messages"] == 5
    items3 = store2.get_checklist()
    w10_2 = [i for i in items3 if i.title == "W10"][0]
    assert w10_2.done
    kb3 = store2.read_knowledge()
    assert len(kb3["L1"]) == 1

print('ALL OK')
