"""Platform Shared - GitHub CLI 包装器

通过 subprocess 包装 `gh` CLI（https://cli.github.com/），提供：
    - 仓库信息查询
    - Issues 列表与详情
    - PRs 列表、详情、Diff、评论
    - 在浏览器中打开 Issue/PR/Commit

**设计原则**:
    - 不依赖 FastAPI（路由层做参数解析 + 调本模块）
    - 不依赖 requests/aiohttp（仅 stdlib subprocess）
    - 所有 gh 调用有 30 秒默认超时
    - 统一返回 dict: {"ok": bool, "error": str|None, "data": Any, "stderr": str}
    - gh 未安装/未登录时返回 ok=False + 友好错误信息

**使用规则**:
    - ✅ 路由层: `from platformkit.shared.github_core import list_issues`
    - ❌ 禁止: 在 routes/github.py 中直接 subprocess.run(["gh", ...])
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
from typing import Any, Optional


DEFAULT_TIMEOUT_S = 30
GIT_TERMINAL_PROMPT = "0"  # 禁止 gh 弹交互式 prompt


def _gh_path() -> Optional[str]:
    """查找 gh 可执行文件路径，未找到返回 None。"""
    return shutil.which("gh")


def _make_result(
    ok: bool,
    data: Any = None,
    error: Optional[str] = None,
    stderr: str = "",
    returncode: int = 0,
) -> dict:
    """统一返回结构。"""
    return {
        "ok": ok,
        "data": data,
        "error": error,
        "stderr": stderr,
        "returncode": returncode,
    }


def check_gh_available() -> dict:
    """检查 gh CLI 是否已安装且已登录。

    Returns:
        dict: {
            "ok": bool,
            "data": {"installed": bool, "authenticated": bool, "version": str, "user": str},
            "error": str|None,
        }
    """
    path = _gh_path()
    if path is None:
        return _make_result(
            ok=False,
            data={"installed": False, "authenticated": False},
            error="未检测到 gh CLI，请先安装 GitHub CLI（https://cli.github.com/）",
        )
    try:
        env = os.environ.copy()
        env["GH_PROMPT_DISABLED"] = "1"
        env["GIT_TERMINAL_PROMPT"] = GIT_TERMINAL_PROMPT
        version_proc = subprocess.run(
            [path, "--version"],
            capture_output=True,
            text=True,
            timeout=10,
            env=env,
        )
        version = version_proc.stdout.strip() or "unknown"
        status_proc = subprocess.run(
            [path, "auth", "status"],
            capture_output=True,
            text=True,
            timeout=10,
            env=env,
        )
        authenticated = status_proc.returncode == 0
        user = ""
        if authenticated:
            try:
                user_proc = subprocess.run(
                    [path, "api", "user", "--jq", ".login"],
                    capture_output=True,
                    text=True,
                    timeout=10,
                    env=env,
                )
                user = user_proc.stdout.strip()
            except Exception:
                user = ""
        return _make_result(
            ok=True,
            data={
                "installed": True,
                "authenticated": authenticated,
                "version": version,
                "user": user,
            },
        )
    except subprocess.TimeoutExpired:
        return _make_result(
            ok=False,
            data={"installed": True, "authenticated": False},
            error="gh auth status 超时",
        )
    except Exception as e:
        return _make_result(ok=False, data={"installed": True}, error=str(e))


def _run_gh(args: list[str], cwd: Optional[str] = None, timeout_s: int = DEFAULT_TIMEOUT_S) -> dict:
    """底层 gh 命令执行器。

    Args:
        args: gh 子命令及参数（不含 "gh" 本身）
        cwd: 工作目录（用于指定 repo 上下文）
        timeout_s: 超时秒数

    Returns:
        dict: {"ok", "data", "error", "stderr", "returncode"}
    """
    path = _gh_path()
    if path is None:
        return _make_result(
            ok=False,
            error="gh CLI 未安装，无法执行此操作",
        )
    try:
        env = os.environ.copy()
        env["GH_PROMPT_DISABLED"] = "1"
        env["GIT_TERMINAL_PROMPT"] = GIT_TERMINAL_PROMPT
        proc = subprocess.run(
            [path, *args],
            capture_output=True,
            text=True,
            timeout=timeout_s,
            cwd=cwd,
            env=env,
        )
        stderr = (proc.stderr or "").strip()
        if proc.returncode != 0:
            if "not logged in" in stderr.lower() or "auth login" in stderr.lower():
                return _make_result(
                    ok=False,
                    returncode=proc.returncode,
                    stderr=stderr,
                    error="gh 未登录，请先执行 `gh auth login`",
                )
            return _make_result(
                ok=False,
                returncode=proc.returncode,
                stderr=stderr,
                error=stderr or f"gh 命令失败，退出码 {proc.returncode}",
            )
        return _make_result(ok=True, data=proc.stdout, stderr=stderr, returncode=0)
    except subprocess.TimeoutExpired:
        return _make_result(ok=False, error=f"gh 命令超时（>{timeout_s}s）")
    except Exception as e:
        return _make_result(ok=False, error=str(e))


def get_repo_info(cwd: Optional[str] = None) -> dict:
    """获取当前仓库信息。

    Returns:
        {"ok", "data": {"name", "owner", "defaultBranch", "description", "url"} | None, "error"}
    """
    res = _run_gh(["repo", "view", "--json", "nameWithOwner,defaultBranchRef,description,url"], cwd=cwd)
    if not res["ok"]:
        return res
    try:
        obj = json.loads(res["data"])
        return _make_result(
            ok=True,
            data={
                "name": obj.get("nameWithOwner", ""),
                "owner": obj.get("nameWithOwner", "").split("/")[0] if obj.get("nameWithOwner") else "",
                "defaultBranch": (obj.get("defaultBranchRef") or {}).get("name", ""),
                "description": obj.get("description", ""),
                "url": obj.get("url", ""),
            },
        )
    except Exception as e:
        return _make_result(ok=False, error=f"解析 repo info 失败: {e}")


def list_issues(
    cwd: Optional[str] = None,
    state: str = "open",
    assignee: str = "",
    author: str = "",
    limit: int = 30,
) -> dict:
    """列出当前仓库的 Issues。

    Args:
        cwd: 工作目录
        state: open | closed | all
        assignee: 按 assignee 过滤（"" = 不过滤）
        author: 按 author 过滤（"" = 不过滤）
        limit: 数量上限

    Returns:
        {"ok", "data": [issue, ...] | None, "error"}
    """
    fields = "number,title,state,author,assignees,labels,createdAt,updatedAt,url,body"
    args = [
        "issue", "list",
        "--state", state,
        "--limit", str(limit),
        "--json", fields,
    ]
    if assignee:
        args += ["--assignee", assignee]
    if author:
        args += ["--author", author]
    res = _run_gh(args, cwd=cwd)
    if not res["ok"]:
        return res
    try:
        issues = json.loads(res["data"])
        normalized = [
            {
                "number": it.get("number"),
                "title": it.get("title", ""),
                "state": (it.get("state") or "").lower(),
                "author": (it.get("author") or {}).get("login", ""),
                "assignees": [a.get("login", "") for a in (it.get("assignees") or [])],
                "labels": [lbl.get("name", "") for lbl in (it.get("labels") or [])],
                "createdAt": it.get("createdAt", ""),
                "updatedAt": it.get("updatedAt", ""),
                "url": it.get("url", ""),
                "body": it.get("body", ""),
            }
            for it in issues
        ]
        return _make_result(ok=True, data=normalized)
    except Exception as e:
        return _make_result(ok=False, error=f"解析 issues 失败: {e}")


def list_prs(
    cwd: Optional[str] = None,
    state: str = "open",
    search: str = "",
    limit: int = 30,
) -> dict:
    """列出当前仓库的 Pull Requests。

    Args:
        cwd: 工作目录
        state: open | closed | merged | draft | all
        search: 搜索关键字
        limit: 数量上限

    Returns:
        {"ok", "data": [pr, ...] | None, "error"}
    """
    fields = "number,title,state,author,headRefName,baseRefName,createdAt,updatedAt,url,isDraft,additions,deletions,changedFiles"
    args = [
        "pr", "list",
        "--state", state,
        "--limit", str(limit),
        "--json", fields,
    ]
    if search:
        args += ["--search", search]
    res = _run_gh(args, cwd=cwd)
    if not res["ok"]:
        return res
    try:
        prs = json.loads(res["data"])
        normalized = [
            {
                "number": it.get("number"),
                "title": it.get("title", ""),
                "state": (it.get("state") or "").lower(),
                "author": (it.get("author") or {}).get("login", ""),
                "headRef": it.get("headRefName", ""),
                "baseRef": it.get("baseRefName", ""),
                "isDraft": it.get("isDraft", False),
                "createdAt": it.get("createdAt", ""),
                "updatedAt": it.get("updatedAt", ""),
                "url": it.get("url", ""),
                "additions": it.get("additions", 0),
                "deletions": it.get("deletions", 0),
                "changedFiles": it.get("changedFiles", 0),
            }
            for it in prs
        ]
        return _make_result(ok=True, data=normalized)
    except Exception as e:
        return _make_result(ok=False, error=f"解析 PRs 失败: {e}")


def get_pr_detail(pr_number: int, cwd: Optional[str] = None) -> dict:
    """获取单个 PR 的完整详情（标题/描述/作者/分支/统计等）。

    Returns:
        {"ok", "data": {pr 详情} | None, "error"}
    """
    fields = (
        "number,title,state,author,headRefName,baseRefName,body,url,isDraft,"
        "additions,deletions,changedFiles,createdAt,updatedAt,mergeable,labels,milestone"
    )
    res = _run_gh(["pr", "view", str(pr_number), "--json", fields], cwd=cwd)
    if not res["ok"]:
        return res
    try:
        obj = json.loads(res["data"])
        return _make_result(
            ok=True,
            data={
                "number": obj.get("number"),
                "title": obj.get("title", ""),
                "state": (obj.get("state") or "").lower(),
                "author": (obj.get("author") or {}).get("login", ""),
                "headRef": obj.get("headRefName", ""),
                "baseRef": obj.get("baseRefName", ""),
                "body": obj.get("body", ""),
                "url": obj.get("url", ""),
                "isDraft": obj.get("isDraft", False),
                "additions": obj.get("additions", 0),
                "deletions": obj.get("deletions", 0),
                "changedFiles": obj.get("changedFiles", 0),
                "createdAt": obj.get("createdAt", ""),
                "updatedAt": obj.get("updatedAt", ""),
                "mergeable": obj.get("mergeable", ""),
                "labels": [lbl.get("name", "") for lbl in (obj.get("labels") or [])],
                "milestone": (obj.get("milestone") or {}).get("title", "") if obj.get("milestone") else "",
            },
        )
    except Exception as e:
        return _make_result(ok=False, error=f"解析 PR 详情失败: {e}")


def get_pr_diff(pr_number: int, cwd: Optional[str] = None) -> dict:
    """获取 PR 的 diff 文本（原始 unified diff）。

    Returns:
        {"ok", "data": "diff 文本" | None, "error"}
    """
    res = _run_gh(["pr", "diff", str(pr_number)], cwd=cwd)
    if not res["ok"]:
        return res
    return _make_result(ok=True, data=res["data"])


def get_pr_files(pr_number: int, cwd: Optional[str] = None) -> dict:
    """获取 PR 修改的文件清单（path + additions/deletions）。

    Returns:
        {"ok", "data": [file, ...] | None, "error"}
    """
    res = _run_gh(["pr", "view", str(pr_number), "--json", "files"], cwd=cwd)
    if not res["ok"]:
        return res
    try:
        obj = json.loads(res["data"])
        files = obj.get("files") or []
        normalized = [
            {
                "path": f.get("path", ""),
                "additions": f.get("additions", 0),
                "deletions": f.get("deletions", 0),
                "changeType": f.get("changeType", ""),
            }
            for f in files
        ]
        return _make_result(ok=True, data=normalized)
    except Exception as e:
        return _make_result(ok=False, error=f"解析 PR 文件清单失败: {e}")


def list_pr_comments(pr_number: int, cwd: Optional[str] = None) -> dict:
    """获取 PR 的所有评论（issue comments + review comments）。

    Returns:
        {"ok", "data": [comment, ...] | None, "error"}
    """
    res = _run_gh(
        ["pr", "view", str(pr_number), "--json", "comments,reviews"],
        cwd=cwd,
    )
    if not res["ok"]:
        return res
    try:
        obj = json.loads(res["data"])
        comments = []
        for c in obj.get("comments") or []:
            comments.append(
                {
                    "kind": "issue_comment",
                    "id": c.get("id"),
                    "author": (c.get("author") or {}).get("login", ""),
                    "body": c.get("body", ""),
                    "createdAt": c.get("createdAt", ""),
                }
            )
        for r in obj.get("reviews") or []:
            comments.append(
                {
                    "kind": "review",
                    "id": r.get("id"),
                    "author": (r.get("author") or {}).get("login", ""),
                    "body": r.get("body", ""),
                    "state": r.get("state", ""),
                    "createdAt": r.get("createdAt", ""),
                }
            )
        comments.sort(key=lambda x: x.get("createdAt") or "")
        return _make_result(ok=True, data=comments)
    except Exception as e:
        return _make_result(ok=False, error=f"解析 PR 评论失败: {e}")


def create_pr_comment(pr_number: int, body: str, cwd: Optional[str] = None) -> dict:
    """在 PR 上创建一条评论。

    Returns:
        {"ok", "data": None, "error"}
    """
    if not body or not body.strip():
        return _make_result(ok=False, error="评论内容不能为空")
    res = _run_gh(["pr", "comment", str(pr_number), "--body", body.strip()], cwd=cwd)
    if not res["ok"]:
        return res
    return _make_result(ok=True, data={"commented": True, "pr": pr_number})


def get_issue_detail(issue_number: int, cwd: Optional[str] = None) -> dict:
    """获取单个 Issue 的详情 + 评论。

    Returns:
        {"ok", "data": {issue} | None, "error"}
    """
    res = _run_gh(
        [
            "issue", "view", str(issue_number),
            "--json",
            "number,title,state,author,body,labels,assignees,createdAt,updatedAt,url,comments",
        ],
        cwd=cwd,
    )
    if not res["ok"]:
        return res
    try:
        obj = json.loads(res["data"])
        comments = [
            {
                "id": c.get("id"),
                "author": (c.get("author") or {}).get("login", ""),
                "body": c.get("body", ""),
                "createdAt": c.get("createdAt", ""),
            }
            for c in (obj.get("comments") or [])
        ]
        return _make_result(
            ok=True,
            data={
                "number": obj.get("number"),
                "title": obj.get("title", ""),
                "state": (obj.get("state") or "").lower(),
                "author": (obj.get("author") or {}).get("login", ""),
                "body": obj.get("body", ""),
                "labels": [lbl.get("name", "") for lbl in (obj.get("labels") or [])],
                "assignees": [a.get("login", "") for a in (obj.get("assignees") or [])],
                "createdAt": obj.get("createdAt", ""),
                "updatedAt": obj.get("updatedAt", ""),
                "url": obj.get("url", ""),
                "comments": comments,
            },
        )
    except Exception as e:
        return _make_result(ok=False, error=f"解析 Issue 详情失败: {e}")


def open_in_browser(target: str, cwd: Optional[str] = None) -> dict:
    """在浏览器中打开 Issue/PR/Commit/file。

    Args:
        target: issue 编号（如 "123"）、PR 编号（如 "45"）、commit hash、文件路径
        cwd: 工作目录

    Returns:
        {"ok", "data": {"url": "..."} | None, "error"}
    """
    res = _run_gh(["browse", target], cwd=cwd)
    if not res["ok"]:
        return res
    return _make_result(ok=True, data={"opened": True, "target": target})


__all__ = [
    "check_gh_available",
    "get_repo_info",
    "list_issues",
    "list_prs",
    "get_pr_detail",
    "get_pr_diff",
    "get_pr_files",
    "list_pr_comments",
    "create_pr_comment",
    "get_issue_detail",
    "open_in_browser",
]
