import os
import sys
import json
import time
import shutil
import subprocess
import threading
import http.server
import http.client
import socketserver
import socket
import urllib.request
import urllib.error
import urllib.parse
from typing import Optional
from pathlib import Path

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"

OLLAMA_AGENT_MAX_STEPS = 10
TOOL_TEXT_LIMIT = 12000
COMMAND_OUTPUT_LIMIT = 8000

TOOL_CALLING_MODEL_PATTERNS = [
    "qwen3", "qwen2.5", "qwen2-",
    "llama3.1", "llama3.2", "llama3.3", "llama4",
    "mistral", "mixtral",
    "command-r",
    "gemma2", "gemma3",
    "phi3", "phi4",
    "deepseek-r1", "deepseek-coder-v2", "deepseek-v3",
    "snowflake-arctic",
    "cogito",
    "devstral",
]

UV_PYTHON_VERSION = "3.12"

MIRROR_SOURCES = {
    "official": {
        "label": "🌐 官方源",
        "node": "https://nodejs.org/dist/",
        "github_proxy": "",
        "uv_python_mirror": "",
        "pypi_index": "https://pypi.org/simple/",
    },
    "china": {
        "label": "🇨🇳 国内镜像",
        "node": "https://npmmirror.com/mirrors/node/",
        "github_proxy": "https://gh-proxy.com/",
        "uv_python_mirror": "https://registry.npmmirror.com/-/binary/python-build-standalone/",
        "pypi_index": "https://pypi.tuna.tsinghua.edu.cn/simple/",
    },
}

ZHIPU_DEFAULT_BASE_URL = "https://open.bigmodel.cn/api/paas/v4"

ZHIPU_STATIC_MODELS = [
    {"id": "glm-5.1", "name": "GLM-5.1", "desc": "新一代旗舰推理模型，754B参数/200K上下文，长程任务支持8小时", "toolSupport": True},
    {"id": "glm-5", "name": "GLM-5", "desc": "旗舰基座模型，面向Agentic Engineering，对标Claude Opus 4.5", "toolSupport": True},
    {"id": "glm-5-turbo", "name": "GLM-5-Turbo", "desc": "面向OpenClaw场景优化的基座模型，智能体调用能力强", "toolSupport": True},
    {"id": "glm-4.7", "name": "GLM-4.7", "desc": "GLM-4.7稳定版本，通用能力强", "toolSupport": True},
    {"id": "glm-4.6", "name": "GLM-4.6", "desc": "GLM-4.6稳定版，高效推理", "toolSupport": True},
    {"id": "glm-4.7-flash", "name": "GLM-4.7-Flash", "desc": "免费模型，轻量快速，免费无限", "toolSupport": True},
    {"id": "glm-4-plus", "name": "GLM-4-Plus", "desc": "旗舰模型，最强推理能力", "toolSupport": True},
    {"id": "glm-4-flash", "name": "GLM-4-Flash", "desc": "免费模型，快速响应（推荐）", "toolSupport": True},
    {"id": "glm-4-flash-250414", "name": "GLM-4-Flash-250414", "desc": "免费模型，最新版本", "toolSupport": True},
    {"id": "glm-4-air", "name": "GLM-4-Air", "desc": "均衡模型，性价比高", "toolSupport": True},
    {"id": "glm-4-air-0111", "name": "GLM-4-Air-0111", "desc": "均衡模型，优化版本", "toolSupport": True},
    {"id": "glm-4-long", "name": "GLM-4-Long", "desc": "长文本模型，128K上下文", "toolSupport": True},
    {"id": "glm-4v", "name": "GLM-4V", "desc": "视觉模型，支持图片理解", "toolSupport": False},
    {"id": "glm-4v-plus", "name": "GLM-4V-Plus", "desc": "增强视觉模型", "toolSupport": False},
    {"id": "glm-4", "name": "GLM-4", "desc": "标准模型", "toolSupport": True},
    {"id": "glm-3-turbo", "name": "GLM-3-Turbo", "desc": "轻量快速模型", "toolSupport": True},
]

API_SERVICE_REGISTRY = {
    "qwen2api": {
        "label": "千问",
        "default_port": 7777,
        "code_dir": "api/qwen2api",
        "entry_module": "qwen2api.main:app",
        "service_type": "qwen",
        "data_dir": "api/qwen2api",
        "model_prefixes": ["qwen"],
    },
    "zhipu2api": {
        "label": "智谱",
        "default_port": 7780,
        "code_dir": "api/zhipu2api",
        "entry_module": "zhipu2api.main:app",
        "service_type": "zhipu",
        "data_dir": "api/zhipu2api",
        "model_prefixes": ["glm", "chatglm"],
    },
}

_qwen2api_proc = None
_zhipu2api_proc = None

_login_state = {
    "busy": False,
    "done": False,
    "ok": False,
    "email": "",
    "error": "",
}

_register_state = {
    "busy": False,
    "done": False,
    "ok": False,
    "email": "",
    "error": "",
    "log_offset": 0,
    "log_lines": [],
}


def _exe_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    else:
        return str(Path(__file__).resolve().parent.parent.parent)

def _app_dir() -> str:
    return os.path.join(_exe_dir(), "app")

def _data_dir() -> str:
    return _ensure_dir(os.path.join(_exe_dir(), "data"))

def _temp_dir() -> str:
    return _ensure_dir(os.path.join(_exe_dir(), "temp"))

def _ensure_dir(path: str) -> str:
    os.makedirs(path, exist_ok=True)
    return path

def _get_api_code_dir(service_name: str) -> str:
    return os.path.join(_app_dir(), "api", service_name)

def _get_api_data_dir(service_name: str) -> str:
    return _ensure_dir(os.path.join(_data_dir(), "api", service_name))

def _get_api_venv_dir(service_name: str = "") -> str:
    return os.path.join(_data_dir(), ".venv")

def _get_log_path(log_name: str) -> str:
    return os.path.join(_temp_dir(), "logs", log_name)

def _get_cache_dir(cache_type: str) -> str:
    return _ensure_dir(os.path.join(_temp_dir(), "cache", cache_type))

def _get_debug_path(debug_file: str) -> str:
    return os.path.join(_temp_dir(), "debug", debug_file)

def _get_venv_python(venv_dir: str) -> str:
    if os.name == "nt":
        return os.path.join(venv_dir, "Scripts", "python.exe")
    return os.path.join(venv_dir, "bin", "python")

def _check_deps_installed(venv_python: str) -> bool:
    try:
        result = subprocess.run(
            [venv_python, "-c", "import fastapi"],
            capture_output=True,
            timeout=5
        )
        return result.returncode == 0
    except:
        return False

def _uv_exe() -> str:
    return os.path.join(_app_dir(), "uv", "uv.exe")

def _uv_python_dir() -> str:
    return os.path.join(_app_dir(), "python")

def _load_mirror_key() -> str:
    try:
        fp = os.path.join(_data_dir(), "mirror_source.json")
        if os.path.isfile(fp):
            with open(fp, "r", encoding="utf-8") as f:
                key = f.read().strip()
            if key in MIRROR_SOURCES:
                return key
    except Exception:
        pass
    return "china"

def _uv_env() -> dict:
    env = os.environ.copy()
    env["UV_PYTHON_INSTALL_DIR"] = _uv_python_dir()
    env["UV_PYTHON_DOWNLOADS"] = "auto"
    uv_cache_dir = os.path.join(_temp_dir(), "cache", "uv_cache")
    os.makedirs(uv_cache_dir, exist_ok=True)
    env["UV_CACHE_DIR"] = uv_cache_dir
    mirror = MIRROR_SOURCES.get(_load_mirror_key(), MIRROR_SOURCES["china"])
    python_mirror = mirror.get("uv_python_mirror", "")
    if python_mirror:
        env["UV_PYTHON_INSTALL_MIRROR"] = python_mirror
    pypi_index = mirror.get("pypi_index", "")
    if pypi_index:
        env["UV_INDEX_URL"] = pypi_index
        env["UV_DEFAULT_INDEX"] = pypi_index
    return env

def _extract_port(base_url: str = "", default: int = 7777) -> int:
    base = _default_base(base_url)
    try:
        parsed = urllib.parse.urlparse(base)
        return parsed.port or default
    except Exception:
        return default

def _default_base(base_url: str = "") -> str:
    return (base_url or "").strip() or "http://127.0.0.1:7777"

def _stop_port_service(port: int) -> bool:
    stopped = False
    try:
        r = subprocess.run(
            ["netstat", "-ano"],
            capture_output=True, text=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        for line in r.stdout.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                parts = line.split()
                pid = int(parts[-1])
                if pid and pid != os.getpid():
                    try:
                        import signal
                        os.kill(pid, signal.SIGTERM)
                        stopped = True
                    except Exception:
                        try:
                            subprocess.run(
                                ["taskkill", "/F", "/PID", str(pid)],
                                capture_output=True, timeout=10,
                                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                            )
                            stopped = True
                        except Exception:
                            pass
                break
    except Exception:
        pass
    return stopped


def fetch_json_with_timeout(url: str, headers: dict = None, timeout_ms: int = 15000, method: str = "GET", body: bytes = None):
    try:
        req = urllib.request.Request(url, method=method, headers=headers or {})
        if body:
            req.data = body
        resp = urllib.request.urlopen(req, timeout=timeout_ms / 1000)
        text = resp.read().decode("utf-8")
        data = None
        try:
            data = json.loads(text)
        except:
            pass
        return {"ok": True, "status": resp.status, "data": data, "text": text}
    except urllib.error.HTTPError as e:
        text = ""
        try:
            text = e.read().decode("utf-8")
        except:
            pass
        data = None
        try:
            data = json.loads(text)
        except:
            pass
        return {"ok": False, "status": e.code, "data": data, "text": text}
    except Exception as e:
        return {"ok": False, "status": 0, "data": None, "text": str(e)}


def normalize_model_entries(lst: list, provider: str) -> list:
    if not isinstance(lst, list):
        return []
    result = []
    for item in lst[:120]:
        mid = str(item.get("id", "") or "").strip()
        if not mid:
            continue
        name = str(item.get("name", "") or item.get("display_name", "") or mid).strip()
        result.append({"id": mid, "name": name, "provider": provider})
    return result


def guess_tool_support_by_name(name: str) -> bool:
    lower = (name or "").lower()
    if lower.endswith(":cloud"):
        return False
    return any(p in lower for p in TOOL_CALLING_MODEL_PATTERNS)


def fetch_ollama_capabilities(base_url: str, model_name: str, timeout_ms: int = 5000) -> Optional[dict]:
    try:
        url = f"{base_url.rstrip('/')}/api/show"
        body = json.dumps({"name": model_name}).encode("utf-8")
        result = fetch_json_with_timeout(
            url,
            headers={"Content-Type": "application/json"},
            timeout_ms=min(timeout_ms, 5000),
            method="POST",
            body=body,
        )
        if not result["ok"]:
            return None
        data = result.get("data") or {}
        caps = data.get("capabilities")
        if caps and isinstance(caps, dict):
            return caps
        caps = (data.get("model_info") or {}).get("capabilities")
        if caps and isinstance(caps, dict):
            return caps
        return None
    except:
        return None


def list_openrouter_models(timeout_ms: int = 15000) -> dict:
    result = fetch_json_with_timeout("https://openrouter.ai/api/v1/models", timeout_ms=timeout_ms)
    if not result["ok"]:
        return {"ok": False, "error": f"OpenRouter API failed ({result['status']})"}
    models = normalize_model_entries((result.get("data") or {}).get("data", []), "openrouter")
    return {"ok": True, "models": models}


def list_zhipu_models(api_key: str = "", base_url: str = "", timeout_ms: int = 15000) -> dict:
    base = (base_url or "").strip() or ZHIPU_DEFAULT_BASE_URL
    models = []

    if api_key and api_key.strip():
        try:
            url = f"{base.rstrip('/')}/models"
            headers = {
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key.strip()}",
            }
            result = fetch_json_with_timeout(url, headers=headers, timeout_ms=timeout_ms)
            if result["ok"]:
                raw_models = ((result.get("data") or {}).get("data", []))[:120]
                for m in raw_models:
                    model_id = m.get("id", m.get("name", ""))
                    if model_id:
                        models.append({
                            "id": model_id,
                            "name": m.get("name", model_id),
                            "provider": "zhipu",
                            "toolSupport": True,
                            "size": "",
                            "family": m.get("owned_by", "zhipu"),
                            "paramCount": "",
                            "loadable": True,
                            "healthError": "",
                        })
        except Exception:
            pass

    if not models:
        for m in ZHIPU_STATIC_MODELS:
            models.append({
                "id": m["id"],
                "name": m["name"],
                "provider": "zhipu",
                "toolSupport": m.get("toolSupport", True),
                "size": "",
                "family": "zhipu",
                "paramCount": "",
                "loadable": True,
                "healthError": "",
                "desc": m.get("desc", ""),
            })

    return {"ok": True, "models": models}


def check_zhipu_api(api_key: str = "", base_url: str = "", timeout_ms: int = 10000) -> dict:
    base = (base_url or "").strip() or ZHIPU_DEFAULT_BASE_URL
    if not api_key or not api_key.strip():
        return {"ok": False, "error": "请输入智谱 API Key"}
    try:
        url = f"{base.rstrip('/')}/models"
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key.strip()}",
        }
        result = fetch_json_with_timeout(url, headers=headers, timeout_ms=timeout_ms)
        if result["ok"]:
            return {"ok": True, "running": True, "message": "智谱 API 连接成功"}
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"连接失败({result.get('status', '?')}): {err}".strip()}
    except Exception as e:
        return {"ok": False, "error": f"连接异常: {str(e)[:200]}"}


def list_anthropic_models(api_key: str, timeout_ms: int = 15000) -> dict:
    if not api_key or not api_key.strip():
        return {"ok": False, "error": "Anthropic API key is required."}
    result = fetch_json_with_timeout(
        "https://api.anthropic.com/v1/models",
        headers={
            "x-api-key": api_key.strip(),
            "anthropic-version": "2023-06-01",
        },
        timeout_ms=timeout_ms,
    )
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"Anthropic API failed ({result['status']}) {err}".strip()}
    models = normalize_model_entries((result.get("data") or {}).get("data", []), "anthropic")
    return {"ok": True, "models": models}


def check_ollama_model_health(base_url: str, model_name: str, timeout_ms: int = 30000) -> dict:
    result = {"loadable": True, "tool_support": None, "error": ""}
    try:
        url = f"{base_url.rstrip('/')}/v1/chat/completions"
        body = json.dumps({
            "model": model_name,
            "messages": [{"role": "user", "content": "hi"}],
            "stream": False,
            "max_tokens": 5,
        }).encode("utf-8")
        resp = fetch_json_with_timeout(
            url,
            headers={"Content-Type": "application/json"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if resp.get("ok"):
            result["loadable"] = True
        else:
            status = resp.get("status", 0)
            err_text = (resp.get("text", "") or "")[:300]
            if status == 500 and "unable to load" in err_text.lower():
                result["loadable"] = False
                result["error"] = "模型文件损坏或无法加载"
            elif status == 400 and "does not support tools" in err_text:
                result["loadable"] = True
                result["tool_support"] = False
            elif status >= 400:
                result["loadable"] = False
                result["error"] = f"API错误({status})"
            else:
                result["loadable"] = False
                result["error"] = err_text[:100] or "未知错误"
    except Exception as e:
        result["loadable"] = False
        result["error"] = str(e)[:100]
    return result


def list_ollama_models(base_url: str, timeout_ms: int = 15000, check_health: bool = False) -> dict:
    base = (base_url or "").strip() or "http://127.0.0.1:11434"
    url = f"{base.rstrip('/')}/api/tags"
    result = fetch_json_with_timeout(url, timeout_ms=timeout_ms)
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"Ollama API failed ({result['status']}) {err}".strip()}
    raw_models = ((result.get("data") or {}).get("models", []))[:120]

    cap_results = [None] * len(raw_models)
    health_results = [None] * len(raw_models) if check_health else []

    def _fetch_cap(idx, m):
        cap_results[idx] = fetch_ollama_capabilities(base, m.get("name", ""), timeout_ms)

    def _fetch_health(idx, m):
        health_results[idx] = check_ollama_model_health(base, m.get("name", ""), min(timeout_ms, 60000))

    threads = []
    for i, m in enumerate(raw_models):
        t = threading.Thread(target=_fetch_cap, args=(i, m), daemon=True)
        t.start()
        threads.append(t)
        if check_health:
            ht = threading.Thread(target=_fetch_health, args=(i, m), daemon=True)
            ht.start()
            threads.append(ht)
    for t in threads:
        t.join(timeout=120 if check_health else 10)

    models = []
    for i, m in enumerate(raw_models):
        tool_support = guess_tool_support_by_name(m.get("name", ""))
        cap = cap_results[i]
        cap_resolved = False
        if cap:
            if isinstance(cap.get("tool_calling"), bool):
                tool_support = cap["tool_calling"]
                cap_resolved = True
            elif isinstance(cap.get("input_tool"), bool):
                tool_support = cap["input_tool"]
                cap_resolved = True
            if not cap_resolved and len(cap) > 0:
                tool_support = False
        loadable = True
        health_error = ""
        if check_health and health_results:
            hr = health_results[i]
            if hr:
                loadable = hr.get("loadable", True)
                health_error = hr.get("error", "")
                if hr.get("tool_support") is False:
                    tool_support = False
        size_bytes = m.get("size", 0) or 0
        size_str = ""
        if size_bytes > 0:
            size_gb = size_bytes / (1024 * 1024 * 1024)
            if size_gb >= 1:
                size_str = f"{size_gb:.1f}GB"
            else:
                size_mb = size_bytes / (1024 * 1024)
                size_str = f"{size_mb:.0f}MB"
        models.append({
            "id": m.get("name", ""),
            "name": m.get("name", ""),
            "provider": "ollama",
            "toolSupport": tool_support,
            "size": size_str,
            "family": m.get("details", {}).get("family", "") if isinstance(m.get("details"), dict) else "",
            "paramCount": m.get("details", {}).get("parameter_size", "") if isinstance(m.get("details"), dict) else "",
            "loadable": loadable,
            "healthError": health_error,
        })
    return {"ok": True, "models": models}


def list_api_models(base_url: str, api_key: str = "", timeout_ms: int = 15000) -> dict:
    base = _default_base(base_url)
    url = f"{base.rstrip('/')}/v1/models"
    headers = {"Content-Type": "application/json"}
    if api_key and api_key.strip():
        headers["Authorization"] = f"Bearer {api_key.strip()}"
    result = fetch_json_with_timeout(url, headers=headers, timeout_ms=timeout_ms)
    if not result["ok"]:
        err = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"API 服务连接失败 ({result['status']}) {err}".strip()}
    raw_models = ((result.get("data") or {}).get("data", []))[:120]
    models = []
    for m in raw_models:
        model_id = m.get("id", m.get("name", ""))
        models.append({
            "id": model_id,
            "name": m.get("name", model_id),
            "provider": "api",
            "toolSupport": True,
            "size": "",
            "family": m.get("owned_by", ""),
            "paramCount": "",
            "loadable": True,
            "healthError": "",
        })
    if not models:
        models.append({
            "id": "qwen3.6-plus",
            "name": "qwen3.6-plus",
            "provider": "api",
            "toolSupport": True,
            "size": "",
            "family": "qwen",
            "paramCount": "",
            "loadable": True,
            "healthError": "",
        })
    return {"ok": True, "models": models}


def check_api_service(base_url: str, timeout_ms: int = 5000) -> dict:
    base = _default_base(base_url)
    try:
        result = fetch_json_with_timeout(f"{base.rstrip('/')}/healthz", timeout_ms=timeout_ms)
        if result["ok"]:
            info = {"ok": True, "running": True}
            try:
                models_result = fetch_json_with_timeout(
                    f"{base.rstrip('/')}/v1/models",
                    headers={"Content-Type": "application/json", "Authorization": "Bearer admin"},
                    timeout_ms=timeout_ms,
                )
                if models_result["ok"]:
                    models_data = (models_result.get("data") or {}).get("data") or []
                    if models_data:
                        owners = set(m.get("owned_by", "") for m in models_data)
                        if "zhipu" in owners:
                            info["serviceType"] = "zhipu"
                        elif "qwen" in owners:
                            info["serviceType"] = "qwen"
                        else:
                            info["serviceType"] = "unknown"
                    else:
                        info["serviceType"] = "unknown"
            except Exception:
                pass
            try:
                acct_result = fetch_json_with_timeout(
                    f"{base.rstrip('/')}/api/admin/accounts",
                    headers={"Content-Type": "application/json", "Authorization": "Bearer admin"},
                    timeout_ms=timeout_ms,
                )
                if acct_result["ok"]:
                    accounts = (acct_result.get("data") or {}).get("accounts", [])
                    info["accountCount"] = len(accounts)
                    if len(accounts) == 0:
                        info["warning"] = "未添加上游账号，AI 对话将返回 500 错误。请在管理台添加 chat.qwen.ai 的账号 Token。"
            except Exception:
                pass
            return info
    except:
        pass
    return {"ok": True, "running": False}


def _ensure_api_service(base_url: str = "", admin_key: str = "admin") -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"

    def _check_service():
        try:
            urllib.request.urlopen(f"{base.rstrip('/')}/healthz", timeout=3)
            return True
        except Exception:
            pass
        try:
            parsed = urllib.parse.urlparse(base)
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.settimeout(2)
            s.connect((parsed.hostname or "127.0.0.1", parsed.port or _extract_port(base_url, 7777)))
            s.close()
            return True
        except Exception:
            return False

    if _check_service():
        return {"ok": True, "was_running": True}
    port = _extract_port(base_url, 7777)
    start_result = start_qwen2api("", port, ak)
    if not start_result.get("ok"):
        return {"ok": False, "error": f"API 服务未运行且自动启动失败: {start_result.get('message', start_result.get('error', ''))}"}
    for _ in range(20):
        time.sleep(1)
        if _check_service():
            return {"ok": True, "was_running": False}
    return {"ok": False, "error": "API 服务启动超时"}


def add_qwen_account(base_url: str, token: str, admin_key: str = "", timeout_ms: int = 15000) -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    if not token or not token.strip():
        return {"ok": False, "error": "Token 不能为空"}
    try:
        body = json.dumps({"token": token.strip()}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/accounts",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                return {"ok": True, "email": data.get("email", "")}
            return {"ok": False, "error": data.get("error", "添加账户失败")}
        err_text = (result.get("text", "") or "")[:200]
        return {"ok": False, "error": f"请求失败 ({result.get('status', '?')}) {err_text}".strip()}
    except Exception as e:
        return {"ok": False, "error": f"添加账户异常: {e}"}


def list_qwen_accounts(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    try:
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/accounts",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
        )
        if result["ok"]:
            data = result.get("data") or {}
            accounts = data.get("accounts", [])
            sticky = data.get("sticky_email") or ""
            return {"ok": True, "accounts": accounts, "count": len(accounts), "sticky_email": sticky}
        return {"ok": False, "error": "获取账户列表失败"}
    except Exception as e:
        return {"ok": False, "error": f"获取账户列表异常: {e}"}


def delete_qwen_account(base_url: str, email: str, admin_key: str = "") -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    if not email or not email.strip():
        return {"ok": False, "error": "邮箱不能为空"}
    try:
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/{urllib.parse.quote(email.strip())}",
            headers={"Authorization": f"Bearer {ak}"},
            method="DELETE",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return {"ok": data.get("ok", True)}
    except Exception as e:
        return {"ok": False, "error": f"删除账户失败: {e}"}


def set_sticky_account(base_url: str, email: str, admin_key: str = "") -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    if not email or not email.strip():
        return {"ok": False, "error": "邮箱不能为空"}
    try:
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/{urllib.parse.quote(email.strip())}/set-sticky",
            headers={"Authorization": f"Bearer {ak}", "Content-Type": "application/json"},
            method="POST",
            data=b"{}",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data
    except Exception as e:
        return {"ok": False, "error": f"设置优先账户失败: {e}"}


def clear_sticky_account(base_url: str, admin_key: str = "") -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    try:
        req = urllib.request.Request(
            f"{base.rstrip('/')}/api/admin/accounts/clear-sticky",
            headers={"Authorization": f"Bearer {ak}", "Content-Type": "application/json"},
            method="POST",
            data=b"{}",
        )
        with urllib.request.urlopen(req, timeout=10000) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data
    except Exception as e:
        return {"ok": False, "error": f"清除优先账户失败: {e}"}


def _do_login_background(base_url: str, admin_key: str, email: str, password: str):
    global _login_state
    try:
        body = json.dumps({"email": email, "password": password}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/login",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {admin_key}"},
            timeout_ms=120000,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                _login_state["ok"] = True
                _login_state["email"] = data.get("email", "")
                _login_state["error"] = ""
            else:
                _login_state["ok"] = False
                _login_state["error"] = data.get("error", "登录失败")
        else:
            err_text = (result.get("text", "") or "")[:300]
            _login_state["ok"] = False
            _login_state["error"] = f"请求失败 ({result.get('status', '?')}) {err_text}".strip()
    except Exception as e:
        _login_state["ok"] = False
        _login_state["error"] = f"登录异常: {e}"
    finally:
        _login_state["done"] = True
        _login_state["busy"] = False


def start_qwen_login(base_url: str, email: str, password: str, admin_key: str = "") -> dict:
    global _login_state
    if _login_state["busy"]:
        return {"ok": False, "error": "登录正在进行中，请稍候"}
    if not email or not password:
        return {"ok": False, "error": "邮箱和密码不能为空"}
    ak = (admin_key or "").strip() or "admin"
    base = _default_base(base_url)
    ensure = _ensure_api_service(base, ak)
    if not ensure.get("ok"):
        return {"ok": False, "error": ensure.get("error", "API 服务不可用")}
    _login_state = {
        "busy": True,
        "done": False,
        "ok": False,
        "email": "",
        "error": "",
    }
    t = threading.Thread(target=_do_login_background, args=(base, ak, email, password), daemon=True)
    t.start()
    return {"ok": True, "message": "登录已启动"}


def poll_qwen_login() -> dict:
    global _login_state
    return {
        "busy": _login_state["busy"],
        "done": _login_state["done"],
        "success": _login_state["ok"],
        "email": _login_state.get("email", ""),
        "error": _login_state.get("error", ""),
    }


def _get_qwen2api_log_path() -> str:
    log_path = _get_log_path("qwen2api.log")
    if os.path.isfile(log_path):
        return log_path
    old_dir = Path(_app_dir()) / "qwen2api" / "data"
    if old_dir.exists():
        return str(old_dir / "qwen2api.log")
    return ""


def _read_qwen2api_log_tail(n: int = 50, offset: int = 0) -> dict:
    log_path = _get_qwen2api_log_path()
    if not log_path or not os.path.isfile(log_path):
        return {"ok": True, "lines": [], "total": 0}
    try:
        with open(log_path, "r", encoding="utf-8", errors="replace") as f:
            all_lines = f.readlines()
        total = len(all_lines)
        start = max(offset, 0)
        selected = all_lines[start:start + n]
        return {"ok": True, "lines": [l.rstrip("\n\r") for l in selected], "total": total, "nextOffset": start + len(selected)}
    except Exception as e:
        return {"ok": False, "error": str(e), "lines": [], "total": 0}


def _do_register_background(base_url: str, admin_key: str, custom_email: str = "", custom_password: str = "", custom_username: str = ""):
    global _register_state
    try:
        reg_body = {}
        if custom_email:
            reg_body["email"] = custom_email
        if custom_password:
            reg_body["password"] = custom_password
        if custom_username:
            reg_body["username"] = custom_username
        body = json.dumps(reg_body).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/register",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {admin_key}"},
            timeout_ms=180000,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            if data.get("ok"):
                _register_state["ok"] = True
                _register_state["email"] = data.get("email", "")
                _register_state["error"] = ""
            else:
                _register_state["ok"] = False
                _register_state["error"] = data.get("error", "自动注册失败")
        else:
            err_text = (result.get("text", "") or "")[:300]
            _register_state["ok"] = False
            _register_state["error"] = f"请求失败 ({result.get('status', '?')}) {err_text}".strip()
    except Exception as e:
        _register_state["ok"] = False
        _register_state["error"] = f"自动注册异常: {e}"
    finally:
        _register_state["done"] = True
        _register_state["busy"] = False


def start_qwen_register(base_url: str, admin_key: str = "", custom_email: str = "", custom_password: str = "", custom_username: str = "") -> dict:
    global _register_state
    if _register_state["busy"]:
        return {"ok": False, "error": "注册正在进行中，请稍候"}
    ak = (admin_key or "").strip() or "admin"
    base = _default_base(base_url)
    ensure = _ensure_api_service(base, ak)
    if not ensure.get("ok"):
        return {"ok": False, "error": ensure.get("error", "API 服务不可用")}
    _register_state = {
        "busy": True,
        "done": False,
        "ok": False,
        "email": "",
        "error": "",
        "log_offset": 0,
        "log_lines": [],
    }
    log_path = _get_qwen2api_log_path()
    if log_path and os.path.isfile(log_path):
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                _register_state["log_offset"] = len(f.readlines())
        except Exception:
            _register_state["log_offset"] = 0
    t = threading.Thread(target=_do_register_background, args=(base, ak, custom_email, custom_password, custom_username), daemon=True)
    t.start()
    return {"ok": True, "message": "注册已启动"}


def poll_qwen_register() -> dict:
    global _register_state
    log_path = _get_qwen2api_log_path()
    new_lines = []
    if log_path and os.path.isfile(log_path):
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                all_lines = f.readlines()
            offset = _register_state.get("log_offset", 0)
            if offset < len(all_lines):
                new_lines = [l.rstrip("\n\r") for l in all_lines[offset:]]
                _register_state["log_offset"] = len(all_lines)
            _register_state["log_lines"].extend(new_lines)
        except Exception:
            pass
    register_lines = [l for l in new_lines if "[Register]" in l or "[注册]" in l or "register" in l.lower()]
    return {
        "ok": True,
        "busy": _register_state["busy"],
        "done": _register_state["done"],
        "success": _register_state["ok"],
        "email": _register_state.get("email", ""),
        "error": _register_state.get("error", ""),
        "newLines": register_lines,
        "allNewLines": new_lines,
    }


def fetch_api_key(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    base = _default_base(base_url)
    ak = (admin_key or "").strip() or "admin"
    errors = []
    try:
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/keys",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
        )
        if result["ok"]:
            data = result.get("data") or {}
            keys = data.get("keys", [])
            if keys:
                return {"ok": True, "key": keys[0]}
        else:
            errors.append(f"GET /api/admin/keys → HTTP {result.get('status', '?')}: {result.get('text', '')[:200]}")
    except Exception as e:
        errors.append(f"GET /api/admin/keys 异常: {e}")
    try:
        body = json.dumps({}).encode("utf-8")
        result = fetch_json_with_timeout(
            f"{base.rstrip('/')}/api/admin/keys",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {ak}"},
            timeout_ms=timeout_ms,
            method="POST",
            body=body,
        )
        if result["ok"]:
            data = result.get("data") or {}
            key = data.get("key", "")
            if key:
                return {"ok": True, "key": key}
            else:
                errors.append(f"POST /api/admin/keys 返回无 key 字段: data={json.dumps(data)[:200]}")
        else:
            errors.append(f"POST /api/admin/keys → HTTP {result.get('status', '?')}: {result.get('text', '')[:200]}")
    except Exception as e:
        errors.append(f"POST /api/admin/keys 异常: {e}")
    admin_url = base.rstrip("/")
    error_detail = "; ".join(errors) if errors else "未知错误"
    return {"ok": False, "error": f"获取 API Key 失败: {error_detail}", "adminUrl": admin_url}


def get_api_service_info(service_name: str) -> dict:
    if service_name not in API_SERVICE_REGISTRY:
        return {"ok": False, "error": f"未知服务: {service_name}"}
    reg = API_SERVICE_REGISTRY[service_name]
    return {
        "ok": True,
        "name": service_name,
        "label": reg["label"],
        "defaultPort": reg["default_port"],
        "serviceType": reg["service_type"],
        "codeDir": os.path.join(_app_dir(), reg["code_dir"]),
        "dataDir": _get_api_data_dir(reg["data_dir"].split("/")[-1]),
        "venvDir": _get_api_venv_dir(),
        "venvPython": _get_venv_python(_get_api_venv_dir()),
    }


def list_api_services() -> list:
    result = []
    for name, reg in API_SERVICE_REGISTRY.items():
        info = get_api_service_info(name)
        info.pop("ok", None)
        result.append(info)
    return result


def resolve_api_base_url(model: str) -> str:
    model_lower = model.lower()
    for name, reg in API_SERVICE_REGISTRY.items():
        for prefix in reg.get("model_prefixes", []):
            if model_lower.startswith(prefix):
                return f"http://127.0.0.1:{reg['default_port']}"
    if model_lower.startswith("glm"):
        return f"http://127.0.0.1:{API_SERVICE_REGISTRY['zhipu2api']['default_port']}"
    return f"http://127.0.0.1:{API_SERVICE_REGISTRY['qwen2api']['default_port']}"


def start_all_api_services() -> dict:
    results = {}
    all_ok = True
    for name, reg in API_SERVICE_REGISTRY.items():
        base_url = f"http://127.0.0.1:{reg['default_port']}"
        chk = check_api_service(base_url)
        if chk.get("running") and chk.get("serviceType") == reg["service_type"]:
            results[name] = {"ok": True, "message": f"{reg['label']}服务已运行", "alreadyRunning": True}
            continue
        if chk.get("running") and chk.get("serviceType") != reg["service_type"]:
            _stop_port_service(reg["default_port"])
            time.sleep(1)
        if name == "qwen2api":
            r = start_qwen2api(port=reg["default_port"])
        elif name == "zhipu2api":
            r = start_zhipu2api(port=reg["default_port"])
        else:
            results[name] = {"ok": False, "error": f"未知服务: {name}"}
            all_ok = False
            continue
        if r.get("ok"):
            for _ in range(15):
                time.sleep(1)
                chk2 = check_api_service(base_url)
                if chk2.get("running") and chk2.get("serviceType") == reg["service_type"]:
                    results[name] = {"ok": True, "message": f"{reg['label']}服务已启动"}
                    break
            else:
                results[name] = {"ok": False, "error": f"{reg['label']}服务启动超时"}
                all_ok = False
        else:
            results[name] = r
            all_ok = False
    return {"ok": all_ok, "services": results}


def list_all_models() -> dict:
    all_models = []
    for name, reg in API_SERVICE_REGISTRY.items():
        base_url = f"http://127.0.0.1:{reg['default_port']}"
        try:
            result = fetch_json_with_timeout(
                f"{base_url}/v1/models",
                headers={"Content-Type": "application/json", "Authorization": "Bearer admin"},
                timeout_ms=5000,
            )
            if result.get("ok"):
                models_data = (result.get("data") or {}).get("data") or []
                all_models.extend(models_data)
        except Exception:
            pass
    return {"ok": True, "models": all_models}


def check_environment() -> dict:
    results = {}

    def _check_python():
        try:
            v = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
            return {"ok": True, "label": "Python 运行时", "version": v, "description": "Python 解释器，核心运行环境", "dependencies": ["Python 3.8+"]}
        except Exception as e:
            return {"ok": False, "label": "Python 运行时", "error": str(e), "fix": "请重新安装应用程序"}

    def _check_uv():
        try:
            uv = _uv_exe()
            if os.path.isfile(uv):
                r = subprocess.run([uv, "--version"], capture_output=True, timeout=5, text=True)
                v = r.stdout.strip() if r.returncode == 0 else "已安装"
                return {"ok": True, "label": "UV 包管理器", "version": v, "description": "Python 包管理工具，用于安装依赖", "dependencies": ["uv.exe"]}
            return {"ok": False, "label": "UV 包管理器", "error": "uv.exe 不存在", "fix": "请重新安装应用程序"}
        except Exception as e:
            return {"ok": False, "label": "UV 包管理器", "error": str(e), "fix": "请重新安装应用程序"}

    def _check_venv():
        try:
            venv_dir = _get_api_venv_dir()
            python_exe = _get_venv_python(venv_dir)
            if os.path.isfile(python_exe):
                r = subprocess.run([python_exe, "--version"], capture_output=True, timeout=5, text=True)
                v = r.stdout.strip() if r.returncode == 0 else "已安装"
                deps_ok = _check_deps_installed(python_exe)
                info = {"ok": True, "label": "API 虚拟环境", "version": v, "description": "API 服务的 Python 虚拟环境", "dependencies": ["fastapi", "uvicorn", "httpx"]}
                if not deps_ok:
                    info["ok"] = False
                    info["error"] = "依赖未安装"
                    info["fix"] = "点击启动 API 服务将自动安装依赖"
                return info
            return {"ok": False, "label": "API 虚拟环境", "error": "虚拟环境不存在", "fix": "点击启动 API 服务将自动创建虚拟环境"}
        except Exception as e:
            return {"ok": False, "label": "API 虚拟环境", "error": str(e), "fix": "请尝试重新启动 API 服务"}

    def _check_qwen2api():
        try:
            base_url = f"http://127.0.0.1:{API_SERVICE_REGISTRY['qwen2api']['default_port']}"
            chk = check_api_service(base_url)
            if chk.get("running") and chk.get("serviceType") == "qwen":
                info = {"ok": True, "label": "千问 API 服务", "version": "运行中", "description": "Qwen2API 代理服务，提供千问模型接口", "dependencies": ["fastapi", "uvicorn", "qwen2api"]}
                if chk.get("accountCount") is not None:
                    info["accountCount"] = chk["accountCount"]
                if chk.get("warning"):
                    info["ok"] = False
                    info["error"] = chk["warning"]
                    info["fix"] = "请在管理台添加 chat.qwen.ai 的账号 Token"
                return info
            return {"ok": False, "label": "千问 API 服务", "error": "服务未运行", "fix": "点击启动千问 API 服务"}
        except Exception as e:
            return {"ok": False, "label": "千问 API 服务", "error": str(e), "fix": "请尝试重新启动服务"}

    def _check_zhipu2api():
        try:
            base_url = f"http://127.0.0.1:{API_SERVICE_REGISTRY['zhipu2api']['default_port']}"
            chk = check_api_service(base_url)
            if chk.get("running") and chk.get("serviceType") == "zhipu":
                info = {"ok": True, "label": "智谱 API 服务", "version": "运行中", "description": "Zhipu2API 代理服务，提供智谱模型接口", "dependencies": ["fastapi", "uvicorn", "zhipu2api"]}
                if chk.get("accountCount") is not None:
                    info["accountCount"] = chk["accountCount"]
                if chk.get("warning"):
                    info["ok"] = False
                    info["error"] = chk["warning"]
                    info["fix"] = "请在管理台添加智谱账号"
                return info
            return {"ok": False, "label": "智谱 API 服务", "error": "服务未运行", "fix": "点击启动智谱 API 服务"}
        except Exception as e:
            return {"ok": False, "label": "智谱 API 服务", "error": str(e), "fix": "请尝试重新启动服务"}

    def _check_ollama():
        try:
            from services.config_service import EnvFileManager, SETTINGS_KEYS
            settings = EnvFileManager(os.path.join(_data_dir(), ".env")).read_settings()
            ollama_url = (settings.get("OLLAMA_BASE_URL") or "http://127.0.0.1:11434").strip()
            result = fetch_json_with_timeout(f"{ollama_url.rstrip('/')}/api/tags", timeout_ms=5000)
            if result.get("ok"):
                models = (result.get("data") or {}).get("models") or []
                model_names = [m.get("name", "") for m in models]
                return {"ok": True, "label": "Ollama 本地模型", "version": f"{len(models)} 个模型", "description": "本地大语言模型运行时", "dependencies": model_names[:5] if model_names else ["Ollama"]}
            return {"ok": False, "label": "Ollama 本地模型", "error": "Ollama 服务未运行", "fix": "请启动 Ollama 应用程序"}
        except Exception:
            return {"ok": False, "label": "Ollama 本地模型", "error": "无法连接 Ollama 服务", "fix": "请安装并启动 Ollama"}

    def _check_data_dir():
        try:
            data = _data_dir()
            temp = _temp_dir()
            writable = os.access(data, os.W_OK) and os.access(temp, os.W_OK)
            if writable:
                return {"ok": True, "label": "数据目录", "version": "可写", "description": "用户数据与临时文件存储", "dependencies": [f"data: {data}", f"temp: {temp}"]}
            return {"ok": False, "label": "数据目录", "error": "目录不可写", "fix": "请检查目录权限"}
        except Exception as e:
            return {"ok": False, "label": "数据目录", "error": str(e), "fix": "请检查目录权限"}

    results["python"] = _check_python()
    results["uv"] = _check_uv()
    results["venv"] = _check_venv()
    results["qwen2api"] = _check_qwen2api()
    results["zhipu2api"] = _check_zhipu2api()
    results["ollama"] = _check_ollama()
    results["dataDir"] = _check_data_dir()
    return results


def _kill_zhipu2api(port: int = 7780):
    global _zhipu2api_proc
    try:
        if _zhipu2api_proc and _zhipu2api_proc.poll() is None:
            _zhipu2api_proc.terminate()
            try:
                _zhipu2api_proc.wait(timeout=5)
            except Exception:
                _zhipu2api_proc.kill()
            _zhipu2api_proc = None
    except Exception:
        pass
    try:
        if os.name == "nt":
            subprocess.run(
                f'for /f "tokens=5" %a in (\'netstat -aon ^| findstr :{port} ^| findstr LISTENING\') do taskkill /F /PID %a',
                shell=True, capture_output=True, timeout=10,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
        else:
            subprocess.run(["fuser", "-k", f"{port}/tcp"], capture_output=True, timeout=10)
    except Exception:
        pass
    time.sleep(1)


def _qwen2api_venv_python() -> str:
    venv_dir = _get_api_venv_dir("qwen2api")
    return _get_venv_python(venv_dir)

def _qwen2api_venv_exists() -> bool:
    return os.path.isfile(_qwen2api_venv_python())

def _check_qwen2api_deps() -> bool:
    if not _qwen2api_venv_exists():
        return False
    venv_python = _qwen2api_venv_python()
    try:
        r = subprocess.run(
            [venv_python, "-c",
             "import fastapi, uvicorn, httpx, pydantic_settings, tiktoken, curl_cffi, camoufox; print('ok')"],
            capture_output=True, text=True, timeout=15,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        return r.returncode == 0 and "ok" in (r.stdout or "")
    except Exception:
        return False


def stop_qwen2api(base_url: str = "") -> dict:
    global _qwen2api_proc
    stopped = False
    if _qwen2api_proc is not None and _qwen2api_proc.poll() is None:
        try:
            _qwen2api_proc.terminate()
            try:
                _qwen2api_proc.wait(timeout=10)
            except Exception:
                _qwen2api_proc.kill()
                try:
                    _qwen2api_proc.wait(timeout=5)
                except Exception:
                    pass
            stopped = True
        except Exception:
            pass
        _qwen2api_proc = None
    port = _extract_port(base_url, 7777)
    if _stop_port_service(port):
        stopped = True
    if stopped:
        return {"ok": True, "message": "API 服务已停止"}
    return {"ok": True, "message": "API 服务未在运行"}


def start_qwen2api(project_dir: str = "", port: int = 7777, admin_key: str = "admin") -> dict:
    global _qwen2api_proc

    qwen_code_dir = project_dir.strip()
    if not qwen_code_dir:
        qwen_code_dir = _get_api_code_dir("qwen2api")

    if not Path(qwen_code_dir).exists():
        return {"ok": False, "error": f"未找到 qwen2api 目录: {qwen_code_dir}"}

    qwen_data_dir = _get_api_data_dir("qwen2api")
    venv_dir = _get_api_venv_dir("qwen2api")
    venv_python = _get_venv_python(venv_dir)

    old_venvs_dir = os.path.join(_data_dir(), "venvs")
    if os.path.isdir(old_venvs_dir):
        try:
            shutil.rmtree(old_venvs_dir, ignore_errors=True)
        except Exception:
            pass

    uv = _uv_exe()
    if not os.path.isfile(venv_python):
        if not os.path.isfile(uv):
            return {"ok": False, "error": "uv 未安装，请先在部署维护中安装 uv"}
        try:
            subprocess.check_call(
                [uv, "venv", venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                env=_uv_env(),
                timeout=300,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        except Exception as e:
            return {"ok": False, "error": f"创建虚拟环境失败: {e}"}

    req_file = Path(qwen_code_dir) / "backend" / "requirements.txt"
    if req_file.exists() and not _check_deps_installed(venv_python):
        try:
            subprocess.check_call(
                [uv, "pip", "install", "-r", str(req_file), "--python", venv_python],
                env=_uv_env(),
                timeout=300,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        except Exception as e:
            return {"ok": False, "error": f"安装 API 服务依赖失败: {e}"}

    env = _uv_env()
    env.update({
        "PYTHONPATH": str(qwen_code_dir),
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
        "PYTHONDONTWRITEBYTECODE": "1",
        "PORT": str(port),
        "ADMIN_KEY": admin_key,
        "WORKERS": "1",
        "ENGINE_MODE": "httpx",
        "LOG_LEVEL": "WARNING",
        "ACCOUNTS_FILE": str(Path(qwen_data_dir) / "accounts.json"),
        "USERS_FILE": str(Path(qwen_data_dir) / "users.json"),
        "CAPTURES_FILE": str(Path(qwen_data_dir) / "captures.json"),
        "CONFIG_FILE": str(Path(qwen_data_dir) / "config.json"),
        "QWEN_DATA_DIR": str(qwen_data_dir),
        "VIRTUAL_ENV": venv_dir,
        "PYDANTIC_SETTINGS_DISABLE_DOTENV": "1",
    })

    Path(qwen_data_dir).mkdir(exist_ok=True)

    log_path = _get_log_path("qwen2api.log")
    try:
        log_file = open(log_path, "a", encoding="utf-8")
    except Exception:
        log_file = subprocess.PIPE

    debug_path = _get_debug_path("start_qwen2api_debug.log")
    try:
        with open(debug_path, "w", encoding="utf-8") as df:
            df.write(f"qwen_code_dir={qwen_code_dir}\n")
            df.write(f"qwen_data_dir={qwen_data_dir}\n")
            df.write(f"venv_dir={venv_dir}\n")
            df.write(f"log_path={log_path}\n")
    except:
        pass

    base = f"http://127.0.0.1:{port}"
    status = check_api_service(base)
    if status.get("running"):
        return {"ok": True, "message": "API 服务已在运行", "baseUrl": base}

    if _qwen2api_proc and _qwen2api_proc.poll() is None:
        return {"ok": True, "message": "API 服务正在启动中", "baseUrl": base}

    try:
        proc = subprocess.Popen(
            [venv_python, "-m", "uvicorn", "backend.main:app",
             "--host", "0.0.0.0", "--port", str(port), "--workers", "1"],
            cwd=qwen_code_dir,
            env=env,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        _qwen2api_proc = proc
    except Exception as e:
        if log_file != subprocess.PIPE:
            try: log_file.close()
            except: pass
        return {"ok": False, "error": f"启动失败: {e}"}

    time.sleep(2)
    if proc.poll() is not None:
        err_msg = "进程意外退出"
        if log_file != subprocess.PIPE:
            try:
                log_file.close()
                with open(log_path, "r", encoding="utf-8", errors="replace") as lf:
                    tail = lf.read()[-2000:]
                if tail.strip():
                    err_msg = tail.strip().split("\n")[-1][:200]
            except:
                pass
        return {"ok": False, "error": f"启动失败: {err_msg}", "logPath": log_path}

    return {"ok": True, "message": "服务已启动", "baseUrl": base, "pid": proc.pid, "logPath": log_path}


def _zhipu2api_venv_python():
    venv_dir = _get_api_venv_dir("zhipu2api")
    return _get_venv_python(venv_dir)

def _check_zhipu2api_deps():
    vp = _zhipu2api_venv_python()
    if not os.path.isfile(vp):
        return False
    try:
        r = subprocess.run(
            [vp, "-c", "import fastapi; import uvicorn; import httpx; import pydantic_settings"],
            capture_output=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        return r.returncode == 0
    except Exception:
        return False

def _sync_zhipu_keys_from_env(base_url: str, admin_key: str = "admin"):
    keys_to_sync = set()
    try:
        env_path = os.path.join(_data_dir(), ".env")
        if not os.path.exists(env_path):
            env_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), ".env")
        if os.path.exists(env_path):
            with open(env_path, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line or line.startswith("#"):
                        continue
                    if "=" not in line:
                        continue
                    key, val = line.split("=", 1)
                    key = key.strip()
                    val = val.strip()
                    if key in ("ZHIPU_API_KEY", "API_KEY") and val and len(val) > 10 and not val.startswith("sk-zhipu-"):
                        keys_to_sync.add(val)
    except Exception:
        pass
    try:
        user_dir = os.path.join(_data_dir(), "users", "default")
        zhipu_keys_path = os.path.join(user_dir, "zhipu_keys.json")
        if os.path.exists(zhipu_keys_path):
            with open(zhipu_keys_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            if isinstance(data, list):
                for item in data:
                    if isinstance(item, dict) and item.get("key") and len(item["key"]) > 10 and not item["key"].startswith("sk-zhipu-"):
                        keys_to_sync.add(item["key"])
    except Exception:
        pass
    for k in keys_to_sync:
        try:
            add_zhipu_account(base_url, k, admin_key, label="auto-synced", timeout_ms=5000)
        except Exception:
            pass


def start_zhipu2api(project_dir: str = "", port: int = 7780, admin_key: str = "admin") -> dict:
    global _zhipu2api_proc

    zhipu_code_dir = project_dir.strip()
    if not zhipu_code_dir:
        zhipu_code_dir = _get_api_code_dir("zhipu2api")

    if not Path(zhipu_code_dir).exists():
        return {"ok": False, "error": f"未找到 zhipu2api 目录: {zhipu_code_dir}"}

    zhipu_data_dir = _get_api_data_dir("zhipu2api")
    venv_dir = _get_api_venv_dir("zhipu2api")
    venv_python = _get_venv_python(venv_dir)

    old_venvs_dir = os.path.join(_data_dir(), "venvs")
    if os.path.isdir(old_venvs_dir):
        try:
            shutil.rmtree(old_venvs_dir, ignore_errors=True)
        except Exception:
            pass

    uv = _uv_exe()
    if not os.path.isfile(venv_python):
        if not os.path.isfile(uv):
            return {"ok": False, "error": "uv 未安装，请先在部署维护中安装 uv"}
        try:
            subprocess.check_call(
                [uv, "venv", venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                env=_uv_env(),
                timeout=300,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        except Exception as e:
            return {"ok": False, "error": f"创建虚拟环境失败: {e}"}

    req_file = Path(zhipu_code_dir) / "requirements.txt"
    if req_file.exists() and not _check_zhipu2api_deps():
        try:
            subprocess.check_call(
                [uv, "pip", "install", "-r", str(req_file), "--python", venv_python],
                env=_uv_env(),
                timeout=300,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        except Exception as e:
            return {"ok": False, "error": f"安装智谱 API 服务依赖失败: {e}"}

    env = _uv_env()
    env.update({
        "PYTHONPATH": str(zhipu_code_dir),
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
        "PYTHONDONTWRITEBYTECODE": "1",
        "PORT": str(port),
        "ADMIN_KEY": admin_key,
        "ZHIPU_DATA_DIR": str(zhipu_data_dir),
        "ZHIPU_BASE_URL": "https://open.bigmodel.cn/api/paas/v4",
        "VIRTUAL_ENV": venv_dir,
    })

    Path(zhipu_data_dir).mkdir(exist_ok=True)

    log_path = _get_log_path("zhipu2api.log")
    try:
        log_file = open(log_path, "a", encoding="utf-8")
    except Exception:
        log_file = subprocess.PIPE

    base = f"http://127.0.0.1:{port}"
    status = check_api_service(base)
    if status.get("running"):
        try:
            probe = urllib.request.urlopen(f"{base}/v1/models", timeout=3)
            if probe.status == 200:
                return {"ok": True, "message": "智谱 API 服务已在运行", "baseUrl": base}
        except Exception:
            pass
        _kill_zhipu2api(port)

    if _zhipu2api_proc and _zhipu2api_proc.poll() is None:
        return {"ok": True, "message": "智谱 API 服务正在启动中", "baseUrl": base}

    try:
        proc = subprocess.Popen(
            [venv_python, "-m", "uvicorn", "main:app",
             "--host", "0.0.0.0", "--port", str(port), "--workers", "1"],
            cwd=zhipu_code_dir,
            env=env,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        _zhipu2api_proc = proc
    except Exception as e:
        if log_file != subprocess.PIPE:
            try: log_file.close()
            except: pass
        return {"ok": False, "error": f"启动智谱 API 服务失败: {e}"}

    time.sleep(2)
    if proc.poll() is not None:
        err_msg = "进程意外退出"
        if log_file != subprocess.PIPE:
            try:
                log_file.close()
                with open(log_path, "r", encoding="utf-8", errors="replace") as lf:
                    tail = lf.read()[-2000:]
                if tail.strip():
                    err_msg = tail.strip().split("\n")[-1][:200]
            except:
                pass
        return {"ok": False, "error": f"启动失败: {err_msg}", "logPath": log_path}

    result = {"ok": True, "message": "智谱服务已启动", "baseUrl": base, "pid": proc.pid, "logPath": log_path}
    _sync_zhipu_keys_from_env(base, admin_key)
    return result


def stop_zhipu2api(base_url: str = "") -> dict:
    global _zhipu2api_proc
    stopped = False
    if _zhipu2api_proc is not None and _zhipu2api_proc.poll() is None:
        try:
            _zhipu2api_proc.terminate()
            try: _zhipu2api_proc.wait(timeout=10)
            except Exception:
                _zhipu2api_proc.kill()
            stopped = True
        except Exception:
            pass
        _zhipu2api_proc = None
    port = _extract_port(base_url, 7780)
    if _stop_port_service(port):
        stopped = True
    if stopped:
        return {"ok": True, "message": "智谱 API 服务已停止"}
    return {"ok": True, "message": "智谱 API 服务未在运行"}


def add_zhipu_account(base_url: str, api_key: str, admin_key: str = "", label: str = "", timeout_ms: int = 15000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts",
            method="POST",
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {admin_key or 'admin'}",
            },
            body=json.dumps({"api_key": api_key.strip(), "label": label.strip()}).encode("utf-8"),
            timeout_ms=timeout_ms,
        )
        return result
    except Exception as e:
        return {"ok": False, "error": str(e)}


def start_zhipu_register(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/register",
            method="POST",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}", "Content-Type": "application/json"},
            body=json.dumps({}).encode("utf-8"),
            timeout_ms=timeout_ms,
        )
        if result.get("ok"):
            data = result.get("data", {})
            if isinstance(data, dict):
                return data
            return {"ok": True, "message": "注册已启动"}
        error_msg = result.get("text", "") or f"HTTP {result.get('status', 0)}"
        return {"ok": False, "error": error_msg}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def poll_zhipu_register(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/register/status",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        if result.get("ok"):
            data = result.get("data", {})
            if isinstance(data, dict):
                return data
            return {"ok": True, "busy": False, "status": "unknown"}
        error_msg = result.get("text", "") or f"HTTP {result.get('status', 0)}"
        return {"ok": False, "error": error_msg, "busy": False}
    except Exception as e:
        return {"ok": False, "error": str(e), "busy": False}


def login_zhipu_account(base_url: str, email: str, password: str, region: str = "international", admin_key: str = "", timeout_ms: int = 20000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/login",
            method="POST",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}", "Content-Type": "application/json"},
            body=json.dumps({"email": email, "password": password, "region": region}).encode("utf-8"),
            timeout_ms=timeout_ms,
        )
        if result.get("ok"):
            data = result.get("data", {})
            if isinstance(data, dict):
                return data
            return {"ok": True}
        error_msg = result.get("text", "") or f"HTTP {result.get('status', 0)}"
        return {"ok": False, "error": error_msg}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def list_zhipu_accounts(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        return result
    except Exception as e:
        return {"ok": False, "error": str(e)}


def delete_zhipu_account(base_url: str, api_key: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/{api_key}",
            method="DELETE",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        return result
    except Exception as e:
        return {"ok": False, "error": str(e)}


def validate_zhipu_account(base_url: str, api_key: str, admin_key: str = "", timeout_ms: int = 15000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/accounts/{api_key}/validate",
            method="POST",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        return result
    except Exception as e:
        return {"ok": False, "error": str(e)}


def fetch_zhipu_api_key(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/keys",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        if result.get("ok"):
            data = result.get("data") or {}
            raw_keys = data.get("keys", [])
            if raw_keys:
                keys = []
                for k in raw_keys:
                    if isinstance(k, dict):
                        keys.append({"key": k.get("key", ""), "label": k.get("label", k.get("key", "")[:8] + "...")})
                    elif isinstance(k, str):
                        keys.append({"key": k, "label": k[:8] + "..."})
                return {"ok": True, "keys": keys}
            return {"ok": True, "keys": []}
        return {"ok": False, "error": f"HTTP {result.get('status', '?')}: {result.get('text', '')[:200]}"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def create_zhipu_api_key(base_url: str, admin_key: str = "", timeout_ms: int = 10000) -> dict:
    try:
        result = fetch_json_with_timeout(
            f"{base_url.rstrip('/')}/api/admin/keys",
            method="POST",
            headers={"Authorization": f"Bearer {admin_key or 'admin'}"},
            timeout_ms=timeout_ms,
        )
        if result.get("ok"):
            data = result.get("data") or {}
            key = data.get("key", "")
            if key:
                return {"ok": True, "key": key}
            return {"ok": False, "error": f"创建成功但未返回 key: {json.dumps(data)[:200]}"}
        return {"ok": False, "error": f"HTTP {result.get('status', '?')}: {result.get('text', '')[:200]}"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


class OllamaProxyHandler(http.server.BaseHTTPRequestHandler):
    """将 Anthropic Messages API 转换为 Ollama /api/chat API"""

    target_url = None
    proxy_model = "qwen3:8b"
    close_connection = True
    _proxy_log_file = None
    _proxy_log_dir = None

    def log_message(self, format, *args):
        pass

    @classmethod
    def _log_proxy(cls, msg):
        try:
            if cls._proxy_log_file is None:
                log_dir = cls._proxy_log_dir or os.path.join(_temp_dir(), "logs")
                os.makedirs(log_dir, exist_ok=True)
                cls._proxy_log_file = open(os.path.join(log_dir, "proxy_debug.log"), "a", encoding="utf-8")
            ts = time.strftime("%H:%M:%S")
            cls._proxy_log_file.write(f"[{ts}] {msg}\n")
            cls._proxy_log_file.flush()
        except Exception as e:
            try:
                sys.stderr.write(f"[proxy_log_err] {e}\n")
            except:
                pass

    def do_POST(self):
        if self.path.startswith("/v1/messages"):
            self._handle_messages()
        else:
            self.send_error(404)

    def _handle_messages(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body_bytes = self.rfile.read(content_length)
        try:
            anthropic_body = json.loads(body_bytes)
        except:
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": "Invalid JSON"}).encode())
            return

        msgs = anthropic_body.get("messages", [])
        msg_summary = []
        for m in msgs:
            role = m.get("role", "?")
            content = m.get("content", "")
            if isinstance(content, list):
                types = [b.get("type", "?") for b in content]
                msg_summary.append(f"{role}[{','.join(types)}]")
            elif isinstance(content, str):
                msg_summary.append(f"{role}({len(content)})")
            else:
                msg_summary.append(f"{role}(?)")
        self._log_proxy(f"REQUEST: msgs={len(msgs)} summary={msg_summary} stream={anthropic_body.get('stream')} tools={len(anthropic_body.get('tools', []))}")

        ollama_messages, system_prompt = self._convert_messages(anthropic_body)
        ollama_tools = self._convert_tools(anthropic_body)

        MAX_MESSAGES = 30
        if len(ollama_messages) > MAX_MESSAGES:
            self._log_proxy(f"TRUNCATE: {len(ollama_messages)} msgs > {MAX_MESSAGES}, keeping last {MAX_MESSAGES}")
            ollama_messages = ollama_messages[-MAX_MESSAGES:]

        stream = anthropic_body.get("stream") is True
        ollama_body = {
            "model": self.proxy_model,
            "messages": ollama_messages,
            "stream": stream,
            "options": {
                "num_ctx": 65536,
            },
        }
        if system_prompt:
            ollama_body["system"] = system_prompt
        if ollama_tools:
            ollama_body["tools"] = ollama_tools

        env_temp = os.environ.get("AI_TEMPERATURE", "").strip()
        env_max_tokens = os.environ.get("AI_MAX_TOKENS", "").strip()
        if env_temp:
            try:
                ollama_body["options"]["temperature"] = float(env_temp)
            except ValueError:
                pass
        if env_max_tokens:
            try:
                ollama_body["options"]["num_predict"] = int(env_max_tokens)
            except ValueError:
                pass

        self._send_to_ollama(ollama_body, stream)

    def _convert_messages(self, body: dict):
        ollama_msgs = []
        system_prompt = ""

        if isinstance(body.get("system"), str) and body["system"].strip():
            system_prompt = body["system"]
        elif isinstance(body.get("system"), list):
            system_prompt = "\n".join(
                b.get("text", "") for b in body["system"] if b.get("type") == "text"
            )
        else:
            lang = os.environ.get("AI_LANGUAGE", "zh").strip().lower()
            if lang == "zh":
                system_prompt = "你是一个专业的AI编程助手。请始终使用中文回答。只有在用户明确要求执行编程任务时才使用工具。普通对话请直接回答。"
            elif lang == "ja":
                system_prompt = "あなたはプロのAIプログラミングアシスタントです。日本語で回答してください。"
            elif lang == "ko":
                system_prompt = "당신은 전문 AI 프로그래밍 어시스턴트입니다. 한국어로 답변해 주세요."
            elif lang and lang != "en":
                system_prompt = f"You are a professional AI coding assistant. Please respond in {lang}."

        for msg in body.get("messages", []):
            if msg["role"] == "user":
                tool_results, text_parts = [], []
                content = msg.get("content", "")
                if isinstance(content, list):
                    for block in content:
                        if block.get("type") == "tool_result":
                            tool_results.append(block)
                        elif block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                elif isinstance(content, str):
                    text_parts.append(content)

                for tr in tool_results:
                    c = tr.get("content", "")
                    if isinstance(c, list):
                        c = "\n".join(b.get("text", "") for b in c if b.get("type") == "text")
                    elif not isinstance(c, str):
                        c = ""
                    ollama_msgs.append({
                        "role": "tool",
                        "name": tr.get("tool_use_id", "unknown"),
                        "content": c or "(no output)",
                    })
                text = "\n".join(p for p in text_parts if p)
                if text:
                    ollama_msgs.append({"role": "user", "content": text})

            elif msg["role"] == "assistant":
                tool_calls, text_parts = [], []
                content = msg.get("content", "")
                if isinstance(content, list):
                    for block in content:
                        if block.get("type") == "tool_use":
                            tool_calls.append(block)
                        elif block.get("type") == "text":
                            text_parts.append(block.get("text", ""))
                elif isinstance(content, str):
                    text_parts.append(content)

                text = "\n".join(p for p in text_parts if p)
                assistant_msg = {"role": "assistant", "content": text or ""}
                ollama_tool_calls = []
                for tc in tool_calls:
                    args = tc.get("input", {})
                    if isinstance(args, str):
                        try:
                            args = json.loads(args)
                        except:
                            args = {}
                    elif not isinstance(args, dict):
                        args = {}
                    ollama_tool_calls.append({
                        "function": {
                            "name": tc.get("name", ""),
                            "arguments": args,
                        },
                    })
                if ollama_tool_calls:
                    assistant_msg["tool_calls"] = ollama_tool_calls
                ollama_msgs.append(assistant_msg)

        self._log_proxy(f"CONVERT: {len(body.get('messages', []))} anthropic msgs -> {len(ollama_msgs)} ollama msgs")
        for i, m in enumerate(ollama_msgs):
            role = m.get("role", "?")
            tc = m.get("tool_calls", [])
            self._log_proxy(f"  msg[{i}]: role={role} has_tool_calls={len(tc) > 0} content_len={len(m.get('content', ''))}")

        return ollama_msgs, system_prompt

    def _convert_tools(self, body: dict) -> list:
        ollama_tools = []
        for tool in body.get("tools", []):
            if tool.get("type") == "custom":
                continue
            ollama_tools.append({
                "type": "function",
                "function": {
                    "name": tool.get("name", ""),
                    "description": tool.get("description", ""),
                    "parameters": tool.get("input_schema", {"type": "object", "properties": {}}),
                },
            })
        return ollama_tools if ollama_tools else None

    def _send_to_ollama(self, body_obj: dict, stream: bool):
        import re as _re
        body_str = json.dumps(body_obj, ensure_ascii=False).encode("utf-8")
        target = self.target_url

        msg_count = len(body_obj.get("messages", []))
        has_tools = "yes" if body_obj.get("tools") else "no"
        self._log_proxy(f"OLLAMA: msgs={msg_count} tools={has_tools} stream={stream} model={body_obj.get('model')} body_len={len(body_str)}")
        if msg_count > 0:
            last_msg = body_obj["messages"][-1]
            self._log_proxy(f"OLLAMA: last_msg role={last_msg.get('role')} content_preview={str(last_msg.get('content', ''))[:200]}")

        conn = None
        try:
            conn = http.client.HTTPConnection(
                target.hostname,
                target.port or 11434,
                timeout=30,
            )
            conn.request("POST", "/api/chat", body=body_str, headers={"Content-Type": "application/json"})
            resp = conn.getresponse()
            self._log_proxy(f"OLLAMA: response status={resp.status}")

            try:
                if conn.sock:
                    conn.sock.settimeout(90)
                    self._log_proxy("OLLAMA: set read timeout=90s")
            except Exception as te:
                self._log_proxy(f"OLLAMA: settimeout failed: {te}")

            if resp.status != 200:
                err_body = resp.read().decode("utf-8", errors="ignore")
                self._log_proxy(f"OLLAMA ERROR: code={resp.status} body={err_body[:500]}")
                if resp.status == 400 and "does not support tools" in err_body and body_obj.get("tools"):
                    fallback_body = {k: v for k, v in body_obj.items() if k != "tools"}
                    ollama_tools = body_obj.get("tools", [])
                    if ollama_tools:
                        tool_descs = "\n".join(
                            f"- {t.get('function', {}).get('name', '')}: {t.get('function', {}).get('description', '')} Params: {json.dumps(t.get('function', {}).get('parameters', {}))}"
                            for t in ollama_tools
                        )
                        bt = "`" * 3
                        tool_prompt = (
                            f"\n\nYou have access to the following tools. To call a tool, output a JSON block enclosed in {bt} tags like this:\n"
                            f"{bt}\n"
                            f'{{"name": "tool_name", "arguments": {{"param": "value"}}}}\n'
                            f"{bt}\n"
                            f"You can call multiple tools. After each tool call, wait for the result in the next user message enclosed in <tool_result> tags.\n"
                            f"Available tools:\n{tool_descs}"
                        )
                        fallback_body["system"] = fallback_body.get("system", "") + tool_prompt
                    fallback_str = json.dumps(fallback_body).encode("utf-8")
                    conn2 = http.client.HTTPConnection(
                        target.hostname,
                        target.port or 11434,
                        timeout=300,
                    )
                    conn2.request("POST", "/api/chat", body=fallback_str, headers={"Content-Type": "application/json"})
                    resp2 = conn2.getresponse()
                    if resp2.status != 200:
                        err_body2 = resp2.read().decode("utf-8", errors="ignore")
                        self.send_response(resp2.status)
                        self.send_header("Content-Type", "application/json")
                        self.end_headers()
                        self.wfile.write(json.dumps({
                            "type": "error",
                            "error": {"type": "invalid_request_error", "message": f"Ollama error ({resp2.status}): {err_body2}"},
                        }).encode())
                        conn2.close()
                        conn.close()
                        return
                    if stream:
                        self._handle_ndjson_stream(resp2, is_retry=True)
                    else:
                        self._handle_non_stream_response(resp2, is_retry=True)
                    return
                else:
                    self.send_response(resp.status)
                    self.send_header("Content-Type", "application/json")
                    self.end_headers()
                    self.wfile.write(json.dumps({
                        "type": "error",
                        "error": {"type": "invalid_request_error", "message": f"Ollama error ({resp.status}): {err_body}"},
                    }).encode())
                    conn.close()
                    return

            if stream:
                self._handle_ndjson_stream(resp)
            else:
                self._handle_non_stream_response(resp)
            self._log_proxy("OLLAMA: stream handling completed")
            try:
                conn.close()
            except:
                pass

        except Exception as e:
            self._log_proxy(f"OLLAMA EXCEPTION: {e}")
            try:
                self.send_response(502)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({
                    "type": "error",
                    "error": {"type": "api_error", "message": f"Proxy error: {e}"},
                }).encode())
            except:
                pass
            if conn:
                try:
                    conn.close()
                except:
                    pass

    def _handle_ndjson_stream(self, resp, is_retry=False):
        import re as _re
        msg_id = f"msg_{int(time.time())}"
        sent_start = False
        content_block_index = 0
        current_tool_use_index = 0
        has_tool_calls = False
        full_content = ""
        input_tokens = 0
        output_tokens = 0
        text_block_started = False
        text_block_closed = False

        if is_retry:
            resp_data = resp.read().decode("utf-8", errors="ignore")
            for line in resp_data.split("\n"):
                trimmed = line.strip()
                if not trimmed:
                    continue
                try:
                    chunk = json.loads(trimmed)
                    if chunk.get("message", {}).get("content"):
                        full_content += chunk["message"]["content"]
                    if chunk.get("prompt_eval_count"):
                        input_tokens = chunk["prompt_eval_count"]
                    if chunk.get("eval_count"):
                        output_tokens = chunk["eval_count"]
                except:
                    pass

            tool_calls = self._parse_tool_calls_from_text(full_content)
            clean_content = self._remove_tool_call_blocks_from_text(full_content)

            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.end_headers()
            self._write_sse("message_start", {
                "type": "message_start",
                "message": {
                    "id": msg_id, "type": "message", "role": "assistant",
                    "content": [], "model": self.proxy_model,
                    "stop_reason": None, "stop_sequence": None,
                    "usage": {"input_tokens": input_tokens, "output_tokens": 0},
                },
            })

            cb_index = 0
            if clean_content.strip():
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": clean_content}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                cb_index = 1

            for i, tc in enumerate(tool_calls):
                tool_use_id = f"toolu_{int(time.time())}_{i}"
                self._write_sse("content_block_start", {"type": "content_block_start", "index": cb_index, "content_block": {"type": "tool_use", "id": tool_use_id, "name": tc["name"], "input": {}}})
                self._write_sse("content_block_delta", {"type": "content_block_delta", "index": cb_index, "delta": {"type": "input_json_delta", "partial_json": json.dumps(tc["arguments"])}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": cb_index})
                cb_index += 1

            if cb_index == 0:
                self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})

            stop_reason = "tool_use" if tool_calls else "end_turn"
            self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop_reason, "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
            self._write_sse("message_stop", {"type": "message_stop"})
            self.wfile.flush()
            return

        try:
            buffer = ""
            chunk_count = 0
            last_data_time = time.time()
            ollama_timeout = 120
            read_was_timeout = False
            while True:
                try:
                    chunk = resp.read(4096)
                except (socket.timeout, OSError) as read_err:
                    self._log_proxy(f"STREAM: read error after {chunk_count} chunks: {read_err}")
                    read_was_timeout = isinstance(read_err, socket.timeout)
                    break
                if not chunk:
                    self._log_proxy(f"STREAM: chunk empty after {chunk_count} chunks, connection closed by Ollama")
                    break
                chunk_count += 1
                last_data_time = time.time()
                buffer += chunk.decode("utf-8", errors="ignore")

                lines = buffer.split("\n")
                buffer = lines.pop() or ""

                for line in lines:
                    trimmed = line.strip()
                    if not trimmed:
                        continue

                    try:
                        ollama_chunk = json.loads(trimmed)
                    except:
                        self._log_proxy(f"STREAM: failed to parse chunk: {trimmed[:200]}")
                        continue

                    has_chunk_content = bool(ollama_chunk.get("message", {}).get("content"))
                    has_chunk_tools = bool(ollama_chunk.get("message", {}).get("tool_calls"))
                    is_done = ollama_chunk.get("done", False)
                    if is_done or has_chunk_content or has_chunk_tools or chunk_count <= 3:
                        self._log_proxy(f"STREAM chunk#{chunk_count}: content={has_chunk_content} tools={has_chunk_tools} done={is_done} eval_count={ollama_chunk.get('eval_count')} prompt_eval_count={ollama_chunk.get('prompt_eval_count')}")

                    if ollama_chunk.get("prompt_eval_count"):
                        input_tokens = ollama_chunk["prompt_eval_count"]
                    if ollama_chunk.get("eval_count"):
                        output_tokens = ollama_chunk["eval_count"]

                    if not sent_start:
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.send_header("Connection", "close")
                        self.end_headers()
                        self._write_sse("message_start", {
                            "type": "message_start",
                            "message": {
                                "id": msg_id, "type": "message", "role": "assistant",
                                "content": [], "model": self.proxy_model,
                                "stop_reason": None, "stop_sequence": None,
                                "usage": {"input_tokens": input_tokens, "output_tokens": 0},
                            },
                        })
                        self._write_sse("content_block_start", {
                            "type": "content_block_start", "index": 0,
                            "content_block": {"type": "text", "text": ""},
                        })
                        text_block_started = True
                        sent_start = True

                    if not ollama_chunk.get("message", {}).get("content") and not ollama_chunk.get("message", {}).get("tool_calls") and not ollama_chunk.get("done"):
                        try:
                            self.wfile.write(b": heartbeat\n\n")
                            self.wfile.flush()
                        except:
                            pass

                    if ollama_chunk.get("message", {}).get("content"):
                        text_content = ollama_chunk["message"]["content"]
                        if not has_tool_calls:
                            full_content += text_content
                            self._write_sse("content_block_delta", {
                                "type": "content_block_delta", "index": 0,
                                "delta": {"type": "text_delta", "text": text_content},
                            })

                    if ollama_chunk.get("message", {}).get("tool_calls") and len(ollama_chunk["message"]["tool_calls"]) > 0:
                        if text_block_started and not text_block_closed and not has_tool_calls:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                            text_block_closed = True
                            content_block_index = 1
                        elif not text_block_started:
                            content_block_index = 0

                        has_tool_calls = True

                        for tc in ollama_chunk["message"]["tool_calls"]:
                            tool_use_id = f"toolu_{int(time.time())}_{current_tool_use_index}"
                            tool_name = (tc.get("function", {}) or {}).get("name", "unknown")
                            tool_input = (tc.get("function", {}) or {}).get("arguments", {})

                            self._write_sse("content_block_start", {
                                "type": "content_block_start",
                                "index": content_block_index,
                                "content_block": {
                                    "type": "tool_use",
                                    "id": tool_use_id,
                                    "name": tool_name,
                                    "input": {},
                                },
                            })
                            self._write_sse("content_block_delta", {
                                "type": "content_block_delta",
                                "index": content_block_index,
                                "delta": {
                                    "type": "input_json_delta",
                                    "partial_json": json.dumps(tool_input),
                                },
                            })
                            self._write_sse("content_block_stop", {
                                "type": "content_block_stop", "index": content_block_index,
                            })

                            content_block_index += 1
                            current_tool_use_index += 1

                    if ollama_chunk.get("done"):
                        self._log_proxy(f"STREAM: ollama done, has_tool_calls={has_tool_calls} text_block_started={text_block_started} text_block_closed={text_block_closed}")
                        if not has_tool_calls and text_block_started and not text_block_closed:
                            self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})

                        done_reason = ollama_chunk.get("done_reason", "")
                        stop_reason = "tool_use" if has_tool_calls else ("max_tokens" if done_reason == "length" else "end_turn")

                        self._write_sse("message_delta", {
                            "type": "message_delta",
                            "delta": {"stop_reason": stop_reason, "stop_sequence": None},
                            "usage": {"output_tokens": output_tokens or len(full_content)},
                        })
                        self._write_sse("message_stop", {"type": "message_stop"})
                        self.wfile.flush()
                        return

        except Exception as stream_err:
            self._log_proxy(f"STREAM ERROR: sent_start={sent_start} text_block_started={text_block_started} err={stream_err}")
            try:
                if sent_start:
                    if not text_block_started and not has_tool_calls:
                        self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                        if full_content:
                            self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": full_content}})
                        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                    elif text_block_started and not text_block_closed:
                        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                    self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
                    self._write_sse("message_stop", {"type": "message_stop"})
                else:
                    self._send_empty_message(msg_id)
                self.wfile.flush()
            except:
                pass
            return

        self._log_proxy(f"STREAM END: sent_start={sent_start} text_block_started={text_block_started} text_block_closed={text_block_closed} has_tool_calls={has_tool_calls} text_len={len(full_content)} chunks={chunk_count} timeout={read_was_timeout}")
        if sent_start:
            try:
                if not text_block_started and not has_tool_calls:
                    self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
                    timeout_msg = ""
                    if read_was_timeout and not full_content:
                        timeout_msg = "\n\n[代理超时] Ollama未在90秒内返回数据，对话可能因上下文过长而中断。请尝试开启新对话。"
                    if full_content or timeout_msg:
                        self._write_sse("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": full_content + timeout_msg}})
                    self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                elif text_block_started and not text_block_closed:
                    self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
                self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": output_tokens or len(full_content)}})
                self._write_sse("message_stop", {"type": "message_stop"})
                self.wfile.flush()
                self._log_proxy("STREAM: Complete end events sent successfully")
            except Exception as e:
                self._log_proxy(f"STREAM END ERROR: {e}")
        else:
            self._send_empty_message(msg_id)

    def _handle_non_stream_response(self, resp, is_retry=False):
        import re as _re
        resp_data = resp.read().decode("utf-8", errors="ignore")
        full_content = ""
        input_tokens = 0
        output_tokens = 0
        tool_calls = []

        for line in resp_data.split("\n"):
            trimmed = line.strip()
            if not trimmed:
                continue
            try:
                chunk = json.loads(trimmed)
                if chunk.get("message", {}).get("content"):
                    full_content += chunk["message"]["content"]
                if chunk.get("message", {}).get("tool_calls"):
                    for tc in chunk["message"]["tool_calls"]:
                        tool_calls.append(tc)
                if chunk.get("prompt_eval_count"):
                    input_tokens = chunk["prompt_eval_count"]
                if chunk.get("eval_count"):
                    output_tokens = chunk["eval_count"]
            except:
                pass

        if is_retry and not tool_calls and full_content:
            parsed_calls = self._parse_tool_calls_from_text(full_content)
            for pc in parsed_calls:
                tool_calls.append({"function": {"name": pc["name"], "arguments": pc["arguments"]}})
            if tool_calls:
                full_content = self._remove_tool_call_blocks_from_text(full_content)

        content = []
        if full_content:
            content.append({"type": "text", "text": full_content})

        tool_use_index = 0
        for tc in tool_calls:
            func = tc.get("function", {})
            args = func.get("arguments", {})
            if isinstance(args, str):
                try:
                    args = json.loads(args)
                except:
                    args = {}
            content.append({
                "type": "tool_use",
                "id": f"toolu_{int(time.time())}_{tool_use_index}",
                "name": func.get("name", "unknown"),
                "input": args,
            })
            tool_use_index += 1

        if not content:
            content.append({"type": "text", "text": ""})

        stop_reason = "tool_use" if tool_calls else "end_turn"

        response = {
            "id": f"msg_{int(time.time())}",
            "type": "message",
            "role": "assistant",
            "content": content,
            "model": self.proxy_model,
            "stop_reason": stop_reason,
            "stop_sequence": None,
            "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
        }

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def _parse_tool_calls_from_text(self, text):
        import re as _re
        calls = []
        seen = set()
        patterns = [
            r'```(?:json)?\s*\n?([\s\S]*?)\n?```',
            r'\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\s*\}',
        ]
        for pattern in patterns:
            for match in _re.finditer(pattern, text):
                json_str = match.group(1) if match.lastindex and match.lastindex >= 1 else match.group(0)
                try:
                    parsed = json.loads(json_str)
                    if isinstance(parsed, dict) and parsed.get("name") and isinstance(parsed.get("name"), str):
                        key = parsed["name"] + ":" + json.dumps(parsed.get("arguments", {}), sort_keys=True)
                        if key not in seen:
                            seen.add(key)
                            calls.append({"name": parsed["name"], "arguments": parsed.get("arguments", {})})
                except:
                    if match.lastindex and match.lastindex >= 1:
                        try:
                            inner = json.loads(match.group(1))
                            if isinstance(inner, dict) and inner.get("name") and isinstance(inner.get("name"), str):
                                key = inner["name"] + ":" + json.dumps(inner.get("arguments", {}), sort_keys=True)
                                if key not in seen:
                                    seen.add(key)
                                    calls.append({"name": inner["name"], "arguments": inner.get("arguments", {})})
                        except:
                            pass
        return calls

    def _remove_tool_call_blocks_from_text(self, text):
        import re as _re
        result = _re.sub(r'```(?:json)?\s*\n?[\s\S]*?\n?```', lambda m: "" if self._is_tool_call_json(m.group(0)) else m.group(0), text)
        result = _re.sub(r'\{\s*"name"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:\s*\{[^}]*\}\s*\}', "", result)
        result = _re.sub(r'\n{3,}', '\n\n', result).strip()
        return result

    def _is_tool_call_json(self, block):
        import re as _re
        inner = _re.sub(r'^```(?:json)?\s*\n?', '', block)
        inner = _re.sub(r'\n?```$', '', inner)
        try:
            parsed = json.loads(inner)
            return isinstance(parsed, dict) and parsed.get("name") and parsed.get("arguments")
        except:
            return False

    def _send_empty_message(self, msg_id):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        self._write_sse("message_start", {
            "type": "message_start",
            "message": {"id": msg_id, "type": "message", "role": "assistant", "content": [],
                        "model": self.proxy_model, "stop_reason": None, "stop_sequence": None,
                        "usage": {"input_tokens": 0, "output_tokens": 0}},
        })
        self._write_sse("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})
        self._write_sse("content_block_stop", {"type": "content_block_stop", "index": 0})
        self._write_sse("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 0}})
        self._write_sse("message_stop", {"type": "message_stop"})

    def _write_sse(self, event: str, data: dict):
        payload = f"event: {event}\ndata: {json.dumps(data)}\n\n".encode()
        try:
            self.wfile.write(payload)
            self.wfile.flush()
            if event in ("message_start", "content_block_start", "content_block_stop", "message_delta", "message_stop"):
                self._log_proxy(f"SSE SENT: event={event} payload_len={len(payload)}")
        except Exception as e:
            self._log_proxy(f"SSE WRITE ERROR: event={event} err={e}")
            raise


class OllamaProxyServer:
    """管理 Ollama 代理的生命周期"""

    def __init__(self):
        self.server = None
        self.port = 0
        self.thread = None
        self._target_base_url = ""
        self._model = ""

    def start(self, target_base_url: str, model: str, log_dir: str = None) -> int:
        self._target_base_url = target_base_url
        self._model = model or "qwen3:8b"

        if log_dir:
            OllamaProxyHandler._proxy_log_dir = log_dir
            OllamaProxyHandler._proxy_log_file = None

        if self.server and self.thread and self.thread.is_alive():
            OllamaProxyHandler.target_url = urllib.parse.urlparse(target_base_url)
            OllamaProxyHandler.proxy_model = self._model
            return self.port

        if self.server:
            try:
                self.server.server_close()
            except:
                pass
            self.server = None

        target_url = urllib.parse.urlparse(target_base_url)
        OllamaProxyHandler.target_url = target_url
        OllamaProxyHandler.proxy_model = self._model

        self.server = socketserver.TCPServer(("127.0.0.1", 0), OllamaProxyHandler)
        self.port = self.server.server_address[1]
        self.server.timeout = 1

        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()
        return self.port

    def _serve(self):
        while self.server:
            try:
                self.server.handle_request()
            except:
                if not self.server:
                    break

    def stop(self):
        if self.server:
            try:
                self.server.server_close()
            except:
                pass
            self.server = None
            self.port = 0


def _get_base_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    return str(Path(__file__).resolve().parent.parent.parent)


class AiService:
    def __init__(self):
        self._proxy_server = None
        self._cli_runner = None
        self._sessions = {}

    def _read_settings(self) -> dict:
        from services.config_service import EnvFileManager
        env_path = os.path.join(_get_base_dir(), "data", ".env")
        return EnvFileManager(env_path).read_settings()

    async def stream_chat(self, params: dict):
        import asyncio
        settings = self._read_settings()
        provider = params.get("provider") or settings.get("MODEL_PROVIDER", "ollama")
        model = params.get("model") or ""
        prompt = params.get("prompt", "")
        session_id = params.get("session_id") or ""
        system_prompt = params.get("system_prompt") or settings.get("SYSTEM_PROMPT", "")
        temperature = params.get("ai_temperature")
        max_tokens = params.get("ai_max_tokens")
        # 2026-06-08 TASK-2.3 引入：图片附件（base64 数据 URL 列表）
        raw_images = params.get("images") or []

        if not self._proxy_server:
            self._proxy_server = OllamaProxyServer()

        queue = asyncio.Queue()

        def _parse_data_url(data_url: str):
            """把 data URL 拆成 (media_type, base64_data)。失败返回 (None, None)。"""
            if not isinstance(data_url, str):
                return None, None
            if not data_url.startswith("data:"):
                # 视为已经是 base64 字符串
                return "image/png", data_url
            try:
                head, b64 = data_url.split(",", 1)
                # head = "data:image/png;base64"
                media_type = head.split(";")[0].replace("data:", "").strip() or "image/png"
                return media_type, b64
            except Exception:
                return None, None

        def _build_anthropic_body():
            # 2026-06-08 TASK-2.3: images 转 Anthropic vision blocks
            if raw_images:
                content_blocks: list[dict] = []
                for url in raw_images:
                    media_type, b64 = _parse_data_url(url)
                    if not b64:
                        continue
                    content_blocks.append({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type or "image/png",
                            "data": b64,
                        },
                    })
                content_blocks.append({"type": "text", "text": prompt})
                messages = [{"role": "user", "content": content_blocks}]
            else:
                messages = [{"role": "user", "content": prompt}]
            body = {
                "model": model or "qwen3:8b",
                "max_tokens": max_tokens or 4096,
                "messages": messages,
                "stream": True,
            }
            if system_prompt:
                body["system"] = system_prompt
            if temperature is not None:
                body["temperature"] = temperature
            env_temp = settings.get("AI_TEMPERATURE", "")
            env_max_tokens = settings.get("AI_MAX_TOKENS", "")
            if env_temp and temperature is None:
                try:
                    body["temperature"] = float(env_temp)
                except ValueError:
                    pass
            if env_max_tokens and not max_tokens:
                try:
                    body["max_tokens"] = int(env_max_tokens)
                except ValueError:
                    pass
            return body

        def _stream_ollama():
            try:
                ollama_base = (settings.get("OLLAMA_BASE_URL") or "http://127.0.0.1:11434").strip()
                ollama_model = model or (settings.get("OLLAMA_MODEL") or "qwen3:8b")
                port = self._proxy_server.start(ollama_base, ollama_model)
                body = _build_anthropic_body()
                body["model"] = ollama_model
                conn = http.client.HTTPConnection("127.0.0.1", port, timeout=120)
                conn.request("POST", "/v1/messages", json.dumps(body).encode(), {"Content-Type": "application/json"})
                resp = conn.getresponse()
                try:
                    if conn.sock:
                        conn.sock.settimeout(90)
                except Exception:
                    pass
                buffer = ""
                while True:
                    try:
                        chunk = resp.read(4096)
                    except (socket.timeout, OSError):
                        break
                    if not chunk:
                        break
                    buffer += chunk.decode("utf-8", errors="ignore")
                    lines = buffer.split("\n")
                    buffer = lines.pop() or ""
                    for line in lines:
                        stripped = line.strip()
                        if not stripped:
                            continue
                        if stripped.startswith("data: "):
                            data_str = stripped[6:]
                            if data_str == "[DONE]":
                                queue.put_nowait(None)
                                return
                            try:
                                queue.put_nowait(json.loads(data_str))
                            except Exception:
                                pass
                queue.put_nowait(None)
            except Exception as e:
                queue.put_nowait({"error": str(e)})
                queue.put_nowait(None)

        def _stream_anthropic():
            try:
                api_key = settings.get("ANTHROPIC_API_KEY", "")
                base_url = (settings.get("ANTHROPIC_BASE_URL") or "https://api.anthropic.com").strip()
                anthropic_model = model or (settings.get("ANTHROPIC_MODEL") or "claude-sonnet-4-20250514")
                body = _build_anthropic_body()
                body["model"] = anthropic_model
                parsed = urllib.parse.urlparse(base_url)
                use_https = parsed.scheme == "https"
                host = parsed.hostname or "api.anthropic.com"
                port = parsed.port or (443 if use_https else 80)
                prefix = parsed.path.rstrip("/") if parsed.path else ""
                if use_https:
                    conn = http.client.HTTPSConnection(host, port, timeout=120)
                else:
                    conn = http.client.HTTPConnection(host, port, timeout=120)
                headers = {
                    "Content-Type": "application/json",
                    "x-api-key": api_key,
                    "anthropic-version": "2023-06-01",
                }
                conn.request("POST", f"{prefix}/v1/messages", json.dumps(body).encode(), headers)
                resp = conn.getresponse()
                buffer = ""
                while True:
                    try:
                        chunk = resp.read(4096)
                    except (socket.timeout, OSError):
                        break
                    if not chunk:
                        break
                    buffer += chunk.decode("utf-8", errors="ignore")
                    lines = buffer.split("\n")
                    buffer = lines.pop() or ""
                    for line in lines:
                        stripped = line.strip()
                        if not stripped:
                            continue
                        if stripped.startswith("data: "):
                            data_str = stripped[6:]
                            if data_str == "[DONE]":
                                queue.put_nowait(None)
                                return
                            try:
                                queue.put_nowait(json.loads(data_str))
                            except Exception:
                                pass
                queue.put_nowait(None)
            except Exception as e:
                queue.put_nowait({"error": str(e)})
                queue.put_nowait(None)

        def _stream_openai_compat():
            try:
                api_base = (settings.get("API_BASE_URL") or resolve_api_base_url(model)).strip()
                api_key = settings.get("API_KEY", "")
                api_model = model or settings.get("API_MODEL", "qwen3.6-plus")
                messages = []
                if system_prompt:
                    messages.append({"role": "system", "content": system_prompt})
                # 2026-06-08 TASK-2.3: images 转 OpenAI vision content blocks
                if raw_images:
                    content_blocks: list[dict] = []
                    for url in raw_images:
                        if not isinstance(url, str) or not url:
                            continue
                        # OpenAI 接受 data URL 或 https URL
                        if url.startswith("data:") or url.startswith("http"):
                            content_blocks.append({
                                "type": "image_url",
                                "image_url": {"url": url},
                            })
                        else:
                            content_blocks.append({
                                "type": "image_url",
                                "image_url": {"url": f"data:image/png;base64,{url}"},
                            })
                    content_blocks.append({"type": "text", "text": prompt})
                    messages.append({"role": "user", "content": content_blocks})
                else:
                    messages.append({"role": "user", "content": prompt})
                body = {"model": api_model, "messages": messages, "stream": True}
                if temperature is not None:
                    body["temperature"] = temperature
                if max_tokens:
                    body["max_tokens"] = max_tokens
                parsed = urllib.parse.urlparse(api_base)
                use_https = parsed.scheme == "https"
                host = parsed.hostname or "127.0.0.1"
                port = parsed.port or (443 if use_https else 80)
                if use_https:
                    conn = http.client.HTTPSConnection(host, port, timeout=120)
                else:
                    conn = http.client.HTTPConnection(host, port, timeout=120)
                headers = {"Content-Type": "application/json"}
                if api_key:
                    headers["Authorization"] = f"Bearer {api_key}"
                conn.request("POST", "/v1/chat/completions", json.dumps(body).encode(), headers)
                resp = conn.getresponse()
                buffer = ""
                while True:
                    try:
                        chunk = resp.read(4096)
                    except (socket.timeout, OSError):
                        break
                    if not chunk:
                        break
                    buffer += chunk.decode("utf-8", errors="ignore")
                    lines = buffer.split("\n")
                    buffer = lines.pop() or ""
                    for line in lines:
                        stripped = line.strip()
                        if not stripped:
                            continue
                        if stripped.startswith("data: "):
                            data_str = stripped[6:]
                            if data_str == "[DONE]":
                                queue.put_nowait(None)
                                return
                            try:
                                queue.put_nowait(json.loads(data_str))
                            except Exception:
                                pass
                queue.put_nowait(None)
            except Exception as e:
                queue.put_nowait({"error": str(e)})
                queue.put_nowait(None)

        if provider == "ollama":
            target = _stream_ollama
        elif provider == "anthropic":
            target = _stream_anthropic
        else:
            target = _stream_openai_compat

        thread = threading.Thread(target=target, daemon=True)
        thread.start()
        if session_id:
            self._sessions[session_id] = {"thread": thread, "active": True}

        while True:
            chunk = await queue.get()
            if chunk is None:
                break
            yield json.dumps(chunk, ensure_ascii=False)

        if session_id:
            self._sessions.pop(session_id, None)

    async def stop_chat(self, session_id=None):
        if session_id and session_id in self._sessions:
            self._sessions[session_id]["active"] = False
            self._sessions.pop(session_id, None)
            return {"ok": True, "message": "对话已停止"}
        return {"ok": True, "message": "无活跃对话"}

    async def list_models(self, provider, base_url=None, api_key=None, check_health=False):
        if provider == "ollama":
            return list_ollama_models(base_url or "", check_health=check_health)
        elif provider == "openrouter":
            return list_openrouter_models()
        elif provider == "zhipu":
            return list_zhipu_models(api_key or "", base_url or "")
        elif provider == "anthropic":
            return list_anthropic_models(api_key or "")
        elif provider == "api":
            return list_api_models(base_url or "", api_key or "")
        return {"ok": False, "error": f"不支持的提供商: {provider}"}

    async def list_providers(self):
        return {
            "providers": [
                {"id": "ollama", "name": "Ollama", "type": "local", "description": "本地大语言模型"},
                {"id": "anthropic", "name": "Anthropic", "type": "cloud", "description": "Claude 系列模型"},
                {"id": "openrouter", "name": "OpenRouter", "type": "cloud", "description": "多模型聚合平台"},
                {"id": "zhipu", "name": "智谱", "type": "cloud", "description": "GLM 系列模型"},
                {"id": "api", "name": "API服务", "type": "local", "description": "本地API代理服务"},
            ]
        }

    async def check_provider(self, provider, base_url=None, api_key=None):
        if provider == "zhipu":
            return check_zhipu_api(api_key or "", base_url or "")
        elif provider == "ollama":
            ollama_url = (base_url or "http://127.0.0.1:11434").strip()
            result = fetch_json_with_timeout(f"{ollama_url.rstrip('/')}/api/tags", timeout_ms=5000)
            if result["ok"]:
                return {"ok": True, "running": True, "message": "Ollama 连接成功"}
            return {"ok": False, "error": "Ollama 服务未运行"}
        elif provider == "api":
            return check_api_service(base_url or "")
        elif provider == "anthropic":
            if not api_key:
                return {"ok": False, "error": "请输入 Anthropic API Key"}
            result = list_anthropic_models(api_key)
            if result.get("ok"):
                return {"ok": True, "running": True, "message": "Anthropic API 连接成功"}
            return {"ok": False, "error": result.get("error", "连接失败")}
        elif provider == "openrouter":
            result = list_openrouter_models()
            if result.get("ok"):
                return {"ok": True, "running": True, "message": "OpenRouter 连接成功"}
            return {"ok": False, "error": result.get("error", "连接失败")}
        return {"ok": False, "error": f"不支持的提供商: {provider}"}

    async def get_ollama_capabilities(self, base_url, model_name):
        ollama_url = (base_url or "http://127.0.0.1:11434").strip()
        caps = fetch_ollama_capabilities(ollama_url, model_name)
        if caps:
            return {"ok": True, "capabilities": caps}
        return {"ok": True, "capabilities": {}}

    async def check_ollama_health(self, base_url, model_name, timeout_ms=30000):
        ollama_url = (base_url or "http://127.0.0.1:11434").strip()
        return check_ollama_model_health(ollama_url, model_name, timeout_ms)
