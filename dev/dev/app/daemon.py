#!/usr/bin/env python3
"""
云集智能编程工作站 - 独立 Daemon 模式 CLI

2026-06-09 TASK-4.1 引入：Phase 4 W13 远程 Daemon 模式

设计目标：
    后端可独立运行（不依赖桌面端），方便远程访问。
    提供 CLI 控制：start / stop / status / restart / info / logs

使用示例：
    # 启动（默认 127.0.0.1:18080）
    python dev/app/daemon.py start

    # 启动 LAN 模式（自动生成配对码）
    python dev/app/daemon.py start --lan --port 18080

    # 状态查询
    python dev/app/daemon.py status

    # 停止
    python dev/app/daemon.py stop

    # 重启
    python dev/app/daemon.py restart

    # 查看最近日志
    python dev/app/daemon.py logs --tail 50

    # 完整信息
    python dev/app/daemon.py info
"""

import argparse
import json
import os
import sys
import time
from pathlib import Path

# 让 daemon.py 可作为脚本直接执行
_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from platformkit.shared.daemon_core import (
    DaemonConfig,
    DaemonManager,
    DaemonStatus,
    default_daemon_dir,
    format_uptime,
    get_local_ip,
)


# ──────────── 子命令处理 ────────────


def cmd_start(args) -> int:
    """启动 daemon"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    daemon_dir.mkdir(parents=True, exist_ok=True)
    mgr = DaemonManager(daemon_dir=daemon_dir)

    if mgr.is_running():
        cur = mgr.status()
        print(f"❌ Daemon 已经在运行: pid={cur.pid}, port={cur.port}")
        return 1

    cfg = DaemonConfig(
        host=args.host,
        port=args.port,
        enable_lan=args.lan,
        workspace=args.workspace,
        daemon_dir=str(daemon_dir),
    )

    try:
        # 重建 manager 用新配置
        mgr = DaemonManager(config=cfg, daemon_dir=daemon_dir)
        state = mgr.start()
    except RuntimeError as e:
        print(f"❌ 启动失败: {e}")
        return 1

    # 打印启动信息
    print("=" * 60)
    print(f"✅ 云集 Daemon 启动成功")
    print(f"   PID:        {state.pid}")
    print(f"   状态:       {state.status.value}")
    print(f"   监听:       http://{state.host}:{state.port}")
    print(f"   工作区:     {state.workspace or '(默认)'}")
    print(f"   数据目录:   {daemon_dir}")
    print(f"   日志:       {cfg.log_file}")
    if state.token:
        ip = get_local_ip()
        print()
        print(f"   🔑 配对码:    {state.token}")
        print(f"   📱 手机访问:  http://{ip}:{state.port}?token={state.token}")
    print()
    print(f"   API 文档:   http://{state.host}:{state.port}/docs")
    print("=" * 60)
    return 0


def cmd_stop(args) -> int:
    """停止 daemon"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    mgr = DaemonManager(daemon_dir=daemon_dir)
    if not mgr.is_running():
        print("ℹ️  Daemon 未运行")
        return 0
    try:
        state = mgr.stop(timeout=args.timeout)
    except RuntimeError as e:
        print(f"❌ 停止失败: {e}")
        return 1
    print(f"✅ Daemon 已停止 (was pid={state.pid or '?'})")
    return 0


def cmd_status(args) -> int:
    """查询 daemon 状态"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    mgr = DaemonManager(daemon_dir=daemon_dir)
    state = mgr.status()
    print("=" * 60)
    print(f"📊 云集 Daemon 状态")
    print(f"   状态:       {state.status.value}")
    print(f"   PID:        {state.pid or '—'}")
    print(f"   监听:       http://{state.host}:{state.port}")
    print(f"   工作区:     {state.workspace or '(默认)'}")
    if state.status == DaemonStatus.RUNNING and state.uptime_seconds is not None:
        print(f"   运行时长:   {format_uptime(state.uptime_seconds)}")
    if state.token:
        print(f"   配对码:     {state.token}")
    if state.last_health:
        print(f"   上次健康:   {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(state.last_health))}")
    if state.version:
        print(f"   版本:       {state.version}")
    # 健康检查
    if state.status == DaemonStatus.RUNNING:
        healthy = mgr.health_check()
        print(f"   健康检查:   {'✅ 通过' if healthy else '⚠️  HTTP 不可达'}")
    print("=" * 60)
    if args.json:
        # JSON 输出（适合脚本调用）
        print()
        print(json.dumps(state.to_dict(), ensure_ascii=False, indent=2))
    return 0


def cmd_restart(args) -> int:
    """重启 daemon"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    mgr = DaemonManager(daemon_dir=daemon_dir)
    if mgr.is_running():
        try:
            mgr.stop(timeout=args.timeout)
        except RuntimeError as e:
            print(f"❌ 停止失败: {e}")
            return 1
        time.sleep(0.5)
    # 复用 start 参数
    return cmd_start(args)


def cmd_info(args) -> int:
    """完整信息"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    mgr = DaemonManager(daemon_dir=daemon_dir)
    info = mgr.info()
    print(json.dumps(info, ensure_ascii=False, indent=2))
    return 0


def cmd_logs(args) -> int:
    """查看日志"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    log_file = daemon_dir / "daemon.log"
    if not log_file.exists():
        print(f"ℹ️  日志文件不存在: {log_file}")
        return 0
    # 读最后 N 行
    with open(log_file, "r", encoding="utf-8", errors="ignore") as f:
        lines = f.readlines()
    tail = lines[-args.tail:] if args.tail > 0 else lines
    print(f"📋 日志 ({log_file}, 最后 {len(tail)} 行):")
    print("-" * 60)
    for line in tail:
        print(line.rstrip())
    print("-" * 60)
    return 0


def cmd_clean(args) -> int:
    """清理 daemon 数据（PID/状态/日志）"""
    daemon_dir = Path(args.dir) if args.dir else default_daemon_dir()
    if not daemon_dir.exists():
        print(f"ℹ️  Daemon 目录不存在: {daemon_dir}")
        return 0
    # 警告：会停止 daemon
    mgr = DaemonManager(daemon_dir=daemon_dir)
    if mgr.is_running():
        print("❌ Daemon 正在运行，请先 stop")
        return 1
    if not args.yes:
        confirm = input(f"确认清理 {daemon_dir}? [y/N] ")
        if confirm.lower() != "y":
            print("已取消")
            return 0
    # 清理文件
    for fname in ("daemon.pid", "daemon.json", "daemon.config.json", "daemon.log"):
        f = daemon_dir / fname
        if f.exists():
            f.unlink()
            print(f"  删除: {f}")
    print(f"✅ 清理完成: {daemon_dir}")
    return 0


# ──────────── CLI 入口 ────────────


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="云集智能编程工作站 - Daemon 模式 CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s start                 # 启动 daemon
  %(prog)s start --lan --port 18080  # LAN 模式
  %(prog)s status                # 查看状态
  %(prog)s stop                  # 停止
  %(prog)s restart               # 重启
  %(prog)s logs --tail 50        # 查看日志
""",
    )
    parser.add_argument(
        "--dir",
        default=None,
        help="Daemon 数据目录（默认 ~/.yunji/daemon/）",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # start
    p_start = sub.add_parser("start", help="启动 daemon")
    p_start.add_argument("--host", default="127.0.0.1", help="监听地址（默认 127.0.0.1）")
    p_start.add_argument("--port", type=int, default=18080, help="监听端口（默认 18080）")
    p_start.add_argument("--lan", action="store_true", help="启用 LAN 模式（自动生成配对码）")
    p_start.add_argument("--workspace", default=None, help="工作区路径")
    p_start.set_defaults(func=cmd_start)

    # stop
    p_stop = sub.add_parser("stop", help="停止 daemon")
    p_stop.add_argument("--timeout", type=float, default=5.0, help="停止超时（秒）")
    p_stop.set_defaults(func=cmd_stop)

    # status
    p_status = sub.add_parser("status", help="查询状态")
    p_status.add_argument("--json", action="store_true", help="JSON 输出")
    p_status.set_defaults(func=cmd_status)

    # restart
    p_restart = sub.add_parser("restart", help="重启 daemon")
    p_restart.add_argument("--host", default="127.0.0.1", help="监听地址")
    p_restart.add_argument("--port", type=int, default=18080, help="监听端口")
    p_restart.add_argument("--lan", action="store_true", help="启用 LAN 模式")
    p_restart.add_argument("--workspace", default=None, help="工作区路径")
    p_restart.add_argument("--timeout", type=float, default=5.0, help="停止超时（秒）")
    p_restart.set_defaults(func=cmd_restart)

    # info
    p_info = sub.add_parser("info", help="完整信息（JSON）")
    p_info.set_defaults(func=cmd_info)

    # logs
    p_logs = sub.add_parser("logs", help="查看日志")
    p_logs.add_argument("--tail", type=int, default=50, help="显示最后 N 行（默认 50）")
    p_logs.set_defaults(func=cmd_logs)

    # clean
    p_clean = sub.add_parser("clean", help="清理 daemon 数据")
    p_clean.add_argument("-y", "--yes", action="store_true", help="跳过确认")
    p_clean.set_defaults(func=cmd_clean)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.func(args)
    except KeyboardInterrupt:
        print("\n⚠️  已中断")
        return 130
    except Exception as e:
        print(f"❌ 错误: {e}")
        if os.environ.get("YUNJI_DAEMON_DEBUG"):
            import traceback
            traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
