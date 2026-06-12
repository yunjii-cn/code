from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional, List

from services.project_service import ProjectService

router = APIRouter(prefix="/api/project", tags=["项目管理"])
project_service = ProjectService()


class CreateProjectRequest(BaseModel):
    name: str = Field(..., min_length=1, description="项目名称")
    workspace_path: Optional[str] = Field(None, description="工作目录")


class UpdateProjectRequest(BaseModel):
    name: Optional[str] = Field(None, description="新名称")
    workspace_path: Optional[str] = Field(None, description="新工作目录")


class SaveConversationRequest(BaseModel):
    project_id: str = Field(..., description="项目ID")
    session_id: str = Field(..., description="会话ID")
    messages: List[dict] = Field(default_factory=list, description="消息列表")
    title: Optional[str] = Field(None, description="会话标题")


class CopyConversationRequest(BaseModel):
    source_project_id: str = Field(..., description="源项目ID")
    session_id: str = Field(..., description="会话ID")
    target_project_id: str = Field(..., description="目标项目ID")


class RenameConversationRequest(BaseModel):
    project_id: str = Field(..., description="项目ID")
    session_id: str = Field(..., description="会话ID")
    new_title: str = Field(..., min_length=1, description="新标题")


class SaveClaudeMdRequest(BaseModel):
    project_id: str = Field(..., description="项目ID")
    content: str = Field(..., description="CLAUDE.md内容")


class SaveMemoryRequest(BaseModel):
    project_id: str = Field(..., description="项目ID")
    filename: str = Field(..., description="文件名")
    content: str = Field(..., description="内容")
    mem_type: Optional[str] = Field("project", description="记忆类型")


class CreateFromTemplateRequest(BaseModel):
    project_id: str = Field(..., description="项目ID")
    template_id: str = Field(..., description="模板ID")


class SaveCustomTemplateRequest(BaseModel):
    name: str = Field(..., description="模板名称")
    category: str = Field(..., description="分类")
    desc: Optional[str] = Field("", description="描述")
    prompt: str = Field(..., description="提示词")
    files: Optional[str] = Field("", description="文件内容JSON")


@router.get("/list")
async def list_projects():
    """列出所有项目"""
    return await project_service.list_projects()


@router.get("/active")
async def get_active_project():
    """获取当前活跃项目"""
    return await project_service.get_active_project()


@router.post("/create")
async def create_project(req: CreateProjectRequest):
    """创建项目"""
    return await project_service.create_project(req.name, req.workspace_path)


@router.post("/switch/{project_id}")
async def switch_project(project_id: str):
    """切换项目"""
    return await project_service.switch_project(project_id)


@router.post("/rename/{project_id}")
async def rename_project(project_id: str, new_name: str = Query(..., description="新名称")):
    """重命名项目"""
    return await project_service.rename_project(project_id, new_name)


@router.post("/update/{project_id}")
async def update_project(project_id: str, req: UpdateProjectRequest):
    """更新项目"""
    return await project_service.update_project(project_id, req.name, req.workspace_path)


@router.delete("/{project_id}")
async def delete_project(project_id: str):
    """删除项目"""
    return await project_service.delete_project(project_id)


@router.get("/{project_id}/default-path")
async def get_default_project_path(project_id: str, name: str = Query(..., description="项目名称")):
    """获取项目默认路径"""
    return await project_service.get_default_path(name)


@router.get("/{project_id}/conversations")
async def list_conversations(project_id: str):
    """列出项目会话"""
    return await project_service.list_conversations(project_id)


@router.get("/{project_id}/conversations/{session_id}")
async def load_conversation(project_id: str, session_id: str):
    """加载会话消息"""
    return await project_service.load_conversation(project_id, session_id)


@router.post("/conversations/save")
async def save_conversation(req: SaveConversationRequest):
    """保存会话"""
    return await project_service.save_conversation(
        req.project_id, req.session_id, req.messages, req.title
    )


@router.post("/conversations/copy")
async def copy_conversation(req: CopyConversationRequest):
    """复制会话到另一项目"""
    return await project_service.copy_conversation(
        req.source_project_id, req.session_id, req.target_project_id
    )


@router.post("/conversations/rename")
async def rename_conversation(req: RenameConversationRequest):
    """重命名会话"""
    return await project_service.rename_conversation(
        req.project_id, req.session_id, req.new_title
    )


@router.delete("/{project_id}/conversations/{session_id}")
async def delete_conversation(project_id: str, session_id: str):
    """删除会话"""
    return await project_service.delete_conversation(project_id, session_id)


@router.get("/{project_id}/conversations/search")
async def search_conversations(project_id: str, keyword: str = Query(..., description="搜索关键词")):
    """搜索会话"""
    return await project_service.search_conversations(project_id, keyword)


@router.get("/{project_id}/context")
async def get_project_context(project_id: str):
    """获取项目上下文"""
    return await project_service.get_project_context(project_id)


@router.get("/{project_id}/claude-md")
async def get_claude_md(project_id: str):
    """获取项目CLAUDE.md"""
    return await project_service.get_claude_md(project_id)


@router.post("/claude-md/save")
async def save_claude_md(req: SaveClaudeMdRequest):
    """保存项目CLAUDE.md"""
    return await project_service.save_claude_md(req.project_id, req.content)


@router.get("/claude-md/global")
async def get_global_claude_md():
    """获取全局CLAUDE.md"""
    return await project_service.get_global_claude_md()


@router.post("/claude-md/global")
async def save_global_claude_md(content: str = Query(..., description="内容")):
    """保存全局CLAUDE.md"""
    return await project_service.save_global_claude_md(content)


@router.get("/{project_id}/memories")
async def list_memories(project_id: str):
    """列出项目记忆"""
    return await project_service.list_memories(project_id)


@router.post("/memories/save")
async def save_memory(req: SaveMemoryRequest):
    """保存记忆"""
    return await project_service.save_memory(
        req.project_id, req.filename, req.content, req.mem_type
    )


@router.delete("/{project_id}/memories/{filename}")
async def delete_memory(project_id: str, filename: str):
    """删除记忆"""
    return await project_service.delete_memory(project_id, filename)


@router.get("/{project_id}/memories/search")
async def search_memories(project_id: str, keyword: str = Query(..., description="搜索关键词")):
    """搜索记忆"""
    return await project_service.search_memories(project_id, keyword)


@router.get("/{project_id}/memories/stats")
async def get_memory_stats(project_id: str):
    """获取记忆统计"""
    return await project_service.get_memory_stats(project_id)


@router.get("/{project_id}/memories/relevant")
async def get_relevant_memories(project_id: str, query: str = Query(..., description="查询文本")):
    """获取相关记忆"""
    return await project_service.get_relevant_memories(project_id, query)


@router.get("/templates")
async def list_templates(category: Optional[str] = Query("all", description="分类筛选")):
    """列出项目模板"""
    return await project_service.list_templates(category)


@router.post("/templates/custom")
async def save_custom_template(req: SaveCustomTemplateRequest):
    """保存自定义模板"""
    return await project_service.save_custom_template(
        req.name, req.category, req.desc, req.prompt, req.files
    )


@router.delete("/templates/custom/{template_id}")
async def delete_custom_template(template_id: str):
    """删除自定义模板"""
    return await project_service.delete_custom_template(template_id)


@router.post("/create-from-template")
async def create_from_template(req: CreateFromTemplateRequest):
    """从模板创建项目"""
    return await project_service.create_from_template(req.project_id, req.template_id)
