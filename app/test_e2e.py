#!/usr/bin/env python3
import sys, os, json, subprocess, time, threading

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.dirname(os.path.abspath(__file__)))

from backend import OllamaProxyServer, ClaudeCliRunner

print("=" * 60)
print("  端到端测试：模拟后端调用 CLI")
print("=" * 60)

proxy = OllamaProxyServer()
ollama_model = "llama3:latest"
proxy_port = proxy.start("http://127.0.0.1:11434", ollama_model)
print(f"[1] Ollama代理已启动 端口={proxy_port}")

env_overrides = {
    "ANTHROPIC_BASE_URL": f"http://127.0.0.1:{proxy_port}",
    "ANTHROPIC_API_KEY": "ollama-local",
    "ANTHROPIC_AUTH_TOKEN": "ollama-local",
    "ANTHROPIC_MODEL": ollama_model,
    "MODEL_PROVIDER": "anthropic",
}

runner = ClaudeCliRunner(
    os.path.dirname(os.path.abspath(__file__)),
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "nodejs", "node-v24.11.1-win-x64"),
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "bun", "bun-windows-x64"),
)

received_text = ""
def on_delta(text):
    global received_text
    received_text += text
    print(f"  [DELTA] \"{text}\"")

def on_log(msg, color="#888"):
    print(f"  [LOG] {msg}")

print(f"[2] 调用CLI...")
result = runner.run(
    prompt="Say hello in one word",
    session_id=str(__import__("uuid").uuid4()),
    model=ollama_model,
    is_resuming=False,
    workspace_path=os.path.dirname(os.path.abspath(__file__)),
    env_overrides=env_overrides,
    on_delta=on_delta,
    on_log=on_log,
)

print(f"\n[3] 结果:")
print(f"  ok={result.get('ok')}")
print(f"  text={result.get('text', '')[:200]}")
print(f"  received_text_len={len(received_text)}")
print(f"  received_text={received_text[:200]}")

if received_text:
    print("\n✅ 成功！前端应该能收到文本")
else:
    print("\n❌ 失败！前端会显示[模型未返回文本]")

print("\n[4] 检查调试日志:")
log_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cli_debug.log")
if os.path.exists(log_path):
    with open(log_path, "r", encoding="utf-8") as f:
        print(f.read())
else:
    print("  日志文件不存在")
