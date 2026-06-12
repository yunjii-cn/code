from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional, Dict

from services.config_service import ConfigService
from platformkit.shared import git_core

router = APIRouter(prefix="/api/system", tags=["系统信息/设置"])
config_service = ConfigService()


class SaveSettingsRequest(BaseModel):
    settings: Dict[str, str] = Field(..., description="设置项键值对")


class RunCommandRequest(BaseModel):
    cmd: str = Field(..., min_length=1, description="命令")
    cwd: Optional[str] = Field(None, description="工作目录")


class AppDataRequest(BaseModel):
    data: str = Field(..., description="应用数据JSON")


@router.get("/state")
async def get_state():
    """获取应用状态"""
    return await config_service.get_state()


@router.get("/settings")
async def get_settings():
    """获取全部设置"""
    return await config_service.get_settings()


@router.post("/settings")
async def save_settings(req: SaveSettingsRequest):
    """保存设置"""
    return await config_service.save_settings(req.settings)


@router.post("/settings/clear-model")
async def clear_model_settings():
    """清除模型相关设置"""
    return await config_service.clear_model_settings()


@router.get("/hardware")
async def detect_hardware():
    """检测硬件信息"""
    return await config_service.detect_hardware()


@router.get("/workspace")
async def get_workspace():
    """获取当前工作目录"""
    return await config_service.get_workspace()


@router.post("/workspace/choose")
async def choose_workspace():
    """选择工作目录"""
    return await config_service.choose_workspace()


@router.post("/terminal/run")
async def run_terminal_command(req: RunCommandRequest):
    """执行终端命令"""
    return await config_service.run_command(req.cmd, req.cwd)


@router.get("/app-data")
async def load_app_data():
    """加载应用数据"""
    return await config_service.load_app_data()


@router.post("/app-data")
async def save_app_data(req: AppDataRequest):
    """保存应用数据"""
    return await config_service.save_app_data(req.data)


@router.get("/git/status")
async def get_git_status(project_path: str = Query(..., description="项目路径")):
    """获取Git状态"""
    return await config_service.get_git_status(project_path)


@router.post("/git/commit")
async def git_commit(project_path: str = Query(..., description="项目路径"), message: str = Query(..., description="提交信息")):
    """Git提交"""
    return await config_service.git_commit(project_path, message)


# 2026-06-08 TASK-2.2 引入：DiffView 增强
class RestoreFileRequest(BaseModel):
    project_path: str = Field(..., description="项目根路径")
    file_path: str = Field(..., description="相对文件路径")
    staged: bool = Field(False, description="True=仅取消暂存，False=完全回退到 HEAD")


@router.post("/git/file/restore")
async def git_restore_file(req: RestoreFileRequest):
    """单文件回退到 HEAD（git restore）。

    Args:
        staged: True=仅取消暂存（git restore --staged），False=完全回退（git restore）
    """
    res = git_core.restore_file(req.project_path, req.file_path, staged=req.staged)
    if not res["ok"]:
        return res
    return res


@router.get("/git/file/diff")
async def git_file_diff(
    project_path: str = Query(..., description="项目根路径"),
    file_path: str = Query(..., description="相对文件路径"),
    staged: bool = Query(False, description="True=已暂存，False=未暂存"),
    context_lines: int = Query(3, ge=0, le=50),
):
    """获取单个文件的 diff（带行号信息的增强格式）。

    Returns:
        {
            "ok": True,
            "data": {
                "file": str,
                "raw": str,
                "hunks": [{oldStart, oldLines, newStart, newLines, header, lines: [...]}],
                "stats": {additions, deletions}
            }
        }
    """
    res = git_core.get_file_diff(project_path, file_path, staged=staged, context_lines=context_lines)
    if not res["ok"]:
        return res
    return res


@router.get("/git/log")
async def get_git_log(project_path: str = Query(..., description="项目路径")):
    """获取Git日志"""
    return await config_service.get_git_log(project_path)


@router.get("/file-tree")
async def get_file_tree(project_path: str = Query(..., description="项目路径")):
    """获取文件树"""
    return await config_service.get_file_tree(project_path)


@router.post("/notification")
async def show_notification(title: str = Query(..., description="标题"), body: str = Query(..., description="内容")):
    """显示桌面通知"""
    return await config_service.show_notification(title, body)


@router.post("/open-url")
async def open_external_url(url: str = Query(..., description="URL")):
    """打开外部链接"""
    return await config_service.open_url(url)


@router.post("/open-explorer")
async def open_in_explorer(path: str = Query(..., description="路径")):
    """在资源管理器中打开"""
    return await config_service.open_in_explorer(path)


@router.post("/select-directory")
async def select_directory():
    """选择目录对话框"""
    return await config_service.select_directory()


@router.get("/plugins")
async def list_plugins():
    """列出插件"""
    return await config_service.list_plugins()


@router.post("/plugins/install")
async def install_plugin(plugin_json: str = Query(..., description="插件元数据JSON")):
    """安装插件"""
    return await config_service.install_plugin(plugin_json)


@router.delete("/plugins/{plugin_id}")
async def uninstall_plugin(plugin_id: str):
    """卸载插件"""
    return await config_service.uninstall_plugin(plugin_id)


@router.post("/plugins/{plugin_id}/execute")
async def execute_plugin(plugin_id: str, input_data: str = Query("", description="输入数据")):
    """执行插件"""
    return await config_service.execute_plugin(plugin_id, input_data)


@router.get("/models/offline")
async def get_offline_models():
    """获取离线模型列表"""
    return await config_service.get_offline_models()


@router.post("/models/download")
async def download_model(url: str = Query(..., description="模型下载URL")):
    """下载模型"""
    return await config_service.download_model(url)


@router.post("/models/delete")
async def delete_model(name: str = Query(..., description="模型名称")):
    """删除模型"""
    return await config_service.delete_model(name)


@router.get("/models/ollama-search")
async def search_ollama_library(query: str = Query("", description="搜索关键词")):
    """搜索Ollama模型库"""
    return await config_service.search_ollama_library(query)


@router.post("/models/pull")
async def pull_model(name: str = Query(..., description="模型名称")):
    """拉取Ollama模型"""
    return await config_service.pull_model(name)


@router.get("/models/recommend")
async def recommend_models():
    """推荐模型"""
    return await config_service.recommend_models()
