"""GitHub 集成路由 - 包装 github_core 暴露给前端

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/github_core.py（无 FastAPI 依赖）
    - 错误统一以 HTTPException 抛出
    - 所有路径前缀 /api/github/*

Endpoints:
    GET  /api/github/status              - 检查 gh 安装/登录状态
    GET  /api/github/repo                - 当前仓库信息
    GET  /api/github/issues              - Issues 列表
    GET  /api/github/issues/{n}          - Issue 详情
    GET  /api/github/prs                 - PRs 列表
    GET  /api/github/prs/{n}             - PR 详情
    GET  /api/github/prs/{n}/diff        - PR diff 文本
    GET  /api/github/prs/{n}/files       - PR 修改文件清单
    GET  /api/github/prs/{n}/comments    - PR 评论列表
    POST /api/github/prs/{n}/comments    - 创建 PR 评论
    POST /api/github/open                - 在浏览器中打开
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared import github_core

router = APIRouter(prefix="/api/github", tags=["GitHub 集成"])


def _core_result_to_response(res: dict) -> dict:
    """把 github_core 的统一返回结构转成前端易用的格式。"""
    if res.get("ok"):
        return {"ok": True, "data": res.get("data")}
    return {
        "ok": False,
        "error": res.get("error") or "未知错误",
        "stderr": res.get("stderr", ""),
    }


@router.get("/status")
async def gh_status():
    """检查 gh CLI 是否已安装、是否已登录、当前用户。"""
    return _core_result_to_response(github_core.check_gh_available())


@router.get("/repo")
async def gh_repo_info(project_path: Optional[str] = Query(None, description="项目路径（cwd）")):
    """获取当前 GitHub 仓库信息。"""
    res = github_core.get_repo_info(cwd=project_path)
    if not res["ok"]:
        # 200 状态但 ok=False，让前端能区分"命令失败"和"网络错误"
        return _core_result_to_response(res)
    return _core_result_to_response(res)


@router.get("/issues")
async def gh_list_issues(
    project_path: Optional[str] = Query(None),
    state: str = Query("open", pattern="^(open|closed|all)$"),
    assignee: str = Query(""),
    author: str = Query(""),
    limit: int = Query(30, ge=1, le=100),
):
    """列出仓库 Issues。"""
    res = github_core.list_issues(
        cwd=project_path, state=state, assignee=assignee, author=author, limit=limit
    )
    return _core_result_to_response(res)


@router.get("/issues/{issue_number}")
async def gh_get_issue(issue_number: int, project_path: Optional[str] = Query(None)):
    """获取 Issue 详情 + 评论。"""
    res = github_core.get_issue_detail(issue_number, cwd=project_path)
    return _core_result_to_response(res)


@router.get("/prs")
async def gh_list_prs(
    project_path: Optional[str] = Query(None),
    state: str = Query("open", pattern="^(open|closed|merged|draft|all)$"),
    search: str = Query(""),
    limit: int = Query(30, ge=1, le=100),
):
    """列出仓库 Pull Requests。"""
    res = github_core.list_prs(cwd=project_path, state=state, search=search, limit=limit)
    return _core_result_to_response(res)


@router.get("/prs/{pr_number}")
async def gh_get_pr(pr_number: int, project_path: Optional[str] = Query(None)):
    """获取 PR 完整详情。"""
    res = github_core.get_pr_detail(pr_number, cwd=project_path)
    return _core_result_to_response(res)


@router.get("/prs/{pr_number}/diff")
async def gh_get_pr_diff(pr_number: int, project_path: Optional[str] = Query(None)):
    """获取 PR diff（unified diff 原始文本）。"""
    res = github_core.get_pr_diff(pr_number, cwd=project_path)
    return _core_result_to_response(res)


@router.get("/prs/{pr_number}/files")
async def gh_get_pr_files(pr_number: int, project_path: Optional[str] = Query(None)):
    """获取 PR 修改的文件清单。"""
    res = github_core.get_pr_files(pr_number, cwd=project_path)
    return _core_result_to_response(res)


@router.get("/prs/{pr_number}/comments")
async def gh_list_pr_comments(pr_number: int, project_path: Optional[str] = Query(None)):
    """获取 PR 评论列表（issue + review）。"""
    res = github_core.list_pr_comments(pr_number, cwd=project_path)
    return _core_result_to_response(res)


class CreateCommentRequest(BaseModel):
    body: str = Field(..., min_length=1, description="评论内容")


@router.post("/prs/{pr_number}/comments")
async def gh_create_pr_comment(
    pr_number: int,
    req: CreateCommentRequest,
    project_path: Optional[str] = Query(None),
):
    """在 PR 上创建一条评论。"""
    res = github_core.create_pr_comment(pr_number, req.body, cwd=project_path)
    return _core_result_to_response(res)


class OpenInBrowserRequest(BaseModel):
    target: str = Field(..., min_length=1, description="目标：issue 编号、PR 编号、commit hash、文件路径")


@router.post("/open")
async def gh_open_in_browser(req: OpenInBrowserRequest, project_path: Optional[str] = Query(None)):
    """在浏览器中打开 Issue / PR / Commit / 文件。"""
    res = github_core.open_in_browser(req.target, cwd=project_path)
    return _core_result_to_response(res)
