from fastapi import APIRouter, Query, UploadFile, File, Form
from pydantic import BaseModel, Field
from typing import Optional
from sse_starlette import EventSourceResponse

from services.ai_service import AiService
from platformkit.shared import whisper_core

router = APIRouter(prefix="/api/ai", tags=["AI对话/模型管理"])
ai_service = AiService()


class ChatRequest(BaseModel):
    prompt: str = Field(..., min_length=1, description="用户输入")
    provider: Optional[str] = Field(None, description="模型提供商")
    model: Optional[str] = Field(None, description="模型名称")
    session_id: Optional[str] = Field(None, description="会话ID")
    workspace_path: Optional[str] = Field(None, description="工作目录")
    ai_language: Optional[str] = Field(None, description="AI语言")
    ai_temperature: Optional[float] = Field(None, ge=0, le=2, description="温度")
    ai_max_tokens: Optional[int] = Field(None, ge=1, description="最大token数")
    system_prompt: Optional[str] = Field(None, description="系统提示词")
    auto_approve: Optional[bool] = Field(False, description="自动批准")
    # 2026-06-08 TASK-2.3 引入：图片附件（多模态）
    images: Optional[list[str]] = Field(
        None,
        description="图片附件列表（base64 数据 URL，如 data:image/png;base64,XXXX）。空列表/None = 无图。",
    )


class StopChatRequest(BaseModel):
    session_id: Optional[str] = Field(None, description="会话ID")


class ProviderCheckRequest(BaseModel):
    provider: str = Field(..., description="提供商名称")
    base_url: Optional[str] = Field(None, description="API地址")
    api_key: Optional[str] = Field(None, description="API密钥")


class OllamaHealthRequest(BaseModel):
    base_url: Optional[str] = Field(None, description="Ollama地址")
    model_name: str = Field(..., description="模型名称")
    timeout_ms: Optional[int] = Field(30000, ge=1000, description="超时(ms)")


@router.post("/chat")
async def chat(req: ChatRequest):
    """流式对话(SSE)"""
    async def event_generator():
        async for chunk in ai_service.stream_chat(req.model_dump()):
            yield chunk
    return EventSourceResponse(event_generator())


@router.post("/chat/stop")
async def stop_chat(req: StopChatRequest):
    """停止对话"""
    return await ai_service.stop_chat(req.session_id)


@router.get("/models/{provider}")
async def list_models(
    provider: str,
    base_url: Optional[str] = Query(None, description="API地址"),
    api_key: Optional[str] = Query(None, description="API密钥"),
    check_health: Optional[bool] = Query(False, description="检测模型健康"),
):
    """获取模型列表"""
    return await ai_service.list_models(provider, base_url, api_key, check_health)


@router.get("/providers")
async def list_providers():
    """获取提供商列表"""
    return await ai_service.list_providers()


@router.post("/providers/check")
async def check_provider(req: ProviderCheckRequest):
    """检测API连接"""
    return await ai_service.check_provider(req.provider, req.base_url, req.api_key)


@router.get("/ollama/capabilities")
async def ollama_capabilities(
    model_name: str = Query(..., description="模型名称"),
    base_url: Optional[str] = Query(None, description="Ollama地址"),
):
    """Ollama模型能力"""
    return await ai_service.get_ollama_capabilities(base_url, model_name)


@router.post("/ollama/health")
async def ollama_health(req: OllamaHealthRequest):
    """Ollama模型健康检测"""
    return await ai_service.check_ollama_health(req.base_url, req.model_name, req.timeout_ms)


# 2026-06-08 TASK-2.4 引入：Composer Autocomplete（@ 文件 + / 斜杠命令）
class SearchFilesResponse(BaseModel):
    ok: bool = True
    data: Optional[list[dict]] = None
    error: Optional[str] = None


@router.get("/files/search")
async def search_workspace_files(
    workspace_path: Optional[str] = Query(None, description="工作目录（默认从 active project 推断）"),
    query: str = Query("", description="搜索关键字（空 = 列出前 N 个）"),
    limit: int = Query(20, ge=1, le=100, description="数量上限"),
):
    """搜索当前工作区中的文件（@ 触发文件补全用）。

    Returns:
        {"ok", "data": [{"path", "name", "type"}], "error"}
    """
    import os
    from pathlib import Path

    if not workspace_path or not os.path.isdir(workspace_path):
        return SearchFilesResponse(ok=False, error="工作目录无效或未设置（请先在项目管理中选择一个项目）")

    EXCLUDE_DIRS = {
        "node_modules", ".git", "__pycache__", "dist", "build", ".venv", "venv",
        "env", ".next", ".nuxt", "target", ".idea", ".vscode", "BAK",
    }
    EXCLUDE_EXTS = {".exe", ".dll", ".so", ".dylib", ".pyc", ".class", ".o", ".obj"}

    results: list[dict] = []
    q_lower = query.lower().strip()
    try:
        # 限制遍历深度（避免极大仓库卡死）
        max_depth = 6
        base = Path(workspace_path)
        base_depth = len(base.parts)
        for root, dirs, files in os.walk(workspace_path, followlinks=False):
            cur_depth = len(Path(root).parts) - base_depth
            if cur_depth > max_depth:
                dirs[:] = []
                continue
            # 过滤排除目录
            dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS and not d.startswith(".")]
            # 收集文件
            for f in files:
                if len(results) >= limit:
                    break
                if f.startswith("."):
                    continue
                ext = os.path.splitext(f)[1].lower()
                if ext in EXCLUDE_EXTS:
                    continue
                rel = os.path.relpath(os.path.join(root, f), workspace_path).replace("\\", "/")
                if q_lower and q_lower not in rel.lower():
                    continue
                results.append({
                    "path": rel,
                    "name": f,
                    "type": "file",
                })
            # 收集目录
            for d in dirs:
                if len(results) >= limit:
                    break
                rel = os.path.relpath(os.path.join(root, d), workspace_path).replace("\\", "/")
                if q_lower and q_lower not in rel.lower():
                    continue
                results.append({
                    "path": rel,
                    "name": d,
                    "type": "dir",
                })
            if len(results) >= limit:
                break
    except Exception as e:
        return SearchFilesResponse(ok=False, error=f"搜索失败: {e}")

    # 排序：query 命中的优先 + 文件优先于目录
    if q_lower:
        def sort_key(item: dict):
            name_lower = item["name"].lower()
            return (
                0 if name_lower.startswith(q_lower) else 1,
                0 if item["type"] == "file" else 1,
                item["path"].lower(),
            )
        results.sort(key=sort_key)
    return SearchFilesResponse(ok=True, data=results[:limit])


@router.get("/slash-commands")
async def list_slash_commands():
    """列出内置的 / 斜杠命令（Composer / 触发命令补全用）。

    Returns:
        {"ok", "data": [{"id", "name", "description", "prompt"}]}
    """
    # 2026-06-08 TASK-2.4 MVP：硬编码 5 个内置命令，prompts 用户自定义留 v1.1
    commands = [
        {
            "id": "init",
            "name": "/init",
            "description": "分析当前项目并生成 CLAUDE.md/AGENTS.md 项目说明",
            "prompt": "请阅读当前工作目录的所有源代码和文档，然后生成一个 AGENTS.md 项目说明文件，包含项目目标、技术栈、目录结构、构建/运行命令、测试约定。",
        },
        {
            "id": "clear",
            "name": "/clear",
            "description": "清空当前对话（不发送到 AI）",
            "prompt": "",  # 客户端处理
        },
        {
            "id": "compact",
            "name": "/compact",
            "description": "压缩当前对话历史（保留核心上下文，节省 token）",
            "prompt": "请把上面的对话历史压缩成一段不超过 500 字的摘要，保留关键决策和当前任务状态。",
        },
        {
            "id": "diff",
            "name": "/diff",
            "description": "查看当前 git 变更摘要",
            "prompt": "请执行 git status 和 git diff --stat，告诉我当前有哪些未提交的变更，并按文件分组简要说明每处变更的目的。",
        },
        {
            "id": "help",
            "name": "/help",
            "description": "显示 AI 助手使用帮助",
            "prompt": "请告诉我你可以做什么，列出你的核心能力（文件读写、命令执行、git 操作、网页搜索、代码搜索等），以及常用的快捷键和命令。",
        },
    ]
    return {"ok": True, "data": commands}
