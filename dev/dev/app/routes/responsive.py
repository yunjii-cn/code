"""感知引擎路由 - 主动感知 + WebSocket 推送

2026-06-09 TASK-3.5 引入：Phase 3 W10 主动感知引擎 API

设计原则（按 AGENTS.md Backend Routing Rules）:
    - 路由层只做参数解析 + 调 core + 返回
    - 业务逻辑全在 platformkit/shared/responsive_core.py
    - 引擎实例缓存：每个 workspace_path 一个 ResponsiveEngine
    - WebSocket 用 fastapi 原生（不依赖 sse_starlette）
    - 推送通过 ConnectionManager 多客户端广播

Endpoints:
    GET   /api/responsive/status         - 引擎状态 + watcher last_scan
    POST  /api/responsive/start          - 启动引擎
    POST  /api/responsive/stop           - 停止引擎
    POST  /api/responsive/scan           - 触发扫描（type=file_change/code_quality/security_risk/progress/all）
    GET   /api/responsive/notifications  - 列出通知（可选类型/严重度过滤）
    POST  /api/responsive/notifications/{id}/dismiss - 标记已读
    WS    /api/responsive/ws             - 实时订阅（每 N 秒扫一次 + 推新通知）
"""
import asyncio
import time
from pathlib import Path
from typing import Dict, List, Optional

from fastapi import APIRouter, HTTPException, Query, WebSocket, WebSocketDisconnect
from pydantic import BaseModel

from platformkit.shared.responsive_core import (
    ResponsiveEngine,
    Notification,
    NotificationType,
    Severity,
)

router = APIRouter(prefix="/api/responsive", tags=["感知引擎（主动）"])


# ──────────── 引擎缓存 ────────────


_engines: Dict[str, ResponsiveEngine] = {}
_dismissed: Dict[str, set] = {}  # workspace -> set of dismissed notification ids


def _engine(workspace_path: Optional[str]) -> ResponsiveEngine:
    key = workspace_path or "_default_"
    if key not in _engines:
        # workspace_path 可空，但 engine 要求一个 path；用 cwd 兜底
        path = Path(workspace_path) if workspace_path else Path.cwd()
        path.mkdir(parents=True, exist_ok=True)
        _engines[key] = ResponsiveEngine(path)
        _dismissed.setdefault(key, set())
    return _engines[key]


def _serial(n: Notification) -> dict:
    return n.to_dict()


# ──────────── ConnectionManager（WebSocket 广播）────────────


class ConnectionManager:
    def __init__(self) -> None:
        self.active: List[WebSocket] = []

    async def connect(self, ws: WebSocket) -> None:
        await ws.accept()
        self.active.append(ws)

    def disconnect(self, ws: WebSocket) -> None:
        if ws in self.active:
            self.active.remove(ws)

    async def broadcast(self, message: dict) -> None:
        dead: List[WebSocket] = []
        for ws in self.active:
            try:
                await ws.send_json(message)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self.disconnect(ws)


ws_manager = ConnectionManager()


# ──────────── RESTful 端点 ────────────


class ScanRequest(BaseModel):
    scan_type: str = "all"


@router.get("/status")
async def status(workspace_path: Optional[str] = Query(None)):
    eng = _engine(workspace_path)
    return {
        "ok": True,
        "data": {
            "running": eng.is_running(),
            "watchers": eng.list_watcher_status(),
        },
    }


@router.post("/start")
async def start(workspace_path: Optional[str] = Query(None)):
    eng = _engine(workspace_path)
    return {"ok": True, "data": eng.start()}


@router.post("/stop")
async def stop(workspace_path: Optional[str] = Query(None)):
    eng = _engine(workspace_path)
    return {"ok": True, "data": eng.stop()}


@router.post("/scan")
async def scan(req: ScanRequest, workspace_path: Optional[str] = Query(None)):
    eng = _engine(workspace_path)
    notifs = eng.trigger_scan(req.scan_type)
    # 自动 dismiss 已读
    key = workspace_path or "_default_"
    dismissed_set = _dismissed.setdefault(key, set())
    payload = [n for n in notifs if n.id not in dismissed_set]
    return {
        "ok": True,
        "data": [_serial(n) for n in payload],
        "total": len(payload),
    }


@router.get("/notifications")
async def list_notifications(
    workspace_path: Optional[str] = Query(None),
    type: Optional[str] = Query(None, description="按类型过滤 file_change/code_quality/security_risk/progress"),
    severity: Optional[str] = Query(None, description="按严重度过滤 info/warning/error"),
    limit: int = Query(50, ge=1, le=200),
):
    eng = _engine(workspace_path)
    notifs = eng.trigger_scan("all")
    if type:
        try:
            notifs = [n for n in notifs if n.type == NotificationType(type)]
        except ValueError:
            raise HTTPException(400, f"invalid type: {type}")
    if severity:
        try:
            notifs = [n for n in notifs if n.severity == Severity(severity)]
        except ValueError:
            raise HTTPException(400, f"invalid severity: {severity}")
    notifs = notifs[:limit]
    return {
        "ok": True,
        "data": [_serial(n) for n in notifs],
        "total": len(notifs),
    }


@router.post("/notifications/{notification_id}/dismiss")
async def dismiss(notification_id: str, workspace_path: Optional[str] = Query(None)):
    key = workspace_path or "_default_"
    dismissed_set = _dismissed.setdefault(key, set())
    dismissed_set.add(notification_id)
    return {"ok": True}


# ──────────── WebSocket（实时订阅）────────────


@router.websocket("/ws")
async def ws_endpoint(ws: WebSocket, workspace_path: Optional[str] = None, interval_s: float = 10.0):
    """WebSocket 实时感知推送

    客户端连接后，服务器每 `interval_s` 秒（默认 10s）触发一次 trigger_scan('all')，
    将新通知广播到所有连接的客户端。

    客户端可发送 {"action": "scan_now"} 立即触发扫描。
    """
    await ws_manager.connect(ws)
    eng = _engine(workspace_path)
    if not eng.is_running():
        eng.start()
    try:
        # 启动时立即推一次
        notifs = eng.trigger_scan("all")
        key = workspace_path or "_default_"
        dismissed_set = _dismissed.setdefault(key, set())
        for n in notifs:
            if n.id in dismissed_set:
                continue
            await ws.send_json({"type": "notification", "data": _serial(n)})

        last_loop = asyncio.create_task(_ws_loop(ws, eng, interval_s, workspace_path))
        # 接收客户端消息
        while True:
            try:
                msg = await ws.receive_json()
                if isinstance(msg, dict) and msg.get("action") == "scan_now":
                    notifs = eng.trigger_scan("all")
                    for n in notifs:
                        if n.id in dismissed_set:
                            continue
                        await ws.send_json({"type": "notification", "data": _serial(n)})
                    await ws.send_json({"type": "ack", "data": {"triggered": True}})
            except WebSocketDisconnect:
                break
            except Exception as e:
                await ws.send_json({"type": "error", "data": {"message": str(e)}})
    except WebSocketDisconnect:
        pass
    finally:
        last_loop.cancel()
        ws_manager.disconnect(ws)


async def _ws_loop(ws: WebSocket, eng: ResponsiveEngine, interval_s: float, workspace_path: Optional[str]):
    """后台循环：每 interval_s 触发扫描并推新通知"""
    key = workspace_path or "_default_"
    dismissed_set = _dismissed.setdefault(key, set())
    while True:
        try:
            await asyncio.sleep(interval_s)
            notifs = eng.trigger_scan("all")
            for n in notifs:
                if n.id in dismissed_set:
                    continue
                try:
                    await ws.send_json({"type": "notification", "data": _serial(n)})
                except Exception:
                    return
        except asyncio.CancelledError:
            return
        except Exception:
            continue
