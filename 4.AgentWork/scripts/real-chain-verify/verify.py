#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""真实链路端到端验证（MCP over stdio，不调用任何内部函数）。

全部经 `aw-mcp-bridge` 的真实 stdio JSON-RPC 路径：

  A. 正常任务：真实产出文件 -> attempt / inflight / dispatch / handback / 产物互相对得上
  B1. 执行中超时：终止受控集合，心跳停止，回执 blocked + timeout
  B2. bridge 猝死：进程被杀后子进程是否被连带收掉（Windows 作业对象 / Linux PDEATHSIG）
      然后重启服务，验证 startup 档自动清扫残留标记
  B3. bridge 猝死后 5s 容差内立刻重启：不误删活标
  B4. 显式取消：同一连接上 task.execute 执行中调用 task.cancel，状态必须是 canceled
  B5. bridge 猝死遗留 cgroup 作用域：下一次启动自动回收，孙进程消失（仅 Linux）
  B6. 跨进程取消：另一个进程写磁盘请求，执行侧心跳读取，三态（pending/acknowledged/done）可区分
  C1/C2/C3. 并发：同 task_id 顺序/两进程、不同 task_id 并行
  D1. 周期清扫：bridge 猝死留下的残标由 period 档（trigger=periodic）清掉
  D2. 逃逸进程整树清理（仅 Linux）：祖父/父/孙 + 孙进程逃出进程组与 cgroup.procs，超时后全灭
  D3. `_session/initialize-*.json` 按天保留：startup 档删除过期记录并留审计

用法：
  python scripts/real-chain-verify/verify.py --binary <aw-mcp-bridge[.exe]> [--only A,B1,...] [--keep]

退出码：0 全部通过；1 有断言失败；2 环境/参数问题。
"""

import argparse
import hashlib
import json
import os
import queue
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

IS_WINDOWS = os.name == "nt"
BIN_NAME = "aw-mcp-bridge.exe" if IS_WINDOWS else "aw-mcp-bridge"

FAILURES = []
EVIDENCE = []


def ev(msg):
    line = "[evidence] " + msg
    print(line, flush=True)
    EVIDENCE.append(line)


def check(cond, msg):
    if cond:
        print("[ok] " + msg, flush=True)
        return True
    print("[FAIL] " + msg, flush=True)
    FAILURES.append(msg)
    return False


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def normalize_path(text):
    r"""Windows canonicalize 会带 \\?\ 前缀；比对时按路径语义归一。"""
    value = str(text).replace("\\", "/")
    if value.startswith("//?/"):
        value = value[4:]
    return value.lower() if IS_WINDOWS else value


def read_json(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def read_jsonl(path):
    if not Path(path).is_file():
        return []
    out = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def wait_until(fn, timeout, interval=0.2):
    deadline = time.time() + timeout
    while time.time() < deadline:
        value = fn()
        if value:
            return value
        time.sleep(interval)
    return None


def find_tmp(root):
    root = Path(root)
    if not root.exists():
        return []
    return sorted(str(p) for p in root.rglob("*.tmp"))


class Bridge:
    """一条真实的 stdio 连接；所有交互都落到 wire.jsonl。"""

    def __init__(self, exe, sandbox, name="bridge", sweep="0", extra_env=None, wire_name="wire.jsonl"):
        self.name = name
        self.exe = Path(exe)
        self.sandbox = Path(sandbox)
        self.work = self.sandbox / "work"
        self.work.mkdir(parents=True, exist_ok=True)
        self.wire_path = self.sandbox / wire_name
        self.pending = {}
        env = dict(os.environ)
        env["APPDATA"] = str(self.sandbox / "appdata")
        env["HOME"] = str(self.sandbox / "appdata")
        # 0 = 只做启动清扫，避免周期线程把证据弄成时序不确定
        env["AW_INFLIGHT_SWEEP_SECS"] = sweep
        if extra_env:
            env.update({str(k): str(v) for k, v in extra_env.items()})
        (self.sandbox / "appdata").mkdir(parents=True, exist_ok=True)
        self.proc = subprocess.Popen(
            [str(self.exe)],
            cwd=str(self.work),
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
        )
        self.queue = queue.Queue()
        self.stderr_lines = []
        self.next_id = 1
        threading.Thread(target=self._pump_stdout, daemon=True).start()
        threading.Thread(target=self._pump_stderr, daemon=True).start()

    def _pump_stdout(self):
        assert self.proc.stdout is not None
        for line in self.proc.stdout:
            self.queue.put(line)

    def _pump_stderr(self):
        assert self.proc.stderr is not None
        for line in self.proc.stderr:
            self.stderr_lines.append(line.rstrip("\n"))

    def _record(self, direction, payload):
        with open(self.wire_path, "a", encoding="utf-8") as handle:
            handle.write(json.dumps({"dir": direction, "payload": payload}, ensure_ascii=False) + "\n")

    def send(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        self._record("send", request)
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()
        return request_id

    def await_id(self, request_id, timeout=180):
        # task.execute 现在在独立线程执行，cancel 的响应可能先到；JSON-RPC 允许按 id 乱序，
        # 这里只做暂存，不改变"每个 id 必须收到且只收到一次"的判据。
        if request_id in self.pending:
            return self.pending.pop(request_id)
        deadline = time.time() + timeout
        while True:
            remaining = deadline - time.time()
            if remaining <= 0:
                raise RuntimeError(f"{self.name}: 等待响应 id={request_id} 超时")
            line = self.queue.get(timeout=remaining)
            response = json.loads(line)
            self._record("recv", response)
            if response.get("id") == request_id:
                return response
            other = response.get("id")
            if other is None:
                raise RuntimeError(f"{self.name}: 收到无 id 的响应：{response}")
            self.pending[other] = response

    def request(self, method, params, timeout=180):
        return self.await_id(self.send(method, params), timeout=timeout)

    def initialize(self):
        response = self.request(
            "initialize", {"clientInfo": {"name": "real-chain-verify"}, "protocolVersion": "2025-06-18"}
        )
        return response

    def call(self, tool, arguments, timeout=180):
        response = self.request("tools/call", {"name": tool, "arguments": arguments}, timeout=timeout)
        result = response.get("result")
        if result is None:
            return None, True, response
        payload = json.loads(result["content"][0]["text"])
        return payload, bool(result.get("isError")), response

    def kill(self):
        if self.proc.poll() is None:
            self.proc.kill()
        try:
            self.proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            pass

    def close(self):
        try:
            if self.proc.stdin is not None:
                self.proc.stdin.close()
        except Exception:
            pass
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait(timeout=10)


# ---- 平台命令 -------------------------------------------------------------

def cmd_write(path, content):
    if IS_WINDOWS:
        return f"echo {content}>{path}"
    return f"printf '%s\\n' '{content}' > {path}"


def cmd_heartbeat(path, ticks):
    """每秒追加一行，用于观察 bridge 死亡后子进程是否仍在写（持续时长由 ticks 自封顶）。"""
    if IS_WINDOWS:
        return f"for /l %i in (1,1,{ticks}) do @(echo tick>>{path}&ping -n 2 127.0.0.1>nul)"
    return f"i=0; while [ $i -lt {ticks} ]; do echo tick >> {path}; sleep 1; i=$((i+1)); done"


def cmd_slow_append(path, delay_s):
    if IS_WINDOWS:
        return f"ping -n {delay_s + 1} 127.0.0.1>nul&echo run>>{path}"
    return f"sleep {delay_s}; echo run >> {path}"


def make_sandbox(root, label, scenario):
    base = Path(root) if root else Path(tempfile.gettempdir())
    base.mkdir(parents=True, exist_ok=True)
    sandbox = Path(tempfile.mkdtemp(prefix=f"aw-real-{label.lower()}-{scenario.lower()}-", dir=str(base)))
    (sandbox / "bin").mkdir()
    return sandbox


def install_binary(exe, sandbox):
    target = sandbox / "bin" / BIN_NAME
    shutil.copy2(str(exe), str(target))
    return target


def runs_dir_of(sandbox):
    return sandbox / "bin" / "runs"


# ---- 场景 A：正常任务 -----------------------------------------------------

def scenario_a(exe, root, label):
    print("\n== 场景 A：真实小任务全链路 ==", flush=True)
    sandbox = make_sandbox(root, label, "A")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="A")
    task_id = "real-a-normal"
    artifact = "artifact-a.txt"
    try:
        bridge.initialize()
        listing = bridge.request("tools/list", {})
        names = sorted(tool.get("name") for tool in listing.get("result", {}).get("tools", []))
        check("task.execute" in names, f"A tools/list 暴露 task.execute：{names}")
        check("task.cancel" in names, f"A tools/list 暴露显式取消 task.cancel：{names}")
        ev(f"A tool_names={names} cancel_tool=true")
        payload, is_error, _ = bridge.call(
            "task.execute",
            {
                "task_id": task_id,
                "cmd": cmd_write(artifact, "hello-real-chain-A"),
                "cwd": str(work),
                "timeout_secs": 30,
            },
        )
        check(not is_error, "A task.execute 不是工具错误")
        check(payload.get("status") == "ok", f"A status=ok（实际 {payload.get('status')}）")
        check(payload.get("exit_code") == 0, f"A exit_code=0（实际 {payload.get('exit_code')}）")
        check(payload.get("attempt") == 1, "A attempt=1")
        produced = work / artifact
        check(produced.is_file(), f"A 产物真实存在：{artifact}")
        content = produced.read_text(encoding="utf-8", errors="replace").strip() if produced.is_file() else ""
        check(content == "hello-real-chain-A", f"A 产物内容正确（{content!r}）")

        attempt_dir = runs / task_id / "attempt-1"
        expected = {"dispatch.json", "stdout.txt", "stderr.txt", "handback.json"}
        actual = {p.name for p in attempt_dir.iterdir()} if attempt_dir.is_dir() else set()
        check(expected <= actual, f"A attempt-1 关键文件齐全：{sorted(actual)}")
        extra_tmp = [p for p in actual if p.endswith(".tmp")]
        check(not extra_tmp, f"A attempt-1 无 .tmp：{extra_tmp}")

        dispatch = read_json(attempt_dir / "dispatch.json")
        handback = read_json(attempt_dir / "handback.json")
        check(dispatch.get("task_id") == task_id, "A dispatch.task_id 与请求一致")
        check(dispatch.get("timeout_secs") == 30, "A dispatch.timeout_secs 与请求一致")
        check(
            normalize_path(dispatch.get("cwd", "")) == normalize_path(os.path.realpath(work)),
            f"A dispatch.cwd 是规范化绝对路径：{dispatch.get('cwd')}",
        )
        check(bool(dispatch.get("dispatchedAt")), "A dispatch.dispatchedAt 存在")
        check(bool(handback.get("finishedAt")), "A handback.finishedAt 存在")

        stdout_digest = sha256_file(attempt_dir / "stdout.txt")
        stderr_digest = sha256_file(attempt_dir / "stderr.txt")
        artifacts = handback.get("artifacts", [])
        by_kind = {item.get("kind"): item for item in artifacts}
        check(by_kind.get("stdout", {}).get("hash") == stdout_digest, "A artifacts[stdout].hash == 盘上 sha256")
        check(by_kind.get("stderr", {}).get("hash") == stderr_digest, "A artifacts[stderr].hash == 盘上 sha256")
        check(by_kind.get("stdout", {}).get("locator") == f"runs/{task_id}/attempt-1/stdout.txt", "A locator 指向真实路径")
        check(
            (runs.parent / by_kind.get("stdout", {}).get("locator", "")).is_file(),
            "A locator 从 runs 的父目录可解析到文件",
        )

        inflight_list, _, _ = bridge.call("runs.inflight", {"action": "list"})
        check(inflight_list.get("total") == 0, f"A 正常结束后 inflight 无残留（total={inflight_list.get('total')}）")

        receipt, receipt_error, _ = bridge.call("receipt.get", {"task_id": task_id})
        check(not receipt_error, "A receipt.get 可读")
        check(receipt.get("finishedAt") == handback.get("finishedAt"), "A receipt.get 与 handback 同一条")
        check(receipt.get("status") == "ok", "A receipt.get status=ok")

        disk_attempts = sorted(p.name for p in (runs / task_id).iterdir() if p.name.startswith("attempt-"))
        check(disk_attempts == ["attempt-1"], f"A 只有一个 attempt 目录：{disk_attempts}")
        ev(
            f"A label={label} status=ok attempt=1 inflight_total={inflight_list.get('total')} "
            f"tmp_files={len(find_tmp(runs))} artifacts_match=true"
        )
    finally:
        bridge.close()
    return sandbox


# ---- 场景 B1：执行中超时 --------------------------------------------------

def scenario_b_timeout(exe, root, label):
    print("\n== 场景 B1：执行中超时 ==", flush=True)
    sandbox = make_sandbox(root, label, "B1")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="B1")
    task_id = "real-b-timeout"
    heartbeat = "heartbeat-b.txt"
    try:
        bridge.initialize()
        started = time.time()
        payload, is_error, _ = bridge.call(
            "task.execute",
            {
                "task_id": task_id,
                "cmd": cmd_heartbeat(heartbeat, 30),
                "cwd": str(work),
                "timeout_secs": 2,
            },
            timeout=120,
        )
        elapsed = time.time() - started
        check(not is_error, "B1 超时是正常回执（不是工具错误）")
        check(payload.get("status") == "blocked", f"B1 status=blocked（实际 {payload.get('status')}）")
        reason = payload.get("blocked_reason") or ""
        check(reason.startswith("timeout："), f"B1 blocked_reason 是 timeout 档：{reason[:60]}...")
        check(1.8 <= elapsed <= 20, f"B1 超时在 timeout_secs 附近生效（{elapsed:.2f}s）")

        attempt_dir = runs / task_id / "attempt-1"
        check((attempt_dir / "handback.json").is_file(), "B1 超时也落盘 handback.json")
        handback = read_json(attempt_dir / "handback.json")
        check(handback.get("status") == "blocked", "B1 盘上 handback.status=blocked")
        check(handback.get("blocked_reason") == reason, "B1 盘上回执与响应同一条")

        beat = work / heartbeat
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(3)
        size_after = beat.stat().st_size if beat.is_file() else 0
        check(size_after == size_before, f"B1 超时清理后心跳不再增长（{size_before} -> {size_after}）")

        inflight, _, _ = bridge.call("runs.inflight", {"action": "list"})
        check(inflight.get("total") == 0, "B1 超时返回后无残留标记")
        ev(
            f"B1 label={label} status=blocked elapsed={elapsed:.2f}s controlled_set_silent=true "
            f"inflight_total={inflight.get('total')} tmp_files={len(find_tmp(runs))}"
        )
    finally:
        bridge.close()
    return sandbox


# ---- 场景 B2/B3：执行中取消 + 重启 startup 清扫 ---------------------------

def scenario_b_cancel(exe, root, label):
    print("\n== 场景 B2：执行中取消（bridge 进程死亡）+ 重启 startup 清扫 ==", flush=True)
    sandbox = make_sandbox(root, label, "B2")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    task_id = "real-b-cancel"
    heartbeat = "heartbeat-c.txt"

    bridge = Bridge(binary, sandbox, name="B2-first")
    try:
        bridge.initialize()
        # 先完成一个短任务：bridge 猝死不得误伤已完成任务的产物与回执
        done_task = "real-b-completed"
        done_artifact = "artifact-b-completed.txt"
        done, done_error, _ = bridge.call(
            "task.execute",
            {
                "task_id": done_task,
                "cmd": cmd_write(done_artifact, "completed-before-kill"),
                "cwd": str(work),
                "timeout_secs": 30,
            },
        )
        check(not done_error and done.get("status") == "ok", "B2 先完成一个短任务（ok）")
        done_handback = runs / done_task / "attempt-1" / "handback.json"
        done_bytes = done_handback.read_bytes() if done_handback.is_file() else b""
        request_id = bridge.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd_heartbeat(heartbeat, 40),
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        check(request_id > 0, "B2 已发出 task.execute（尚未收到响应即杀进程）")
        marker = runs / ".inflight" / f"{task_id}.json"
        found = wait_until(lambda: marker.is_file(), 10)
        check(bool(found), f"B2 执行中 inflight 标记存在：runs/.inflight/{task_id}.json")
        started_at = read_json(marker).get("startedAt") if found else None
        attempt_dir = runs / task_id / "attempt-1"
        check(attempt_dir.is_dir(), "B2 执行中 attempt-1 已占号")
        time.sleep(3)
        bridge.kill()
        check(bridge.proc.poll() is not None, "B2 bridge 已被杀死（模拟非正常退出）")

        beat = work / heartbeat
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(3)
        size_after = beat.stat().st_size if beat.is_file() else 0
        # Windows：Job Object KILL_ON_JOB_CLOSE；Linux：v1.47 起 pre_exec 设 PR_SET_PDEATHSIG，
        # 只保证直接子进程；孙进程（fork 后再 exec）不受 PDEATHSIG 约束，见 D2/契约 §2.7。
        ev(
            f"B2 label={label} killed_at=~3s child_heartbeat_before={size_before} after={size_after} "
            f"child_survived={size_after > size_before}"
        )
        check(
            size_after == size_before,
            f"B2 bridge 猝死后直接子进程在可观测时间内消失（心跳 {size_before} -> {size_after}）",
        )
        check(marker.is_file(), "B2 进程被杀死后残留标记仍在（没有机会执行 Drop）")
        check(
            (attempt_dir / "dispatch.json").is_file()
            and not (attempt_dir / "handback.json").is_file(),
            "B2 取消后 attempt-1 只有 dispatch、无 handback（不宣称完成）",
        )
        check(
            (work / done_artifact).is_file()
            and (work / done_artifact).read_text(encoding="utf-8", errors="replace").strip()
            == "completed-before-kill",
            "B2 已完成的短任务产物未被误杀",
        )
        check(
            done_handback.is_file() and done_handback.read_bytes() == done_bytes,
            "B2 已完成任务的回执字节未变",
        )

        # 5s 容差：startedAt + 5s 之后才判 stale
        wait_until(
            lambda: time.time() - time.mktime(time.strptime(started_at[:19], "%Y-%m-%dT%H:%M:%S")) > 7
            if started_at
            else True,
            15,
        )
        restarted = Bridge(binary, sandbox, name="B2-restarted")
        try:
            restarted.initialize()
            sweep_lines = read_jsonl(runs / "inflight-sweep.jsonl")
            startup = [row for row in sweep_lines if row.get("trigger") == "startup"]
            check(bool(startup), "B2 重启后审计出现 trigger=startup 记录")
            removed_here = startup[-1].get("removed_total") if startup else None
            check(removed_here == 1, f"B2 startup 清扫删除 1 条（实际 {removed_here}）")
            deleted_ids = [
                item.get("task_id")
                for row in startup
                for item in row.get("removed", [])
                if item.get("task_id")
            ]
            check(task_id in deleted_ids, f"B2 startup 审计点名了该 task：{deleted_ids}")
            check(not marker.is_file(), "B2 残留标记已被删除")

            listed, _, _ = restarted.call("runs.inflight", {"action": "list"})
            check(listed.get("total") == 0, f"B2 重启后 inflight 干净（total={listed.get('total')}）")

            receipt, receipt_error, receipt_raw = restarted.call("receipt.get", {"task_id": task_id})
            check(receipt_error, "B2 取消的 task 没有回执（不会被误报成 completed）")
            check(
                isinstance(receipt, dict) and "回执不存在" in json.dumps(receipt, ensure_ascii=False),
                f"B2 receipt.get 明确说回执不存在：{receipt}",
            )
            check(
                (runs / task_id / "attempt-1").is_dir(),
                "B2 attempt-1 目录留在原位（inflight 清扫只删标记，不动 attempt）",
            )
            check(not (runs / ".prune-trash").exists(), "B2 没有 .prune-trash（未走 prune）")
            ev(
                f"B2 label={label} startup_removed={removed_here} marker_gone=true "
                f"attempt_kept=true receipt_exists=false tmp_files={len(find_tmp(runs))}"
            )
        finally:
            restarted.close()
    finally:
        bridge.close()
    return sandbox


def scenario_b_grace(exe, root, label):
    print("\n== 场景 B3：取消后立刻重启（5s 容差内的真实行为）==", flush=True)
    sandbox = make_sandbox(root, label, "B3")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    task_id = "real-b-cancel-grace"

    bridge = Bridge(binary, sandbox, name="B3-first")
    try:
        bridge.initialize()
        bridge.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd_heartbeat("heartbeat-d.txt", 30),
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        marker = runs / ".inflight" / f"{task_id}.json"
        found = wait_until(lambda: marker.is_file(), 10)
        check(bool(found), "B3 执行中标记存在")
        started = time.time()
        time.sleep(2)
        bridge.kill()
        started_at = read_json(marker).get("startedAt") if marker.is_file() else None

        restarted = Bridge(binary, sandbox, name="B3-early")
        try:
            restarted.initialize()
            listed, _, _ = restarted.call("runs.inflight", {"action": "list"})
            markers = listed.get("markers", [])
            stale = [m for m in markers if m.get("task_id") == task_id]
            check(bool(stale), "B3 立刻重启时标记仍在（5s 容差内不误删）")
            if stale:
                check(stale[0].get("stale") is False, "B3 容差内 stale=false")
                ev(
                    f"B3 label={label} early_restart kept=true stale={stale[0].get('stale')} "
                    f"reason={stale[0].get('stale_reason')} pid_alive={stale[0].get('pid_alive')}"
                )
            sweep_lines = [
                row
                for row in read_jsonl(runs / "inflight-sweep.jsonl")
                if row.get("trigger") == "startup"
            ]
            # 契约 §2.9：无可清残标时不删除、不追加审计。这里应看到“零审计行”，
            # 同时标记仍在 —— 两者共同证明容差内没有被误删。
            check(
                all(row.get("removed_total") == 0 for row in sweep_lines),
                "B3 早重启的 startup 零删除（容差内不删活标）",
            )
            ev(f"B3 label={label} early_restart_audit_rows={len(sweep_lines)} kept=true")
        finally:
            restarted.close()

        # 等到 startedAt + 5s 之后，再重启一次
        while time.time() - started < 7:
            time.sleep(0.3)
        late = Bridge(binary, sandbox, name="B3-late")
        try:
            late.initialize()
            sweep_lines = [
                row
                for row in read_jsonl(runs / "inflight-sweep.jsonl")
                if row.get("trigger") == "startup"
            ]
            removed = sum(row.get("removed_total", 0) for row in sweep_lines)
            check(removed >= 1, f"B3 容差过后 startup 清扫删除（累计 removed_total={removed}）")
            check(not marker.is_file(), "B3 容差过后标记已删除")
            ev(f"B3 label={label} late_restart removed_total={removed} marker_gone=true")
        finally:
            late.close()
    finally:
        bridge.close()
    return sandbox


# ---- 场景 B4：显式取消（真实 MCP 同连接） ---------------------------------

def scenario_b4_explicit_cancel(exe, root, label):
    print("\n== 场景 B4：执行中显式 task.cancel（同一连接）==", flush=True)
    sandbox = make_sandbox(root, label, "B4")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="B4")
    task_id = "real-b4-cancel"
    heartbeat = "heartbeat-b4.txt"
    try:
        bridge.initialize()
        execute_id = bridge.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd_heartbeat(heartbeat, 60),
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        marker = runs / ".inflight" / f"{task_id}.json"
        dispatch = runs / task_id / "attempt-1" / "dispatch.json"
        check(bool(wait_until(lambda: marker.is_file() and dispatch.is_file(), 15)), "B4 执行已开始")
        time.sleep(1)

        canceled, canceled_error, _ = bridge.call(
            "task.cancel", {"task_id": task_id, "reason": "真实链路验收取消"}
        )
        check(not canceled_error, "B4 task.cancel 不是工具错误")
        check(canceled.get("status") == "canceled", f"B4 cancel status=canceled（实际 {canceled.get('status')}）")
        check(canceled.get("cleanup_claimed") is True, "B4 回执已落盘为 canceled 才宣称清理")
        check(canceled.get("handback_status") == "canceled", f"B4 cancel 观测到 canceled 回执：{canceled.get('handback_status')}")

        response = bridge.await_id(execute_id, timeout=60)
        result = response.get("result", {})
        payload = json.loads(result["content"][0]["text"])
        check(payload.get("status") == "canceled", f"B4 task.execute 状态是 canceled（实际 {payload.get('status')}）")
        reason = payload.get("blocked_reason") or ""
        check(reason.startswith("canceled："), f"B4 blocked_reason 前缀是 canceled：{reason[:50]}...")
        check("请求方理由：真实链路验收取消" in reason, "B4 请求方理由被写进回执")
        check(not reason.startswith("timeout："), "B4 取消没有被误记成超时")

        beat = work / heartbeat
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(2)
        size_after = beat.stat().st_size if beat.is_file() else 0
        check(size_after == size_before, f"B4 取消后心跳停止（{size_before} -> {size_after}）")
        check(not marker.is_file(), "B4 取消后活跃标记已清理")
        handback = read_json(runs / task_id / "attempt-1" / "handback.json")
        check(handback.get("status") == "canceled", "B4 盘上 handback.status=canceled")

        ghost, ghost_error, _ = bridge.call("task.cancel", {"task_id": "real-b4-ghost"})
        check(not ghost_error and ghost.get("status") == "not_running", f"B4 看不到的执行返回 not_running：{ghost}")
        check(ghost.get("cleanup_claimed") is False, "B4 not_running 不宣称已清理")

        pruned, prune_error, _ = bridge.call(
            "runs.prune", {"older_than_days": 1, "status": ["canceled"], "dry_run": True}
        )
        check(not prune_error, f"B4 runs.prune 接受 canceled 状态过滤：{pruned}")
        ev(
            f"B4 label={label} execute_status=canceled cancel_status=canceled "
            f"heartbeat_rate=0 marker_gone=true not_running_claims_cleanup=false canceled_filter_ok=true"
        )
    finally:
        bridge.close()
    return sandbox


# ---- 场景 B5：PDEATHSIG 的作用域边界（仅 Linux） ---------------------------

def scenario_b5_grandchild_scope(exe, root, label):
    print("\n== 场景 B5：bridge 猝死遗留 cgroup 作用域由下一次启动自动回收 ==", flush=True)
    if IS_WINDOWS:
        ev("B5 label=windows skipped reason=Windows 作业对象在句柄关闭时整棵树回收，无遗留 cgroup 边界")
        return None
    sandbox = make_sandbox(root, label, "B5")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="B5")
    task_id = "real-b5-grandchild"
    beat = work / "grandchild-beat.txt"
    pid_file = work / "grandchild.pid"
    decoy = Path("/sys/fs/cgroup") / f"aw-mcp-bridge-decoy-{os.getpid()}"
    cmd = (
        f"setsid sh -c 'echo $$ > {shlex.quote(str(pid_file))}; "
        f"while :; do echo tick >> {shlex.quote(str(beat))}; sleep 1; done' & sleep 60"
    )
    grand_pid = None
    scope_dir = None
    try:
        bridge.initialize()
        bridge.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd,
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        marker = runs / ".inflight" / f"{task_id}.json"
        check(
            bool(wait_until(lambda: marker.is_file() and pid_file.is_file(), 15)),
            "B5 孙进程已写出 pid",
        )
        time.sleep(1)
        grand_pid = int(pid_file.read_text(encoding="utf-8").strip())
        candidates = [
            p
            for p in Path("/sys/fs/cgroup").glob(f"aw-mcp-bridge-{bridge.proc.pid}-*")
            if p.is_dir()
        ]
        check(len(candidates) == 1, f"B5 找到本轮作用域目录：{[p.name for p in candidates]}")
        scope_dir = candidates[0] if candidates else None
        scope_name = scope_dir.name if scope_dir else ""
        sidecar = runs / ".cgroup-scopes" / f"{scope_name}.json" if scope_name else None
        check(bool(sidecar) and sidecar.is_file(), f"B5 作用域归属记录已落盘：{scope_name}")
        owner = read_json(sidecar) if sidecar and sidecar.is_file() else {}
        check(scope_dir is not None and scope_dir.is_dir(), f"B5 作用域目录存在：{scope_dir}")
        check(owner.get("task_id") == task_id, "B5 sidecar 点名本轮 task_id")
        check(owner.get("attempt") == 1, f"B5 sidecar 点名 attempt（{owner.get('attempt')}）")
        procs = ""
        if scope_dir and scope_dir.is_dir():
            procs = (scope_dir / "cgroup.procs").read_text(encoding="utf-8")
        check(str(grand_pid) in procs.split(), f"B5 孙进程在作用域成员内：{procs.split()}")

        # 负向对照：没有 sidecar 的作用域看着像孤儿，但判不准，不得清理
        decoy_created = False
        try:
            decoy.mkdir()
            decoy_created = True
        except OSError as error:
            ev(f"B5 label={label} decoy_create_failed={error}")

        bridge.kill()
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(3)
        size_after = beat.stat().st_size if beat.is_file() else 0
        survived = size_after > size_before and pid_alive(grand_pid)
        check(survived, "B5 红证据：bridge 被 kill -9 后孙进程仍存活（PDEATHSIG 不遗传）")
        check(scope_dir is not None and scope_dir.is_dir(), "B5 红证据：遗留作用域目录仍在")
        check(bool(sidecar) and sidecar.is_file(), "B5 红证据：遗留 sidecar 仍在")

        restarted = Bridge(binary, sandbox, name="B5-restarted")
        try:
            restarted.initialize()
            gone = wait_until(lambda: not pid_alive(grand_pid), 10)
            check(bool(gone), "B5 启动回收后孙进程消失（自动收敛，不含人工巡检）")
            size_after_restart = beat.stat().st_size if beat.is_file() else 0
            time.sleep(2)
            stable = beat.stat().st_size if beat.is_file() else 0
            check(stable == size_after_restart, f"B5 回收后心跳停止（{size_after_restart} -> {stable}）")
            check(not scope_dir.is_dir(), f"B5 作用域目录已删除：{scope_dir}")
            check(not sidecar.exists() if sidecar else True, "B5 sidecar 已删除")
            check((runs / task_id / "attempt-1" / "dispatch.json").is_file(), "B5 attempt 目录与 dispatch 保留")

            audit = read_jsonl(runs / "cgroup-reclaim.jsonl")
            plan = [
                row
                for row in audit
                if row.get("trigger") == "startup"
                and row.get("phase") == "plan"
                and row.get("scope") == scope_name
            ]
            result = [
                row
                for row in audit
                if row.get("trigger") == "startup"
                and row.get("phase") == "result"
                and row.get("scope") == scope_name
            ]
            check(bool(plan), "B5 审计有 trigger=startup 的 plan 行")
            check(bool(result), "B5 审计有 trigger=startup 的 result 行")
            if plan:
                check(plan[-1].get("task_id") == task_id and plan[-1].get("attempt") == 1,
                      "B5 plan 行点名 task_id/attempt")
                check(len(plan[-1].get("members_before") or []) >= 1,
                      f"B5 plan 行记录清理前成员：{plan[-1].get('members_before')}")
            if result:
                check(result[-1].get("outcome") == "reclaimed",
                      f"B5 result outcome=reclaimed（{result[-1].get('outcome')}）")
                check((result[-1].get("members_after") or []) == [],
                      f"B5 result 记录清理后成员为空：{result[-1].get('members_after')}")
            ev(
                f"B5 label={label} scope={scope_name} grandchild_pid={grand_pid} "
                f"survived_before_restart={survived} gone_after_restart={bool(gone)} "
                f"audit_plan={len(plan)} audit_result={len(result)}"
            )

            if decoy_created:
                kept = [
                    row
                    for row in audit
                    if row.get("trigger") == "startup"
                    and row.get("phase") == "kept"
                    and row.get("scope") == decoy.name
                ]
                check(decoy.is_dir(), "B5 无 sidecar 的同类目录未被清理（判不准不动）")
                check(bool(kept), "B5 审计写明为什么不动（outcome=kept）")
                if kept:
                    check(kept[-1].get("reason") == "owner_record_missing",
                          f"B5 kept reason={kept[-1].get('reason')}")
                ev(f"B5 label={label} decoy_kept={decoy.is_dir()} kept_rows={len(kept)}")
        finally:
            restarted.close()
    finally:
        if grand_pid:
            try:
                os.kill(grand_pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        if decoy.is_dir():
            try:
                os.rmdir(str(decoy))
            except OSError:
                pass
        bridge.close()
    return sandbox


# ---- 场景 B6：跨进程取消 ---------------------------------------------------

def scenario_b6_cross_process_cancel(exe, root, label):
    print("\n== 场景 B6：task.cancel 跨进程可见（磁盘请求 + 心跳读取）==", flush=True)
    sandbox = make_sandbox(root, label, "B6")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    task_id = "real-b6-cross-cancel"
    beat = work / "b6-beat.txt"
    bridge_a = Bridge(binary, sandbox, name="B6-a", wire_name="wire-a.jsonl")
    bridge_b = Bridge(binary, sandbox, name="B6-b", wire_name="wire-b.jsonl")
    if IS_WINDOWS:
        cmd = cmd_heartbeat(beat, 20)
    else:
        cmd = f"trap '' TERM; i=0; while [ $i -lt 20 ]; do echo tick >> {shlex.quote(str(beat))}; sleep 1; i=$((i+1)); done"
    observed = []
    try:
        bridge_a.initialize()
        bridge_b.initialize()
        request_id = bridge_a.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd,
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        marker = runs / ".inflight" / f"{task_id}.json"
        check(bool(wait_until(lambda: marker.is_file() and (runs / task_id / "attempt-1" / "dispatch.json").is_file(), 15)),
              "B6 A 进程任务已开始")
        marker_record = read_json(marker)
        request_file = runs / ".cancel" / f"{task_id}.json"
        time.sleep(1)

        outcome = {}

        def cancel_from_b():
            payload, is_error, _ = bridge_b.call(
                "task.cancel",
                {"task_id": task_id, "reason": "B6 跨进程取消", "requested_by": "real-chain-b6"},
                timeout=60,
            )
            outcome["payload"] = payload
            outcome["is_error"] = is_error

        worker = threading.Thread(target=cancel_from_b, daemon=True)
        worker.start()
        deadline = time.time() + 30
        while time.time() < deadline and worker.is_alive():
            if request_file.is_file():
                try:
                    state = read_json(request_file).get("status")
                except (json.JSONDecodeError, OSError):
                    state = None
                if state and (not observed or observed[-1] != state):
                    observed.append(state)
            time.sleep(0.02)
        worker.join(timeout=30)
        check(not worker.is_alive(), "B6 跨进程取消请求在 30s 内返回")
        check(not outcome.get("is_error"), "B6 跨进程 task.cancel 不是工具错误")
        payload = outcome.get("payload") or {}
        check(payload.get("cross_process") is True, f"B6 标明跨进程：{payload.get('cross_process')}")
        check(payload.get("status") == "canceled", f"B6 跨进程取消最终 canceled（{payload.get('status')}）")
        check(payload.get("request_state") == "done", f"B6 request_state=done（{payload.get('request_state')}）")
        check(payload.get("cleanup_claimed") is True, "B6 已落盘 canceled 回执才宣称清理")

        final = read_json(request_file) if request_file.is_file() else {}
        check(final.get("status") == "done", f"B6 请求文件终态 done（{final.get('status')}）")
        check(final.get("requested_by") == "real-chain-b6", "B6 请求记录发起者")
        check(bool(final.get("requestedAt")), "B6 请求记录时间")
        target = final.get("target") or {}
        check(target.get("pid") == marker_record.get("pid"), "B6 请求 target 指向 A 的进程")
        check(target.get("startedAt") == marker_record.get("startedAt"), "B6 请求 target 指向本轮执行")
        check(bool(final.get("acknowledgedAt")), "B6 记录执行侧已读时刻（acknowledged）")
        check(bool(final.get("finishedAt")), "B6 记录完成时刻（done）")
        check(observed[:1] in (["pending"], ["acknowledged"]),
              f"B6 观测到落盘后未读或已读在清理的中间态：{observed}")
        check("acknowledged" in observed or bool(final.get("acknowledgedAt")),
              f"B6 三态可区分：observed={observed}")
        check("done" in observed or final.get("status") == "done", "B6 三态可区分：done 被观测到")
        ev(
            f"B6 label={label} states={observed} cancel_status={payload.get('status')} "
            f"request_state={payload.get('request_state')} requested_by={final.get('requested_by')} "
            f"target_pid={target.get('pid')} acknowledgedAt={bool(final.get('acknowledgedAt'))}"
        )

        executed = bridge_a.await_id(request_id, timeout=60)
        exec_payload = json.loads(executed["result"]["content"][0]["text"])
        check(exec_payload.get("status") == "canceled", f"B6 A 侧执行状态 canceled（{exec_payload.get('status')}）")
        check((exec_payload.get("blocked_reason") or "").startswith("canceled："),
              f"B6 A 侧回执是取消档：{(exec_payload.get('blocked_reason') or '')[:40]}...")
        handback = read_json(runs / task_id / "attempt-1" / "handback.json")
        check(handback.get("status") == "canceled", "B6 盘上 handback.status=canceled")
        check(not marker.is_file(), "B6 取消完成后活跃标记已删除")
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(2)
        size_after = beat.stat().st_size if beat.is_file() else 0
        check(size_after == size_before, f"B6 取消后心跳停止（{size_before} -> {size_after}）")

        again, again_error, _ = bridge_b.call(
            "task.cancel",
            {"task_id": task_id, "reason": "B6 再查一次", "requested_by": "real-chain-b6"},
        )
        check(not again_error, "B6 第二次跨进程 task.cancel 不是工具错误")
        check(again.get("status") == "not_running",
              f"B6 执行已结束、标记已删，第二次调用返回 not_running：{again.get('status')}")
        kept = read_json(request_file) if request_file.is_file() else {}
        check(kept.get("status") == "done" and kept.get("requested_by") == "real-chain-b6",
              f"B6 第二次调用不覆盖已落盘的 done 请求：{kept.get('status')}/{kept.get('requested_by')}")

        ghost, ghost_error, _ = bridge_b.call(
            "task.cancel", {"task_id": "real-b6-ghost", "requested_by": "real-chain-b6"}
        )
        check(not ghost_error and ghost.get("status") == "not_running",
              f"B6 不存在的 task_id 返回 not_running：{ghost.get('status')}")
        check(not (runs / ".cancel" / "real-b6-ghost.json").exists(),
              "B6 not_running 不生成取消请求文件")
    finally:
        bridge_a.close()
        bridge_b.close()
    return sandbox


# ---- 场景 C：并发 ---------------------------------------------------------

def scenario_c_sequential(exe, root, label):
    print("\n== 场景 C1：同 task_id 顺序连派两次 ==", flush=True)
    sandbox = make_sandbox(root, label, "C1")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="C1")
    task_id = "real-c-sequential"
    counter = "once-c1.txt"
    try:
        bridge.initialize()
        args = {
            "task_id": task_id,
            "cmd": cmd_slow_append(counter, 2),
            "cwd": str(work),
            "timeout_secs": 60,
        }
        first, first_error, _ = bridge.call("task.execute", args)
        second, second_error, _ = bridge.call("task.execute", args)
        check(not first_error and not second_error, "C1 两次调用都不是工具错误")
        check(first.get("attempt") == 1 and second.get("attempt") == 1, "C1 两次都是 attempt=1")
        # dispatched 契约：重投返回既有回执原文，不新增任何 replay 字段（那是 standalone 的字段）
        check(second == first, "C1 第二次与第一次逐字段相同（回放既有回执）")
        check("replayed" not in second, "C1 dispatched 回执不新增 replayed 字段（契约口径）")
        check(second.get("finishedAt") == first.get("finishedAt"), "C1 第二次没有重跑（finishedAt 不变）")
        lines = (work / counter).read_text(encoding="utf-8").splitlines()
        check(len(lines) == 1, f"C1 命令只执行一次（once 文件 {len(lines)} 行）")
        attempts = sorted(p.name for p in (runs / task_id).iterdir() if p.name.startswith("attempt-"))
        check(attempts == ["attempt-1"], f"C1 只有一个 attempt 目录：{attempts}")

        # 异参重投：必须明确拒绝，且不追加 attempt
        conflict_args = dict(args)
        conflict_args["cmd"] = cmd_write("other-c1.txt", "different")
        conflict, conflict_error, _ = bridge.call("task.execute", conflict_args)
        check(conflict_error, "C1 同 id 异参是工具错误")
        check(
            "task_id_conflict" in json.dumps(conflict, ensure_ascii=False),
            f"C1 异参被具名拒绝：{conflict}",
        )
        attempts_after = sorted(p.name for p in (runs / task_id).iterdir() if p.name.startswith("attempt-"))
        check(attempts_after == ["attempt-1"], f"C1 拒绝后没有新 attempt：{attempts_after}")
        check(not (work / "other-c1.txt").exists(), "C1 被拒绝的请求零执行（没有产物）")
        ev(
            f"C1 label={label} replay_identical=true replayed_field_absent=true attempts=1 "
            f"conflict_named=true conflict_new_attempt=false"
        )
    finally:
        bridge.close()
    return sandbox


def scenario_c_same_id_two_processes(exe, root, label):
    print("\n== 场景 C2：同 task_id 两进程同时派 ==", flush=True)
    sandbox = make_sandbox(root, label, "C2")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    task_id = "real-c-two-procs"
    counter = "once-c2.txt"
    first = Bridge(binary, sandbox, name="C2-first")
    second = Bridge(binary, sandbox, name="C2-second")
    try:
        first.initialize()
        second.initialize()
        args = {
            "task_id": task_id,
            "cmd": cmd_slow_append(counter, 3),
            "cwd": str(work),
            "timeout_secs": 60,
        }
        id_a = first.send("tools/call", {"name": "task.execute", "arguments": args})
        id_b = second.send("tools/call", {"name": "task.execute", "arguments": args})

        def payload_of(bridge, request_id):
            response = bridge.await_id(request_id, timeout=120)
            result = response["result"]
            return json.loads(result["content"][0]["text"]), bool(result.get("isError"))

        a, a_error = payload_of(first, id_a)
        b, b_error = payload_of(second, id_b)
        check(not a_error and not b_error, "C2 两个进程都拿到正常回执")
        check(a.get("status") == "ok" and b.get("status") == "ok", "C2 两回执 status=ok")
        check(a == b, "C2 两进程拿到逐字段相同的回执（同一 attempt 的同一份证据）")
        check(a.get("attempt") == b.get("attempt") == 1, "C2 两方都落到 attempt-1")
        check(a.get("finishedAt") == b.get("finishedAt"), "C2 两方看到同一 finishedAt")
        check(a.get("stdout_digest") == b.get("stdout_digest"), "C2 两方看到同一 stdout_digest")
        attempts = sorted(p.name for p in (runs / task_id).iterdir() if p.name.startswith("attempt-"))
        check(attempts == ["attempt-1"], f"C2 没有第二个 attempt 目录：{attempts}")
        lines = (work / counter).read_text(encoding="utf-8").splitlines()
        check(len(lines) == 1, f"C2 命令只执行一次（once 文件 {len(lines)} 行）")
        ev(
            f"C2 label={label} identical_receipt=true attempts=1 once_lines={len(lines)} "
            f"same_finishedAt=true"
        )
    finally:
        first.close()
        second.close()
    return sandbox


def scenario_c_parallel(exe, root, label):
    print("\n== 场景 C3：两个不同 task_id 并行 ==", flush=True)
    sandbox = make_sandbox(root, label, "C3")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge_x = Bridge(binary, sandbox, name="C3-x")
    bridge_y = Bridge(binary, sandbox, name="C3-y")
    observer = Bridge(binary, sandbox, name="C3-observer")
    ids = {"real-c-par-x": "out-x.txt", "real-c-par-y": "out-y.txt"}
    try:
        bridge_x.initialize()
        bridge_y.initialize()
        observer.initialize()
        sent = {}
        for bridge, task_id, delay in (
            (bridge_x, "real-c-par-x", 3),
            (bridge_y, "real-c-par-y", 3),
        ):
            sent[task_id] = bridge.send(
                "tools/call",
                {
                    "name": "task.execute",
                    "arguments": {
                        "task_id": task_id,
                        "cmd": cmd_slow_append(ids[task_id], delay),
                        "cwd": str(work),
                        "timeout_secs": 60,
                    },
                },
            )

        observed = wait_until(
            lambda: (
                lambda listed: listed
                if listed.get("total") == 2
                else None
            )(observer.call("runs.inflight", {"action": "list"})[0]),
            8,
        )
        check(bool(observed), "C3 并行期间观察者看到 2 条 inflight 标记")
        if observed:
            seen = sorted(m.get("task_id") for m in observed.get("markers", []))
            check(seen == sorted(ids), f"C3 标记按 task 分开：{seen}")
            ev(f"C3 label={label} observed_markers={seen}")

        results = {}
        for bridge, task_id in ((bridge_x, "real-c-par-x"), (bridge_y, "real-c-par-y")):
            response = bridge.await_id(sent[task_id], timeout=120)
            result = response["result"]
            results[task_id] = json.loads(result["content"][0]["text"])
        for task_id, artifact in ids.items():
            payload = results[task_id]
            check(payload.get("status") == "ok", f"C3 {task_id} status=ok")
            locators = [a.get("locator") for a in payload.get("artifacts", [])]
            check(
                all(task_id in locator for locator in locators) and locators,
                f"C3 {task_id} artifacts 全部指向自己的 attempt：{locators}",
            )
            check((work / artifact).is_file(), f"C3 {task_id} 产物落盘：{artifact}")
        check(
            "out-x.txt" in (results["real-c-par-x"].get("verify_cmd") or "")
            and "out-y.txt" in (results["real-c-par-y"].get("verify_cmd") or ""),
            "C3 两任务回执各自带回自己的命令",
        )
        check(
            "real-c-par-y" not in json.dumps(results["real-c-par-x"], ensure_ascii=False),
            "C3 x 的回执不混入 y",
        )
        final_markers, _, _ = observer.call("runs.inflight", {"action": "list"})
        check(final_markers.get("total") == 0, "C3 全部结束后无残留标记")
        ev(
            f"C3 label={label} x_attempt={results['real-c-par-x'].get('attempt')} "
            f"y_attempt={results['real-c-par-y'].get('attempt')} final_inflight=0"
        )
    finally:
        bridge_x.close()
        bridge_y.close()
        observer.close()
    return sandbox


# ---- 场景 D1：周期清扫（真实周期线程） ------------------------------------

def scenario_d1_periodic_sweep(exe, root, label):
    print("\n== 场景 D1：bridge 猝死残标由周期清扫收掉 ==", flush=True)
    sandbox = make_sandbox(root, label, "D1")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    task_id = "real-d1-periodic"
    first = Bridge(binary, sandbox, name="D1-first")
    second = None
    try:
        first.initialize()
        first.send(
            "tools/call",
            {
                "name": "task.execute",
                "arguments": {
                    "task_id": task_id,
                    "cmd": cmd_heartbeat("heartbeat-d1.txt", 60),
                    "cwd": str(work),
                    "timeout_secs": 120,
                },
            },
        )
        marker = runs / ".inflight" / f"{task_id}.json"
        check(bool(wait_until(lambda: marker.is_file(), 15)), "D1 执行中标记存在")
        time.sleep(1)
        first.kill()
        check(first.proc.poll() is not None, "D1 bridge 猝死")

        # 立刻起观察者：startup 在 startedAt+5s 容差内，应保留；随后 period 档才清
        second = Bridge(binary, sandbox, name="D1-periodic", sweep="1")
        second.initialize()
        startup_rows = [
            row
            for row in read_jsonl(runs / "inflight-sweep.jsonl")
            if row.get("trigger") == "startup"
        ]
        startup_hit = [
            item
            for row in startup_rows
            for item in row.get("removed", [])
            if item.get("task_id") == task_id
        ]
        check(not startup_hit, f"D1 startup 未删除仍处容差内的标记：{startup_hit}")

        gone = wait_until(lambda: not marker.is_file(), 20)
        check(bool(gone), "D1 周期清扫在 20s 内删除残标")
        periodic_rows = [
            row
            for row in read_jsonl(runs / "inflight-sweep.jsonl")
            if row.get("trigger") == "periodic"
        ]
        removed = [
            item
            for row in periodic_rows
            for item in row.get("removed", [])
            if item.get("task_id") == task_id
        ]
        check(bool(removed), f"D1 审计点名 trigger=periodic 删除：{removed}")
        ev(
            f"D1 label={label} startup_removed=False periodic_removed={len(removed)} "
            f"marker_gone={not marker.is_file()}"
        )
    finally:
        first.close()
        if second is not None:
            second.close()
    return sandbox


# ---- 场景 D2：逃逸进程整树清理（仅 Linux） ---------------------------------

def scenario_d2_escaped_tree(exe, root, label):
    print("\n== 场景 D2：逃逸进程整树清理 ==", flush=True)
    if IS_WINDOWS:
        ev(
            "D2 label=windows skipped reason=需要作业外进程可枚举可终止的 OS 原语，本机不存在；"
            "Windows 侧能力位保持 false（见 capabilities）"
        )
        return None
    sandbox = make_sandbox(root, label, "D2")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    work = sandbox / "work"
    bridge = Bridge(binary, sandbox, name="D2")
    task_id = "real-d2-escape"
    grand = work / "escape-grand.sh"
    script = work / "escape-tree.sh"
    pid_file = work / "escaped.pid"
    move_file = work / "escaped.move"
    sink_file = work / "escaped.sink"
    beat = work / "escaped-beat.txt"
    grand.write_text(
        "#!/bin/sh\n"
        "echo $$ > \"$1\"\n"
        "echo $$ > \"$2/cgroup.procs\" && echo moved > \"$3\" || echo not_moved > \"$3\"\n"
        "i=0\n"
        "while [ $i -lt 60 ]; do echo tick >> \"$4\"; sleep 1; i=$((i+1)); done\n",
        encoding="utf-8",
    )
    script.write_text(
        "#!/bin/sh\n"
        "me=$(awk -F: '$1==\"0\"{print $3}' /proc/self/cgroup)\n"
        "base=/sys/fs/cgroup$(dirname \"$me\")\n"
        "sink=\"$base/aw-real-escape-$$\"\n"
        "mkdir -p \"$sink\" || true\n"
        f"printf '%s' \"$sink\" > {shlex.quote(str(sink_file))}\n"
        f"setsid sh {shlex.quote(str(grand))} {shlex.quote(str(pid_file))} \"$sink\" "
        f"{shlex.quote(str(move_file))} {shlex.quote(str(beat))} &\n"
        "sleep 60\n",
        encoding="utf-8",
    )
    try:
        bridge.initialize()
        payload, is_error, _ = bridge.call(
            "task.execute",
            {
                "task_id": task_id,
                "cmd": f"sh {shlex.quote(str(script))}",
                "cwd": str(work),
                "timeout_secs": 2,
            },
            timeout=120,
        )
        check(not is_error, "D2 超时是正常回执")
        check(payload.get("status") == "blocked", f"D2 status=blocked（实际 {payload.get('status')}）")
        check(
            (payload.get("blocked_reason") or "").startswith("timeout："),
            f"D2 超时档：{(payload.get('blocked_reason') or '')[:60]}...",
        )
        has_pid = bool(wait_until(lambda: pid_file.is_file(), 10))
        check(has_pid, "D2 孙进程写出了自己的 pid")
        escaped_pid = int(pid_file.read_text(encoding="utf-8").strip()) if has_pid else None
        moved = move_file.read_text(encoding="utf-8").strip() if move_file.is_file() else ""
        check(moved == "moved", f"D2 孙进程已逃出 bridge 的 cgroup.procs（move={moved}）")

        if escaped_pid:
            gone = wait_until(lambda: not pid_alive(escaped_pid), 10)
            check(bool(gone), f"D2 逃逸 pid {escaped_pid} 在清理后消失")
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(2)
        size_after = beat.stat().st_size if beat.is_file() else 0
        check(size_after == size_before, f"D2 逃逸进程心跳停止（{size_before} -> {size_after}）")

        sink = Path(sink_file.read_text(encoding="utf-8")) if sink_file.is_file() else None
        if sink and sink.is_dir():
            procs = (sink / "cgroup.procs").read_text(encoding="utf-8").strip()
            events = (sink / "cgroup.events").read_text(encoding="utf-8")
            check(procs == "", f"D2 旁路 cgroup 的 cgroup.procs 为空：{procs!r}")
            check("populated 0" in events, f"D2 旁路 cgroup populated 0：{events.strip()!r}")
            os.rmdir(str(sink))
        ev(
            f"D2 label={label} escaped_pid={escaped_pid} moved_out_of_cgroup=true "
            f"escaped_gone=true sink_populated=0"
        )
    finally:
        bridge.close()
    return sandbox


def pid_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


# ---- 场景 D3：`_session/initialize-*.json` 按天保留 + 增长实测 ---------------

def scenario_d3_session_retention(exe, root, label):
    print("\n== 场景 D3：_session 按天保留与体积实测 ==", flush=True)
    sandbox = make_sandbox(root, label, "D3")
    binary = install_binary(exe, sandbox)
    runs = runs_dir_of(sandbox)
    session = runs / "_session"
    session.mkdir(parents=True, exist_ok=True)
    now = time.time()
    old_ts = int((now - 45 * 86400) * 1000)
    fresh_ts = int((now - 86400) * 1000)
    old = session / f"initialize-999-{old_ts}-0.json"
    fresh = session / f"initialize-998-{fresh_ts}-0.json"
    unparsable = session / "initialize-broken-name.json"
    for path in (old, fresh, unparsable):
        path.write_text('{"clientInfo":{"name":"seed"}}', encoding="utf-8")
    os.utime(str(old), (now - 45 * 86400, now - 45 * 86400))

    bridge = Bridge(binary, sandbox, name="D3")
    try:
        bridge.initialize()
        check(not old.exists(), "D3 startup 删除了超过 30 天的 initialize 记录")
        check(fresh.exists(), "D3 1 天前的记录仍在保留期内")
        check(unparsable.exists(), "D3 名字判不准的记录保留")
        audit = read_jsonl(runs / "session-retention.jsonl")
        check(
            any(row.get("phase") == "plan" and row.get("trigger") == "startup" for row in audit),
            f"D3 审计有 plan 行：{len(audit)} 行",
        )
        check(
            any(
                item.get("file") == old.name and item.get("outcome") == "deleted"
                for row in audit
                for item in row.get("removed", [])
            ),
            "D3 审计点名被删除的文件",
        )
        check(
            any(
                item.get("file") == unparsable.name
                for row in audit
                for item in row.get("kept", [])
            ),
            "D3 审计记录了判不准而不删的文件",
        )

        # 增长实测：每次 initialize 一份新记录，量一量单份体积
        burst = 100
        before_files = sorted(session.glob("initialize-*.json"))
        before_bytes = sum(path.stat().st_size for path in before_files)
        for _ in range(burst):
            bridge.request(
                "initialize",
                {"clientInfo": {"name": "growth-probe"}, "protocolVersion": "2025-06-18"},
            )
        after_files = sorted(session.glob("initialize-*.json"))
        after_bytes = sum(path.stat().st_size for path in after_files)
        created = len(after_files) - len(before_files)
        growth = after_bytes - before_bytes
        per_file = (growth / created) if created else 0
        check(created == burst, f"D3 {burst} 次 initialize 生成 {burst} 份记录（实际 {created}）")
        ev(
            f"D3 label={label} removed_old=true kept_fresh=true kept_unparsable=true "
            f"records_created={created} bytes_growth={growth} bytes_per_record={per_file:.1f} "
            f"per_1000_records={per_file * 1000 / 1024:.1f}KiB"
        )
        print(
            f"[growth] 单份约 {per_file:.1f} B；若每天 1000 次 initialize，"
            f"30 天约 {per_file * 1000 * 30 / 1024 / 1024:.2f} MiB",
            flush=True,
        )
    finally:
        bridge.close()
    return sandbox


# ---- 主流程 ---------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, help="aw-mcp-bridge 可执行文件")
    parser.add_argument("--only", default="A,B1,B2,B3,B4,B5,B6,C1,C2,C3,D1,D2,D3")
    parser.add_argument("--sandbox-root", default="")
    parser.add_argument("--label", default="windows" if IS_WINDOWS else "linux")
    parser.add_argument("--keep", action="store_true", help="保留 sandbox 便于人工核对")
    args = parser.parse_args()

    exe = Path(args.binary).resolve()
    if not exe.is_file():
        print(f"二进制不存在：{exe}", file=sys.stderr)
        return 2

    print(f"label={args.label} platform={'windows' if IS_WINDOWS else 'posix'} binary={exe}", flush=True)
    selected = {item.strip().upper() for item in args.only.split(",") if item.strip()}
    sandboxes = []
    try:
        if "A" in selected:
            sandboxes.append(("A", scenario_a(exe, args.sandbox_root, args.label)))
        if "B1" in selected:
            sandboxes.append(("B1", scenario_b_timeout(exe, args.sandbox_root, args.label)))
        if "B2" in selected:
            sandboxes.append(("B2", scenario_b_cancel(exe, args.sandbox_root, args.label)))
        if "B3" in selected:
            sandboxes.append(("B3", scenario_b_grace(exe, args.sandbox_root, args.label)))
        if "B4" in selected:
            sandboxes.append(("B4", scenario_b4_explicit_cancel(exe, args.sandbox_root, args.label)))
        if "B5" in selected:
            sandboxes.append(("B5", scenario_b5_grandchild_scope(exe, args.sandbox_root, args.label)))
        if "B6" in selected:
            sandboxes.append(("B6", scenario_b6_cross_process_cancel(exe, args.sandbox_root, args.label)))
        if "C1" in selected:
            sandboxes.append(("C1", scenario_c_sequential(exe, args.sandbox_root, args.label)))
        if "C2" in selected:
            sandboxes.append(("C2", scenario_c_same_id_two_processes(exe, args.sandbox_root, args.label)))
        if "C3" in selected:
            sandboxes.append(("C3", scenario_c_parallel(exe, args.sandbox_root, args.label)))
        if "D1" in selected:
            sandboxes.append(("D1", scenario_d1_periodic_sweep(exe, args.sandbox_root, args.label)))
        if "D2" in selected:
            sandboxes.append(("D2", scenario_d2_escaped_tree(exe, args.sandbox_root, args.label)))
        if "D3" in selected:
            sandboxes.append(("D3", scenario_d3_session_retention(exe, args.sandbox_root, args.label)))
    finally:
        if not args.keep:
            for _, sandbox in sandboxes:
                if sandbox is not None:
                    shutil.rmtree(str(sandbox), ignore_errors=True)
        else:
            for name, sandbox in sandboxes:
                if sandbox is not None:
                    print(f"[keep] {name}: {sandbox}", flush=True)

    print("\n==== 汇总 ====", flush=True)
    print(f"evidence_lines={len(EVIDENCE)} failures={len(FAILURES)}", flush=True)
    for item in FAILURES:
        print(f"  [FAIL] {item}", flush=True)
    return 0 if not FAILURES else 1


if __name__ == "__main__":
    sys.exit(main())
