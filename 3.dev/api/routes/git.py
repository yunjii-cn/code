"""Git 路由 - 包装 git_core 暴露给前端

TASK-2.2 (2026-06-10)：Git DiffView 增强后端
    - 暴露 git_core 的 get_file_diff / restore_file
    - 让前端能拿到带行号信息的 hunk 数据用于彩色渲染
    - 让前端能单文件回退

设计原则:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/git_core.py
    - 错误统一以 HTTPException 抛出
    - 所有路径前缀 /api/git/*
"""
from typing import Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from platformkit.shared import git_core

router = APIRouter(prefix="/api/git", tags=["Git 增强"])


def _core_result_to_response(res: dict) -> dict:
    """把 git_core 的统一返回结构转成前端易用的格式。"""
    if res.get("ok"):
        return {"ok": True, "data": res.get("data")}
    return {
        "ok": False,
        "error": res.get("error") or "未知错误",
        "stderr": res.get("stderr", ""),
    }


@router.get("/file/diff")
async def git_get_file_diff(
    project_path: str = Query(..., description="项目根目录（绝对路径）"),
    file_path: str = Query(..., description="相对项目根的文件路径"),
    staged: bool = Query(False, description="True = 已暂存（--cached）"),
    context_lines: int = Query(3, ge=0, le=20),
):
    """获取单个文件的彩色 diff（带行号信息的 hunk 列表）。"""
    if not project_path:
        raise HTTPException(status_code=400, detail="project_path 不能为空")
    res = git_core.get_file_diff(
        cwd=project_path,
        file_path=file_path,
        staged=staged,
        context_lines=context_lines,
    )
    return _core_result_to_response(res)


class RestoreFileRequest(BaseModel):
    project_path: str = Field(..., min_length=1, description="项目根目录（绝对路径）")
    file_path: str = Field(..., min_length=1, description="相对项目根的文件路径")
    staged: bool = Field(False, description="True = 只取消暂存（保留 working tree 修改）")


@router.post("/file/restore")
async def git_restore_file(req: RestoreFileRequest):
    """单文件回退到 HEAD 版本（git restore）。

    - staged=False：完全回退 working tree 修改（丢弃所有未保存改动）
    - staged=True：仅取消暂存（保留 working tree 修改）
    """
    res = git_core.restore_file(
        cwd=req.project_path,
        file_path=req.file_path,
        staged=req.staged,
    )
    return _core_result_to_response(res)
