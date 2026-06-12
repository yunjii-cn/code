from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional

from services.env_service import EnvService

router = APIRouter(prefix="/api/env", tags=["环境部署"])
env_service = EnvService()


class MirrorRequest(BaseModel):
    mirror_key: str = Field(..., description="镜像源标识")


class ServiceStartRequest(BaseModel):
    service_name: str = Field(..., description="服务名称")
    project_dir: Optional[str] = Field(None, description="项目目录")
    port: Optional[int] = Field(None, description="端口")
    admin_key: Optional[str] = Field(None, description="管理密钥")


class ServiceStopRequest(BaseModel):
    service_name: str = Field(..., description="服务名称")
    base_url: Optional[str] = Field(None, description="服务地址")


@router.get("/check")
async def check_environment():
    """检测运行环境"""
    return await env_service.check_environment()


@router.get("/status")
async def get_env_status():
    """获取环境安装状态"""
    return await env_service.get_status()


@router.post("/install")
async def install_component(component: str = Query(..., description="组件名称")):
    """安装指定组件(node/bun/deps/uv/qwen2api)"""
    return await env_service.install_component(component)


@router.post("/install-all")
async def install_all():
    """一键安装全部环境"""
    return await env_service.install_all()


@router.get("/mirror")
async def get_mirror():
    """获取当前镜像源"""
    return await env_service.get_mirror()


@router.post("/mirror")
async def set_mirror(req: MirrorRequest):
    """设置镜像源"""
    return await env_service.set_mirror(req.mirror_key)


@router.get("/services")
async def list_services():
    """列出API服务"""
    return await env_service.list_services()


@router.get("/services/{service_name}")
async def get_service_info(service_name: str):
    """获取服务详情"""
    return await env_service.get_service_info(service_name)


@router.post("/services/start")
async def start_service(req: ServiceStartRequest):
    """启动API服务"""
    return await env_service.start_service(req.service_name, req.project_dir, req.port, req.admin_key)


@router.post("/services/stop")
async def stop_service(req: ServiceStopRequest):
    """停止API服务"""
    return await env_service.stop_service(req.service_name, req.base_url)


@router.post("/services/start-all")
async def start_all_services():
    """启动全部API服务"""
    return await env_service.start_all_services()
