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
    QMessageBox, QFileDialog,
)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, QTimer, QUrl
from PyQt6.QtGui import QFont, QIcon
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWebChannel import QWebChannel

from PyQt6.QtCore import QObject

# 导入后端模块
from backend import (
    EnvFileManager, OllamaProxyServer, ClaudeCliRunner,
    list_openrouter_models, list_anthropic_models, list_ollama_models,
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

# ── Git 仓库配置 ──
GIT_REMOTE = "git@gitee.com:yunjii/code.git"
GIT_BRANCH = "main"


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

    def switch_to_exe(self, exe_path: str):
        """切换到指定 EXE 并重启（当前 EXE 退出后启动新 EXE）"""
        if not os.path.exists(exe_path):
            self.log(f"[错误] EXE 不存在: {exe_path}", "#F44336")
            return False

        # 构造重启命令：等待当前进程退出后启动新 EXE
        current_pid = os.getpid()
        new_exe = exe_path
        # 用 ping 延迟等待当前进程退出
        cmd = f'ping -n 3 127.0.0.1 >nul & start "" "{new_exe}"'
        subprocess.Popen(cmd, shell=True, creationflags=subprocess.CREATE_NO_WINDOW)

        self.log(f"正在切换到 {os.path.basename(exe_path)}...", "#4CAF50")

        # 退出当前程序
        QApplication.quit()
        return True


# ── QWebChannel 桥接对象 (替代 Electron preload.cjs) ──
class BackendBridge(QObject):
    """暴露给前端 JS 的 Python 对象，替代 Electron 的 desktopApi"""

    # 信号：前端通过 onDelta/onStatus 连接
    deltaReceived = pyqtSignal(str)   # JSON string: {"text": "..."}
    statusReceived = pyqtSignal(str)  # JSON string: {"busy": true, ...}

    def __init__(self, parent=None):
        super().__init__(parent)
        self._app_ref = None  # 由 MainWindow 设置

    def _get_main(self):
        return self._app_ref

    # ── 前端可调用方法 (自动暴露为 JS 方法) ──

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

    def newSession(self):
        main = self._get_main()
        if not main:
            return json.dumps({"sessionId": ""})
        main.active_session_id = _uuid()
        return json.dumps({"sessionId": main.active_session_id})

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
            return json.dumps({"ok": False, "error": "A request is already running."})

        prompt = (payload.get("prompt") or "").strip()
        if not prompt:
            return json.dumps({"ok": False, "error": "Prompt cannot be empty."})

        env = main.env_manager
        settings = env.read_settings()
        provider = (payload.get("provider") or settings.get("MODEL_PROVIDER", "anthropic")).lower()
        model = (payload.get("model") or "").strip()
        if not model:
            model = settings.get("OLLAMA_MODEL", "") if provider == "ollama" else settings.get("ANTHROPIC_MODEL", "")

        # 启动异步处理
        main.is_busy = True
        self.statusReceived.emit(json.dumps({"busy": True}))

        t = threading.Thread(target=self._run_cli, args=(prompt, model, provider, settings), daemon=True)
        t.start()

        return json.dumps({"ok": True, "sessionId": main.active_session_id})

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

    def getWorkspace(self):
        main = self._get_main()
        path = main.current_workspace if main else ""
        return json.dumps({"path": path})

    def chooseWorkspace(self):
        """由 Python 端弹出文件夹选择对话框"""
        main = self._get_main()
        if not main:
            return json.dumps({"ok": False, "error": "App not ready"})
        # 需要在主线程执行，用信号通知
        main.workspace_choose_requested.emit()
        return json.dumps({"ok": True, "path": main.current_workspace})

    def getSettings(self):
        main = self._get_main()
        if not main:
            return json.dumps({})
        return json.dumps(main.env_manager.read_settings())

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

    def clearModelSettings(self):
        main = self._get_main()
        if not main:
            return json.dumps({})
        result = main.env_manager.clear_model_settings()
        return json.dumps(result)

    def listModels(self, payload_json: str):
        """获取模型列表"""
        try:
            payload = json.loads(payload_json) if isinstance(payload_json, str) else payload_json
        except:
            return json.dumps({"ok": False, "error": "Invalid payload"})

        source = (payload.get("source") or "").lower()
        main = self._get_main()
        settings = main.env_manager.read_settings() if main else {}
        timeout = int(settings.get("API_TIMEOUT_MS", "15000") or "15000")

        if source == "openrouter":
            result = list_openrouter_models(timeout)
        elif source == "anthropic":
            api_key = payload.get("apiKey", "") or settings.get("ANTHROPIC_API_KEY", "")
            result = list_anthropic_models(api_key, timeout)
        elif source == "ollama":
            base_url = payload.get("baseUrl", "") or settings.get("OLLAMA_BASE_URL", "") or "http://127.0.0.1:11434"
            result = list_ollama_models(base_url, timeout)
        else:
            result = {"ok": False, "error": "Unsupported source."}

        return json.dumps(result)

    # ── 内部方法 ──

    def _run_cli(self, prompt: str, model: str, provider: str, settings: dict):
        """在后台线程运行 CLI"""
        main = self._get_main()
        if not main:
            return

        env_overrides = None
        if provider == "ollama":
            ollama_target = (settings.get("OLLAMA_BASE_URL", "") or "http://127.0.0.1:11434").strip()
            ollama_model = (settings.get("OLLAMA_MODEL", "") or "qwen3:8b").strip()
            proxy_port = main.ollama_proxy.start(ollama_target, ollama_model)
            env_overrides = {
                "ANTHROPIC_BASE_URL": f"http://127.0.0.1:{proxy_port}",
                "ANTHROPIC_API_KEY": "ollama-local",
                "ANTHROPIC_AUTH_TOKEN": "ollama-local",
                "ANTHROPIC_MODEL": ollama_model,
            }

        is_resuming = main.active_session_id in main.started_sessions
        result = main.cli_runner.run(
            prompt=prompt,
            session_id=main.active_session_id,
            model=model,
            is_resuming=is_resuming,
            workspace_path=main.current_workspace,
            env_overrides=env_overrides,
            on_delta=lambda text: self.deltaReceived.emit(json.dumps({"text": text})),
        )

        if result.get("ok"):
            main.started_sessions.add(main.active_session_id)

        main.is_busy = False
        self.statusReceived.emit(json.dumps({"busy": False}))

        # 通知前端最终结果
        main.result_ready_signal.emit(json.dumps(result))


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

    def check_node(self): return os.path.exists(self.node_exe)
    def check_bun(self): return os.path.exists(self.bun_exe)
    def check_deps(self):
        tsx = os.path.join(self.base_dir, "node_modules", "tsx", "dist", "loader.mjs")
        return os.path.exists(os.path.join(self.base_dir, "node_modules")) and os.path.exists(tsx)
    def check_dist(self): return os.path.exists(os.path.join(self.base_dir, "desktop", "dist"))
    def check_electron(self):
        return os.path.exists(os.path.join(self.base_dir, "node_modules", ".bin", "electron.cmd"))

    def check_all(self):
        return {
            "node": self.check_node(), "bun": self.check_bun(),
            "deps": self.check_deps(), "dist": self.check_dist(),
            "electron": self.check_electron(),
        }

    def _download(self, url: str, dest: str, label: str):
        self.log(f"正在下载 {label}...")
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
        url = f"https://nodejs.org/dist/{NODE_VERSION}/node-{NODE_VERSION}-win-x64.zip"
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
        url = f"https://github.com/oven-sh/bun/releases/download/bun-v{BUN_VERSION}/bun-windows-x64.zip"
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
                           cwd=self.base_dir, env=env, startupinfo=self._si(), capture_output=True, timeout=30)
            r = subprocess.run([self.bun_exe, "install"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=300)
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
                               startupinfo=self._si(), capture_output=True, text=True, timeout=120)
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
        ])


# ── 主窗口 ──
class MainWindow(QMainWindow):
    log_signal = pyqtSignal(str, str)
    progress_signal = pyqtSignal(int, str)
    status_signal = pyqtSignal(str)
    result_ready_signal = pyqtSignal(str)  # JSON result from CLI
    workspace_choose_requested = pyqtSignal()
    update_info_signal = pyqtSignal(str)   # JSON: 更新信息

    def __init__(self):
        super().__init__()
        self.setWindowTitle(f"云集智能编程工作站 v{VERSION}")
        self.setMinimumSize(980, 680)

        # 图标
        try:
            if hasattr(sys, 'frozen'):
                icon_path = os.path.join(os.path.dirname(sys.executable), "icon.ico")
            else:
                icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.ico")
            if os.path.exists(icon_path):
                self.setWindowIcon(QIcon(icon_path))
        except:
            pass

        # 基础目录
        # --onedir 打包后: EXE 在 build/发布/xxx/ 下，_internal/ 和 desktop/ 同级
        # 开发模式: 脚本在 dev/app/ 下
        if hasattr(sys, 'frozen'):
            # PyInstaller 打包模式：EXE 所在目录就是 base_dir
            self.base_dir = os.path.abspath(os.path.dirname(sys.executable))
            # dev/ 根目录 = EXE 所在目录（EXE 直接放在 dev/ 下）
            # 或 EXE 在 ver/ 子目录下
            exe_dir = os.path.dirname(sys.executable)
            if os.path.basename(exe_dir) == "ver":
                self.dev_dir = os.path.dirname(exe_dir)
            else:
                self.dev_dir = exe_dir
        else:
            self.base_dir = os.path.dirname(os.path.abspath(__file__))
            # 开发模式: main.py 在 dev/app/ 下，dev/ 是上级目录
            self.dev_dir = os.path.dirname(self.base_dir)

        # 初始化后端
        self.env_manager = EnvFileManager(os.path.join(self.base_dir, ".env"))
        self.ollama_proxy = OllamaProxyServer()
        self.cli_runner = ClaudeCliRunner(
            self.base_dir,
            os.path.join(self.base_dir, "nodejs", NODE_DIR_NAME),
            os.path.join(self.base_dir, "bun", BUN_DIR_NAME),
        )
        self.installer = EnvInstaller(self.base_dir)
        self.updater = SoftwareUpdater(self.dev_dir)

        # 状态
        self.active_session_id = _uuid()
        self.started_sessions = set()
        self.current_workspace = self.base_dir
        self.is_busy = False
        self.active_proc = None

        # 构建 UI
        self._setup_ui()

        # 连接信号
        self.log_signal.connect(self._append_log)
        self.progress_signal.connect(self._update_progress)
        self.status_signal.connect(self._update_status)
        self.result_ready_signal.connect(self._on_result_ready)
        self.workspace_choose_requested.connect(self._choose_workspace_dialog)

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

        # ── 顶部工具栏 ──
        toolbar = QFrame()
        toolbar.setFixedHeight(48)
        toolbar.setStyleSheet("QFrame { background-color: #1a1a1a; border-bottom: 1px solid #2a2a2a; }")
        tb_layout = QHBoxLayout(toolbar)
        tb_layout.setContentsMargins(12, 0, 12, 0)

        title = QLabel("💻 云集智能编程工作站")
        title.setFont(QFont("Microsoft YaHei", 11, QFont.Weight.Bold))
        title.setStyleSheet("color: #fff; border: none;")
        tb_layout.addWidget(title)

        tb_layout.addStretch()

        self.status_label = QLabel("⏹ 就绪")
        self.status_label.setStyleSheet("color: #888; font-size: 12px; border: none;")
        tb_layout.addWidget(self.status_label)

        self.btn_deploy = QPushButton("⚙️ 部署维护")
        self.btn_deploy.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: 2px solid #388E3C; border-radius: 6px; padding: 6px 14px; font-size: 12px; }
            QPushButton:hover { background-color: #388E3C; }
        """)
        self.btn_deploy.clicked.connect(self._on_deploy)
        tb_layout.addWidget(self.btn_deploy)

        self.btn_update = QPushButton("🔄 软件更新")
        self.btn_update.setStyleSheet("""
            QPushButton { background-color: #1565C0; border: 2px solid #1976D2; border-radius: 6px; padding: 6px 14px; font-size: 12px; }
            QPushButton:hover { background-color: #1976D2; }
        """)
        self.btn_update.clicked.connect(self._on_update)
        tb_layout.addWidget(self.btn_update)

        layout.addWidget(toolbar)

        # ── 主内容区：WebEngine + 日志面板（可折叠） ──
        self.splitter = QFrame()
        splitter_layout = QVBoxLayout(self.splitter)
        splitter_layout.setSpacing(0)
        splitter_layout.setContentsMargins(0, 0, 0, 0)

        # QWebEngineView 加载 Vue 前端
        self.web_view = QWebEngineView()
        self.web_view.setStyleSheet("background-color: #0d0d0d;")

        # QWebChannel 桥接
        self.channel = QWebChannel()
        self.bridge = BackendBridge()
        self.bridge._app_ref = self
        self.channel.registerObject("backend", self.bridge)
        self.web_view.page().setWebChannel(self.channel)

        splitter_layout.addWidget(self.web_view, 1)

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

        splitter_layout.addWidget(self.log_panel)

        layout.addWidget(self.splitter, 1)

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

        self.btn_show_log = QPushButton("📋 日志")
        self.btn_show_log.setStyleSheet("QPushButton { background: #333; border: 1px solid #444; border-radius: 4px; padding: 2px 8px; font-size: 10px; }")
        self.btn_show_log.clicked.connect(lambda: self.log_panel.setVisible(not self.log_panel.isVisible()))
        sb_layout.addWidget(self.btn_show_log)

        layout.addWidget(statusbar)

    # ── 环境检查与自动加载 ──
    def _auto_check_and_load(self):
        """启动时自动检查环境，如需安装则先安装，然后加载前端"""
        def _check():
            checks = self.installer.check_all()
            all_ok = all(checks.values())

            # 更新环境状态
            parts = []
            for k, v in checks.items():
                parts.append(f"{k}:{'✓' if v else '✗'}")
            self.log_signal.emit("环境检查: " + " ".join(parts), "#666")

            if not all_ok:
                self.log_signal.emit("环境未完全就绪，开始自动安装...", "#FF9800")
                self.installer.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
                self.installer.progress = lambda p, l: self.progress_signal.emit(p, l)

                if self.installer.install_all():
                    self.log_signal.emit("✓ 环境安装完成", "#4CAF50")
                else:
                    self.log_signal.emit("⚠ 部分环境安装失败，请使用部署维护", "#FF9800")

            # 加载前端
            QTimer.singleShot(100, self._load_frontend)

        t = threading.Thread(target=_check, daemon=True)
        t.start()

    def _load_frontend(self):
        """加载 Vue 前端到 QWebEngineView"""
        # 先注入 QWebChannel 的 qwebchannel.js
        dist_path = os.path.join(self.base_dir, "desktop", "dist", "index.html")

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
        labels = {"node": "Node", "bun": "Bun", "deps": "依赖", "dist": "前端", "electron": "Electron"}
        for k, v in checks.items():
            parts.append(f"{labels.get(k, k)}:{'✓' if v else '✗'}")
        self.env_status.setText("环境: " + " | ".join(parts))

    # ── 日志 ──
    def _append_log(self, message: str, color: str):
        if not self.log_text:
            return
        ts = datetime.now().strftime("%H:%M:%S")
        self.log_text.append(f'<span style="color:#666">[{ts}]</span> <span style="color:{color}">{message}</span>')
        sb = self.log_text.verticalScrollBar()
        sb.setValue(sb.maximum())

    def _update_progress(self, percent: int, label: str):
        pass  # 可扩展

    def _update_status(self, text: str):
        self.status_label.setText(text)

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
    def _on_deploy(self):
        if self.is_busy:
            return
        self.log_panel.setVisible(True)
        self.installer.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)

        def _deploy():
            self.log_signal.emit("━━━ 部署维护 ━━━", "#2E7D32")
            if self.installer.install_all():
                self.log_signal.emit("✓ 部署维护完成", "#4CAF50")
            else:
                self.log_signal.emit("⚠ 部署维护部分失败", "#FF9800")
            self._update_env_status()
            # 重新加载前端
            QTimer.singleShot(500, self._load_frontend)

        t = threading.Thread(target=_deploy, daemon=True)
        t.start()

    # ── 软件更新 ──
    def _on_update(self):
        """打开软件更新对话框"""
        self.log_panel.setVisible(True)
        self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)

        # 检查更新
        self.log_signal.emit("━━━ 软件更新 ━━━", "#1565C0")

        if not self.updater.is_git_repo():
            self.log_signal.emit("当前不是 Git 仓库，无法检查更新", "#FF9800")
            return

        def _check_and_show():
            result = self.updater.check_update()
            self.update_info_signal.emit(json.dumps(result))

        t = threading.Thread(target=_check_and_show, daemon=True)
        t.start()

        # 连接信号，收到结果后弹出对话框
        try:
            self.update_info_signal.disconnect(self._show_update_dialog)
        except:
            pass
        self.update_info_signal.connect(self._show_update_dialog)

    def _show_update_dialog(self, info_json: str):
        """显示更新信息对话框"""
        try:
            info = json.loads(info_json)
        except:
            return

        local = info.get("local", "unknown")
        remote = info.get("remote", "unknown")
        has_update = info.get("has_update", False)
        error = info.get("error", "")

        # 构建信息文本
        lines = [
            f"<b>当前资源包版本:</b> {local}",
            f"<b>远程最新版本:</b> {remote}",
        ]

        if error:
            lines.append(f"<br><span style='color:#FF9800'>{error}</span>")

        # 列出稳定版 EXE
        stable_exes = self.updater.list_stable_exes()
        if stable_exes:
            lines.append("<br><b>稳定版 EXE:</b>")
            for exe in stable_exes:
                current_marker = ""
                if hasattr(sys, 'frozen'):
                    current_exe = os.path.basename(sys.executable)
                    if exe["filename"] == current_exe:
                        current_marker = " <span style='color:#4CAF50'>(当前)</span>"
                lines.append(f"  {exe['filename']} ({exe['size_mb']}MB){current_marker}")

        if not stable_exes:
            lines.append("<br><span style='color:#888'>暂无稳定版 EXE（ver/ 目录为空）</span>")

        msg_text = "<br>".join(lines)

        # 弹出对话框
        dlg = QMessageBox(self)
        dlg.setWindowTitle("软件更新")
        dlg.setTextFormat(Qt.TextFormat.RichText)
        dlg.setText(msg_text)
        dlg.setStyleSheet("""
            QMessageBox { background-color: #1a1a1a; color: #f0f0f0; }
            QLabel { color: #f0f0f0; }
            QPushButton { background-color: #333; border: 1px solid #555; border-radius: 4px; padding: 6px 16px; color: white; min-width: 80px; }
            QPushButton:hover { background-color: #444; }
        """)

        # 按钮
        if has_update:
            btn_update = dlg.addButton("📥 更新资源包", QMessageBox.ButtonRole.AcceptRole)
            btn_update.setStyleSheet("background-color: #1565C0; border: 1px solid #1976D2;")
        dlg.addButton("关闭", QMessageBox.ButtonRole.RejectRole)

        # 如果有稳定版 EXE 且有多个，添加切换按钮
        if stable_exes and hasattr(sys, 'frozen'):
            current_exe = os.path.basename(sys.executable)
            other_exes = [e for e in stable_exes if e["filename"] != current_exe]
            if other_exes:
                btn_switch = dlg.addButton("🔄 切换到最新稳定版", QMessageBox.ButtonRole.ResetRole)
                btn_switch.setStyleSheet("background-color: #6A1B9A; border: 1px solid #7B1FA2;")

        dlg.exec()

        clicked = dlg.clickedButton()
        if has_update and hasattr(self, '_update_btn_ref') is False:
            pass  # 不会走到这里

        # 判断点击了哪个按钮
        if has_update:
            try:
                if clicked == btn_update:
                    self._do_pull_update()
            except:
                pass

        try:
            if clicked == btn_switch and other_exes:
                self._do_switch_exe(other_exes[0]["path"])
        except:
            pass

    def _do_pull_update(self):
        """执行资源包更新"""
        self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
        self.updater.progress = lambda p, l: self.progress_signal.emit(p, l)

        def _pull():
            if self.updater.pull_update():
                self.log_signal.emit("✓ 资源包更新完成，部分功能可能需要重启生效", "#4CAF50")
            else:
                self.log_signal.emit("✗ 资源包更新失败", "#F44336")

        t = threading.Thread(target=_pull, daemon=True)
        t.start()

    def _do_switch_exe(self, exe_path: str):
        """切换到指定稳定版 EXE"""
        exe_name = os.path.basename(exe_path)
        reply = QMessageBox.question(
            self, "切换 EXE 版本",
            f"确定要切换到 {exe_name} 吗？\n当前程序将退出并启动新版本。",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )
        if reply == QMessageBox.StandardButton.Yes:
            self.updater.switch_to_exe(exe_path)

    # ── 关闭 ──
    def closeEvent(self, event):
        self.ollama_proxy.stop()
        event.accept()


def main():
    # 设置高 DPI
    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )

    app = QApplication(sys.argv)
    window = MainWindow()
    window.resize(1260, 860)
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
