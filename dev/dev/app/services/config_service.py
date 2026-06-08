import os
import re

MODEL_KEYS = [
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "OLLAMA_MODEL",
    "ZHIPU_MODEL",
]

SETTINGS_KEYS = [
    "MODEL_PROVIDER",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "OLLAMA_BASE_URL",
    "OLLAMA_MODEL",
    "API_BASE_URL",
    "API_MODEL",
    "API_KEY",
    "API_SOURCE",
    "API_TIMEOUT_MS",
    "ZHIPU_API_KEY",
    "ZHIPU_MODEL",
    "ZHIPU_BASE_URL",
    "DISABLE_TELEMETRY",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "AI_LANGUAGE",
    "AI_TEMPERATURE",
    "AI_MAX_TOKENS",
    "SYSTEM_PROMPT",
]


class EnvFileManager:
    """读写 .env 文件中的配置项"""

    def __init__(self, env_path: str):
        self.env_path = env_path

    def parse_env_lines(self, raw: str):
        lines = raw.splitlines()
        kv = {}
        for line in lines:
            if not line or line.strip().startswith("#"):
                continue
            idx = line.index("=") if "=" in line else -1
            if idx <= 0:
                continue
            kv[line[:idx].strip()] = line[idx + 1:]
        return lines, kv

    def read_settings(self) -> dict:
        if not os.path.exists(self.env_path):
            return {}
        with open(self.env_path, "r", encoding="utf-8") as f:
            raw = f.read()
        _, kv = self.parse_env_lines(raw)
        return {k: kv.get(k, "") for k in SETTINGS_KEYS}

    def write_settings(self, partial: dict) -> dict:
        partial = partial or {}
        existing = ""
        if os.path.exists(self.env_path):
            with open(self.env_path, "r", encoding="utf-8") as f:
                existing = f.read()
        lines, kv = self.parse_env_lines(existing)
        for key in SETTINGS_KEYS:
            if key in partial:
                kv[key] = str(partial[key] or "")

        used = set()
        next_lines = []
        for line in lines:
            idx = line.index("=") if "=" in line else -1
            if idx <= 0:
                next_lines.append(line)
                continue
            key = line[:idx].strip()
            if key not in SETTINGS_KEYS:
                next_lines.append(line)
                continue
            used.add(key)
            next_lines.append(f"{key}={kv.get(key, '')}")

        for key in SETTINGS_KEYS:
            if key not in used and key in kv:
                next_lines.append(f"{key}={kv[key]}")

        content = "\n".join(next_lines)
        content = re.sub(r"\n{3,}", "\n\n", content).strip() + "\n"
        with open(self.env_path, "w", encoding="utf-8") as f:
            f.write(content)
        return self.read_settings()

    def clear_model_settings(self) -> dict:
        reset = {k: "" for k in MODEL_KEYS}
        return self.write_settings(reset)


def _get_base_dir() -> str:
    import sys
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


class ConfigService:
    def __init__(self):
        base_dir = _get_base_dir()
        self._base_dir = base_dir
        self._data_dir = os.path.join(base_dir, "data")
        self._env_path = os.path.join(self._data_dir, ".env")
        self._app_data_path = os.path.join(self._data_dir, "app_data.json")
        self._plugins_dir = os.path.join(self._data_dir, "plugins")

    def _env_manager(self) -> EnvFileManager:
        return EnvFileManager(self._env_path)

    async def get_state(self):
        import platform
        settings = self._env_manager().read_settings()
        provider = settings.get("MODEL_PROVIDER", "")
        model = ""
        if provider == "ollama":
            model = settings.get("OLLAMA_MODEL", "")
        elif provider == "anthropic":
            model = settings.get("ANTHROPIC_MODEL", "")
        elif provider == "api":
            model = settings.get("API_MODEL", "")
        elif provider == "zhipu":
            model = settings.get("ZHIPU_MODEL", "")
        return {
            "ok": True,
            "platform": platform.system(),
            "python": platform.python_version(),
            "provider": provider,
            "model": model,
            "dataDir": self._data_dir,
        }

    async def get_settings(self):
        return self._env_manager().read_settings()

    async def save_settings(self, settings: dict):
        return self._env_manager().write_settings(settings)

    async def clear_model_settings(self):
        return self._env_manager().clear_model_settings()

    async def detect_hardware(self):
        import platform
        info = {
            "ok": True,
            "platform": platform.system(),
            "platformRelease": platform.release(),
            "platformVersion": platform.version(),
            "architecture": platform.machine(),
            "processor": platform.processor(),
            "python": platform.python_version(),
        }
        try:
            import psutil
            info["cpuCount"] = psutil.cpu_count(logical=True)
            info["cpuCountPhysical"] = psutil.cpu_count(logical=False)
            mem = psutil.virtual_memory()
            info["memoryTotal"] = mem.total
            info["memoryAvailable"] = mem.available
            info["memoryPercent"] = mem.percent
        except ImportError:
            try:
                info["cpuCount"] = os.cpu_count() or 0
            except Exception:
                pass
        try:
            import subprocess
            r = subprocess.run(
                ["nvidia-smi", "--query-gpu=name,memory.total", "--format=csv,noheader"],
                capture_output=True, text=True, timeout=5,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            if r.returncode == 0:
                gpus = []
                for line in r.stdout.strip().splitlines():
                    parts = [p.strip() for p in line.split(",")]
                    if len(parts) >= 2:
                        gpus.append({"name": parts[0], "memory": parts[1]})
                info["gpus"] = gpus
        except Exception:
            pass
        return info

    async def get_workspace(self):
        settings = self._env_manager().read_settings()
        return {"path": os.getcwd(), "settingsWorkspace": settings.get("WORKSPACE_PATH", "")}

    async def choose_workspace(self):
        import tkinter as tk
        from tkinter import filedialog
        root = tk.Tk()
        root.withdraw()
        root.attributes("-topmost", True)
        path = filedialog.askdirectory(title="选择工作目录")
        root.destroy()
        if path:
            return {"ok": True, "path": path}
        return {"ok": False, "path": ""}

    async def run_command(self, cmd: str, cwd: str = None):
        import subprocess
        try:
            r = subprocess.run(
                cmd, shell=True, capture_output=True, text=True, timeout=60,
                cwd=cwd or None,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {
                "ok": r.returncode == 0,
                "stdout": r.stdout,
                "stderr": r.stderr,
                "returnCode": r.returncode,
            }
        except subprocess.TimeoutExpired:
            return {"ok": False, "error": "命令执行超时"}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def load_app_data(self):
        if not os.path.exists(self._app_data_path):
            return {"ok": True, "data": "{}"}
        try:
            with open(self._app_data_path, "r", encoding="utf-8") as f:
                content = f.read()
            return {"ok": True, "data": content}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def save_app_data(self, data: str):
        try:
            os.makedirs(os.path.dirname(self._app_data_path), exist_ok=True)
            with open(self._app_data_path, "w", encoding="utf-8") as f:
                f.write(data)
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def get_git_status(self, project_path: str):
        import subprocess
        try:
            r = subprocess.run(
                ["git", "status", "--porcelain"],
                capture_output=True, text=True, cwd=project_path, timeout=10,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            files = []
            for line in r.stdout.strip().splitlines():
                if line.strip():
                    status = line[:2].strip()
                    filepath = line[3:].strip()
                    files.append({"status": status, "path": filepath})
            r2 = subprocess.run(
                ["git", "rev-parse", "--short", "HEAD"],
                capture_output=True, text=True, cwd=project_path, timeout=5,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            branch_r = subprocess.run(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"],
                capture_output=True, text=True, cwd=project_path, timeout=5,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {
                "ok": True,
                "branch": branch_r.stdout.strip() if branch_r.returncode == 0 else "",
                "commit": r2.stdout.strip() if r2.returncode == 0 else "",
                "files": files,
            }
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def git_commit(self, project_path: str, message: str):
        import subprocess
        try:
            subprocess.run(
                ["git", "add", "-A"],
                capture_output=True, cwd=project_path, timeout=30,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            r = subprocess.run(
                ["git", "commit", "-m", message],
                capture_output=True, text=True, cwd=project_path, timeout=30,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {"ok": r.returncode == 0, "output": r.stdout or r.stderr}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def get_git_log(self, project_path: str):
        import subprocess
        try:
            r = subprocess.run(
                ["git", "log", "-20", "--oneline", "--format=%h|%s|%an|%ar"],
                capture_output=True, text=True, cwd=project_path, timeout=10,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            commits = []
            for line in r.stdout.strip().splitlines():
                parts = line.strip().split("|", 3)
                if len(parts) >= 4:
                    commits.append({
                        "hash": parts[0],
                        "message": parts[1],
                        "author": parts[2],
                        "time": parts[3],
                    })
            return {"ok": True, "commits": commits}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def get_file_tree(self, project_path: str):
        ignore_dirs = {".git", "node_modules", "__pycache__", ".venv", "venv", ".idea", ".vscode", "dist", "build"}
        tree = []
        try:
            for root, dirs, files in os.walk(project_path):
                dirs[:] = [d for d in dirs if d not in ignore_dirs]
                rel = os.path.relpath(root, project_path)
                if rel == ".":
                    rel = ""
                for f in files:
                    fp = os.path.join(rel, f) if rel else f
                    tree.append(fp)
        except Exception:
            pass
        return {"ok": True, "files": tree}

    async def show_notification(self, title: str, body: str):
        try:
            if os.name == "nt":
                from ctypes import windll
                windll.user32.MessageBoxTimeoutW(0, body, title, 0x40, 0, 3000)
                return {"ok": True}
        except Exception:
            pass
        try:
            import subprocess
            if os.name == "nt":
                ps_script = f'[System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms"); $n = New-Object System.Windows.Forms.NotifyIcon; $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $true; $n.ShowBalloonTip(3000, "{title}", "{body}", [System.Windows.Forms.ToolTipIcon]::Info); Start-Sleep -Seconds 4; $n.Dispose()'
                subprocess.run(
                    ["powershell.exe", "-WindowStyle", "Hidden", "-Command", ps_script],
                    capture_output=True, timeout=10,
                    creationflags=subprocess.CREATE_NO_WINDOW,
                )
                return {"ok": True}
        except Exception as e:
            return {"ok": False, "error": str(e)}
        return {"ok": False, "error": "不支持的通知方式"}

    async def open_url(self, url: str):
        import webbrowser
        try:
            webbrowser.open(url)
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def open_in_explorer(self, path: str):
        try:
            if os.name == "nt":
                os.startfile(path)
            else:
                import subprocess
                subprocess.run(["xdg-open", path], capture_output=True, timeout=5)
            return {"ok": True}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def select_directory(self):
        import tkinter as tk
        from tkinter import filedialog
        root = tk.Tk()
        root.withdraw()
        root.attributes("-topmost", True)
        path = filedialog.askdirectory(title="选择目录")
        root.destroy()
        if path:
            return {"ok": True, "path": path}
        return {"ok": False, "path": ""}

    async def list_plugins(self):
        if not os.path.exists(self._plugins_dir):
            return {"ok": True, "plugins": []}
        plugins = []
        for fname in os.listdir(self._plugins_dir):
            if fname.endswith(".json"):
                fpath = os.path.join(self._plugins_dir, fname)
                try:
                    with open(fpath, "r", encoding="utf-8") as f:
                        data = json.load(f)
                    data["id"] = fname[:-5]
                    plugins.append(data)
                except Exception:
                    pass
        return {"ok": True, "plugins": plugins}

    async def install_plugin(self, plugin_json: str):
        try:
            data = json.loads(plugin_json)
            plugin_id = data.get("id", "")
            if not plugin_id:
                return {"ok": False, "error": "插件缺少 id"}
            os.makedirs(self._plugins_dir, exist_ok=True)
            fpath = os.path.join(self._plugins_dir, f"{plugin_id}.json")
            with open(fpath, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            return {"ok": True, "id": plugin_id}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def uninstall_plugin(self, plugin_id: str):
        fpath = os.path.join(self._plugins_dir, f"{plugin_id}.json")
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return {"ok": True}
            except Exception as e:
                return {"ok": False, "error": str(e)}
        return {"ok": False, "error": "插件不存在"}

    async def execute_plugin(self, plugin_id: str, input_data: str = ""):
        fpath = os.path.join(self._plugins_dir, f"{plugin_id}.json")
        if not os.path.exists(fpath):
            return {"ok": False, "error": "插件不存在"}
        try:
            with open(fpath, "r", encoding="utf-8") as f:
                data = json.load(f)
            cmd = data.get("command", data.get("exec", ""))
            if not cmd:
                return {"ok": False, "error": "插件未配置执行命令"}
            import subprocess
            r = subprocess.run(
                cmd, shell=True, capture_output=True, text=True, timeout=60,
                input=input_data or None,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {"ok": r.returncode == 0, "stdout": r.stdout, "stderr": r.stderr}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def get_offline_models(self):
        models_dir = os.path.join(self._data_dir, "models")
        if not os.path.exists(models_dir):
            return {"ok": True, "models": []}
        models = []
        for fname in os.listdir(models_dir):
            fpath = os.path.join(models_dir, fname)
            if os.path.isfile(fpath):
                size = os.path.getsize(fpath)
                models.append({"name": fname, "size": size, "path": fpath})
        return {"ok": True, "models": models}

    async def download_model(self, url: str):
        import urllib.request
        models_dir = os.path.join(self._data_dir, "models")
        os.makedirs(models_dir, exist_ok=True)
        fname = url.split("/")[-1] or "model.bin"
        dest = os.path.join(models_dir, fname)
        try:
            urllib.request.urlretrieve(url, dest)
            return {"ok": True, "path": dest}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def delete_model(self, name: str):
        models_dir = os.path.join(self._data_dir, "models")
        fpath = os.path.join(models_dir, name)
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return {"ok": True}
            except Exception as e:
                return {"ok": False, "error": str(e)}
        return {"ok": False, "error": "模型文件不存在"}

    async def search_ollama_library(self, query: str = ""):
        import urllib.request
        import json as _json
        try:
            url = f"https://registry.ollama.ai/api/models?q={query}" if query else "https://registry.ollama.ai/api/models"
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            resp = urllib.request.urlopen(req, timeout=10)
            data = _json.loads(resp.read().decode("utf-8"))
            return {"ok": True, "models": data if isinstance(data, list) else []}
        except Exception as e:
            return {"ok": False, "error": str(e), "models": []}

    async def pull_model(self, name: str):
        import subprocess
        try:
            r = subprocess.run(
                ["ollama", "pull", name],
                capture_output=True, text=True, timeout=600,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {"ok": r.returncode == 0, "output": r.stdout or r.stderr}
        except FileNotFoundError:
            return {"ok": False, "error": "Ollama 未安装"}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def recommend_models(self):
        return {
            "ok": True,
            "models": [
                {"name": "qwen3:8b", "desc": "通义千问3 8B，中文能力强，支持工具调用", "size": "5.2GB", "toolSupport": True},
                {"name": "qwen3:4b", "desc": "通义千问3 4B，轻量版，适合低配设备", "size": "2.6GB", "toolSupport": True},
                {"name": "llama3.1:8b", "desc": "Llama 3.1 8B，Meta开源模型", "size": "4.9GB", "toolSupport": True},
                {"name": "gemma3:4b", "desc": "Gemma 3 4B，Google开源模型", "size": "3.3GB", "toolSupport": True},
                {"name": "deepseek-r1:8b", "desc": "DeepSeek R1 8B，推理能力强", "size": "5.2GB", "toolSupport": False},
                {"name": "phi4:14b", "desc": "Phi-4 14B，微软小型高效模型", "size": "9.1GB", "toolSupport": True},
            ],
        }
