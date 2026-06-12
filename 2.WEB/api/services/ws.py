import json
import asyncio
from typing import Dict, Set, Optional
from fastapi import WebSocket


class WebSocketManager:
    def __init__(self):
        self._connections: Dict[WebSocket, str] = {}
        self._channels: Dict[str, Set[WebSocket]] = {}
        self._ai_service = None
        self._terminal_sessions: Dict[str, dict] = {}

    def set_ai_service(self, ai_service):
        self._ai_service = ai_service

    async def connect(self, websocket: WebSocket, channel: str = None):
        await websocket.accept()
        ch = channel or "default"
        self._connections[websocket] = ch
        if ch not in self._channels:
            self._channels[ch] = set()
        self._channels[ch].add(websocket)

    def disconnect(self, websocket: WebSocket, channel: str = None):
        ch = self._connections.pop(websocket, None) or channel or "default"
        if ch in self._channels:
            self._channels[ch].discard(websocket)
            if not self._channels[ch]:
                del self._channels[ch]

    def parse_message(self, data: str):
        try:
            return json.loads(data)
        except (json.JSONDecodeError, TypeError):
            return {"type": "text", "content": data}

    async def handle_message(self, websocket: WebSocket, msg: dict):
        msg_type = msg.get("type", "unknown")
        if msg_type == "ping":
            await websocket.send_json({"type": "pong"})
        elif msg_type == "broadcast":
            content = msg.get("content", "")
            channel = self._connections.get(websocket, "default")
            await self._broadcast(channel, {"type": "broadcast", "content": content}, exclude=websocket)
        elif msg_type == "subscribe":
            channel = msg.get("channel", "default")
            old_ch = self._connections.get(websocket, "default")
            if old_ch in self._channels:
                self._channels[old_ch].discard(websocket)
                if not self._channels[old_ch]:
                    del self._channels[old_ch]
            self._connections[websocket] = channel
            if channel not in self._channels:
                self._channels[channel] = set()
            self._channels[channel].add(websocket)
            await websocket.send_json({"type": "subscribed", "channel": channel})
        else:
            await websocket.send_json({"type": "echo", "content": msg})

    async def handle_ai_message(self, websocket: WebSocket, msg: dict):
        if not self._ai_service:
            await websocket.send_json({"type": "error", "message": "AI 服务未初始化"})
            return
        params = msg.get("params", {})
        try:
            async for chunk in self._ai_service.stream_chat(params):
                await websocket.send_json({"type": "ai_chunk", "data": chunk})
            await websocket.send_json({"type": "ai_done"})
        except Exception as e:
            await websocket.send_json({"type": "ai_error", "message": str(e)})

    async def handle_terminal_message(self, websocket: WebSocket, msg: dict):
        import subprocess
        import os
        action = msg.get("action", "exec")
        if action == "exec":
            cmd = msg.get("command", "")
            cwd = msg.get("cwd", None)
            session_id = msg.get("session_id", "")
            if not cmd:
                await websocket.send_json({"type": "terminal_error", "message": "命令为空"})
                return
            try:
                proc = subprocess.Popen(
                    cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    cwd=cwd, creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
                self._terminal_sessions[session_id] = {"process": proc}
                stdout, stderr = proc.communicate(timeout=120)
                out = stdout.decode("utf-8", errors="replace")
                err = stderr.decode("utf-8", errors="replace")
                await websocket.send_json({
                    "type": "terminal_output",
                    "session_id": session_id,
                    "stdout": out,
                    "stderr": err,
                    "returnCode": proc.returncode,
                })
            except subprocess.TimeoutExpired:
                proc.kill()
                await websocket.send_json({
                    "type": "terminal_error",
                    "session_id": session_id,
                    "message": "命令执行超时",
                })
            except Exception as e:
                await websocket.send_json({
                    "type": "terminal_error",
                    "session_id": session_id,
                    "message": str(e),
                })
            finally:
                self._terminal_sessions.pop(session_id, None)
        elif action == "kill":
            session_id = msg.get("session_id", "")
            session = self._terminal_sessions.get(session_id)
            if session and session.get("process"):
                try:
                    session["process"].kill()
                except Exception:
                    pass
                self._terminal_sessions.pop(session_id, None)
                await websocket.send_json({"type": "terminal_killed", "session_id": session_id})
            else:
                await websocket.send_json({"type": "terminal_error", "message": "会话不存在"})

    async def _broadcast(self, channel: str, message: dict, exclude: WebSocket = None):
        if channel in self._channels:
            for ws in list(self._channels[channel]):
                if ws != exclude:
                    try:
                        await ws.send_json(message)
                    except Exception:
                        self.disconnect(ws, channel)
