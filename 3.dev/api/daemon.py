"""dev/app/daemon.py

2026-06-10 TASK-4.1 引入：独立 Daemon 模式 CLI 入口

位置：dev/app/daemon.py
关联：dev/app/api_main.py（FastAPI app）+ dev/app/platformkit/shared/daemon_core.py（核心库）

目标：把 FastAPI 后端从 desktop shell 拆出来，作为独立 daemon 进程
- CLI 控制：start / stop / restart / status / logs / config
- 后台运行（detached=True）
- 健康检查：HTTP 探测 /api/health + 进程存活
- 配置持久化：~/.yunji/daemon/{config.json, state.json, pid, log}
- 支持 LAN 模式（自动生成 token）

使用：
  # 启动
  python dev/app/daemon.py start                          # 默认 127.0.0.1:18080
  python dev/app/daemon.py start --host 0.0.0.0 --port 8080
  python dev/app/daemon.py start --lan                    # 启用 LAN 模式（生成 token）
  python dev/app/daemon.py start --workspace /path/to/ws

  # 查看状态
  python dev/app/daemon.py status                         # 详细状态
  python dev/app/daemon.py status --json                  # JSON 格式

  # 重启 / 停止
  python dev/app/daemon.py restart
  python dev/app/daemon.py stop --timeout 10

  # 日志
  python dev/app/daemon.py logs                           # 最近 50 行
  python dev/app/daemon.py logs -n 200                    # 最近 200 行
  python dev/app/daemon.py logs -f                        # 跟踪（tail -f）

  # 配置
  python dev/app/daemon.py config --show                  # 显示当前配置
  python dev/app/daemon.py config --port 8080             # 改端口
  python dev/app/daemon.py config --lan on                # 开关 LAN
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any, Dict, Optional

# 允许独立运行：把当前文件目录加到 sys.path
_APP_DIR = Path(__file__).resolve().parent
if str(_APP_DIR) not in sys.path:
    sys.path.insert(0, str(_APP_DIR))

from platformkit.shared.daemon_core import (  # noqa: E402
    DaemonConfig,
    DaemonManager,
    DaemonState,
    DaemonStatus,
    default_daemon_dir,
    format_uptime,
    get_local_ip,
)


# ──────────── 颜色（终端 ANSI） ────────────


class C:
    """ANSI 颜色码（不支持时自动降级）"""
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    GRAY = "\033[90m"

    @classmethod
    def disable(cls):
        for attr in dir(cls):
            if attr.isupper() and not attr.startswith("_"):
                setattr(cls, attr, "")


# 检测是否 TTY / 强制禁用
if not sys.stdout.isatty() or os.environ.get("YUNJI_DAEMON_NO_COLOR") or os.environ.get("NO_COLOR"):
    C.disable()


def cprint(color: str, msg: str) -> None:
    print(f"{color}{msg}{C.RESET}")


def ok(msg: str) -> None:
    cprint(f"{C.GREEN}✓{C.RESET}", msg)


def warn(msg: str) -> None:
    cprint(f"{C.YELLOW}!{C.RESET}", msg)


def err(msg: str) -> None:
    cprint(f"{C.RED}✗{C.RESET}", msg)


def info(msg: str) -> None:
    cprint(f"{C.BLUE}→{C.RESET}", msg)


# ──────────── 子命令：start ────────────


def cmd_start(args: argparse.Namespace) -> int:
    """启动 daemon"""
    try:
        mgr = DaemonManager()
        state = mgr.start(
            host=args.host,
            port=args.port,
            enable_lan=args.lan,
            workspace=args.workspace,
            detached=not args.foreground,
        )
    except RuntimeError as e:
        err(str(e))
        return 1
    except Exception as e:  # noqa: BLE001
        err(f"启动失败: {e}")
        return 2

    if state.status == DaemonStatus.RUNNING:
        ok(f"daemon 已启动 (pid={state.pid})")
        info(f"  监听地址:  http://{state.host}:{state.port}")
        if state.token:
            info(f"  配对 token: {state.token}（LAN 模式）")
        info(f"  日志文件:  {state.log_file}")
        if state.workspace:
            info(f"  工作区:    {state.workspace}")
        return 0
    if state.status == DaemonStatus.STARTING:
        warn(f"daemon 正在启动中 (pid={state.pid})，请稍后用 'status' 检查")
        return 0
    err(f"daemon 启动失败: status={state.status.value}")
    return 1


# ──────────── 子命令：stop ────────────


def cmd_stop(args: argparse.Namespace) -> int:
    """停止 daemon"""
    try:
        mgr = DaemonManager()
        if not mgr.is_running():
            warn("daemon 未在运行")
            return 0
        state = mgr.stop(timeout=args.timeout)
    except Exception as e:  # noqa: BLE001
        err(f"停止失败: {e}")
        return 2

    if state.status == DaemonStatus.STOPPED:
        ok("daemon 已停止")
        return 0
    warn(f"daemon 未完全停止: status={state.status.value}")
    return 1


# ──────────── 子命令：restart ────────────


def cmd_restart(args: argparse.Namespace) -> int:
    """重启 daemon"""
    try:
        mgr = DaemonManager()
        if mgr.is_running():
            info("停止现有 daemon ...")
            mgr.stop(timeout=args.timeout)
            time.sleep(0.5)
        info("启动新 daemon ...")
        state = mgr.start(detached=True)
    except Exception as e:  # noqa: BLE001
        err(f"重启失败: {e}")
        return 2

    if state.status == DaemonStatus.RUNNING:
        ok(f"daemon 已重启 (pid={state.pid})")
        info(f"  监听: http://{state.host}:{state.port}")
        return 0
    err(f"daemon 重启失败: status={state.status.value}")
    return 1


# ──────────── 子命令：status ────────────


def _format_state(state: DaemonState) -> None:
    """格式化输出 daemon 状态（人类可读）"""
    status_color = {
        DaemonStatus.RUNNING: C.GREEN,
        DaemonStatus.STOPPED: C.GRAY,
        DaemonStatus.CRASHED: C.RED,
        DaemonStatus.STARTING: C.YELLOW,
        DaemonStatus.UNKNOWN: C.MAGENTA,
    }.get(state.status, C.RESET)

    cprint(f"{C.BOLD}=== Yunji Daemon Status ==={C.RESET}", "")
    print(f"  状态:     {status_color}{state.status.value}{C.RESET}")
    print(f"  PID:      {state.pid or '(none)'}")
    print(f"  监听:     http://{state.host}:{state.port}")
    if state.workspace:
        print(f"  工作区:   {state.workspace}")
    if state.token:
        print(f"  Token:    {C.MAGENTA}{state.token}{C.RESET} (LAN 模式)")
    if state.started_at:
        uptime = format_uptime(time.time() - state.started_at)
        print(f"  启动时间: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(state.started_at))}（运行 {uptime}）")
    if state.last_health:
        ago = format_uptime(time.time() - state.last_health)
        print(f"  健康:     {C.GREEN}OK{C.RESET}（{ago}前）")
    if state.stopped_at and state.status == DaemonStatus.STOPPED:
        ago = format_uptime(time.time() - state.stopped_at)
        print(f"  上次停止: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(state.stopped_at))}（{ago}前）")
    print(f"  版本:     {state.version}")


def cmd_status(args: argparse.Namespace) -> int:
    """查看 daemon 状态"""
    try:
        mgr = DaemonManager()
        state = mgr.status()
    except Exception as e:  # noqa: BLE001
        err(f"状态查询失败: {e}")
        return 2

    if args.json:
        print(json.dumps(state.to_dict(), ensure_ascii=False, indent=2))
    else:
        _format_state(state)
    return 0 if state.status == DaemonStatus.RUNNING else 1


# ──────────── 子命令：logs ────────────


def cmd_logs(args: argparse.Namespace) -> int:
    """查看日志"""
    log_file = args.log_file
    if not log_file:
        cfg = DaemonConfig()
        log_file = cfg.log_file or str(default_daemon_dir() / "daemon.log")

    p = Path(log_file)
    if not p.exists():
        warn(f"日志文件不存在: {p}")
        return 1

    if args.follow:
        # tail -f 行为
        import select
        size = 0
        try:
            with open(p, "r", encoding="utf-8", errors="replace") as f:
                # 先输出最后 N 行
                lines = f.readlines()
                start = max(0, len(lines) - args.lines)
                for line in lines[start:]:
                    print(line, end="")
                size = f.tell()
                # 然后跟踪
                while True:
                    time.sleep(0.5)
                    line = f.readline()
                    if line:
                        print(line, end="")
                    else:
                        # 检查文件是否被截断/重写
                        if p.exists() and p.stat().st_size < size:
                            f.seek(0)
                            size = 0
        except KeyboardInterrupt:
            print()  # 换行
        return 0

    # 非 follow 模式：输出最后 N 行
    with open(p, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()
    start = max(0, len(lines) - args.lines)
    for line in lines[start:]:
        print(line, end="")
    return 0


# ──────────── 子命令：config ────────────


def cmd_config(args: argparse.Namespace) -> int:
    """管理 daemon 配置"""
    mgr = DaemonManager()

    if args.show:
        # 显示当前配置
        print(json.dumps(mgr.config.to_dict(), ensure_ascii=False, indent=2))
        return 0

    # 修改配置
    changed = False
    if args.host is not None:
        mgr.config.host = args.host
        changed = True
    if args.port is not None:
        mgr.config.port = args.port
        changed = True
    if args.workspace is not None:
        mgr.config.workspace = str(Path(args.workspace).resolve())
        changed = True
    if args.lan is not None:
        v = args.lan.lower()
        mgr.config.enable_lan = v in ("1", "true", "yes", "on")
        changed = True

    if not changed:
        warn("未指定任何 --host/--port/--workspace/--lan 改动；用 --show 查看当前配置")
        return 1

    mgr._save_config()
    ok("配置已更新")
    print(json.dumps(mgr.config.to_dict(), ensure_ascii=False, indent=2))
    return 0


# ──────────── 子命令：info（显示路径信息） ────────────


def cmd_info(_args: argparse.Namespace) -> int:
    """显示 daemon 数据目录和默认路径"""
    d = default_daemon_dir()
    print(f"{C.BOLD}=== Daemon Info ==={C.RESET}")
    print(f"  数据目录:    {d}")
    print(f"  配置文件:    {d / 'config.json'}")
    print(f"  状态文件:    {d / 'state.json'}")
    print(f"  PID 文件:    {d / 'daemon.pid'}")
    print(f"  默认日志:    {d / 'daemon.log'}")
    print(f"  局域网 IP:   {get_local_ip()}")
    return 0


# ──────────── argparse ────────────


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="yunji-daemon",
        description="云集智能编程工作站 · 独立 Daemon 模式控制台",
    )

    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")

    # start
    p_start = sub.add_parser("start", help="启动 daemon")
    p_start.add_argument("--host", default=None, help="监听地址（默认 127.0.0.1）")
    p_start.add_argument("--port", type=int, default=None, help="监听端口（默认 18080）")
    p_start.add_argument("--lan", action="store_true", help="启用 LAN 模式（自动生成 token）")
    p_start.add_argument("--workspace", default=None, help="工作区路径")
    p_start.add_argument("--foreground", "-f", action="store_true", help="前台运行（不分离）")
    p_start.set_defaults(func=cmd_start)

    # stop
    p_stop = sub.add_parser("stop", help="停止 daemon")
    p_stop.add_argument("--timeout", type=float, default=5.0, help="超时（秒）")
    p_stop.set_defaults(func=cmd_stop)

    # restart
    p_restart = sub.add_parser("restart", help="重启 daemon")
    p_restart.add_argument("--timeout", type=float, default=5.0, help="停止超时（秒）")
    p_restart.set_defaults(func=cmd_restart)

    # status
    p_status = sub.add_parser("status", help="查看状态")
    p_status.add_argument("--json", action="store_true", help="JSON 格式输出")
    p_status.set_defaults(func=cmd_status)

    # logs
    p_logs = sub.add_parser("logs", help="查看日志")
    p_logs.add_argument("-n", "--lines", type=int, default=50, help="显示行数（默认 50）")
    p_logs.add_argument("-f", "--follow", action="store_true", help="跟踪模式（tail -f）")
    p_logs.add_argument("--log-file", default=None, help="日志文件路径（默认自动）")
    p_logs.set_defaults(func=cmd_logs)

    # config
    p_config = sub.add_parser("config", help="管理配置")
    p_config.add_argument("--show", action="store_true", help="显示当前配置")
    p_config.add_argument("--host", default=None, help="设置监听地址")
    p_config.add_argument("--port", type=int, default=None, help="设置端口")
    p_config.add_argument("--workspace", default=None, help="设置工作区")
    p_config.add_argument("--lan", default=None, help="LAN 模式 on/off")
    p_config.set_defaults(func=cmd_config)

    # info
    p_info = sub.add_parser("info", help="显示路径信息")
    p_info.set_defaults(func=cmd_info)

    return parser


def main(argv: Optional[list] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except KeyboardInterrupt:
        warn("已取消")
        return 130
    except Exception as e:  # noqa: BLE001
        err(f"未预期错误: {e}")
        import traceback
        traceback.print_exc()
        return 99


if __name__ == "__main__":
    sys.exit(main())
