from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional
from sse_starlette import EventSourceResponse

from services.ai_service import AiService

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
