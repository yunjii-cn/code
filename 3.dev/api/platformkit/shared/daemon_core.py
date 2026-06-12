"""Platform Shared - 独立 Daemon 模式核心 (Daemon Core)

2026-06-09 TASK-4.1 引入：Phase 4 W13 远程 Daemon 模式

设计目标：
    后端可独立运行（不依赖桌面端），方便远程访问。
    - 状态管理：PID 文件 + 状态文件（YUNJI_DAEMON_DIR/.yunji/daemon/）
    - 进程控制：start/stop/restart/status（CLI 由 daemon.py 提供）
    - 配置：监听地址、端口、Token、工作区、日志路径
    - 健康检查：HTTP 探测 /api/health + 进程存活

本模块职责：
    - DaemonConfig（配置数据类）
    - DaemonState（运行状态：running/stopped/crashed/starting）
    - DaemonManager（start/stop/status/restart + PID 管理 + 日志）

不负责：
    - CLI 命令行解析（daemon.py 负责）
    - uvicorn 启动（daemon.py 调 api_main 的 app）
    - systemd / Windows Service 集成（daemon.py 后续接入）

使用示例：
    from platformkit.shared.daemon_core import DaemonManager, DaemonConfig

    cfg = DaemonConfig(host="0.0.0.0", port=18080, workspace=Path("."))
    mgr = DaemonManager(cfg)

    # 启动
    pid = mgr.start()
    print(f"started pid={pid}")

    # 状态
    state = mgr.status()
    print(state.status, state.pid, state.uptime_seconds)

    # 停止
    mgr.stop()
"""

from __future__ import annotations

import json
import os
import platform
import signal
import socket
import time
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, Optional, Union


# ──────────── 状态枚举 ────────────


class DaemonStatus(str, Enum):
    """Daemon 运行状态"""

    STOPPED = "stopped"        # 未运行
    STARTING = "starting"      # 正在启动
    RUNNING = "running"        # 运行中
    CRASHED = "crashed"        # 崩溃（进程不在了，但状态文件还在）
    UNKNOWN = "unknown"        # 无法判断


# ──────────── 配置 / 状态数据类 ────────────


@dataclass
class DaemonConfig:
    """Daemon 配置"""

    host: str = "127.0.0.1"          # 监听地址（lan 模式改为 0.0.0.0）
    port: int = 18080                 # 监听端口
    workspace: Optional[str] = None    # 工作区路径
    daemon_dir: Optional[str] = None   # Daemon 数据目录（PID/状态/日志）
    log_file: Optional[str] = None     # 日志文件路径
    enable_lan: bool = False           # 是否启用 LAN 模式（生成 token）
    token: Optional[str] = None        # 配对码（自动生成 6 位 hex）
    extra_args: list = field(default_factory=list)  # 透传给 api_main 的额外参数

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "DaemonConfig":
        return cls(
            host=d.get("host", "127.0.0.1"),
            port=int(d.get("port", 18080)),
            workspace=d.get("workspace"),
            daemon_dir=d.get("daemon_dir"),
            log_file=d.get("log_file"),
            enable_lan=bool(d.get("enable_lan", False)),
            token=d.get("token"),
            extra_args=list(d.get("extra_args", [])),
        )


@dataclass
class DaemonState:
    """Daemon 运行时状态"""

    status: DaemonStatus = DaemonStatus.STOPPED
    pid: Optional[int] = None
    started_at: Optional[float] = None
    stopped_at: Optional[float] = None
    host: str = "127.0.0.1"
    port: int = 18080
    workspace: Optional[str] = None
    token: Optional[str] = None
    log_file: Optional[str] = None
    version: str = "unknown"
    last_health: Optional[float] = None  # 上次健康检查通过的时间

    @property
    def uptime_seconds(self) -> Optional[float]:
        if self.status == DaemonStatus.RUNNING and self.started_at:
            return max(0, time.time() - self.started_at)
        return None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["status"] = self.status.value
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "DaemonState":
        return cls(
            status=DaemonStatus(d.get("status", "stopped")),
            pid=d.get("pid"),
            started_at=d.get("started_at"),
            stopped_at=d.get("stopped_at"),
            host=d.get("host", "127.0.0.1"),
            port=int(d.get("port", 18080)),
            workspace=d.get("workspace"),
            token=d.get("token"),
            log_file=d.get("log_file"),
            version=d.get("version", "unknown"),
            last_health=d.get("last_health"),
        )


# ──────────── 默认目录 ────────────


# 全局 daemon 数据目录
def default_daemon_dir() -> Path:
    """默认 daemon 数据目录（~/.yunji/daemon/）"""
    if os.environ.get("YUNJI_DAEMON_DIR"):
        return Path(os.environ["YUNJI_DAEMON_DIR"])
    home = Path.home()
    return home / ".yunji" / "daemon"


def default_log_file(daemon_dir: Path) -> Path:
    """默认日志文件"""
    return daemon_dir / "daemon.log"


# ──────────── Daemon Manager ────────────


_PID_FILE = "daemon.pid"
_STATE_FILE = "daemon.json"
_CONFIG_FILE = "daemon.config.json"


class DaemonManager:
    """Daemon 管理器

    提供 start / stop / status / restart / health_check 接口。
    """

    def __init__(
        self,
        config: Optional[DaemonConfig] = None,
        daemon_dir: Optional[Union[Path, str]] = None,
    ) -> None:
        self.daemon_dir: Path = Path(daemon_dir) if daemon_dir else default_daemon_dir()
        self.daemon_dir.mkdir(parents=True, exist_ok=True)
        self.config: DaemonConfig = config or self._load_config() or DaemonConfig(
            daemon_dir=str(self.daemon_dir),
            log_file=str(default_log_file(self.daemon_dir)),
        )
        # 确保 config 包含 daemon_dir 和 log_file
        if not self.config.daemon_dir:
            self.config.daemon_dir = str(self.daemon_dir)
        if not self.config.log_file:
            self.config.log_file = str(default_log_file(self.daemon_dir))
        # 持久化配置
        self._save_config()

    # ──────────── 配置文件 ────────────

    def _config_path(self) -> Path:
        return self.daemon_dir / _CONFIG_FILE

    def _load_config(self) -> Optional[DaemonConfig]:
        p = self._config_path()
        if not p.exists():
            return None
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            return DaemonConfig.from_dict(data)
        except (json.JSONDecodeError, OSError):
            return None

    def _save_config(self) -> None:
        p = self._config_path()
        p.write_text(
            json.dumps(self.config.to_dict(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

    # ──────────── PID 文件 ────────────

    def _pid_path(self) -> Path:
        return self.daemon_dir / _PID_FILE

    def _read_pid(self) -> Optional[int]:
        p = self._pid_path()
        if not p.exists():
            return None
        try:
            return int(p.read_text(encoding="utf-8").strip())
        except (ValueError, OSError):
            return None

    def _write_pid(self, pid: int) -> None:
        self._pid_path().write_text(str(pid), encoding="utf-8")

    def _clear_pid(self) -> None:
        p = self._pid_path()
        if p.exists():
            try:
                p.unlink()
            except OSError:
                pass

    # ──────────── 状态文件 ────────────

    def _state_path(self) -> Path:
        return self.daemon_dir / _STATE_FILE

    def _read_state(self) -> Optional[DaemonState]:
        p = self._state_path()
        if not p.exists():
            return None
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            return DaemonState.from_dict(data)
        except (json.JSONDecodeError, OSError):
            return None

    def _write_state(self, state: DaemonState) -> None:
        self._state_path().write_text(
            json.dumps(state.to_dict(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

    # ──────────── 进程检查 ────────────

    @staticmethod
    def _is_pid_alive(pid: int) -> bool:
        """检查 PID 是否存活（跨平台）"""
        if pid is None or pid <= 0:
            return False
        try:
            if platform.system() == "Windows":
                # Windows: 用 OpenProcess
                import ctypes
                from ctypes import wintypes
                PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
                kernel32 = ctypes.windll.kernel32
                STILL_ACTIVE = 259
                h = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
                if h == 0:
                    return False
                try:
                    exit_code = wintypes.DWORD()
                    ok = kernel32.GetExitCodeProcess(h, ctypes.byref(exit_code))
                    if not ok:
                        return False
                    return exit_code.value == STILL_ACTIVE
                finally:
                    kernel32.CloseHandle(h)
            else:
                # Unix: kill 0
                try:
                    os.kill(pid, 0)
                except OSError:
                    return False
                return True
        except Exception:
            return False

    def _is_port_listening(self) -> bool:
        """检查配置的端口是否在监听（粗略检查）"""
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.settimeout(0.5)
                return s.connect_ex((self.config.host, self.config.port)) == 0
        except OSError:
            return False

    # ──────────── 公共 API ────────────

    def status(self) -> DaemonState:
        """获取当前 daemon 状态（实时探测）"""
        state = self._read_state() or DaemonState(host=self.config.host, port=self.config.port)
        pid = self._read_pid()
        if pid is None and state.pid is None:
            state.status = DaemonStatus.STOPPED
            return state
        # 检查 PID
        check_pid = pid or state.pid
        if check_pid and not self._is_pid_alive(check_pid):
            # 进程不在了 → crashed
            state.pid = check_pid
            state.status = DaemonStatus.CRASHED
            return state
        # 进程存在 → 看状态文件
        if state.status == DaemonStatus.STOPPED or state.status == DaemonStatus.CRASHED:
            # 状态文件不一致，但进程在 → 实际是 running（可能外部启动）
            state.status = DaemonStatus.RUNNING
        # 端口检查（健康度）
        if state.status == DaemonStatus.RUNNING and self._is_port_listening():
            state.last_health = time.time()
            self._write_state(state)
        return state

    def is_running(self) -> bool:
        return self.status().status == DaemonStatus.RUNNING

    def start(
        self,
        host: Optional[str] = None,
        port: Optional[int] = None,
        enable_lan: Optional[bool] = None,
        workspace: Optional[Union[Path, str]] = None,
        detached: bool = True,
    ) -> DaemonState:
        """启动 daemon

        Args:
            host: 覆盖监听地址
            port: 覆盖监听端口
            enable_lan: 是否启用 LAN（自动生成 token）
            workspace: 工作区路径
            detached: True = 后台运行（子进程）；False = 前台（阻塞）

        Returns:
            更新后的 DaemonState
        """
        if self.is_running():
            cur = self.status()
            raise RuntimeError(f"daemon already running: pid={cur.pid}")

        # 更新配置
        if host is not None:
            self.config.host = host
        if port is not None:
            self.config.port = port
        if enable_lan is not None:
            self.config.enable_lan = enable_lan
        if workspace is not None:
            self.config.workspace = str(Path(workspace).resolve())
        if self.config.enable_lan and not self.config.token:
            import secrets
            self.config.token = secrets.token_hex(3).upper()
        self._save_config()

        # 标记 starting
        state = DaemonState(
            status=DaemonStatus.STARTING,
            pid=None,
            started_at=time.time(),
            host=self.config.host,
            port=self.config.port,
            workspace=self.config.workspace,
            token=self.config.token,
            log_file=self.config.log_file,
        )
        self._write_state(state)

        if detached:
            pid = self._spawn_subprocess()
            state.pid = pid
            self._write_pid(pid)
            # 等 1-2 秒看是否启动成功
            time.sleep(1.5)
            # 再探测
            new_state = self.status()
            if new_state.status != DaemonStatus.RUNNING:
                # 启动失败
                self._clear_pid()
                state.status = DaemonStatus.CRASHED
                state.stopped_at = time.time()
                self._write_state(state)
                raise RuntimeError(
                    f"daemon failed to start (pid={pid}). Check log: {self.config.log_file}"
                )
            return new_state
        else:
            # 前台模式：调用方自己 uvicorn.run(app)
            state.status = DaemonStatus.RUNNING
            self._write_state(state)
            return state

    def stop(self, timeout: float = 5.0) -> DaemonState:
        """停止 daemon"""
        state = self.status()
        if state.status == DaemonStatus.STOPPED:
            return state
        pid = state.pid
        if pid and self._is_pid_alive(pid):
            try:
                if platform.system() == "Windows":
                    # Windows: 优雅关闭用 CTRL_BREAK_EVENT（需要 CREATE_NEW_PROCESS_GROUP）
                    # 否则用 terminate
                    import subprocess
                    subprocess.run(
                        ["taskkill", "/F", "/T", "/PID", str(pid)],
                        check=False,
                        capture_output=True,
                    )
                else:
                    os.kill(pid, signal.SIGTERM)
                    # 等待
                    start = time.time()
                    while self._is_pid_alive(pid) and (time.time() - start) < timeout:
                        time.sleep(0.2)
                    if self._is_pid_alive(pid):
                        os.kill(pid, signal.SIGKILL)
            except OSError as e:
                raise RuntimeError(f"failed to stop daemon pid={pid}: {e}")
        # 清理
        self._clear_pid()
        state.status = DaemonStatus.STOPPED
        state.stopped_at = time.time()
        state.pid = None
        self._write_state(state)
        return state

    def restart(self, **kwargs) -> DaemonState:
        """重启 daemon"""
        try:
            self.stop()
        except Exception:
            pass
        time.sleep(0.5)
        return self.start(**kwargs)

    def health_check(self) -> bool:
        """HTTP 健康检查（探测 /api/health）"""
        if not self.is_running():
            return False
        try:
            import urllib.request
            url = f"http://{self.config.host}:{self.config.port}/api/health"
            req = urllib.request.Request(url, method="GET")
            with urllib.request.urlopen(req, timeout=2) as resp:
                return resp.status == 200
        except Exception:
            return False

    def mark_running(self, pid: int, version: str = "unknown") -> DaemonState:
        """标记 daemon 为运行状态（由 daemon 子进程自己调用）"""
        state = DaemonState(
            status=DaemonStatus.RUNNING,
            pid=pid,
            started_at=time.time(),
            host=self.config.host,
            port=self.config.port,
            workspace=self.config.workspace,
            token=self.config.token,
            log_file=self.config.log_file,
            version=version,
        )
        self._write_pid(pid)
        self._write_state(state)
        return state

    def mark_stopped(self) -> DaemonState:
        """标记 daemon 为停止（由 daemon 子进程自己调用）"""
        state = self._read_state() or DaemonState()
        state.status = DaemonStatus.STOPPED
        state.stopped_at = time.time()
        state.pid = None
        self._write_state(state)
        self._clear_pid()
        return state

    # ──────────── 启动子进程 ────────────

    def _spawn_subprocess(self) -> int:
        """以子进程方式启动 daemon

        实际启动 api_main 的 uvicorn，注入 --dev 模式（不启动 pywebview）。
        """
        import subprocess
        import sys

        # 构造启动命令：python -c "<launcher code>"
        cmd = [
            sys.executable,
            "-c",
            self._build_launcher_code(),
        ]
        # 透传环境变量
        env = os.environ.copy()
        env["YUNJI_DAEMON_DIR"] = str(self.daemon_dir)
        env["YUNJI_DAEMON_PID_FILE"] = str(self._pid_path())
        env["YUNJI_DAEMON_STATE_FILE"] = str(self._state_path())
        env["YUNJI_DAEMON_HOST"] = self.config.host
        env["YUNJI_DAEMON_PORT"] = str(self.config.port)
        env["YUNJI_DAEMON_LAN"] = "1" if self.config.enable_lan else "0"
        if self.config.workspace:
            env["YUNJI_WORKSPACE"] = self.config.workspace
        # 日志文件
        log_path = self.config.log_file or str(default_log_file(self.daemon_dir))

        # Windows: 用 DETACHED_PROCESS + CREATE_NEW_PROCESS_GROUP 让子进程独立
        # Linux: 用 start_new_session
        kwargs: Dict[str, Any] = {
            "stdin": subprocess.DEVNULL,
            "stdout": open(log_path, "a", encoding="utf-8") if log_path else subprocess.DEVNULL,
            "stderr": subprocess.STDOUT,
            "env": env,
            "close_fds": True,
        }
        if platform.system() == "Windows":
            DETACHED_PROCESS = 0x00000008
            CREATE_NEW_PROCESS_GROUP = 0x00000200
            kwargs["creationflags"] = DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        else:
            kwargs["start_new_session"] = True

        proc = subprocess.Popen(cmd, **kwargs)
        return proc.pid

    def _build_launcher_code(self) -> str:
        """构造启动 api_main 的 Python 代码片段

        子进程会用 `os.getpid()` 动态获取自己的 PID，然后调用 mark_running。
        强制 --dev 模式（不启动 pywebview，仅 API 服务）。
        """
        return (
            "import sys, os; "
            "sys.path.insert(0, os.getcwd()); "
            # 强制 --dev 模式：仅 API，不启动 pywebview 窗口
            "host = os.environ.get('YUNJI_DAEMON_HOST', '127.0.0.1'); "
            "port = int(os.environ.get('YUNJI_DAEMON_PORT', '18080')); "
            "lan = os.environ.get('YUNJI_DAEMON_LAN', '0') == '1'; "
            "sys.argv = ['api_main', '--dev', '--host', host, '--port', str(port)] + (['--lan'] if lan else []); "
            "import api_main; "
            "from platformkit.shared.daemon_core import DaemonManager; "
            f"mgr = DaemonManager(daemon_dir=os.environ.get('YUNJI_DAEMON_DIR')); "
            "mgr.mark_running(pid=os.getpid(), version=api_main.VERSION); "
            "api_main.main()"
        )

    # ──────────── 自检 / 信息 ────────────

    def info(self) -> Dict[str, Any]:
        """完整信息（含配置 + 状态）"""
        state = self.status()
        return {
            "config": self.config.to_dict(),
            "state": state.to_dict(),
            "daemon_dir": str(self.daemon_dir),
            "is_alive": self._is_pid_alive(state.pid) if state.pid else False,
            "port_listening": self._is_port_listening(),
            "platform": platform.system(),
            "python_version": platform.python_version(),
        }


# ──────────── 工具函数 ────────────


def get_local_ip() -> str:
    """获取本机 IP（局域网友好）"""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except OSError:
        return socket.gethostbyname(socket.gethostname())


def format_uptime(seconds: Optional[float]) -> str:
    """格式化 uptime"""
    if seconds is None:
        return "—"
    seconds = int(seconds)
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        m, s = divmod(seconds, 60)
        return f"{m}m{s}s"
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    if s > 0:
        return f"{h}h{m}m{s}s"
    return f"{h}h{m}m"
