#!/usr/bin/env python3
import sys, os, json, subprocess, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.dirname(os.path.abspath(__file__)))

from backend import OllamaProxyServer

proxy = OllamaProxyServer()
port = proxy.start("http://127.0.0.1:11434", "llama3:latest")
print(f"PROXY_PORT={port}")

env = dict(os.environ)
env["MODEL_PROVIDER"] = "anthropic"
env["ANTHROPIC_BASE_URL"] = f"http://127.0.0.1:{port}"
env["ANTHROPIC_API_KEY"] = "ollama-local"
env["ANTHROPIC_AUTH_TOKEN"] = "ollama-local"
env["ANTHROPIC_MODEL"] = "llama3:latest"
env["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"] = "1"
env["DISABLE_TELEMETRY"] = "1"

node_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "nodejs", "node-v24.11.1-win-x64", "node.exe")
cli_entry = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bin", "claude-code-tudou")
session_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

si = subprocess.STARTUPINFO()
si.dwFlags |= subprocess.STARTF_USESHOWWINDOW

proc = subprocess.Popen(
    [node_path, cli_entry, "-p", "--output-format", "stream-json", "--verbose", "--session-id", session_id],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, env=env, cwd=os.path.dirname(os.path.abspath(__file__)),
    startupinfo=si, creationflags=subprocess.CREATE_NO_WINDOW,
)
proc.stdin.write("Hi, say hello in one word")
proc.stdin.close()

line_count = 0
for line in proc.stdout:
    trimmed = line.strip()
    if not trimmed:
        continue
    line_count += 1
    try:
        parsed = json.loads(trimmed)
        evt = parsed.get("type", "?")
        if evt == "stream_event":
            sub = parsed.get("event", {}).get("type", "?")
            delta = parsed.get("event", {}).get("delta", {}).get("type", "?")
            text = parsed.get("event", {}).get("delta", {}).get("text", "")
            if text:
                print(f"  [DELTA] {text[:50]}")
            else:
                print(f"  [STREAM] {sub}.{delta}")
        elif evt == "system":
            sub_type = parsed.get("subtype", "?")
            print(f"  [SYSTEM] {sub_type}")
        elif evt == "assistant":
            msg = parsed.get("message", {})
            content = msg.get("content", [])
            texts = [b.get("text", "") for b in content if b.get("type") == "text"]
            total_len = sum(len(t) for t in texts)
            print(f"  [ASSISTANT] texts={len(texts)} len={total_len}")
        elif evt == "result":
            is_err = parsed.get("is_error")
            result_str = str(parsed.get("result", ""))[:100]
            print(f"  [RESULT] is_error={is_err} result={result_str}")
        else:
            print(f"  [{evt}] {trimmed[:100]}")
    except Exception:
        print(f"  [RAW] {trimmed[:100]}")
    if line_count > 100:
        break

proc.wait(timeout=60)
stderr = proc.stderr.read()
if stderr:
    print(f"  [STDERR] {stderr[:500]}")
print(f"RC={proc.returncode} LINES={line_count}")
