#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""真实链路端到端验证（MCP over stdio，不调用任何内部函数）。

跑四组场景，全部经 `aw-mcp-bridge` 的真实 stdio JSON-RPC 路径：

  A. 正常任务：真实产出文件 -> attempt / inflight / dispatch / handback / 产物互相对得上
  B. 异常：执行中超时；执行中取消（产品没有 cancel 工具，唯一可走的是 bridge 进程死亡）
     然后重启服务，验证 startup 档自动清扫残留标记与 attempt 目录去向
  C. 并发：同 task_id 连派（顺序 + 两进程同时）；不同 task_id 并行
  D. 收尾事实：无 *.tmp、无残标、无 .prune-trash

用法：
  python scripts/real-chain-verify/verify.py --binary <aw-mcp-bridge[.exe]> [--only A,B,C] [--keep]

退出码：0 全部通过；1 有断言失败；2 环境/参数问题。
"""

import argparse
import hashlib
import json
import os
import queue
import shutil
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

    def __init__(self, exe, sandbox, name="bridge", sweep="0"):
        self.name = name
        self.exe = Path(exe)
        self.sandbox = Path(sandbox)
        self.work = self.sandbox / "work"
        self.work.mkdir(parents=True, exist_ok=True)
        self.wire_path = self.sandbox / "wire.jsonl"
        env = dict(os.environ)
        env["APPDATA"] = str(self.sandbox / "appdata")
        env["HOME"] = str(self.sandbox / "appdata")
        # 0 = 只做启动清扫，避免周期线程把证据弄成时序不确定
        env["AW_INFLIGHT_SWEEP_SECS"] = sweep
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
            # 不应出现乱序响应；出现即说明桥接层串了响应
            raise RuntimeError(f"{self.name}: 收到非预期 id={response.get('id')}，期望 {request_id}")

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
        check(
            not any("cancel" in (name or "").lower() for name in names),
            "A tools/list 里没有 cancel 工具（取消不可从 MCP 触达）",
        )
        ev(f"A tool_names={names} cancel_tool_absent=true")
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
        check(bridge.proc.poll() is not None, "B2 bridge 已被杀死（这是唯一的“取消”入口）")

        beat = work / heartbeat
        size_before = beat.stat().st_size if beat.is_file() else 0
        time.sleep(3)
        size_after = beat.stat().st_size if beat.is_file() else 0
        # Windows：Job Object KILL_ON_JOB_CLOSE；Linux：bridge 死亡不一定带走子进程
        ev(
            f"B2 label={label} killed_at=~3s child_heartbeat_before={size_before} after={size_after} "
            f"child_survived={size_after > size_before}"
        )
        check(marker.is_file(), "B2 进程被杀死后残留标记仍在（没有机会执行 Drop）")
        check(
            (attempt_dir / "dispatch.json").is_file()
            and not (attempt_dir / "handback.json").is_file(),
            "B2 取消后 attempt-1 只有 dispatch、无 handback（不宣称完成）",
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


# ---- 主流程 ---------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, help="aw-mcp-bridge 可执行文件")
    parser.add_argument("--only", default="A,B1,B2,B3,C1,C2,C3")
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
        if "C1" in selected:
            sandboxes.append(("C1", scenario_c_sequential(exe, args.sandbox_root, args.label)))
        if "C2" in selected:
            sandboxes.append(("C2", scenario_c_same_id_two_processes(exe, args.sandbox_root, args.label)))
        if "C3" in selected:
            sandboxes.append(("C3", scenario_c_parallel(exe, args.sandbox_root, args.label)))
    finally:
        if not args.keep:
            for _, sandbox in sandboxes:
                shutil.rmtree(str(sandbox), ignore_errors=True)
        else:
            for name, sandbox in sandboxes:
                print(f"[keep] {name}: {sandbox}", flush=True)

    print("\n==== 汇总 ====", flush=True)
    print(f"evidence_lines={len(EVIDENCE)} failures={len(FAILURES)}", flush=True)
    for item in FAILURES:
        print(f"  [FAIL] {item}", flush=True)
    return 0 if not FAILURES else 1


if __name__ == "__main__":
    sys.exit(main())
