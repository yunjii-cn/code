import os
import sys
import json
import time
import subprocess
import threading
import queue
from typing import Callable
from pathlib import Path

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"


def _data_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        exe_dir = os.path.abspath(dev_dir) if dev_dir else os.path.abspath(os.path.dirname(sys.executable))
    else:
        exe_dir = str(Path(__file__).resolve().parent.parent.parent)
    d = os.path.join(exe_dir, "data")
    os.makedirs(d, exist_ok=True)
    return d

def _temp_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        exe_dir = os.path.abspath(dev_dir) if dev_dir else os.path.abspath(os.path.dirname(sys.executable))
    else:
        exe_dir = str(Path(__file__).resolve().parent.parent.parent)
    d = os.path.join(exe_dir, "temp")
    os.makedirs(d, exist_ok=True)
    return d

def _get_debug_path(debug_file: str) -> str:
    return os.path.join(_temp_dir(), "debug", debug_file)


class ClaudeCliRunner:
    """管理 claude-code-tudou CLI 子进程"""

    def __init__(self, project_root: str, node_dir: str, bun_dir: str):
        self.project_root = project_root
        self.node_dir = node_dir
        self.bun_dir = bun_dir
        self.cli_entry = os.path.join(project_root, "src", "entrypoints", "cli.tsx")
        self.cli_wrapper = os.path.join(project_root, "bin", "claude-code-tudou")

    def _find_node(self) -> str:
        local = os.path.join(self.node_dir, "node.exe")
        if os.path.exists(local):
            return local
        return "node"

    def _find_bun(self) -> str:
        local = os.path.join(self.bun_dir, "bun.exe")
        if os.path.exists(local):
            return local
        return None

    @staticmethod
    def _tool_display_name(name: str) -> str:
        _names = {
            "fs_open_file": "读取文件", "Read": "读取文件",
            "fs_put_file": "写入文件", "Write": "写入文件",
            "fs_patch_file": "编辑文件", "Edit": "编辑文件",
            "shell_run": "执行命令", "Bash": "执行命令",
            "text_search": "搜索文本", "Grep": "搜索文本",
            "path_find": "查找文件", "Glob": "查找文件",
            "http_get_url": "获取网页", "WebFetch": "获取网页",
            "web_query": "搜索网络", "WebSearch": "搜索网络",
            "NotebookEdit": "编辑笔记本",
            "Task": "子任务",
            "TodoWrite": "更新任务",
            "AskUserQuestion": "询问用户",
            "Skill": "调用技能",
        }
        return _names.get(name, name)

    def _build_args(self, session_id: str, model: str, is_resuming: bool,
                    system_prompt: str = None, auto_approve: bool = False,
                    workspace_path: str = None) -> list:
        env_file = os.path.join(_data_dir(), ".env")
        version = "1.0.0"
        try:
            pkg_path = os.path.join(self.project_root, "package.json")
            if os.path.exists(pkg_path):
                with open(pkg_path, "r", encoding="utf-8") as f:
                    pkg = json.load(f)
                version = pkg.get("version", version)
        except Exception:
            pass
        args = [
            f"--env-file-if-exists={env_file}",
            "--define", f"MACRO.VERSION=\"{version}\"",
            "--define", "MACRO.BUILD_TIME=\"\"",
            "--define", "MACRO.PACKAGE_URL=\"@anthropic-ai/claude-code\"",
            "--define", "MACRO.NATIVE_PACKAGE_URL=\"@anthropic-ai/claude-code-native\"",
            "--define", "MACRO.ISSUES_EXPLAINER=\"https://github.com/anthropics/claude-code/issues\"",
            "--define", "MACRO.FEEDBACK_CHANNEL=\"https://github.com/anthropics/claude-code/discussions\"",
            "--define", "MACRO.VERSION_CHANGELOG=\"\"",
            self.cli_entry,
            "-p",
            "--output-format", "stream-json",
            "--include-partial-messages",
            "--verbose",
            "--max-turns", "50",
        ]
        if auto_approve:
            args.append("--dangerously-skip-permissions")
        if is_resuming:
            args.extend(["--resume", session_id])
        else:
            args.extend(["--session-id", session_id])
        if model and model.strip():
            args.extend(["--model", model.strip()])
        if system_prompt and system_prompt.strip():
            args.extend(["--system-prompt", system_prompt.strip()])
        return args

    def run(self, prompt: str, session_id: str, model: str, is_resuming: bool,
            workspace_path: str, env_overrides: dict = None,
            on_delta: Callable = None, on_status: Callable = None,
            on_log: Callable = None, on_proc: Callable = None,
            system_prompt: str = None, auto_approve: bool = False) -> dict:
        bun_path = self._find_bun()
        node_path = self._find_node()
        use_bun = bun_path is not None
        runner_path = bun_path if use_bun else node_path
        args = self._build_args(session_id, model, is_resuming, system_prompt, auto_approve, workspace_path)

        env = dict(os.environ)
        if os.path.exists(self.node_dir):
            env["PATH"] = self.node_dir + ";" + env.get("PATH", "")
        if os.path.exists(self.bun_dir):
            env["PATH"] = self.bun_dir + ";" + env.get("PATH", "")
        node_modules_dir = os.path.join(self.project_root, "node_modules")
        if os.path.isdir(node_modules_dir):
            existing_node_path = env.get("NODE_PATH", "")
            if existing_node_path:
                env["NODE_PATH"] = node_modules_dir + ";" + existing_node_path
            else:
                env["NODE_PATH"] = node_modules_dir
        if env_overrides:
            env.update(env_overrides)
        if workspace_path:
            if not os.path.isdir(workspace_path):
                os.makedirs(workspace_path, exist_ok=True)
            env["CLAUDE_CODE_WORKSPACE"] = workspace_path
            env["CLI_WORKSPACE"] = workspace_path
        env.setdefault("CLAUDE_CODE_GLOB_TIMEOUT_SECONDS", "60")

        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW

        def _log(msg, color="#888"):
            if on_log:
                on_log(msg, color)

        _log(f"[CLI] 启动: runner={runner_path} bun={use_bun} entry_exists={os.path.exists(self.cli_entry)}")
        _log(f"[CLI] MODEL_PROVIDER={env.get('MODEL_PROVIDER')} API_BASE_URL={env.get('API_BASE_URL')} OLLAMA_BASE_URL={env.get('OLLAMA_BASE_URL')} API_MODEL={env.get('API_MODEL')} OLLAMA_MODEL={env.get('OLLAMA_MODEL')}")

        _log_path = _get_debug_path("cli_debug.log")
        _log_file = open(_log_path, "a", encoding="utf-8")
        _log_file.write(f"=== CLI Debug Log {time.strftime('%Y-%m-%d %H:%M:%S')} ===\n")
        _log_file.write(f"runner={runner_path}\n")
        _log_file.write(f"use_bun={use_bun}\n")
        _log_file.write(f"args={[runner_path] + args}\n")
        _log_file.write(f"actual_cwd={workspace_path or self.project_root}\n")
        _log_file.write(f"workspace_path={workspace_path}\n")
        _log_file.write(f"project_root={self.project_root}\n")
        _log_file.write(f"env_overrides={env_overrides}\n")
        _log_file.write(f"MODEL_PROVIDER={env.get('MODEL_PROVIDER')}\n")
        _log_file.write(f"API_BASE_URL={env.get('API_BASE_URL')}\n")
        _log_file.write(f"API_MODEL={env.get('API_MODEL')}\n")
        _log_file.write(f"NODE_PATH={env.get('NODE_PATH')}\n")
        _log_file.write(f"runner_exists={os.path.exists(runner_path)}\n")
        _log_file.write(f"cli_entry_exists={os.path.exists(self.cli_entry)}\n")
        _log_file.flush()

        try:
            proc = subprocess.Popen(
                [runner_path] + args,
                cwd=workspace_path or self.project_root,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=env,
                startupinfo=si,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )

            _log_file.write(f"proc_pid={proc.pid}\n")
            _log_file.flush()
            _log(f"[CLI] 进程已启动 pid={proc.pid}")

            if on_proc:
                on_proc(proc)

            proc.stdin.write(prompt)
            proc.stdin.close()

            cli_timeout = 300
            timed_out = False

            def _timeout_watcher():
                nonlocal timed_out
                time.sleep(cli_timeout)
                if proc.poll() is None:
                    timed_out = True
                    try:
                        proc.kill()
                    except:
                        pass

            timeout_thread = threading.Thread(target=_timeout_watcher, daemon=True)
            timeout_thread.start()

            stderr_lines = []
            def _read_stderr():
                try:
                    for line in proc.stderr:
                        stderr_lines.append(line)
                except:
                    pass

            stderr_thread = threading.Thread(target=_read_stderr, daemon=True)
            stderr_thread.start()

            last_text = ""
            last_result = ""
            last_tool_info = ""
            all_tool_calls = []
            all_tool_results = []
            all_text_parts = []
            api_retry_count = 0
            api_retry_last_error = ""
            stdout_log = ""
            stdout_tail = []
            cli_session_id = ""
            has_stream_delta = False
            line_count = 0
            event_types = []
            idle_count = 0
            max_idle_after_exit = 30
            proc_exited = False

            stdout_queue = queue.Queue()

            def _read_stdout():
                try:
                    while True:
                        line = proc.stdout.readline()
                        if not line:
                            break
                        stdout_queue.put(line)
                except:
                    pass
                finally:
                    stdout_queue.put(None)

            stdout_reader = threading.Thread(target=_read_stdout, daemon=True)
            stdout_reader.start()

            while True:
                try:
                    line = stdout_queue.get(timeout=0.5)
                except queue.Empty:
                    if proc.poll() is not None:
                        proc_exited = True
                        idle_count += 1
                        if idle_count > max_idle_after_exit:
                            _log_file.write(f"  [TIMEOUT] stdout pipe timeout after process exit (pid={proc.pid} rc={proc.returncode}), breaking\n")
                            _log_file.flush()
                            try:
                                proc.kill()
                            except:
                                pass
                            break
                    continue

                if line is None:
                    break

                if not line:
                    if proc.poll() is not None:
                        proc_exited = True
                        idle_count += 1
                        if idle_count > max_idle_after_exit:
                            _log_file.write(f"  [TIMEOUT] stdout pipe still open after process exit (pid={proc.pid} rc={proc.returncode}), breaking\n")
                            _log_file.flush()
                            try:
                                proc.kill()
                            except:
                                pass
                            break
                        time.sleep(0.1)
                        continue
                    time.sleep(0.05)
                    continue
                idle_count = 0
                trimmed = line.strip()
                if not trimmed:
                    continue
                line_count += 1
                stdout_log += trimmed + "\n"
                stdout_tail.append(trimmed)
                if len(stdout_tail) > 20:
                    stdout_tail.pop(0)
                try:
                    parsed = json.loads(trimmed)
                except:
                    continue

                evt_type = parsed.get("type", "?")
                if evt_type not in event_types:
                    event_types.append(evt_type)
                    _log_file.write(f"  [NEW-TYPE] {evt_type}: {trimmed[:200]}\n")
                    _log_file.flush()
                    _log(f"[CLI] 事件类型: {evt_type}")

                if evt_type == "system" and parsed.get("subtype") == "init":
                    cli_session_id = parsed.get("session_id", "")
                    if cli_session_id:
                        _log(f"[CLI] 捕获session_id={cli_session_id}")
                        _log_file.write(f"  [SESSION] cli_session_id={cli_session_id}\n")
                        _log_file.flush()

                if evt_type == "system" and parsed.get("subtype") == "api_retry":
                    api_retry_count += 1
                    api_retry_last_error = parsed.get("error", "")
                    attempt = parsed.get("attempt", "?")
                    max_retries = parsed.get("max_retries", "?")
                    status = parsed.get("error_status", "?")
                    _log(f"[CLI] API重试 {attempt}/{max_retries} (HTTP {status})", "#FF9800")
                    if on_delta:
                        if api_retry_count <= 2:
                            on_delta(f"\x00TOOL\x00⚠️ API重试 {attempt}/{max_retries} (HTTP {status})")
                        elif attempt == max_retries:
                            on_delta(f"\x00TOOL\x00⚠️ API重试 {attempt}/{max_retries} - 仍受限流，请稍后再试")

                if evt_type == "stream_event":
                    sub_type = parsed.get("event", {}).get("type", "?")
                    delta_type = parsed.get("event", {}).get("delta", {}).get("type", "?")
                    if f"stream_event.{sub_type}.{delta_type}" not in event_types:
                        event_types.append(f"stream_event.{sub_type}.{delta_type}")
                        _log_file.write(f"  [STREAM-EVT] {sub_type}.{delta_type}\n")
                        _log_file.flush()

                if (parsed.get("type") == "stream_event" and
                    parsed.get("event", {}).get("type") == "content_block_delta" and
                    parsed.get("event", {}).get("delta", {}).get("type") == "text_delta"):
                    text = parsed["event"]["delta"].get("text", "")
                    if text and on_delta:
                        on_delta(text)
                        has_stream_delta = True

                if parsed.get("type") == "assistant":
                    msg = parsed.get("message", {})
                    if isinstance(msg.get("content"), list):
                        parts = [b.get("text", "") for b in msg["content"] if b.get("type") == "text" and isinstance(b.get("text"), str)]
                        if parts:
                            last_text = "\n".join(parts)
                            all_text_parts.extend(parts)
                            if not has_stream_delta and on_delta:
                                on_delta(last_text)
                                _log(f"[CLI] 从assistant消息补充文本 len={len(last_text)}", "#FF9800")
                        tool_uses = [b for b in msg["content"] if b.get("type") == "tool_use"]
                        if tool_uses:
                            for tu in tool_uses:
                                all_tool_calls.append(tu.get("name", "?"))
                            if not parts:
                                tool_names = [t.get("name", "?") for t in tool_uses]
                                last_tool_info = f"[AI调用了工具: {', '.join(tool_names)}]"
                                _log(f"[CLI] assistant仅含工具调用: {tool_names}", "#FF9800")
                                if on_delta:
                                    tool_display = [self._tool_display_name(n) for n in tool_names]
                                    on_delta(f"\x00TOOL\x00🔧 调用工具: {', '.join(tool_display)}")

                if parsed.get("type") == "result":
                    raw_result = parsed.get("result")
                    if raw_result is not None:
                        last_result = str(raw_result) if not isinstance(raw_result, str) else raw_result
                    result_subtype = parsed.get("subtype", "")
                    if not has_stream_delta and not last_text and on_delta:
                        if last_result.strip():
                            on_delta(last_result)
                            has_stream_delta = True
                            _log(f"[CLI] 从result补充文本 len={len(last_result)}", "#FF9800")
                        elif result_subtype == "error_max_turns":
                            _log(f"[CLI] result subtype=error_max_turns, num_turns={parsed.get('num_turns')}", "#FF9800")

                if parsed.get("type") == "user":
                    msg = parsed.get("message", {})
                    if isinstance(msg.get("content"), list):
                        for block in msg["content"]:
                            if block.get("type") == "tool_result":
                                content = block.get("content", "")
                                if isinstance(content, str):
                                    summary = content[:200].replace("\n", " ")
                                elif isinstance(content, list):
                                    texts = [b.get("text", "") for b in content if b.get("type") == "text"]
                                    summary = " ".join(texts)[:200].replace("\n", " ")
                                else:
                                    summary = ""
                                is_error = block.get("is_error", False)
                                all_tool_results.append({"error": is_error, "summary": summary})
                                if on_delta and len(all_tool_results) <= len(all_tool_calls):
                                    idx = len(all_tool_results) - 1
                                    tc_name = all_tool_calls[idx] if idx < len(all_tool_calls) else "?"
                                    tc_display = self._tool_display_name(tc_name)
                                    status_icon = "❌" if is_error else "✅"
                                    on_delta(f"\x00TOOL\x00{status_icon} {tc_display}: {summary[:80]}")

            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                try:
                    proc.kill()
                except:
                    pass
                proc.wait(timeout=5)
            stderr_thread.join(timeout=5)
            stderr_out = "".join(stderr_lines)

            _log_file.write(f"\n=== RESULT ===\n")
            _log_file.write(f"lines={line_count} types={event_types} has_delta={has_stream_delta} last_text_len={len(last_text)} rc={proc.returncode}\n")
            _log_file.write(f"tool_calls={all_tool_calls} tool_results_count={len(all_tool_results)} text_parts_count={len(all_text_parts)} api_retry_count={api_retry_count}\n")
            if stderr_out:
                _log_file.write(f"stderr={stderr_out[:2000]}\n")
            if not has_stream_delta and not last_text and stdout_log:
                _log_file.write(f"stdout_sample={stdout_log[:3000]}\n")
            if proc.returncode != 0 and stdout_tail:
                _log_file.write(f"stdout_tail_last5={stdout_tail[-5:]}\n")
            _log_file.flush()
            _log_file.close()

            _log(f"[CLI] 完成: lines={line_count} types={event_types} has_delta={has_stream_delta} rc={proc.returncode}",
                 "#4CAF50" if proc.returncode == 0 else "#F44336")
            if stderr_out and proc.returncode != 0:
                _log(f"[CLI] stderr: {stderr_out[:300]}", "#F44336")

            if proc.returncode == 0:
                return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id, "streamed": has_stream_delta}
            else:
                if timed_out:
                    return {"ok": False, "error": f"CLI执行超时({cli_timeout}秒)，已自动终止", "sessionId": session_id, "cliSessionId": cli_session_id or session_id}
                if has_stream_delta:
                    _log(f"[CLI] rc={proc.returncode} 但已收到流式文本(has_delta=True)，视为成功", "#FF9800")
                    return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id, "streamed": True}
                if last_text.strip():
                    _log(f"[CLI] rc={proc.returncode} 但已收到文本内容(len={len(last_text.strip())})，视为成功", "#FF9800")
                    return {"ok": True, "text": last_text.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id, "streamed": False}
                if last_result.strip():
                    _log(f"[CLI] rc={proc.returncode} 但已收到result内容(len={len(last_result.strip())})，视为成功", "#FF9800")
                    return {"ok": True, "text": last_result.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id, "streamed": False}
                if last_tool_info.strip():
                    _log(f"[CLI] rc={proc.returncode} 但AI已调用工具，返回工具信息", "#FF9800")
                    summary_parts = []
                    if all_text_parts:
                        summary_parts.append("\n".join(all_text_parts))
                    tool_errors = []
                    for i, tc_name in enumerate(all_tool_calls):
                        tr = all_tool_results[i] if i < len(all_tool_results) else None
                        if tr and tr.get("error"):
                            summary_parts.append(f"🔧 {tc_name}: ❌ {tr['summary'][:100]}")
                            tool_errors.append(tc_name)
                        elif tr:
                            summary_parts.append(f"🔧 {tc_name}: ✅ {tr['summary'][:100]}")
                        else:
                            summary_parts.append(f"🔧 {tc_name}")
                    combined = "\n".join(summary_parts) if summary_parts else last_tool_info
                    if tool_errors:
                        combined += f"\n\n⚠️ 部分工具执行出错({', '.join(tool_errors)})，AI未能完成回复。请尝试更具体的指令或缩小搜索范围后重试。"
                    else:
                        combined += "\n\n⚠️ AI在调用工具后中断，未能生成最终回复。请发送新消息继续对话。"
                    return {"ok": True, "text": combined.strip(), "sessionId": session_id, "cliSessionId": cli_session_id or session_id, "streamed": False}
                error = stderr_out.strip() or "Unknown CLI error."
                fallback = ""
                if api_retry_count > 0:
                    if "1113" in str(api_retry_last_error) or "余额不足" in str(api_retry_last_error) or "资源包" in str(api_retry_last_error):
                        fallback = f"❌ 模型余额不足，请充值后使用该模型（HTTP {api_retry_last_error}）"
                    else:
                        fallback = f"API服务暂时不可用(重试{api_retry_count}次后失败，HTTP {api_retry_last_error})，请稍后再试"
                if not fallback and error == "Unknown CLI error.":
                    for raw_line in stdout_log.split("\n"):
                        raw_line = raw_line.strip()
                        if not raw_line:
                            continue
                        try:
                            pj = json.loads(raw_line)
                            pj_sub = pj.get("subtype", "")
                            if pj.get("type") == "result" and pj_sub.startswith("error"):
                                fallback = pj.get("result", "") or pj.get("error", "")
                                if not fallback and pj_sub == "error_max_turns":
                                    fallback = f"已达到最大对话轮次限制({pj.get('num_turns', '?')}轮)，请发送新消息继续"
                                if fallback and ("1113" in str(fallback) or "余额不足" in str(fallback) or "资源包" in str(fallback) or "quota" in str(fallback).lower()):
                                    fallback = f"❌ 模型余额不足，请充值后使用该模型"
                                if fallback:
                                    break
                            if pj.get("type") == "error":
                                fallback = pj.get("error", {}).get("message", "") if isinstance(pj.get("error"), dict) else str(pj.get("error", ""))
                                if fallback:
                                    break
                        except:
                            pass
                    if not fallback:
                        fallback = f"CLI异常退出(rc={proc.returncode})"
                return {"ok": False, "error": fallback or error, "sessionId": session_id, "cliSessionId": cli_session_id or session_id}

        except Exception as e:
            return {"ok": False, "error": str(e), "sessionId": session_id, "cliSessionId": cli_session_id if 'cli_session_id' in dir() else session_id}
