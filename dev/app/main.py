#!/usr/bin/env python3
"""
云集智能编程工作站 - 统一启动器 v3.0
所有功能内嵌在一个 EXE 中，不再依赖 Electron

架构:
- PyQt6 + QWebEngineView 替代 Electron
- QWebChannel 替代 Electron IPC (preload.cjs)
- backend.py 提供 Ollama 代理 / CLI 管理 / 配置管理
- Vue 前端通过 QWebChannel 与 Python 通信
- 部署维护只是 EXE 的一个功能模块
"""

import sys
import os
import json
import time
import subprocess
import threading
import traceback
import zipfile
import shutil
import urllib.request
import urllib.error
import urllib.parse
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional

import psutil

from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLabel, QTextEdit, QFrame, QProgressBar,
    QMessageBox, QFileDialog, QStackedWidget, QSizePolicy,
    QTabWidget, QScrollArea, QComboBox,
)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, pyqtSlot, QTimer, QUrl
from PyQt6.QtGui import QFont, QIcon, QColor
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWebChannel import QWebChannel

from PyQt6.QtCore import QObject

# 导入后端模块
import backend
from backend import (
    EnvFileManager, ClaudeCliRunner,
    list_openrouter_models, list_anthropic_models, list_ollama_models, list_api_models,
    SETTINGS_KEYS,
)


# ── 版本号 ──
def get_version_from_filename():
    try:
        if hasattr(sys, 'frozen'):
            exe_name = os.path.basename(sys.executable)
            import re
            m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', exe_name)
            if m:
                return m.group(1)
        return datetime.now().strftime("%Y.%m.%d.%H%M")
    except:
        return datetime.now().strftime("%Y.%m.%d.%H%M")


VERSION = get_version_from_filename()

# ── 环境路径常量 ──
NODE_VERSION = "v24.11.1"
NODE_DIR_NAME = f"node-{NODE_VERSION}-win-x64"
BUN_VERSION = "1.1.42"
BUN_DIR_NAME = "bun-windows-x64"
UV_VERSION = "0.11.7"
UV_PYTHON_VERSION = "3.12"

# ── 下载源配置 ──
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
MIRROR_SETTINGS_FILE = "mirror_source.json"

# ── Git 仓库配置 ──
GIT_REMOTE = "git@gitee.com:yunjii/code.git"
GIT_BRANCH = "main"
REMOTE_VERSIONS_URL = f"https://gitee.com/yunjii/code/raw/{GIT_BRANCH}/app/version_history.json"


# ── 软件更新器 ──
class SoftwareUpdater:
    """基于 Git 的软件更新和 EXE 版本切换"""

    def __init__(self, dev_dir: str, log_func=None, progress_func=None):
        self.dev_dir = dev_dir          # dev/ 根目录（Git 仓库）
        self.app_dir = os.path.join(dev_dir, "app")  # 资源包
        self.ver_dir = os.path.join(dev_dir, "ver")   # 稳定版 EXE
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func

    def _si(self):
        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        return si

    def _run_git(self, *args, cwd=None, timeout=60):
        """执行 git 命令"""
        cmd = ["git"] + list(args)
        try:
            r = subprocess.run(
                cmd, cwd=cwd or self.dev_dir,
                capture_output=True, text=True, timeout=timeout,
                startupinfo=self._si(),
                encoding="utf-8", errors="replace",
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return {"ok": r.returncode == 0, "stdout": r.stdout.strip(), "stderr": r.stderr.strip(), "code": r.returncode}
        except subprocess.TimeoutExpired:
            return {"ok": False, "stdout": "", "stderr": "命令超时", "code": -1}
        except Exception as e:
            return {"ok": False, "stdout": "", "stderr": str(e), "code": -1}

    def is_git_repo(self):
        """检查 dev_dir 是否是 Git 仓库"""
        r = self._run_git("rev-parse", "--is-inside-work-tree")
        return r["ok"] and r["stdout"] == "true"

    def get_current_commit(self):
        """获取当前 commit hash"""
        r = self._run_git("rev-parse", "--short", "HEAD")
        return r["stdout"] if r["ok"] else "unknown"

    def get_remote_commit(self):
        """获取远程最新 commit hash（不合并）"""
        r = self._run_git("fetch", "origin", GIT_BRANCH, timeout=30)
        if not r["ok"]:
            return None
        r2 = self._run_git("rev-parse", "--short", f"origin/{GIT_BRANCH}")
        return r2["stdout"] if r2["ok"] else None

    def check_update(self):
        """检查是否有资源包更新，返回 {has_update, local, remote}"""
        if not self.is_git_repo():
            return {"has_update": False, "error": "不是 Git 仓库，无法检查更新"}

        local = self.get_current_commit()
        self.log(f"本地版本: {local}")

        remote = self.get_remote_commit()
        if remote is None:
            return {"has_update": False, "local": local, "remote": "无法获取", "error": "无法连接远程仓库"}

        self.log(f"远程版本: {remote}")
        has_update = local != remote
        if has_update:
            self.log(f"发现资源包更新: {local} → {remote}", "#4CAF50")
        else:
            self.log("资源包已是最新版本")
        return {"has_update": has_update, "local": local, "remote": remote}

    def pull_update(self):
        """拉取资源包更新（git pull）"""
        if not self.is_git_repo():
            self.log("[错误] 不是 Git 仓库，无法更新", "#F44336")
            return False

        self.log("正在更新资源包...")
        self.progress(10, "正在拉取远程更新...")

        # git stash 保存本地修改（如 .env）
        r = self._run_git("stash")
        stashed = r["ok"] and "Saved" in r["stdout"]

        # git pull
        r = self._run_git("pull", "origin", GIT_BRANCH, timeout=120)
        if not r["ok"]:
            self.log(f"[错误] 更新失败: {r['stderr'][:200]}", "#F44336")
            if stashed:
                self._run_git("stash", "pop")
            return False

        self.progress(70, "正在恢复本地配置...")

        # 恢复 stash
        if stashed:
            self._run_git("stash", "pop")

        new_commit = self.get_current_commit()
        self.log(f"✓ 资源包已更新到 {new_commit}", "#4CAF50")
        self.progress(100, "更新完成")
        return True

    def list_stable_exes(self):
        """列出 ver/ 目录中的稳定版 EXE"""
        if not os.path.isdir(self.ver_dir):
            return []

        exes = []
        for f in os.listdir(self.ver_dir):
            if f.endswith(".exe"):
                path = os.path.join(self.ver_dir, f)
                # 从文件名提取版本号
                import re
                m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', f)
                ver = m.group(1) if m else "unknown"
                size_mb = os.path.getsize(path) / (1024 * 1024)
                exes.append({
                    "filename": f,
                    "path": path,
                    "version": ver,
                    "size_mb": round(size_mb, 1),
                })

        # 按版本号降序排列
        exes.sort(key=lambda x: x["version"], reverse=True)
        return exes

    def switch_to_exe(self, exe_path: str, git_commit: str = ""):
        """切换到指定 EXE 并重启，同时回滚代码到对应 git commit"""
        if not os.path.exists(exe_path):
            self.log(f"[错误] EXE 不存在: {exe_path}", "#F44336")
            return False

        if git_commit and self.is_git_repo():
            self.log(f"正在回滚代码到 commit {git_commit}...", "#FF9800")
            r = self._run_git("stash")
            stashed = r["ok"] and "Saved" in r["stdout"]
            r = self._run_git("checkout", git_commit, timeout=30)
            if not r["ok"]:
                self.log(f"[警告] 代码回滚失败: {r['stderr'][:200]}", "#FF9800")
                if stashed:
                    self._run_git("stash", "pop")
            else:
                self.log(f"✓ 代码已回滚到 {git_commit}", "#4CAF50")
                if stashed:
                    self._run_git("stash", "pop")

        current_pid = os.getpid()
        new_exe = exe_path
        cmd = f'ping -n 3 127.0.0.1 >nul & start "" "{new_exe}"'
        subprocess.Popen(cmd, shell=True, creationflags=subprocess.CREATE_NO_WINDOW)

        self.log(f"正在切换到 {os.path.basename(exe_path)}...", "#4CAF50")

        QApplication.quit()
        return True

    def fetch_remote_version_history(self):
        """通过 HTTP API 获取远程版本历史（零 subprocess，零弹窗）"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            req = Request(REMOTE_VERSIONS_URL)
            req.add_header('User-Agent', 'Mozilla/5.0')
            resp = urlopen(req, timeout=10)
            content = resp.read().decode('utf-8')
            data = json.loads(content)
            if isinstance(data, list):
                return data
            elif isinstance(data, dict):
                result = []
                for vname, vinfo in data.items():
                    entry = dict(vinfo)
                    entry["name"] = vname
                    if "version_number" in entry and "version" not in entry:
                        entry["version"] = entry["version_number"]
                    result.append(entry)
                result.sort(key=lambda x: x.get("version", ""), reverse=True)
                return result
            return []
        except HTTPError as e:
            print(f"远程版本获取失败 (HTTP {e.code}): {e.reason}")
            return None
        except URLError as e:
            print(f"远程版本获取失败 (网络错误): {e.reason}")
            return None
        except Exception as e:
            print(f"远程版本获取失败: {e}")
            return None

    def get_local_version_history(self):
        """获取本地版本历史（返回 list 格式）"""
        path = os.path.join(self.app_dir, "version_history.json")
        if not os.path.exists(path):
            return []
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            if isinstance(data, list):
                return data
            elif isinstance(data, dict):
                result = []
                for vname, vinfo in data.items():
                    entry = dict(vinfo)
                    entry["name"] = vname
                    if "version_number" in entry and "version" not in entry:
                        entry["version"] = entry["version_number"]
                    result.append(entry)
                result.sort(key=lambda x: x.get("version", ""), reverse=True)
                return result
            return []
        except:
            return []

    def compare_versions(self, local_versions, remote_versions):
        """对比本地和远程版本，返回远程新增版本列表"""
        if not remote_versions:
            return []

        local_ver_set = set()
        for v in (local_versions or []):
            ver = v.get("version", v.get("version_number", ""))
            if ver:
                local_ver_set.add(ver)

        new_versions = []
        for v in remote_versions:
            ver = v.get("version", v.get("version_number", ""))
            if ver and ver not in local_ver_set:
                new_versions.append(v)

        return new_versions


# ── QWebChannel 桥接对象 (替代 Electron preload.cjs) ──
class BackendBridge(QObject):
    """暴露给前端 JS 的 Python 对象，替代 Electron 的 desktopApi"""

    # 信号：前端通过 onDelta/onStatus/onModelsLoaded 连接
    deltaReceived = pyqtSignal(str)   # JSON string: {"text": "..."}
    statusReceived = pyqtSignal(str)  # JSON string: {"busy": true, ...}
    modelsLoaded = pyqtSignal(str)    # JSON string: {"ok": true, "models": [...]}

    def __init__(self, parent=None):
        super().__init__(parent)
        self._app_ref = None  # 由 MainWindow 设置

    def _get_main(self):
        return self._app_ref

    # ── 前端可调用方法 (通过 pyqtSlot 暴露给 QWebChannel) ──

    @pyqtSlot(result=str)
    def getState(self):
        """获取应用状态"""
        main = self._get_main()
        if not main:
            return json.dumps({})
        env = main.env_manager
        settings = env.read_settings()
        return json.dumps({
            "sessionId": main.active_session_id,
            "model": settings.get("ANTHROPIC_MODEL", "") or settings.get("OLLAMA_MODEL", ""),
            "busy": main.is_busy,
            "settings": settings,
            "workspacePath": main.current_workspace,
        })

    @pyqtSlot(result=str)
    def newSession(self):
        main = self._get_main()
        if not main:
            return json.dumps({"sessionId": ""})
        if main.is_busy:
            return json.dumps({"sessionId": main.active_session_id or ""})
        main.active_session_id = _uuid()
        main.started_sessions.discard(main.active_session_id)
        return json.dumps({"sessionId": main.active_session_id})

    @pyqtSlot(str, result=str)
    def sendMessage(self, payload_json: str):
        """发送消息给 AI"""
        main = self._get_main()
        if not main:
            return json.dumps({"ok": False, "error": "App not ready"})

        try:
            payload = json.loads(payload_json) if isinstance(payload_json, str) else payload_json
        except:
            return json.dumps({"ok": False, "error": "Invalid payload"})

        if main.is_busy:
            if main.active_proc and main.active_proc.poll() is None:
                try:
                    main.active_proc.kill()
                    main.log_signal.emit("[恢复] 终止了残留的CLI进程", "#FF9800")
                except:
                    pass
                try:
                    main.active_proc.wait(timeout=3)
                except:
                    pass
            main.is_busy = False
            main.active_proc = None
            self.statusReceived.emit(json.dumps({"busy": False}))

        prompt = (payload.get("prompt") or "").strip()
        if not prompt:
            return json.dumps({"ok": False, "error": "Prompt cannot be empty."})

        env = main.env_manager
        settings = env.read_settings()
        provider = (payload.get("provider") or settings.get("MODEL_PROVIDER", "anthropic")).lower()
        model = (payload.get("model") or "").strip()
        if not model:
            model = settings.get("OLLAMA_MODEL", "") if provider == "ollama" else settings.get("ANTHROPIC_MODEL", "")

        if payload.get("ai_language"):
            settings["AI_LANGUAGE"] = payload["ai_language"]
        if payload.get("ai_temperature"):
            settings["AI_TEMPERATURE"] = payload["ai_temperature"]
        if payload.get("ai_max_tokens"):
            settings["AI_MAX_TOKENS"] = payload["ai_max_tokens"]
        if payload.get("system_prompt"):
            settings["SYSTEM_PROMPT"] = payload["system_prompt"]

        main.is_busy = True
        self.statusReceived.emit(json.dumps({"busy": True}))

        t = threading.Thread(target=self._run_cli, args=(prompt, model, provider, settings), daemon=True)
        t.start()

        return json.dumps({"ok": True, "sessionId": main.active_session_id})

    @pyqtSlot(result=str)
    def stopMessage(self):
        main = self._get_main()
        if not main or not main.is_busy:
            return json.dumps({"ok": False, "error": "No running task."})

        if main.active_proc and main.active_proc.poll() is None:
            try:
                main.active_proc.kill()
            except:
                pass

        main.is_busy = False
        main.active_session_id = _uuid()
        self.statusReceived.emit(json.dumps({"busy": False}))
        return json.dumps({"ok": True, "sessionId": main.active_session_id})

    @pyqtSlot(result=str)
    def getWorkspace(self):
        main = self._get_main()
        path = main.current_workspace if main else ""
        return json.dumps({"path": path})

    @pyqtSlot(result=str)
    def chooseWorkspace(self):
        """由 Python 端弹出文件夹选择对话框"""
        main = self._get_main()
        if not main:
            return json.dumps({"ok": False, "error": "App not ready"})
        # 需要在主线程执行，用信号通知
        main.workspace_choose_requested.emit()
        return json.dumps({"ok": True, "path": main.current_workspace})

    @pyqtSlot(result=str)
    def getSettings(self):
        main = self._get_main()
        if not main:
            return json.dumps({})
        return json.dumps(main.env_manager.read_settings())

    @pyqtSlot(str, result=str)
    def saveSettings(self, payload_json: str):
        main = self._get_main()
        if not main:
            return json.dumps({})
        try:
            payload = json.loads(payload_json) if isinstance(payload_json, str) else payload_json
        except:
            return json.dumps({})
        result = main.env_manager.write_settings(payload)
        return json.dumps(result)

    @pyqtSlot(result=str)
    def clearModelSettings(self):
        main = self._get_main()
        if not main:
            return json.dumps({})
        result = main.env_manager.clear_model_settings()
        return json.dumps(result)

    @pyqtSlot(str, result=str)
    def listModels(self, payload_json: str):
        """获取模型列表（在后台线程中执行，通过信号返回结果）"""
        try:
            payload = json.loads(payload_json) if isinstance(payload_json, str) else payload_json
        except:
            return json.dumps({"ok": False, "error": "Invalid payload"})

        source = (payload.get("source") or "").lower()
        main = self._get_main()
        settings = main.env_manager.read_settings() if main else {}
        timeout = int(settings.get("API_TIMEOUT_MS", "15000") or "15000")

        def _do_load():
            if source == "openrouter":
                result = list_openrouter_models(timeout)
            elif source == "anthropic":
                api_key = payload.get("apiKey", "") or settings.get("ANTHROPIC_API_KEY", "")
                result = list_anthropic_models(api_key, timeout)
            elif source == "ollama":
                base_url = payload.get("baseUrl", "") or settings.get("OLLAMA_BASE_URL", "") or "http://127.0.0.1:11434"
                check_health = payload.get("checkHealth", False)
                result = list_ollama_models(base_url, timeout, check_health=check_health)
            elif source == "api":
                api_base = payload.get("baseUrl", "") or settings.get("API_BASE_URL", "") or "http://127.0.0.1:7860"
                api_key = payload.get("apiKey", "") or settings.get("API_KEY", "")
                result = list_api_models(api_base, api_key, timeout)
            else:
                result = {"ok": False, "error": "Unsupported source."}
            self.modelsLoaded.emit(json.dumps(result))

        t = threading.Thread(target=_do_load, daemon=True)
        t.start()
        return json.dumps({"ok": True, "loading": True})

    @pyqtSlot(result=str)
    def detectHardware(self):
        """检测硬件信息，用于自动配置推荐"""
        info = {"total_ram": 0, "gpu_name": "", "gpu_vram_gb": 0, "cpu_name": "", "cpu_cores": 0}

        try:
            import psutil as _ps
            info["total_ram"] = _ps.virtual_memory().total
            info["cpu_cores"] = _ps.cpu_count(logical=False) or _ps.cpu_count(logical=True) or 0
        except:
            pass

        try:
            import platform
            info["cpu_name"] = platform.processor() or ""
        except:
            pass

        try:
            result = subprocess.run(
                ["nvidia-smi", "--query-gpu=name,memory.total", "--format=csv,noheader,nounits"],
                capture_output=True, text=True, timeout=5,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            if result.returncode == 0:
                line = result.stdout.strip().split("\n")[0].strip()
                if "," in line:
                    name_part, vram_part = line.split(",", 1)
                    info["gpu_name"] = name_part.strip()
                    try:
                        info["gpu_vram_gb"] = round(float(vram_part.strip()) / 1024, 1)
                    except:
                        pass
        except:
            pass

        return json.dumps(info)

    @pyqtSlot(str, result=str)
    def deleteModel(self, payload_json: str):
        try:
            payload = json.loads(payload_json) if isinstance(payload_json, str) else payload_json
        except:
            return json.dumps({"ok": False, "error": "Invalid payload"})
        model_name = (payload.get("name") or "").strip()
        if not model_name:
            return json.dumps({"ok": False, "error": "模型名称不能为空"})
        main = self._get_main()
        settings = main.env_manager.read_settings() if main else {}
        base_url = (settings.get("OLLAMA_BASE_URL", "") or "http://127.0.0.1:11434").strip()
        try:
            url = f"{base_url.rstrip('/')}/api/delete"
            body = json.dumps({"name": model_name}).encode("utf-8")
            req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"}, method="DELETE")
            resp = urllib.request.urlopen(req, timeout=15)
            return json.dumps({"ok": True})
        except urllib.error.HTTPError as e:
            err = ""
            try:
                err = e.read().decode("utf-8")[:200]
            except:
                pass
            return json.dumps({"ok": False, "error": f"删除失败({e.code}): {err}"})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)[:200]})

    @pyqtSlot(result=str)
    def recommendModels(self):
        hw = json.loads(self.detectHardware())
        total_ram_gb = (hw.get("total_ram", 0) or 0) / (1024 ** 3)
        gpu_vram_gb = hw.get("gpu_vram_gb", 0) or 0
        recommendations = []
        if gpu_vram_gb >= 20:
            recommendations.append({"name": "qwen3:32b", "size": "~20GB", "reason": "GPU显存充足，推荐32B参数量模型，工具调用支持完善", "toolSupport": True})
            recommendations.append({"name": "huihui_ai/qwen3-abliterated:14b", "size": "~9GB", "reason": "14B去审查版，工具调用支持完善，响应更快", "toolSupport": True})
        if gpu_vram_gb >= 12:
            recommendations.append({"name": "huihui_ai/qwen3-abliterated:14b", "size": "~9GB", "reason": "14B去审查版，工具调用支持完善", "toolSupport": True})
            recommendations.append({"name": "qwen3:14b", "size": "~9GB", "reason": "官方14B模型，工具调用支持完善", "toolSupport": True})
        if gpu_vram_gb >= 8 or total_ram_gb >= 16:
            recommendations.append({"name": "qwen3:8b", "size": "~5GB", "reason": "8B轻量模型，工具调用支持完善，适合8GB显存", "toolSupport": True})
        if gpu_vram_gb >= 6 or total_ram_gb >= 12:
            recommendations.append({"name": "huihui_ai/qwen3-vl-abliterated:8b", "size": "~6GB", "reason": "8B视觉模型，支持图片理解", "toolSupport": False})
        if total_ram_gb >= 8:
            recommendations.append({"name": "llama3:8b", "size": "~5GB", "reason": "Meta Llama3 8B，通用对话模型", "toolSupport": False})
        seen = set()
        unique = []
        for r in recommendations:
            if r["name"] not in seen:
                seen.add(r["name"])
                unique.append(r)
        return json.dumps({"ok": True, "models": unique, "hardware": hw})

    @pyqtSlot(str, result=str)
    def fetchApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.fetch_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"fetchApiKey异常: {e}"})

    @pyqtSlot(str, result=str)
    def startQwen2Api(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            project_dir = payload.get("projectDir", "").strip()
            port = int(payload.get("port", 7860) or 7860)
            admin_key = payload.get("adminKey", "admin").strip() or "admin"
            result = backend.start_qwen2api(project_dir, port, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startQwen2Api异常: {e}"})

    @pyqtSlot(str, result=str)
    def checkApiService(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            result = backend.check_api_service(base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"checkApiService异常: {e}"})

    @pyqtSlot(str, result=str)
    def addQwenAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            token = payload.get("token", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.add_qwen_account(base_url, token, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"addQwenAccount异常: {e}"})

    @pyqtSlot(str, result=str)
    def listQwenAccounts(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.list_qwen_accounts(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listQwenAccounts异常: {e}"})

    @pyqtSlot(str, result=str)
    def startQwenRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.start_qwen_register(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startQwenRegister异常: {e}"})

    @pyqtSlot(result=str)
    def pollQwenRegister(self):
        try:
            result = backend.poll_qwen_register()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollQwenRegister异常: {e}"})

    # ── 内部方法 ──

    def _run_cli(self, prompt: str, model: str, provider: str, settings: dict):
        """在后台线程运行 CLI"""
        main = self._get_main()
        if not main:
            return

        try:
            env_overrides = {}
            if provider == "ollama":
                ollama_target = (settings.get("OLLAMA_BASE_URL", "") or "http://127.0.0.1:11434").strip()
                ollama_model = (settings.get("OLLAMA_MODEL", "") or "qwen3:8b").strip()
                main.log_signal.emit(f"[代理] 使用Node.js代理 模型={ollama_model} 目标={ollama_target}", "#2196F3")
                env_overrides["MODEL_PROVIDER"] = "ollama"
                env_overrides["OLLAMA_BASE_URL"] = ollama_target
                env_overrides["OLLAMA_MODEL"] = ollama_model
                main.log_signal.emit(f"[代理] Node.js代理模式 模型={ollama_model}", "#2196F3")
            elif provider == "api":
                api_base = (settings.get("API_BASE_URL", "") or "http://127.0.0.1:7860").strip()
                api_model = (settings.get("API_MODEL", "") or "qwen3.6-plus").strip()
                api_key = (settings.get("API_KEY", "") or "").strip()
                main.log_signal.emit(f"[代理] API模式 模型={api_model} 目标={api_base}", "#2196F3")
                env_overrides["MODEL_PROVIDER"] = "api"
                env_overrides["API_BASE_URL"] = api_base
                env_overrides["API_MODEL"] = api_model
                if api_key:
                    env_overrides["API_KEY"] = api_key
            else:
                env_overrides["MODEL_PROVIDER"] = "anthropic"
                env_overrides.pop("OLLAMA_BASE_URL", None)
                env_overrides.pop("OLLAMA_MODEL", None)

            if settings.get("AI_TEMPERATURE"):
                env_overrides["AI_TEMPERATURE"] = settings["AI_TEMPERATURE"]
                os.environ["AI_TEMPERATURE"] = settings["AI_TEMPERATURE"]
            else:
                os.environ.pop("AI_TEMPERATURE", None)
            if settings.get("AI_MAX_TOKENS"):
                env_overrides["AI_MAX_TOKENS"] = settings["AI_MAX_TOKENS"]
                os.environ["AI_MAX_TOKENS"] = settings["AI_MAX_TOKENS"]
            else:
                os.environ.pop("AI_MAX_TOKENS", None)
            if settings.get("AI_LANGUAGE"):
                env_overrides["AI_LANGUAGE"] = settings["AI_LANGUAGE"]
                os.environ["AI_LANGUAGE"] = settings["AI_LANGUAGE"]
            else:
                os.environ.pop("AI_LANGUAGE", None)

            is_resuming = main.active_session_id in main.started_sessions

            system_prompt = self._build_system_prompt(settings)

            def _on_delta(text):
                self.deltaReceived.emit(json.dumps({"text": text}))

            result = main.cli_runner.run(
                prompt=prompt,
                session_id=main.active_session_id,
                model=model,
                is_resuming=is_resuming,
                workspace_path=main.current_workspace,
                env_overrides=env_overrides,
                on_delta=_on_delta,
                on_log=lambda msg, color="#888": (
                    main.log_signal.emit(msg, color),
                    main.debug_log_signal.emit(msg, color),
                ),
                on_proc=lambda p: setattr(main, 'active_proc', p),
                system_prompt=system_prompt,
            )

            if result.get("ok"):
                cli_sid = result.get("cliSessionId", "")
                if cli_sid and cli_sid != main.active_session_id:
                    main.log_signal.emit(f"[CLI] 更新session_id: {main.active_session_id} -> {cli_sid}", "#2196F3")
                    main.active_session_id = cli_sid
                main.started_sessions.add(main.active_session_id)
            else:
                cli_sid = result.get("cliSessionId", "")
                if cli_sid and cli_sid != main.active_session_id:
                    main.active_session_id = cli_sid
                main.started_sessions.add(main.active_session_id)
                err = result.get("error", "")[:200]
                main.log_signal.emit(f"[CLI 错误] {err}", "#F44336")
                if err and not result.get("text"):
                    self.deltaReceived.emit(json.dumps({"text": f"❌ {err}"}))

            main.result_ready_signal.emit(json.dumps(result))
        except Exception as e:
            main.log_signal.emit(f"[线程异常] {e}", "#F44336")
            self.deltaReceived.emit(json.dumps({"text": f"❌ 线程异常: {str(e)[:200]}"}))
            main.result_ready_signal.emit(json.dumps({"ok": False, "error": str(e)}))
        finally:
            import time as _time
            _time.sleep(0.3)
            main.is_busy = False
            main.active_proc = None
            self.statusReceived.emit(json.dumps({"busy": False}))


    @staticmethod
    def _build_system_prompt(settings: dict) -> str:
        custom_prompt = (settings.get("SYSTEM_PROMPT") or "").strip()
        if custom_prompt:
            return custom_prompt

        language = (settings.get("AI_LANGUAGE") or "zh").strip().lower()
        parts = []

        if language == "zh":
            parts.append("你是一个专业的AI编程助手，请始终使用中文回答。")
            parts.append("对于简短的问候或问题，请简洁友好地回应。对于编程任务，你可以读取文件、编辑代码、执行命令来完成。")
            parts.append("重要：只有在用户明确要求执行编程任务时才使用工具。普通对话和问题请直接回答，不要调用任何工具。")
        elif language == "en":
            parts.append("You are a professional AI coding assistant.")
            parts.append("For brief greetings or questions, respond concisely and friendly. For coding tasks, you can read files, edit code, and execute commands.")
            parts.append("Important: Only use tools when the user explicitly requests a coding task. For normal conversation and questions, respond directly without calling any tools.")
        elif language == "ja":
            parts.append("あなたはプロのAIプログラミングアシスタントです。日本語で回答してください。")
            parts.append("簡単な挨拶や質問には簡潔に友好的に答えてください。プログラミングタスクでは、ファイルの読み取り、コードの編集、コマンドの実行ができます。")
            parts.append("重要：プログラミングタスクが明示的に要求された場合のみツールを使用してください。通常の会話や質問には直接回答し、ツールを呼び出さないでください。")
        elif language == "ko":
            parts.append("당신은 전문 AI 프로그래밍 어시스턴트입니다. 한국어로 답변해 주세요.")
            parts.append("간단한 인사나 질문에는 간결하고 친절하게 답변하세요. 프로그래밍 작업에서는 파일 읽기, 코드 편집, 명령 실행이 가능합니다.")
            parts.append("중요: 프로그래밍 작업이 명시적으로 요청된 경우에만 도구를 사용하세요. 일반 대화와 질문에는 도구를 호출하지 말고 직접 답변하세요.")
        else:
            parts.append(f"You are a professional AI coding assistant. Please respond in {language}.")
            parts.append("Important: Only use tools when the user explicitly requests a coding task. For normal conversation, respond directly.")

        return "\n".join(parts)


def _uuid() -> str:
    import uuid as _u
    return str(_u.uuid4())


# ── 环境安装器 ──
class EnvInstaller:
    """便携版环境下载与安装"""

    def __init__(self, base_dir: str, log_func=None, progress_func=None):
        self.base_dir = base_dir
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func
        self._mirror_key = "china"

    def _load_mirror(self):
        try:
            fp = os.path.join(self.base_dir, MIRROR_SETTINGS_FILE)
            if os.path.isfile(fp):
                with open(fp, "r", encoding="utf-8") as f:
                    key = f.read().strip()
                if key in MIRROR_SOURCES:
                    self._mirror_key = key
        except Exception:
            pass

    def _save_mirror(self, key: str):
        self._mirror_key = key
        try:
            fp = os.path.join(self.base_dir, MIRROR_SETTINGS_FILE)
            with open(fp, "w", encoding="utf-8") as f:
                f.write(key)
        except Exception:
            pass

    @property
    def mirror(self) -> dict:
        return MIRROR_SOURCES.get(self._mirror_key, MIRROR_SOURCES["china"])

    def _github_url(self, original_url: str) -> str:
        proxy = self.mirror.get("github_proxy", "")
        if proxy and "github.com" in original_url:
            return proxy + original_url
        return original_url

    @property
    def nodejs_dir(self): return os.path.join(self.base_dir, "nodejs")
    @property
    def node_extract(self): return os.path.join(self.nodejs_dir, NODE_DIR_NAME)
    @property
    def node_exe(self): return os.path.join(self.node_extract, "node.exe")
    @property
    def bun_dir(self): return os.path.join(self.base_dir, "bun")
    @property
    def bun_extract(self): return os.path.join(self.bun_dir, BUN_DIR_NAME)
    @property
    def bun_exe(self): return os.path.join(self.bun_extract, "bun.exe")
    @property
    def uv_dir(self): return os.path.join(self.base_dir, "uv")
    @property
    def uv_exe(self): return os.path.join(self.uv_dir, "uv.exe")
    @property
    def uv_python_dir(self): return os.path.join(self.base_dir, "python")
    @property
    def scripts_dir(self): return os.path.join(self.base_dir, "scripts")
    @property
    def venv_dir(self): return os.path.join(self.scripts_dir, ".venv")
    @property
    def venv_python(self):
        return os.path.join(self.venv_dir, "Scripts", "python.exe") if os.name == "nt" else os.path.join(self.venv_dir, "bin", "python")

    def check_node(self): return os.path.exists(self.node_exe)
    def check_bun(self): return os.path.exists(self.bun_exe)
    def check_deps(self):
        tsx = os.path.join(self.base_dir, "node_modules", "tsx", "dist", "loader.mjs")
        return os.path.exists(os.path.join(self.base_dir, "node_modules")) and os.path.exists(tsx)
    def check_dist(self): return os.path.exists(os.path.join(self.base_dir, "desktop", "dist"))
    def check_electron(self):
        return os.path.exists(os.path.join(self.base_dir, "node_modules", ".bin", "electron.cmd"))

    def check_uv(self): return os.path.exists(self.uv_exe)

    def check_qwen2api(self):
        if not os.path.isfile(self.venv_python):
            return False
        try:
            r = subprocess.run(
                [self.venv_python, "-c",
                 "import fastapi, uvicorn, httpx, pydantic_settings, tiktoken, curl_cffi; print('ok')"],
                capture_output=True, text=True, timeout=15,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return r.returncode == 0 and "ok" in (r.stdout or "")
        except Exception:
            return False

    def check_all(self):
        return {
            "node": self.check_node(), "bun": self.check_bun(),
            "deps": self.check_deps(), "dist": self.check_dist(),
            "electron": self.check_electron(),
            "uv": self.check_uv(), "qwen2api": self.check_qwen2api(),
        }

    def _download(self, url: str, dest: str, label: str):
        mirror_label = self.mirror.get("label", "")
        self.log(f"正在下载 {label}... [{mirror_label}]")
        try:
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            resp = urllib.request.urlopen(req, timeout=120)
            total = int(resp.headers.get("Content-Length", 0))
            dl = 0
            with open(dest, "wb") as f:
                while True:
                    chunk = resp.read(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    dl += len(chunk)
                    if total > 0 and self.progress:
                        self.progress(int(dl * 100 / total), f"下载 {label} {int(dl*100/total)}%")
            self.log(f"✓ {label} 下载完成")
            return True
        except Exception as e:
            self.log(f"[错误] 下载 {label} 失败: {e}", "#F44336")
            return False

    def _unzip(self, zip_path, dest_dir, label):
        self.log(f"正在解压 {label}...")
        try:
            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(dest_dir)
            try:
                os.remove(zip_path)
            except:
                pass
            self.log(f"✓ {label} 解压完成")
            return True
        except Exception as e:
            self.log(f"[错误] 解压 {label} 失败: {e}", "#F44336")
            return False

    def _si(self):
        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        return si

    def install_node(self):
        if self.check_node():
            self.log("✓ Node.js 已安装")
            return True
        zip_path = os.path.join(self.nodejs_dir, f"{NODE_DIR_NAME}.zip")
        node_base = self.mirror.get("node", "https://nodejs.org/dist/")
        url = f"{node_base}{NODE_VERSION}/node-{NODE_VERSION}-win-x64.zip"
        if not os.path.exists(zip_path):
            if not self._download(url, zip_path, f"Node.js {NODE_VERSION}"):
                return False
        if os.path.exists(self.node_extract):
            shutil.rmtree(self.node_extract, ignore_errors=True)
        return self._unzip(zip_path, self.nodejs_dir, "Node.js")

    def install_bun(self):
        if self.check_bun():
            self.log("✓ Bun 已安装")
            return True
        zip_path = os.path.join(self.bun_dir, f"{BUN_DIR_NAME}.zip")
        url = self._github_url(f"https://github.com/oven-sh/bun/releases/download/bun-v{BUN_VERSION}/bun-windows-x64.zip")
        if not os.path.exists(zip_path):
            if not self._download(url, zip_path, f"Bun {BUN_VERSION}"):
                return False
        if os.path.exists(self.bun_extract):
            shutil.rmtree(self.bun_extract, ignore_errors=True)
        return self._unzip(zip_path, self.bun_dir, "Bun")

    def install_deps(self):
        if self.check_deps():
            self.log("✓ 依赖已安装")
            return True
        if not self.check_bun():
            self.log("[错误] Bun 未安装", "#F44336")
            return False
        try:
            env = dict(os.environ)
            if os.path.exists(self.node_extract):
                env["PATH"] = self.node_extract + ";" + env.get("PATH", "")
            if os.path.exists(self.bun_extract):
                env["PATH"] = self.bun_extract + ";" + env.get("PATH", "")
            subprocess.run([self.bun_exe, "config", "set", "registry", "https://registry.npmmirror.com"],
                           cwd=self.base_dir, env=env, startupinfo=self._si(), capture_output=True, timeout=30,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            r = subprocess.run([self.bun_exe, "install"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=300,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                self.log("✓ 依赖安装完成")
                return True
            self.log(f"[警告] bun install 返回码: {r.returncode}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] 依赖安装失败: {e}", "#F44336")
            return False

    def build_frontend(self):
        dist = os.path.join(self.base_dir, "desktop", "dist")
        if os.path.exists(dist):
            self.log("✓ 前端已构建")
            return True
        if not self.check_bun():
            self.log("[错误] Bun 未安装", "#F44336")
            return False
        try:
            env = dict(os.environ)
            if os.path.exists(self.node_extract):
                env["PATH"] = self.node_extract + ";" + env.get("PATH", "")
            if os.path.exists(self.bun_extract):
                env["PATH"] = self.bun_extract + ";" + env.get("PATH", "")
            r = subprocess.run([self.bun_exe, "run", "desktop:build"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=120,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                self.log("✓ 前端构建完成")
                return True
            self.log(f"[警告] 前端构建返回码: {r.returncode}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] 前端构建失败: {e}", "#F44336")
            return False

    def install_all(self):
        return all([
            self.install_node(),
            self.install_bun(),
            self.install_deps(),
            self.build_frontend(),
            self.install_uv(),
            self.install_qwen2api_deps(),
        ])

    def install_uv(self):
        if self.check_uv():
            self.log("✓ uv 已安装")
            return True
        os.makedirs(self.uv_dir, exist_ok=True)
        try:
            env = os.environ.copy()
            env["UV_INSTALL_DIR"] = self.uv_dir
            mirror = self.mirror
            install_url = "https://astral.sh/uv/install.ps1"
            github_proxy = mirror.get("github_proxy", "")
            proxy_urls = []
            if github_proxy:
                proxy_urls.append(github_proxy + "https://astral.sh/uv/install.ps1")
            proxy_urls.append("https://astral.sh/uv/install.ps1")
            si = self._si()
            success = False
            for url in proxy_urls:
                source_label = "代理" if url != proxy_urls[-1] else "直连"
                self.log(f"正在下载 uv 安装脚本 ({source_label})... [{mirror.get('label', '')}]")
                ps_script = f"""
$ProgressPreference = 'SilentlyContinue'
$env:UV_INSTALL_DIR = '{self.uv_dir}'
try {{
    Invoke-WebRequest -Uri '{url}' -OutFile '$env:TEMP\\uv-install.ps1' -TimeoutSec 30
    powershell -ExecutionPolicy Bypass -File '$env:TEMP\\uv-install.ps1'
    Remove-Item '$env:TEMP\\uv-install.ps1' -ErrorAction SilentlyContinue
}} catch {{
    Write-Output "DOWNLOAD_FAILED"
    Exit 1
}}
"""
                proc = subprocess.Popen(
                    ["powershell.exe", "-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", ps_script],
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                    env=env, startupinfo=si, creationflags=subprocess.CREATE_NO_WINDOW,
                )
                output_lines = []
                while proc.poll() is None:
                    line = proc.stdout.readline()
                    if line and line.strip():
                        self.log(f"  {line.strip()}")
                        output_lines.append(line.strip())
                if "DOWNLOAD_FAILED" in output_lines:
                    self.log(f"  {source_label}下载失败，尝试下一个源...", "#FF9800")
                    continue
                if proc.returncode == 0:
                    success = True
                    break
                self.log(f"  {source_label}安装失败 (返回码: {proc.returncode})", "#FF9800")
            if not success and os.path.exists(os.path.expanduser("~/.local/bin/uv.exe")):
                src = os.path.expanduser("~/.local/bin/uv.exe")
                import shutil
                shutil.copy2(src, self.uv_exe)
                self.log("✓ uv 安装完成（从默认路径复制）")
                return True
            if success and os.path.exists(self.uv_exe):
                self.log("✓ uv 安装完成")
                return True
            if not success:
                self.log("[错误] uv 安装失败，所有源均不可用", "#F44336")
                return False
            self.log("✓ uv 安装完成")
            return True
        except Exception as e:
            self.log(f"[错误] uv 安装异常: {e}", "#F44336")
            return False

    def _uv_env(self):
        env = os.environ.copy()
        env["UV_PYTHON_INSTALL_DIR"] = self.uv_python_dir
        env["UV_PYTHON_DOWNLOADS"] = "auto"
        python_mirror = self.mirror.get("uv_python_mirror", "")
        if python_mirror:
            env["UV_PYTHON_INSTALL_MIRROR"] = python_mirror
        pypi_index = self.mirror.get("pypi_index", "")
        if pypi_index:
            env["UV_INDEX_URL"] = pypi_index
            env["UV_DEFAULT_INDEX"] = pypi_index
        return env

    def install_qwen2api_deps(self):
        if self.check_qwen2api():
            self.log("✓ API 服务依赖已安装")
            return True
        qwen_dir = os.path.join(self.base_dir, "qwen2api")
        if not os.path.isdir(qwen_dir):
            self.log("⚠ 未找到 qwen2api 目录，跳过 API 服务依赖安装", "#FF9800")
            return True
        req_file = os.path.join(qwen_dir, "backend", "requirements.txt")
        if not os.path.exists(req_file):
            self.log("⚠ qwen2api/requirements.txt 不存在，跳过", "#FF9800")
            return True
        if not self.install_uv():
            self.log("[错误] uv 安装失败，无法继续", "#F44336")
            return False
        try:
            env = self._uv_env()

            if not os.path.isfile(self.venv_python):
                self.log("正在创建虚拟环境 (uv + Python 3.12)...")
                r = subprocess.run(
                    [self.uv_exe, "venv", self.venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                    env=env, capture_output=True, text=True, timeout=300,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
                if r.returncode != 0:
                    mirror = self.mirror
                    python_mirror = mirror.get("uv_python_mirror", "")
                    if python_mirror and "UV_PYTHON_INSTALL_MIRROR" in env:
                        self.log("镜像下载 Python 失败，尝试直连下载...", "#FF9800")
                        env_fallback = env.copy()
                        del env_fallback["UV_PYTHON_INSTALL_MIRROR"]
                        r = subprocess.run(
                            [self.uv_exe, "venv", self.venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                            env=env_fallback, capture_output=True, text=True, timeout=300,
                            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                        )
                if r.returncode != 0:
                    self.log(f"[错误] 创建虚拟环境失败: {r.stderr[:500]}", "#F44336")
                    return False
                self.log("✓ 虚拟环境已创建")

            self.log("正在安装 API 服务依赖 (uv pip)...")
            r = subprocess.run(
                [self.uv_exe, "pip", "install", "-r", req_file, "--python", self.venv_python],
                env=env, capture_output=True, text=True, timeout=600,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            if r.returncode == 0:
                self.log("✓ API 服务依赖安装完成")
                return True
            self.log(f"[警告] API 服务依赖安装返回码: {r.returncode}", "#FF9800")
            if r.stderr:
                self.log(f"  stderr: {r.stderr[:500]}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] API 服务依赖安装失败: {e}", "#F44336")
            return False


# ── 主窗口 ──
class MainWindow(QMainWindow):
    log_signal = pyqtSignal(str, str)
    debug_log_signal = pyqtSignal(str, str)
    progress_signal = pyqtSignal(int, str)
    status_signal = pyqtSignal(str)
    result_ready_signal = pyqtSignal(str)
    workspace_choose_requested = pyqtSignal()
    update_info_signal = pyqtSignal(str)
    _remote_ver_signal = pyqtSignal(object)
    deploy_step_signal = pyqtSignal(str, str, int)

    def __init__(self):
        super().__init__()
        self.setWindowTitle(f"云集智能编程工作站 v{VERSION}")
        self.setMinimumSize(980, 680)

        # 图标
        try:
            if hasattr(sys, 'frozen'):
                icon_path = os.path.join(os.path.dirname(sys.executable), "app", "icon.ico")
            else:
                icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.ico")
            if os.path.exists(icon_path):
                self.setWindowIcon(QIcon(icon_path))
        except:
            pass

        # 基础目录
        # 架构（对齐参考项目）:
        #   dev/*.exe          = 开发测试 EXE（gitignore）
        #   dev/_internal/     = PyInstaller 运行时（gitignore）
        #   dev/app/           = 资源目录（main.py, desktop/, nodejs/ 等，git 管理）
        #   dev/ver/*.exe      = 稳定版 EXE（git 跟踪）
        #   dev/               = Git 仓库根目录
        #
        # --onedir 打包后: EXE 在 dev/ 下，_internal/ 也在 dev/ 下
        #   desktop/、nodejs/ 等资源在 dev/app/ 下
        # 开发模式: main.py 在 dev/app/ 下
        if hasattr(sys, 'frozen'):
            # PyInstaller 打包模式：EXE 在 dev/ 下
            exe_dir = os.path.abspath(os.path.dirname(sys.executable))
            self.base_dir = exe_dir       # dev/ (EXE 所在目录)
            self.app_dir = os.path.join(exe_dir, "app")  # dev/app/ (资源目录)
            self.dev_dir = exe_dir        # dev/ = Git 仓库根
        else:
            self.base_dir = os.path.dirname(os.path.abspath(__file__))  # dev/app/ (脚本所在)
            self.app_dir = self.base_dir  # 开发模式: main.py 在 dev/app/ 下
            self.dev_dir = os.path.dirname(self.base_dir)  # dev/

        # 初始化后端（desktop/、nodejs/ 等资源在 app_dir 下）
        self.env_manager = EnvFileManager(os.path.join(self.app_dir, ".env"))
        self.cli_runner = ClaudeCliRunner(
            self.app_dir,
            os.path.join(self.app_dir, "nodejs", NODE_DIR_NAME),
            os.path.join(self.app_dir, "bun", BUN_DIR_NAME),
        )
        self.installer = EnvInstaller(self.app_dir)
        self.updater = SoftwareUpdater(self.dev_dir)

        # 状态
        self.active_session_id = _uuid()
        self.started_sessions = set()
        self.current_workspace = self.app_dir
        self.is_busy = False
        self.active_proc = None

        # 构建 UI
        self._setup_ui()

        # 连接信号
        self.log_signal.connect(self._append_log)
        self.debug_log_signal.connect(self._append_debug_log)
        self.progress_signal.connect(self._update_progress)
        self.deploy_step_signal.connect(self._update_deploy_step)
        self.status_signal.connect(self._update_status)
        self.result_ready_signal.connect(self._on_result_ready)
        self.workspace_choose_requested.connect(self._choose_workspace_dialog)
        self._remote_ver_signal.connect(self._on_remote_ver_fetched)

        # 启动时检查环境
        QTimer.singleShot(800, self._auto_check_and_load)

    def _setup_ui(self):
        self.setStyleSheet("""
            QMainWindow, QWidget { background-color: #0d0d0d; color: #f0f0f0; font-family: 'Microsoft YaHei', sans-serif; }
            QPushButton { color: white; border-radius: 8px; padding: 10px 20px; font-size: 13px; font-weight: bold; border: 2px solid transparent; }
            QPushButton:disabled { background-color: #333; border-color: #333; color: #757575; }
        """)

        central = QWidget()
        self.setCentralWidget(central)
        layout = QVBoxLayout(central)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # ── 顶部导航栏（Tab 式整体导航，底部蓝色指示条）──
        nav_bar = QFrame()
        nav_bar.setFixedHeight(38)
        nav_bar.setStyleSheet("QFrame { background-color: #1a1a1a; border-bottom: 1px solid #2a2a2a; }")
        nav_layout = QHBoxLayout(nav_bar)
        nav_layout.setSpacing(1)
        nav_layout.setContentsMargins(8, 4, 8, 0)

        # 导航按钮样式（Tab 式：无边框，底部指示条标识选中）
        menu_button_style = """
            QPushButton {
                background-color: transparent; color: #999999;
                border: none; border-bottom: 3px solid transparent;
                border-radius: 0px; padding: 4px 12px 6px 12px; font-size: 12px; font-weight: normal;
            }
            QPushButton:hover { color: #ffffff; background-color: #252525; }
            QPushButton:checked { color: #ffffff; border-bottom: 3px solid #3b82f6; }
            QPushButton:checked:hover { background-color: #252525; }
        """

        # 运行服务按钮（首页）
        self.btn_home = QPushButton("🚀 运行服务")
        self.btn_home.setCheckable(True)
        self.btn_home.setChecked(True)
        self.btn_home.setStyleSheet(menu_button_style)
        self.btn_home.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_home.clicked.connect(lambda: self._switch_page(0))
        nav_layout.addWidget(self.btn_home)

        # 部署维护按钮
        self.btn_deploy_nav = QPushButton("⚙️ 部署维护")
        self.btn_deploy_nav.setCheckable(True)
        self.btn_deploy_nav.setStyleSheet(menu_button_style)
        self.btn_deploy_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_deploy_nav.clicked.connect(lambda: self._switch_page(1))
        nav_layout.addWidget(self.btn_deploy_nav)

        # 软件更新按钮
        self.btn_update_nav = QPushButton("📋 版本管理")
        self.btn_update_nav.setCheckable(True)
        self.btn_update_nav.setStyleSheet(menu_button_style)
        self.btn_update_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_update_nav.clicked.connect(lambda: self._switch_page(2))
        nav_layout.addWidget(self.btn_update_nav)

        layout.addWidget(nav_bar)

        # ── 页面堆叠窗口 ──
        self.page_stack = QStackedWidget()

        # 页面0：首页 - 运行服务（WebEngine）
        self.home_page = self._create_home_page()
        self.page_stack.addWidget(self.home_page)

        # 页面1：部署维护
        self.deploy_page = self._create_deploy_page()
        self.page_stack.addWidget(self.deploy_page)

        # 页面2：软件更新
        self.update_page = self._create_update_page()
        self.page_stack.addWidget(self.update_page)

        layout.addWidget(self.page_stack, 1)

        # ── 底部状态栏 ──
        statusbar = QFrame()
        statusbar.setFixedHeight(28)
        statusbar.setStyleSheet("QFrame { background-color: #1a1a1a; border-top: 1px solid #2a2a2a; }")
        sb_layout = QHBoxLayout(statusbar)
        sb_layout.setContentsMargins(12, 0, 12, 0)

        self.env_status = QLabel("环境: 检查中...")
        self.env_status.setStyleSheet("color: #666; font-size: 10px; border: none;")
        sb_layout.addWidget(self.env_status)

        sb_layout.addStretch()

        layout.addWidget(statusbar)

    # ── 页面创建 ──

    def _create_home_page(self):
        """创建首页 - 运行服务（QWebEngineView）"""
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setSpacing(0)
        page_layout.setContentsMargins(0, 0, 0, 0)

        # QWebEngineView 加载 Vue 前端
        self.web_view = QWebEngineView()
        self.web_view.setStyleSheet("background-color: #0d0d0d;")

        # 防止白色闪屏：初始隐藏 web_view，用深色占位
        self.web_view.setVisible(False)
        self._loading_label = QLabel("正在加载...")
        self._loading_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._loading_label.setStyleSheet("color: #666; font-size: 14px; background-color: #0d0d0d; border: none;")
        page_layout.addWidget(self._loading_label, 1)

        # 设置 WebEngine 页面背景色为深色（防止渲染白色闪烁）
        self.web_view.page().setBackgroundColor(Qt.GlobalColor.black)

        # 页面加载完成后显示 web_view，隐藏占位标签
        self.web_view.loadFinished.connect(self._on_web_load_finished)

        # QWebChannel 桥接
        self.channel = QWebChannel()
        self.bridge = BackendBridge()
        self.bridge._app_ref = self
        self.channel.registerObject("backend", self.bridge)
        self.web_view.page().setWebChannel(self.channel)

        page_layout.addWidget(self.web_view, 1)

        # 底部日志面板（默认折叠）
        self.log_panel = QFrame()
        self.log_panel.setVisible(False)
        self.log_panel.setStyleSheet("QFrame { background-color: #111; border-top: 1px solid #2a2a2a; }")
        log_layout = QVBoxLayout(self.log_panel)
        log_layout.setContentsMargins(8, 4, 8, 4)

        log_header = QHBoxLayout()
        log_title = QLabel("📋 运行日志")
        log_title.setStyleSheet("color: #888; font-size: 11px; font-weight: bold; border: none;")
        log_header.addWidget(log_title)
        log_header.addStretch()

        self.btn_toggle_log = QPushButton("收起")
        self.btn_toggle_log.setStyleSheet("QPushButton { background: #333; border: 1px solid #444; border-radius: 4px; padding: 2px 8px; font-size: 10px; }")
        self.btn_toggle_log.clicked.connect(lambda: self.log_panel.setVisible(False))
        log_header.addWidget(self.btn_toggle_log)
        log_layout.addLayout(log_header)

        self.log_text = QTextEdit()
        self.log_text.setReadOnly(True)
        self.log_text.setMaximumHeight(150)
        self.log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: 1px solid #222; border-radius: 4px; padding: 4px; font-family: Consolas, monospace; font-size: 11px; }")
        log_layout.addWidget(self.log_text)

        page_layout.addWidget(self.log_panel)

        # 日志切换按钮（浮在首页右下角）
        self.btn_show_log = QPushButton("📋 日志")
        self.btn_show_log.setStyleSheet("QPushButton { background: #333; border: 1px solid #444; border-radius: 4px; padding: 2px 8px; font-size: 10px; }")
        self.btn_show_log.clicked.connect(lambda: self.log_panel.setVisible(not self.log_panel.isVisible()))

        return page

    def _create_deploy_page(self):
        """创建部署维护页面"""
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setSpacing(8)
        layout.setContentsMargins(12, 10, 12, 10)

        # 标题
        title = QLabel("⚙️ 部署维护")
        title.setFont(QFont("Microsoft YaHei", 13, QFont.Weight.Bold))
        title.setStyleSheet("color: #4CAF50; border: none;")
        layout.addWidget(title)

        # 环境状态 + 安装流程（合二为一）
        deploy_group = QFrame()
        deploy_group.setStyleSheet("QFrame { background-color: #1a1a1a; border: 1px solid #333; border-radius: 6px; }")
        deploy_layout = QVBoxLayout(deploy_group)
        deploy_layout.setSpacing(4)
        deploy_layout.setContentsMargins(8, 8, 8, 8)

        # 顶部：标题 + 下载源
        top_row = QHBoxLayout()
        top_row.setSpacing(12)
        deploy_title = QLabel("📦 部署维护")
        deploy_title.setFont(QFont("Microsoft YaHei", 11, QFont.Weight.Bold))
        deploy_title.setStyleSheet("color: #fff; border: none;")
        top_row.addWidget(deploy_title)

        top_row.addStretch()

        mirror_label = QLabel("📡 下载源")
        mirror_label.setStyleSheet("color: #aaa; font-size: 10px; border: none;")
        top_row.addWidget(mirror_label)

        self.mirror_combo = QComboBox()
        self.mirror_combo.setStyleSheet("""
            QComboBox { background: #2a2a2a; border: 1px solid #444; border-radius: 4px; padding: 3px 8px; color: #fff; font-size: 11px; min-width: 120px; }
            QComboBox::drop-down { border: none; }
            QComboBox QAbstractItemView { background: #2a2a2a; color: #fff; selection-background-color: #1565C0; }
        """)
        for key, cfg in MIRROR_SOURCES.items():
            self.mirror_combo.addItem(cfg["label"], key)
        self.installer._load_mirror()
        idx = list(MIRROR_SOURCES.keys()).index(self.installer._mirror_key) if self.installer._mirror_key in MIRROR_SOURCES else 0
        self.mirror_combo.setCurrentIndex(idx)
        self.mirror_combo.currentIndexChanged.connect(self._on_mirror_changed)
        top_row.addWidget(self.mirror_combo)
        deploy_layout.addLayout(top_row)

        # 一键部署按钮
        self.btn_install_all = QPushButton("🔄 一键部署全部")
        self.btn_install_all.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: 2px solid #388E3C; border-radius: 6px; padding: 8px 16px; font-size: 12px; font-weight: bold; }
            QPushButton:hover { background-color: #388E3C; }
            QPushButton:disabled { background-color: #333; color: #666; border-color: #444; }
        """)
        self.btn_install_all.clicked.connect(self._on_deploy)
        deploy_layout.addWidget(self.btn_install_all)

        # 步骤进度条按钮
        self.deploy_env_labels = {}
        self.deploy_steps = {}
        checks = self.installer.check_all()
        steps_info = [
            ("node", "Node.js", "#1565C0"),
            ("bun", "Bun", "#1565C0"),
            ("deps", "npm 依赖", "#1565C0"),
            ("dist", "前端构建", "#6A1B9A"),
            ("uv", "uv", "#00695C"),
            ("qwen2api", "API 依赖", "#E65100"),
        ]

        for idx_s, (key, label, color) in enumerate(steps_info):
            step_btn = QPushButton()
            step_btn.setFixedHeight(28)
            installed = checks.get(key, False)

            bar_val = 100 if installed else 0
            icon_text = "✓" if installed else "○"
            status_text = "已安装" if installed else "待安装"
            status_color = "#4CAF50" if installed else "#666"

            step_btn.setText(f"  {icon_text}  {label}  {status_text}  ")
            step_btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #222; border: 1px solid #333; border-radius: 4px;
                    color: {status_color}; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:hover {{
                    background-color: #2a2a2a; border-color: {color};
                }}
                QPushButton:disabled {{
                    background-color: #1a1a1a; color: #555; border-color: #222;
                }}
            """)
            step_btn.clicked.connect(lambda checked, k=key: self._on_install_single(k))
            deploy_layout.addWidget(step_btn)

            self.deploy_env_labels[key] = step_btn
            self.deploy_steps[key] = {
                "btn": step_btn, "color": color, "label": label,
            }

        # Electron 状态（只显示，不可安装）
        electron_row = QHBoxLayout()
        electron_row.setSpacing(4)
        electron_lbl = QLabel("  ○  Electron")
        electron_lbl.setStyleSheet("color: #888; font-size: 11px; border: none;")
        electron_row.addWidget(electron_lbl)
        electron_row.addStretch()
        self.electron_status_lbl = QLabel("✓ 已安装" if checks.get("electron") else "✗ 未安装")
        self.electron_status_lbl.setStyleSheet(f"color: {'#4CAF50' if checks.get('electron') else '#F44336'}; font-size: 11px; border: none;")
        electron_row.addWidget(self.electron_status_lbl)
        deploy_layout.addLayout(electron_row)

        layout.addWidget(deploy_group)

        # 日志区域（标签页切换）
        log_group = QFrame()
        log_group.setStyleSheet("QFrame { background-color: #0a0a0a; border: 1px solid #222; border-radius: 8px; }")
        log_l = QVBoxLayout(log_group)
        log_l.setContentsMargins(4, 4, 4, 4)
        log_l.setSpacing(0)

        self.deploy_log_tabs = QTabWidget()
        self.deploy_log_tabs.setStyleSheet("""
            QTabWidget::pane { border: none; background: #0a0a0a; }
            QTabBar::tab { background: #1a1a1a; color: #888; padding: 6px 14px; border: 1px solid #333; border-bottom: none; border-radius: 4px 4px 0 0; font-size: 11px; }
            QTabBar::tab:selected { background: #0a0a0a; color: #4CAF50; font-weight: bold; }
            QTabBar::tab:hover { color: #ccc; }
        """)

        # Tab 1: 部署日志
        deploy_log_tab = QWidget()
        deploy_log_layout = QVBoxLayout(deploy_log_tab)
        deploy_log_layout.setContentsMargins(0, 4, 0, 0)
        self.deploy_log_text = QTextEdit()
        self.deploy_log_text.setReadOnly(True)
        self.deploy_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        deploy_log_layout.addWidget(self.deploy_log_text)
        self.deploy_log_tabs.addTab(deploy_log_tab, "📋 部署日志")

        # Tab 2: 运行日志
        runtime_log_tab = QWidget()
        runtime_log_layout = QVBoxLayout(runtime_log_tab)
        runtime_log_layout.setContentsMargins(0, 4, 0, 0)
        runtime_log_layout.setSpacing(4)

        runtime_toolbar = QHBoxLayout()
        self.btn_clear_runtime_log = QPushButton("清空")
        self.btn_clear_runtime_log.setStyleSheet("QPushButton { background: #333; border: 1px solid #444; border-radius: 4px; padding: 2px 8px; font-size: 10px; color: #aaa; }")
        self.btn_clear_runtime_log.clicked.connect(lambda: self.runtime_log_text.clear())
        runtime_toolbar.addStretch()
        runtime_toolbar.addWidget(self.btn_clear_runtime_log)
        runtime_log_layout.addLayout(runtime_toolbar)

        self.runtime_log_text = QTextEdit()
        self.runtime_log_text.setReadOnly(True)
        self.runtime_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        runtime_log_layout.addWidget(self.runtime_log_text)
        self.deploy_log_tabs.addTab(runtime_log_tab, "🔄 运行日志")

        # Tab 3: 调试日志
        debug_log_tab = QWidget()
        debug_log_layout = QVBoxLayout(debug_log_tab)
        debug_log_layout.setContentsMargins(0, 4, 0, 0)
        debug_log_layout.setSpacing(4)

        debug_toolbar = QHBoxLayout()
        self.btn_clear_debug_log = QPushButton("清空")
        self.btn_clear_debug_log.setStyleSheet("QPushButton { background: #333; border: 1px solid #444; border-radius: 4px; padding: 2px 8px; font-size: 10px; color: #aaa; }")
        self.btn_clear_debug_log.clicked.connect(lambda: self.debug_log_text.clear())
        debug_toolbar.addStretch()
        debug_toolbar.addWidget(self.btn_clear_debug_log)
        debug_log_layout.addLayout(debug_toolbar)

        self.debug_log_text = QTextEdit()
        self.debug_log_text.setReadOnly(True)
        self.debug_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        debug_log_layout.addWidget(self.debug_log_text)
        self.deploy_log_tabs.addTab(debug_log_tab, "🐛 调试日志")

        log_l.addWidget(self.deploy_log_tabs)

        layout.addWidget(log_group, 1)

        return page

    def _create_update_page(self):
        """创建软件更新页面 - 版本历史管理"""
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setSpacing(8)
        layout.setContentsMargins(12, 10, 12, 10)

        top_bar = QHBoxLayout()
        title = QLabel("🔄 版本管理")
        title.setFont(QFont("Microsoft YaHei", 13, QFont.Weight.Bold))
        title.setStyleSheet("color: #1565C0; border: none;")
        top_bar.addWidget(title)
        top_bar.addStretch()

        self.btn_check_update = QPushButton("🔍 检查更新")
        self.btn_check_update.setStyleSheet("""
            QPushButton { background-color: #1565C0; border: 2px solid #1976D2; border-radius: 6px; padding: 8px 16px; font-size: 12px; color: white; }
            QPushButton:hover { background-color: #1976D2; }
        """)
        self.btn_check_update.clicked.connect(self._on_update)
        top_bar.addWidget(self.btn_check_update)

        self.btn_pull_update = QPushButton("📥 更新资源包")
        self.btn_pull_update.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: 2px solid #388E3C; border-radius: 6px; padding: 8px 16px; font-size: 12px; color: white; }
            QPushButton:hover { background-color: #388E3C; }
            QPushButton:disabled { background-color: #1a1a1a; border-color: #333; color: #555; }
        """)
        self.btn_pull_update.clicked.connect(self._do_pull_update)
        self.btn_pull_update.setEnabled(False)
        top_bar.addWidget(self.btn_pull_update)

        refresh_btn = QPushButton("🔄 刷新")
        refresh_btn.setStyleSheet("""
            QPushButton { background-color: #2D2D2D; border: 1px solid #424242; border-radius: 4px; padding: 8px 12px; font-size: 12px; color: #F0F0F0; }
            QPushButton:hover { background-color: #424242; border-color: #555; }
        """)
        refresh_btn.clicked.connect(lambda: self._fetch_and_refresh_ver_list())
        top_bar.addWidget(refresh_btn)

        layout.addLayout(top_bar)

        current_frame = QFrame()
        current_frame.setStyleSheet("QFrame { background-color: #1a1a1a; border: 1px solid #333; border-radius: 6px; padding: 10px; } QLabel { border: none; background: transparent; }")
        current_layout = QVBoxLayout(current_frame)
        current_layout.setSpacing(4)

        self.update_info_label = QLabel("点击「检查更新」查看最新版本")
        self.update_info_label.setStyleSheet("color: #ccc; font-size: 12px;")
        self.update_info_label.setWordWrap(True)
        current_layout.addWidget(self.update_info_label)

        layout.addWidget(current_frame)

        sep = QFrame()
        sep.setFrameShape(QFrame.Shape.HLine)
        sep.setStyleSheet("background-color: #333;")
        sep.setMaximumHeight(1)
        layout.addWidget(sep)

        list_label = QLabel("版本历史")
        list_label.setFont(QFont("Microsoft YaHei", 11, QFont.Weight.Bold))
        list_label.setStyleSheet("color: #888; border: none;")
        layout.addWidget(list_label)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QFrame.Shape.NoFrame)
        scroll.setStyleSheet("QScrollArea { background-color: transparent; border: none; }")

        self.ver_list_container = QWidget()
        self.ver_list_container.setStyleSheet("background-color: transparent; border: none;")
        self.ver_list_layout = QVBoxLayout(self.ver_list_container)
        self.ver_list_layout.setSpacing(6)
        self.ver_list_layout.setContentsMargins(4, 4, 4, 4)

        scroll.setWidget(self.ver_list_container)
        layout.addWidget(scroll, 1)

        self.update_log_text = QTextEdit()
        self.update_log_text.setReadOnly(True)
        self.update_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        self.update_log_text.hide()

        return page

    # ── 页面切换 ──

    def _switch_page(self, index):
        """切换页面"""
        self.btn_home.setChecked(index == 0)
        self.btn_deploy_nav.setChecked(index == 1)
        self.btn_update_nav.setChecked(index == 2)
        self.page_stack.setCurrentIndex(index)

        # 切换到部署维护页面时刷新环境状态
        if index == 1:
            self._refresh_deploy_env_status()
        # 切换到软件更新页面时刷新稳定版列表
        if index == 2:
            self._fetch_and_refresh_ver_list()

    # ── 环境检查与自动加载 ──
    def _auto_check_and_load(self):
        def _check():
            checks = self.installer.check_all()

            parts = []
            for k, v in checks.items():
                parts.append(f"{k}:{'✓' if v else '✗'}")
            self.log_signal.emit("环境检查: " + " ".join(parts), "#666")

            QTimer.singleShot(100, self._load_frontend)
            QTimer.singleShot(200, self._refresh_deploy_env_status)

        t = threading.Thread(target=_check, daemon=True)
        t.start()

    def _on_web_load_finished(self, ok: bool):
        """Vue 前端加载完成，隐藏占位标签，显示 web_view"""
        if ok:
            self._loading_label.setVisible(False)
            self.web_view.setVisible(True)

    def _load_frontend(self):
        """加载 Vue 前端到 QWebEngineView"""
        dist_path = os.path.join(self.app_dir, "desktop", "dist", "index.html")

        if not os.path.exists(dist_path):
            self._update_status("✗ 前端未构建")
            self.log_signal.emit("[错误] 前端未构建，请先运行部署维护", "#F44336")
            return

        # 使用 file:// URL 加载
        url = QUrl.fromLocalFile(dist_path)
        self.web_view.load(url)
        self._update_status("🟢 就绪")

        # 更新环境状态栏
        self._update_env_status()

    def _update_env_status(self):
        checks = self.installer.check_all()
        parts = []
        labels = {"node": "Node", "bun": "Bun", "deps": "依赖", "dist": "前端", "electron": "Electron", "qwen2api": "API"}
        for k, v in checks.items():
            parts.append(f"{labels.get(k, k)}:{'✓' if v else '✗'}")
        self.env_status.setText("环境: " + " | ".join(parts))

    def _refresh_deploy_env_status(self):
        """刷新部署维护页面的环境状态"""
        if not hasattr(self, 'deploy_steps'):
            return
        checks = self.installer.check_all()
        if not self.is_busy:
            for key in self.deploy_steps:
                if checks.get(key, False):
                    self._update_deploy_step(key, "skip", 100)
                else:
                    self._update_deploy_step(key, "reset", 0)
        if hasattr(self, 'electron_status_lbl'):
            installed = checks.get("electron", False)
            self.electron_status_lbl.setText("✓ 已安装" if installed else "✗ 未安装")
            self.electron_status_lbl.setStyleSheet(f"color: {'#4CAF50' if installed else '#F44336'}; font-size: 11px; border: none;")

    def _refresh_ver_list(self, remote_versions=None):
        """刷新软件更新页面的版本历史列表"""
        if not hasattr(self, 'ver_list_layout'):
            return

        while self.ver_list_layout.count():
            item = self.ver_list_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        local_versions = self.updater.get_local_version_history()

        stable_exes = self.updater.list_stable_exes()
        exe_versions = {}
        for exe in stable_exes:
            exe_versions[exe["version"]] = exe

        current_version = ""
        if hasattr(sys, 'frozen'):
            import re
            m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', os.path.basename(sys.executable))
            if m:
                current_version = m.group(1)

        local_ver_set = set()
        all_versions = []

        for v in local_versions:
            ver = v.get("version", v.get("version_number", ""))
            import re as _re
            m = _re.search(r'v?(\d+\.\d+\.\d+\.\d+)', ver)
            ver_num = m.group(1) if m else ver
            if not ver_num:
                continue
            local_ver_set.add(ver_num)
            all_versions.append({
                "version": ver_num,
                "name": v.get("name", f"v{ver_num}"),
                "changes": v.get("changes", []),
                "build_time": v.get("build_time", ""),
                "git_commit": v.get("git_commit", ""),
                "available": ver_num in exe_versions,
                "exe_info": exe_versions.get(ver_num),
                "is_remote_new": False,
            })

        if remote_versions:
            for v in remote_versions:
                ver = v.get("version", v.get("version_number", ""))
                import re as _re2
                m = _re2.search(r'v?(\d+\.\d+\.\d+\.\d+)', ver)
                ver_num = m.group(1) if m else ver
                if not ver_num:
                    continue
                if ver_num in local_ver_set:
                    continue
                all_versions.append({
                    "version": ver_num,
                    "name": v.get("name", f"v{ver_num}"),
                    "changes": v.get("changes", []),
                    "build_time": v.get("build_time", ""),
                    "git_commit": v.get("git_commit", ""),
                    "available": ver_num in exe_versions,
                    "exe_info": exe_versions.get(ver_num),
                    "is_remote_new": True,
                })

        for ver, exe in exe_versions.items():
            if ver not in local_ver_set:
                all_versions.append({
                    "version": ver,
                    "name": exe["filename"],
                    "changes": [],
                    "build_time": "",
                    "git_commit": "",
                    "available": True,
                    "exe_info": exe,
                    "is_remote_new": False,
                })

        all_versions.sort(key=lambda x: x["version"], reverse=True)

        if not all_versions:
            no_ver = QLabel("暂无版本历史记录")
            no_ver.setStyleSheet("color: #555; padding: 20px; border: none; background: transparent;")
            no_ver.setAlignment(Qt.AlignmentFlag.AlignCenter)
            self.ver_list_layout.addWidget(no_ver)
            return

        for v in all_versions:
            self._create_version_card(v, v["version"] == current_version)

    def _fetch_and_refresh_ver_list(self):
        """后台获取远程版本信息并刷新列表"""
        if hasattr(self, '_fetching_remote') and self._fetching_remote:
            return
        self._fetching_remote = True

        self.update_info_label.setText("⏳ 正在获取远程版本信息...")

        def _do_fetch():
            remote = self.updater.fetch_remote_version_history()
            self._fetching_remote = False
            self._remote_ver_signal.emit(remote)

        t = threading.Thread(target=_do_fetch, daemon=True)
        t.start()

    def _on_remote_ver_fetched(self, remote_versions):
        """远程版本信息获取完成"""
        if remote_versions is None:
            self.update_info_label.setText("⚠️ 无法获取远程版本信息（网络不可达）")
            self._refresh_ver_list(remote_versions=None)
        else:
            local_versions = self.updater.get_local_version_history()
            new_versions = self.updater.compare_versions(local_versions, remote_versions)
            if new_versions:
                names = ", ".join(f"v{v.get('version', v.get('version_number', '?'))}" for v in new_versions[:3])
                self.update_info_label.setText(f"🆕 发现 {len(new_versions)} 个远程新版本: {names}")
            else:
                self.update_info_label.setText("✅ 已是最新版本")
            self._refresh_ver_list(remote_versions=remote_versions)

    def _create_version_card(self, version_info, is_current):
        """创建版本卡片"""
        ver = version_info["version"]
        is_available = version_info.get("available", False)
        is_remote_new = version_info.get("is_remote_new", False)
        changes = version_info.get("changes", [])
        build_time = version_info.get("build_time", "")
        exe_info = version_info.get("exe_info")

        card = QFrame()
        card.setObjectName("verCard")
        if is_current:
            card.setStyleSheet("""
                #verCard { background-color: #162016; border: 1px solid #1f3a1f; border-radius: 8px; }
                #verCard:hover { background-color: #1a2a1a; border-color: #2a4a2a; }
                QLabel { border: none; background: transparent; }
                QWidget { border: none; background: transparent; }
                QPushButton { border: none; }
            """)
        elif is_remote_new:
            card.setStyleSheet("""
                #verCard { background-color: #161620; border: 1px solid #1f3a4f; border-radius: 8px; }
                #verCard:hover { background-color: #1a1a2a; border-color: #2a4a6a; }
                QLabel { border: none; background: transparent; }
                QWidget { border: none; background: transparent; }
                QPushButton { border: none; }
            """)
        elif is_available:
            card.setStyleSheet("""
                #verCard { background-color: #161616; border: 1px solid #222; border-radius: 8px; }
                #verCard:hover { background-color: #1c1c1c; border-color: #333; }
                QLabel { border: none; background: transparent; }
                QWidget { border: none; background: transparent; }
                QPushButton { border: none; }
            """)
        else:
            card.setStyleSheet("""
                #verCard { background-color: #111; border: 1px solid #1a1a1a; border-radius: 8px; }
                #verCard:hover { background-color: #161616; border-color: #222; }
                QLabel { border: none; background: transparent; }
                QWidget { border: none; background: transparent; }
                QPushButton { border: none; }
            """)

        cl = QVBoxLayout(card)
        cl.setSpacing(4)
        cl.setContentsMargins(14, 10, 14, 10)

        header = QHBoxLayout()
        header.setSpacing(8)

        ver_label = QLabel(f"v{ver}")
        ver_label.setFont(QFont("Consolas", 11, QFont.Weight.Bold))
        if is_current:
            ver_label.setStyleSheet("color: #4CAF50;")
        elif is_remote_new:
            ver_label.setStyleSheet("color: #42A5F5;")
        elif not is_available:
            ver_label.setStyleSheet("color: #555;")
        else:
            ver_label.setStyleSheet("color: #E0E0E0;")
        header.addWidget(ver_label)

        if build_time:
            try:
                from datetime import datetime as _dt
                dt = _dt.fromisoformat(build_time)
                date_str = dt.strftime("%Y-%m-%d %H:%M")
            except:
                date_str = build_time[:16]
            date_label = QLabel(date_str)
            date_label.setFont(QFont("Consolas", 9))
            date_label.setStyleSheet("color: #555;")
            header.addWidget(date_label)

        header.addStretch()

        if is_remote_new:
            remote_tag = QLabel("🆕 远程新版本")
            remote_tag.setFont(QFont("Microsoft YaHei", 9))
            remote_tag.setStyleSheet("color: #42A5F5;")
            header.addWidget(remote_tag)
        elif is_available and exe_info and exe_info.get("size_mb"):
            size_label = QLabel(f"{exe_info['size_mb']}MB")
            size_label.setFont(QFont("Consolas", 9))
            size_label.setStyleSheet("color: #555;")
            header.addWidget(size_label)
        elif not is_available:
            status_label = QLabel("未提供")
            status_label.setFont(QFont("Microsoft YaHei", 9))
            status_label.setStyleSheet("color: #444;")
            header.addWidget(status_label)

        if is_current:
            current_tag = QLabel("● 当前版本")
            current_tag.setFont(QFont("Microsoft YaHei", 9))
            current_tag.setStyleSheet("color: #4CAF50;")
            header.addWidget(current_tag)
        elif is_available and exe_info:
            switch_btn = QPushButton("切换")
            switch_btn.setFixedWidth(55)
            switch_btn.setStyleSheet("""
                QPushButton { background-color: #1e1e1e; border: 1px solid #2a2a2a; border-radius: 4px; padding: 3px 10px; font-size: 11px; color: #AAA; }
                QPushButton:hover { background-color: #2a2a2a; border-color: #3a3a3a; color: #FFF; }
            """)
            gc = version_info.get("git_commit", "")
            switch_btn.clicked.connect(lambda checked, p=exe_info["path"], c=gc: self.updater.switch_to_exe(p, c))
            header.addWidget(switch_btn)

        cl.addLayout(header)

        detail = QWidget()
        detail.setStyleSheet("border: none; background: transparent;")
        dl = QVBoxLayout(detail)
        dl.setSpacing(2)
        dl.setContentsMargins(0, 4, 0, 0)

        git_commit = version_info.get("git_commit", "")
        if git_commit:
            commit_label = QLabel(f"🔗 commit: {git_commit}")
            commit_label.setFont(QFont("Consolas", 8))
            commit_label.setStyleSheet("color: #555;")
            dl.addWidget(commit_label)

        if changes:
            for ch in changes:
                ch_label = QLabel(f"· {ch}")
                ch_label.setFont(QFont("Microsoft YaHei", 9))
                ch_label.setStyleSheet("color: #777;")
                ch_label.setWordWrap(True)
                dl.addWidget(ch_label)
        else:
            no_ch = QLabel("暂无修改记录")
            no_ch.setFont(QFont("Microsoft YaHei", 9))
            no_ch.setStyleSheet("color: #3a3a3a;")
            dl.addWidget(no_ch)

        cl.addWidget(detail)
        self.ver_list_layout.addWidget(card)

    # ── 日志 ──
    def _append_log(self, message: str, color: str):
        ts = datetime.now().strftime("%H:%M:%S")
        line = f'<span style="color:#666">[{ts}]</span> <span style="color:{color}">{message}</span>'

        for log_widget in [self.log_text, self.deploy_log_text, self.update_log_text]:
            if log_widget:
                log_widget.append(line)
                sb = log_widget.verticalScrollBar()
                sb.setValue(sb.maximum())

        if hasattr(self, 'runtime_log_text') and self.runtime_log_text:
            self.runtime_log_text.append(line)
            sb = self.runtime_log_text.verticalScrollBar()
            sb.setValue(sb.maximum())

    def _append_debug_log(self, message: str, color: str):
        ts = datetime.now().strftime("%H:%M:%S")
        line = f'<span style="color:#666">[{ts}]</span> <span style="color:{color}">{message}</span>'

        if hasattr(self, 'debug_log_text') and self.debug_log_text:
            self.debug_log_text.append(line)
            sb = self.debug_log_text.verticalScrollBar()
            sb.setValue(sb.maximum())

    def _update_progress(self, percent: int, label: str):
        pass  # 可扩展

    def _update_deploy_step(self, key: str, state: str, value: int):
        step = self.deploy_steps.get(key)
        if not step:
            return
        btn = step["btn"]
        color = step["color"]
        label = step["label"]
        if state == "running":
            btn.setText(f"  ◐  {label}  安装中...  ")
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #1a2a1a; border: 2px solid {color}; border-radius: 4px;
                    color: #FFC107; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:disabled {{
                    background-color: #1a2a1a; color: #FFC107; border-color: {color};
                }}
            """)
            btn.setEnabled(False)
        elif state == "done":
            btn.setText(f"  ✓  {label}  已完成  ")
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #1a2a1a; border: 1px solid #333; border-radius: 4px;
                    color: #4CAF50; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:hover {{
                    background-color: #2a2a2a; border-color: {color};
                }}
            """)
            btn.setEnabled(True)
        elif state == "fail":
            btn.setText(f"  ✗  {label}  安装失败  ")
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #2a1a1a; border: 1px solid #F44336; border-radius: 4px;
                    color: #F44336; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:hover {{
                    background-color: #3a1a1a; border-color: #F44336;
                }}
            """)
            btn.setEnabled(True)
        elif state == "skip":
            btn.setText(f"  ✓  {label}  已安装  ")
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #222; border: 1px solid #333; border-radius: 4px;
                    color: #4CAF50; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:hover {{
                    background-color: #2a2a2a; border-color: {color};
                }}
            """)
            btn.setEnabled(True)
        elif state == "reset":
            btn.setText(f"  ○  {label}  待安装  ")
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: #222; border: 1px solid #333; border-radius: 4px;
                    color: #666; font-size: 11px; text-align: left;
                    padding: 2px 8px;
                }}
                QPushButton:hover {{
                    background-color: #2a2a2a; border-color: {color};
                }}
            """)
            btn.setEnabled(True)

    def _update_status(self, text: str):
        self.env_status.setText(text)

    def _on_result_ready(self, result_json: str):
        """CLI 执行完成"""
        try:
            result = json.loads(result_json)
        except:
            return
        if not result.get("ok") and result.get("error"):
            self.log_signal.emit(f"[CLI 错误] {result['error'][:200]}", "#FF9800")

    # ── 文件夹选择 ──
    def _choose_workspace_dialog(self):
        """在主线程弹出文件夹选择"""
        path = QFileDialog.getExistingDirectory(self, "选择项目目录", self.current_workspace)
        if path and os.path.isdir(path):
            self.current_workspace = path

    # ── 部署维护 ──
    def _on_mirror_changed(self, index):
        key = self.mirror_combo.currentData()
        if key:
            self.installer._save_mirror(key)
            self.log_signal.emit(f"下载源已切换为: {MIRROR_SOURCES[key]['label']}", "#4CAF50")

    def _on_deploy(self):
        if self.is_busy:
            self.log_signal.emit("⚠ 安装任务进行中，请稍候...", "#FF9800")
            return
        self.is_busy = True
        self.installer.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
        self.installer.progress = lambda p, l: self.progress_signal.emit(p, l)
        for k in self.deploy_steps:
            self.deploy_step_signal.emit(k, "reset", 0)

        deploy_order = ["node", "bun", "deps", "dist", "uv", "qwen2api"]
        install_funcs = {
            "node": self.installer.install_node,
            "bun": self.installer.install_bun,
            "deps": self.installer.install_deps,
            "dist": self.installer.build_frontend,
            "uv": self.installer.install_uv,
            "qwen2api": self.installer.install_qwen2api_deps,
        }
        check_funcs = {
            "node": self.installer.check_node,
            "bun": self.installer.check_bun,
            "deps": self.installer.check_deps,
            "dist": self.installer.check_dist,
            "uv": self.installer.check_uv,
            "qwen2api": self.installer.check_qwen2api,
        }

        def _deploy():
            try:
                self.log_signal.emit("━━━ 部署维护 ━━━", "#2E7D32")
                total = len(deploy_order)
                for i, key in enumerate(deploy_order):
                    if check_funcs[key]():
                        self.deploy_step_signal.emit(key, "skip", 100)
                        continue
                    self.deploy_step_signal.emit(key, "running", int(i / total * 100))
                    func = install_funcs[key]
                    ok = func()
                    if ok:
                        self.deploy_step_signal.emit(key, "done", 100)
                    else:
                        self.deploy_step_signal.emit(key, "fail", int(i / total * 100))
                self.log_signal.emit("✓ 部署维护完成", "#4CAF50")
            except Exception as e:
                self.log_signal.emit(f"⚠ 部署维护异常: {e}", "#FF9800")
            finally:
                self.is_busy = False
            self._update_env_status()
            QTimer.singleShot(100, self._refresh_deploy_env_status)
            QTimer.singleShot(500, self._load_frontend)

        t = threading.Thread(target=_deploy, daemon=True)
        t.start()

    def _on_install_single(self, component: str):
        if self.is_busy:
            self.log_signal.emit("⚠ 安装任务进行中，请稍候...", "#FF9800")
            return
        self.is_busy = True
        self.installer.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
        self.installer.progress = lambda p, l: self.progress_signal.emit(p, l)

        install_funcs = {
            "node": self.installer.install_node,
            "bun": self.installer.install_bun,
            "deps": self.installer.install_deps,
            "dist": self.installer.build_frontend,
            "uv": self.installer.install_uv,
            "qwen2api": self.installer.install_qwen2api_deps,
        }

        func = install_funcs.get(component)
        if not func:
            self.is_busy = False
            return

        def _install():
            try:
                labels = {"node": "Node.js", "bun": "Bun", "deps": "npm 依赖", "dist": "前端构建", "uv": "uv", "qwen2api": "API 服务依赖"}
                self.deploy_step_signal.emit(component, "running", 30)
                self.log_signal.emit(f"━━━ 安装 {labels.get(component, component)} ━━━", "#1976D2")
                if func():
                    self.deploy_step_signal.emit(component, "done", 100)
                    self.log_signal.emit(f"✓ {labels.get(component, component)} 安装完成", "#4CAF50")
                else:
                    self.deploy_step_signal.emit(component, "fail", 30)
                    self.log_signal.emit(f"✗ {labels.get(component, component)} 安装失败", "#F44336")
            finally:
                self.is_busy = False
            self._update_env_status()
            QTimer.singleShot(100, self._refresh_deploy_env_status)

        t = threading.Thread(target=_install, daemon=True)
        t.start()

    # ── 软件更新 ──
    def _on_update(self):
        """检查更新"""
        self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)

        if not self.updater.is_git_repo():
            self.log_signal.emit("当前不是 Git 仓库，无法检查更新", "#FF9800")
            self.update_info_label.setText("<span style='color:#F44336'>当前不是 Git 仓库，无法检查更新</span>")
            return

        self.btn_check_update.setEnabled(False)
        self.update_info_label.setText("正在检查更新...")

        def _check():
            result = self.updater.check_update()
            self.update_info_signal.emit(json.dumps(result))

        t = threading.Thread(target=_check, daemon=True)
        t.start()

        try:
            self.update_info_signal.disconnect(self._on_update_result)
        except:
            pass
        self.update_info_signal.connect(self._on_update_result)

    def _on_update_result(self, info_json: str):
        """更新检查结果回调"""
        self.btn_check_update.setEnabled(True)
        try:
            info = json.loads(info_json)
        except:
            return

        local = info.get("local", "unknown")
        remote = info.get("remote", "unknown")
        has_update = info.get("has_update", False)
        error = info.get("error", "")

        if error:
            self.update_info_label.setText(f"<b>本地版本:</b> {local} | <b>远程版本:</b> {remote}<br><span style='color:#FF9800'>{error}</span>")
        elif has_update:
            self.update_info_label.setText(f"<b>本地版本:</b> {local} | <b>远程版本:</b> <span style='color:#4CAF50'>{remote}</span><br><span style='color:#4CAF50'>发现资源包更新！</span>")
            self.btn_pull_update.setEnabled(True)
        else:
            self.update_info_label.setText(f"<b>本地版本:</b> {local} | <b>远程版本:</b> {remote}<br>资源包已是最新版本")
            self.btn_pull_update.setEnabled(False)

    def _do_pull_update(self):
        """执行资源包更新"""
        self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
        self.updater.progress = lambda p, l: self.progress_signal.emit(p, l)
        self.btn_pull_update.setEnabled(False)
        self.btn_check_update.setEnabled(False)

        def _pull():
            if self.updater.pull_update():
                self.log_signal.emit("✓ 资源包更新完成", "#4CAF50")
            else:
                self.log_signal.emit("✗ 资源包更新失败", "#F44336")
            self.btn_check_update.setEnabled(True)
            # 更新后重新检查版本
            QTimer.singleShot(500, self._on_update)

        t = threading.Thread(target=_pull, daemon=True)
        t.start()

    # ── 关闭 ──
    def closeEvent(self, event):
        event.accept()


def main():
    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )

    app = QApplication(sys.argv)

    palette = app.palette()
    palette.setColor(palette.ColorRole.Window, QColor("#0d0d0d"))
    palette.setColor(palette.ColorRole.WindowText, QColor("#f0f0f0"))
    palette.setColor(palette.ColorRole.Base, QColor("#0d0d0d"))
    palette.setColor(palette.ColorRole.Text, QColor("#f0f0f0"))
    palette.setColor(palette.ColorRole.Button, QColor("#1a1a1a"))
    palette.setColor(palette.ColorRole.ButtonText, QColor("#f0f0f0"))
    app.setPalette(palette)

    window = MainWindow()
    window.resize(1260, 860)
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
