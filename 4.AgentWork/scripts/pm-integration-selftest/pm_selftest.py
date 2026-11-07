#!/usr/bin/env python3
"""PM 行为模拟器（v1.50）：只用公开 MCP 接口，验证「正常派单 / 超时 / 取消」三种行为。

用法：
    python scripts/pm-integration-selftest/pm_selftest.py --binary <aw-mcp-bridge 可执行文件>

行为：
  1. 正常派单：写文件 + 打印，校验 status=ok / exit_code=0 / 产物哈希 / receipt.get 回读；
  2. 超时：timeout_secs=1 执行长命令，校验 status=blocked 且 blocked_reason 以 timeout： 开头；
  3. 取消：长任务执行中在同一连接发 task.cancel，校验 canceled + cleanup_claimed + request_state=done，
     且执行侧最终回执也是 canceled。

全部通过退出 0；任一失败退出 1，并打印原因。
依赖：Python 3.7+ 标准库，无需安装任何包。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import uuid

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

IS_WINDOWS = os.name == "nt"
PASSED = 0
FAILED = 0


def check(name: str, condition: bool, detail: str = "") -> None:
    global PASSED, FAILED
    if condition:
        PASSED += 1
        print(f"[PASS] {name}")
    else:
        FAILED += 1
        print(f"[FAIL] {name}" + (f" — {detail}" if detail else ""))


def wait_for(predicate, timeout: float, interval: float = 0.05, description: str = ""):
    deadline = time.time() + timeout
    while time.time() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(interval)
    raise AssertionError(f"等待超时：{description or 'condition'}")


class Bridge:
    """最小 MCP stdio 客户端：按 id 匹配响应，不假设响应顺序。"""

    def __init__(self, exe: str, cwd: str):
        self.proc = subprocess.Popen(
            [exe],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            cwd=cwd,
        )
        self.next_id = 0
        self.pending = {}
        self.lock = threading.Lock()
        self.stderr_lines = []
        threading.Thread(target=self._read_stdout, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()
        result = self.call_method(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "pm-selftest", "version": "1.0.0"},
            },
            timeout=15,
        )
        if result.get("serverInfo", {}).get("name") != "aw-mcp-bridge":
            raise AssertionError(f"initialize 返回异常：{result}")
        self.send_raw({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def _read_stdout(self):
        assert self.proc.stdout is not None
        for line in self.proc.stdout:
            line = line.strip()
            if not line:
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(message, dict) and "id" in message:
                with self.lock:
                    self.pending[message["id"]] = message

    def _read_stderr(self):
        assert self.proc.stderr is not None
        for line in self.proc.stderr:
            self.stderr_lines.append(line.rstrip())

    def send_raw(self, message):
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(message, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()

    def send(self, method: str, params) -> int:
        with self.lock:
            self.next_id += 1
            request_id = self.next_id
        self.send_raw({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        return request_id

    def wait(self, request_id: int, timeout: float):
        return wait_for(
            lambda: self.pending.get(request_id),
            timeout,
            description=f"response id={request_id}",
        )

    def call_method(self, method: str, params, timeout: float = 30):
        request_id = self.send(method, params)
        message = self.wait(request_id, timeout)
        if "error" in message:
            raise AssertionError(f"{method} JSON-RPC 错误：{message['error']}")
        return message.get("result", {})

    def call_tool(self, name: str, arguments, timeout: float = 60):
        result = self.call_method(
            "tools/call", {"name": name, "arguments": arguments}, timeout=timeout
        )
        text = result["content"][0]["text"]
        return json.loads(text), bool(result.get("isError"))

    def close(self):
        try:
            if self.proc.stdin:
                self.proc.stdin.close()
            self.proc.wait(timeout=10)
        except Exception:
            self.proc.kill()


def sha256_of(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def long_command() -> str:
    return "ping -n 30 127.0.0.1 >nul" if IS_WINDOWS else "sleep 30"


def write_command(marker: str) -> str:
    if IS_WINDOWS:
        return f"echo {marker}> pm-selftest-out.txt && echo {marker}"
    return f"printf '%s\\n' {marker} > pm-selftest-out.txt && echo {marker}"


def scenario_normal(bridge: Bridge, runs_dir: str, work_dir: str) -> None:
    task_id = "pm-selftest-normal-" + uuid.uuid4().hex[:8]
    marker = "pm-normal-ok"
    payload, is_error = bridge.call_tool(
        "task.execute",
        {"task_id": task_id, "cmd": write_command(marker), "cwd": work_dir, "timeout_secs": 30},
    )
    check("normal: 调用未报错", not is_error, str(payload))
    check("normal: status=ok", payload.get("status") == "ok", str(payload.get("status")))
    check("normal: exit_code=0", payload.get("exit_code") == 0, str(payload.get("exit_code")))
    artifacts = payload.get("artifacts") or []
    check("normal: 两个产物", len(artifacts) >= 2, str(artifacts))
    stdout_artifact = next((a for a in artifacts if a.get("kind") == "stdout"), None)
    check("normal: 有 stdout 产物", stdout_artifact is not None, str(artifacts))
    if stdout_artifact:
        # locator 形如 runs/<task>/attempt-1/stdout.txt，相对 bridge 可执行文件目录
        stdout_path = os.path.join(
            os.path.dirname(runs_dir), stdout_artifact["locator"].replace("/", os.sep)
        )
        exists = os.path.isfile(stdout_path)
        check("normal: stdout 产物文件存在", exists, stdout_path)
        if exists:
            with open(stdout_path, "r", encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            check("normal: stdout 内容正确", marker in text, text)
            check(
                "normal: stdout 哈希匹配",
                sha256_of(stdout_path) == stdout_artifact.get("hash"),
                f"{sha256_of(stdout_path)} vs {stdout_artifact.get('hash')}",
            )
    handback_path = os.path.join(runs_dir, task_id, "attempt-1", "handback.json")
    check("normal: handback.json 落盘", os.path.isfile(handback_path), handback_path)
    latest, is_error = bridge.call_tool("receipt.get", {"task_id": task_id})
    check(
        "normal: receipt.get 回读一致",
        (not is_error) and latest.get("status") == "ok" and latest.get("task_id") == task_id,
        str(latest),
    )


def scenario_timeout(bridge: Bridge, work_dir: str) -> None:
    task_id = "pm-selftest-timeout-" + uuid.uuid4().hex[:8]
    started = time.time()
    payload, is_error = bridge.call_tool(
        "task.execute",
        {"task_id": task_id, "cmd": long_command(), "cwd": work_dir, "timeout_secs": 1},
        timeout=60,
    )
    elapsed = time.time() - started
    check("timeout: 调用未报错", not is_error, str(payload))
    check("timeout: status=blocked", payload.get("status") == "blocked", str(payload.get("status")))
    reason = payload.get("blocked_reason") or ""
    check("timeout: reason 以 timeout： 开头", reason.startswith("timeout："), reason)
    check("timeout: exit_code 为 null", payload.get("exit_code") is None, str(payload.get("exit_code")))
    check("timeout: 耗时不少于 1s", elapsed >= 1.0, f"{elapsed:.2f}s")
    check(
        "timeout: 回执耗时不少于 900ms",
        (payload.get("duration_ms") or 0) >= 900,
        str(payload.get("duration_ms")),
    )


def scenario_cancel(bridge: Bridge, runs_dir: str, work_dir: str) -> None:
    task_id = "pm-selftest-cancel-" + uuid.uuid4().hex[:8]
    execute_id = bridge.send(
        "tools/call",
        {
            "name": "task.execute",
            "arguments": {
                "task_id": task_id,
                "cmd": long_command(),
                "cwd": work_dir,
                "timeout_secs": 60,
            },
        },
    )
    marker_path = os.path.join(runs_dir, ".inflight", f"{task_id}.json")
    wait_for(lambda: os.path.isfile(marker_path), 15, description="inflight marker")
    with open(marker_path, "r", encoding="utf-8") as handle:
        marker = json.load(handle)
    check("cancel: 活跃标记含 pid/startedAt", bool(marker.get("pid")) and bool(marker.get("startedAt")), str(marker))

    cancel_payload, is_error = bridge.call_tool(
        "task.cancel",
        {
            "task_id": task_id,
            "reason": "pm-selftest 取消",
            "requested_by": "pm-selftest",
        },
        timeout=30,
    )
    check("cancel: 调用未报错", not is_error, str(cancel_payload))
    check("cancel: status=canceled", cancel_payload.get("status") == "canceled", str(cancel_payload.get("status")))
    check("cancel: canceled=true", cancel_payload.get("canceled") is True, str(cancel_payload.get("canceled")))
    check(
        "cancel: cleanup_claimed=true",
        cancel_payload.get("cleanup_claimed") is True,
        str(cancel_payload.get("cleanup_claimed")),
    )
    check(
        "cancel: request_state=done",
        cancel_payload.get("request_state") == "done",
        str(cancel_payload.get("request_state")),
    )
    check(
        "cancel: handback_status=canceled",
        cancel_payload.get("handback_status") == "canceled",
        str(cancel_payload.get("handback_status")),
    )
    check(
        "cancel: requested_by 回显",
        cancel_payload.get("requested_by") == "pm-selftest",
        str(cancel_payload.get("requested_by")),
    )

    execute_message = bridge.wait(execute_id, 30)
    execute_payload = json.loads(execute_message["result"]["content"][0]["text"])
    check(
        "cancel: 执行侧回执 status=canceled",
        execute_payload.get("status") == "canceled",
        str(execute_payload.get("status")),
    )
    reason = execute_payload.get("blocked_reason") or ""
    check("cancel: 回执 reason 以 canceled： 开头", reason.startswith("canceled："), reason)
    check(
        "cancel: 保留了请求方理由",
        "pm-selftest 取消" in reason,
        reason,
    )

    latest, is_error = bridge.call_tool("receipt.get", {"task_id": task_id})
    check(
        "cancel: receipt.get 也是 canceled",
        (not is_error) and latest.get("status") == "canceled",
        str(latest),
    )
    request_file = os.path.join(runs_dir, ".cancel", f"{task_id}.json")
    check("cancel: 取消请求文件落盘", os.path.isfile(request_file), request_file)
    if os.path.isfile(request_file):
        with open(request_file, "r", encoding="utf-8") as handle:
            request = json.load(handle)
        check(
            "cancel: 请求文件状态 done",
            request.get("status") == "done",
            str(request.get("status")),
        )
        target = request.get("target") or {}
        check(
            "cancel: target 指向本轮执行",
            target.get("pid") == marker.get("pid") and target.get("startedAt") == marker.get("startedAt"),
            f"{target} vs {marker}",
        )
    wait_for(lambda: not os.path.exists(marker_path), 10, description="inflight marker 清理")
    check("cancel: 活跃标记已删除", not os.path.exists(marker_path), marker_path)


def main() -> int:
    parser = argparse.ArgumentParser(description="PM 行为模拟器：正常派单 / 超时 / 取消")
    parser.add_argument("--binary", required=True, help="aw-mcp-bridge 可执行文件路径")
    parser.add_argument("--keep", action="store_true", help="保留临时目录，便于排查")
    args = parser.parse_args()

    binary = os.path.abspath(args.binary)
    if not os.path.isfile(binary):
        print(f"[FAIL] 找不到二进制：{binary}")
        return 1

    temp_root = tempfile.mkdtemp(prefix="aw-pm-selftest-")
    work_dir = os.path.join(temp_root, "work")
    os.makedirs(work_dir, exist_ok=True)
    # 复制到临时目录：runs/ 跟随可执行文件，避免污染仓库 target 目录
    local_exe = os.path.join(temp_root, os.path.basename(binary))
    shutil.copy2(binary, local_exe)
    runs_dir = os.path.join(temp_root, "runs")

    bridge = None
    try:
        bridge = Bridge(local_exe, work_dir)
        scenario_normal(bridge, runs_dir, work_dir)
        scenario_timeout(bridge, work_dir)
        scenario_cancel(bridge, runs_dir, work_dir)
    except Exception as error:  # noqa: BLE001 — 自测器需要把任何异常都转成失败结论
        check("未捕获异常", False, repr(error))
        if bridge is not None:
            print("[INFO] bridge stderr（尾 20 行）：")
            for line in bridge.stderr_lines[-20:]:
                print("    " + line)
    finally:
        if bridge is not None:
            bridge.close()
        if args.keep:
            print(f"[INFO] 临时目录保留：{temp_root}")
        else:
            shutil.rmtree(temp_root, ignore_errors=True)

    print(f"[SELFTEST] pass={PASSED} fail={FAILED}")
    return 0 if FAILED == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
