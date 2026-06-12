"""Web Push 核心模块

2026-06-09 TASK-4.3 引入：Phase 4 W14 PWA 完整支持

设计：
  - VAPID 密钥对在首次启动时生成，持久化到 push_vapid.json
  - 订阅存储到 push_subscriptions.json（in-memory 缓存 + 文件持久化）
  - 推送通过 pywebpush + VAPID headers 发送
  - 推送策略：
      * severity = error: 总是推送
      * severity = warning: 默认推送（可配置）
      * severity = info: 不推送
  - 推送节流：同一 endpoint 60 秒内同类型只推一次
  - 失效订阅自动清理（410 Gone 或 404）

VAPID 协议：
  - Voluntary Application Server Identification
  - 让浏览器识别推送来源（避免冒充）
  - 包含 contact + 公钥
"""
from __future__ import annotations

import base64
import hashlib
import json
import logging
import os
import threading
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


# ──────────── 异常 ────────────


class PushError(Exception):
    pass


class VAPIDNotInitialized(PushError):
    pass


class SubscriptionNotFound(PushError):
    pass


# ──────────── VAPID 密钥管理 ────────────


@dataclass
class VAPIDKeys:
    """VAPID 密钥对"""
    public_key: str  # urlsafe-base64（前端订阅用）
    private_key_pem: str  # PEM 格式（后端签名用）
    subject: str  # contact 信息（mailto:... 或 https://...）
    created_at: float

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class VAPIDStore:
    """VAPID 密钥持久化

    存储路径：$APP_DATA/push_vapid.json
    首次启动时生成密钥对
    """

    _instance: Optional["VAPIDStore"] = None
    _lock = threading.Lock()

    def __init__(self, storage_dir: str):
        self.storage_dir = storage_dir
        self.keys_file = Path(storage_dir) / "push_vapid.json"
        self._keys: Optional[VAPIDKeys] = None

    @classmethod
    def instance(cls) -> "VAPIDStore":
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    # 默认存储在 app data 目录
                    app_dir = os.path.dirname(os.path.abspath(__file__))
                    app_data = os.environ.get("YUNJI_APP_DATA_DIR")
                    if not app_data:
                        app_data = os.path.join(app_dir, "data")
                    os.makedirs(app_data, exist_ok=True)
                    cls._instance = VAPIDStore(app_data)
        return cls._instance

    def get_or_generate(self, subject: str = "mailto:admin@yunjiai.local") -> VAPIDKeys:
        """获取密钥对，不存在则生成"""
        if self._keys is not None:
            return self._keys

        # 尝试从文件加载
        if self.keys_file.exists():
            try:
                with open(self.keys_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                self._keys = VAPIDKeys(
                    public_key=data["public_key"],
                    private_key_pem=data["private_key_pem"],
                    subject=data.get("subject", subject),
                    created_at=data.get("created_at", time.time()),
                )
                logger.info(f"[VAPID] loaded from {self.keys_file}")
                return self._keys
            except Exception as e:
                logger.warning(f"[VAPID] failed to load, regenerating: {e}")

        # 生成新的
        self._keys = self._generate(subject)
        self._save()
        return self._keys

    def regenerate(self, subject: str = "mailto:admin@yunjiai.local") -> VAPIDKeys:
        """重新生成密钥对（会废弃所有现有订阅）"""
        self._keys = self._generate(subject)
        self._save()
        logger.info(f"[VAPID] regenerated, all subscriptions invalidated")
        return self._keys

    def _generate(self, subject: str) -> VAPIDKeys:
        try:
            from cryptography.hazmat.primitives import serialization
            from cryptography.hazmat.primitives.asymmetric import ec
            from cryptography.hazmat.backends import default_backend

            # 生成 EC P-256 密钥对（VAPID 标准）
            private_key = ec.generate_private_key(ec.SECP256R1(), default_backend())
            private_pem = private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption(),
            ).decode("utf-8")

            public_key = private_key.public_key().public_numbers()
            # 转换为 uncompressed point (65 bytes: 0x04 + X + Y)
            x_bytes = public_key.x.to_bytes(32, "big")
            y_bytes = public_key.y.to_bytes(32, "big")
            uncompressed = b"\x04" + x_bytes + y_bytes

            # urlsafe-base64 编码（去掉 padding）
            public_key_b64 = base64.urlsafe_b64encode(uncompressed).rstrip(b"=").decode("ascii")

            return VAPIDKeys(
                public_key=public_key_b64,
                private_key_pem=private_pem,
                subject=subject,
                created_at=time.time(),
            )
        except ImportError as e:
            raise PushError(
                "cryptography library not installed, run: pip install cryptography pywebpush"
            ) from e

    def _save(self):
        try:
            os.makedirs(self.storage_dir, exist_ok=True)
            with open(self.keys_file, "w", encoding="utf-8") as f:
                json.dump(self._keys.to_dict(), f, indent=2, ensure_ascii=False)
            # 设置文件权限为 600（仅所有者可读写）
            try:
                os.chmod(self.keys_file, 0o600)
            except Exception:
                pass
        except Exception as e:
            logger.error(f"[VAPID] failed to save: {e}")


# ──────────── 订阅管理 ────────────


@dataclass
class PushSubscription:
    """单个推送订阅"""
    id: str  # sha256(endpoint)
    endpoint: str
    p256dh: str
    auth: str
    user_agent: str
    created_at: float
    last_seen_at: float
    last_pushed_at: float = 0.0
    push_count: int = 0
    fail_count: int = 0
    enabled: bool = True

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_request(cls, data: Dict[str, Any], user_agent: str = "") -> "PushSubscription":
        """从浏览器 pushManager.subscribe() 的结果构造"""
        endpoint = data.get("endpoint", "")
        if not endpoint:
            raise PushError("endpoint is required")
        keys = data.get("keys", {})
        p256dh = keys.get("p256dh", "")
        auth = keys.get("auth", "")
        if not p256dh or not auth:
            raise PushError("keys.p256dh and keys.auth are required")
        sub_id = hashlib.sha256(endpoint.encode("utf-8")).hexdigest()[:16]
        now = time.time()
        return cls(
            id=sub_id,
            endpoint=endpoint,
            p256dh=p256dh,
            auth=auth,
            user_agent=user_agent,
            created_at=now,
            last_seen_at=now,
        )


class SubscriptionStore:
    """订阅存储（in-memory + JSON 文件）"""

    _instance: Optional["SubscriptionStore"] = None
    _lock = threading.Lock()

    def __init__(self, storage_dir: str):
        self.storage_dir = storage_dir
        self.subs_file = Path(storage_dir) / "push_subscriptions.json"
        self._subs: Dict[str, PushSubscription] = {}
        self._load()

    @classmethod
    def instance(cls) -> "SubscriptionStore":
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    app_dir = os.path.dirname(os.path.abspath(__file__))
                    app_data = os.environ.get("YUNJI_APP_DATA_DIR")
                    if not app_data:
                        app_data = os.path.join(app_dir, "data")
                    os.makedirs(app_data, exist_ok=True)
                    cls._instance = SubscriptionStore(app_data)
        return cls._instance

    def _load(self):
        if not self.subs_file.exists():
            return
        try:
            with open(self.subs_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            for sid, sdata in data.get("subscriptions", {}).items():
                self._subs[sid] = PushSubscription(**sdata)
            logger.info(f"[Subs] loaded {len(self._subs)} subscriptions")
        except Exception as e:
            logger.warning(f"[Subs] failed to load: {e}")

    def _save(self):
        try:
            os.makedirs(self.storage_dir, exist_ok=True)
            data = {
                "subscriptions": {sid: s.to_dict() for sid, s in self._subs.items()},
                "updated_at": time.time(),
            }
            with open(self.subs_file, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
        except Exception as e:
            logger.error(f"[Subs] failed to save: {e}")

    def add(self, sub: PushSubscription) -> PushSubscription:
        """添加或更新订阅（按 endpoint 去重）"""
        with self._lock:
            existing = self._subs.get(sub.id)
            if existing:
                # 更新 p256dh / auth（key 可能轮换）
                existing.p256dh = sub.p256dh
                existing.auth = sub.auth
                existing.last_seen_at = sub.last_seen_at
                existing.user_agent = sub.user_agent
                existing.enabled = True
                existing.fail_count = 0
                self._save()
                return existing
            self._subs[sub.id] = sub
            self._save()
            logger.info(f"[Subs] added {sub.id} ({sub.user_agent[:30]})")
            return sub

    def remove(self, sub_id: str) -> bool:
        with self._lock:
            if sub_id in self._subs:
                del self._subs[sub_id]
                self._save()
                logger.info(f"[Subs] removed {sub_id}")
                return True
            return False

    def remove_by_endpoint(self, endpoint: str) -> bool:
        sub_id = hashlib.sha256(endpoint.encode("utf-8")).hexdigest()[:16]
        return self.remove(sub_id)

    def get(self, sub_id: str) -> Optional[PushSubscription]:
        return self._subs.get(sub_id)

    def list_all(self, enabled_only: bool = False) -> List[PushSubscription]:
        subs = list(self._subs.values())
        if enabled_only:
            subs = [s for s in subs if s.enabled]
        return subs

    def count(self) -> int:
        return len(self._subs)

    def mark_pushed(self, sub_id: str):
        with self._lock:
            sub = self._subs.get(sub_id)
            if sub:
                sub.last_pushed_at = time.time()
                sub.push_count += 1
                self._save()

    def mark_failed(self, sub_id: str, permanent: bool = False):
        with self._lock:
            sub = self._subs.get(sub_id)
            if sub:
                sub.fail_count += 1
                if permanent:
                    sub.enabled = False
                self._save()


# ──────────── 推送发送 ────────────


@dataclass
class PushPayload:
    """推送负载"""
    title: str
    body: str
    icon: str = "/icons/icon-192.svg"
    badge: str = "/icons/favicon.svg"
    url: str = "/"  # 点击通知后打开的 URL
    tag: str = ""  # 通知 tag（去重）
    ttl: int = 60 * 60 * 24  # 24h
    urgency: str = "normal"  # very-low / low / normal / high

    def to_json(self) -> str:
        return json.dumps(
            {
                "title": self.title,
                "body": self.body,
                "icon": self.icon,
                "badge": self.badge,
                "url": self.url,
                "tag": self.tag,
                "urgency": self.urgency,
            },
            ensure_ascii=False,
        )


@dataclass
class PushSendResult:
    """单次推送结果"""
    sub_id: str
    ok: bool
    status: int = 0
    error: str = ""
    permanent: bool = False  # 是否需要清理订阅


class PushSender:
    """推送发送器

    使用 pywebpush + VAPID 签名
    """

    def __init__(self, vapid_store: VAPIDStore, subs_store: SubscriptionStore):
        self.vapid_store = vapid_store
        self.subs_store = subs_store
        self._throttle_lock = threading.Lock()
        self._throttle_cache: Dict[str, float] = {}  # key: sub_id+tag, value: last push time

    def send_to_all(
        self,
        payload: PushPayload,
        throttle_tag: Optional[str] = None,
        throttle_seconds: int = 60,
    ) -> List[PushSendResult]:
        """推送给所有启用的订阅"""
        subs = self.subs_store.list_all(enabled_only=True)
        results: List[PushSendResult] = []
        for sub in subs:
            # 节流检查
            if throttle_tag:
                cache_key = f"{sub.id}:{throttle_tag}"
                with self._throttle_lock:
                    last = self._throttle_cache.get(cache_key, 0)
                    if time.time() - last < throttle_seconds:
                        logger.debug(f"[Push] throttled: {cache_key}")
                        continue
                    self._throttle_cache[cache_key] = time.time()

            result = self.send_to_one(sub, payload)
            results.append(result)

            if result.ok:
                self.subs_store.mark_pushed(sub.id)
            elif result.permanent:
                # 永久失败（410/404）→ 禁用订阅
                self.subs_store.mark_failed(sub.id, permanent=True)
                logger.info(f"[Push] disabled {sub.id}: {result.error}")

        return results

    def send_to_one(self, sub: PushSubscription, payload: PushPayload) -> PushSendResult:
        """推送给单个订阅"""
        try:
            from pywebpush import webpush, WebPushException
        except ImportError as e:
            return PushSendResult(
                sub_id=sub.id,
                ok=False,
                error="pywebpush not installed: pip install pywebpush",
            )

        vapid = self.vapid_store.get_or_generate()

        subscription_info = {
            "endpoint": sub.endpoint,
            "keys": {
                "p256dh": sub.p256dh,
                "auth": sub.auth,
            },
        }

        try:
            response = webpush(
                subscription_info=subscription_info,
                data=payload.to_json(),
                vapid_private_key=vapid.private_key_pem,
                vapid_claims={
                    "sub": vapid.subject,
                    "exp": int(time.time()) + 12 * 3600,  # 12h
                },
                ttl=payload.ttl,
                urgency=payload.urgency,
            )
            return PushSendResult(
                sub_id=sub.id,
                ok=True,
                status=response.status_code if response else 200,
            )
        except Exception as e:
            # pywebpush 异常处理
            from pywebpush import WebPushException

            error_msg = str(e)
            status = 0
            permanent = False

            if isinstance(e, WebPushException):
                if e.response is not None:
                    status = e.response.status_code
                    if status in (404, 410):
                        permanent = True
                        error_msg = f"subscription gone ({status})"

            # 失败计数
            self.subs_store.mark_failed(sub.id, permanent=permanent)

            return PushSendResult(
                sub_id=sub.id,
                ok=False,
                status=status,
                error=error_msg,
                permanent=permanent,
            )

    @staticmethod
    def _import_pywebpush():
        try:
            import pywebpush  # noqa
            return True
        except ImportError:
            return False


# ──────────── 全局单例 ────────────


def get_push_sender() -> PushSender:
    """获取全局 PushSender 单例"""
    vapid = VAPIDStore.instance()
    subs = SubscriptionStore.instance()
    return PushSender(vapid, subs)


# ──────────── 通知 → 推送 桥接 ────────────


def build_push_payload_from_notification(
    notif: Dict[str, Any],
    workspace_path: str = "",
) -> PushPayload:
    """从感知通知构建推送负载"""
    severity = notif.get("severity", "info")
    urgency = {
        "error": "high",
        "warning": "normal",
        "info": "low",
    }.get(severity, "normal")

    title_prefix = {
        "error": "❌ 错误",
        "warning": "⚠️ 警告",
        "info": "ℹ️ 提示",
    }.get(severity, "ℹ️ 提示")

    title = notif.get("title", "感知通知")
    description = notif.get("description", "")

    # 构造 URL（点击通知后打开应用并跳转到对应位置）
    url = "/"
    if notif.get("file_path"):
        url = f"/workspace?file={notif.get('file_path', '')}"
    elif notif.get("type") == "code_quality":
        url = "/quality"
    elif notif.get("type") == "security_risk":
        url = "/security"

    return PushPayload(
        title=f"{title_prefix} {title}",
        body=description,
        url=url,
        tag=f"yj-notif-{notif.get('id', '')[:8]}",
        urgency=urgency,
        ttl=60 * 60 * 4,  # 4h
    )


def should_push(notif: Dict[str, Any], min_severity: str = "warning") -> bool:
    """根据严重度判断是否需要推送"""
    severity = notif.get("severity", "info")
    levels = {"info": 0, "warning": 1, "error": 2}
    return levels.get(severity, 0) >= levels.get(min_severity, 1)
