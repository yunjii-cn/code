from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional

from services.update_service import UpdateService

router = APIRouter(prefix="/api/version", tags=["版本更新"])
update_service = UpdateService()


class SwitchCommitRequest(BaseModel):
    commit_hash: str = Field(..., min_length=1, description="目标commit哈希")


class SwitchExeRequest(BaseModel):
    exe_path: str = Field(..., min_length=1, description="EXE路径")
    git_commit: Optional[str] = Field(None, description="对应Git commit")


@router.get("/current")
async def get_current_version():
    """获取当前版本信息"""
    return await update_service.get_current_version()


@router.get("/history")
async def get_version_history():
    """获取版本历史"""
    return await update_service.get_version_history()


@router.get("/remote")
async def get_remote_versions():
    """获取远程版本信息"""
    return await update_service.fetch_remote_versions()


@router.get("/check-update")
async def check_update():
    """检查资源包更新"""
    return await update_service.check_update()


@router.post("/pull-update")
async def pull_update():
    """拉取资源包更新"""
    return await update_service.pull_update()


@router.get("/commits")
async def get_remote_commits(limit: int = Query(30, ge=1, le=100, description="数量")):
    """获取远程提交记录"""
    return await update_service.fetch_remote_commits(limit)


@router.get("/git-history")
async def get_git_history(limit: int = Query(20, ge=1, le=100, description="数量")):
    """获取本地Git历史"""
    return await update_service.get_git_history(limit)


@router.post("/switch-commit")
async def switch_commit(req: SwitchCommitRequest):
    """切换到指定commit"""
    return await update_service.switch_commit(req.commit_hash)


@router.post("/switch-exe")
async def switch_exe(req: SwitchExeRequest):
    """切换到指定EXE版本"""
    return await update_service.switch_exe(req.exe_path, req.git_commit)


@router.get("/stable-exes")
async def list_stable_exes():
    """列出稳定版EXE"""
    return await update_service.list_stable_exes()


@router.post("/download-update")
async def download_update(source: str = Query("gitee", description="下载源(gitee/github)")):
    """下载更新包"""
    return await update_service.download_update(source)
