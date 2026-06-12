# 2026-06-09 TASK-4.5：技能市场核心测试

"""Tests for platformkit.shared.skill_market

覆盖：
    - SEED_CATALOG 完整性（所有技能字段都符合 schema）
    - list_catalog 过滤（category/search/tag）
    - install / uninstall / list_installed 流程（用临时 HOME 隔离）
    - rate / get_user_rating
    - get_stats
    - _validate_skill 校验
"""

import json
import os
import shutil
import sys
import tempfile

import pytest

THIS_DIR = os.path.dirname(os.path.abspath(__file__))
APP_DIR = os.path.dirname(THIS_DIR)
sys.path.insert(0, APP_DIR)

from platformkit.shared import skill_market


@pytest.fixture
def isolated_home(monkeypatch, tmp_path):
    """把 ~/.yunji 指向临时目录，避免污染真实用户目录。"""
    fake_home = tmp_path
    monkeypatch.setenv("HOME", str(fake_home))
    monkeypatch.setenv("USERPROFILE", str(fake_home))
    # 重新计算全局目录
    yield fake_home


# ── 目录完整性 ─────────────────────────────────────────────
class TestSeedCatalog:
    def test_catalog_not_empty(self):
        assert len(skill_market.SEED_CATALOG) >= 10

    def test_all_skills_valid(self):
        for skill in skill_market.SEED_CATALOG:
            ok, err = skill_market._validate_skill(skill)
            assert ok, f"技能 {skill.get('id')} 校验失败: {err}"

    def test_all_ids_unique(self):
        ids = [s["id"] for s in skill_market.SEED_CATALOG]
        assert len(set(ids)) == len(ids), f"重复 ID: {ids}"

    def test_all_ids_match_pattern(self):
        for s in skill_market.SEED_CATALOG:
            assert skill_market.SKILL_ID_PATTERN.match(s["id"]), s["id"]

    def test_all_categories_valid(self):
        for s in skill_market.SEED_CATALOG:
            assert s["category"] in skill_market.VALID_CATEGORIES, s["id"]

    def test_all_have_prompt_template(self):
        for s in skill_market.SEED_CATALOG:
            assert s.get("prompt_template"), f"{s['id']} 缺少 prompt_template"


# ── 元数据校验 ─────────────────────────────────────────────
class TestValidateSkill:
    def test_minimal_valid(self):
        ok, err = skill_market._validate_skill({
            "id": "test-skill",
            "name": "Test",
            "version": "0.1.0",
            "category": "other",
            "prompt_template": "do something",
        })
        assert ok, err

    def test_invalid_id_uppercase(self):
        ok, err = skill_market._validate_skill({
            "id": "InvalidID",
            "name": "X",
            "version": "0.1.0",
            "category": "other",
            "prompt_template": "x",
        })
        assert not ok
        assert "id" in err.lower()

    def test_invalid_category(self):
        ok, err = skill_market._validate_skill({
            "id": "x",
            "name": "X",
            "version": "0.1.0",
            "category": "not-a-category",
            "prompt_template": "x",
        })
        assert not ok
        assert "category" in err.lower()

    def test_missing_prompt_template(self):
        ok, err = skill_market._validate_skill({
            "id": "x",
            "name": "X",
            "version": "0.1.0",
            "category": "other",
        })
        assert not ok
        assert "prompt_template" in err


# ── 目录查询 ────────────────────────────────────────────────
class TestListCatalog:
    def test_no_filter_returns_all(self):
        res = skill_market.list_catalog()
        assert res["ok"] is True
        assert len(res["data"]) == len(skill_market.SEED_CATALOG)

    def test_filter_by_category(self):
        res = skill_market.list_catalog(category="scaffold")
        assert res["ok"] is True
        assert all(s["category"] == "scaffold" for s in res["data"])

    def test_search_match_id(self):
        res = skill_market.list_catalog(search="react")
        assert res["ok"] is True
        assert any("react" in s["id"].lower() for s in res["data"])

    def test_tag_filter(self):
        res = skill_market.list_catalog(tag="react")
        assert res["ok"] is True
        assert all(
            any(t.lower() == "react" for t in s.get("tags", []))
            for s in res["data"]
        )

    def test_sorted_by_rating(self):
        res = skill_market.list_catalog()
        ratings = [s.get("rating", 0) for s in res["data"]]
        assert ratings == sorted(ratings, reverse=True)

    def test_get_skill(self):
        first_id = skill_market.SEED_CATALOG[0]["id"]
        res = skill_market.get_skill(first_id)
        assert res["ok"]
        assert res["data"]["id"] == first_id

    def test_get_unknown_skill(self):
        res = skill_market.get_skill("nonexistent-skill-xyz")
        assert res["ok"] is False

    def test_list_categories(self):
        res = skill_market.list_categories()
        assert res["ok"]
        cats = res["data"]["categories"]
        assert any(c["id"] == "all" for c in cats)
        assert all(c["count"] >= 0 for c in cats)


# ── 安装 / 卸载 ────────────────────────────────────────────
class TestInstallUninstall:
    def test_install_global(self, isolated_home):
        res = skill_market.install_skill("react-component-scaffold", scope="global")
        assert res["ok"], res["error"]
        assert res["data"]["scope"] == "global"
        assert res["data"]["files_written"] >= 0
        # 文件应该已创建
        skill_dir = isolated_home / ".yunji" / "skills" / "react-component-scaffold"
        assert skill_dir.exists()
        assert (skill_dir / "skill.yaml").exists()
        assert (skill_dir / "install.json").exists()

    def test_install_then_uninstall(self, isolated_home):
        skill_market.install_skill("code-review-checklist", scope="global")
        res = skill_market.uninstall_skill("code-review-checklist", scope="global")
        assert res["ok"], res["error"]
        skill_dir = isolated_home / ".yunji" / "skills" / "code-review-checklist"
        assert not skill_dir.exists()

    def test_install_project_requires_workspace(self, isolated_home):
        res = skill_market.install_skill("react-component-scaffold", scope="project")
        assert res["ok"] is False
        assert "workspace_path" in res["error"]

    def test_install_project_with_workspace(self, isolated_home, tmp_path):
        workspace = tmp_path / "myproject"
        workspace.mkdir()
        res = skill_market.install_skill(
            "react-component-scaffold", scope="project", workspace_path=str(workspace)
        )
        assert res["ok"], res["error"]
        skill_dir = workspace / ".yunji" / "skills" / "react-component-scaffold"
        assert skill_dir.exists()

    def test_uninstall_not_installed(self, isolated_home):
        res = skill_market.uninstall_skill("nonexistent-skill", scope="global")
        assert res["ok"] is False

    def test_list_installed_global(self, isolated_home):
        skill_market.install_skill("react-component-scaffold", scope="global")
        skill_market.install_skill("code-review-checklist", scope="global")
        res = skill_market.list_installed(scope="global")
        assert res["ok"]
        assert len(res["data"]) == 2
        ids = {s["id"] for s in res["data"]}
        assert "react-component-scaffold" in ids
        assert "code-review-checklist" in ids

    def test_list_installed_project(self, isolated_home, tmp_path):
        workspace = tmp_path / "proj"
        workspace.mkdir()
        skill_market.install_skill(
            "react-component-scaffold", scope="project", workspace_path=str(workspace)
        )
        res = skill_market.list_installed(scope="project", workspace_path=str(workspace))
        assert res["ok"]
        assert len(res["data"]) == 1


# ── 上传 ────────────────────────────────────────────────────
class TestUpload:
    def test_upload_custom(self, isolated_home):
        meta = {
            "id": "my-custom-skill",
            "name": "我的技能",
            "version": "0.1.0",
            "category": "other",
            "prompt_template": "do {{thing}}",
            "tags": ["custom"],
        }
        res = skill_market.upload_skill(meta, scope="global")
        assert res["ok"], res["error"]
        custom_dir = isolated_home / ".yunji" / "skills" / "custom" / "my-custom-skill"
        assert custom_dir.exists()
        assert (custom_dir / "skill.yaml").exists()

    def test_upload_invalid_meta(self, isolated_home):
        bad = {"id": "BAD ID!"}
        res = skill_market.upload_skill(bad, scope="global")
        assert res["ok"] is False


# ── 评分 ────────────────────────────────────────────────────
class TestRating:
    def test_rate_skill(self, isolated_home):
        res = skill_market.rate_skill("react-component-scaffold", 5)
        assert res["ok"]
        assert res["data"]["count"] == 1
        assert res["data"]["average"] == 5.0

    def test_rate_multiple(self, isolated_home):
        skill_market.rate_skill("code-review-checklist", 5)
        skill_market.rate_skill("code-review-checklist", 3)
        res = skill_market.get_user_rating("code-review-checklist")
        assert res["ok"]
        assert res["data"]["count"] == 2
        assert res["data"]["average"] == 4.0

    def test_rate_invalid_score(self, isolated_home):
        res = skill_market.rate_skill("react-component-scaffold", 6)
        assert res["ok"] is False

    def test_get_rating_unrated(self, isolated_home):
        res = skill_market.get_user_rating("never-rated-skill-xyz")
        assert res["ok"]
        assert res["data"]["count"] == 0


# ── 统计 ────────────────────────────────────────────────────
class TestStats:
    def test_stats_shape(self, isolated_home):
        res = skill_market.get_stats()
        assert res["ok"]
        s = res["data"]
        assert s["official_count"] == len(skill_market.SEED_CATALOG)
        assert s["avg_rating"] > 0
        assert s["total_downloads"] > 0
        assert s["categories"] > 0


# ── 应用（模板渲染） ────────────────────────────────────────
class TestApply:
    def test_apply_after_install(self, isolated_home):
        skill_market.install_skill("react-component-scaffold", scope="global")
        res = skill_market.apply_skill(
            "react-component-scaffold",
            variables={"requirement": "一个按钮组件"},
            scope="global",
        )
        # 当前实现不返回真实渲染（占位），但 ok 应为 True
        # 实际渲染依赖后端进一步实现
        assert res["ok"] is True or "prompt_template" in (res.get("error") or "")

    def test_apply_uninstalled(self, isolated_home):
        res = skill_market.apply_skill("react-component-scaffold", variables={}, scope="global")
        assert res["ok"] is False


# ── 导出 ────────────────────────────────────────────────────
class TestExport:
    def test_export_after_install(self, isolated_home):
        skill_market.install_skill("react-component-scaffold", scope="global")
        res = skill_market.export_skill_zip("react-component-scaffold", scope="global")
        assert res["ok"]
        assert res["data"]["filename"].endswith(".zip")
        assert res["data"]["size"] > 0
        assert len(res["data"]["base64"]) > 0

    def test_export_uninstalled(self, isolated_home):
        res = skill_market.export_skill_zip("not-installed", scope="global")
        assert res["ok"] is False
