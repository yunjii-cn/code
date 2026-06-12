"""鉴权核心：JWT + 用户 + 工作区 + 设备

2026-06-09 TASK-4.4 引入：Phase 4 W14 多用户协作

四层鉴权模型（从 W13 扩展）：
  L1  配对码 (device)    ─ W13 已有，设备级标识
  L2  Tailscale 身份     ─ W13 已有，可信设备标记
  L3  JWT (user)         ─ 本模块新增，用户级（access + refresh）
  L4  工作区权限         ─ 本模块新增，字段级（owner/editor/viewer）

数据模型：
  User:           id, username, email, password_hash, display_name, created_at
  Workspace:      id, name, owner_id, created_at
  WorkspaceMember: workspace_id, user_id, role( owner/editor/viewer ), joined_at
  Device:         id, name, user_id, pair_token, last_seen, tailscale_ip

存储：SQLite (内置，零依赖)
"""
from __future__ import annotations

import hashlib
import hmac
import json
import logging
import os
import secrets
import sqlite3
import threading
import time
from dataclasses import dataclass, asdict, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


# ──────────── 配置 ────────────


@dataclass
class AuthConfig:
    jwt_secret: str
    access_token_ttl: int = 3600  # 1h
    refresh_token_ttl: int = 7 * 24 * 3600  # 7d
    password_min_length: int = 8
    bcrypt_rounds: int = 12
    db_path: str = ""  # 自动派生


# ──────────── 异常 ────────────


class AuthError(Exception):
    """基础鉴权错误"""
    pass


class InvalidCredentials(AuthError):
    pass


class TokenExpired(AuthError):
    pass


class TokenInvalid(AuthError):
    pass


class PermissionDenied(AuthError):
    pass


class UserNotFound(AuthError):
    pass


class WorkspaceNotFound(AuthError):
    pass


# ──────────── 模型 ────────────


@dataclass
class User:
    id: str
    username: str
    email: str
    display_name: str
    password_hash: str
    created_at: float
    last_login_at: Optional[float] = None
    is_active: bool = True

    def to_dict(self, include_sensitive: bool = False) -> Dict[str, Any]:
        d = {
            "id": self.id,
            "username": self.username,
            "email": self.email,
            "display_name": self.display_name,
            "created_at": self.created_at,
            "last_login_at": self.last_login_at,
            "is_active": self.is_active,
        }
        if include_sensitive:
            d["password_hash"] = self.password_hash
        return d


@dataclass
class Workspace:
    id: str
    name: str
    owner_id: str
    created_at: float
    description: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# 工作区角色
ROLE_OWNER = "owner"
ROLE_EDITOR = "editor"
ROLE_VIEWER = "viewer"

ROLE_LEVELS = {ROLE_VIEWER: 0, ROLE_EDITOR: 1, ROLE_OWNER: 2}


@dataclass
class WorkspaceMember:
    workspace_id: str
    user_id: str
    role: str  # owner/editor/viewer
    joined_at: float
    display_name: str = ""  # 冗余字段，避免 JOIN

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class Device:
    id: str
    name: str
    user_id: Optional[str]  # 可空（配对后绑定）
    pair_token: str
    created_at: float
    last_seen_at: float
    tailscale_ip: Optional[str] = None
    user_agent: str = ""
    enabled: bool = True

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ──────────── 存储 ────────────


class AuthStore:
    """SQLite 存储 + 内存缓存"""

    _instance: Optional["AuthStore"] = None
    _lock = threading.Lock()

    def __init__(self, config: AuthConfig):
        self.config = config
        if not config.db_path:
            app_dir = os.path.dirname(os.path.abspath(__file__))
            app_data = os.environ.get("YUNJI_APP_DATA_DIR") or os.path.join(app_dir, "data")
            os.makedirs(app_data, exist_ok=True)
            config.db_path = os.path.join(app_data, "auth.db")
        self._conn_lock = threading.Lock()
        self._init_db()

    @classmethod
    def instance(cls, config: Optional[AuthConfig] = None) -> "AuthStore":
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    if not config:
                        config = AuthConfig(
                            jwt_secret=os.environ.get("YUNJI_JWT_SECRET") or secrets.token_urlsafe(32),
                        )
                    cls._instance = AuthStore(config)
        return cls._instance

    def _conn(self) -> sqlite3.Connection:
        c = sqlite3.connect(self.config.db_path, check_same_thread=False)
        c.row_factory = sqlite3.Row
        return c

    def _init_db(self):
        with self._conn_lock:
            c = self._conn()
            try:
                c.executescript(
                    """
                    CREATE TABLE IF NOT EXISTS users (
                        id TEXT PRIMARY KEY,
                        username TEXT UNIQUE NOT NULL,
                        email TEXT,
                        display_name TEXT,
                        password_hash TEXT NOT NULL,
                        created_at REAL NOT NULL,
                        last_login_at REAL,
                        is_active INTEGER DEFAULT 1
                    );

                    CREATE TABLE IF NOT EXISTS workspaces (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        owner_id TEXT NOT NULL,
                        created_at REAL NOT NULL,
                        description TEXT DEFAULT '',
                        FOREIGN KEY (owner_id) REFERENCES users(id)
                    );

                    CREATE TABLE IF NOT EXISTS workspace_members (
                        workspace_id TEXT NOT NULL,
                        user_id TEXT NOT NULL,
                        role TEXT NOT NULL,
                        joined_at REAL NOT NULL,
                        display_name TEXT DEFAULT '',
                        PRIMARY KEY (workspace_id, user_id),
                        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
                    );

                    CREATE TABLE IF NOT EXISTS devices (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        user_id TEXT,
                        pair_token TEXT UNIQUE NOT NULL,
                        created_at REAL NOT NULL,
                        last_seen_at REAL NOT NULL,
                        tailscale_ip TEXT,
                        user_agent TEXT DEFAULT '',
                        enabled INTEGER DEFAULT 1,
                        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
                    );

                    CREATE TABLE IF NOT EXISTS refresh_tokens (
                        token TEXT PRIMARY KEY,
                        user_id TEXT NOT NULL,
                        created_at REAL NOT NULL,
                        expires_at REAL NOT NULL,
                        revoked INTEGER DEFAULT 0
                    );

                    CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);
                    CREATE INDEX IF NOT EXISTS idx_workspace_members_user ON workspace_members(user_id);
                    """
                )
                c.commit()
            finally:
                c.close()

    # ──────────── 密码 ────────────

    def _hash_password(self, password: str) -> str:
        """使用 PBKDF2-SHA256（无第三方依赖）"""
        salt = secrets.token_bytes(16)
        # 100,000 轮（OWASP 2026 推荐）
        h = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt, 100_000)
        return f"pbkdf2$100000${salt.hex()}${h.hex()}"

    def _verify_password(self, password: str, stored: str) -> bool:
        try:
            algo, iters, salt_hex, hash_hex = stored.split("$")
            if algo != "pbkdf2":
                return False
            salt = bytes.fromhex(salt_hex)
            expected = bytes.fromhex(hash_hex)
            h = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt, int(iters))
            return hmac.compare_digest(h, expected)
        except Exception:
            return False

    # ──────────── 用户 CRUD ────────────

    def create_user(self, username: str, password: str, email: str = "", display_name: str = "") -> User:
        if len(password) < self.config.password_min_length:
            raise AuthError(f"密码至少 {self.config.password_min_length} 位")
        if not username or len(username) < 3:
            raise AuthError("用户名至少 3 位")
        user_id = "u_" + secrets.token_urlsafe(8)
        now = time.time()
        user = User(
            id=user_id,
            username=username,
            email=email,
            display_name=display_name or username,
            password_hash=self._hash_password(password),
            created_at=now,
        )
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute(
                    """INSERT INTO users (id, username, email, display_name, password_hash, created_at, is_active)
                       VALUES (?, ?, ?, ?, ?, ?, 1)""",
                    (user.id, user.username, user.email, user.display_name, user.password_hash, user.created_at),
                )
                c.commit()
            except sqlite3.IntegrityError as e:
                if "username" in str(e):
                    raise AuthError(f"用户名 {username} 已存在")
                raise
            finally:
                c.close()
        logger.info(f"[Auth] user created: {username} ({user_id})")
        return user

    def get_user(self, user_id: str) -> Optional[User]:
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute("SELECT * FROM users WHERE id = ?", (user_id,)).fetchone()
                if not row:
                    return None
                return self._row_to_user(row)
            finally:
                c.close()

    def get_user_by_username(self, username: str) -> Optional[User]:
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute("SELECT * FROM users WHERE username = ?", (username,)).fetchone()
                if not row:
                    return None
                return self._row_to_user(row)
            finally:
                c.close()

    def authenticate(self, username: str, password: str) -> User:
        user = self.get_user_by_username(username)
        if not user or not user.is_active:
            raise InvalidCredentials("用户名或密码错误")
        if not self._verify_password(password, user.password_hash):
            raise InvalidCredentials("用户名或密码错误")
        # 更新 last_login_at
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute("UPDATE users SET last_login_at = ? WHERE id = ?", (time.time(), user.id))
                c.commit()
            finally:
                c.close()
        return user

    def _row_to_user(self, row: sqlite3.Row) -> User:
        return User(
            id=row["id"],
            username=row["username"],
            email=row["email"] or "",
            display_name=row["display_name"] or row["username"],
            password_hash=row["password_hash"],
            created_at=row["created_at"],
            last_login_at=row["last_login_at"],
            is_active=bool(row["is_active"]),
        )

    # ──────────── 工作区 ────────────

    def create_workspace(self, name: str, owner_id: str, description: str = "") -> Workspace:
        ws_id = "w_" + secrets.token_urlsafe(8)
        now = time.time()
        ws = Workspace(
            id=ws_id,
            name=name,
            owner_id=owner_id,
            created_at=now,
            description=description,
        )
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute(
                    "INSERT INTO workspaces (id, name, owner_id, created_at, description) VALUES (?, ?, ?, ?, ?)",
                    (ws.id, ws.name, ws.owner_id, ws.created_at, ws.description),
                )
                # 创建者自动成为 owner
                user = self.get_user(owner_id)
                display = user.display_name if user else ""
                c.execute(
                    """INSERT INTO workspace_members (workspace_id, user_id, role, joined_at, display_name)
                       VALUES (?, ?, ?, ?, ?)""",
                    (ws.id, owner_id, ROLE_OWNER, now, display),
                )
                c.commit()
            finally:
                c.close()
        return ws

    def get_workspace(self, workspace_id: str) -> Optional[Workspace]:
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute("SELECT * FROM workspaces WHERE id = ?", (workspace_id,)).fetchone()
                if not row:
                    return None
                return Workspace(
                    id=row["id"],
                    name=row["name"],
                    owner_id=row["owner_id"],
                    created_at=row["created_at"],
                    description=row["description"] or "",
                )
            finally:
                c.close()

    def list_user_workspaces(self, user_id: str) -> List[Tuple[Workspace, str]]:
        """返回 (workspace, role) 列表"""
        with self._conn_lock:
            c = self._conn()
            try:
                rows = c.execute(
                    """SELECT w.*, m.role FROM workspaces w
                       INNER JOIN workspace_members m ON m.workspace_id = w.id
                       WHERE m.user_id = ? ORDER BY m.joined_at DESC""",
                    (user_id,),
                ).fetchall()
                results = []
                for r in rows:
                    ws = Workspace(
                        id=r["id"],
                        name=r["name"],
                        owner_id=r["owner_id"],
                        created_at=r["created_at"],
                        description=r["description"] or "",
                    )
                    results.append((ws, r["role"]))
                return results
            finally:
                c.close()

    def add_member(self, workspace_id: str, user_id: str, role: str) -> WorkspaceMember:
        if role not in ROLE_LEVELS:
            raise AuthError(f"无效角色: {role}")
        now = time.time()
        user = self.get_user(user_id)
        if not user:
            raise UserNotFound(user_id)
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute(
                    """INSERT OR REPLACE INTO workspace_members
                       (workspace_id, user_id, role, joined_at, display_name)
                       VALUES (?, ?, ?, ?, ?)""",
                    (workspace_id, user_id, role, now, user.display_name),
                )
                c.commit()
            finally:
                c.close()
        return WorkspaceMember(
            workspace_id=workspace_id,
            user_id=user_id,
            role=role,
            joined_at=now,
            display_name=user.display_name,
        )

    def get_member(self, workspace_id: str, user_id: str) -> Optional[WorkspaceMember]:
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute(
                    """SELECT * FROM workspace_members WHERE workspace_id = ? AND user_id = ?""",
                    (workspace_id, user_id),
                ).fetchone()
                if not row:
                    return None
                return WorkspaceMember(
                    workspace_id=row["workspace_id"],
                    user_id=row["user_id"],
                    role=row["role"],
                    joined_at=row["joined_at"],
                    display_name=row["display_name"] or "",
                )
            finally:
                c.close()

    def check_permission(self, workspace_id: str, user_id: str, required_role: str) -> bool:
        """检查用户在工作区的角色是否 >= required_role"""
        member = self.get_member(workspace_id, user_id)
        if not member:
            return False
        return ROLE_LEVELS.get(member.role, -1) >= ROLE_LEVELS.get(required_role, 99)

    def remove_member(self, workspace_id: str, user_id: str) -> bool:
        with self._conn_lock:
            c = self._conn()
            try:
                cur = c.execute(
                    "DELETE FROM workspace_members WHERE workspace_id = ? AND user_id = ?",
                    (workspace_id, user_id),
                )
                c.commit()
                return cur.rowcount > 0
            finally:
                c.close()

    # ──────────── 设备 ────────────

    def create_device(self, name: str, pair_token: Optional[str] = None) -> Device:
        device_id = "d_" + secrets.token_urlsafe(8)
        token = pair_token or secrets.token_urlsafe(16)
        now = time.time()
        device = Device(
            id=device_id,
            name=name,
            user_id=None,
            pair_token=token,
            created_at=now,
            last_seen_at=now,
        )
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute(
                    """INSERT INTO devices (id, name, user_id, pair_token, created_at, last_seen_at, enabled)
                       VALUES (?, ?, ?, ?, ?, ?, 1)""",
                    (device.id, device.name, device.user_id, device.pair_token, device.created_at, device.last_seen_at),
                )
                c.commit()
            finally:
                c.close()
        return device

    def get_device_by_token(self, token: str) -> Optional[Device]:
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute("SELECT * FROM devices WHERE pair_token = ?", (token,)).fetchone()
                if not row:
                    return None
                return self._row_to_device(row)
            finally:
                c.close()

    def bind_device_to_user(self, device_id: str, user_id: str) -> bool:
        with self._conn_lock:
            c = self._conn()
            try:
                cur = c.execute(
                    "UPDATE devices SET user_id = ? WHERE id = ?",
                    (user_id, device_id),
                )
                c.commit()
                return cur.rowcount > 0
            finally:
                c.close()

    def touch_device(self, device_id: str, tailscale_ip: Optional[str] = None):
        with self._conn_lock:
            c = self._conn()
            try:
                if tailscale_ip:
                    c.execute(
                        "UPDATE devices SET last_seen_at = ?, tailscale_ip = ? WHERE id = ?",
                        (time.time(), tailscale_ip, device_id),
                    )
                else:
                    c.execute(
                        "UPDATE devices SET last_seen_at = ? WHERE id = ?",
                        (time.time(), device_id),
                    )
                c.commit()
            finally:
                c.close()

    def _row_to_device(self, row: sqlite3.Row) -> Device:
        return Device(
            id=row["id"],
            name=row["name"],
            user_id=row["user_id"],
            pair_token=row["pair_token"],
            created_at=row["created_at"],
            last_seen_at=row["last_seen_at"],
            tailscale_ip=row["tailscale_ip"],
            user_agent=row["user_agent"] or "",
            enabled=bool(row["enabled"]),
        )

    # ──────────── Refresh Token ────────────

    def save_refresh_token(self, token: str, user_id: str):
        now = time.time()
        expires = now + self.config.refresh_token_ttl
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute(
                    "INSERT OR REPLACE INTO refresh_tokens (token, user_id, created_at, expires_at, revoked) VALUES (?, ?, ?, ?, 0)",
                    (token, user_id, now, expires),
                )
                c.commit()
            finally:
                c.close()

    def validate_refresh_token(self, token: str) -> Optional[str]:
        """返回 user_id 或 None"""
        with self._conn_lock:
            c = self._conn()
            try:
                row = c.execute(
                    "SELECT * FROM refresh_tokens WHERE token = ? AND revoked = 0",
                    (token,),
                ).fetchone()
                if not row:
                    return None
                if time.time() > row["expires_at"]:
                    return None
                return row["user_id"]
            finally:
                c.close()

    def revoke_refresh_token(self, token: str):
        with self._conn_lock:
            c = self._conn()
            try:
                c.execute("UPDATE refresh_tokens SET revoked = 1 WHERE token = ?", (token,))
                c.commit()
            finally:
                c.close()


# ──────────── JWT 签发与校验 ────────────


class JWTManager:
    """轻量级 JWT（HS256，零依赖）"""

    def __init__(self, secret: str):
        self.secret = secret.encode("utf-8")

    def encode(self, payload: Dict[str, Any], ttl: int) -> str:
        """编码 payload 为 JWT"""
        import base64

        header = {"alg": "HS256", "typ": "JWT"}
        now = int(time.time())
        payload = {
            **payload,
            "iat": now,
            "exp": now + ttl,
            "jti": secrets.token_urlsafe(8),
        }
        # base64url 编码
        def b64url(d: Dict[str, Any]) -> str:
            s = json.dumps(d, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
            return base64.urlsafe_b64encode(s).rstrip(b"=").decode("ascii")

        h = b64url(header)
        p = b64url(payload)
        signing_input = f"{h}.{p}".encode("ascii")
        sig = hmac.new(self.secret, signing_input, hashlib.sha256).digest()
        sig_b64 = base64.urlsafe_b64encode(sig).rstrip(b"=").decode("ascii")
        return f"{h}.{p}.{sig_b64}"

    def decode(self, token: str) -> Dict[str, Any]:
        """解码 JWT，返回 payload"""
        import base64

        try:
            h, p, sig = token.split(".")
        except ValueError:
            raise TokenInvalid("token 格式错误")
        signing_input = f"{h}.{p}".encode("ascii")
        expected_sig = hmac.new(self.secret, signing_input, hashlib.sha256).digest()
        try:
            actual_sig = base64.urlsafe_b64decode(sig + "=" * (-len(sig) % 4))
        except Exception:
            raise TokenInvalid("签名解码失败")
        if not hmac.compare_digest(expected_sig, actual_sig):
            raise TokenInvalid("签名错误")
        # 补 padding
        p_padded = p + "=" * (-len(p) % 4)
        try:
            payload = json.loads(base64.urlsafe_b64decode(p_padded))
        except Exception:
            raise TokenInvalid("payload 解码失败")
        if "exp" in payload and time.time() > payload["exp"]:
            raise TokenExpired("token 已过期")
        return payload


# ──────────── 鉴权门面 ────────────


class AuthService:
    """对外暴露的鉴权服务"""

    def __init__(self, store: AuthStore, jwt: JWTManager, config: AuthConfig):
        self.store = store
        self.jwt = jwt
        self.config = config

    def register(
        self, username: str, password: str, email: str = "", display_name: str = ""
    ) -> Tuple[User, str, str]:
        """注册并返回 (user, access_token, refresh_token)"""
        user = self.store.create_user(username, password, email, display_name)
        access, refresh = self._issue_tokens(user)
        return user, access, refresh

    def login(self, username: str, password: str) -> Tuple[User, str, str]:
        """登录并返回 (user, access_token, refresh_token)"""
        user = self.store.authenticate(username, password)
        access, refresh = self._issue_tokens(user)
        return user, access, refresh

    def refresh(self, refresh_token: str) -> Tuple[str, str]:
        """刷新 access_token，返回 (new_access, new_refresh)"""
        user_id = self.store.validate_refresh_token(refresh_token)
        if not user_id:
            raise TokenInvalid("refresh token 无效")
        user = self.store.get_user(user_id)
        if not user or not user.is_active:
            raise TokenInvalid("用户不存在或已禁用")
        # 旋转 refresh token
        self.store.revoke_refresh_token(refresh_token)
        return self._issue_tokens(user)

    def _issue_tokens(self, user: User) -> Tuple[str, str]:
        access = self.jwt.encode(
            {
                "sub": user.id,
                "username": user.username,
                "type": "access",
            },
            ttl=self.config.access_token_ttl,
        )
        refresh = secrets.token_urlsafe(32)
        self.store.save_refresh_token(refresh, user.id)
        return access, refresh

    def verify_access_token(self, token: str) -> Dict[str, Any]:
        """校验 access token，返回 payload"""
        payload = self.jwt.decode(token)
        if payload.get("type") != "access":
            raise TokenInvalid("非 access token")
        return payload

    def get_user_from_token(self, token: str) -> Optional[User]:
        try:
            payload = self.verify_access_token(token)
            return self.store.get_user(payload["sub"])
        except (TokenInvalid, TokenExpired):
            return None


# ──────────── 全局单例 ────────────


_auth_service: Optional[AuthService] = None
_auth_lock = threading.Lock()


def get_auth_service() -> AuthService:
    global _auth_service
    if _auth_service is None:
        with _auth_lock:
            if _auth_service is None:
                config = AuthConfig(
                    jwt_secret=os.environ.get("YUNJI_JWT_SECRET") or secrets.token_urlsafe(32),
                )
                store = AuthStore.instance(config)
                jwt = JWTManager(config.jwt_secret)
                _auth_service = AuthService(store, jwt, config)
                logger.info(f"[Auth] service initialized, db={config.db_path}")
    return _auth_service
