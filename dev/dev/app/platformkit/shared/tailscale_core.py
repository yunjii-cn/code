"""Platform Shared - Tailscale 集成核心 (Tailscale Core)

2026-06-09 TASK-4.2 引入：Phase 4 W13 Tailscale 远程访问

设计目标：
    自动检测 Tailscale 状态，生成安全的远程访问 URL。
    局域网优先（Tailscale 不可用时回退到局域网 IP）。
    Token 配对机制：电脑显示 6 位配对码，手机扫码/手动输入。

设计原则：
    1. **不暴露公网** — 只用 LAN + Tailscale 内网 IP（100.x.x.x）
    2. **不自动安装** — Tailscale 未安装时不强制安装，仅提示
    3. **配对码短时** — 6 位 hex（3 bytes），每次启动重新生成
    4. **配对码可关** — 用户可禁用配对码（仅 LAN 内网访问）

本模块职责：
    - TailscaleStatus（安装/未安装/运行中/未登录）
    - TailscaleInfo（IP/hostname/在线状态/账户/Net Check）
    - PairingCode（6 位 hex 配对码）
    - RemoteAccessHint（推荐访问 URL + 备选 URL）

不负责：
    - HTTP 路由（routes/tailscale.py）
    - Tailscale 安装/登录（提示用户自行操作）
    - QR 码生成（前端做）

使用示例：
    from platformkit.shared.tailscale_core import (
        TailscaleDetector, PairingCode, RemoteAccessHint
    )

    # 探测
    info = TailscaleDetector.detect()
    if info.installed and info.running:
        print(f"Tailscale IP: {info.ipv4}")
        # 用 info.ipv4 生成远程 URL

    # 配对码
    code = PairingCode.generate()
    print(f"配对码: {code.value}")
"""

from __future__ import annotations

import platform
import re
import secrets
import socket
import subprocess
import time
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Tuple


# ──────────── 状态枚举 ────────────


class TailscaleStatus(str, Enum):
    """Tailscale 安装/运行状态"""

    UNKNOWN = "unknown"        # 未检测
    NOT_INSTALLED = "not_installed"  # 未安装
    INSTALLED_STOPPED = "installed_stopped"  # 已安装但未运行
    RUNNING_LOGGED_OUT = "running_logged_out"  # 运行但未登录
    RUNNING_ONLINE = "running_online"  # 运行且在线
    ERROR = "error"            # 检测出错


# ──────────── 数据类 ────────────


@dataclass
class TailscaleInfo:
    """Tailscale 状态信息"""

    status: TailscaleStatus = TailscaleStatus.UNKNOWN
    installed: bool = False
    running: bool = False
    logged_in: bool = False
    online: bool = False  # 是否连接到 tailnet（不是 offline）
    ipv4: Optional[str] = None
    ipv6: Optional[str] = None
    hostname: Optional[str] = None
    tailnet: Optional[str] = None
    account: Optional[str] = None
    error: Optional[str] = None
    detected_at: float = field(default_factory=time.time)
    raw_output: Optional[str] = None

    def to_dict(self) -> Dict[str, any]:
        d = asdict(self)
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, any]) -> "TailscaleInfo":
        return cls(
            status=TailscaleStatus(d.get("status", "unknown")),
            installed=bool(d.get("installed", False)),
            running=bool(d.get("running", False)),
            logged_in=bool(d.get("logged_in", False)),
            online=bool(d.get("online", False)),
            ipv4=d.get("ipv4"),
            ipv6=d.get("ipv6"),
            hostname=d.get("hostname"),
            tailnet=d.get("tailnet"),
            account=d.get("account"),
            error=d.get("error"),
            detected_at=d.get("detected_at", time.time()),
            raw_output=d.get("raw_output"),
        )


@dataclass
class PairingCode:
    """配对码（6 位 hex 大写）"""

    value: str
    generated_at: float = field(default_factory=time.time)
    ttl_seconds: int = 3600  # 1 小时

    def to_dict(self) -> Dict[str, any]:
        return asdict(self)

    @property
    def is_expired(self) -> bool:
        return (time.time() - self.generated_at) > self.ttl_seconds

    @property
    def formatted(self) -> str:
        """格式化为 3-3 分组（更易读）"""
        if len(self.value) == 6:
            return f"{self.value[:3]}-{self.value[3:]}"
        return self.value


@dataclass
class RemoteAccessHint:
    """远程访问提示（主推荐 + 备选）"""

    primary_url: Optional[str] = None  # 推荐的 URL
    backup_urls: List[str] = field(default_factory=list)  # 备选 URL
    pairing_code: Optional[str] = None  # 配对码（6 位）
    note: Optional[str] = None  # 备注（如"Tailscale 已登录，建议用 100.x.x.x"）
    requires_token: bool = False  # URL 是否需要 ?token=xxx

    def to_dict(self) -> Dict[str, any]:
        return asdict(self)


# ──────────── Tailscale 探测 ────────────


class TailscaleDetector:
    """Tailscale 状态探测器

    设计原则：
    - 跨平台：Windows / macOS / Linux
    - 不强制安装：探测失败返回 NOT_INSTALLED，不抛异常
    - 缓存：探测结果 5 秒内复用（避免频繁调用 `tailscale` 命令）
    """

    CACHE_TTL = 5.0  # 5 秒缓存
    _cache: Optional[TailscaleInfo] = None
    _cache_time: float = 0.0

    @classmethod
    def detect(
        cls,
        use_cache: bool = True,
        timeout: float = 3.0,
    ) -> TailscaleInfo:
        """探测 Tailscale 状态"""
        # 缓存
        if use_cache and cls._cache and (time.time() - cls._cache_time) < cls.CACHE_TTL:
            return cls._cache

        info = TailscaleInfo()
        try:
            # 1. 检查是否安装（`tailscale` 命令是否存在）
            binary = cls._find_tailscale_binary()
            if not binary:
                info.status = TailscaleStatus.NOT_INSTALLED
                info.installed = False
                cls._cache = info
                cls._cache_time = time.time()
                return info

            info.installed = True

            # 2. 探测运行状态（`tailscale status --json` 或简化版）
            status_json = cls._run_tailscale([binary, "status", "--json"], timeout=timeout)
            if not status_json:
                info.status = TailscaleStatus.INSTALLED_STOPPED
                cls._cache = info
                cls._cache_time = time.time()
                return info

            info.running = True
            info.raw_output = status_json

            # 3. 解析 JSON
            import json
            try:
                data = json.loads(status_json)
            except json.JSONDecodeError as e:
                info.status = TailscaleStatus.ERROR
                info.error = f"JSON parse error: {e}"
                cls._cache = info
                cls._cache_time = time.time()
                return info

            # 4. 提取信息
            info.logged_in = bool(data.get("Self", {}).get("ID"))
            # BackendState: "Running" / "NeedsLogin" / "Stopped" / "Starting" 等
            backend_state = data.get("BackendState", "")
            info.online = backend_state == "Running" and info.logged_in
            info.tailnet = data.get("MagicDNSSuffix") or data.get("CurrentTailnet", {}).get("Name")
            # 自身信息
            self_node = data.get("Self", {}) or {}
            addrs = self_node.get("TailscaleIPs", []) or []
            for addr in addrs:
                if ":" not in addr and info.ipv4 is None:
                    info.ipv4 = addr
                elif ":" in addr and info.ipv6 is None:
                    info.ipv6 = addr
            info.hostname = self_node.get("HostName") or self_node.get("DNSName")
            if info.hostname and info.tailnet and info.hostname.endswith("." + info.tailnet):
                info.hostname = info.hostname[: -(len(info.tailnet) + 1)]
            # 账户（UserID → 通过 status 查不到，需要 `tailscale whois` 或 status）
            user_id = self_node.get("UserID")
            if user_id:
                info.account = f"user-{user_id}"

            # 5. 综合状态
            if info.logged_in and info.online:
                info.status = TailscaleStatus.RUNNING_ONLINE
            elif info.running and not info.logged_in:
                info.status = TailscaleStatus.RUNNING_LOGGED_OUT
            elif info.running:
                info.status = TailscaleStatus.RUNNING_ONLINE
            else:
                info.status = TailscaleStatus.INSTALLED_STOPPED

        except subprocess.TimeoutExpired:
            info.status = TailscaleStatus.ERROR
            info.error = "tailscale command timeout"
        except FileNotFoundError:
            info.status = TailscaleStatus.NOT_INSTALLED
            info.installed = False
        except Exception as e:
            info.status = TailscaleStatus.ERROR
            info.error = f"{type(e).__name__}: {e}"

        cls._cache = info
        cls._cache_time = time.time()
        return info

    @staticmethod
    def _find_tailscale_binary() -> Optional[str]:
        """查找 tailscale 可执行文件"""
        candidates = ["tailscale"]
        if platform.system() == "Windows":
            candidates = [
                r"C:\Program Files\Tailscale\tailscale.exe",
                r"C:\Program Files (x86)\Tailscale\tailscale.exe",
                "tailscale.exe",
            ]
        elif platform.system() == "Darwin":
            candidates = [
                "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
                "/usr/local/bin/tailscale",
                "/opt/homebrew/bin/tailscale",
                "tailscale",
            ]
        else:
            candidates = [
                "/usr/bin/tailscale",
                "/usr/local/bin/tailscale",
                "/snap/bin/tailscale",
                "tailscale",
            ]
        for c in candidates:
            try:
                if platform.system() == "Windows" and "\\" in c:
                    # 绝对路径
                    if Path(c).exists():
                        return c
                else:
                    # 用 which/where 找
                    result = subprocess.run(
                        ["which", c] if platform.system() != "Windows" else ["where", c],
                        capture_output=True,
                        text=True,
                        timeout=2,
                    )
                    if result.returncode == 0 and result.stdout.strip():
                        return result.stdout.strip().split("\n")[0].strip()
            except Exception:
                continue
        return None

    @staticmethod
    def _run_tailscale(cmd: List[str], timeout: float = 3.0) -> Optional[str]:
        """执行 tailscale 子命令"""
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            if result.returncode == 0:
                return result.stdout
            return None
        except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
            return None

    @classmethod
    def clear_cache(cls) -> None:
        """清缓存"""
        cls._cache = None
        cls._cache_time = 0.0


# ──────────── 配对码 ────────────


class PairingCodeStore:
    """配对码存储（进程内）"""

    _instance: Optional["PairingCodeStore"] = None
    _current: Optional[PairingCode] = None

    def __init__(self) -> None:
        pass

    @classmethod
    def instance(cls) -> "PairingCodeStore":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def get_or_generate(self, ttl_seconds: int = 3600) -> PairingCode:
        """获取当前配对码（过期则重新生成）"""
        if self._current is None or self._current.is_expired or self._current.ttl_seconds != ttl_seconds:
            self._current = PairingCode(
                value=secrets.token_hex(3).upper(),
                ttl_seconds=ttl_seconds,
            )
        return self._current

    def regenerate(self, ttl_seconds: int = 3600) -> PairingCode:
        """强制重新生成"""
        self._current = PairingCode(
            value=secrets.token_hex(3).upper(),
            ttl_seconds=ttl_seconds,
        )
        return self._current

    def verify(self, code: str) -> bool:
        """校验配对码（常量时间比较）"""
        if not self._current or self._current.is_expired:
            return False
        return secrets.compare_digest(self._current.value, code.upper())

    def clear(self) -> None:
        self._current = None


# 兼容 PairingCode.generate() 静态接口
def PairingCode_generate() -> PairingCode:  # noqa: N802
    return PairingCodeStore.instance().get_or_generate()


# ──────────── 远程访问 URL 生成 ────────────


def get_lan_ip() -> Optional[str]:
    """获取本机局域网 IP（用于 fallback）"""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except OSError:
        try:
            return socket.gethostbyname(socket.gethostname())
        except OSError:
            return None


def build_remote_hint(
    port: int = 18080,
    info: Optional[TailscaleInfo] = None,
    pairing_code: Optional[str] = None,
) -> RemoteAccessHint:
    """构建远程访问提示

    优先级：
        1. Tailscale 在线 → 推荐 Tailscale IP
        2. LAN IP → fallback
        3. 127.0.0.1 → 仅本机
    """
    if info is None:
        info = TailscaleDetector.detect()

    hint = RemoteAccessHint()
    hint.pairing_code = pairing_code
    hint.requires_token = pairing_code is not None

    urls: List[str] = []
    note: Optional[str] = None

    # 1. Tailscale 优先
    if info.status == TailscaleStatus.RUNNING_ONLINE and info.ipv4:
        url = f"http://{info.ipv4}:{port}"
        if pairing_code:
            url += f"?token={pairing_code}"
        hint.primary_url = url
        note = f"✅ Tailscale 已登录（{info.tailnet or 'tailnet'}）"
    elif info.installed and info.running and not info.logged_in:
        note = "ℹ️  Tailscale 已运行但未登录，可用 LAN IP 访问"
    elif info.installed and not info.running:
        note = "ℹ️  Tailscale 已安装但未启动"
    elif not info.installed:
        note = "ℹ️  未安装 Tailscale，仅 LAN 访问"
    else:
        note = f"ℹ️  Tailscale 状态异常: {info.status.value}"

    # 2. LAN IP
    lan_ip = get_lan_ip()
    if lan_ip:
        url = f"http://{lan_ip}:{port}"
        if pairing_code:
            url += f"?token={pairing_code}"
        urls.append(url)

    # 3. Tailscale hostname（如果有）
    if info.hostname and info.tailnet and info.status == TailscaleStatus.RUNNING_ONLINE:
        hostname_url = f"http://{info.hostname}.{info.tailnet}:{port}"
        if pairing_code:
            hostname_url += f"?token={pairing_code}"
        urls.append(hostname_url)

    # 4. 127.0.0.1
    urls.append(f"http://127.0.0.1:{port}")

    if not hint.primary_url:
        hint.primary_url = urls[0] if urls else None
    hint.backup_urls = urls[1:]
    hint.note = note
    return hint


# ──────────── 状态摘要 ────────────


def tailscale_summary() -> Dict[str, any]:
    """Tailscale 状态摘要（供 /api/tailscale/status 端点）"""
    info = TailscaleDetector.detect()
    return {
        "tailscale": info.to_dict(),
        "platform": platform.system(),
        "lan_ip": get_lan_ip(),
    }
