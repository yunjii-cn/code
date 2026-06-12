import json
from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from typing import Dict, Set

from services.ws import WebSocketManager

router = APIRouter(tags=["WebSocket"])
ws_manager = WebSocketManager()


@router.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket主端点，用于实时通信"""
    await ws_manager.connect(websocket)
    try:
        while True:
            data = await websocket.receive_text()
            msg = ws_manager.parse_message(data)
            await ws_manager.handle_message(websocket, msg)
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket)


@router.websocket("/ws/ai/{session_id}")
async def websocket_ai_stream(websocket: WebSocket, session_id: str):
    """AI对话流式WebSocket端点"""
    await ws_manager.connect(websocket, channel=f"ai:{session_id}")
    try:
        while True:
            data = await websocket.receive_text()
            msg = ws_manager.parse_message(data)
            msg["session_id"] = session_id
            await ws_manager.handle_ai_message(websocket, msg)
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket, channel=f"ai:{session_id}")


@router.websocket("/ws/terminal")
async def websocket_terminal(websocket: WebSocket):
    """终端WebSocket端点"""
    await ws_manager.connect(websocket, channel="terminal")
    try:
        while True:
            data = await websocket.receive_text()
            msg = ws_manager.parse_message(data)
            await ws_manager.handle_terminal_message(websocket, msg)
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket, channel="terminal")
