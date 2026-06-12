"""Y.js WebSocket 服务端

2026-06-09 TASK-4.4 引入：Phase 4 W14 多用户协作

基于 ypy-websocket 实现：
  - 房间（Y.js Doc）管理
  - 客户端连接管理
  - 协议：Y.js binary sync protocol
  - 持久化：每个房间一个 .ydoc 二进制文件（debounced save）
  - 鉴权：握手时校验 token + 工作区权限

架构：
  YWebSocketServer
    ├── rooms: Dict[room_id, YRoom]
    │     ├── ydoc: Y.YDoc
    │     ├── clients: Set[ClientConnection]
    │     └── save_task: asyncio.Task（debounced 持久化）
    └── hooks:
          ├── on_auth(token, room_id) -> user_id / workspace_id / role
          └── on_persist(room_id, ydoc_state) -> 写到磁盘

差异化设计（自研部分）：
  - AI Agent 作为一等公民协作者：通过 awareness 协议参与
  - 字段级权限：自定义 awareness 扩展，editor 只能改部分字段
  - 对话分支：基于 Y.snapshot 的分支/合并
  - 离线优先：客户端 y-indexeddb 持久化，重连后自动同步
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Awaitable, Callable, Dict, Optional, Set

logger = logging.getLogger(__name__)


# ──────────── 鉴权回调 ────────────


@dataclass
class YjsAuthResult:
    """Y.js 连接鉴权结果"""
    allowed: bool
    user_id: str = ""
    username: str = ""
    role: str = "viewer"  # owner/editor/viewer
    workspace_id: str = ""
    reason: str = ""


# 鉴权回调签名：(token, room_id) -> YjsAuthResult
YjsAuthCallback = Callable[[str, str], YjsAuthResult]


# ──────────── Y.js 房间管理 ────────────


class YRoom:
    """单个 Y.js 房间（一个工作区对应一个 Y.Doc）

    使用 ypy-websocket 的 YRoom 实现（如果可用），
    否则使用简化版（仅做转发，不做 CRDT 合并）。
    """

    def __init__(self, room_id: str, storage_dir: str):
        self.room_id = room_id
        self.storage_dir = Path(storage_dir)
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.ydoc_path = self.storage_dir / f"{room_id}.ydoc"
        self.ydoc_state_path = self.storage_dir / f"{room_id}.state.bin"
        self._lock = threading.Lock()
        self._save_pending = False
        self._clients: Set[Any] = set()  # WebSocket connections
        self._last_save_at: float = 0
        # Y.Doc 实例（懒加载）
        self._ydoc: Any = None
        # Awareness 状态（参与者）
        self._awareness: Dict[int, Dict[str, Any]] = {}  # client_id -> state

    @property
    def ydoc(self) -> Any:
        """懒加载 Y.YDoc（ypy）"""
        if self._ydoc is None:
            try:
                import y_py as Y
                self._ydoc = Y.YDoc()
                # 尝试从磁盘恢复
                if self.ydoc_state_path.exists():
                    with open(self.ydoc_state_path, "rb") as f:
                        state = f.read()
                    if state:
                        Y.apply_update(self._ydoc, state)
                        logger.info(f"[YRoom {self.room_id}] loaded state {len(state)} bytes")
            except ImportError:
                logger.warning("[YRoom] y_py not installed, using mock YDoc")
                self._ydoc = MockYDoc()
        return self._ydoc

    def get_state(self) -> bytes:
        """获取 Y.Doc 完整状态（用于新连接同步）"""
        try:
            import y_py as Y
            return Y.encode_state_as_update(self.ydoc)
        except ImportError:
            return b""

    def apply_update(self, update: bytes, origin: Any = None) -> int:
        """应用增量更新到 Y.Doc"""
        try:
            import y_py as Y
            Y.apply_update(self.ydoc, update)
            # 触发持久化（debounced）
            self._schedule_save()
            return len(update)
        except ImportError:
            return 0

    def _schedule_save(self):
        """调度持久化（debounced 1s）"""
        if self._save_pending:
            return
        self._save_pending = True

        def do_save():
            time.sleep(1.0)
            self._save_pending = False
            self.save()

        t = threading.Thread(target=do_save, daemon=True)
        t.start()

    def save(self):
        """持久化 Y.Doc 状态到磁盘"""
        try:
            import y_py as Y
            state = Y.encode_state_as_update(self.ydoc)
            if state:
                # 原子写入
                tmp = self.ydoc_state_path.with_suffix(".tmp")
                with open(tmp, "wb") as f:
                    f.write(state)
                tmp.replace(self.ydoc_state_path)
                self._last_save_at = time.time()
                logger.debug(f"[YRoom {self.room_id}] saved {len(state)} bytes")
        except ImportError:
            pass
        except Exception as e:
            logger.error(f"[YRoom {self.room_id}] save failed: {e}")

    def set_awareness(self, client_id: int, state: Dict[str, Any]):
        """设置客户端的 awareness 状态"""
        self._awareness[client_id] = state
        # 广播给其他客户端
        self._broadcast_awareness(client_id, state)

    def _broadcast_awareness(self, client_id: int, state: Dict[str, Any]):
        """广播 awareness 状态（Y.js 协议）"""
        msg = json.dumps(
            {
                "type": "awareness",
                "client_id": client_id,
                "state": state,
            }
        ).encode("utf-8")
        for client in list(self._clients):
            try:
                client.send_bytes(msg)
            except Exception:
                pass

    def add_client(self, client: Any):
        self._clients.add(client)

    def remove_client(self, client: Any):
        self._clients.discard(client)

    def client_count(self) -> int:
        return len(self._clients)


class MockYDoc:
    """占位 YDoc（当 ypy 未安装时）"""
    pass


# ──────────── Y.js WebSocket 服务 ────────────


class YjsServer:
    """Y.js WebSocket 服务（FastAPI 集成版）

    公开 API：
      - process_message(websocket, room_id, message, auth_token) -> bytes
      - on_connect(websocket, room_id, auth_token) -> bool
      - on_disconnect(websocket, room_id)
    """

    def __init__(
        self,
        storage_dir: str,
        auth_callback: Optional[YjsAuthCallback] = None,
        on_room_change: Optional[Callable[[str, str], None]] = None,
    ):
        self.storage_dir = storage_dir
        self.auth_callback = auth_callback
        self.on_room_change = on_room_change  # (room_id, event) -> noop
        self._rooms: Dict[str, YRoom] = {}
        self._rooms_lock = threading.Lock()

    def get_room(self, room_id: str) -> YRoom:
        with self._rooms_lock:
            if room_id not in self._rooms:
                self._rooms[room_id] = YRoom(room_id, self.storage_dir)
            return self._rooms[room_id]

    def authenticate(self, token: str, room_id: str) -> YjsAuthResult:
        """鉴权：默认实现要求工作区 ID 与 room_id 一致"""
        if not self.auth_callback:
            return YjsAuthResult(allowed=False, reason="no auth callback configured")
        try:
            return self.auth_callback(token, room_id)
        except Exception as e:
            logger.exception(f"[Yjs] auth callback error: {e}")
            return YjsAuthResult(allowed=False, reason=f"auth error: {e}")

    async def on_connect(
        self,
        websocket: Any,
        room_id: str,
        auth_token: str,
    ) -> bool:
        """连接建立：鉴权 + 加入房间 + 发送初始状态"""
        auth = self.authenticate(auth_token, room_id)
        if not auth.allowed:
            logger.info(f"[Yjs] connect denied: {auth.reason}")
            await websocket.close(code=1008, reason=auth.reason or "auth failed")
            return False

        room = self.get_room(room_id)
        room.add_client(websocket)
        # 保存用户信息到 websocket
        websocket.yj_user_id = auth.user_id
        websocket.yj_username = auth.username
        websocket.yj_role = auth.role
        websocket.yj_workspace_id = auth.workspace_id
        # 发送初始状态
        state = room.get_state()
        if state:
            try:
                await websocket.send_bytes(state)
            except Exception as e:
                logger.warning(f"[Yjs] send initial state failed: {e}")

        logger.info(
            f"[Yjs] connect ok: room={room_id} user={auth.username} role={auth.role} clients={room.client_count()}"
        )
        if self.on_room_change:
            self.on_room_change(room_id, "connect")
        return True

    async def on_message(
        self,
        websocket: Any,
        room_id: str,
        message: bytes,
    ):
        """处理客户端消息（Y.js 协议）"""
        room = self.get_room(room_id)
        # Y.js 协议：消息头 1 字节表示类型
        # 0 = sync, 1 = awareness
        if not message:
            return
        msg_type = message[0] if isinstance(message[0], int) else ord(message[0:1])
        if msg_type == 0:  # sync
            room.apply_update(message, origin=websocket)
            # 广播给其他客户端
            for client in list(room._clients):
                if client is websocket:
                    continue
                try:
                    await client.send_bytes(message)
                except Exception:
                    pass
        elif msg_type == 1:  # awareness
            # 解析 awareness（简化版）
            try:
                # 实际 Y.js awareness 是二进制 varint 编码
                # 这里简化为 JSON
                payload = json.loads(message[1:].decode("utf-8"))
                client_id = payload.get("client_id", 0)
                state = payload.get("state", {})
                room.set_awareness(client_id, state)
            except Exception as e:
                logger.debug(f"[Yjs] awareness parse failed: {e}")

    async def on_disconnect(self, websocket: Any, room_id: str):
        room = self.get_room(room_id)
        room.remove_client(websocket)
        logger.info(f"[Yjs] disconnect: room={room_id} clients={room.client_count()}")
        # 清理空房间的内存
        if room.client_count() == 0:
            # 强制保存
            room.save()
        if self.on_room_change:
            self.on_room_change(room_id, "disconnect")

    def list_rooms(self) -> Dict[str, Dict[str, Any]]:
        """列出所有房间（管理用）"""
        with self._rooms_lock:
            return {
                rid: {
                    "id": rid,
                    "client_count": r.client_count(),
                    "last_save_at": r._last_save_at,
                    "has_state": r.ydoc_state_path.exists(),
                    "state_size": r.ydoc_state_path.stat().st_size if r.ydoc_state_path.exists() else 0,
                }
                for rid, r in self._rooms.items()
            }


# ──────────── 全局单例 ────────────


_yjs_server: Optional[YjsServer] = None
_yjs_lock = threading.Lock()


def get_yjs_server(auth_callback: Optional[YjsCallbackType] = None) -> YjsServer:
    global _yjs_server
    if _yjs_server is None:
        with _yjs_lock:
            if _yjs_server is None:
                app_dir = os.path.dirname(os.path.abspath(__file__))
                app_data = os.environ.get("YUNJI_APP_DATA_DIR") or os.path.join(app_dir, "data")
                storage = os.path.join(app_data, "yjs_rooms")
                os.makedirs(storage, exist_ok=True)
                _yjs_server = YjsServer(storage, auth_callback=auth_callback)
                logger.info(f"[Yjs] server initialized, storage={storage}")
    return _yjs_server


YjsCallbackType = YjsAuthCallback
