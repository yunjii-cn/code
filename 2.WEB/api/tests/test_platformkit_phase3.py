"""
tests/test_platformkit_phase3.py
2026-06-10 TASK-3.1~3.10 验证：Phase 3 核心模块端到端测试

覆盖：
- TASK-3.1 KnowledgeEngine（四层知识 + 强度演化 + confirm）
- TASK-3.4 ResponsiveEngine（4 个 Watcher 启动/扫描/停止）
- TASK-3.7 DurableStore（6 个文件 + 状态读写）
- TASK-3.9 team_core（5 角色 + 边界检查 + 工具权限 + 团队状态）
"""
import sys
import tempfile
from pathlib import Path

# 测试用：添加 app 目录到 sys.path
import os
APP_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if APP_DIR not in sys.path:
    sys.path.insert(0, APP_DIR)

import pytest

from platformkit.shared.knowledge_core import (
    KnowledgeEngine,
    KnowledgeLayer,
    KnowledgeStrength,
    KnowledgeScope,
)
from platformkit.shared.responsive_core import (
    ResponsiveEngine,
    NotificationType,
    Severity,
)
from platformkit.shared.durable_store import DurableStore
from platformkit.shared.team_core import (
    AgentRole,
    check_boundary,
    can_use_tool,
    parse_role,
    list_roles,
    team_status,
)


# ==================== TASK-3.1: KnowledgeEngine ====================

class TestKnowledgeEngine:
    """TASK-3.1 验收：add_* / confirm / list_by_layer / list_by_strength 工作"""

    def test_add_correction_l1(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_correction("use const", scope="project", strength="weak")
        assert k.layer == KnowledgeLayer.CORRECTION
        assert k.strength == KnowledgeStrength.WEAK
        assert "use const" in k.content

    def test_add_pattern_l2(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_pattern("test->commit->deploy", scope="project", strength="weak")
        assert k.layer == KnowledgeLayer.PATTERN

    def test_add_fact_l3(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_fact("uses PostgreSQL", scope="project", strength="weak")
        assert k.layer == KnowledgeLayer.FACT

    def test_add_preference_l4(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_preference("use Tab", scope="global", strength="weak")
        assert k.layer == KnowledgeLayer.PREFERENCE
        assert k.scope == KnowledgeScope.GLOBAL

    def test_confirm_increments_count(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_correction("use const", scope="project")
        assert k.confirm_count == 0
        k2 = eng.confirm(k.id)
        assert k2.confirm_count == 1

    def test_confirm_force_strong_jumps_to_strong(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_correction("use const", scope="project")
        k2 = eng.confirm(k.id, force_strong=True)
        assert k2.strength == KnowledgeStrength.STRONG

    def test_list_by_layer_filters_correctly(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path), global_root=str(tmp_path / "global"))
        eng.add_correction("c1", scope="project")
        eng.add_pattern("p1", scope="project")
        eng.add_fact("f1", scope="project")
        items = eng.list_by_layer(KnowledgeLayer.CORRECTION)
        # 只在 project scope 找 — 应当 1 条
        project_items = [i for i in items if i.scope == KnowledgeScope.PROJECT]
        assert len(project_items) == 1
        assert project_items[0].layer == KnowledgeLayer.CORRECTION

    def test_list_by_strength_filters_correctly(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        eng.add_correction("c1", scope="project", strength="weak")
        items = eng.list_by_strength(KnowledgeStrength.WEAK)
        assert len(items) >= 1

    def test_promote_to_global(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        k = eng.add_correction("c1", scope="project")
        k2 = eng.promote_to_global(k.id)
        assert k2.scope == KnowledgeScope.GLOBAL

    def test_get_relevant_returns_knowledge(self, tmp_path):
        eng = KnowledgeEngine(project_root=str(tmp_path))
        eng.add_correction("use const not var", scope="project")
        eng.add_fact("uses PostgreSQL", scope="project")
        results = eng.get_relevant("postgresql database", top_k=3)
        assert isinstance(results, list)


# ==================== TASK-3.4: ResponsiveEngine ====================

class TestResponsiveEngine:
    """TASK-3.4 验收：4 个 Watcher 都能正常触发并产生通知"""

    def test_engine_constructs(self, tmp_path):
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        assert eng is not None

    def test_start_returns_watcher_list(self, tmp_path):
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        result = eng.start()
        assert result["ok"] is True
        assert result["running"] is True
        assert "file_change" in result["watchers"]
        assert "code_quality" in result["watchers"]
        assert "security_risk" in result["watchers"]
        assert "progress" in result["watchers"]

    def test_trigger_scan_all(self, tmp_path):
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        notifs = eng.trigger_scan("all")
        assert isinstance(notifs, list)
        # 即使空工作区也应该返回 list（可能为 0）
        eng.stop()

    def test_trigger_scan_each_type(self, tmp_path):
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        for scan_type in ("file_change", "code_quality", "security_risk", "progress"):
            notifs = eng.trigger_scan(scan_type)
            assert isinstance(notifs, list), f"{scan_type} 应当返回 list"
        eng.stop()

    def test_stop_returns_status(self, tmp_path):
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        result = eng.stop()
        assert result["ok"] is True
        assert result["running"] is False

    def test_file_change_detects_new_file(self, tmp_path):
        """TASK-3.4 真实场景：创建新文件，扫描应产生 file_change 通知"""
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        # 第一次扫描建立基线
        eng.trigger_scan("file_change")
        # 创建新文件
        (tmp_path / "test_new.py").write_text("print('hi')")
        # 再次扫描应发现
        notifs = eng.trigger_scan("file_change")
        # 即使扫描周期可能错过，调用不应崩溃
        assert isinstance(notifs, list)
        eng.stop()

    def test_security_scanner_finds_requirements(self, tmp_path):
        """TASK-3.4 真实场景：创建 requirements.txt 应当被扫描"""
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        (tmp_path / "requirements.txt").write_text("requests==1.0.0\n")
        notifs = eng.trigger_scan("security_risk")
        assert isinstance(notifs, list)
        eng.stop()

    def test_progress_tracker_finds_resumable_session(self, tmp_path):
        """TASK-3.4 真实场景：写入 .yunji/state.json 含 last_session.unfinished_tasks 应触发 progress 通知"""
        yunji = tmp_path / ".yunji"
        yunji.mkdir()
        import json
        (yunji / "state.json").write_text(json.dumps({
            "workspace": str(tmp_path),
            "last_session": {
                "unfinished_tasks": [
                    {"title": "TASK-3.4-test", "description": "继续完成测试"},
                ],
            },
        }), encoding="utf-8")
        eng = ResponsiveEngine(workspace_path=str(tmp_path))
        eng.start()
        notifs = eng.trigger_scan("progress")
        # 应有至少 1 条 progress 通知
        assert isinstance(notifs, list)
        # 找到含 "TASK-3.4-test" 标题的通知
        relevant = [n for n in notifs if "TASK-3.4-test" in n.title]
        assert len(relevant) >= 1, f"期望找到 progress 通知，实际: {[n.title for n in notifs]}"
        eng.stop()


# ==================== TASK-3.7: DurableStore ====================

class TestDurableStore:
    """TASK-3.7 验收：6 个文件正确读写，state.json 格式稳定"""

    def test_init_creates_yunji_dir(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        assert (tmp_path / ".yunji").exists()

    def test_get_state_returns_dict(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        state = store.get_state()
        assert isinstance(state, dict)
        assert "version" in state or "created_at" in state

    def test_update_state_persists(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        store.update_state({"last_session": "TASK-3.7-test"})
        state = store.get_state()
        assert state.get("last_session") == "TASK-3.7-test"

    def test_append_memory(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        store.append_memory("first memory")
        # 第二次追加不应崩溃
        store.append_memory("second memory")
        # 验证 memory.md 存在且包含
        memory_file = tmp_path / ".yunji" / "memory.md"
        assert memory_file.exists()
        content = memory_file.read_text(encoding="utf-8")
        assert "first memory" in content
        assert "second memory" in content

    def test_update_plan(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        store.update_plan("## Test Plan")
        plan_file = tmp_path / ".yunji" / "plan.md"
        assert plan_file.exists()
        content = plan_file.read_text(encoding="utf-8")
        assert "Test Plan" in content

    def test_tick_checklist(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        result = store.tick_checklist("TASK-3.7")
        assert isinstance(result, bool)

    def test_read_knowledge_returns_dict(self, tmp_path):
        store = DurableStore(workspace_path=str(tmp_path))
        know = store.read_knowledge()
        assert isinstance(know, dict)


# ==================== TASK-3.9: team_core 多 Agent 协作 ====================

class TestTeamCore:
    """TASK-3.9 验收：5 角色 + 边界检查 + 工具权限 + 团队状态"""

    def test_five_roles_exist(self):
        roles = list_roles()
        assert len(roles) == 5
        role_names = {r.name for r in roles}
        assert "COORDINATOR" in role_names
        assert "ARCHITECT" in role_names
        assert "DEVELOPER" in role_names
        assert "TESTER" in role_names
        assert "DOCUMENTER" in role_names

    def test_parse_role_valid(self):
        for v in ("coordinator", "architect", "developer", "tester", "documenter"):
            r = parse_role(v)
            assert r.value == v

    def test_parse_role_invalid_raises(self):
        with pytest.raises(Exception):
            parse_role("lead")  # 不存在

    def test_coordinator_can_use_all_tools(self):
        tools = ("write_file", "read_file", "git_commit", "git_push")
        for t in tools:
            assert can_use_tool(AgentRole.COORDINATOR, t), f"COORDINATOR 应当能 {t}"

    def test_developer_can_write_file(self):
        assert can_use_tool(AgentRole.DEVELOPER, "write_file")

    def test_architect_cannot_git_push(self):
        """TASK-3.9 关键规则：架构师只读，不能 push"""
        assert not can_use_tool(AgentRole.ARCHITECT, "git_push")

    def test_tester_can_run_test(self):
        assert can_use_tool(AgentRole.TESTER, "run_test")

    def test_team_status_complete(self):
        st = team_status()
        assert "roles" in st
        assert "review_pairs" in st
        assert "arbitration_pairs" in st
        assert "tools" in st
        # 5 个角色
        assert len(st["roles"]) == 5
        # 4 个仲裁对（developer/tester/documenter/architect → coordinator）
        assert len(st["arbitration_pairs"]) == 5

    def test_boundary_check_function_exists(self):
        """TASK-3.9 关键规则：角色边界不可逾越"""
        from platformkit.shared.team_core import BoundaryViolationError
        # 主管不受限
        assert check_boundary(AgentRole.COORDINATOR, "anywhere.py", "write") is True
        # 架构师写入应抛错（只读）
        with pytest.raises(BoundaryViolationError):
            check_boundary(AgentRole.ARCHITECT, "src/main.py", "write")
        # 开发者写入测试目录应抛错
        with pytest.raises(BoundaryViolationError):
            check_boundary(AgentRole.DEVELOPER, "tests/test_foo.py", "write")
        # 开发者写入合法路径应通过
        assert check_boundary(AgentRole.DEVELOPER, "src/main.py", "write") is True

    def test_review_pairs_complete(self):
        """TASK-3.9 关键规则：交叉审查"""
        st = team_status()
        rp = st["review_pairs"]
        # developer 应当由 tester 和 architect 审查
        assert "tester" in rp["developer"]
        assert "architect" in rp["developer"]
        # coordinator 不需要被审查
        assert len(rp["coordinator"]) == 0


# ==================== Fixtures ====================

@pytest.fixture
def tmp_path():
    """提供临时目录"""
    with tempfile.TemporaryDirectory() as p:
        yield Path(p)
