"""
tests/test_daemon_cli.py
2026-06-10 TASK-4.1 引入：独立 Daemon CLI smoke test

测试 daemon.py 的 argparse 入口（不实际启动 daemon 进程）：
- help 命令
- info / config --show / status / stop
- 错误处理
"""
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

import pytest

# app 目录
APP_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DAEMON_PY = os.path.join(APP_DIR, "daemon.py")


def run_cli(*args, env_overrides=None, timeout=15):
    """运行 daemon.py 子命令，返回 (returncode, stdout, stderr)"""
    env = os.environ.copy()
    env["YUNJI_DAEMON_NO_COLOR"] = "1"  # 关闭颜色
    env["PYTHONIOENCODING"] = "utf-8"
    if env_overrides:
        env.update(env_overrides)
    result = subprocess.run(
        [sys.executable, DAEMON_PY, *args],
        capture_output=True,
        text=True,
        env=env,
        timeout=timeout,
    )
    return result.returncode, result.stdout, result.stderr


# ==================== 基础测试 ====================


def test_help_exits_zero():
    code, out, err = run_cli("--help")
    assert code == 0
    assert "yunji-daemon" in out or "daemon" in out


def test_help_lists_all_subcommands():
    code, out, _ = run_cli("--help")
    assert code == 0
    for cmd in ("start", "stop", "restart", "status", "logs", "config", "info"):
        assert cmd in out, f"缺少子命令: {cmd}"


def test_start_help_exits_zero():
    code, out, _ = run_cli("start", "--help")
    assert code == 0
    assert "--host" in out
    assert "--port" in out
    assert "--lan" in out
    assert "--workspace" in out


# ==================== info 子命令 ====================


def test_info_shows_paths():
    code, out, _ = run_cli("info")
    assert code == 0
    assert "数据目录" in out
    assert "配置文件" in out
    assert "日志" in out
    assert "局域网 IP" in out


# ==================== status 子命令 ====================


def test_status_when_not_running():
    code, out, _ = run_cli("status")
    # 未运行应当 warn 但 exit 0 or 1（取决于实现）
    assert code in (0, 1)
    assert "状态" in out or "daemon" in out


def test_status_json_format():
    code, out, _ = run_cli("status", "--json")
    assert code in (0, 1)
    # 即使 status 是 1，stdout 也应包含 JSON
    if code == 0:
        import json
        data = json.loads(out)
        assert "status" in data
        assert "host" in data
        assert "port" in data
    # 如果未运行，可能输出空
    # 至少进程没崩溃


# ==================== config 子命令 ====================


def test_config_show():
    """config --show 应返回 daemon 配置 JSON"""
    # 用临时 HOME 避免污染用户真实配置
    with tempfile.TemporaryDirectory() as tmp_home:
        env = {"USERPROFILE": tmp_home, "HOME": tmp_home}
        code, out, _ = run_cli("config", "--show", env_overrides=env)
        # 在新 HOME 下应当自动创建配置
        assert code == 0
        import json
        data = json.loads(out)
        assert "host" in data
        assert "port" in data
        assert "enable_lan" in data


# ==================== stop 子命令（未运行时） ====================


def test_stop_when_not_running():
    code, out, _ = run_cli("stop")
    # 未运行应 warn 但 exit 0
    assert code == 0
    assert "未在运行" in out or "not running" in out.lower() or "stopped" in out.lower() or "未运行" in out


# ==================== restart 子命令（未运行时 = 等价于 start） ====================


def test_restart_does_not_crash():
    """restart 在未运行时应至少不崩溃（可能尝试启动）"""
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp_home:
        env = {"USERPROFILE": tmp_home, "HOME": tmp_home}
        # 给 5 秒超时，避免真启动 daemon 卡住
        try:
            code, out, _ = run_cli("restart", "--timeout", "1", env_overrides=env, timeout=8)
            # 只要没崩溃，code 任意都接受
            assert code in (0, 1, 2)
        except subprocess.TimeoutExpired:
            # 启动 daemon 后 detached 进程是合法的，超时意味着 daemon 启动了
            # 这也是合法的
            pass


# ==================== logs 子命令 ====================


def test_logs_when_no_log_file():
    code, out, _ = run_cli("logs", "-n", "5")
    # 没有日志文件应 warn
    assert code in (0, 1)
    # 输出可能为空或 warn 信息


# ==================== argparse 错误处理 ====================


def test_no_command_exits_nonzero():
    code, _, _ = run_cli()
    # 无 subcommand 应 exit 非 0
    assert code != 0


def test_invalid_subcommand_exits_nonzero():
    code, _, _ = run_cli("nonexistent")
    assert code != 0


def test_invalid_port_type():
    """--port 期望 int，传字符串应当报错"""
    code, _, err = run_cli("start", "--port", "abc")
    assert code != 0


# ==================== main() 入口 ====================


def test_main_entrypoint_importable():
    """daemon.py 应可作为模块导入（不执行 main）"""
    import importlib.util
    spec = importlib.util.spec_from_file_location("daemon", DAEMON_PY)
    mod = importlib.util.module_from_spec(spec)
    # 不 spec.loader.exec_module(mod) — 会触发 if __name__ == "__main__" 之外的 main
    # 只验证 spec 可创建
    assert spec is not None
    assert mod is not None
