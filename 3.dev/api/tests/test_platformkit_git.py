"""git_core 单元测试 - TASK-2.2 (2026-06-10)

覆盖:
    - get_file_diff: 解析 hunks + 行号
    - get_file_diff: add/remove/context 三种 line type
    - get_file_diff: stats 计数
    - get_file_diff: path traversal 防御
    - get_file_diff: 非 git 仓库优雅失败
    - restore_file: path traversal 防御
"""
from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path

import pytest

from platformkit.shared import git_core


@pytest.fixture
def git_repo():
    """创建一个临时 git 仓库用于测试。"""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        env = os.environ.copy()
        env["GIT_AUTHOR_NAME"] = "test"
        env["GIT_AUTHOR_EMAIL"] = "test@example.com"
        env["GIT_COMMITTER_NAME"] = "test"
        env["GIT_COMMITTER_EMAIL"] = "test@example.com"
        env["GIT_AUTHOR_DATE"] = "2026-01-01T00:00:00Z"
        env["GIT_COMMITTER_DATE"] = "2026-01-01T00:00:00Z"
        # Windows 不弹窗
        creationflags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
        def run(*args):
            subprocess.run(
                ["git", *args],
                cwd=str(repo),
                env=env,
                check=True,
                capture_output=True,
                creationflags=creationflags,
            )
        run("init", "-q")
        run("config", "user.email", "test@example.com")
        run("config", "user.name", "test")
        # 初始文件
        (repo / "hello.txt").write_text("line1\nline2\nline3\nline4\n", encoding="utf-8")
        run("add", "hello.txt")
        run("commit", "-q", "-m", "init")
        yield repo


def _run(cmd, cwd, env):
    creationflags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    subprocess.run(
        ["git", *cmd],
        cwd=str(cwd),
        env=env,
        check=True,
        capture_output=True,
        creationflags=creationflags,
    )


# ============ get_file_diff 测试 ============

def test_get_file_diff_returns_ok_with_empty_diff(git_repo):
    """未修改的文件，hunks 应为空数组。"""
    res = git_core.get_file_diff(str(git_repo), "hello.txt")
    assert res["ok"] is True
    assert res["data"]["hunks"] == []
    assert res["data"]["stats"] == {"additions": 0, "deletions": 0}


def test_get_file_diff_parses_add_remove_context(git_repo):
    """修改文件，验证 add/remove/context 行被正确分类。"""
    (git_repo / "hello.txt").write_text("line1\nline2_modified\nline3\nline4_new\nline5\n", encoding="utf-8")
    res = git_core.get_file_diff(str(git_repo), "hello.txt")
    assert res["ok"] is True
    hunks = res["data"]["hunks"]
    assert len(hunks) == 1
    lines = hunks[0]["lines"]
    # 期望 1 个删除（line2）+ 1 个添加（line2_modified）+ 1 个 context（line3）
    # + 1 个删除（line4）+ 1 个添加（line4_new）+ 1 个 context（line5）
    # 因为 git 默认会有上下文行
    types = [ln["type"] for ln in lines]
    assert "remove" in types
    assert "add" in types
    assert "context" in types


def test_get_file_diff_stats(git_repo):
    """验证 stats.additions / deletions 计数正确。"""
    (git_repo / "hello.txt").write_text("line1\nnew_line\nline2\nline3\n", encoding="utf-8")
    res = git_core.get_file_diff(str(git_repo), "hello.txt")
    assert res["ok"] is True
    stats = res["data"]["stats"]
    assert stats["additions"] >= 1
    assert stats["deletions"] >= 0
    # line2 被替换为 new_line：1 个删除 + 1 个添加
    assert stats["additions"] >= 1
    assert stats["deletions"] >= 1


def test_get_file_diff_line_numbers(git_repo):
    """验证 oldLineNo / newLineNo 字段被正确填充。"""
    (git_repo / "hello.txt").write_text("line1\nline2\nline3_modified\nline4\n", encoding="utf-8")
    res = git_core.get_file_diff(str(git_repo), "hello.txt")
    assert res["ok"] is True
    lines = res["data"]["hunks"][0]["lines"]
    # 找到 remove 行（旧 line2）
    remove_lines = [ln for ln in lines if ln["type"] == "remove"]
    assert remove_lines
    assert remove_lines[0]["oldLineNo"] is not None
    assert remove_lines[0]["newLineNo"] is None
    # 找到 add 行
    add_lines = [ln for ln in lines if ln["type"] == "add"]
    assert add_lines
    assert add_lines[0]["oldLineNo"] is None
    assert add_lines[0]["newLineNo"] is not None


def test_get_file_diff_staged_flag(git_repo):
    """staged=True 时应读取已暂存的差异。"""
    (git_repo / "hello.txt").write_text("line1\nline2\nstaged_change\nline4\n", encoding="utf-8")
    _run(["add", "hello.txt"], git_repo, os.environ.copy())
    # working tree 继续修改
    (git_repo / "hello.txt").write_text("line1\nline2\nstaged_change\nline4\nworking_change\n", encoding="utf-8")

    res_unstaged = git_core.get_file_diff(str(git_repo), "hello.txt", staged=False)
    res_staged = git_core.get_file_diff(str(git_repo), "hello.txt", staged=True)

    assert res_unstaged["ok"] is True
    assert res_staged["ok"] is True
    # 未暂存应包含 working_change
    assert any("working_change" in ln["text"] for ln in res_unstaged["data"]["hunks"][0]["lines"])
    # 已暂存应包含 staged_change
    assert any("staged_change" in ln["text"] for ln in res_staged["data"]["hunks"][0]["lines"])


def test_get_file_diff_path_traversal_blocked(git_repo):
    """尝试路径越界应返回错误，不执行 git。"""
    res = git_core.get_file_diff(str(git_repo), "../outside.txt")
    assert res["ok"] is False
    assert "越界" in (res["error"] or "") or "路径" in (res["error"] or "")


def test_get_file_diff_absolute_path_blocked(git_repo):
    """绝对路径应被拒绝。"""
    res = git_core.get_file_diff(str(git_repo), "/etc/passwd")
    assert res["ok"] is False


def test_get_file_diff_empty_path_blocked(git_repo):
    """空路径应被拒绝。"""
    res = git_core.get_file_diff(str(git_repo), "")
    assert res["ok"] is False


def test_get_file_diff_nonexistent_file_returns_empty(git_repo):
    """对未跟踪文件，git diff 返回空（不报错）。"""
    (git_repo / "new_file.txt").write_text("brand new\n", encoding="utf-8")
    res = git_core.get_file_diff(str(git_repo), "new_file.txt")
    assert res["ok"] is True
    assert res["data"]["hunks"] == []


def test_get_file_diff_invalid_cwd():
    """无效的 cwd 应返回错误而不是抛异常。"""
    res = git_core.get_file_diff("C:/this/does/not/exist", "file.txt")
    assert res["ok"] is False


# ============ restore_file 测试 ============

def test_restore_file_restores_working_tree(git_repo):
    """restore_file 应丢弃 working tree 修改。"""
    (git_repo / "hello.txt").write_text("MODIFIED CONTENT\n", encoding="utf-8")
    res = git_core.restore_file(str(git_repo), "hello.txt", staged=False)
    assert res["ok"] is True
    content = (git_repo / "hello.txt").read_text(encoding="utf-8")
    assert content == "line1\nline2\nline3\nline4\n"


def test_restore_file_unstage_only(git_repo):
    """staged=True 时应只取消暂存，保留 working tree 修改。"""
    (git_repo / "hello.txt").write_text("STAGED\n", encoding="utf-8")
    _run(["add", "hello.txt"], git_repo, os.environ.copy())
    (git_repo / "hello.txt").write_text("STAGED + WORKING\n", encoding="utf-8")

    res = git_core.restore_file(str(git_repo), "hello.txt", staged=True)
    assert res["ok"] is True
    # working tree 保留修改
    assert "WORKING" in (git_repo / "hello.txt").read_text(encoding="utf-8")


def test_restore_file_path_traversal_blocked(git_repo):
    res = git_core.restore_file(str(git_repo), "../../../etc/passwd")
    assert res["ok"] is False


def test_restore_file_empty_path_blocked(git_repo):
    res = git_core.restore_file(str(git_repo), "")
    assert res["ok"] is False


def test_restore_file_invalid_cwd():
    res = git_core.restore_file("C:/invalid/path", "file.txt")
    assert res["ok"] is False
