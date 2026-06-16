#!/usr/bin/env python3
"""
云集智能编程工作站 - 统一启动器 v3.1
所有功能内嵌在一个 EXE 中，不再依赖 Electron

架构:
- PyQt6 + Edge WebView2 (pywebview) 替代 Electron/QWebEngine
- pywebview js_api 替代 QWebChannel IPC
- backend.py 提供 Ollama 代理 / CLI 管理 / 配置管理
- Vue 前端通过 window.pywebview.api 与 Python 通信
- 部署维护只是 EXE 的一个功能模块
"""

import sys
import os

# 将 Python 字节码缓存目录重定向到 temp/__pycache__/
# 避免在 app/ 目录下生成 __pycache__ 污染代码目录
if hasattr(sys, 'frozen'):
    # 打包模式: 使用 exe 同级 temp/
    _temp_pycache = os.path.join(os.path.abspath(os.path.dirname(sys.executable)), "temp", "__pycache__")
else:
    # 开发模式: 使用 dev/temp/
    _script_dir = os.path.dirname(os.path.abspath(__file__))
    _temp_pycache = os.path.join(os.path.dirname(_script_dir), "temp", "__pycache__")
os.makedirs(_temp_pycache, exist_ok=True)
os.environ["PYTHONPYCACHEPREFIX"] = _temp_pycache

# ── onefile 模式: 将嵌入资源目录加入 sys.path ──
# PyInstaller --onefile 将所有嵌入文件解压到 _MEIPASS 临时目录
# 需要将 _MEIPASS 加入 sys.path 以便 import backend 等模块
if getattr(sys, 'frozen', False) and hasattr(sys, '_MEIPASS'):
    if sys._MEIPASS not in sys.path:
        sys.path.insert(0, sys._MEIPASS)

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
import json
import time
import re
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
    QTabWidget, QScrollArea, QComboBox, QSplashScreen,
    QSplitter, QListWidget, QListWidgetItem, QLineEdit, QCheckBox, QInputDialog
)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, pyqtSlot, QTimer, QUrl, QPropertyAnimation, QEasingCurve, pyqtProperty, QRectF, QEvent, QSize
from PyQt6.QtGui import QFont, QIcon, QColor, QPixmap, QPainter, QLinearGradient, QPalette, QFontDatabase

try:
    import webview
    WEBVIEW_AVAILABLE = True
except ImportError:
    WEBVIEW_AVAILABLE = False

# QtWebView2: 轻量级 WebView2 控件，支持内嵌到 Qt 布局，EXE 不膨胀
try:
    from qtwebview2 import QtWebView2Widget
    QTWEBVIEW2_AVAILABLE = True
except ImportError:
    QTWEBVIEW2_AVAILABLE = False

from PyQt6.QtCore import QObject


MENU_TRANSLATIONS = {
    "Back": "后退",
    "Forward": "前进",
    "Reload": "重新加载",
    "Cut": "剪切",
    "Copy": "复制",
    "Paste": "粘贴",
    "Undo": "撤销",
    "Redo": "重做",
    "Select All": "全选",
    "Inspect": "检查",
    "Save Image": "保存图片",
    "Copy Image": "复制图片",
    "Copy Link": "复制链接",
    "Copy Image Address": "复制图片地址",
    "Save Link": "保存链接",
    "Open Link in New Tab": "在新标签页中打开链接",
    "View Source": "查看源代码",
    "Copy Link Address": "复制链接地址",
    "Open Link in New Window": "在新窗口中打开链接",
    "Open Image in New Tab": "在新标签页中打开图片",
    "Save Page As": "页面另存为",
    "Copy Page Link": "复制页面链接",
    "Select All Text": "全选文本",
    "Search": "搜索",
    "Translate": "翻译",
    "Print": "打印",
    "Create QR Code for this Page": "为本页创建二维码",
    "Cast": "投射",
    "Share": "分享",
    "Exit Full Screen": "退出全屏",
    "Enter Full Screen": "进入全屏",
    "Mute Site": "静音网站",
    "Unmute Site": "取消静音网站",
    "Check Spelling": "检查拼写",
    "Look Up": "查找",
    "Search with Google": "使用Google搜索",
    "Search the Web": "搜索网页",
    "Add to Dictionary": "添加到字典",
    "No suggestions": "无建议",
    "Spelling Suggestions": "拼写建议",
    "Inspect Element": "检查元素",
}


class BackendBridge:
    """暴露给前端 JS 的 Python 对象，通过 pywebview js_api 暴露"""

    def __init__(self, parent=None):
        self._app_ref = None  # 由 MainWindow 设置

    def _get_main(self):
        return self._app_ref

# 导入后端模块
import backend
from backend import (
    EnvFileManager, ClaudeCliRunner,
    list_openrouter_models, list_anthropic_models, list_ollama_models, list_api_models,
    list_zhipu_models, check_zhipu_api, ZHIPU_DEFAULT_BASE_URL,
    SETTINGS_KEYS,
)


# ══════════════════════════════════════════════════════════════
# 品牌名（杀同名逻辑依赖，必须在单实例控制区段之前定义）
# ══════════════════════════════════════════════════════════════
BRAND_NAME = "云集智能编程工作站"


# ══════════════════════════════════════════════════════════════
# 单实例控制: 杀同名进程 + mutex 占位（最简单方案）
# ══════════════════════════════════════════════════════════════
# 设计:
#   - 启动时直接杀掉所有同名前缀的旧 EXE（杀同名是最简单的方式）
#   - mutex 仅用于防两个进程同时启动时的 race
#   - 不需要共享内存 / SHUTDOWN_EVENT / 激活窗口
#   - 升级场景: 旧 EXE 被杀 → 启动新 EXE，天然完成切换
#   - 资源名: 固定不带版本号（升级是切换版本，不存在同时运行）

import ctypes
from ctypes import wintypes

_kernel32 = ctypes.windll.kernel32
_user32 = ctypes.windll.user32

ERROR_ALREADY_EXISTS = 183
TH32CS_SNAPPROCESS = 0x00000002
INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value

MUTEX_NAME = "YunJiCode_SingleInstance"


class PROCESSENTRY32W(ctypes.Structure):
    _fields_ = [
        ("dwSize", ctypes.wintypes.DWORD),
        ("cntUsage", ctypes.wintypes.DWORD),
        ("th32ProcessID", ctypes.wintypes.DWORD),
        ("th32DefaultHeapID", ctypes.POINTER(ctypes.c_ulong)),
        ("th32ModuleID", ctypes.wintypes.DWORD),
        ("cntThreads", ctypes.wintypes.DWORD),
        ("th32ParentProcessID", ctypes.wintypes.DWORD),
        ("pcPriClassBase", ctypes.c_long),
        ("dwFlags", ctypes.wintypes.DWORD),
        ("szExeFile", ctypes.c_wchar * 260),
    ]


def _kill_same_name_processes():
    """杀掉所有同名旧进程（EXE 模式按 BRAND_NAME 前缀，开发模式按 main.py）"""
    if sys.platform != 'win32':
        return

    my_pid = _kernel32.GetCurrentProcessId()
    is_frozen = getattr(sys, 'frozen', False)
    base_prefix = BRAND_NAME.lower()

    # 1. 创建进程快照
    snap = _kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snap == INVALID_HANDLE_VALUE:
        return

    # 2. 收集当前进程的祖先链（防止杀掉自己的父/祖父导致自己被连带）
    my_ancestor_pids = set()
    try:
        import psutil as _ps_anc
        cur = _ps_anc.Process(my_pid)
        while True:
            par = cur.parent()
            if par is None or par.pid == 0:
                break
            my_ancestor_pids.add(par.pid)
            cur = par
    except Exception:
        pass

    # 3. 遍历进程，找同名
    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
    pids_to_kill = []

    if _kernel32.Process32FirstW(snap, ctypes.byref(entry)):
        while True:
            pid = entry.th32ProcessID
            if pid != my_pid and pid not in my_ancestor_pids:
                exe_name = (entry.szExeFile or "").lower()
                should_kill = False
                if is_frozen:
                    # EXE 模式: 同名前缀的 .exe
                    should_kill = (
                        exe_name.startswith(base_prefix)
                        and exe_name.endswith('.exe')
                    )
                else:
                    # 开发模式: 杀所有跑 main.py 的 python.exe
                    # (bat 启动时 cmdline 可能是 "python main.py" 不含 1.pc,
                    #  所以只用 main.py 字符串匹配，不强制 1.pc)
                    if exe_name in ('python.exe', 'pythonw.exe'):
                        try:
                            import psutil
                            cmdline = ' '.join(psutil.Process(pid).cmdline()).lower()
                            if 'main.py' in cmdline:
                                should_kill = True
                        except Exception:
                            pass
                if should_kill:
                    pids_to_kill.append(pid)

            entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
            if not _kernel32.Process32NextW(snap, ctypes.byref(entry)):
                break

    _kernel32.CloseHandle(snap)

    PROCESS_TERMINATE = 0x0001
    for pid in pids_to_kill:
        h = _kernel32.OpenProcess(PROCESS_TERMINATE, False, pid)
        if h:
            _kernel32.TerminateProcess(h, 0)
            _kernel32.CloseHandle(h)

    # 等旧进程退出（最多 2 秒）
    if pids_to_kill:
        for _ in range(20):
            time.sleep(0.1)
            still_alive = []
            for pid in pids_to_kill:
                h = _kernel32.OpenProcess(0x00100000, False, pid)
                if h:
                    exit_code = ctypes.c_ulong()
                    if _kernel32.GetExitCodeProcess(h, ctypes.byref(exit_code)) and exit_code.value == 259:
                        still_alive.append(pid)
                    _kernel32.CloseHandle(h)
            if not still_alive:
                break


def _ensure_single_instance():
    """单实例控制: 杀同名进程（mutex 仅做一次性冲突检测，用完即弃）

    设计:
      - 杀同名前缀的旧 EXE / 旧 python main.py（最简单方式）
      - mutex 不保留句柄，避免泄漏导致后续启动冲突
      - 杀得彻底的话，mutex 永远成功
      - 杀不彻底（race 或被杀进程未响应）→ 弹窗提示并退出
    """
    if sys.platform != 'win32':
        return

    # 1. 杀所有同名旧进程
    _kill_same_name_processes()

    # 2. 创建 mutex 做一次性冲突检测（用完即 CloseHandle，不留句柄）
    m = _kernel32.CreateMutexW(None, True, MUTEX_NAME)
    last_err = ctypes.GetLastError()
    if m:
        _kernel32.CloseHandle(m)  # 立即释放（关键：不留句柄）
    if last_err == ERROR_ALREADY_EXISTS:
        # 杀得不彻底（极端 race / 旧进程未响应）
        ctypes.windll.user32.MessageBoxW(
            0, f"{BRAND_NAME} 旧实例未能退出，请手动结束进程后重试。", "提示", 0x40
        )
        sys.exit(0)


def _cleanup_single_instance():
    """清理单实例资源（窗口关闭时调用）— 当前无需清理（mutex 已用完即弃）"""
    pass


# ══════════════════════════════════════════════════════════════
# ── 版本号 ──
def get_version_from_filename():
    try:
        # 优先使用构建信息 (build_pc.py 在打包时写入 _build_info.py)
        try:
            from _build_info import BUILD_VERSION
            return BUILD_VERSION
        except Exception:
            pass
        if hasattr(sys, 'frozen'):
            exe_name = os.path.basename(sys.executable)
            import re
            m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', exe_name)
            if m:
                return m.group(1)
        # 2026-06-16 修复: 开发模式（python main.py）下回退用稳定字符串 "dev"
        # 之前用 datetime.now() 导致每次启动 VERSION 都不同
        return "dev"
    except:
        return "dev"


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


class ProjectManager:
    """项目管理器 - 支持多用户数据隔离架构
    
    数据目录结构 (数据分级):
        {base_dir}/
        └── data/                    # 数据总目录
            ├── public/              # 公共数据 (所有用户共享)
            │   ├── models/         # AI模型文件
            │   ├── templates/      # 共享模板
            │   └── plugins/        # 插件
            └── users/               # 用户数据隔离目录
                └── {user_id}/      # 用户目录 (默认: "default")
                    ├── projects/   # 项目注册表
                    ├── sessions/   # AI会话数据
                    └── .env        # 用户私密配置
    
    未来多用户扩展:
        users/
        ├── default/                 # 本地默认用户
        ├── user_abc123/             # 登录用户A
        └── user_def456/             # 登录用户B
    """
    
    DEFAULT_USER_ID = "default"
    
    def __init__(self, app_dir: str, base_dir: str, data_dir: str = None, user_id: str = None):
        self.app_dir = app_dir
        self.base_dir = base_dir
        # 使用传入的 data_dir 作为根数据目录，如果没有则使用默认位置
        self.data_root = data_dir if data_dir else os.path.join(base_dir, "data")
        # 用户ID，未来支持多用户登录切换
        self.user_id = user_id or self.DEFAULT_USER_ID
        # 公共数据目录: data/public/
        self.public_dir = os.path.join(self.data_root, "public")
        # 实际用户数据目录: data/users/{user_id}/
        self.data_dir = os.path.join(self.data_root, "users", self.user_id)
        self.projects_dir = os.path.join(self.data_dir, "projects")
        self.sessions_dir = os.path.join(self.data_dir, "sessions")
        self.registry_path = os.path.join(self.projects_dir, "registry.json")
        os.makedirs(self.projects_dir, exist_ok=True)
        os.makedirs(self.sessions_dir, exist_ok=True)
        os.makedirs(self.public_dir, exist_ok=True)
        self._registry = self._load_registry()
        self._migrate_old_projects()

    def _get_data_path(self, project_id: str) -> str:
        dp = os.path.join(self.sessions_dir, project_id)
        os.makedirs(dp, exist_ok=True)
        return dp

    def _migrate_old_projects(self):
        """向后兼容：迁移旧版数据到新的用户隔离架构"""
        # 迁移路径1: ~/.yunji/projects/ → {data_dir}/default/projects/
        old_yunji_projects = os.path.join(os.path.expanduser("~"), ".yunji", "projects")
        if os.path.exists(old_yunji_projects) and not os.path.exists(self.projects_dir):
            try:
                import shutil
                shutil.copytree(old_yunji_projects, self.projects_dir)
            except Exception:
                pass
        
        # 迁移路径2: 旧版扁平结构 {data_dir}/projects/ → {data_dir}/default/projects/
        old_flat_projects = os.path.join(self.data_root, "projects")
        if os.path.exists(old_flat_projects) and old_flat_projects != self.projects_dir:
            if not os.path.exists(self.projects_dir):
                try:
                    import shutil
                    shutil.copytree(old_flat_projects, self.projects_dir)
                except Exception:
                    pass
        
        # 迁移路径3: 旧版扁平结构 {data_dir}/sessions/ → {data_dir}/default/sessions/
        old_flat_sessions = os.path.join(self.data_root, "sessions")
        if os.path.exists(old_flat_sessions) and old_flat_sessions != self.sessions_dir:
            if not os.path.exists(self.sessions_dir):
                try:
                    import shutil
                    shutil.copytree(old_flat_sessions, self.sessions_dir)
                except Exception:
                    pass
        
        # 迁移 registry 中的项目数据路径
        for pid, info in self._registry.get("projects", {}).items():
            if "data_path" not in info:
                dp = self._get_data_path(pid)
                old_path = info.get("path", "")
                for sub in ["conversations", "memories"]:
                    old_sub = os.path.join(old_path, sub)
                    new_sub = os.path.join(dp, sub)
                    if os.path.exists(old_sub) and not os.path.exists(new_sub):
                        try:
                            import shutil
                            shutil.copytree(old_sub, new_sub)
                        except Exception:
                            pass
                old_claude = os.path.join(old_path, "CLAUDE.md")
                new_claude = os.path.join(dp, "CLAUDE.md")
                if os.path.exists(old_claude) and not os.path.exists(new_claude):
                    try:
                        import shutil
                        shutil.copy2(old_claude, new_claude)
                    except Exception:
                        pass
                old_mem_dir = os.path.join(old_path, ".claude", "memories")
                new_mem_dir = os.path.join(dp, "memories")
                if os.path.exists(old_mem_dir) and not os.path.exists(new_mem_dir):
                    try:
                        import shutil
                        shutil.copytree(old_mem_dir, new_mem_dir)
                    except Exception:
                        pass
                old_pj = os.path.join(old_path, "project.json")
                new_pj = os.path.join(dp, "project.json")
                if os.path.exists(old_pj) and not os.path.exists(new_pj):
                    try:
                        import shutil
                        shutil.copy2(old_pj, new_pj)
                    except Exception:
                        pass
                info["data_path"] = dp
                if not info.get("workspace_path"):
                    info["workspace_path"] = old_path
        self._save_registry()

    def get_default_path(self, name: str) -> str:
        safe_name = "".join(c for c in name if c not in r'\/:*?"<>|').strip()
        if not safe_name:
            safe_name = "project"
        candidate = os.path.join(self.projects_dir, safe_name)
        suffix = 1
        while os.path.exists(candidate):
            candidate = os.path.join(self.projects_dir, f"{safe_name}_{suffix}")
            suffix += 1
        return candidate

    def _load_registry(self) -> dict:
        if os.path.exists(self.registry_path):
            try:
                with open(self.registry_path, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                pass
        return {"projects": {}, "active_project": None}

    def _save_registry(self):
        with open(self.registry_path, "w", encoding="utf-8") as f:
            json.dump(self._registry, f, ensure_ascii=False, indent=2)

    def list_projects(self) -> list:
        result = []
        for pid, info in self._registry.get("projects", {}).items():
            result.append({"id": pid, **info})
        return result

    def get_active_project(self) -> dict:
        pid = self._registry.get("active_project")
        if pid and pid in self._registry.get("projects", {}):
            return {"id": pid, **self._registry["projects"][pid]}
        return None

    def create_project(self, name: str, workspace_path: str = "") -> dict:
        pid = _uuid()
        data_path = self._get_data_path(pid)
        conv_dir = os.path.join(data_path, "conversations")
        os.makedirs(conv_dir, exist_ok=True)
        mem_dir = os.path.join(data_path, "memories")
        os.makedirs(mem_dir, exist_ok=True)
        ws = workspace_path.strip() if workspace_path else ""
        if ws:
            os.makedirs(ws, exist_ok=True)
        info = {
            "name": name,
            "data_path": data_path,
            "workspace_path": ws,
            "created_at": datetime.now().isoformat(),
            "updated_at": datetime.now().isoformat(),
        }
        self._registry["projects"][pid] = info
        self._registry["active_project"] = pid
        self._save_registry()
        with open(os.path.join(data_path, "project.json"), "w", encoding="utf-8") as f:
            json.dump({"id": pid, **info}, f, ensure_ascii=False, indent=2)
        return {"id": pid, **info}

    def switch_project(self, project_id: str) -> dict:
        if project_id not in self._registry.get("projects", {}):
            return None
        self._registry["active_project"] = project_id
        self._registry["projects"][project_id]["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        return {"id": project_id, **self._registry["projects"][project_id]}

    def rename_project(self, project_id: str, new_name: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        self._registry["projects"][project_id]["name"] = new_name
        self._registry["projects"][project_id]["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        dp = self._get_data_path(project_id)
        meta_path = os.path.join(dp, "project.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"id": project_id, **self._registry["projects"][project_id]}, f, ensure_ascii=False, indent=2)
        return True

    def update_project(self, project_id: str, new_name: str = None, new_workspace_path: str = None) -> dict:
        if project_id not in self._registry.get("projects", {}):
            return None
        info = self._registry["projects"][project_id]
        if new_name is not None and new_name.strip():
            info["name"] = new_name.strip()
        if new_workspace_path is not None:
            ws = new_workspace_path.strip()
            if ws:
                os.makedirs(ws, exist_ok=True)
            info["workspace_path"] = ws
        info["updated_at"] = datetime.now().isoformat()
        self._save_registry()
        dp = self._get_data_path(project_id)
        meta_path = os.path.join(dp, "project.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"id": project_id, **info}, f, ensure_ascii=False, indent=2)
        return {"id": project_id, **info}

    def delete_project(self, project_id: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        info = self._registry["projects"].pop(project_id)
        if self._registry.get("active_project") == project_id:
            self._registry["active_project"] = None
        self._save_registry()
        return True

    def save_conversation(self, project_id: str, session_id: str, messages: list, title: str = ""):
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        os.makedirs(conv_dir, exist_ok=True)
        conv_path = os.path.join(conv_dir, f"{session_id}.json")
        existing_data = {}
        if os.path.exists(conv_path):
            try:
                with open(conv_path, "r", encoding="utf-8") as f:
                    existing_data = json.load(f)
            except Exception:
                pass
        if not title:
            for m in messages:
                if m.get("role") == "user" and m.get("text", "").strip():
                    title = m["text"].strip()[:50]
                    break
        if not title:
            title = existing_data.get("title", "")
        data = {
            "session_id": session_id,
            "project_id": project_id,
            "title": title,
            "messages": messages,
            "created_at": existing_data.get("created_at", datetime.now().isoformat()),
            "updated_at": datetime.now().isoformat(),
        }
        with open(conv_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        return True

    def load_conversation(self, project_id: str, session_id: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if not os.path.exists(conv_path):
            return []
        try:
            with open(conv_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            return data.get("messages", [])
        except Exception:
            return []

    def list_conversations(self, project_id: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        if not os.path.exists(conv_dir):
            return []
        result = []
        for fname in os.listdir(conv_dir):
            if fname.endswith(".json"):
                try:
                    with open(os.path.join(conv_dir, fname), "r", encoding="utf-8") as f:
                        data = json.load(f)
                    msgs = data.get("messages", [])
                    title = data.get("title", "")
                    if not title:
                        for m in msgs:
                            if m.get("role") == "user" and m.get("text", "").strip():
                                title = m["text"].strip()[:50]
                                break
                    result.append({
                        "session_id": data.get("session_id", fname[:-5]),
                        "title": title,
                        "updated_at": data.get("updated_at", ""),
                        "created_at": data.get("created_at", ""),
                        "message_count": len(msgs),
                    })
                except Exception:
                    pass
        result.sort(key=lambda c: c.get("updated_at", ""), reverse=True)
        return result

    def list_all_conversations(self) -> list:
        """2026-06-16 新增：列出所有项目下的对话历史（用于历史页面）

        返回每条记录都额外带 project_id / project_name 字段，
        方便在历史页面里按项目分组显示。
        """
        result = []
        for pid, info in self._registry.get("projects", {}).items():
            project_name = info.get("name", "未命名项目")
            for c in self.list_conversations(pid):
                c["project_id"] = pid
                c["project_name"] = project_name
                result.append(c)
        # 整体按 updated_at 倒序
        result.sort(key=lambda c: c.get("updated_at", ""), reverse=True)
        return result

    def copy_conversation(self, source_project_id: str, session_id: str, target_project_id: str) -> bool:
        if source_project_id not in self._registry.get("projects", {}):
            return False
        if target_project_id not in self._registry.get("projects", {}):
            return False
        src_dp = self._get_data_path(source_project_id)
        dst_dp = self._get_data_path(target_project_id)
        src_file = os.path.join(src_dp, "conversations", f"{session_id}.json")
        if not os.path.exists(src_file):
            return False
        dst_dir = os.path.join(dst_dp, "conversations")
        os.makedirs(dst_dir, exist_ok=True)
        try:
            with open(src_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            data["project_id"] = target_project_id
            data["updated_at"] = datetime.now().isoformat()
            with open(os.path.join(dst_dir, f"{session_id}.json"), "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def get_project_context(self, project_id: str, max_conversations: int = 3) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        convs = self.list_conversations(project_id)
        context = []
        for c in convs[:max_conversations]:
            msgs = self.load_conversation(project_id, c["session_id"])
            if msgs:
                context.extend(msgs)
        return context

    def search_conversations(self, project_id: str, keyword: str) -> list:
        if project_id not in self._registry.get("projects", {}):
            return []
        if not keyword.strip():
            return self.list_conversations(project_id)
        dp = self._get_data_path(project_id)
        conv_dir = os.path.join(dp, "conversations")
        if not os.path.exists(conv_dir):
            return []
        kw = keyword.strip().lower()
        result = []
        for fname in os.listdir(conv_dir):
            if not fname.endswith(".json"):
                continue
            try:
                with open(os.path.join(conv_dir, fname), "r", encoding="utf-8") as f:
                    data = json.load(f)
                msgs = data.get("messages", [])
                title = data.get("title", "")
                matched = False
                snippet = ""
                if kw in title.lower():
                    matched = True
                for m in msgs:
                    text = m.get("text", "").lower()
                    if kw in text:
                        matched = True
                        if not snippet:
                            idx = text.find(kw)
                            start = max(0, idx - 20)
                            end = min(len(m.get("text", "")), idx + len(kw) + 20)
                            snippet = "..." + m.get("text", "")[start:end] + "..."
                if matched:
                    result.append({
                        "session_id": data.get("session_id", fname[:-5]),
                        "title": title,
                        "updated_at": data.get("updated_at", ""),
                        "created_at": data.get("created_at", ""),
                        "message_count": len(msgs),
                        "snippet": snippet,
                    })
            except Exception:
                pass
        result.sort(key=lambda c: c.get("updated_at", ""), reverse=True)
        return result

    def delete_conversation(self, project_id: str, session_id: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if os.path.exists(conv_path):
            try:
                os.remove(conv_path)
                return True
            except Exception:
                return False
        return False

    def rename_conversation(self, project_id: str, session_id: str, new_title: str) -> bool:
        if project_id not in self._registry.get("projects", {}):
            return False
        dp = self._get_data_path(project_id)
        conv_path = os.path.join(dp, "conversations", f"{session_id}.json")
        if not os.path.exists(conv_path):
            return False
        try:
            with open(conv_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            data["title"] = new_title.strip()
            data["updated_at"] = datetime.now().isoformat()
            with open(conv_path, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def get_claude_md(self, project_id: str) -> str:
        parts = []
        dp = self._get_data_path(project_id)
        data_claude = os.path.join(dp, "CLAUDE.md")
        if os.path.exists(data_claude):
            try:
                with open(data_claude, "r", encoding="utf-8") as f:
                    parts.append(f.read())
            except Exception:
                pass
        info = self._registry.get("projects", {}).get(project_id, {})
        ws = info.get("workspace_path", "")
        if ws:
            for candidate in [os.path.join(ws, "CLAUDE.md"), os.path.join(ws, ".claude", "CLAUDE.md")]:
                if os.path.exists(candidate):
                    try:
                        with open(candidate, "r", encoding="utf-8") as f:
                            content = f.read()
                        if content not in parts:
                            parts.append(content)
                    except Exception:
                        pass
                    break
        return "\n\n".join(parts) if parts else ""

    def save_claude_md(self, project_id: str, content: str) -> bool:
        dp = self._get_data_path(project_id)
        target = os.path.join(dp, "CLAUDE.md")
        try:
            with open(target, "w", encoding="utf-8") as f:
                f.write(content)
            return True
        except Exception:
            return False

    def get_global_claude_md(self) -> str:
        home = os.path.expanduser("~")
        path = os.path.join(home, ".claude", "CLAUDE.md")
        if os.path.exists(path):
            try:
                with open(path, "r", encoding="utf-8") as f:
                    return f.read()
            except Exception:
                pass
        return ""

    def save_global_claude_md(self, content: str) -> bool:
        home = os.path.expanduser("~")
        claude_dir = os.path.join(home, ".claude")
        os.makedirs(claude_dir, exist_ok=True)
        path = os.path.join(claude_dir, "CLAUDE.md")
        try:
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            return True
        except Exception:
            return False

    def list_memories(self, project_id: str) -> list:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        if not os.path.exists(mem_dir):
            return []
        result = []
        for fname in os.listdir(mem_dir):
            if not fname.endswith(".md"):
                continue
            fpath = os.path.join(mem_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    content = f.read()
                mem_type = "project"
                title = fname[:-3]
                if content.startswith("---"):
                    end = content.find("---", 3)
                    if end > 0:
                        header = content[3:end].strip()
                        for line in header.split("\n"):
                            if line.startswith("type:"):
                                mem_type = line.split(":", 1)[1].strip().strip('"').strip("'")
                            elif line.startswith("title:"):
                                title = line.split(":", 1)[1].strip().strip('"').strip("'")
                result.append({
                    "filename": fname,
                    "title": title,
                    "type": mem_type,
                    "content": content,
                    "updated_at": os.path.getmtime(fpath),
                })
            except Exception:
                pass
        result.sort(key=lambda m: m.get("updated_at", 0), reverse=True)
        return result

    def save_memory(self, project_id: str, filename: str, content: str, mem_type: str = "project") -> bool:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        os.makedirs(mem_dir, exist_ok=True)
        if not filename.endswith(".md"):
            filename += ".md"
        fpath = os.path.join(mem_dir, filename)
        try:
            header = f"---\ntype: {mem_type}\ntitle: {filename[:-3]}\nupdated: {datetime.now().isoformat()}\n---\n\n"
            if content.startswith("---"):
                end = content.find("---", 3)
                if end > 0:
                    header = ""
            with open(fpath, "w", encoding="utf-8") as f:
                f.write(header + content if header else content)
            return True
        except Exception:
            return False

    def delete_memory(self, project_id: str, filename: str) -> bool:
        dp = self._get_data_path(project_id)
        fpath = os.path.join(dp, "memories", filename)
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return True
            except Exception:
                return False
        return False

    def search_memories(self, project_id: str, keyword: str) -> list:
        all_mems = self.list_memories(project_id)
        if not keyword.strip():
            return all_mems
        kw = keyword.strip().lower()
        scored = []
        for m in all_mems:
            score = 0
            title = m.get("title", "").lower()
            content = m.get("content", "").lower()
            if kw in title:
                score += 10
            if kw in content:
                score += 5
                idx = content.find(kw)
                m["match_snippet"] = m.get("content", "")[max(0, idx - 30):idx + len(kw) + 30]
            if score > 0:
                m["relevance"] = score
                scored.append(m)
        scored.sort(key=lambda x: x.get("relevance", 0), reverse=True)
        return scored

    def auto_extract_memories(self, project_id: str, messages: list) -> list:
        extracted = []
        user_prefs = []
        feedback_items = []
        project_facts = []
        reference_items = []
        pref_patterns = [
            (r"我喜欢", "用户偏好"), (r"我偏好", "用户偏好"), (r"我习惯", "用户偏好"),
            (r"请用", "用户偏好"), (r"我喜欢用", "用户偏好"), (r"我更喜欢", "用户偏好"),
            (r"我通常", "用户偏好"), (r"我的风格", "用户偏好"), (r"我倾向于", "用户偏好"),
            (r"我一般", "用户偏好"), (r"我常用", "用户偏好"), (r"我习惯用", "用户偏好"),
            (r"我比较喜欢", "用户偏好"), (r"我偏好使用", "用户偏好"),
        ]
        feedback_patterns = [
            (r"不要", "反馈纠正"), (r"别这样", "反馈纠正"), (r"不对", "反馈纠正"),
            (r"错了", "反馈纠正"), (r"不是这样的", "反馈纠正"), (r"应该", "反馈纠正"),
            (r"改一下", "反馈纠正"), (r"换个方式", "反馈纠正"), (r"不行", "反馈纠正"),
            (r"这不对", "反馈纠正"), (r"重新来", "反馈纠正"), (r"不对劲", "反馈纠正"),
            (r"有问题", "反馈纠正"), (r"需要修改", "反馈纠正"), (r"请修正", "反馈纠正"),
        ]
        fact_patterns = [
            (r"项目是", "项目上下文"), (r"用的是", "项目上下文"), (r"技术栈是", "项目上下文"),
            (r"框架是", "项目上下文"), (r"基于", "项目上下文"), (r"运行在", "项目上下文"),
            (r"版本是", "项目上下文"), (r"数据库是", "项目上下文"), (r"语言是", "项目上下文"),
            (r"使用的是", "项目上下文"), (r"开发环境", "项目上下文"), (r"部署在", "项目上下文"),
        ]
        ref_patterns = [
            (r"参考", "外部引用"), (r"文档在", "外部引用"), (r"链接是", "外部引用"),
            (r"官方文档", "外部引用"), (r"API文档", "外部引用"), (r"教程在", "外部引用"),
            (r"仓库是", "外部引用"), (r"源码在", "外部引用"), (r"https?://", "外部引用"),
        ]
        import re
        for m in messages:
            text = m.get("text", "")
            if not text.strip():
                continue
            text_lower = text.lower()
            role = m.get("role", "")
            if role != "user":
                continue
            for pattern, _ in pref_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in user_prefs:
                        user_prefs.append(snippet)
                    break
            for pattern, _ in feedback_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in feedback_items:
                        feedback_items.append(snippet)
                    break
            for pattern, _ in fact_patterns:
                match = re.search(pattern, text)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 60)
                    snippet = text[start:end].strip()
                    if snippet not in project_facts:
                        project_facts.append(snippet)
                    break
            for pattern, _ in ref_patterns:
                match = re.search(pattern, text, re.IGNORECASE)
                if match:
                    start = max(0, match.start() - 15)
                    end = min(len(text), match.end() + 80)
                    snippet = text[start:end].strip()
                    if snippet not in reference_items:
                        reference_items.append(snippet)
                    break
        if user_prefs:
            content = self._merge_memory_content(project_id, "user-preferences.md", user_prefs)
            self.save_memory(project_id, "user-preferences.md", content, "user")
            extracted.append({"type": "user", "count": len(user_prefs)})
        if feedback_items:
            content = self._merge_memory_content(project_id, "user-feedback.md", feedback_items)
            self.save_memory(project_id, "user-feedback.md", content, "feedback")
            extracted.append({"type": "feedback", "count": len(feedback_items)})
        if project_facts:
            content = self._merge_memory_content(project_id, "project-context.md", project_facts)
            self.save_memory(project_id, "project-context.md", content, "project")
            extracted.append({"type": "project", "count": len(project_facts)})
        if reference_items:
            content = self._merge_memory_content(project_id, "external-references.md", reference_items)
            self.save_memory(project_id, "external-references.md", content, "reference")
            extracted.append({"type": "reference", "count": len(reference_items)})
        return extracted

    def _merge_memory_content(self, project_id: str, filename: str, new_items: list) -> str:
        dp = self._get_data_path(project_id)
        mem_dir = os.path.join(dp, "memories")
        fpath = os.path.join(mem_dir, filename)
        existing_lines = set()
        if os.path.exists(fpath):
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    old_content = f.read()
                body = old_content
                if body.startswith("---"):
                    end = body.find("---", 3)
                    if end > 0:
                        body = body[end + 3:].strip()
                for line in body.split("\n"):
                    stripped = line.strip()
                    if stripped.startswith("- "):
                        existing_lines.add(stripped[2:].strip().lower())
            except Exception:
                pass
        merged = []
        for item in new_items:
            if item.lower() not in existing_lines:
                merged.append(item)
        if not merged:
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    return f.read()
            except Exception:
                pass
        all_items = list(existing_lines) + [m.lower() for m in merged]
        content_lines = []
        for item in new_items:
            content_lines.append(f"- {item}")
        if os.path.exists(fpath):
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    old_content = f.read()
                body = old_content
                if body.startswith("---"):
                    end = body.find("---", 3)
                    if end > 0:
                        body = body[end + 3:].strip()
                for line in body.split("\n"):
                    stripped = line.strip()
                    if stripped.startswith("- "):
                        item_text = stripped[2:].strip()
                        if item_text.lower() not in [m.lower() for m in merged]:
                            content_lines.append(stripped)
            except Exception:
                pass
        return "\n".join(content_lines)

    def get_memory_stats(self, project_id: str) -> dict:
        mems = self.list_memories(project_id)
        stats = {"total": len(mems), "by_type": {}, "total_size": 0}
        for m in mems:
            t = m.get("type", "project")
            stats["by_type"][t] = stats["by_type"].get(t, 0) + 1
            stats["total_size"] += len(m.get("content", ""))
        return stats

    def get_relevant_memories(self, project_id: str, query: str, limit: int = 5) -> list:
        results = self.search_memories(project_id, query)
        return results[:limit]

    def _get_templates_dir(self) -> str:
        """获取模板目录，公共数据"""
        return os.path.join(self.public_dir, "templates")

    def save_custom_template(self, name: str, category: str, desc: str, prompt: str, files: str = "") -> bool:
        tpl_dir = self._get_templates_dir()
        os.makedirs(tpl_dir, exist_ok=True)
        safe_name = name.replace(" ", "-").lower()
        fpath = os.path.join(tpl_dir, f"{safe_name}.json")
        tpl = {
            "id": safe_name,
            "name": name,
            "desc": desc,
            "category": category,
            "prompt": prompt,
            "files": files,
            "custom": True,
            "created_at": datetime.now().isoformat(),
        }
        try:
            with open(fpath, "w", encoding="utf-8") as f:
                json.dump(tpl, f, ensure_ascii=False, indent=2)
            return True
        except Exception:
            return False

    def list_custom_templates(self) -> list:
        tpl_dir = self._get_templates_dir()
        if not os.path.exists(tpl_dir):
            return []
        result = []
        for fname in os.listdir(tpl_dir):
            if not fname.endswith(".json"):
                continue
            fpath = os.path.join(tpl_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    tpl = json.load(f)
                tpl["custom"] = True
                result.append(tpl)
            except Exception:
                pass
        return result

    def delete_custom_template(self, template_id: str) -> bool:
        tpl_dir = self._get_templates_dir()
        fpath = os.path.join(tpl_dir, f"{template_id}.json")
        if os.path.exists(fpath):
            try:
                os.remove(fpath)
                return True
            except Exception:
                return False
        return False

    def create_project_from_template(self, project_id: str, template_id: str) -> dict:
        builtin = {
            "react-app": {"files": {"package.json": '{"name": "react-app", "version": "0.1.0", "scripts": {"dev": "vite", "build": "vite build"}}', "src/App.tsx": 'export default function App() { return <div>Hello React</div>; }', "vite.config.ts": 'import { defineConfig } from "vite";\nimport react from "@vitejs/plugin-react";\nexport default defineConfig({ plugins: [react()] });'}, "init_cmd": "npm install"},
            "vue-app": {"files": {"package.json": '{"name": "vue-app", "version": "0.1.0", "scripts": {"dev": "vite", "build": "vite build"}}', "src/App.vue": '<template><div>Hello Vue</div></template>', "vite.config.ts": 'import { defineConfig } from "vite";\nimport vue from "@vitejs/plugin-vue";\nexport default defineConfig({ plugins: [vue()] });'}, "init_cmd": "npm install"},
            "flask-api": {"files": {"requirements.txt": "flask\nflask-sqlalchemy", "app.py": 'from flask import Flask\napp = Flask(__name__)\n\n@app.route("/")\ndef hello():\n    return "Hello Flask"\n\nif __name__ == "__main__":\n    app.run(debug=True)'}, "init_cmd": "pip install -r requirements.txt"},
            "fastapi-app": {"files": {"requirements.txt": "fastapi\nuvicorn", "main.py": 'from fastapi import FastAPI\napp = FastAPI()\n\n@app.get("/")\ndef hello():\n    return {"msg": "Hello FastAPI"}'}, "init_cmd": "pip install -r requirements.txt"},
            "static-site": {"files": {"index.html": '<!DOCTYPE html>\n<html><head><title>My Site</title></head><body><h1>Hello World</h1></body></html>', "style.css": "body { font-family: sans-serif; }", "script.js": 'console.log("Hello");'}},
            "landing-page": {"files": {"index.html": '<!DOCTYPE html>\n<html><head><title>Landing Page</title><link rel="stylesheet" href="style.css"></head><body><header><h1>Welcome</h1></header><main><section class="hero"><h2>Our Product</h2><p>Description here</p></section></main></body></html>', "style.css": "* { margin: 0; padding: 0; box-sizing: border-box; }\nbody { font-family: sans-serif; }\n.hero { padding: 80px 20px; text-align: center; }"}},
        }
        tpl_dir = self._get_templates_dir()
        custom_fpath = os.path.join(tpl_dir, f"{template_id}.json")
        tpl_data = None
        if os.path.exists(custom_fpath):
            try:
                with open(custom_fpath, "r", encoding="utf-8") as f:
                    tpl_data = json.load(f)
            except Exception:
                pass
        if not tpl_data and template_id in builtin:
            tpl_data = builtin[template_id]
        if not tpl_data:
            return {"success": False, "error": f"模板 {template_id} 不存在"}
        info = self._registry.get("projects", {}).get(project_id, {})
        ws = info.get("workspace_path", "")
        if not ws:
            dp = self._get_data_path(project_id)
            ws = dp
        os.makedirs(ws, exist_ok=True)
        files_created = []
        tpl_files = tpl_data.get("files", {})
        if isinstance(tpl_files, str):
            try:
                tpl_files = json.loads(tpl_files)
            except Exception:
                tpl_files = {}
        for rel_path, content in tpl_files.items():
            fpath = os.path.join(ws, rel_path)
            os.makedirs(os.path.dirname(fpath), exist_ok=True)
            try:
                with open(fpath, "w", encoding="utf-8") as f:
                    f.write(content)
                files_created.append(rel_path)
            except Exception:
                pass
        init_cmd = tpl_data.get("init_cmd", "")
        return {"success": True, "files": files_created, "init_cmd": init_cmd}

# ── Git 仓库配置 ──
GIT_REMOTE = "https://github.com/yunjii-cn/code.git"
GIT_BRANCH = "main"
GITEE_OWNER = "yunjii-cn"
GITEE_REPO = "code"
GITEE_API_BASE = f"https://api.github.com/repos/{GITEE_OWNER}/{GITEE_REPO}"
REMOTE_VERSIONS_URL = f"https://raw.githubusercontent.com/{GITEE_OWNER}/{GITEE_REPO}/{GIT_BRANCH}/dev/ver/version.json"


# ── 软件更新器 ──
class SoftwareUpdater:
    """软件更新器 - 支持 Git 命令和 Gitee API 双通道"""

    def __init__(self, dev_dir: str, log_func=None, progress_func=None):
        self.dev_dir = dev_dir
        self.app_dir = os.path.join(dev_dir, "app")
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

    def get_git_history(self, limit=20):
        """获取Git提交历史记录"""
        if not self.is_git_repo():
            return []
        r = self._run_git("log", f"-{limit}", "--oneline", "--format=%h|%s|%an|%ar", timeout=30)
        if not r["ok"]:
            return []
        commits = []
        for line in r["stdout"].splitlines():
            parts = line.strip().split("|", 3)
            if len(parts) >= 4:
                commits.append({
                    "hash": parts[0],
                    "message": parts[1],
                    "author": parts[2],
                    "time": parts[3],
                })
        return commits

    def switch_git_commit(self, commit_hash: str):
        """切换到指定Git提交（资源包版本）"""
        if not self.is_git_repo():
            self.log("[错误] 不是 Git 仓库，无法切换版本", "#F44336")
            return False
        
        self.log(f"正在切换到 commit {commit_hash}...", "#FF9800")
        r = self._run_git("stash")
        stashed = r["ok"] and "Saved" in r["stdout"]
        
        r = self._run_git("checkout", commit_hash, timeout=60)
        if not r["ok"]:
            self.log(f"[错误] 切换失败: {r['stderr'][:200]}", "#F44336")
            if stashed:
                self._run_git("stash", "pop")
            return False
        
        if stashed:
            self._run_git("stash", "pop")
        
        self.log(f"✓ 已切换到 commit {commit_hash}", "#4CAF50")
        return True

    def switch_to_exe(self, exe_path: str, git_commit: str = ""):
        """切换到指定 EXE 并重启，同时回滚代码到对应 git commit
        
        实现方式：
        1. ver/ 目录存储所有历史版本 EXE
        2. dev/ 目录只保留当前使用的 EXE（通过复制覆盖实现单一版本）
        """
        if not os.path.exists(exe_path):
            self.log(f"[错误] EXE 不存在: {exe_path}", "#F44336")
            return False

        # 实现单一 EXE 版本管理：从 ver/ 复制到 dev/ 根目录
        exe_filename = os.path.basename(exe_path)
        dev_exe_path = os.path.join(self.dev_dir, exe_filename)
        
        try:
            import shutil
            shutil.copy2(exe_path, dev_exe_path)
            self.log(f"✓ 已将版本 {exe_filename} 复制到根目录", "#4CAF50")
        except Exception as e:
            self.log(f"[警告] 复制 EXE 失败: {e}", "#FF9800")
            # 即使复制失败，也尝试直接启动原来的exe
            dev_exe_path = exe_path

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
        new_exe = dev_exe_path
        cmd = f'ping -n 3 127.0.0.1 >nul & start "" "{new_exe}"'
        subprocess.Popen(cmd, shell=True, creationflags=subprocess.CREATE_NO_WINDOW)

        self.log(f"正在切换到 {exe_filename}...", "#4CAF50")

        QApplication.quit()
        return True

    def _get_gitee_token(self):
        token = ""
        env_path = os.path.join(self.dev_dir, "data", ".env")
        if not os.path.exists(env_path):
            env_path = os.path.join(self.app_dir, ".env")
        if os.path.exists(env_path):
            try:
                with open(env_path, "r", encoding="utf-8") as f:
                    for line in f:
                        line = line.strip()
                        for key in ("GITHUB_TOKEN", "GITEE_TOKEN"):
                            if line.startswith(f"{key}="):
                                val = line.split("=", 1)[1].strip()
                                if val:
                                    return val
            except Exception:
                pass
        return token

    def fetch_remote_version_history(self):
        """获取远程版本历史 - 三通道：Gitee API → Git命令 → HTTP直链"""
        result = self._fetch_remote_via_gitee_api()
        if result is not None:
            return result
        result = self._fetch_remote_via_git()
        if result is not None:
            return result
        return self._fetch_remote_via_http()

    def _fetch_remote_via_gitee_api(self):
        """通过 GitHub API 获取远程版本历史（无需安装git，支持私有仓库）"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            import base64
            token = self._get_gitee_token()
            for path in ["dev/ver/version.json", "app/version_history.json"]:
                try:
                    url = f"{GITEE_API_BASE}/contents/{path}?ref={GIT_BRANCH}"
                    req = Request(url)
                    req.add_header('User-Agent', 'Mozilla/5.0')
                    if token:
                        req.add_header('Authorization', f'token {token}')
                    resp = urlopen(req, timeout=15)
                    data = json.loads(resp.read().decode())
                    if isinstance(data, list):
                        continue
                    content_b64 = data.get("content", "")
                    if not content_b64:
                        continue
                    content = base64.b64decode(content_b64).decode('utf-8')
                    version_data = json.loads(content)
                    if isinstance(version_data, dict) and "versions" in version_data:
                        remote_versions = version_data.get("versions", [])
                    elif isinstance(version_data, list):
                        remote_versions = version_data
                    else:
                        continue
                    result = []
                    for v in remote_versions:
                        entry = dict(v)
                        if "date" in entry and "build_time" not in entry:
                            entry["build_time"] = entry["date"]
                        result.append(entry)
                    result.sort(key=lambda x: x.get("version", ""), reverse=True)
                    return result
                except (HTTPError, URLError, Exception):
                    continue
            return None
        except Exception as e:
            print(f"GitHub API 获取失败: {e}")
            return None

    def _fetch_remote_via_git(self):
        """通过 git 命令获取远程版本历史"""
        try:
            if not self.is_git_repo():
                return None
            r = self._run_git("fetch", "origin", timeout=30)
            if not r["ok"]:
                return None
            show_result = subprocess.run(
                ["git", "show", f"origin/{GIT_BRANCH}:dev/ver/version.json"],
                capture_output=True, text=True, cwd=self.dev_dir, timeout=10,
                startupinfo=self._si(),
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            if show_result.returncode != 0:
                return None
            data = json.loads(show_result.stdout)
            remote_versions = data.get("versions", [])
            result = []
            for v in remote_versions:
                entry = dict(v)
                if "date" in entry and "build_time" not in entry:
                    entry["build_time"] = entry["date"]
                result.append(entry)
            result.sort(key=lambda x: x.get("version", ""), reverse=True)
            return result
        except Exception:
            return None

    def _fetch_remote_via_http(self):
        """HTTP 直链方式获取远程版本历史（仅公开仓库可用）"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            req = Request(REMOTE_VERSIONS_URL)
            req.add_header('User-Agent', 'Mozilla/5.0')
            resp = urlopen(req, timeout=10)
            content = resp.read().decode('utf-8')
            data = json.loads(content)
            if isinstance(data, dict) and "versions" in data:
                result = []
                for v in data["versions"]:
                    entry = dict(v)
                    if "date" in entry and "build_time" not in entry:
                        entry["build_time"] = entry["date"]
                    result.append(entry)
                result.sort(key=lambda x: x.get("version", ""), reverse=True)
                return result
            elif isinstance(data, list):
                return data
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

    def fetch_remote_commits(self, limit=30):
        """获取远程Git提交历史 - 双通道：Gitee API → Git命令"""
        commits = self._fetch_commits_via_gitee_api(limit)
        if commits is not None:
            return commits
        return self.get_git_history(limit)

    def _fetch_commits_via_gitee_api(self, limit=30):
        """通过 GitHub API 获取提交历史"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            token = self._get_gitee_token()
            url = f"{GITEE_API_BASE}/commits?sha={GIT_BRANCH}&per_page={limit}"
            req = Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            if token:
                req.add_header('Authorization', f'token {token}')
            resp = urlopen(req, timeout=15)
            data = json.loads(resp.read().decode())
            commits = []
            for c in data:
                commit = c.get("commit", {})
                commits.append({
                    "hash": c.get("sha", "")[:7],
                    "message": commit.get("message", "").split("\n")[0][:80],
                    "author": commit.get("author", {}).get("name", ""),
                    "time": commit.get("author", {}).get("date", "")[:10],
                })
            return commits
        except Exception as e:
            print(f"GitHub API 提交历史获取失败: {e}")
            return None

    def download_update_via_gitee(self, progress_func=None):
        """通过 GitHub API 下载仓库zip包更新（无需git）"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            import tempfile
            import zipfile
            token = self._get_gitee_token()
            url = f"{GITEE_API_BASE}/zipball/{GIT_BRANCH}"
            self.log("正在从 GitHub 下载更新包...", "#FF9800")
            if progress_func:
                progress_func(10, "正在下载更新包...")
            req = Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            if token:
                req.add_header('Authorization', f'token {token}')
            resp = urlopen(req, timeout=120)
            total_size = int(resp.headers.get('Content-Length', 0))
            downloaded = 0
            chunk_size = 65536
            zip_data = bytearray()
            while True:
                chunk = resp.read(chunk_size)
                if not chunk:
                    break
                zip_data.extend(chunk)
                downloaded += len(chunk)
                if total_size > 0 and progress_func:
                    pct = int(10 + 50 * downloaded / total_size)
                    progress_func(pct, f"正在下载... {downloaded // 1024}KB / {total_size // 1024}KB")
            if not zip_data:
                self.log("[错误] 下载的更新包为空", "#F44336")
                return False
            if progress_func:
                progress_func(65, "正在解压更新包...")
            self.log("正在解压更新包...", "#FF9800")
            temp_dir = os.path.join(self.dev_dir, "temp", "update_tmp")
            if os.path.exists(temp_dir):
                shutil.rmtree(temp_dir, ignore_errors=True)
            os.makedirs(temp_dir, exist_ok=True)
            zip_path = os.path.join(temp_dir, "update.zip")
            with open(zip_path, "wb") as f:
                f.write(zip_data)
            with zipfile.ZipFile(zip_path, 'r') as zf:
                zf.extractall(temp_dir)
            os.remove(zip_path)
            extracted_dirs = [d for d in os.listdir(temp_dir) if os.path.isdir(os.path.join(temp_dir, d))]
            if not extracted_dirs:
                self.log("[错误] 解压后未找到内容", "#F44336")
                shutil.rmtree(temp_dir, ignore_errors=True)
                return False
            src_dir = os.path.join(temp_dir, extracted_dirs[0])
            if progress_func:
                progress_func(75, "正在应用更新...")
            self.log("正在应用更新...", "#FF9800")
            protect_patterns = {".env", ".env.local", "projects", "venvs", "api"}
            for item in os.listdir(src_dir):
                src_path = os.path.join(src_dir, item)
                dst_path = os.path.join(self.dev_dir, item)
                if item in protect_patterns:
                    if os.path.isdir(src_path) and os.path.isdir(dst_path):
                        self._merge_protected_dir(src_path, dst_path, protect_patterns)
                    continue
                if os.path.isdir(src_path):
                    if os.path.exists(dst_path):
                        shutil.rmtree(dst_path, ignore_errors=True)
                    shutil.copytree(src_path, dst_path)
                else:
                    if os.path.exists(dst_path):
                        os.remove(dst_path)
                    shutil.copy2(src_path, dst_path)
            shutil.rmtree(temp_dir, ignore_errors=True)
            if progress_func:
                progress_func(100, "更新完成")
            self.log("✓ 更新包已成功应用", "#4CAF50")
            return True
        except Exception as e:
            self.log(f"[错误] 下载更新失败: {e}", "#F44336")
            temp_dir = os.path.join(self.dev_dir, "temp", "update_tmp")
            if os.path.exists(temp_dir):
                shutil.rmtree(temp_dir, ignore_errors=True)
            return False

    def _merge_protected_dir(self, src, dst, protect_patterns):
        """合并受保护目录，不覆盖用户数据"""
        for item in os.listdir(src):
            src_path = os.path.join(src, item)
            dst_path = os.path.join(dst, item)
            if item in protect_patterns:
                if os.path.isdir(src_path) and os.path.isdir(dst_path):
                    self._merge_protected_dir(src_path, dst_path, protect_patterns)
                continue
            if os.path.isdir(src_path):
                if os.path.exists(dst_path):
                    shutil.rmtree(dst_path, ignore_errors=True)
                shutil.copytree(src_path, dst_path)
            else:
                if os.path.exists(dst_path):
                    os.remove(dst_path)
                shutil.copy2(src_path, dst_path)

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


# ── pywebview js_api 桥接对象 (替代 QWebChannel) ──
class BackendBridge:
    """暴露给前端 JS 的 Python 对象，通过 pywebview js_api 暴露"""

    def __init__(self, parent=None):
        self._app_ref = None  # 由 MainWindow 设置

    def _get_main(self):
        return self._app_ref

    def frontendReady(self):
        main = self._get_main()
        if main and hasattr(main, '_finish_splash'):
            QTimer.singleShot(300, main._finish_splash)

    # ── 用户管理 API (多用户登录架构) ──

    def getCurrentUser(self):
        """获取当前用户信息"""
        main = self._get_main()
        if not main:
            return json.dumps({"id": "default", "name": "本地用户"})
        return json.dumps({
            "id": main.current_user_id,
            "name": main.current_user_id,
            "is_default": main.current_user_id == "default"
        })

    def listUsers(self):
        """列出所有用户目录"""
        main = self._get_main()
        if not main:
            return json.dumps([{"id": "default", "name": "本地用户"}])
        users = []
        users_dir = os.path.join(main.data_dir, "users")
        if os.path.exists(users_dir):
            for uid in os.listdir(users_dir):
                user_path = os.path.join(users_dir, uid)
                if os.path.isdir(user_path):
                    users.append({"id": uid, "name": uid})
        if not users:
            users.append({"id": "default", "name": "本地用户"})
        return json.dumps(users)

    def switchUser(self, user_id: str):
        """切换用户 - 重新初始化 ProjectManager 和相关组件"""
        main = self._get_main()
        if not main:
            return False
        try:
            # 保存当前用户状态
            if main.project_mgr:
                main.project_mgr._save_registry()
            
            # 切换用户ID
            main.current_user_id = user_id or "default"
            main.user_dir = os.path.join(main.data_dir, "users", main.current_user_id)
            os.makedirs(main.user_dir, exist_ok=True)
            os.makedirs(os.path.join(main.user_dir, "projects"), exist_ok=True)
            os.makedirs(os.path.join(main.user_dir, "sessions"), exist_ok=True)
            
            # 重新初始化 ProjectManager
            main.project_mgr = ProjectManager(
                main.app_dir, 
                main.exe_dir, 
                data_dir=main.data_dir,
                user_id=main.current_user_id
            )
            
            # 重新加载环境配置
            main.env_manager = EnvFileManager(os.path.join(main.user_dir, ".env"))
            
            # 更新当前项目状态
            main.active_project_id = None
            main.current_workspace = main.app_dir
            active_proj = main.project_mgr.get_active_project()
            if active_proj:
                main.active_project_id = active_proj["id"]
                main.current_workspace = active_proj.get("workspace_path") or main.app_dir
            
            return True
        except Exception as e:
            print(f"切换用户失败: {e}")
            return False

    # ── 项目管理 API ──

    def listProjects(self):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_projects())

    def getActiveProject(self):
        main = self._get_main()
        if not main:
            return json.dumps(None)
        proj = main.project_mgr.get_active_project()
        return json.dumps(proj)

    def createProject(self, name: str, workspace_path: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.create_project(name, workspace_path)
        main.active_project_id = proj["id"]
        main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    def switchProject(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.switch_project(project_id)
        if proj:
            main.active_project_id = proj["id"]
            main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    def renameProject(self, project_id: str, new_name: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.rename_project(project_id, new_name)

    def deleteProject(self, project_id: str):
        main = self._get_main()
        if not main:
            return False
        result = main.project_mgr.delete_project(project_id)
        if result and main.active_project_id == project_id:
            main.active_project_id = None
            main.current_workspace = main.app_dir
        return result

    def updateProject(self, project_id: str, new_name: str, new_workspace_path: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.update_project(project_id, new_name or None, new_workspace_path or None)
        if proj and main.active_project_id == project_id:
            main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    def getDefaultProjectPath(self, name: str):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_default_path(name)

    def listConversations(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_conversations(project_id))

    def listAllConversations(self):
        """2026-06-16 新增：列出所有项目的对话历史（历史页面用）"""
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_all_conversations())

    def loadConversation(self, project_id: str, session_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        msgs = main.project_mgr.load_conversation(project_id, session_id)
        return json.dumps(msgs)

    def copyConversation(self, source_project_id: str, session_id: str, target_project_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.copy_conversation(source_project_id, session_id, target_project_id)

    def getProjectContext(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        ctx = main.project_mgr.get_project_context(project_id)
        return json.dumps(ctx)

    def saveConversation(self, project_id: str, session_id: str, messages_json: str):
        main = self._get_main()
        if not main:
            return False
        try:
            msgs = json.loads(messages_json)
        except Exception:
            msgs = []
        return main.project_mgr.save_conversation(project_id, session_id, msgs)

    def searchConversations(self, project_id: str, keyword: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.search_conversations(project_id, keyword))

    def deleteConversation(self, project_id: str, session_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_conversation(project_id, session_id)

    def renameConversation(self, project_id: str, session_id: str, new_title: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.rename_conversation(project_id, session_id, new_title)

    def getClaudeMd(self, project_id: str):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_claude_md(project_id)

    def saveClaudeMd(self, project_id: str, content: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_claude_md(project_id, content)

    def getGlobalClaudeMd(self):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_global_claude_md()

    def saveGlobalClaudeMd(self, content: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_global_claude_md(content)

    def listMemories(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_memories(project_id))

    def saveMemory(self, project_id: str, filename: str, content: str, mem_type: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_memory(project_id, filename, content, mem_type)

    def deleteMemory(self, project_id: str, filename: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_memory(project_id, filename)

    def searchMemories(self, project_id: str, keyword: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.search_memories(project_id, keyword))

    def autoExtractMemories(self, project_id: str, messages_json: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        try:
            msgs = json.loads(messages_json)
        except Exception:
            msgs = []
        return json.dumps(main.project_mgr.auto_extract_memories(project_id, msgs))

    def getMemoryStats(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"total": 0, "by_type": {}, "total_size": 0})
        return json.dumps(main.project_mgr.get_memory_stats(project_id))

    def getRelevantMemories(self, project_id: str, query: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.get_relevant_memories(project_id, query))

    def listProjectTemplates(self, category: str):
        templates = [
            {"id": "react-app", "name": "React 应用", "desc": "React + TypeScript + Vite", "category": "frontend", "prompt": "创建一个 React + TypeScript + Vite 项目，包含基本路由和状态管理", "scaffold": True},
            {"id": "vue-app", "name": "Vue 应用", "desc": "Vue 3 + TypeScript + Vite", "category": "frontend", "prompt": "创建一个 Vue 3 + TypeScript + Vite 项目，包含 Vue Router 和 Pinia", "scaffold": True},
            {"id": "next-app", "name": "Next.js 应用", "desc": "Next.js + React + TypeScript", "category": "frontend", "prompt": "创建一个 Next.js + TypeScript 项目，包含 App Router 和基本页面", "scaffold": False},
            {"id": "flask-api", "name": "Flask API", "desc": "Flask + SQLAlchemy + SQLite", "category": "backend", "prompt": "创建一个 Flask REST API 项目，包含 SQLAlchemy ORM 和 SQLite 数据库", "scaffold": True},
            {"id": "fastapi-app", "name": "FastAPI 应用", "desc": "FastAPI + SQLAlchemy + Pydantic", "category": "backend", "prompt": "创建一个 FastAPI 项目，包含 SQLAlchemy ORM、Pydantic 模型和自动文档", "scaffold": True},
            {"id": "express-api", "name": "Express API", "desc": "Express + TypeScript + Prisma", "category": "backend", "prompt": "创建一个 Express + TypeScript REST API 项目，包含 Prisma ORM", "scaffold": False},
            {"id": "fullstack-vue", "name": "Vue 全栈", "desc": "Vue 3 + FastAPI + SQLite", "category": "fullstack", "prompt": "创建一个全栈项目：前端 Vue 3 + Vite，后端 FastAPI + SQLite，包含前后端通信", "scaffold": False},
            {"id": "fullstack-react", "name": "React 全栈", "desc": "React + Express + MongoDB", "category": "fullstack", "prompt": "创建一个全栈项目：前端 React + Vite，后端 Express + MongoDB，包含 REST API", "scaffold": False},
            {"id": "static-site", "name": "静态网站", "desc": "HTML + CSS + JavaScript", "category": "frontend", "prompt": "创建一个静态网站项目，包含 HTML5 + CSS3 + 原生 JavaScript，响应式设计", "scaffold": True},
            {"id": "landing-page", "name": "落地页", "desc": "单页营销/宣传网站", "category": "frontend", "prompt": "创建一个精美的单页落地页/宣传网站，包含 Hero、功能展示、定价、联系方式等区块", "scaffold": True},
            {"id": "cli-tool", "name": "CLI 工具", "desc": "Python CLI + Click/Typer", "category": "backend", "prompt": "创建一个 Python CLI 工具项目，使用 Typer 框架，包含命令行参数解析和配置管理", "scaffold": False},
            {"id": "python-scraper", "name": "爬虫项目", "desc": "Python + httpx + BeautifulSoup", "category": "backend", "prompt": "创建一个 Python 爬虫项目，包含 httpx 请求、BeautifulSoup 解析、数据存储", "scaffold": False},
            {"id": "desktop-electron", "name": "Electron 桌面应用", "desc": "Electron + Vue 3 + TypeScript", "category": "desktop", "prompt": "创建一个 Electron 桌面应用项目，使用 Vue 3 + TypeScript，包含主进程和渲染进程", "scaffold": False},
            {"id": "desktop-pyqt", "name": "PyQt 桌面应用", "desc": "PyQt6 + Python", "category": "desktop", "prompt": "创建一个 PyQt6 桌面应用项目，包含主窗口、菜单栏、工具栏和状态栏", "scaffold": False},
            {"id": "data-analysis", "name": "数据分析", "desc": "Python + Pandas + Matplotlib", "category": "datascience", "prompt": "创建一个 Python 数据分析项目，包含 Pandas 数据处理、Matplotlib 可视化和 Jupyter Notebook", "scaffold": False},
            {"id": "ml-project", "name": "机器学习项目", "desc": "Python + scikit-learn", "category": "datascience", "prompt": "创建一个机器学习项目，包含数据预处理、模型训练、评估和预测脚本", "scaffold": False},
        ]
        if category and category != "all":
            templates = [t for t in templates if t["category"] == category]
        main = self._get_main()
        if main:
            custom = main.project_mgr.list_custom_templates()
            if category and category != "all":
                custom = [t for t in custom if t.get("category") == category]
            templates = templates + custom
        return json.dumps(templates)

    def saveCustomTemplate(self, name: str, category: str, desc: str, prompt: str, files: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_custom_template(name, category, desc, prompt, files)

    def deleteCustomTemplate(self, template_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_custom_template(template_id)

    def _get_app_data_path(self) -> str:
        return os.path.join(self.user_dir, "app_data.json")

    def saveAppData(self, data_json: str):
        try:
            data_path = self._get_app_data_path()
            os.makedirs(os.path.dirname(data_path), exist_ok=True)
            with open(data_path, "w", encoding="utf-8") as f:
                f.write(data_json)
            print(f"[saveAppData] OK path={data_path} size={len(data_json)}")
            return json.dumps({"ok": True})
        except Exception as e:
            print(f"[saveAppData] ERROR: {e}")
            return json.dumps({"ok": False, "error": str(e)})

    def loadAppData(self):
        try:
            data_path = self._get_app_data_path()
            if os.path.exists(data_path):
                with open(data_path, "r", encoding="utf-8") as f:
                    content = f.read()
                print(f"[loadAppData] OK path={data_path} size={len(content)}")
                return content
            print(f"[loadAppData] NOT FOUND path={data_path}")
            return ""
        except Exception as e:
            print(f"[loadAppData] ERROR: {e}")
            return ""

    def createProjectFromTemplate(self, project_id: str, template_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"success": False, "error": "main not available"})
        return json.dumps(main.project_mgr.create_project_from_template(project_id, template_id))

    def getVersionHistory(self):
        vh_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "version_history.json")
        if not os.path.exists(vh_path):
            return json.dumps([])
        try:
            with open(vh_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            return json.dumps(data)
        except Exception:
            return json.dumps([])

    def runTerminalCommand(self, cmd: str, cwd: str):
        try:
            if not cwd or not os.path.isdir(cwd):
                cwd = os.path.expanduser("~")
            result = subprocess.run(
                cmd, shell=True, capture_output=True, text=True,
                cwd=cwd, timeout=30, encoding="utf-8", errors="replace"
            )
            output = result.stdout or ""
            if result.stderr:
                output += ("\n" if output else "") + result.stderr
            new_cwd = cwd
            if cmd.strip().startswith("cd "):
                target = cmd.strip()[3:].strip()
                if os.path.isabs(target):
                    new_cwd = target
                else:
                    new_cwd = os.path.normpath(os.path.join(cwd, target))
                if os.path.isdir(new_cwd):
                    pass
                else:
                    new_cwd = cwd
            return json.dumps({"output": output.rstrip(), "cwd": new_cwd, "returncode": result.returncode})
        except subprocess.TimeoutExpired:
            return json.dumps({"output": "命令超时（30秒）", "error": "timeout", "cwd": cwd})
        except Exception as e:
            return json.dumps({"output": str(e), "error": str(e), "cwd": cwd})

    def getGitStatus(self, project_path: str):
        try:
            if not project_path or not os.path.isdir(project_path):
                return json.dumps({"error": "无效路径"})
            result = subprocess.run(
                ["git", "status", "--porcelain=v1", "--branch"],
                capture_output=True, text=True, cwd=project_path,
                timeout=10, encoding="utf-8", errors="replace"
            )
            if result.returncode != 0:
                return json.dumps({"error": result.stderr.strip() or "不是 Git 仓库"})
            lines = result.stdout.strip().split("\n") if result.stdout.strip() else []
            branch = ""
            files = []
            for line in lines:
                if line.startswith("## "):
                    branch = line[3:].strip()
                    if "..." in branch:
                        branch = branch.split("...")[0]
                elif len(line) >= 3:
                    status = line[:2].strip()
                    filepath = line[3:].strip()
                    if filepath.startswith('"') and filepath.endswith('"'):
                        filepath = filepath[1:-1]
                    files.append({"status": status, "path": filepath})
            return json.dumps({"branch": branch, "files": files, "total": len(files)})
        except Exception as e:
            return json.dumps({"error": str(e)})

    def gitCommit(self, project_path: str, message: str):
        try:
            if not project_path or not message.strip():
                return json.dumps({"error": "路径或消息无效"})
            subprocess.run(["git", "add", "-A"], capture_output=True, text=True, cwd=project_path, timeout=10)
            result = subprocess.run(
                ["git", "commit", "-m", message.strip()],
                capture_output=True, text=True, cwd=project_path, timeout=10,
                encoding="utf-8", errors="replace"
            )
            if result.returncode != 0:
                return json.dumps({"error": result.stderr.strip() or "提交失败"})
            return json.dumps({"success": True, "output": result.stdout.strip()})
        except Exception as e:
            return json.dumps({"error": str(e)})

    def getGitLog(self, project_path: str):
        try:
            if not project_path or not os.path.isdir(project_path):
                return json.dumps({"error": "无效路径"})
            result = subprocess.run(
                ["git", "log", "--oneline", "-20", "--format=%h|%s|%an|%ar"],
                capture_output=True, text=True, cwd=project_path,
                timeout=10, encoding="utf-8", errors="replace"
            )
            if result.returncode != 0:
                return json.dumps({"error": result.stderr.strip()})
            commits = []
            for line in result.stdout.strip().split("\n"):
                if "|" in line:
                    parts = line.split("|", 3)
                    commits.append({
                        "hash": parts[0] if len(parts) > 0 else "",
                        "message": parts[1] if len(parts) > 1 else "",
                        "author": parts[2] if len(parts) > 2 else "",
                        "date": parts[3] if len(parts) > 3 else "",
                    })
            return json.dumps({"commits": commits})
        except Exception as e:
            return json.dumps({"error": str(e)})

    def getFileTree(self, project_path: str):
        try:
            if not project_path or not os.path.isdir(project_path):
                return json.dumps([])
            skip_dirs = {".git", "node_modules", "__pycache__", ".venv", "venv", "dist", "build", ".next", ".nuxt", ".cache", ".tox", "target"}
            result = []
            for root, dirs, files in os.walk(project_path):
                dirs[:] = [d for d in dirs if d not in skip_dirs]
                rel = os.path.relpath(root, project_path)
                depth = rel.count(os.sep) if rel != "." else 0
                if depth > 3:
                    dirs.clear()
                    continue
                for d in sorted(dirs):
                    result.append({"name": d, "path": os.path.join(rel, d) if rel != "." else d, "type": "dir"})
                for f in sorted(files)[:50]:
                    result.append({"name": f, "path": os.path.join(rel, f) if rel != "." else f, "type": "file"})
                if len(result) > 200:
                    break
            return json.dumps(result[:200])
        except Exception as e:
            return json.dumps([])

    def showDesktopNotification(self, title: str, body: str):
        try:
            from PyQt6.QtWidgets import QSystemTrayIcon
            main = self._get_main()
            if main and hasattr(main, 'tray_icon') and main.tray_icon:
                main.tray_icon.showMessage(title, body, QSystemTrayIcon.MessageIcon.Information, 3000)
                return True
            return False
        except Exception:
            return False

    def openExternalUrl(self, url: str):
        try:
            import webbrowser
            webbrowser.open(url)
            return True
        except Exception:
            try:
                from PyQt6.QtGui import QDesktopServices
                from PyQt6.QtCore import QUrl
                QDesktopServices.openUrl(QUrl(url))
                return True
            except Exception:
                return False

    def _get_plugins_dir(self) -> str:
        """获取插件目录，公共数据"""
        return os.path.join(self.public_dir, "plugins")

    def listPlugins(self):
        plugin_dir = self._get_plugins_dir()
        os.makedirs(plugin_dir, exist_ok=True)
        result = []
        for fname in os.listdir(plugin_dir):
            if not fname.endswith(".json"):
                continue
            fpath = os.path.join(plugin_dir, fname)
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    meta = json.load(f)
                meta["id"] = fname[:-5]
                plugin_file = os.path.join(plugin_dir, meta.get("id", ""), "index.js")
                if os.path.exists(plugin_file):
                    meta["installed"] = True
                else:
                    meta["installed"] = False
                result.append(meta)
            except Exception:
                pass
        return json.dumps(result)

    def installPlugin(self, plugin_json: str):
        try:
            meta = json.loads(plugin_json)
            plugin_dir = self._get_plugins_dir()
            os.makedirs(plugin_dir, exist_ok=True)
            meta_path = os.path.join(plugin_dir, f"{meta.get('id', 'unknown')}.json")
            with open(meta_path, "w", encoding="utf-8") as f:
                json.dump(meta, f, ensure_ascii=False, indent=2)
            plugin_subdir = os.path.join(plugin_dir, meta.get("id", "unknown"))
            os.makedirs(plugin_subdir, exist_ok=True)
            if meta.get("code"):
                code_path = os.path.join(plugin_subdir, "index.js")
                with open(code_path, "w", encoding="utf-8") as f:
                    f.write(meta["code"])
            return True
        except Exception:
            return False

    def uninstallPlugin(self, plugin_id: str):
        try:
            plugin_dir = self._get_plugins_dir()
            meta_path = os.path.join(plugin_dir, f"{plugin_id}.json")
            if os.path.exists(meta_path):
                os.remove(meta_path)
            plugin_subdir = os.path.join(plugin_dir, plugin_id)
            if os.path.exists(plugin_subdir):
                shutil.rmtree(plugin_subdir, ignore_errors=True)
            return True
        except Exception:
            return False

    def executePlugin(self, plugin_id: str, input_data: str):
        plugin_dir = self._get_plugins_dir()
        code_path = os.path.join(plugin_dir, plugin_id, "index.js")
        if not os.path.exists(code_path):
            return json.dumps({"error": "插件未安装"})
        try:
            with open(code_path, "r", encoding="utf-8") as f:
                code = f.read()
            local_vars = {"input": input_data, "json": json, "os": os, "result": None}
            exec(compile(code, code_path, "exec"), local_vars)
            output = local_vars.get("result", "")
            return json.dumps({"output": str(output) if output is not None else ""})
        except Exception as e:
            return json.dumps({"error": str(e)})

    def startVoiceInput(self, language: str):
        try:
            main = self._get_main()
            if not main:
                return False
            if not hasattr(main, '_voice_thread') or main._voice_thread is None:
                def record_voice():
                    try:
                        import speech_recognition as sr
                        r = sr.Recognizer()
                        with sr.Microphone() as source:
                            r.adjust_for_ambient_noise(source, duration=0.5)
                            audio = r.listen(source, timeout=10, phrase_time_limit=30)
                        text = r.recognize_google(audio, language=language or "zh-CN")
                        main.voice_result_signal.emit(text)
                    except Exception as e:
                        main.voice_result_signal.emit(f"[语音识别失败: {e}]")
                    main._voice_thread = None
                main._voice_thread = threading.Thread(target=record_voice, daemon=True)
                main._voice_thread.start()
                return True
            return False
        except Exception:
            return False

    def speakText(self, text: str):
        try:
            def _speak():
                try:
                    import pyttsx3
                    engine = pyttsx3.init()
                    engine.setProperty('rate', 180)
                    engine.setProperty('volume', 0.9)
                    engine.say(text)
                    engine.runAndWait()
                except Exception:
                    pass
            threading.Thread(target=_speak, daemon=True).start()
            return True
        except Exception:
            return False

    def _get_models_dir(self) -> str:
        """获取模型目录，公共数据 (所有用户共享)"""
        return os.path.join(self.public_dir, "models")

    def getOfflineModels(self):
        # 模型是公共数据，所有用户共享
        models_dir = self._get_models_dir()
        os.makedirs(models_dir, exist_ok=True)
        models = []
        if os.path.exists(models_dir):
            for fname in os.listdir(models_dir):
                if fname.endswith(".gguf") or fname.endswith(".bin"):
                    fpath = os.path.join(models_dir, fname)
                    size_mb = os.path.getsize(fpath) / (1024 * 1024)
                    models.append({"name": fname, "path": fpath, "size_mb": round(size_mb, 1)})
        return json.dumps(models)

    def downloadModel(self, url: str):
        try:
            # 模型是公共数据，所有用户共享
            models_dir = self._get_models_dir()
            os.makedirs(models_dir, exist_ok=True)
            fname = url.split("/")[-1] or "model.gguf"
            fpath = os.path.join(models_dir, fname)
            if os.path.exists(fpath):
                return True

            def _download():
                try:
                    urllib.request.urlretrieve(url, fpath)
                except Exception:
                    pass
            threading.Thread(target=_download, daemon=True).start()
            return True
        except Exception:
            return False

        # ── pywebview JS通知辅助方法 ──
    def _notify_delta(self, data: str):
        main = self._get_main()
        # QtWebView2: 优先使用 _vue2_widget.evaluate_js
        if main and hasattr(main, '_vue2_widget') and main._vue2_widget:
            try:
                main._vue2_widget.evaluate_js(
                    "if(window.onDelta) window.onDelta(" + data + ");"
                )
            except Exception:
                pass
        # pywebview: 回退到 _webview_window.evaluate_js
        elif main and hasattr(main, '_webview_window') and main._webview_window:
            try:
                main._webview_window.evaluate_js(
                    "if(window.onDelta) window.onDelta(" + data + ");"
                )
            except Exception:
                pass

    def _notify_status(self, data: str):
        main = self._get_main()
        if main and hasattr(main, '_vue2_widget') and main._vue2_widget:
            try:
                main._vue2_widget.evaluate_js(
                    "if(window.onStatus) window.onStatus(" + data + ");"
                )
            except Exception:
                pass
        elif main and hasattr(main, '_webview_window') and main._webview_window:
            try:
                main._webview_window.evaluate_js(
                    "if(window.onStatus) window.onStatus(" + data + ");"
                )
            except Exception:
                pass

    def _notify_models(self, data: str):
        main = self._get_main()
        if main and hasattr(main, '_vue2_widget') and main._vue2_widget:
            try:
                main._vue2_widget.evaluate_js(
                    "if(window.onModelsLoaded) window.onModelsLoaded(" + data + ");"
                )
            except Exception:
                pass
        elif main and hasattr(main, '_webview_window') and main._webview_window:
            try:
                main._webview_window.evaluate_js(
                    "if(window.onModelsLoaded) window.onModelsLoaded(" + data + ");"
                )
            except Exception:
                pass

    # ── 前端可调用方法 (通过 pywebview js_api 暴露) ──

    def getState(self):
        """获取应用状态"""
        main = self._get_main()
        if not main:
            return json.dumps({})
        env = main.env_manager
        settings = env.read_settings()
        provider = settings.get("MODEL_PROVIDER", "")
        if provider == "api":
            api_source = settings.get("API_SOURCE", "")
            if api_source == "zhipu":
                active_model = settings.get("ZHIPU_MODEL", "")
            else:
                active_model = settings.get("API_MODEL", "")
        elif provider == "ollama":
            active_model = settings.get("OLLAMA_MODEL", "")
        else:
            active_model = settings.get("ANTHROPIC_MODEL", "")
        return json.dumps({
            "sessionId": main.active_session_id,
            "model": active_model,
            "busy": main.is_busy,
            "settings": settings,
            "workspacePath": main.current_workspace,
            "activeProjectId": main.active_project_id,
        })

    def newSession(self):
        main = self._get_main()
        if not main:
            return json.dumps({"sessionId": ""})
        if main.is_busy:
            return json.dumps({"sessionId": main.active_session_id or ""})
        main.active_session_id = _uuid()
        main.started_sessions.discard(main.active_session_id)
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
            self._notify_status(json.dumps({"busy": False}))

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
        if payload.get("auto_approve"):
            settings["AUTO_APPROVE"] = True
        else:
            settings["AUTO_APPROVE"] = False
        if payload.get("workspace_path"):
            settings["WORKSPACE_PATH"] = payload["workspace_path"]
            main.current_workspace = payload["workspace_path"]

        main.is_busy = True
        self._notify_status(json.dumps({"busy": True}))

        t = threading.Thread(target=self._run_cli, args=(prompt, model, provider, settings), daemon=True)
        t.start()

        return json.dumps({"ok": True, "sessionId": main.active_session_id})

    def selectDirectory(self):
        try:
            from PyQt6.QtWidgets import QFileDialog
            from PyQt6.QtCore import QDir
            main = self._get_main()
            parent = main if main else None
            path = QFileDialog.getExistingDirectory(
                parent, "选择项目工作目录", QDir.homePath(),
                QFileDialog.Option.ShowDirsOnly
            )
            if path:
                return json.dumps({"ok": True, "path": path})
            return json.dumps({"ok": False, "path": ""})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def openInExplorer(self, path: str):
        try:
            import subprocess
            if os.path.isdir(path):
                subprocess.Popen(f'explorer "{path}"')
                return True
            return False
        except Exception:
            return False

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
        self._notify_status(json.dumps({"busy": False}))
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
                api_base = payload.get("baseUrl", "") or settings.get("API_BASE_URL", "") or "http://127.0.0.1:7777"
                api_key = payload.get("apiKey", "") or settings.get("API_KEY", "")
                result = list_api_models(api_base, api_key, timeout)
            elif source == "zhipu":
                zhipu_key = payload.get("apiKey", "") or settings.get("ZHIPU_API_KEY", "")
                zhipu_base = payload.get("baseUrl", "") or settings.get("ZHIPU_BASE_URL", "") or ZHIPU_DEFAULT_BASE_URL
                result = list_zhipu_models(zhipu_key, zhipu_base, timeout)
            else:
                result = {"ok": False, "error": "Unsupported source."}
            self._notify_models(json.dumps(result))

        t = threading.Thread(target=_do_load, daemon=True)
        t.start()
        return json.dumps({"ok": True, "loading": True})

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

    # ── Ollama 可选对接：按需检测 + 一键安装（不写入部署维护默认项） ──

    def detectOllama(self):
        """检测 Ollama 本地运行时状态：是否安装、是否在运行

        返回结构化结果，前端根据不同状态展示不同提示：
        - {installed: False} → 引导一键安装
        - {installed: True, running: False} → 引导启动 Ollama
        - {installed: True, running: True} → 正常对接
        """
        result = {
            "installed": False,
            "running": False,
            "installPath": None,
            "version": None,
            "downloadUrl": "https://ollama.com/download/OllamaSetup.exe",
            "ollamaUrl": "http://127.0.0.1:11434",
            "error": None,
        }
        try:
            # ── Step 1: 检测是否安装 ──
            # Windows 标准安装路径
            candidate_paths = []
            localapp = os.environ.get("LOCALAPPDATA", "")
            programfiles = os.environ.get("ProgramFiles", r"C:\Program Files")
            programfiles86 = os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")
            userprofile = os.environ.get("USERPROFILE", "")

            if localapp:
                candidate_paths.append(os.path.join(localapp, "Programs", "Ollama", "ollama.exe"))
            if programfiles:
                candidate_paths.append(os.path.join(programfiles, "Ollama", "ollama.exe"))
            if programfiles86:
                candidate_paths.append(os.path.join(programfiles86, "Ollama", "ollama.exe"))
            # 用户可能解压到任意目录
            if userprofile:
                candidate_paths.append(os.path.join(userprofile, "ollama.exe"))
                candidate_paths.append(os.path.join(userprofile, "Downloads", "ollama.exe"))
            # PATH 中的 ollama
            try:
                import shutil as _sh
                which_ollama = _sh.which("ollama")
                if which_ollama:
                    candidate_paths.append(which_ollama)
            except Exception:
                pass

            for p in candidate_paths:
                if p and os.path.isfile(p):
                    result["installed"] = True
                    result["installPath"] = p
                    break

            # ── Step 2: 检测服务是否在跑 ──
            settings_path = os.path.join(
                self._get_main().data_dir if self._get_main() else "data",
                ".env",
            )
            ollama_url = "http://127.0.0.1:11434"
            try:
                if self._get_main() and hasattr(self._get_main(), "env_manager"):
                    settings = self._get_main().env_manager.read_settings()
                    ollama_url = (settings.get("OLLAMA_BASE_URL") or ollama_url).strip()
            except Exception:
                pass
            result["ollamaUrl"] = ollama_url

            if result["installed"]:
                try:
                    req = urllib.request.Request(
                        f"{ollama_url.rstrip('/')}/api/version",
                        headers={"User-Agent": "YunjiOllamaDetector/1.0"},
                    )
                    with urllib.request.urlopen(req, timeout=3) as resp:
                        if resp.status == 200:
                            data = json.loads(resp.read().decode("utf-8"))
                            result["running"] = True
                            result["version"] = data.get("version", "")
                except Exception:
                    result["running"] = False

            return json.dumps(result)
        except Exception as e:
            result["error"] = str(e)[:200]
            return json.dumps(result)

    def installOllama(self):
        """一键下载并静默安装 Ollama（仅在用户主动点击时触发）

        流程：下载 OllamaSetup.exe → 静默安装 /S → 等待进程结束 → 重新检测
        """
        try:
            import shutil as _sh
            main = self._get_main()
            if not main:
                return json.dumps({"ok": False, "error": "no main"})

            data_dir = main.data_dir
            os.makedirs(data_dir, exist_ok=True)
            setup_path = os.path.join(data_dir, "OllamaSetup.exe")

            # 下载（如果还没有）
            if not os.path.isfile(setup_path) or os.path.getsize(setup_path) < 1024 * 1024:
                url = "https://ollama.com/download/OllamaSetup.exe"
                main.log_signal.emit(f"[Ollama] 正在下载安装包: {url}", "#2196F3")
                urllib.request.urlretrieve(url, setup_path)
                main.log_signal.emit(f"[Ollama] 下载完成: {setup_path}", "#4CAF50")

            # 静默安装（OllamaSetup.exe 支持 /S 静默参数）
            main.log_signal.emit("[Ollama] 正在静默安装（约 30-60 秒）...", "#2196F3")
            r = subprocess.run(
                [setup_path, "/S"],
                capture_output=True, text=True, timeout=300,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            main.log_signal.emit(f"[Ollama] 安装退出码: {r.returncode}", "#4CAF50")

            # 等待几秒让 ollama app 完成注册
            import time as _t
            _t.sleep(3)

            # 重新检测
            return self.detectOllama()
        except Exception as e:
            return json.dumps({"ok": False, "error": f"安装失败: {e}"})

    def openOllamaDownloadPage(self):
        """打开 Ollama 官方下载页（兜底方案）"""
        try:
            from PyQt6.QtCore import QUrl
            from PyQt6.QtGui import QDesktopServices
            QDesktopServices.openUrl(QUrl("https://ollama.com/download"))
            return json.dumps({"ok": True})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def searchOllamaLibrary(self, query: str):
        try:
            q = (query or "").strip()
            url = f"https://ollama.com/api/models?q={urllib.parse.quote(q)}" if q else "https://ollama.com/api/models"
            req = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "Mozilla/5.0"})
            resp = urllib.request.urlopen(req, timeout=15)
            data = json.loads(resp.read().decode("utf-8"))
            models = []
            for m in (data if isinstance(data, list) else data.get("models", [])):
                name = m.get("name", "") or m.get("id", "")
                desc = m.get("description", "") or ""
                sizes = m.get("sizes", []) or []
                pulls = m.get("pulls", 0) or 0
                tags = m.get("tags", []) or []
                cap_tool = m.get("capabilities", {}).get("tools", False) if isinstance(m.get("capabilities"), dict) else False
                size_str = ""
                if sizes:
                    size_str = ", ".join(str(s) for s in sizes[:4])
                elif m.get("size"):
                    size_str = str(m["size"])
                models.append({
                    "name": name,
                    "description": desc,
                    "sizes": sizes,
                    "sizeStr": size_str,
                    "pulls": pulls,
                    "tags": tags,
                    "toolSupport": cap_tool,
                })
            return json.dumps({"ok": True, "models": models})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)[:200]})

    def pullModel(self, payload_json: str):
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

        def _do_pull():
            try:
                url = f"{base_url.rstrip('/')}/api/pull"
                body = json.dumps({"name": model_name, "stream": False}).encode("utf-8")
                req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"}, method="POST")
                resp = urllib.request.urlopen(req, timeout=600)
                result = json.loads(resp.read().decode("utf-8"))
                self._notify_models(json.dumps({"ok": True, "action": "pull_complete", "model": model_name}))
            except Exception as e:
                self._notify_models(json.dumps({"ok": False, "action": "pull_failed", "model": model_name, "error": str(e)[:200]}))

        t = threading.Thread(target=_do_pull, daemon=True)
        t.start()
        return json.dumps({"ok": True, "loading": True, "action": "pulling", "model": model_name})

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

    def fetchApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.fetch_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"fetchApiKey异常: {e}"})

    def listApiServices(self):
        try:
            return json.dumps({"ok": True, "services": backend.list_api_services()})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def getApiServiceInfo(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            service_name = payload.get("service", "").strip()
            return json.dumps(backend.get_api_service_info(service_name))
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def stopServiceByPort(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            port = int(payload.get("port", 0))
            if port:
                backend._stop_port_service(port)
            return json.dumps({"ok": True})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def startAllApiServices(self):
        try:
            result = backend.start_all_api_services()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def listAllModels(self):
        try:
            result = backend.list_all_models()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    def startQwen2Api(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            project_dir = payload.get("projectDir", "").strip()
            port = int(payload.get("port", 7777) or 7777)
            admin_key = payload.get("adminKey", "admin").strip() or "admin"
            result = backend.start_qwen2api(project_dir, port, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startQwen2Api异常: {e}"})

    def stopQwen2Api(self, payload: str = ""):
        try:
            base_url = ""
            if payload:
                try:
                    data = json.loads(payload)
                    base_url = data.get("baseUrl", "")
                except Exception:
                    pass
            result = backend.stop_qwen2api(base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"stopQwen2Api异常: {e}"})

    def checkApiService(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            result = backend.check_api_service(base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"checkApiService异常: {e}"})

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

    def listQwenAccounts(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.list_qwen_accounts(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listQwenAccounts异常: {e}"})

    def deleteQwenAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            email = payload.get("email", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.delete_qwen_account(base_url, email, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"deleteQwenAccount异常: {e}"})

    def startZhipu2Api(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            project_dir = payload.get("projectDir", "").strip()
            port = int(payload.get("port", 7780) or 7780)
            admin_key = payload.get("adminKey", "admin").strip() or "admin"
            result = backend.start_zhipu2api(project_dir, port, admin_key)
            if result.get("ok"):
                base_url = result.get("baseUrl", f"http://127.0.0.1:{port}")
                main = self._get_main()
                if main:
                    settings = main.env_manager.read_settings()
                    zhipu_key = (settings.get("ZHIPU_API_KEY", "") or settings.get("API_KEY", "")).strip()
                    if zhipu_key and len(zhipu_key) > 10 and not zhipu_key.startswith("sk-zhipu-"):
                        try:
                            backend.add_zhipu_account(base_url, zhipu_key, admin_key, label="auto-synced")
                        except Exception:
                            pass
                    zhipu_keys_path = os.path.join(main.user_dir, "zhipu_keys.json")
                    if os.path.exists(zhipu_keys_path):
                        try:
                            with open(zhipu_keys_path, "r", encoding="utf-8") as f:
                                keys_data = json.load(f)
                            if isinstance(keys_data, list):
                                for item in keys_data:
                                    if isinstance(item, dict) and item.get("key") and len(item["key"]) > 10 and not item["key"].startswith("sk-zhipu-"):
                                        try:
                                            backend.add_zhipu_account(base_url, item["key"], admin_key, label=item.get("label", "local-synced"))
                                        except Exception:
                                            pass
                        except Exception:
                            pass
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startZhipu2Api异常: {e}"})

    def stopZhipu2Api(self, payload: str = ""):
        try:
            base_url = ""
            if payload:
                try:
                    data = json.loads(payload)
                    base_url = data.get("baseUrl", "")
                except Exception:
                    pass
            result = backend.stop_zhipu2api(base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"stopZhipu2Api异常: {e}"})

    def addZhipuAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            api_key = payload.get("apiKey", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            label = payload.get("label", "").strip()
            result = backend.add_zhipu_account(base_url, api_key, admin_key, label)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"addZhipuAccount异常: {e}"})

    def listZhipuAccounts(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.list_zhipu_accounts(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listZhipuAccounts异常: {e}"})

    def deleteZhipuAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            api_key = payload.get("apiKey", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.delete_zhipu_account(base_url, api_key, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"deleteZhipuAccount异常: {e}"})

    def validateZhipuAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            api_key = payload.get("apiKey", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.validate_zhipu_account(base_url, api_key, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"validateZhipuAccount异常: {e}"})

    def fetchZhipuApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.fetch_zhipu_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"fetchZhipuApiKey异常: {e}"})

    def createZhipuApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.create_zhipu_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"createZhipuApiKey异常: {e}"})

    def startZhipuRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.start_zhipu_register(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startZhipuRegister异常: {e}"})

    def pollZhipuRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.poll_zhipu_register(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollZhipuRegister异常: {e}"})

    def loginZhipuAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            email = payload.get("email", "").strip()
            password = payload.get("password", "").strip()
            region = payload.get("region", "international").strip()
            result = backend.login_zhipu_account(base_url, email, password, region)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"loginZhipuAccount异常: {e}"})

    def setStickyAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            email = payload.get("email", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.set_sticky_account(base_url, email, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"setStickyAccount异常: {e}"})

    def clearStickyAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.clear_sticky_account(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"clearStickyAccount异常: {e}"})

    def startQwenLogin(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            email = payload.get("email", "").strip()
            password = payload.get("password", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.start_qwen_login(base_url, email, password, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startQwenLogin异常: {e}"})

    def pollQwenLogin(self):
        try:
            result = backend.poll_qwen_login()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollQwenLogin异常: {e}"})

    def startQwenRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            custom_email = payload.get("email", "").strip()
            custom_password = payload.get("password", "").strip()
            custom_username = payload.get("username", "").strip()
            result = backend.start_qwen_register(base_url, admin_key, custom_email, custom_password, custom_username)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startQwenRegister异常: {e}"})

    def pollQwenRegister(self):
        try:
            result = backend.poll_qwen_register()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollQwenRegister异常: {e}"})

    def checkZhipuApi(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            api_key = payload.get("apiKey", "").strip()
            base_url = payload.get("baseUrl", "").strip()
            result = check_zhipu_api(api_key, base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"checkZhipuApi异常: {e}"})

    def listZhipuModels(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            api_key = payload.get("apiKey", "").strip()
            base_url = payload.get("baseUrl", "").strip()
            result = list_zhipu_models(api_key, base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listZhipuModels异常: {e}"})

    def checkEnvironment(self):
        try:
            result = backend.check_environment()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"overall": {"ok": False, "label": "检测服务", "error": str(e), "fix": "请确保后端服务正常运行"}})

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
                api_source = (settings.get("API_SOURCE", "") or "").strip().lower()
                if api_source == "zhipu":
                    zhipu_key = (settings.get("ZHIPU_API_KEY", "") or "").strip()
                    zhipu_model = (settings.get("ZHIPU_MODEL", "") or "glm-4.7-flash").strip()
                    zhipu_base = (settings.get("ZHIPU_BASE_URL", "") or "http://127.0.0.1:7780").strip()
                    zhipu_base = zhipu_base.rstrip("/").removesuffix("/v1")
                    is_proxy = ":7780" in zhipu_base or "127.0.0.1:7780" in zhipu_base or "localhost:7780" in zhipu_base
                    if is_proxy:
                        main.log_signal.emit(f"[代理] 智谱API代理模式(Anthropic兼容) 模型={zhipu_model} 代理={zhipu_base}", "#2196F3")
                        env_overrides["MODEL_PROVIDER"] = "anthropic"
                        env_overrides["ANTHROPIC_BASE_URL"] = zhipu_base
                        env_overrides["ANTHROPIC_API_KEY"] = zhipu_key
                        env_overrides["ANTHROPIC_AUTH_TOKEN"] = zhipu_key
                        env_overrides["ANTHROPIC_MODEL"] = zhipu_model
                        env_overrides["API_BASE_URL"] = zhipu_base
                        env_overrides["API_MODEL"] = zhipu_model
                        env_overrides["API_KEY"] = zhipu_key
                    else:
                        main.log_signal.emit(f"[代理] 智谱API直连模式(OpenAI兼容代理) 模型={zhipu_model} 目标={zhipu_base}", "#2196F3")
                        env_overrides["MODEL_PROVIDER"] = "api"
                        env_overrides["API_BASE_URL"] = zhipu_base
                        env_overrides["API_MODEL"] = zhipu_model
                        env_overrides["API_KEY"] = zhipu_key
                        env_overrides["API_OPENAI_COMPAT"] = "1"
                        env_overrides["ANTHROPIC_API_KEY"] = zhipu_key
                        env_overrides["ANTHROPIC_AUTH_TOKEN"] = zhipu_key
                        env_overrides["ANTHROPIC_MODEL"] = zhipu_model
                else:
                    api_base = (settings.get("API_BASE_URL", "") or "http://127.0.0.1:7777").strip()
                    api_model = (settings.get("API_MODEL", "") or "qwen3.6-plus").strip()
                    api_key = (settings.get("API_KEY", "") or "").strip()
                    main.log_signal.emit(f"[代理] Qwen API模式 模型={api_model} 目标={api_base}", "#2196F3")
                    env_overrides["MODEL_PROVIDER"] = "api"
                    env_overrides["API_BASE_URL"] = api_base
                    env_overrides["API_MODEL"] = api_model
                    if api_key:
                        env_overrides["API_KEY"] = api_key
                        env_overrides["ANTHROPIC_API_KEY"] = api_key
                    env_overrides["ANTHROPIC_BASE_URL"] = api_base
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

            system_prompt = self._build_system_prompt(settings, main.current_workspace)

            def _on_delta(text):
                self._notify_delta(json.dumps({"text": text}))

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
                auto_approve=settings.get("AUTO_APPROVE", False),
            )

            if result.get("ok"):
                cli_sid = result.get("cliSessionId", "")
                if cli_sid and cli_sid != main.active_session_id:
                    main.log_signal.emit(f"[CLI] 更新session_id: {main.active_session_id} -> {cli_sid}", "#2196F3")
                    main.active_session_id = cli_sid
                main.started_sessions.add(main.active_session_id)
                result_text = result.get("text", "").strip()
                if result_text and not result.get("streamed"):
                    self._notify_delta(json.dumps({"text": result_text}))
            else:
                cli_sid = result.get("cliSessionId", "")
                if cli_sid and cli_sid != main.active_session_id:
                    main.active_session_id = cli_sid
                main.started_sessions.add(main.active_session_id)
                err = result.get("error", "")[:200]
                main.log_signal.emit(f"[CLI 错误] {err}", "#F44336")
                if err and not result.get("text"):
                    self._notify_delta(json.dumps({"text": f"❌ {err}"}))

            main.result_ready_signal.emit(json.dumps(result))
        except Exception as e:
            main.log_signal.emit(f"[线程异常] {e}", "#F44336")
            self._notify_delta(json.dumps({"text": f"❌ 线程异常: {str(e)[:200]}"}))
            main.result_ready_signal.emit(json.dumps({"ok": False, "error": str(e)}))
        finally:
            import time as _time
            _time.sleep(0.3)
            main.is_busy = False
            main.active_proc = None
            self._notify_status(json.dumps({"busy": False}))


    @staticmethod
    def _build_system_prompt(settings: dict, workspace: str = "") -> str:
        custom_prompt = (settings.get("SYSTEM_PROMPT") or "").strip()
        if custom_prompt:
            return custom_prompt

        language = (settings.get("AI_LANGUAGE") or "zh").strip().lower()
        parts = []

        if workspace:
            parts.append(f"当前项目工作目录: {workspace}")
            parts.append(f"所有文件操作（创建、读取、编辑）都必须使用绝对路径，以 {workspace} 为根目录。例如创建文件应写 {workspace}\\index.html 而非 index.html。")
            parts.append("")

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

    def __init__(self, base_dir: str, log_func=None, progress_func=None, data_dir: str = None, temp_dir: str = None):
        self.base_dir = base_dir
        self.data_dir = data_dir or base_dir
        self.temp_dir = temp_dir or os.path.join(os.path.dirname(base_dir), "temp")
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func
        self._mirror_key = "china"

    def _load_mirror(self):
        try:
            fp = os.path.join(self.data_dir, MIRROR_SETTINGS_FILE)
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
            fp = os.path.join(self.data_dir, MIRROR_SETTINGS_FILE)
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
    def venv_dir(self): return os.path.join(self.data_dir, ".venv")
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
            bun_cache_dir = os.path.join(self.temp_dir, "cache", "bun_cache")
            os.makedirs(bun_cache_dir, exist_ok=True)
            env["BUN_INSTALL_CACHE_DIR"] = bun_cache_dir
            subprocess.run([self.bun_exe, "config", "set", "registry", "https://registry.npmmirror.com"],
                           cwd=self.base_dir, env=env, startupinfo=self._si(), capture_output=True, timeout=30,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            r = subprocess.run([self.bun_exe, "install"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=300,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                self.log("✓ 依赖安装完成")
                self._fix_jsonc_parser_esm()
                return True
            self.log(f"[警告] bun install 返回码: {r.returncode}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] 依赖安装失败: {e}", "#F44336")
            return False

    def _fix_jsonc_parser_esm(self):
        esm_dir = os.path.join(self.base_dir, "node_modules", "jsonc-parser", "lib", "esm")
        if not os.path.isdir(esm_dir):
            return
        pkg_path = os.path.join(esm_dir, "package.json")
        if not os.path.exists(pkg_path):
            try:
                with open(pkg_path, 'w', encoding='utf-8') as f:
                    f.write('{"type":"module"}')
                self.log("  修复 jsonc-parser ESM 类型声明")
            except Exception:
                pass
        targets = ['scanner', 'parser', 'format', 'edit', 'string-intern']
        count = 0
        for root, dirs, files in os.walk(esm_dir):
            for fname in files:
                if not fname.endswith('.js'):
                    continue
                fpath = os.path.join(root, fname)
                try:
                    with open(fpath, 'r', encoding='utf-8') as f:
                        content = f.read()
                    original = content
                    for t in targets:
                        content = content.replace(f"from './{t}'", f"from './{t}.js'")
                        content = content.replace(f"from '../{t}'", f"from '../{t}.js'")
                    if content != original:
                        with open(fpath, 'w', encoding='utf-8') as f:
                            f.write(content)
                        count += 1
                except Exception:
                    pass
        if count:
            self.log(f"  修复 jsonc-parser ESM 导入 ({count} 文件)")

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
            bun_cache_dir = os.path.join(self.temp_dir, "cache", "bun_cache")
            os.makedirs(bun_cache_dir, exist_ok=True)
            env["BUN_INSTALL_CACHE_DIR"] = bun_cache_dir
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
        # 设置 UV 缓存目录设置到我们的 app 目录下，避免权限问题
        uv_cache_dir = os.path.join(self.temp_dir, "cache", "uv_cache")
        os.makedirs(uv_cache_dir, exist_ok=True)
        env["UV_CACHE_DIR"] = uv_cache_dir
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
        if not self.install_uv():
            self.log("[错误] uv 安装失败，无法继续", "#F44336")
            return False
        try:
            env = self._uv_env()

            uv_exe_abs = os.path.abspath(self.uv_exe)
            venv_dir_abs = os.path.abspath(self.venv_dir)
            venv_python_abs = os.path.abspath(self.venv_python)
            base_dir_abs = os.path.abspath(self.base_dir)

            os.makedirs(os.path.dirname(venv_dir_abs), exist_ok=True)

            old_venv_dir = os.path.join(self.data_dir, "venvs", ".venv")
            if os.path.isdir(old_venv_dir) and not os.path.isdir(venv_dir_abs):
                self.log("  迁移旧虚拟环境: data/venvs/.venv → data/.venv")
                try:
                    shutil.move(old_venv_dir, venv_dir_abs)
                    old_venvs_parent = os.path.join(self.data_dir, "venvs")
                    if os.path.isdir(old_venvs_parent) and not os.listdir(old_venvs_parent):
                        os.rmdir(old_venvs_parent)
                    self.log("  ✓ 虚拟环境迁移完成")
                except Exception as e:
                    self.log(f"  迁移失败: {e}，将重新创建", "#FF9800")

            venv_valid = False
            if os.path.isfile(venv_python_abs):
                pip_exe = os.path.join(venv_dir_abs, "Scripts", "pip.exe")
                if not os.path.exists(pip_exe):
                    pip_exe = os.path.join(venv_dir_abs, "bin", "pip")
                if os.path.exists(pip_exe):
                    venv_valid = True
                    self.log(f"✓ 虚拟环境已存在且有效: {venv_python_abs}")
                else:
                    self.log(f"  警告: 虚拟环境已存在但缺少 pip.exe，需要重建", "#FF9800")
            
            if not venv_valid:
                self.log("正在创建虚拟环境...")
                self.log(f"  虚拟环境目录: {venv_dir_abs}")
                if os.path.exists(venv_dir_abs):
                    import shutil
                    try:
                        shutil.rmtree(venv_dir_abs)
                        self.log(f"  已清理旧的虚拟环境目录")
                    except Exception as e:
                        self.log(f"  警告：无法清理旧的虚拟环境目录: {e}")
                try:
                    import venv
                    self.log("  使用 Python venv 创建虚拟环境...")
                    venv.create(venv_dir_abs, with_pip=True)
                    self.log("  ✓ venv 创建成功")
                except Exception as e:
                    self.log(f"  警告: venv 创建失败: {e}, 尝试用 uv...", "#FF9800")
                    r = subprocess.run(
                        [uv_exe_abs, "venv", self.venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=base_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if r.returncode != 0:
                        self.log(f"  uv 创建也失败 (返回码: {r.returncode})", "#FF9800")
                        if r.stdout:
                            self.log(f"  stdout: {r.stdout[:500]}")
                        if r.stderr:
                            self.log(f"  stderr: {r.stderr[:500]}", "#FF9800")
                if not os.path.exists(venv_python_abs):
                    self.log(f"[错误] 找不到 python.exe: {venv_python_abs}", "#F44336")
                    if os.path.exists(venv_dir_abs):
                        contents = os.listdir(venv_dir_abs)
                        self.log(f"  虚拟环境目录内容: {contents}")
                    return False
                self.log("✓ 虚拟环境已创建")

            self.log("正在安装 API 服务依赖...")
            pip_exe = os.path.join(venv_dir_abs, "Scripts", "pip.exe")
            if not os.path.exists(pip_exe):
                pip_exe = os.path.join(venv_dir_abs, "bin", "pip")

            req_files = []
            for svc in ["qwen2api", "zhipu2api"]:
                for candidate in [
                    os.path.join(self.base_dir, "api", svc, "backend", "requirements.txt"),
                    os.path.join(self.base_dir, "api", svc, "requirements.txt"),
                ]:
                    if os.path.exists(candidate):
                        req_files.append(candidate)
                        break

            if not req_files:
                self.log("⚠ 未找到任何 API 服务的 requirements.txt", "#FF9800")
                return True

            last_r = None
            if os.path.exists(pip_exe):
                self.log(f"  使用 pip: {pip_exe}")
                self.log("  升级 pip...")
                subprocess.run(
                    [pip_exe, "install", "--upgrade", "pip"],
                    env=env, capture_output=True, text=True, timeout=300,
                    cwd=base_dir_abs,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
                for rf in req_files:
                    self.log(f"  安装 {os.path.basename(os.path.dirname(rf))} 依赖...")
                    last_r = subprocess.run(
                        [pip_exe, "install", "-r", rf],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=base_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if last_r.returncode != 0:
                        break
            else:
                self.log(f"  警告: pip.exe 不存在，尝试用 uv pip")
                for rf in req_files:
                    self.log(f"  安装 {os.path.basename(os.path.dirname(rf))} 依赖...")
                    last_r = subprocess.run(
                        [uv_exe_abs, "pip", "install", "-r", rf],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=venv_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if last_r.returncode != 0:
                        break

            if last_r and last_r.returncode == 0:
                self.log("✓ API 服务依赖安装完成")
                return True
            if last_r:
                self.log(f"[警告] API 服务依赖安装返回码: {last_r.returncode}", "#FF9800")
                if last_r.stdout:
                    self.log(f"  stdout: {last_r.stdout[:500]}")
                if last_r.stderr:
                    self.log(f"  stderr: {last_r.stderr[:500]}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] API 服务依赖安装失败: {e}", "#F44336")
            import traceback
            self.log(f"  堆栈: {traceback.format_exc()[:500]}", "#F44336")
            return False


class ServiceCard(QFrame):
    """运行服务 - 单个服务的状态卡片（参考云集智能视频创意站架构）"""
    restart_clicked = pyqtSignal(str)
    open_clicked = pyqtSignal(str)
    stop_clicked = pyqtSignal(str)

    def __init__(self, service_id: str, info: dict, parent=None):
        super().__init__(parent)
        self.service_id = service_id
        self.info = info
        self.is_running = False
        self.setObjectName("serviceCard")
        self._setup_ui()

    def _setup_ui(self):
        # 2026-06-16 重做：上下两行布局，描述自动换行，永不截断
        layout = QVBoxLayout(self)
        layout.setSpacing(10)
        layout.setContentsMargins(18, 14, 18, 14)

        # ── 第一行：图标 + 名称（占满） + 状态指示 ──
        top_row = QHBoxLayout()
        top_row.setSpacing(12)

        icon_lbl = QLabel(self.info.get("icon", "⚙️"))
        icon_lbl.setStyleSheet("font-size: 26px; background: transparent; border: none;")
        icon_lbl.setFixedWidth(32)
        top_row.addWidget(icon_lbl)

        name_lbl = QLabel(self.info.get("label", self.service_id))
        name_lbl.setStyleSheet("font-size: 16px; font-weight: bold; color: #FFFFFF; background: transparent; border: none;")
        # 不再 setMaximumWidth / 不再 elide，让它自然占满
        name_lbl.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Preferred)
        name_lbl.setTextFormat(Qt.TextFormat.PlainText)
        top_row.addWidget(name_lbl, 1)

        self.status_dot = QLabel()
        self.status_dot.setFixedSize(12, 12)
        self.status_dot.setStyleSheet("background-color: #424242; border: 2px solid #616161; border-radius: 6px;")
        top_row.addWidget(self.status_dot)

        self.status_text = QLabel("未启动")
        self.status_text.setStyleSheet("font-size: 12px; color: #AAAAAA; background: transparent; border: none;")
        self.status_text.setFixedWidth(60)
        top_row.addWidget(self.status_text)

        layout.addLayout(top_row)

        # ── 第二行：描述（自动换行，不再截断）──
        desc_lbl = QLabel(self.info.get("desc", ""))
        desc_lbl.setStyleSheet("font-size: 12px; color: #BBBBBB; background: transparent; border: none; line-height: 1.5;")
        desc_lbl.setWordWrap(True)         # 关键：开启自动换行
        desc_lbl.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        layout.addWidget(desc_lbl)

        # ── 第三行：按钮 + 端口 ──
        btn_row = QHBoxLayout()
        btn_row.setSpacing(8)

        self.restart_btn = QPushButton("🔄 重启")
        self.restart_btn.setStyleSheet("""
            QPushButton {
                background-color: #1565C0; color: #FFFFFF;
                border: 1px solid #1976D2; border-radius: 6px;
                padding: 5px 14px; font-size: 12px; font-weight: bold;
            }
            QPushButton:hover { background-color: #1976D2; }
            QPushButton:disabled { background-color: #1a1a1a; color: #555; border-color: #333; }
        """)
        self.restart_btn.clicked.connect(lambda: self.restart_clicked.emit(self.service_id))
        btn_row.addWidget(self.restart_btn)

        self.open_btn = QPushButton("🌐 打开")
        self.open_btn.setStyleSheet(f"""
            QPushButton {{
                background-color: {self.info.get('color', '#388E3C')}; color: #FFFFFF;
                border: 1px solid {self.info.get('color', '#388E3C')}; border-radius: 6px;
                padding: 5px 14px; font-size: 12px; font-weight: bold;
            }}
            QPushButton:hover {{ background-color: {self.info.get('hover_color', '#4CAF50')}; }}
            QPushButton:disabled {{ background-color: #1a1a1a; color: #555; border-color: #333; }}
        """)
        self.open_btn.clicked.connect(lambda: self.open_clicked.emit(self.service_id))
        btn_row.addWidget(self.open_btn)

        self.stop_btn = QPushButton("⏹ 停止")
        self.stop_btn.setStyleSheet("""
            QPushButton {
                background-color: #C62828; color: #FFFFFF;
                border: 1px solid #D32F2F; border-radius: 6px;
                padding: 5px 14px; font-size: 12px; font-weight: bold;
            }
            QPushButton:hover { background-color: #D32F2F; }
            QPushButton:disabled { background-color: #1a1a1a; color: #555; border-color: #333; }
        """)
        self.stop_btn.clicked.connect(lambda: self.stop_clicked.emit(self.service_id))
        self.stop_btn.setEnabled(False)
        btn_row.addWidget(self.stop_btn)

        btn_row.addStretch()

        # 端口信息
        port = self.info.get("default_port", 0)
        if port:
            self.port_lbl = QLabel(f":{port}")
            self.port_lbl.setStyleSheet("font-size: 12px; color: #AAAAAA; background: transparent; border: none; padding: 4px 8px;")
            btn_row.addWidget(self.port_lbl)

        layout.addLayout(btn_row)

    def set_running(self, running: bool):
        """更新服务运行状态"""
        self.is_running = running
        if running:
            self.status_dot.setStyleSheet("background-color: #4CAF50; border: 2px solid #66BB6A; border-radius: 6px;")
            self.status_text.setText("运行中")
            self.status_text.setStyleSheet("font-size: 12px; color: #4CAF50; background: transparent; border: none;")
            self.restart_btn.setEnabled(True)
            self.stop_btn.setEnabled(True)
            self.open_btn.setEnabled(True)
        else:
            self.status_dot.setStyleSheet("background-color: #424242; border: 2px solid #616161; border-radius: 6px;")
            self.status_text.setText("未启动")
            self.status_text.setStyleSheet("font-size: 12px; color: #AAAAAA; background: transparent; border: none;")
            self.restart_btn.setEnabled(False)
            self.stop_btn.setEnabled(False)
            self.open_btn.setEnabled(False)

    def set_starting(self):
        """显示启动中状态"""
        self.status_dot.setStyleSheet("background-color: #FFC107; border: 2px solid #FFD54F; border-radius: 6px;")
        self.status_text.setText("启动中...")
        self.status_text.setStyleSheet("font-size: 12px; color: #FFC107; background: transparent; border: none;")
        self.restart_btn.setEnabled(False)
        self.stop_btn.setEnabled(False)
        self.open_btn.setEnabled(False)


class SplashScreen(QSplashScreen):
    def __init__(self):
        pixmap = QPixmap(520, 360)
        pixmap.fill(QColor("#0d0d0d"))
        super().__init__(pixmap)
        self.setWindowFlags(Qt.WindowType.WindowStaysOnTopHint | Qt.WindowType.FramelessWindowHint)
        self._progress = 0.0
        self._message = "正在初始化..."
        self._icon_pixmap = None
        try:
            if hasattr(sys, '_MEIPASS'):
                base = sys._MEIPASS
            elif hasattr(sys, 'frozen'):
                base = os.path.dirname(sys.executable)
            else:
                base = os.path.dirname(os.path.abspath(__file__))
            for name in ('icon.png', 'icon.ico'):
                p = os.path.join(base, name)
                if os.path.exists(p):
                    self._icon_pixmap = QPixmap(p)
                    if not self._icon_pixmap.isNull():
                        break
                    self._icon_pixmap = None
        except Exception:
            pass

    def _get_progress(self):
        return self._progress

    def _set_progress(self, val):
        self._progress = val
        self.repaint()

    progress = pyqtProperty(float, _get_progress, _set_progress)

    def set_progress(self, value, message=""):
        if message:
            self._message = message
        anim = QPropertyAnimation(self, b"progress")
        anim.setDuration(300)
        anim.setStartValue(self._progress)
        anim.setEndValue(value)
        anim.start()
        self._anim = anim
        if message:
            self._message = message
            self.repaint()

    def drawContents(self, painter):
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        w, h = self.width(), self.height()
        painter.fillRect(0, 0, w, h, QColor("#0d0d0d"))

        if self._icon_pixmap:
            icon_size = 80
            scaled = self._icon_pixmap.scaled(icon_size, icon_size, Qt.AspectRatioMode.KeepAspectRatio, Qt.TransformationMode.SmoothTransformation)
            ix = (w - scaled.width()) // 2
            painter.drawPixmap(ix, 60, scaled)

        painter.setPen(QColor("#F0F0F0"))
        title_font = QFont("Microsoft YaHei", 22, QFont.Weight.Bold)
        painter.setFont(title_font)
        title = "云集智能编程工作站"
        fm = painter.fontMetrics()
        tw = fm.horizontalAdvance(title)
        painter.drawText((w - tw) // 2, 180, title)

        bar_x, bar_y, bar_w, bar_h = 60, 240, w - 120, 10
        painter.setPen(Qt.PenStyle.NoPen)
        painter.setBrush(QColor("#222222"))
        painter.drawRoundedRect(QRectF(bar_x, bar_y, bar_w, bar_h), 5, 5)

        fill_w = bar_w * min(self._progress, 1.0)
        if fill_w > 0:
            grad = QLinearGradient(bar_x, bar_y, bar_x + fill_w, bar_y)
            grad.setColorAt(0, QColor("#1565C0"))
            grad.setColorAt(1, QColor("#42A5F5"))
            painter.setBrush(grad)
            painter.drawRoundedRect(QRectF(bar_x, bar_y, fill_w, bar_h), 5, 5)

        painter.setPen(QColor("#888888"))
        msg_font = QFont("Microsoft YaHei", 10)
        painter.setFont(msg_font)
        msg = self._message
        fm2 = painter.fontMetrics()
        mw = fm2.horizontalAdvance(msg)
        painter.drawText((w - mw) // 2, 275, msg)

        pct = f"{int(min(self._progress, 1.0) * 100)}%"
        painter.setPen(QColor("#42A5F5"))
        pct_font = QFont("Microsoft YaHei", 9)
        painter.setFont(pct_font)
        fm3 = painter.fontMetrics()
        pw = fm3.horizontalAdvance(pct)
        painter.drawText((w - pw) // 2, 300, pct)


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
    voice_result_signal = pyqtSignal(str)

    def __init__(self, splash=None):
        super().__init__()
        self._splash = splash
        self.setWindowTitle(f"云集智能编程工作站 v{VERSION}")
        self.setMinimumSize(980, 680)

        try:
            if hasattr(sys, '_MEIPASS'):
                icon_path = os.path.join(sys._MEIPASS, 'icon.ico')
            elif hasattr(sys, 'frozen'):
                icon_path = os.path.join(os.path.dirname(sys.executable), 'app', 'icon.ico')
            else:
                icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'icon.ico')
            if os.path.exists(icon_path):
                self.setWindowIcon(QIcon(icon_path))
                if hasattr(sys, 'frozen'):
                    import ctypes
                    hwnd = int(self.winId())
                    cx = ctypes.windll.user32.GetSystemMetrics(49)
                    cy = ctypes.windll.user32.GetSystemMetrics(50)
                    ico_small = ctypes.windll.user32.LoadImageW(0, icon_path, 1, cx, cy, 0x10)
                    ico_big = ctypes.windll.user32.LoadImageW(0, icon_path, 1, ctypes.windll.user32.GetSystemMetrics(11), ctypes.windll.user32.GetSystemMetrics(12), 0x10)
                    if ico_small:
                        ctypes.windll.user32.SendMessageW(hwnd, 0x0080, 0, ico_small)
                    if ico_big:
                        ctypes.windll.user32.SendMessageW(hwnd, 0x0080, 1, ico_big)
        except Exception:
            pass

        # 基础目录（三目录架构）
        # 架构说明：
        #   YunJiSmartIDE/           = 安装目录
        #   ├── YunJiSmartIDE.exe    = 主程序
        #   ├── app/                 = 应用程序（只读资源）
        #   │   ├── main.py
        #   │   ├── desktop/dist/
        #   │   ├── nodejs/
        #   │   ├── api/qwen2api/
        #   │   └── api/zhipu2api/
        #   ├── data/                = 用户数据（可写，需备份）
        #   │   ├── .env
        #   │   ├── projects/
        #   │   ├── sessions/
        #   │   ├── api/qwen2api/
        #   │   ├── api/zhipu2api/
        #   │   └── .venv/
        #   └── temp/                = 临时文件（可清空）
        #       ├── logs/
        #       ├── cache/
        #       └── debug/
        
        if hasattr(sys, 'frozen'):
            # PyInstaller 打包模式
            # --onefile: sys.executable 在临时解压目录，需要用实际 EXE 位置
            # --onedir: sys.executable 就在发布目录
            if hasattr(sys, '_MEIPASS') and not os.path.exists(
                os.path.join(os.path.dirname(sys.executable), "app")
            ):
                # --onefile 模式：EXE 旁边没有 app/，需要在用户可见的 EXE 位置查找
                import ctypes
                buf = ctypes.create_unicode_buffer(512)
                ctypes.windll.kernel32.GetModuleFileNameW(None, buf, 512)
                real_exe = buf.value
                exe_dir = os.path.abspath(os.path.dirname(real_exe))
            else:
                exe_dir = os.path.abspath(os.path.dirname(sys.executable))
            self.exe_dir = exe_dir
            self.app_dir = os.path.join(exe_dir, "app")
            self.data_dir = os.path.join(exe_dir, "data")
            self.temp_dir = os.path.join(exe_dir, "temp")
        else:
            # 开发模式：main.py 在 dev/app/ 下
            script_dir = os.path.dirname(os.path.abspath(__file__))  # dev/app/
            self.exe_dir = os.path.dirname(script_dir)  # dev/
            self.app_dir = script_dir
            self.data_dir = os.path.join(self.exe_dir, "data")
            self.temp_dir = os.path.join(self.exe_dir, "temp")
        
        # 确保目录存在
        os.makedirs(self.data_dir, exist_ok=True)
        os.makedirs(self.temp_dir, exist_ok=True)
        
        # ============================================
        # 数据分级架构 (类似 Windows 用户目录逻辑)
        # ============================================
        # 
        # data/                          ← 数据根目录
        # ├── public/                    ← 公共数据 (所有用户共享)
        # │   ├── models/               ← AI模型文件
        # │   ├── templates/            ← 共享模板
        # │   ├── plugins/              ← 插件
        # │   └── api/                  ← API服务配置
        # │       ├── qwen2api/
        # │       └── zhipu2api/
        # │
        # └── users/                     ← 用户数据隔离目录
        #     └── default/              ← 默认本地用户 (未登录)
        #         ├── projects/         ← 项目注册表
        #         ├── sessions/         ← AI会话数据
        #         ├── .env              ← 用户环境配置 (API密钥等)
        #         └── zhipu_keys.json   ← 用户私密密钥
        #
        # 未来多用户登录:
        #     users/
        #     ├── default/              ← 本地用户
        #     ├── user_abc123/          ← 登录用户A
        #     └── user_def456/          ← 登录用户B
        #
        # ============================================
        
        # 公共数据目录
        self.public_dir = os.path.join(self.data_dir, "public")
        os.makedirs(os.path.join(self.public_dir, "models"), exist_ok=True)
        os.makedirs(os.path.join(self.public_dir, "templates"), exist_ok=True)
        os.makedirs(os.path.join(self.public_dir, "plugins"), exist_ok=True)
        os.makedirs(os.path.join(self.public_dir, "api", "qwen2api"), exist_ok=True)
        os.makedirs(os.path.join(self.public_dir, "api", "zhipu2api"), exist_ok=True)
        
        # 用户数据目录 (当前默认用户)
        self.current_user_id = "default"
        self.user_dir = os.path.join(self.data_dir, "users", self.current_user_id)
        os.makedirs(self.user_dir, exist_ok=True)
        os.makedirs(os.path.join(self.user_dir, "projects"), exist_ok=True)
        os.makedirs(os.path.join(self.user_dir, "sessions"), exist_ok=True)
        
        # 临时目录
        os.makedirs(os.path.join(self.temp_dir, "logs"), exist_ok=True)
        os.makedirs(os.path.join(self.temp_dir, "cache"), exist_ok=True)
        os.makedirs(os.path.join(self.temp_dir, "debug"), exist_ok=True)
        os.makedirs(os.path.join(self.temp_dir, "tmp"), exist_ok=True)

        # 初始化后端组件
        # .env 文件现在在 data/users/default/ 目录下 (用户级私密配置)
        self.env_manager = EnvFileManager(os.path.join(self.user_dir, ".env"))
        
        self.cli_runner = ClaudeCliRunner(
            self.app_dir,
            os.path.join(self.app_dir, "nodejs", NODE_DIR_NAME),
            os.path.join(self.app_dir, "bun", BUN_DIR_NAME),
        )
        
        # EnvInstaller: base_dir=app_dir(资源), data_dir=data_dir(用户设置)
        self.installer = EnvInstaller(self.app_dir, data_dir=self.data_dir, temp_dir=self.temp_dir)
        self.updater = SoftwareUpdater(self.exe_dir)
        
        # ProjectManager 需要知道 data_dir 和 user_id
        self.project_mgr = ProjectManager(
            self.app_dir, 
            self.exe_dir, 
            data_dir=self.data_dir,
            user_id=self.current_user_id
        )

        # 状态
        self.active_session_id = _uuid()
        self.started_sessions = set()
        self.active_project_id = None
        active_proj = self.project_mgr.get_active_project()
        if active_proj:
            self.active_project_id = active_proj["id"]
            self.current_workspace = active_proj.get("workspace_path") or self.app_dir
        else:
            self.current_workspace = self.app_dir
        self.is_busy = False
        self.active_proc = None

        # 服务管理相关
        self.service_cards: dict = {}
        self.service_processes: dict = {}  # service_id -> subprocess.Popen
        self.service_monitor_timer = QTimer(self)
        self.service_monitor_timer.timeout.connect(self._monitor_services)
        self._is_starting_all = False

        # WebView 相关
        self._web_engine_view = None
        self._web_engine_page = None
        self._web_channel = None
        self._using_webengine = False
        self._webview_window = None  # pywebview 独立窗口
        self._webview2_widget = None  # QtWebView2 内嵌 widget
        self._vue2_widget = None  # Vue 内嵌 widget 页面（带侧边栏的完整界面）
        self._webview2_initialized = False  # Vue2 窗口是否已创建

        if self._splash:
            self._splash.set_progress(0.3, "正在构建界面...")

        # 构建 UI
        self._setup_ui()

        if self._splash:
            self._splash.set_progress(0.6, "正在连接信号...")

        # 连接信号
        self.log_signal.connect(self._append_log)
        self.debug_log_signal.connect(self._append_debug_log)
        self.progress_signal.connect(self._update_progress)
        self.deploy_step_signal.connect(self._update_deploy_step)
        self.status_signal.connect(self._update_status)
        self.result_ready_signal.connect(self._on_result_ready)
        self.workspace_choose_requested.connect(self._choose_workspace_dialog)
        self.voice_result_signal.connect(self._on_voice_result)

        if self._splash:
            self._splash.set_progress(0.9, "即将就绪...")

        # 启动时检查环境
        QTimer.singleShot(800, self._auto_check_and_load)

        # 超时保底：如果前端 15 秒内未加载完成，直接关闭 splash 显示主窗口
        QTimer.singleShot(15000, self._splash_fallback)

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

        # ── 导航栏 ──
        nav_bar = QFrame()
        self.voice_result_signal.connect(self._on_voice_result)

        if self._splash:
            self._splash.set_progress(0.9, "即将就绪...")

        # 启动时检查环境
        QTimer.singleShot(800, self._auto_check_and_load)

        # 超时保底：如果前端 15 秒内未加载完成，直接关闭 splash 显示主窗口
        QTimer.singleShot(15000, self._splash_fallback)

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

        # 任务对话按钮（首页，最左边）
        self.btn_chat = QPushButton("💬 任务对话")
        self.btn_chat.setCheckable(True)
        self.btn_chat.setChecked(True)
        self.btn_chat.setStyleSheet(menu_button_style)
        self.btn_chat.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_chat.clicked.connect(lambda: self._switch_page(0))
        nav_layout.addWidget(self.btn_chat)

        # 运行服务按钮
        self.btn_home = QPushButton("🚀 运行服务")
        self.btn_home.setCheckable(True)
        self.btn_home.setStyleSheet(menu_button_style)
        self.btn_home.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_home.clicked.connect(lambda: self._switch_page(1))
        nav_layout.addWidget(self.btn_home)

        # 部署维护按钮
        self.btn_deploy_nav = QPushButton("⚙️ 部署维护")
        self.btn_deploy_nav.setCheckable(True)
        self.btn_deploy_nav.setStyleSheet(menu_button_style)
        self.btn_deploy_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_deploy_nav.clicked.connect(lambda: self._switch_page(2))
        nav_layout.addWidget(self.btn_deploy_nav)

        # 软件更新按钮
        self.btn_update_nav = QPushButton("📋 软件更新")
        self.btn_update_nav.setCheckable(True)
        self.btn_update_nav.setStyleSheet(menu_button_style)
        self.btn_update_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_update_nav.clicked.connect(lambda: self._switch_page(3))
        nav_layout.addWidget(self.btn_update_nav)

        # 项目管理按钮
        self.btn_project_nav = QPushButton("📁 项目管理")
        self.btn_project_nav.setCheckable(True)
        self.btn_project_nav.setStyleSheet(menu_button_style)
        self.btn_project_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        # 2026-06-16 修复：项目管理/系统设置原本都跳到 _switch_page(4) 但 page_stack
        # 根本没有这个 index，导致点击没反应。现在分别为 4 和 5。
        self.btn_project_nav.clicked.connect(lambda: self._switch_page(4))
        nav_layout.addWidget(self.btn_project_nav)

        # 系统设置按钮
        self.btn_settings_nav = QPushButton("⚙️ 系统设置")
        self.btn_settings_nav.setCheckable(True)
        self.btn_settings_nav.setStyleSheet(menu_button_style)
        self.btn_settings_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_settings_nav.clicked.connect(lambda: self._switch_page(5))
        nav_layout.addWidget(self.btn_settings_nav)

        # 2026-06-16: 在导航栏右侧加版本标签，方便用户一眼判断运行的是新代码
        nav_layout.addStretch()
        self._nav_version_label = QLabel("v2026.06.16-FIX5  ✅ 修复折叠崩溃 + 导航按钮响应 + 去掉⋯按钮 + 工作区加高")
        self._nav_version_label.setStyleSheet(
            "color: #10B981; font-size: 11px; background: transparent; border: none; padding-right: 8px;"
        )
        nav_layout.addWidget(self._nav_version_label)

        layout.addWidget(nav_bar)

        # ── 页面堆叠窗口 ──
        self.page_stack = QStackedWidget()

        # 页面0：任务对话（Vue 前端，QtWebView2）
        self.chat_page = self._create_chat_page()
        self.page_stack.addWidget(self.chat_page)

        # 页面1：运行服务（服务状态 + 日志 + 管理按钮）
        self.home_page = self._create_home_page()
        self.page_stack.addWidget(self.home_page)

        # 页面2：部署维护
        self.deploy_page = self._create_deploy_page()
        self.page_stack.addWidget(self.deploy_page)

        # 页面3：软件更新
        self.update_page = self._create_update_page()
        self.page_stack.addWidget(self.update_page)

        # 2026-06-16 修复：补齐页面 4（项目管理）和 5（系统设置），
        # 之前这两个 index 在 QStackedWidget 中不存在，所以点了没反应
        self.project_page = self._create_project_page()
        self.page_stack.addWidget(self.project_page)
        self.settings_page = self._create_settings_page()
        self.page_stack.addWidget(self.settings_page)

        # 2026-06-16 新增：页面 6 = 对话历史（左侧 📜 历史按钮跳转到这里）
        self.history_page = self._create_history_page()
        self.page_stack.addWidget(self.history_page)

        # 2026-06-16: 边栏折叠快捷键（Ctrl+B 左，Ctrl+Shift+B 右）
        self._install_sidebar_shortcuts()

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

    def _install_sidebar_shortcuts(self):
        """2026-06-16：注册左右边栏折叠的键盘快捷键
        - Ctrl+B         折叠/展开左侧 workbuddy 边栏
        - Ctrl+Shift+B   折叠/展开右侧配置边栏
        """
        from PyQt6.QtGui import QShortcut, QKeySequence

        sc_left = QShortcut(QKeySequence("Ctrl+B"), self)
        sc_left.setContext(Qt.ShortcutContext.ApplicationShortcut)
        sc_left.activated.connect(self._toggle_workbuddy_collapse)

        sc_right = QShortcut(QKeySequence("Ctrl+Shift+B"), self)
        sc_right.setContext(Qt.ShortcutContext.ApplicationShortcut)
        sc_right.activated.connect(self._toggle_config_collapse)

        self._sc_collapse_left = sc_left
        self._sc_collapse_right = sc_right

    def _create_chat_page(self):
        """创建任务对话页面 - 2026-06-16 重写为 3 列布局

        克隆 web 改造前的 PyQt6 设计：右边栏（模型与配置）+ 对话窗口（中间）
        左边栏参考 workbuddy 风格：workspace + 项目 + 用户区
        所有功能的完整性：会话、消息、项目、模型、账户、设置
        """
        page = QWidget()
        page.setStyleSheet("background-color: #121212;")
        outer = QHBoxLayout(page)
        outer.setSpacing(0)
        outer.setContentsMargins(0, 0, 0, 0)

        # ── 左侧：workbuddy 风格边栏（220px）──
        self._workbuddy_panel = self._create_workbuddy_panel()
        outer.addWidget(self._workbuddy_panel)

        # ── 中间：对话窗（弹性扩展）──
        self._chat_center = self._create_chat_center()
        outer.addWidget(self._chat_center, 1)

        # ── 右侧：模型与配置面板（380px）──
        self._config_panel = self._create_config_panel()
        outer.addWidget(self._config_panel)

        # pywebview 桥接对象（保留兼容）
        self.bridge = BackendBridge()
        self.bridge._app_ref = self

        self._chat_vue_container = None
        self._chat_vue2_widget = None
        self._chat_sessions = []
        self._chat_current_session_id = None

        # 初始数据
        QTimer.singleShot(200, self._load_initial_data)

        return page

    # ── 左侧 workbuddy 风格边栏 ──

    def _apply_emoji_font(self, btn, size: int = 16):
        """2026-06-16 终极修复：把按钮的 emoji 文字渲染成 QPixmap，设为按钮 icon

        之前尝试过在 stylesheet 里指定 font-family、在 setFont() 里指定字体，
        在 Windows 上都还有 emoji 不显示的问题（Qt 的字体回退链没找到彩色 emoji 字体）。

        这次的方案最稳：
        1. 用 QFontDatabase.addApplicationFont() 直接加载 Windows 自带的
           Segoe UI Emoji 字体文件（seguiemj.ttf），强制定义字体
        2. 用 QPainter 把 emoji 文本画到 QPixmap 上（彩色 emoji 字体自带颜色）
        3. 把 pixmap 设为按钮的 icon，清空按钮文字
        这样完全不依赖 Qt 的字体回退机制，100% 能显示。
        """
        text = btn.text() or ""
        if not text:
            return  # 按钮没文字就没必要做 emoji icon 了

        # 1) 加载 Segoe UI Emoji 字体文件
        family = "Segoe UI Emoji"
        try:
            font_path = os.path.join(
                os.environ.get("WINDIR", "C:/Windows"), "Fonts", "seguiemj.ttf"
            )
            if os.path.exists(font_path):
                fid = QFontDatabase.addApplicationFont(font_path)
                if fid != -1:
                    fams = QFontDatabase.applicationFontFamilies(fid)
                    if fams:
                        family = fams[0]
        except Exception:
            pass

        # 2) 把 emoji 渲染到 pixmap
        # 关键：emoji 字体自带彩色 glyph，painter 拿到的就是彩色 bitmap
        pix_size = max(size + 8, 24)  # 给 emoji 一点边距
        pix = QPixmap(pix_size, pix_size)
        pix.fill(Qt.GlobalColor.transparent)

        painter = QPainter(pix)
        try:
            painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)
            painter.setRenderHint(QPainter.RenderHint.TextAntialiasing, True)
            # 关键 1：不要 SmoothPixmapTransform（会让彩色 emoji 模糊）
            # 关键 2：用 LoadedFont 渲染
            font = QFont(family)
            font.setPixelSize(int(pix_size * 0.85))
            font.setStyle(QFont.Style.StyleNormal)
            font.setBold(False)
            font.setItalic(False)
            painter.setFont(font)
            # 关键 3：彩色 emoji 自带颜色，pen 设什么色都不影响最终颜色
            # 但设个深色背景匹配的色更保险
            painter.setPen(Qt.GlobalColor.white)
            painter.drawText(pix.rect(), Qt.AlignmentFlag.AlignCenter, text)
        finally:
            painter.end()

        # 3) 设到按钮上
        btn.setIcon(QIcon(pix))
        btn.setIconSize(QSize(size, size))
        btn.setText("")  # 清空文字，只显示 icon
        # 2026-06-16 修复：之前用 regex 删 stylesheet 里的 color: / font-* 时，
        # `color\s*:\s*[^;}]+;?` 会误中 `background-color:` / `border-color:` / `outline-color:` 里的
        # `color:` 子串，把所有背景色、边框色都替换成了 transparent，
        # 导致 hover / pressed / checked 效果全部失效。
        # 正确做法：完全不动 stylesheet。文字已经清空，文字色和字体属性都没用了；
        # 背景/边框/hover/pressed/checked 必须保持原样。
        # 因此下面这堆 regex 全部删除。

    def _create_workbuddy_panel(self):
        """workbuddy 风格左侧边栏 - 支持折叠

        展开态（220px）：
          - 顶部：logo + workspace 切换
          - 中部：导航（对话 / 项目 / 历史 / 收藏 / 终端）
          - 中部：当前 workspace 的项目列表
          - 底部：用户区（设置入口）

        折叠态（50px）：
          - 垂直图标栏（⚡💬📁📜⭐💻👤），点击 ⚡ 展开
        """
        panel = QFrame()
        panel.setFixedWidth(220)
        panel.setStyleSheet("QFrame { background-color: #0F0F0F; border-right: 1px solid #1F1F1F; }")
        self._workbuddy_collapsed = False
        self._workbuddy_panel = panel
        layout = QVBoxLayout(panel)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # ── 顶部 logo + workspace + 折叠按钮（单行布局） ──
        header = QFrame()
        header.setStyleSheet("QFrame { background-color: #141414; border-bottom: 1px solid #1F1F1F; }")
        header.setMinimumHeight(70)
        header.setSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Fixed)
        header_layout = QVBoxLayout(header)
        header_layout.setContentsMargins(10, 8, 8, 8)
        header_layout.setSpacing(6)

        # 第 1 行：⚡ 云集智能 + 折叠按钮
        top_row = QHBoxLayout()
        top_row.setSpacing(6)
        logo = QLabel("⚡")
        logo.setStyleSheet("font-size: 18px; background: transparent; border: none;")
        top_row.addWidget(logo)
        brand = QLabel("云集智能")
        brand.setStyleSheet("color: #FFFFFF; font-size: 14px; font-weight: bold; background: transparent; border: none;")
        self._workbuddy_brand_lbl = brand
        top_row.addWidget(brand)
        top_row.addStretch()
        # 折叠按钮：参考 workbuddy 顶部工具按钮风格 - 方块带边框
        self._workbuddy_collapse_btn = QPushButton("‹")
        self._workbuddy_collapse_btn.setFixedSize(24, 22)
        self._workbuddy_collapse_btn.setToolTip("折叠边栏 (Ctrl+B)")
        self._workbuddy_collapse_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._workbuddy_collapse_btn.setStyleSheet("""
            QPushButton {
                background-color: #1F1F1F; color: #DDDDDD; border: 1px solid #2A2A2A;
                border-radius: 4px; font-size: 14px; font-weight: bold; padding: 0px;
            }
            QPushButton:hover { background-color: #2A2A2A; color: #FFFFFF; border-color: #3A3A3A; }
            QPushButton:pressed { background-color: #2563EB; color: #FFFFFF; border-color: #2563EB; }
        """)
        self._workbuddy_collapse_btn.clicked.connect(self._toggle_workbuddy_collapse)
        # 注意：不用 _apply_emoji_font。‹ 是基础符号，用默认字体就能显示，
        # 走 pixmap 路径反而会丢掉 toggle 时 setText("‹"/"›") 的文字更新
        top_row.addWidget(self._workbuddy_collapse_btn)
        header_layout.addLayout(top_row)

        # 第 2 行：工作区下拉（独立一整行）
        ws_row = QHBoxLayout()
        ws_row.setSpacing(0)
        ws_btn = QPushButton("📁 我的工作区  ▼")
        ws_btn.setFixedHeight(28)
        ws_btn.setStyleSheet("""
            QPushButton {
                background-color: #1A1A1A; color: #CCCCCC; border: 1px solid #2A2A2A;
                border-radius: 4px; padding: 0px 10px; font-size: 11px; text-align: left;
            }
            QPushButton:hover { background-color: #252525; color: #FFFFFF; border-color: #3A3A3A; }
        """)
        ws_btn.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
        self._workbuddy_ws_btn = ws_btn
        ws_row.addWidget(ws_btn, 1)
        header_layout.addLayout(ws_row)
        layout.addWidget(header)
        self._workbuddy_header = header

        # ── 中部：导航条目 ──
        nav = QFrame()
        nav.setStyleSheet("QFrame { background-color: transparent; border: none; }")
        nav_layout = QVBoxLayout(nav)
        nav_layout.setContentsMargins(8, 8, 8, 4)
        nav_layout.setSpacing(2)

        nav_items = [
            ("💬", "对话", "session", True),       # 切到当前会话区
            ("📁", "项目", "project", True),
            ("📜", "历史", "history", True),
            ("⭐", "收藏", "favorite", False),       # 暂未实现
            ("💻", "终端", "terminal", False),
        ]
        self._workbuddy_nav_btns = {}  # 保存引用方便后续状态切换
        for icon, label, kind, enabled in nav_items:
            btn = QPushButton(f"  {icon}  {label}")
            btn.setEnabled(enabled)
            btn.setStyleSheet("""
                QPushButton {
                    background-color: transparent; color: #999; border: none;
                    border-radius: 4px; padding: 7px 8px; font-size: 12px; text-align: left;
                }
                QPushButton:hover { background-color: #1F1F1F; color: #FFFFFF; }
                QPushButton:checked { background-color: #1E3A8A; color: #FFFFFF; }
                QPushButton:disabled { color: #555; }
            """)
            btn.setCheckable(True)
            btn.setProperty("nav_kind", kind)
            if enabled:
                # 2026-06-16: 修复"点了没反应" - 真正绑定 click 事件
                if kind == "session":
                    btn.clicked.connect(lambda: (self._refresh_chat_session_list(), self._update_nav_active("session")))
                elif kind == "project":
                    btn.clicked.connect(lambda: (self._switch_page(4), self._update_nav_active("project")))
                elif kind == "history":
                    btn.clicked.connect(lambda: (self._switch_page(6), self._update_nav_active("history")))  # 2026-06-16: 跳到真正的对话历史页（page 6）
            if kind == "session":
                btn.setChecked(True)
            self._workbuddy_nav_btns[kind] = btn
            nav_layout.addWidget(btn)
        nav_layout.addStretch()
        layout.addWidget(nav)

        # ── 项目列表区 ──
        proj_frame = QFrame()
        proj_frame.setStyleSheet("QFrame { background-color: transparent; border: none; }")
        proj_layout = QVBoxLayout(proj_frame)
        proj_layout.setContentsMargins(8, 4, 8, 4)
        proj_layout.setSpacing(4)

        proj_title = QLabel("我的项目")
        proj_title.setStyleSheet("color: #888; font-size: 10px; padding: 4px 8px; background: transparent; border: none;")
        proj_layout.addWidget(proj_title)

        self._workbuddy_proj_list = QListWidget()
        self._workbuddy_proj_list.setStyleSheet("""
            QListWidget { background-color: transparent; border: none; color: #CCC; font-size: 11px; }
            QListWidget::item { padding: 6px 8px; border-radius: 4px; }
            QListWidget::item:hover { background-color: #1F1F1F; color: #FFFFFF; }
            QListWidget::item:selected { background-color: #1E3A8A; color: #FFFFFF; }
        """)
        self._workbuddy_proj_list.itemClicked.connect(self._workbuddy_switch_project)
        # 2026-06-16: 右键菜单（重命名/删除/打开文件夹/复制路径）
        self._workbuddy_proj_list.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self._workbuddy_proj_list.customContextMenuRequested.connect(self._workbuddy_project_context_menu)
        proj_layout.addWidget(self._workbuddy_proj_list, 1)

        proj_actions = QHBoxLayout()
        proj_actions.setSpacing(4)
        new_proj_btn = QPushButton("+ 新建")
        new_proj_btn.setStyleSheet("""
            QPushButton {
                background-color: #1F1F1F; color: #BBB; border: 1px solid #2A2A2A;
                border-radius: 4px; padding: 4px 8px; font-size: 10px;
            }
            QPushButton:hover { background-color: #2563EB; color: #FFFFFF; border-color: #2563EB; }
        """)
        new_proj_btn.clicked.connect(self._project_create_dialog)
        proj_actions.addWidget(new_proj_btn, 1)
        # 2026-06-16: 去掉原来的"⋯"按钮（与"+ 新建"重复且不直观）

        proj_layout.addLayout(proj_actions)
        layout.addWidget(proj_frame, 1)

        # ── 底部用户区 ──
        bottom = QFrame()
        bottom.setStyleSheet("QFrame { background-color: #141414; border-top: 1px solid #1F1F1F; }")
        bottom.setFixedHeight(44)
        bottom_layout = QHBoxLayout(bottom)
        bottom_layout.setContentsMargins(10, 6, 8, 6)
        bottom_layout.setSpacing(8)

        avatar = QLabel("👤")
        avatar.setStyleSheet("font-size: 18px; background: transparent; border: none;")
        avatar.setFixedWidth(24)
        bottom_layout.addWidget(avatar)

        user_info = QVBoxLayout()
        user_info.setSpacing(0)
        user_name = QLabel("本地用户")
        user_name.setStyleSheet("color: #FFFFFF; font-size: 11px; font-weight: bold; background: transparent; border: none;")
        user_info.addWidget(user_name)
        user_status = QLabel("● 在线")
        user_status.setStyleSheet("color: #10B981; font-size: 9px; background: transparent; border: none;")
        user_info.addWidget(user_status)
        bottom_layout.addLayout(user_info, 1)

        settings_btn = QPushButton("⚙")
        settings_btn.setStyleSheet("""
            QPushButton {
                background-color: transparent; color: #888; border: none;
                border-radius: 4px; padding: 4px 6px; font-size: 14px;
            }
            QPushButton:hover { background-color: #252525; color: #FFFFFF; }
        """)
        settings_btn.clicked.connect(lambda: self._switch_page(5))  # 跳到系统设置
        bottom_layout.addWidget(settings_btn)

        layout.addWidget(bottom)

        # 2026-06-16: 保存内嵌部件引用，折叠时按需隐藏
        self._workbuddy_nav = nav
        self._workbuddy_proj_frame = proj_frame
        self._workbuddy_bottom = bottom

        # ── 折叠态：垂直图标栏 ──
        rail = QFrame()
        rail.setStyleSheet("QFrame { background-color: #0A0A0A; border: none; }")
        rail_layout = QVBoxLayout(rail)
        rail_layout.setContentsMargins(4, 12, 4, 12)
        rail_layout.setSpacing(4)
        rail.setVisible(False)
        self._workbuddy_rail = rail
        self._workbuddy_rail_items = []
        rail_icons = [
            ("›", "展开边栏 (Ctrl+B)", None),  # 第一个 = 展开按钮
            ("💬", "对话", None),
            ("📁", "项目", lambda: self._switch_page(4)),
            ("📜", "历史", None),
            ("⭐", "收藏", None),
            ("💻", "终端", None),
        ]
        for i, (icon, tip, slot) in enumerate(rail_icons):
            b = QPushButton(icon)
            b.setFixedSize(40, 36)
            b.setToolTip(tip)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            if i == 0:
                # 展开按钮：醒目蓝色，去掉边框
                b.setStyleSheet("""
                    QPushButton {
                        background-color: #1E3A8A; color: #FFFFFF; border: none;
                        border-radius: 8px;
                    }
                    QPushButton:hover { background-color: #3B82F6; }
                    QPushButton:pressed { background-color: #1D4ED8; }
                """)
            else:
                # 2026-06-16 优化：默认透明，去掉边框，hover/pressed 用对比度强的颜色
                # 之前 #2A2A2A 和 rail 背景 #101010 太接近，看不出效果
                # 现在 hover 用中灰 #374151，pressed 用蓝色 #1E3A8A
                b.setStyleSheet("""
                    QPushButton {
                        background-color: transparent; color: #FFFFFF; border: none;
                        border-radius: 8px;
                    }
                    QPushButton:hover { background-color: #374151; }
                    QPushButton:pressed { background-color: #1E3A8A; }
                    QPushButton:disabled { color: #555; }
                """)
            if slot:
                b.clicked.connect(slot)
            elif i == 0:
                # 2026-06-16 修复：展开按钮的 click 直接绑定到 _toggle_workbuddy_collapse
                # 之前依赖 toggle 函数里用 setIconText 重新绑定，但 QPushButton 没有 setIconText
                # 直接 setIconText 会抛 AttributeError 导致闪退
                b.clicked.connect(self._toggle_workbuddy_collapse)
            else:
                b.setEnabled(icon in ("💬", "📜"))  # 仅有意义的可点击按钮
            # 2026-06-16 终极修复：迷你栏按钮 emoji 不显示
            # 走 QPixmap 路径：load Segoe UI Emoji 字体 → QPainter 画到 pixmap → setIcon
            # size=22 让 24x24 icon 在 40x36 按钮里视觉舒服
            self._apply_emoji_font(b, size=22)
            rail_layout.addWidget(b)
            self._workbuddy_rail_items.append(b)
        # 底部用户头像
        rail_layout.addStretch()
        user_avatar = QPushButton("👤")
        user_avatar.setFixedSize(40, 36)
        user_avatar.setToolTip("用户设置")
        user_avatar.setCursor(Qt.CursorShape.PointingHandCursor)
        # 2026-06-16 优化：和上面的非展开按钮保持一致
        user_avatar.setStyleSheet("""
            QPushButton {
                background-color: transparent; color: #FFFFFF; border: none;
                border-radius: 8px;
            }
            QPushButton:hover { background-color: #374151; }
            QPushButton:pressed { background-color: #1E3A8A; }
        """)
        user_avatar.clicked.connect(lambda: self._switch_page(5))
        # 2026-06-16 终极修复：用户头像走 emoji pixmap 路径
        self._apply_emoji_font(user_avatar, size=20)
        rail_layout.addWidget(user_avatar)
        self._workbuddy_rail_items.append(user_avatar)

        layout.addWidget(rail)

        return panel

    def _animate_panel_width(self, panel, from_w: int, to_w: int, duration: int = 180, on_finish=None):
        """2026-06-16：折叠/展开的 180ms 宽度动画（修复崩溃：单动画 + 截断旧动画）"""
        if panel is None:
            return
        # 截断上一次未完成的动画，避免 _on_done 误触发
        if hasattr(panel, "_anim_min") and panel._anim_min is not None:
            try:
                panel._anim_min.stop()
                panel._anim_max.stop()
            except RuntimeError:
                pass
        anim = QPropertyAnimation(panel, b"minimumWidth", panel)
        anim.setDuration(duration)
        anim.setStartValue(from_w)
        anim.setEndValue(to_w)
        anim.setEasingCurve(QEasingCurve.Type.InOutCubic)
        anim2 = QPropertyAnimation(panel, b"maximumWidth", panel)
        anim2.setDuration(duration)
        anim2.setStartValue(from_w)
        anim2.setEndValue(to_w)
        anim2.setEasingCurve(QEasingCurve.Type.InOutCubic)
        if on_finish:
            anim.finished.connect(on_finish)
        anim.start()
        anim2.start()
        panel._anim_min = anim
        panel._anim_max = anim2

    def _toggle_workbuddy_collapse(self):
        """折叠/展开 workbuddy 边栏（220px ↔ 50px）"""
        if not hasattr(self, "_workbuddy_panel") or self._workbuddy_panel is None:
            return
        if not self._workbuddy_collapsed:
            # 折叠：先动画宽度，结束后切换内部 widget 可见性
            start_w = self._workbuddy_panel.width() or 220
            def _on_done():
                # 用 try 包裹，避免动画回调时主窗口已被销毁
                try:
                    if not hasattr(self, "_workbuddy_panel") or self._workbuddy_panel is None:
                        return
                    for w in (self._workbuddy_header, self._workbuddy_nav,
                              self._workbuddy_proj_frame, self._workbuddy_bottom):
                        if w is not None:
                            w.setVisible(False)
                    self._workbuddy_rail.setVisible(True)
                    self._workbuddy_collapse_btn.setText("›")
                    self._workbuddy_collapse_btn.setToolTip("展开边栏 (Ctrl+B)")
                except RuntimeError:
                    pass
            self._animate_panel_width(self._workbuddy_panel, start_w, 50, on_finish=_on_done)
            self._workbuddy_collapsed = True
        else:
            # 展开
            start_w = self._workbuddy_panel.width() or 50
            for w in (self._workbuddy_header, self._workbuddy_nav,
                      self._workbuddy_proj_frame, self._workbuddy_bottom):
                if w is not None:
                    w.setVisible(True)
            self._workbuddy_rail.setVisible(False)
            self._workbuddy_collapse_btn.setText("‹")
            self._workbuddy_collapse_btn.setToolTip("折叠边栏 (Ctrl+B)")
            def _on_done():
                try:
                    if not hasattr(self, "_workbuddy_panel") or self._workbuddy_panel is None:
                        return
                    self._workbuddy_panel.setMinimumWidth(220)
                    self._workbuddy_panel.setMaximumWidth(220)
                except RuntimeError:
                    pass
            self._animate_panel_width(self._workbuddy_panel, start_w, 220, on_finish=_on_done)
            self._workbuddy_collapsed = False
        # 2026-06-16 修复：删掉之前的 for 循环
        # 原因：循环里调用 btn.setIconText(icon)，但 QPushButton 没有 setIconText 方法
        # （setIconText 是 QAction 的方法），导致 AttributeError 闪退
        # 展开按钮的 click 已在 rail 创建时一次性绑定，无需每次 toggle 都重新绑

    def _workbuddy_switch_project(self, item):
        """workbuddy 边栏：点击项目切换"""
        project_id = item.data(Qt.ItemDataRole.UserRole)
        if not project_id:
            return
        try:
            result = self.bridge.switchProject(project_id)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                self._append_log(f"已切换到项目: {item.text()}", "#4CAF50")
                self._refresh_chat_session_list()
        except Exception as e:
            self._append_log(f"切换项目失败: {e}", "#F44336")

    def _workbuddy_project_context_menu(self, pos):
        """workbuddy 边栏：项目列表右键菜单

        选项：📝 重命名 / 📂 打开文件夹 / 📋 复制路径 / 🗑 删除
        """
        item = self._workbuddy_proj_list.itemAt(pos)
        if not item:
            return
        project_id = item.data(Qt.ItemDataRole.UserRole)
        project_path = item.data(Qt.ItemDataRole.UserRole + 1) or ""
        if not project_id:
            return
        from PyQt6.QtWidgets import QMenu
        menu = QMenu(self._workbuddy_proj_list)
        menu.setStyleSheet("""
            QMenu {
                background-color: #1A1A1A; color: #E0E0E0;
                border: 1px solid #2A2A2A; border-radius: 4px;
                padding: 4px;
            }
            QMenu::item { padding: 6px 18px; border-radius: 3px; }
            QMenu::item:selected { background-color: #2563EB; color: #FFFFFF; }
            QMenu::separator { height: 1px; background: #2A2A2A; margin: 4px 0; }
        """)
        a_rename = menu.addAction("📝 重命名")
        a_open = menu.addAction("📂 打开文件夹")
        menu.addSeparator()
        a_copy = menu.addAction("📋 复制路径")
        a_del = menu.addAction("🗑 删除")
        a_del.setText("🗑  删除项目")
        a_del.setShortcut("Del")
        chosen = menu.exec(self._workbuddy_proj_list.mapToGlobal(pos))
        if chosen is None:
            return
        if chosen is a_rename:
            self._workbuddy_rename_project(project_id, item)
        elif chosen is a_open:
            self._workbuddy_open_in_explorer(project_path)
        elif chosen is a_copy:
            from PyQt6.QtGui import QGuiApplication
            QGuiApplication.clipboard().setText(project_path)
            self._append_log(f"已复制路径: {project_path}", "#4CAF50")
        elif chosen is a_del:
            self._workbuddy_delete_project(project_id, item.text())

    def _workbuddy_rename_project(self, project_id, item):
        """重命名项目"""
        old_name = item.text().replace("📁 ", "")
        from PyQt6.QtWidgets import QInputDialog
        new_name, ok = QInputDialog.getText(self, "重命名项目", "新名称:", text=old_name)
        if not ok or not new_name.strip() or new_name.strip() == old_name:
            return
        try:
            payload = json.dumps({"project_id": project_id, "name": new_name.strip()})
            result = self.bridge.renameProject(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                self._append_log(f"✓ 已重命名: {old_name} → {new_name.strip()}", "#10B981")
                self._refresh_projects_for_sidebar()
            else:
                self._append_log(f"重命名失败: {data}", "#F44336")
        except Exception as e:
            self._append_log(f"重命名失败: {e}", "#F44336")

    def _workbuddy_open_in_explorer(self, path):
        """在资源管理器中打开"""
        if not path:
            self._append_log("⚠ 没有可用的项目路径", "#FF9800")
            return
        try:
            self.bridge.openInExplorer(path)
            self._append_log(f"已打开: {path}", "#4CAF50")
        except Exception as e:
            self._append_log(f"打开失败: {e}", "#F44336")

    def _workbuddy_delete_project(self, project_id, display_name):
        """删除项目（带二次确认）"""
        from PyQt6.QtWidgets import QMessageBox
        reply = QMessageBox.question(
            self, "确认删除",
            f"确定要删除项目「{display_name}」吗？\n\n注意：项目文件本身不会被删除，只从工作区移除。",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )
        if reply != QMessageBox.StandardButton.Yes:
            return
        try:
            payload = json.dumps({"project_id": project_id})
            result = self.bridge.deleteProject(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                self._append_log(f"✓ 已删除项目: {display_name}", "#10B981")
                self._refresh_projects_for_sidebar()
            else:
                self._append_log(f"删除失败: {data}", "#F44336")
        except Exception as e:
            self._append_log(f"删除失败: {e}", "#F44336")

    # ── 中间对话窗（toolbar + messages + composer）──

    def _create_chat_center(self):
        """中间对话窗 - 克隆自 web 改造前的 PyQt6 设计"""
        container = QFrame()
        container.setStyleSheet("QFrame { background-color: #121212; }")
        layout = QVBoxLayout(container)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # ── 顶部 toolbar ──
        toolbar = QFrame()
        toolbar.setFixedHeight(48)
        toolbar.setStyleSheet("QFrame { background-color: #1A1A1A; border-bottom: 1px solid #2A2A2A; }")
        tb_layout = QHBoxLayout(toolbar)
        tb_layout.setContentsMargins(12, 8, 12, 8)
        tb_layout.setSpacing(8)

        self._chat_current_title = QLabel("选择左侧的对话，或点击「+ 新建对话」")
        self._chat_current_title.setStyleSheet("color: #E0E0E0; font-size: 13px; font-weight: bold; background: transparent; border: none;")
        tb_layout.addWidget(self._chat_current_title)
        tb_layout.addStretch()

        def make_btn(text, color, hover, slot, disabled=False):
            b = QPushButton(text)
            b.setStyleSheet(f"""
                QPushButton {{
                    background-color: {color}; color: #FFFFFF; border: none; border-radius: 5px;
                    padding: 5px 12px; font-size: 11px;
                }}
                QPushButton:hover {{ background-color: {hover}; }}
                QPushButton:disabled {{ background-color: #333; color: #777; }}
            """)
            b.clicked.connect(slot)
            b.setEnabled(not disabled)
            return b

        self._btn_open_project = make_btn("📂 打开项目", "#374151", "#4B5563", self._chat_choose_workspace)
        self._btn_new_session = make_btn("🆕 新会话", "#2563EB", "#1D4ED8", self._chat_new_session)
        self._btn_stop_msg = make_btn("⏹ 停止", "#DC2626", "#B91C1C", self._chat_stop_message, disabled=True)
        self._btn_toggle_panel = make_btn("⇄ 面板", "#374151", "#4B5563", self._chat_toggle_panel)

        tb_layout.addWidget(self._btn_open_project)
        tb_layout.addWidget(self._btn_new_session)
        tb_layout.addWidget(self._btn_stop_msg)
        tb_layout.addWidget(self._btn_toggle_panel)
        layout.addWidget(toolbar)

        # ── 消息滚动区 ──
        self._chat_messages_area = QTextEdit()
        self._chat_messages_area.setReadOnly(True)
        self._chat_messages_area.setStyleSheet("""
            QTextEdit {
                background-color: #121212; color: #E0E0E0;
                border: none; font-size: 13px; padding: 16px;
            }
        """)
        layout.addWidget(self._chat_messages_area, 1)
        self._set_welcome_message()

        # ── 输入区（composer）──
        composer = QFrame()
        composer.setStyleSheet("QFrame { background-color: #1A1A1A; border-top: 1px solid #2A2A2A; }")
        cp_layout = QVBoxLayout(composer)
        cp_layout.setContentsMargins(12, 10, 12, 10)
        cp_layout.setSpacing(6)

        self._chat_input = QTextEdit()
        self._chat_input.setFixedHeight(90)
        self._chat_input.setPlaceholderText("输入编码任务（Enter 发送，Shift+Enter 换行）")
        self._chat_input.setStyleSheet("""
            QTextEdit {
                background-color: #0E0E0E; color: #E0E0E0;
                border: 1px solid #333; border-radius: 6px;
                padding: 8px; font-size: 13px;
            }
        """)
        cp_layout.addWidget(self._chat_input)

        foot_row = QHBoxLayout()
        foot_row.setSpacing(8)
        self._composer_status = QLabel("就绪")
        self._composer_status.setStyleSheet("color: #888; font-size: 11px; background: transparent; border: none;")
        foot_row.addWidget(self._composer_status)
        foot_row.addStretch()

        new_btn = QPushButton("🔄 清空")
        new_btn.setStyleSheet("""
            QPushButton {
                background-color: #374151; color: #FFFFFF; border: none; border-radius: 5px;
                padding: 5px 12px; font-size: 11px;
            }
            QPushButton:hover { background-color: #4B5563; }
        """)
        new_btn.clicked.connect(self._chat_clear_messages)
        foot_row.addWidget(new_btn)

        send_btn = QPushButton("🚀 发送任务")
        send_btn.setStyleSheet("""
            QPushButton {
                background-color: #10B981; color: #FFFFFF; border: none; border-radius: 5px;
                padding: 5px 14px; font-size: 11px; font-weight: bold;
            }
            QPushButton:hover { background-color: #059669; }
        """)
        send_btn.clicked.connect(self._chat_send_message)
        foot_row.addWidget(send_btn)
        cp_layout.addLayout(foot_row)
        layout.addWidget(composer)

        # Enter 发送（保留兼容）
        self._chat_input.installEventFilter(self)

        return container

    def _set_welcome_message(self):
        """对话窗默认欢迎信息"""
        if not hasattr(self, "_chat_messages_area"):
            return
        self._chat_messages_area.setHtml("""
            <div style="padding: 24px; text-align: center; color: #888;">
                <div style="font-size: 48px; margin-bottom: 12px;">💬</div>
                <div style="font-size: 18px; color: #FFFFFF; font-weight: bold; margin-bottom: 8px;">
                    欢迎使用云集智能编程工作站
                </div>
                <div style="font-size: 13px; color: #BBB; line-height: 1.8; margin-bottom: 12px;">
                    这是你的 AI 编程工作台。你可以：<br/>
                    · 在左侧创建或选择项目 / 对话<br/>
                    · 在下方输入框描述任务，让 AI 帮你写代码、调试、重构<br/>
                    · 右侧选择运行模式（☁️云端 / 🔗API / 🦙Ollama）和配置
                </div>
                <div style="font-size: 11px; color: #666; margin-top: 16px;">
                    提示：Enter 发送 · Shift+Enter 换行 · 顶部"打开项目"开始第一次会话
                </div>
            </div>
        """)

    # ── 右侧配置面板（克隆自 web 改造前的 PyQt6 设计）──

    def _create_config_panel(self):
        """右侧配置面板 - 克隆自 web 改造前的 Vue panel

        结构：
          - 面板头：模型与配置 + 模式切换（☁️云端 / 🔗API / 🦙Ollama）+ 折叠按钮
          - 3 种模式的具体 UI（QStackedWidget 切换）
          - 对话设置：自动授权 / 项目管理 / 称谓 / 保存
          - 折叠态（50px）：仅显示模式图标
        """
        panel = QFrame()
        panel.setFixedWidth(400)
        panel.setStyleSheet("QFrame { background-color: #161616; border-left: 1px solid #1F1F1F; }")
        self._config_collapsed = False
        self._config_panel_frame = panel
        layout = QVBoxLayout(panel)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # ── 面板头 + 模式切换 ──
        header = QFrame()
        header.setStyleSheet("QFrame { background-color: #1A1A1A; border-bottom: 1px solid #2A2A2A; }")
        header_layout = QVBoxLayout(header)
        header_layout.setContentsMargins(14, 12, 14, 12)
        header_layout.setSpacing(8)

        title_row = QHBoxLayout()
        title_row.setSpacing(6)
        title_lbl = QLabel("⚙ 模型与配置")
        title_lbl.setStyleSheet("color: #FFFFFF; font-size: 14px; font-weight: bold; background: transparent; border: none;")
        title_row.addWidget(title_lbl)
        title_row.addStretch()
        # 折叠按钮：与左侧风格一致的方块按钮
        self._config_collapse_btn = QPushButton("›")
        self._config_collapse_btn.setFixedSize(26, 24)
        self._config_collapse_btn.setToolTip("折叠配置面板 (Ctrl+Shift+B)")
        self._config_collapse_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._config_collapse_btn.setStyleSheet("""
            QPushButton {
                background-color: #1F1F1F; color: #CCCCCC; border: 1px solid #2A2A2A;
                border-radius: 4px; font-size: 16px; font-weight: bold; padding: 0px;
            }
            QPushButton:hover { background-color: #2A2A2A; color: #FFFFFF; border-color: #3A3A3A; }
            QPushButton:pressed { background-color: #2563EB; color: #FFFFFF; border-color: #2563EB; }
        """)
        self._config_collapse_btn.clicked.connect(self._toggle_config_collapse)
        # 同样不用 _apply_emoji_font（› 是基础符号）
        title_row.addWidget(self._config_collapse_btn)
        header_layout.addLayout(title_row)

        mode_row = QHBoxLayout()
        mode_row.setSpacing(4)
        mode_btn_style = """
            QPushButton {
                background-color: #252525; color: #AAA; border: 1px solid #2F2F2F;
                border-radius: 4px; padding: 5px 8px; font-size: 11px;
            }
            QPushButton:hover { background-color: #2F2F2F; color: #FFF; }
            QPushButton:checked { background-color: #2563EB; color: #FFFFFF; border-color: #2563EB; }
        """
        self._run_mode = "ollama"  # 默认值
        self._mode_btns = {}
        for key, label in [("cloud", "☁️ 云端"), ("api", "🔗 API"), ("ollama", "🦙 Ollama")]:
            btn = QPushButton(label)
            btn.setCheckable(True)
            btn.setStyleSheet(mode_btn_style)
            if key == "ollama":
                btn.setChecked(True)
            btn.clicked.connect(lambda _, k=key: self._switch_run_mode(k))
            mode_row.addWidget(btn, 1)
            self._mode_btns[key] = btn
        header_layout.addLayout(mode_row)
        layout.addWidget(header)

        # ── 3 种模式的内容（QStackedWidget）──
        self._mode_stack = QStackedWidget()
        self._mode_stack.setStyleSheet("QStackedWidget { background-color: #161616; border: none; }")

        # 内容区滚动支持
        from PyQt6.QtWidgets import QScrollArea
        scroll_style = """
            QScrollArea { background-color: #161616; border: none; }
            QScrollBar:vertical { background: #1A1A1A; width: 8px; }
            QScrollBar::handle:vertical { background: #3A3A3A; border-radius: 4px; }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }
        """

        # 1. 云端模式
        cloud_scroll = QScrollArea()
        cloud_scroll.setStyleSheet(scroll_style)
        cloud_scroll.setWidgetResizable(True)
        cloud_widget = self._build_cloud_mode()
        cloud_scroll.setWidget(cloud_widget)
        self._mode_stack.addWidget(cloud_scroll)

        # 2. API 模式
        api_scroll = QScrollArea()
        api_scroll.setStyleSheet(scroll_style)
        api_widget = self._build_api_mode()
        api_scroll.setWidget(api_widget)
        self._mode_stack.addWidget(api_scroll)

        # 3. Ollama 模式
        ollama_scroll = QScrollArea()
        ollama_scroll.setStyleSheet(scroll_style)
        ollama_widget = self._build_ollama_mode()
        ollama_scroll.setWidget(ollama_widget)
        self._mode_stack.addWidget(ollama_scroll)

        # 2026-06-16: 记录 3 个滚动区域 + 滚动位置记忆
        self._mode_scroll_areas = {
            "cloud": cloud_scroll, "api": api_scroll, "ollama": ollama_scroll,
        }
        self._mode_scroll_pos = {"cloud": 0, "api": 0, "ollama": 0}
        for k, sa in self._mode_scroll_areas.items():
            sb = sa.verticalScrollBar()
            sb.valueChanged.connect(lambda v, kk=k: self._mode_scroll_pos.__setitem__(kk, v))

        layout.addWidget(self._mode_stack, 1)

        # ── 对话设置（始终在底部）──
        settings_widget = self._build_dialog_settings()
        layout.addWidget(settings_widget)

        # 保存内嵌部件引用供折叠使用
        self._config_header = header
        self._config_settings_widget = settings_widget
        self._config_mode_stack = self._mode_stack

        # ── 折叠态：垂直模式图标 ──
        rail = QFrame()
        rail.setStyleSheet("QFrame { background-color: #101010; border: none; }")
        rail_layout = QVBoxLayout(rail)
        rail_layout.setContentsMargins(4, 12, 4, 12)
        rail_layout.setSpacing(6)
        rail.setVisible(False)
        self._config_rail = rail
        self._config_rail_items = []
        rail_mode_icons = [
            ("☁️", "cloud", "云端"),
            ("🔗️", "api", "API"),
            ("🦙", "ollama", "Ollama"),
        ]
        for icon, key, tip in rail_mode_icons:
            b = QPushButton(icon)
            b.setFixedSize(40, 36)
            b.setToolTip(tip)
            b.setCheckable(True)
            b.setCursor(Qt.CursorShape.PointingHandCursor)
            # 2026-06-16 优化：默认透明，hover/pressed 用对比度强的颜色
            # 之前 regex bug 把 background-color 也改成了 transparent，所以看起来没效果
            b.setStyleSheet("""
                QPushButton {
                    background-color: transparent; color: #FFFFFF; border: none;
                    border-radius: 8px;
                }
                QPushButton:hover { background-color: #374151; }
                QPushButton:pressed { background-color: #1E3A8A; }
                QPushButton:checked {
                    background-color: #1E3A8A;
                }
                QPushButton:checked:hover { background-color: #3B82F6; }
            """)
            if key == "ollama":
                b.setChecked(True)
            b.clicked.connect(lambda _, k=key: self._switch_run_mode(k))
            # 2026-06-16 终极修复：右侧迷你栏图标不显示，走 QPixmap 路径
            self._apply_emoji_font(b, size=22)
            rail_layout.addWidget(b)
            self._config_rail_items.append(b)
        # 分隔条
        sep = QFrame()
        sep.setFrameShape(QFrame.Shape.HLine)
        sep.setStyleSheet("QFrame { background-color: #2A2A2A; border: none; max-height: 1px; min-height: 1px; }")
        rail_layout.addSpacing(8)
        rail_layout.addWidget(sep)
        rail_layout.addSpacing(8)
        rail_layout.addStretch()
        expand_btn = QPushButton("‹")
        expand_btn.setFixedSize(40, 36)
        expand_btn.setToolTip("展开配置面板 (Ctrl+Shift+B)")
        expand_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        expand_btn.setStyleSheet("""
            QPushButton {
                background-color: #1E3A8A; color: #FFFFFF; border: none;
                border-radius: 8px;
            }
            QPushButton:hover { background-color: #3B82F6; }
            QPushButton:pressed { background-color: #1D4ED8; }
        """)
        expand_btn.clicked.connect(self._toggle_config_collapse)
        # 不用 _apply_emoji_font（‹ 是基础符号）
        rail_layout.addWidget(expand_btn)
        self._config_rail_items.append(expand_btn)

        layout.addWidget(rail)

        return panel

    def _toggle_config_collapse(self):
        """折叠/展开右侧配置面板（400px ↔ 50px）"""
        if not hasattr(self, "_config_panel_frame") or self._config_panel_frame is None:
            return
        if not self._config_collapsed:
            start_w = self._config_panel_frame.width() or 400
            def _on_done():
                try:
                    for w in (self._config_header, self._config_mode_stack, self._config_settings_widget):
                        if w is not None:
                            w.setVisible(False)
                    self._config_rail.setVisible(True)
                    self._config_collapse_btn.setText("‹")
                    self._config_collapse_btn.setToolTip("展开配置面板 (Ctrl+Shift+B)")
                except RuntimeError:
                    pass
            self._animate_panel_width(self._config_panel_frame, start_w, 50, on_finish=_on_done)
            self._config_collapsed = True
        else:
            start_w = self._config_panel_frame.width() or 50
            for w in (self._config_header, self._config_mode_stack, self._config_settings_widget):
                if w is not None:
                    w.setVisible(True)
            self._config_rail.setVisible(False)
            self._config_collapse_btn.setText("›")
            self._config_collapse_btn.setToolTip("折叠配置面板 (Ctrl+Shift+B)")
            def _on_done():
                try:
                    self._config_panel_frame.setMinimumWidth(400)
                    self._config_panel_frame.setMaximumWidth(400)
                except RuntimeError:
                    pass
            self._animate_panel_width(self._config_panel_frame, start_w, 400, on_finish=_on_done)
            self._config_collapsed = False

    def _switch_run_mode(self, mode: str):
        """切换右侧面板的 3 种模式（带滚动位置记忆）"""
        self._run_mode = mode
        for k, btn in self._mode_btns.items():
            btn.setChecked(k == mode)
        # rail 模式按钮也同步选中
        if hasattr(self, "_config_rail_items"):
            mode_to_rail_idx = {"cloud": 0, "api": 1, "ollama": 2}
            idx = mode_to_rail_idx.get(mode, 2)
            for i, btn in enumerate(self._config_rail_items[:3]):
                btn.setChecked(i == idx)
        if mode == "cloud":
            self._mode_stack.setCurrentIndex(0)
        elif mode == "api":
            self._mode_stack.setCurrentIndex(1)
        elif mode == "ollama":
            self._mode_stack.setCurrentIndex(2)
            QTimer.singleShot(200, self._ollama_check_status)
        # 恢复滚动位置
        if hasattr(self, "_mode_scroll_areas") and mode in self._mode_scroll_areas:
            QTimer.singleShot(50, lambda m=mode: self._mode_scroll_areas[m].verticalScrollBar().setValue(self._mode_scroll_pos.get(m, 0)))

    def _build_cloud_mode(self):
        """☁️ 云端模式面板"""
        w = QWidget()
        w.setStyleSheet("background-color: #161616;")
        layout = QVBoxLayout(w)
        layout.setContentsMargins(14, 12, 14, 12)
        layout.setSpacing(10)

        layout.addWidget(self._make_section_title("🔑 API Key"))
        self._cloud_api_key = QLineEdit()
        self._cloud_api_key.setPlaceholderText("输入 OpenRouter / Anthropic API Key")
        self._cloud_api_key.setEchoMode(QLineEdit.EchoMode.Password)
        self._cloud_api_key.setStyleSheet(self._input_style())
        layout.addWidget(self._cloud_api_key)

        layout.addWidget(self._make_section_title("云端模型（固定）"))
        model_btn = QPushButton("openrouter/auto  ★ 工具")
        model_btn.setStyleSheet("""
            QPushButton {
                background-color: #1E3A8A; color: #FFFFFF; border: none;
                border-radius: 5px; padding: 8px 10px; font-size: 12px; text-align: left;
            }
        """)
        model_btn.setEnabled(False)
        layout.addWidget(model_btn)

        layout.addWidget(self._make_section_title("模型独立配置"))
        self._cloud_lang = self._make_combo("语言", ["中文", "English"], "中文")
        layout.addLayout(self._cloud_lang)
        self._cloud_temp = self._make_field("Temperature", "0.6~0.8 推荐", "0.7")
        layout.addLayout(self._cloud_temp)
        self._cloud_max_tokens = self._make_field("Max Tokens", "", "4096")
        layout.addLayout(self._cloud_max_tokens)

        self._cloud_prompt = QTextEdit()
        self._cloud_prompt.setPlaceholderText("系统提示词（可选）")
        self._cloud_prompt.setFixedHeight(60)
        self._cloud_prompt.setStyleSheet(self._input_style())
        layout.addWidget(self._cloud_prompt)

        info = QLabel("💡 云端模式无需本地算力，工具调用 ★ 标记表示支持。")
        info.setStyleSheet("color: #888; font-size: 10px; background: transparent; border: none;")
        info.setWordWrap(True)
        layout.addWidget(info)

        layout.addStretch()
        return w

    def _build_api_mode(self):
        """🔗 API 模式面板（含 4 步进度 + 上游账户 + 模型列表）"""
        w = QWidget()
        w.setStyleSheet("background-color: #161616;")
        layout = QVBoxLayout(w)
        layout.setContentsMargins(14, 12, 14, 12)
        layout.setSpacing(10)

        layout.addWidget(self._make_section_title("API 服务地址"))
        addr_row = QHBoxLayout()
        addr_row.setSpacing(0)
        prefix = QLabel("http://")
        prefix.setStyleSheet("padding: 0 6px; font-size: 11px; color: #888; background-color: #1A1A1A; border: 1px solid #333; border-right: none; border-radius: 4px 0 0 4px; height: 28px; line-height: 28px;")
        addr_row.addWidget(prefix)
        self._api_host = QLineEdit("127.0.0.1")
        self._api_host.setStyleSheet(self._input_style() + "border-radius: 0; border-left: none; border-right: none;")
        addr_row.addWidget(self._api_host, 1)
        colon = QLabel(":")
        colon.setStyleSheet("padding: 0 6px; font-size: 13px; color: #888; background-color: #1A1A1A; border: 1px solid #333; border-left: none; border-right: none; height: 28px; line-height: 28px;")
        addr_row.addWidget(colon)
        self._api_port = QLineEdit("7777")
        self._api_port.setFixedWidth(60)
        self._api_port.setStyleSheet(self._input_style() + "border-radius: 0 4px 4px 0; border-left: none; text-align: center;")
        addr_row.addWidget(self._api_port)
        layout.addLayout(addr_row)

        # 4 步进度
        layout.addWidget(self._make_section_title("服务状态"))
        self._api_steps = [
            ("检测服务", "check"),
            ("启动服务", "start"),
            ("获取 Key", "key"),
            ("加载模型", "models"),
        ]
        self._api_step_widgets = []
        steps_row = QHBoxLayout()
        steps_row.setSpacing(4)
        for i, (label, _) in enumerate(self._api_steps):
            step_box = QFrame()
            step_box.setStyleSheet("QFrame { background-color: #1F1F1F; border: 1px solid #2A2A2A; border-radius: 4px; }")
            step_box.setFixedHeight(46)
            sl = QVBoxLayout(step_box)
            sl.setContentsMargins(2, 2, 2, 2)
            sl.setSpacing(1)
            dot = QLabel(f"{i+1}")
            dot.setAlignment(Qt.AlignmentFlag.AlignCenter)
            dot.setStyleSheet("color: #888; font-size: 11px; font-weight: bold; background: transparent; border: none;")
            sl.addWidget(dot)
            lbl = QLabel(label)
            lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
            lbl.setStyleSheet("color: #888; font-size: 9px; background: transparent; border: none;")
            sl.addWidget(lbl)
            steps_row.addWidget(step_box, 1)
            self._api_step_widgets.append((step_box, dot, lbl))
        layout.addLayout(steps_row)

        # 进度条
        self._api_progress = QProgressBar()
        self._api_progress.setRange(0, 100)
        self._api_progress.setValue(0)
        self._api_progress.setTextVisible(False)
        self._api_progress.setFixedHeight(4)
        self._api_progress.setStyleSheet("""
            QProgressBar { background-color: #1F1F1F; border: none; border-radius: 2px; }
            QProgressBar::chunk { background-color: #2563EB; border-radius: 2px; }
        """)
        layout.addWidget(self._api_progress)

        self._api_step_msg = QLabel("点击「一键启动」自动配置 API 服务")
        self._api_step_msg.setStyleSheet("color: #888; font-size: 10px; background: transparent; border: none;")
        self._api_step_msg.setWordWrap(True)
        layout.addWidget(self._api_step_msg)

        api_btn_row = QHBoxLayout()
        api_btn_row.setSpacing(6)
        self._api_start_btn = QPushButton("▶ 一键启动")
        self._api_start_btn.setStyleSheet("""
            QPushButton { background-color: #2563EB; color: #FFFFFF; border: none; border-radius: 5px; padding: 7px 12px; font-size: 12px; font-weight: bold; }
            QPushButton:hover { background-color: #1D4ED8; }
            QPushButton:disabled { background-color: #333; color: #777; }
        """)
        self._api_start_btn.clicked.connect(self._api_step_auto_run)
        api_btn_row.addWidget(self._api_start_btn, 1)
        self._api_stop_btn = QPushButton("■ 停止")
        self._api_stop_btn.setStyleSheet("""
            QPushButton { background-color: #7F1D1D; color: #FFFFFF; border: none; border-radius: 5px; padding: 7px 12px; font-size: 12px; }
            QPushButton:hover { background-color: #991B1B; }
            QPushButton:disabled { background-color: #333; color: #777; }
        """)
        self._api_stop_btn.clicked.connect(self._api_stop_service)
        self._api_stop_btn.setEnabled(False)
        api_btn_row.addWidget(self._api_stop_btn)
        layout.addLayout(api_btn_row)

        # API Key
        layout.addWidget(self._make_section_title("API Key"))
        self._api_key = QLineEdit()
        self._api_key.setPlaceholderText("自动获取或手动输入")
        self._api_key.setStyleSheet(self._input_style())
        layout.addWidget(self._api_key)

        # 上游账户（千问账户）
        layout.addWidget(self._make_section_title("上游账户 (千问)"))
        qwen_header = QHBoxLayout()
        qwen_count_lbl = QLabel("-- 个")
        qwen_count_lbl.setStyleSheet("color: #10B981; font-size: 10px; background: transparent; border: none;")
        self._qwen_count_lbl = qwen_count_lbl
        qwen_header.addWidget(qwen_count_lbl)
        qwen_header.addStretch()
        refresh_btn = QPushButton("🔄 刷新")
        refresh_btn.setStyleSheet("""
            QPushButton { background-color: #1F1F1F; color: #BBB; border: 1px solid #2A2A2A; border-radius: 4px; padding: 3px 8px; font-size: 10px; }
            QPushButton:hover { background-color: #252525; color: #FFF; }
        """)
        refresh_btn.clicked.connect(self._check_qwen_accounts)
        qwen_header.addWidget(refresh_btn)
        layout.addLayout(qwen_header)

        self._qwen_account_list = QListWidget()
        self._qwen_account_list.setStyleSheet("""
            QListWidget { background-color: #0E0E0E; border: 1px solid #2A2A2A; border-radius: 4px; color: #CCC; font-size: 10px; }
            QListWidget::item { padding: 4px 6px; }
            QListWidget::item:hover { background-color: #1F1F1F; }
        """)
        self._qwen_account_list.setMaximumHeight(80)
        # 2026-06-16: 千问账户右键菜单（置顶/取消置顶/删除/复制邮箱）
        self._qwen_account_list.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self._qwen_account_list.customContextMenuRequested.connect(self._qwen_account_context_menu)
        layout.addWidget(self._qwen_account_list)

        # 千问账户操作按钮
        qwen_action_row = QHBoxLayout()
        qwen_action_row.setSpacing(4)
        for text, slot in [("🤖 自动注册", self._auto_register_qwen), ("🔑 登录", self._login_qwen), ("➕ 添加Token", self._add_qwen_token)]:
            b = QPushButton(text)
            b.setStyleSheet("""
                QPushButton { background-color: #1F1F1F; color: #BBB; border: 1px solid #2A2A2A; border-radius: 4px; padding: 4px 8px; font-size: 10px; }
                QPushButton:hover { background-color: #2563EB; color: #FFF; border-color: #2563EB; }
            """)
            b.clicked.connect(slot)
            qwen_action_row.addWidget(b)
        layout.addLayout(qwen_action_row)

        # 模型列表
        layout.addWidget(self._make_section_title("可用模型"))
        self._api_model_list = QListWidget()
        self._api_model_list.setStyleSheet("""
            QListWidget { background-color: #0E0E0E; border: 1px solid #2A2A2A; border-radius: 4px; color: #CCC; font-size: 11px; }
            QListWidget::item { padding: 6px 8px; }
            QListWidget::item:hover { background-color: #1F1F1F; }
            QListWidget::item:selected { background-color: #1E3A8A; color: #FFF; }
        """)
        self._api_model_list.itemClicked.connect(self._api_select_model)
        self._api_model_list.setMaximumHeight(120)
        layout.addWidget(self._api_model_list)

        self._api_model = QLineEdit("qwen3.6-plus")
        self._api_model.setStyleSheet(self._input_style())
        layout.addWidget(self._api_model)

        layout.addStretch()
        return w

    def _build_ollama_mode(self):
        """🦙 Ollama 模式面板"""
        w = QWidget()
        w.setStyleSheet("background-color: #161616;")
        layout = QVBoxLayout(w)
        layout.setContentsMargins(14, 12, 14, 12)
        layout.setSpacing(10)

        # 服务地址
        layout.addWidget(self._make_section_title("Ollama 服务地址"))
        addr_row = QHBoxLayout()
        addr_row.setSpacing(4)
        self._ollama_url = QLineEdit("http://127.0.0.1:11434")
        self._ollama_url.setStyleSheet(self._input_style())
        addr_row.addWidget(self._ollama_url, 1)
        detect_btn = QPushButton("🔍 检测")
        detect_btn.setStyleSheet(self._btn_sm_style("#2563EB"))
        detect_btn.clicked.connect(self._ollama_detect_models)
        addr_row.addWidget(detect_btn)
        layout.addLayout(addr_row)

        # Ollama 状态指示
        self._ollama_status_lbl = QLabel("⏳ 检测中...")
        self._ollama_status_lbl.setStyleSheet("color: #FF9800; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")
        self._ollama_status_lbl.setWordWrap(True)
        layout.addWidget(self._ollama_status_lbl)

        # 推荐安装
        rec_row = QHBoxLayout()
        rec_row.setSpacing(4)
        rec_btn = QPushButton("💡 推荐模型")
        rec_btn.setStyleSheet(self._btn_sm_style("#7C3AED"))
        rec_btn.clicked.connect(self._ollama_load_recommendations)
        rec_row.addWidget(rec_btn, 1)
        detect2_btn = QPushButton("🔄 重新检测")
        detect2_btn.setStyleSheet(self._btn_sm_style("#374151"))
        detect2_btn.clicked.connect(self._ollama_detect_models)
        rec_row.addWidget(detect2_btn)
        layout.addLayout(rec_row)

        self._ollama_rec_list = QListWidget()
        self._ollama_rec_list.setStyleSheet("""
            QListWidget { background-color: #0E0E0E; border: 1px solid #2A2A2A; border-radius: 4px; color: #CCC; font-size: 10px; }
            QListWidget::item { padding: 4px 6px; }
            QListWidget::item:hover { background-color: #1F1F1F; }
        """)
        self._ollama_rec_list.setMaximumHeight(70)
        layout.addWidget(self._ollama_rec_list)

        # 可用模型
        layout.addWidget(self._make_section_title("可用模型"))
        self._ollama_model_list = QListWidget()
        self._ollama_model_list.setStyleSheet("""
            QListWidget { background-color: #0E0E0E; border: 1px solid #2A2A2A; border-radius: 4px; color: #CCC; font-size: 11px; }
            QListWidget::item { padding: 6px 8px; }
            QListWidget::item:hover { background-color: #1F1F1F; }
            QListWidget::item:selected { background-color: #1E3A8A; color: #FFF; }
        """)
        self._ollama_model_list.itemClicked.connect(self._ollama_select_model)
        self._ollama_model_list.setMaximumHeight(140)
        layout.addWidget(self._ollama_model_list)

        # 模型配置
        layout.addWidget(self._make_section_title("模型独立配置"))
        self._ollama_temp = self._make_field("Temperature", "", "0.7")
        layout.addLayout(self._ollama_temp)
        self._ollama_max_tokens = self._make_field("Max Tokens", "", "4096")
        layout.addLayout(self._ollama_max_tokens)

        self._ollama_hint = QLabel("⚠ 不支持工具调用的模型，编程功能将受限。\n★ 工具 表示该模型支持工具调用。")
        self._ollama_hint.setStyleSheet("color: #888; font-size: 10px; background: transparent; border: none;")
        self._ollama_hint.setWordWrap(True)
        layout.addWidget(self._ollama_hint)

        layout.addStretch()
        return w

    def _build_dialog_settings(self):
        """对话设置：自动授权 / 项目管理 / 称谓 / 保存"""
        frame = QFrame()
        frame.setStyleSheet("QFrame { background-color: #1A1A1A; border-top: 1px solid #2A2A2A; }")
        layout = QVBoxLayout(frame)
        layout.setContentsMargins(14, 12, 14, 12)
        layout.setSpacing(8)

        # 分组标题
        layout.addWidget(self._make_section_title("对话设置"))

        # 自动授权 toggle
        auto_row = QHBoxLayout()
        auto_lbl = QLabel("自动授权工具调用")
        auto_lbl.setStyleSheet("color: #DDD; font-size: 11px; background: transparent; border: none;")
        auto_row.addWidget(auto_lbl)
        auto_row.addStretch()
        self._auto_approve_toggle = QCheckBox()
        self._auto_approve_toggle.setStyleSheet("""
            QCheckBox::indicator { width: 32px; height: 16px; }
            QCheckBox::indicator:unchecked { background: #333; border-radius: 8px; }
            QCheckBox::indicator:checked { background: #10B981; border-radius: 8px; }
        """)
        self._auto_approve_toggle.setChecked(False)
        auto_row.addWidget(self._auto_approve_toggle)
        layout.addLayout(auto_row)

        warn = QLabel("⚠ 开启后 AI 可直接读写文件/执行命令，无需逐次审批")
        warn.setStyleSheet("color: #FF9800; font-size: 9px; background: transparent; border: none;")
        warn.setWordWrap(True)
        layout.addWidget(warn)

        # 项目管理
        layout.addSpacing(4)
        layout.addWidget(self._make_section_title("📁 项目管理"))
        proj_btn_row = QHBoxLayout()
        proj_btn_row.setSpacing(4)
        for text, slot in [("📂 打开", self._chat_choose_workspace), ("+ 新建", self._project_create_dialog), ("📋 列表", lambda: self._switch_page(4))]:
            b = QPushButton(text)
            b.setStyleSheet("""
                QPushButton { background-color: #1F1F1F; color: #BBB; border: 1px solid #2A2A2A; border-radius: 4px; padding: 4px 8px; font-size: 10px; }
                QPushButton:hover { background-color: #2563EB; color: #FFF; border-color: #2563EB; }
            """)
            b.clicked.connect(slot)
            proj_btn_row.addWidget(b)
        layout.addLayout(proj_btn_row)

        # 当前项目显示
        self._current_proj_lbl = QLabel("当前项目: 无")
        self._current_proj_lbl.setStyleSheet("color: #42A5F5; font-size: 10px; background: transparent; border: none;")
        layout.addWidget(self._current_proj_lbl)

        # 称谓
        layout.addSpacing(4)
        self._user_name = QLineEdit("你")
        self._user_name.setPlaceholderText("你的称谓")
        self._user_name.setStyleSheet(self._input_style())
        self._user_name.setMaximumWidth(200)
        layout.addWidget(self._user_name)
        self._assistant_name = QLineEdit("助手")
        self._assistant_name.setPlaceholderText("AI 称谓")
        self._assistant_name.setStyleSheet(self._input_style())
        self._assistant_name.setMaximumWidth(200)
        layout.addWidget(self._assistant_name)

        # 保存按钮
        layout.addSpacing(4)
        save_row = QHBoxLayout()
        save_row.setSpacing(6)
        clear_btn = QPushButton("清空模型")
        clear_btn.setStyleSheet("""
            QPushButton { background-color: #374151; color: #FFFFFF; border: none; border-radius: 5px; padding: 6px 12px; font-size: 11px; }
            QPushButton:hover { background-color: #4B5563; }
        """)
        clear_btn.clicked.connect(self._chat_clear_model_fields)
        save_row.addWidget(clear_btn, 1)
        save_btn = QPushButton("💾 保存并启用")
        save_btn.setStyleSheet("""
            QPushButton { background-color: #10B981; color: #FFFFFF; border: none; border-radius: 5px; padding: 6px 12px; font-size: 11px; font-weight: bold; }
            QPushButton:hover { background-color: #059669; }
        """)
        save_btn.clicked.connect(self._chat_save_settings)
        save_row.addWidget(save_btn, 1)
        layout.addLayout(save_row)

        return frame

    # ── 工具方法 ──

    def _input_style(self):
        return """
            QLineEdit, QTextEdit {
                background-color: #0E0E0E; color: #E0E0E0;
                border: 1px solid #2A2A2A; border-radius: 4px;
                padding: 5px 8px; font-size: 11px;
            }
            QLineEdit:focus, QTextEdit:focus { border-color: #2563EB; }
        """

    def _btn_sm_style(self, color):
        hover = {
            "#2563EB": "#1D4ED8", "#7C3AED": "#6D28D9", "#374151": "#4B5563",
        }.get(color, color)
        return f"""
            QPushButton {{ background-color: {color}; color: #FFFFFF; border: none; border-radius: 4px; padding: 5px 10px; font-size: 11px; }}
            QPushButton:hover {{ background-color: {hover}; }}
            QPushButton:disabled {{ background-color: #333; color: #777; }}
        """

    def _make_section_title(self, text):
        lbl = QLabel(text)
        lbl.setStyleSheet("color: #888; font-size: 10px; font-weight: bold; padding: 4px 0 2px 0; background: transparent; border: none;")
        return lbl

    def _make_field(self, label, placeholder, value=""):
        """返回 (label, input) 的横向布局"""
        row = QHBoxLayout()
        row.setSpacing(6)
        l = QLabel(label)
        l.setStyleSheet("color: #AAA; font-size: 11px; background: transparent; border: none; min-width: 80px;")
        row.addWidget(l)
        e = QLineEdit(value)
        e.setPlaceholderText(placeholder)
        e.setStyleSheet(self._input_style())
        row.addWidget(e, 1)
        row._field_input = e  # 标记方便后续取
        return row

    def _make_combo(self, label, options, current):
        row = QHBoxLayout()
        row.setSpacing(6)
        l = QLabel(label)
        l.setStyleSheet("color: #AAA; font-size: 11px; background: transparent; border: none; min-width: 80px;")
        row.addWidget(l)
        from PyQt6.QtWidgets import QComboBox
        c = QComboBox()
        c.addItems(options)
        c.setCurrentText(current)
        c.setStyleSheet("""
            QComboBox { background-color: #0E0E0E; color: #E0E0E0; border: 1px solid #2A2A2A; border-radius: 4px; padding: 4px 8px; font-size: 11px; }
            QComboBox::drop-down { border: none; }
            QComboBox QAbstractItemView { background-color: #1A1A1A; color: #E0E0E0; selection-background-color: #2563EB; }
        """)
        row.addWidget(c, 1)
        return row

    # ── 事件处理 ──

    def _switch_run_mode(self, mode: str):
        """切换右侧面板的 3 种模式"""
        self._run_mode = mode
        for k, btn in self._mode_btns.items():
            btn.setChecked(k == mode)
        if mode == "cloud":
            self._mode_stack.setCurrentIndex(0)
        elif mode == "api":
            self._mode_stack.setCurrentIndex(1)
        elif mode == "ollama":
            self._mode_stack.setCurrentIndex(2)
            # 自动检测 Ollama 状态
            QTimer.singleShot(200, self._ollama_check_status)

    def _chat_toggle_panel(self):
        """显示/隐藏右侧配置面板"""
        if self._config_panel.isVisible():
            self._config_panel.hide()
        else:
            self._config_panel.show()

    def _chat_choose_workspace(self):
        """打开项目 / 选择工作区"""
        try:
            result = self.bridge.chooseWorkspace()
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                path = data.get("path", "")
                self._chat_current_title.setText(f"📂 {path}")
                self._append_log(f"已打开工作区: {path}", "#4CAF50")
                self._load_initial_data()
        except Exception as e:
            self._append_log(f"打开工作区失败: {e}", "#F44336")

    def _chat_new_session(self):
        """新会话"""
        try:
            result = self.bridge.newSession()
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                session_id = data.get("session_id", "")
                self._chat_current_session_id = session_id
                self._chat_current_title.setText(f"会话：{session_id[:8]}...")
                self._chat_messages_area.clear()
                self._set_welcome_message()
                self._append_log(f"已创建新会话: {session_id}", "#4CAF50")
                self._refresh_chat_session_list()
        except Exception as e:
            self._append_log(f"新会话失败: {e}", "#F44336")

    def _chat_stop_message(self):
        """停止当前消息"""
        try:
            self.bridge.stopMessage()
            self._composer_status.setText("就绪")
            self._btn_stop_msg.setEnabled(False)
        except Exception as e:
            self._append_log(f"停止失败: {e}", "#F44336")

    def _chat_send_message(self):
        """发送消息"""
        text = self._chat_input.toPlainText().strip()
        if not text:
            return
        if not self._chat_current_session_id:
            self._chat_new_session()
        self._composer_status.setText("执行中...")
        self._btn_stop_msg.setEnabled(True)
        try:
            payload = json.dumps({
                "session_id": self._chat_current_session_id,
                "text": text,
                "model": self._api_model.text() if self._run_mode == "api" else "",
                "provider": self._run_mode,
            })
            self.bridge.sendMessage(payload)
            self._chat_input.clear()
        except Exception as e:
            self._append_log(f"发送失败: {e}", "#F44336")
            self._composer_status.setText("就绪")
            self._btn_stop_msg.setEnabled(False)

    def _chat_clear_messages(self):
        """清空消息"""
        self._chat_messages_area.clear()
        self._set_welcome_message()

    # ── 2026-06-16: 消息角色 + Markdown 渲染 ──

    _ROLE_META = {
        "user":      {"icon": "👤", "name": "你",      "color": "#3B82F6", "align": "right"},
        "assistant": {"icon": "🤖", "name": "AI 助手", "color": "#10B981", "align": "left"},
        "system":    {"icon": "⚙️", "name": "系统",   "color": "#A78BFA", "align": "left"},
        "error":     {"icon": "⚠️", "name": "错误",   "color": "#F44336", "align": "left"},
    }

    def append_chat_message(self, role: str, text: str, ts: str = "", model: str = ""):
        """往中间对话窗追加一条带角色/Markdown 渲染的消息

        role: user / assistant / system / error
        text: 原始 Markdown 文本
        """
        if not hasattr(self, "_chat_messages_area"):
            return
        meta = self._ROLE_META.get(role, self._ROLE_META["assistant"])
        ts_html = f'<span style="color: #666; font-size: 9px; margin-left: 6px;">{ts}</span>' if ts else ""
        model_html = f'<span style="color: #666; font-size: 9px; margin-left: 6px;">{model}</span>' if model else ""
        # 复制按钮（仅 assistant / user 显示）
        copy_btn_html = f'''
            <a href="copy://{role}/{id(text)}" style="float: right; color: #888; font-size: 10px; text-decoration: none; padding: 2px 6px; border: 1px solid #333; border-radius: 3px;" onmouseover="this.style.color='#FFF';this.style.borderColor='#888'" onmouseout="this.style.color='#888';this.style.borderColor='#333'">📋 复制</a>
        ''' if role in ("assistant", "user") else ""

        body_html = self._markdown_to_html(text)
        # 整条消息 HTML
        msg_html = f'''
            <div style="margin: 8px 0; padding: 8px 4px;">
                <div style="display: flex; align-items: center; margin-bottom: 4px;">
                    <span style="font-size: 16px; margin-right: 6px;">{meta["icon"]}</span>
                    <span style="color: {meta["color"]}; font-size: 12px; font-weight: bold;">{meta["name"]}</span>
                    {ts_html}
                    {model_html}
                    {copy_btn_html}
                </div>
                <div style="color: #E0E0E0; font-size: 13px; line-height: 1.7; padding-left: 22px;">
                    {body_html}
                </div>
            </div>
            <hr style="border: none; border-top: 1px solid #1F1F1F; margin: 6px 0;">
        '''
        # 追加到 QTextEdit
        cursor = self._chat_messages_area.textCursor()
        cursor.movePosition(cursor.MoveOperation.End)
        cursor.insertHtml(msg_html)
        # 滚到底部
        sb = self._chat_messages_area.verticalScrollBar()
        sb.setValue(sb.maximum())

    def _markdown_to_html(self, text: str) -> str:
        """极简 Markdown → HTML（适合 PyQt6 QTextEdit）"""
        import re
        if not text:
            return ""
        # 1. 提取代码块 ```lang\n...\n```
        code_blocks = []
        def _save_code(m):
            lang = m.group(1) or ""
            code = m.group(2)
            escaped = (
                code.replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;")
            )
            lang_html = f'<span style="color: #888; font-size: 10px;">{lang}</span>' if lang else ""
            html = f'''
                <div style="background: #0A0A0A; border: 1px solid #2A2A2A; border-radius: 4px; margin: 6px 0; padding: 8px;">
                    {lang_html}
                    <pre style="color: #E0E0E0; font-family: 'Consolas','Cascadia Code',monospace; font-size: 12px; margin: 4px 0 0 0; white-space: pre-wrap; word-wrap: break-word;">{escaped}</pre>
                </div>
            '''
            code_blocks.append(html)
            return f"\x00CODEBLOCK{len(code_blocks)-1}\x00"
        text = re.sub(r"```(\w*)\n(.*?)```", _save_code, text, flags=re.DOTALL)

        # 2. 转义剩余 HTML 特殊字符
        text = text.replace("&", "&amp;")
        text = text.replace("<", "&lt;")
        text = text.replace(">", "&gt;")

        # 3. 标题 (# / ## / ###)
        text = re.sub(r"^### (.+)$", r'<h4 style="color: #93C5FD; font-size: 13px; margin: 8px 0 4px 0;">\1</h4>', text, flags=re.MULTILINE)
        text = re.sub(r"^## (.+)$", r'<h3 style="color: #93C5FD; font-size: 14px; margin: 8px 0 4px 0;">\1</h3>', text, flags=re.MULTILINE)
        text = re.sub(r"^# (.+)$", r'<h2 style="color: #93C5FD; font-size: 16px; margin: 8px 0 4px 0;">\1</h2>', text, flags=re.MULTILINE)

        # 4. 粗体 **...** 和 斜体 *...*
        text = re.sub(r"\*\*(.+?)\*\*", r'<b style="color: #FCD34D;">\1</b>', text)
        text = re.sub(r"\*(.+?)\*", r'<i style="color: #FCA5A5;">\1</i>', text)

        # 5. 行内代码 `...`
        text = re.sub(r"`([^`]+?)`", r'<code style="background: #1A1A1A; color: #A7F3D0; padding: 1px 4px; border-radius: 3px; font-family: monospace; font-size: 12px;">\1</code>', text)

        # 6. 链接 [text](url)
        text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2" style="color: #60A5FA;">\1</a>', text)

        # 7. 列表 - item
        lines = text.split("\n")
        new_lines = []
        in_ul = False
        in_ol = False
        for line in lines:
            stripped = line.strip()
            is_li = re.match(r"^- ", stripped)
            is_oli = re.match(r"^\d+\. ", stripped)
            is_block = re.match(r"^<(h[1-6]|ul|ol|li|hr|p|div|pre|code|br)\b", stripped)
            # 列表行处理
            if is_li:
                if in_ol:
                    new_lines.append("</ol>")
                    in_ol = False
                if not in_ul:
                    new_lines.append('<ul style="margin: 4px 0; padding-left: 24px; color: #D1D5DB;">')
                    in_ul = True
                new_lines.append(f'<li>{stripped[2:]}</li>')
                continue
            if is_oli:
                if in_ul:
                    new_lines.append("</ul>")
                    in_ul = False
                if not in_ol:
                    new_lines.append('<ol style="margin: 4px 0; padding-left: 24px; color: #D1D5DB;">')
                    in_ol = True
                new_lines.append(f'<li>{stripped[stripped.index(" ")+1:]}</li>')
                continue
            # 非列表行：先关掉进行中的列表
            if in_ul:
                new_lines.append("</ul>")
                in_ul = False
            if in_ol:
                new_lines.append("</ol>")
                in_ol = False
            if not stripped:
                new_lines.append('<br/>')
            elif is_block:
                new_lines.append(stripped)  # 已经是块级元素标签，不包 <p>
            else:
                new_lines.append(f'<p style="margin: 4px 0;">{line}</p>')
        if in_ul:
            new_lines.append("</ul>")
        if in_ol:
            new_lines.append("</ol>")
        text = "\n".join(new_lines)

        # 8. 还原代码块
        for i, code_html in enumerate(code_blocks):
            text = text.replace(f"\x00CODEBLOCK{i}\x00", code_html)

        return text

    def _chat_clear_model_fields(self):
        """清空模型相关字段"""
        self._cloud_api_key.clear()
        self._api_key.clear()
        self._api_model.setText("")
        self._ollama_url.setText("http://127.0.0.1:11434")

    def _chat_save_settings(self):
        """保存并启用设置"""
        try:
            payload = json.dumps({
                "MODEL_PROVIDER": self._run_mode,
                "ANTHROPIC_API_KEY": self._cloud_api_key.text(),
                "API_BASE_URL": f"http://{self._api_host.text()}:{self._api_port.text()}",
                "API_MODEL": self._api_model.text(),
                "API_KEY": self._api_key.text(),
                "OLLAMA_BASE_URL": self._ollama_url.text(),
                "AI_LANGUAGE": "zh",
                "AI_TEMPERATURE": "",
                "AI_MAX_TOKENS": "",
            })
            result = self.bridge.saveSettings(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                self._append_log("✓ 设置已保存并启用", "#10B981")
            else:
                self._append_log(f"保存失败: {data}", "#F44336")
        except Exception as e:
            self._append_log(f"保存失败: {e}", "#F44336")

    # ── 加载初始数据 ──

    def _load_initial_data(self):
        """加载侧栏项目 + 右上角模式状态"""
        try:
            self._refresh_projects_for_sidebar()
            self._ollama_check_status()
            self._api_check_status()
        except Exception as e:
            self._append_log(f"加载初始数据失败: {e}", "#F44336")

    def _refresh_projects_for_sidebar(self):
        """刷新 workbuddy 边栏的项目列表"""
        try:
            result = self.bridge.listProjects()
            data = json.loads(result) if isinstance(result, str) else result
            if not data or not data.get("ok"):
                return
            projects = data.get("projects", [])
            self._workbuddy_proj_list.clear()
            if not projects:
                # 2026-06-16: 空状态占位
                from PyQt6.QtWidgets import QListWidgetItem
                empty = QListWidgetItem("   暂无项目，点 + 新建")
                empty.setFlags(Qt.ItemFlag.NoItemFlags)  # 不可选
                empty.setForeground(Qt.GlobalColor.gray)
                self._workbuddy_proj_list.addItem(empty)
            else:
                for p in projects:
                    from PyQt6.QtWidgets import QListWidgetItem
                    item = QListWidgetItem(f"📁 {p.get('name', '未命名')}")
                    item.setData(Qt.ItemDataRole.UserRole, p.get("id"))
                    item.setData(Qt.ItemDataRole.UserRole + 1, p.get("path") or "")  # 2026-06-16: 存路径供右键菜单用
                    item.setToolTip(p.get("path", ""))
                    self._workbuddy_proj_list.addItem(item)
            # 激活项目
            active = data.get("active")
            if active:
                self._current_proj_lbl.setText(f"当前项目: {active.get('name', '?')}")
        except Exception as e:
            self._append_log(f"刷新项目列表失败: {e}", "#F44336")

    def _refresh_chat_session_list(self):
        """刷新会话列表（保留兼容旧调用）"""
        # 会话列表原本要显示在左侧；现在改到 workbuddy 边栏的导航区
        # 暂时留空，旧调用不再触发错误
        pass

    # ── Ollama 模式相关 ──

    def _ollama_check_status(self):
        """检测 Ollama 状态"""
        try:
            result = self.bridge.detectOllama()
            data = json.loads(result) if isinstance(result, str) else result
            if not data:
                return
            if data.get("installed") and data.get("running"):
                v = data.get("version", "?")
                self._ollama_status_lbl.setText(f"✓ 已运行 · v{v} · {data.get('installPath', '')}")
                self._ollama_status_lbl.setStyleSheet("color: #10B981; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")
            elif data.get("installed") and not data.get("running"):
                self._ollama_status_lbl.setText(f"⚠ 已安装但未启动：{data.get('installPath', '')}（请启动 Ollama 后重新检测）")
                self._ollama_status_lbl.setStyleSheet("color: #FF9800; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")
            else:
                self._ollama_status_lbl.setText("○ 未安装 Ollama。点击下面「检测」可一键安装引导")
                self._ollama_status_lbl.setStyleSheet("color: #FF9800; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")
        except Exception as e:
            self._ollama_status_lbl.setText(f"检测失败: {e}")
            self._ollama_status_lbl.setStyleSheet("color: #F44336; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")

    def _ollama_detect_models(self):
        """检测 Ollama 模型"""
        if not hasattr(self, "_ollama_model_list"):
            return
        self._ollama_model_list.clear()
        self._ollama_status_lbl.setText("⏳ 检测中...")
        QTimer.singleShot(50, self._ollama_do_detect)

    def _ollama_do_detect(self):
        try:
            url = self._ollama_url.text() or "http://127.0.0.1:11434"
            payload = json.dumps({"source": "ollama", "base_url": url})
            result = self.bridge.listModels(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if not data or not data.get("ok"):
                self._ollama_status_lbl.setText(f"⚠ 检测失败: {data.get('error') if data else '未知'}")
                return
            models = data.get("models", [])
            for m in models:
                from PyQt6.QtWidgets import QListWidgetItem
                tool = "★ 工具" if m.get("toolSupport") else ("无工具" if m.get("toolSupport") is False else "")
                size = m.get("size", "")
                line = f"{m.get('name', m.get('id'))}  {tool}  {size}".strip()
                item = QListWidgetItem(line)
                item.setData(Qt.ItemDataRole.UserRole, m.get("id") or m.get("name"))
                self._ollama_model_list.addItem(item)
            self._ollama_status_lbl.setText(f"✓ 已检测 {len(models)} 个模型")
            self._ollama_status_lbl.setStyleSheet("color: #10B981; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")
        except Exception as e:
            self._ollama_status_lbl.setText(f"检测失败: {e}")
            self._ollama_status_lbl.setStyleSheet("color: #F44336; font-size: 10px; background: transparent; border: none; padding: 4px 6px;")

    def _ollama_load_recommendations(self):
        """加载推荐模型"""
        if not hasattr(self, "_ollama_rec_list"):
            return
        self._ollama_rec_list.clear()
        try:
            result = self.bridge.recommendModels()
            data = json.loads(result) if isinstance(result, str) else result
            if not data or not data.get("ok"):
                return
            for r in data.get("recommendations", []):
                from PyQt6.QtWidgets import QListWidgetItem
                tool = "★ 工具" if r.get("toolSupport") else "无工具"
                line = f"{r.get('name', '?')}  {tool}  {r.get('size', '')}"
                item = QListWidgetItem(line)
                item.setData(Qt.ItemDataRole.UserRole, r.get("name"))
                item.setToolTip(r.get("reason", ""))
                self._ollama_rec_list.addItem(item)
        except Exception as e:
            self._append_log(f"加载推荐失败: {e}", "#F44336")

    def _ollama_select_model(self, item):
        """选中 Ollama 模型"""
        model_id = item.data(Qt.ItemDataRole.UserRole)
        if model_id:
            self._chat_input.setPlaceholderText(f"使用 {model_id} 模型 · 输入任务...")

    # ── API 模式相关 ──

    def _api_check_status(self):
        """检测 API 服务状态"""
        try:
            host = self._api_host.text() if hasattr(self, "_api_host") else "127.0.0.1"
            port = self._api_port.text() if hasattr(self, "_api_port") else "7777"
            payload = json.dumps({"host": host, "port": port})
            result = self.bridge.checkApiService(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok") and data.get("running"):
                self._update_api_step(4, "✓ API 服务已就绪")
                self._api_start_btn.setText("✓ API 服务已就绪")
                self._api_start_btn.setEnabled(False)
                self._api_stop_btn.setEnabled(True)
            else:
                self._update_api_step(0, "未启动")
        except Exception:
            self._update_api_step(0, "未启动")

    def _update_api_step(self, progress: int, message: str):
        """更新 4 步进度状态 (0-4, 4 表示完成)"""
        if not hasattr(self, "_api_step_widgets"):
            return
        for i, (box, dot, lbl) in enumerate(self._api_step_widgets):
            if i < progress:
                dot.setText("✓")
                dot.setStyleSheet("color: #10B981; font-size: 11px; font-weight: bold; background: transparent; border: none;")
                lbl.setStyleSheet("color: #10B981; font-size: 9px; background: transparent; border: none;")
                box.setStyleSheet("QFrame { background-color: rgba(16, 185, 129, 0.1); border: 1px solid #10B981; border-radius: 4px; }")
            elif i == progress:
                dot.setText(f"{i+1}")
                dot.setStyleSheet("color: #2563EB; font-size: 11px; font-weight: bold; background: transparent; border: none;")
                lbl.setStyleSheet("color: #2563EB; font-size: 9px; background: transparent; border: none;")
                box.setStyleSheet("QFrame { background-color: rgba(37, 99, 235, 0.1); border: 1px solid #2563EB; border-radius: 4px; }")
            else:
                dot.setText(f"{i+1}")
                dot.setStyleSheet("color: #555; font-size: 11px; background: transparent; border: none;")
                lbl.setStyleSheet("color: #555; font-size: 9px; background: transparent; border: none;")
                box.setStyleSheet("QFrame { background-color: #1F1F1F; border: 1px solid #2A2A2A; border-radius: 4px; }")
        percent = int((progress / 4) * 100) if progress < 4 else 100
        self._api_progress.setValue(percent)
        self._api_step_msg.setText(message)

    def _api_step_auto_run(self):
        """一键启动 API 服务（4 步）"""
        self._api_start_btn.setEnabled(False)
        host = self._api_host.text()
        port = self._api_port.text()

        def step1():
            self._update_api_step(0, "检测服务中...")
            try:
                payload = json.dumps({"host": host, "port": port})
                result = self.bridge.checkApiService(payload)
                data = json.loads(result) if isinstance(result, str) else result
                if data and data.get("ok") and data.get("running"):
                    self._update_api_step(4, "✓ 服务已在运行")
                    self._api_start_btn.setText("✓ API 服务已就绪")
                    self._api_stop_btn.setEnabled(True)
                    return
            except Exception:
                pass
            QTimer.singleShot(200, step2)

        def step2():
            self._update_api_step(1, "启动服务中...")
            try:
                self.bridge.startAllApiServices()
            except Exception as e:
                self._api_step_msg.setText(f"启动失败: {e}")
                self._api_start_btn.setEnabled(True)
                return
            QTimer.singleShot(2500, step3)

        def step3():
            self._update_api_step(2, "获取 API Key 中...")
            try:
                result = self.bridge.fetchApiKey()
                data = json.loads(result) if isinstance(result, str) else result
                if data and data.get("api_key"):
                    self._api_key.setText(data.get("api_key"))
            except Exception:
                pass
            QTimer.singleShot(500, step4)

        def step4():
            self._update_api_step(3, "加载模型列表...")
            try:
                payload = json.dumps({"host": host, "port": port})
                result = self.bridge.listAllModels()
                data = json.loads(result) if isinstance(result, str) else result
                if data and data.get("ok"):
                    self._api_model_list.clear()
                    for m in data.get("models", []):
                        from PyQt6.QtWidgets import QListWidgetItem
                        item = QListWidgetItem(m.get("name") or m.get("id"))
                        item.setData(Qt.ItemDataRole.UserRole, m.get("id") or m.get("name"))
                        self._api_model_list.addItem(item)
            except Exception:
                pass
            self._update_api_step(4, "✓ API 服务已就绪")
            self._api_start_btn.setText("✓ API 服务已就绪")
            self._api_stop_btn.setEnabled(True)
            self._api_check_status()
            self._check_qwen_accounts()

        QTimer.singleShot(50, step1)

    def _api_stop_service(self):
        """停止 API 服务"""
        try:
            host = self._api_host.text()
            port = self._api_port.text()
            payload = json.dumps({"host": host, "port": port})
            self.bridge.stopServiceByPort(payload)
            self._update_api_step(0, "已停止")
            self._api_start_btn.setText("▶ 一键启动")
            self._api_start_btn.setEnabled(True)
            self._api_stop_btn.setEnabled(False)
        except Exception as e:
            self._append_log(f"停止失败: {e}", "#F44336")

    def _api_select_model(self, item):
        """选中 API 模型"""
        model_id = item.data(Qt.ItemDataRole.UserRole)
        if model_id:
            self._api_model.setText(model_id)

    # ── 千问账户管理 ──

    def _check_qwen_accounts(self):
        """刷新千问账户列表"""
        try:
            result = self.bridge.listQwenAccounts("{}")
            data = json.loads(result) if isinstance(result, str) else result
            if not data or not data.get("ok"):
                return
            accounts = data.get("accounts", [])
            self._qwen_account_list.clear()
            for acc in accounts:
                from PyQt6.QtWidgets import QListWidgetItem
                status = "✓" if acc.get("valid") else "✗"
                line = f"{status} {acc.get('email', '?')[:30]}"
                item = QListWidgetItem(line)
                item.setData(Qt.ItemDataRole.UserRole, acc)
                self._qwen_account_list.addItem(item)
            count = len(accounts)
            valid = sum(1 for a in accounts if a.get("valid"))
            self._qwen_count_lbl.setText(f"{valid}/{count} 有效")
        except Exception as e:
            self._qwen_count_lbl.setText(f"-- 错误: {e}")

    def _auto_register_qwen(self):
        """自动注册千问账户"""
        self._append_log("正在自动注册千问账户...", "#2196F3")
        try:
            self.bridge.startQwenRegister("{}")
            self._append_log("已启动注册流程，请关注输出日志", "#4CAF50")
        except Exception as e:
            self._append_log(f"启动注册失败: {e}", "#F44336")

    def _login_qwen(self):
        """登录千问账户"""
        from PyQt6.QtWidgets import QInputDialog
        email, ok = QInputDialog.getText(self, "登录千问", "邮箱:")
        if not ok or not email.strip():
            return
        pwd, ok = QInputDialog.getText(self, "登录千问", "密码:", QLineEdit.EchoMode.Password)
        if not ok or not pwd.strip():
            return
        try:
            payload = json.dumps({"email": email.strip(), "password": pwd})
            self.bridge.startQwenLogin(payload)
            self._append_log(f"已启动登录: {email}", "#4CAF50")
        except Exception as e:
            self._append_log(f"登录失败: {e}", "#F44336")

    def _add_qwen_token(self):
        """手动添加千问 Token"""
        from PyQt6.QtWidgets import QInputDialog
        token, ok = QInputDialog.getText(self, "添加千问 Token", "粘贴 chat.qwen.ai 的 Token:")
        if not ok or not token.strip():
            return
        try:
            payload = json.dumps({"token": token.strip()})
            result = self.bridge.addQwenAccount(payload)
            data = json.loads(result) if isinstance(result, str) else result
            if data and data.get("ok"):
                self._append_log("✓ Token 已添加", "#10B981")
                self._check_qwen_accounts()
            else:
                self._append_log(f"添加失败: {data}", "#F44336")
        except Exception as e:
            self._append_log(f"添加失败: {e}", "#F44336")

    def _qwen_account_context_menu(self, pos):
        """千问账户列表：右键菜单（置顶/取消置顶/复制邮箱/删除）"""
        item = self._qwen_account_list.itemAt(pos)
        if not item:
            return
        acc = item.data(Qt.ItemDataRole.UserRole)
        if not isinstance(acc, dict):
            return
        is_sticky = acc.get("sticky", False)
        is_valid = acc.get("valid", True)
        from PyQt6.QtWidgets import QMenu
        from PyQt6.QtGui import QGuiApplication
        menu = QMenu(self._qwen_account_list)
        menu.setStyleSheet("""
            QMenu {
                background-color: #1A1A1A; color: #E0E0E0;
                border: 1px solid #2A2A2A; border-radius: 4px;
                padding: 4px;
            }
            QMenu::item { padding: 6px 18px; border-radius: 3px; }
            QMenu::item:selected { background-color: #2563EB; color: #FFFFFF; }
            QMenu::item:disabled { color: #555; }
            QMenu::separator { height: 1px; background: #2A2A2A; margin: 4px 0; }
        """)
        a_sticky = menu.addAction("📌 置顶账户" if not is_sticky else "🚫 取消置顶")
        menu.addSeparator()
        a_copy = menu.addAction("📋 复制邮箱")
        a_valid = menu.addAction("🔄 重新验证") if not is_valid else None
        menu.addSeparator()
        a_del = menu.addAction("🗑  删除账户")
        a_del.setShortcut("Del")
        chosen = menu.exec(self._qwen_account_list.mapToGlobal(pos))
        if chosen is None:
            return
        try:
            if chosen is a_sticky:
                if is_sticky:
                    self.bridge.clearStickyAccount("{}")
                    self._append_log("✓ 已取消置顶", "#10B981")
                else:
                    email = acc.get("email", "")
                    payload = json.dumps({"email": email})
                    result = self.bridge.setStickyAccount(payload)
                    data = json.loads(result) if isinstance(result, str) else result
                    if data and data.get("ok"):
                        self._append_log(f"✓ 已置顶: {email}", "#10B981")
                    else:
                        self._append_log(f"置顶失败: {data}", "#F44336")
                self._check_qwen_accounts()
            elif chosen is a_copy:
                QGuiApplication.clipboard().setText(acc.get("email", ""))
                self._append_log(f"已复制邮箱: {acc.get('email', '')}", "#4CAF50")
            elif a_valid is not None and chosen is a_valid:
                email = acc.get("email", "")
                self._append_log(f"正在重新验证: {email}", "#2196F3")
                # 触发登录流程重新验证
                self._login_qwen()
            elif chosen is a_del:
                from PyQt6.QtWidgets import QMessageBox
                email = acc.get("email", "")
                reply = QMessageBox.question(
                    self, "确认删除",
                    f"确定要删除千问账户「{email}」吗？\n\n此操作会从工作区移除该账户。",
                    QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
                    QMessageBox.StandardButton.No,
                )
                if reply == QMessageBox.StandardButton.Yes:
                    payload = json.dumps({"email": email})
                    result = self.bridge.deleteQwenAccount(payload)
                    data = json.loads(result) if isinstance(result, str) else result
                    if data and data.get("ok"):
                        self._append_log(f"✓ 已删除账户: {email}", "#10B981")
                        self._check_qwen_accounts()
                    else:
                        self._append_log(f"删除失败: {data}", "#F44336")
        except Exception as e:
            self._append_log(f"操作失败: {e}", "#F44336")

    def eventFilter(self, obj, event):
        """Enter / Shift+Enter 处理"""
        if obj is getattr(self, "_chat_input", None) and event.type() == QEvent.Type.KeyPress:
            if event.key() in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
                if event.modifiers() & Qt.KeyboardModifier.ShiftModifier:
                    return False
                self._chat_send_message()
                return True
        return super().eventFilter(obj, event)

    def _create_project_page(self):
        """创建项目管理页面 - 原生 PyQt6 实现"""
        page = QWidget()
        page.setStyleSheet("background-color: #121212;")
        layout = QVBoxLayout(page)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # 顶栏
        top = QFrame()
        top.setStyleSheet("QFrame { background-color: #1a1a1a; border-bottom: 1px solid #2a2a2a; }")
        top_layout = QHBoxLayout(top)
        top_layout.setContentsMargins(16, 12, 16, 12)
        top_layout.setSpacing(8)

        title = QLabel("📁 项目管理")
        title.setStyleSheet("color: #FFFFFF; font-size: 16px; font-weight: bold; background: transparent; border: none;")
        top_layout.addWidget(title)

        top_layout.addStretch()

        self._proj_search = QLineEdit()
        self._proj_search.setPlaceholderText("🔍 搜索项目...")
        self._proj_search.setFixedWidth(220)
        self._proj_search.setStyleSheet("""
            QLineEdit {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 6px;
                padding: 6px 10px; font-size: 12px;
            }
        """)
        self._proj_search.textChanged.connect(self._project_filter_changed)
        top_layout.addWidget(self._proj_search)

        btn_new = QPushButton("+ 新建项目")
        btn_new.setStyleSheet("""
            QPushButton {
                background-color: #2563EB; color: #FFFFFF;
                border: none; border-radius: 6px;
                padding: 6px 14px; font-size: 12px; font-weight: bold;
            }
            QPushButton:hover { background-color: #1D4ED8; }
        """)
        btn_new.clicked.connect(self._project_create_dialog)
        top_layout.addWidget(btn_new)

        layout.addWidget(top)

        # 项目列表
        self._project_list = QListWidget()
        self._project_list.setStyleSheet("""
            QListWidget { background-color: #121212; border: none; color: #E0E0E0; font-size: 13px; }
            QListWidget::item { padding: 14px 16px; border-bottom: 1px solid #222; }
            QListWidget::item:hover { background-color: #1f1f1f; }
            QListWidget::item:selected { background-color: #1E3A8A; color: #fff; }
        """)
        layout.addWidget(self._project_list, 1)

        # 底部状态
        self._proj_status = QLabel("加载中...")
        self._proj_status.setStyleSheet("color: #888; font-size: 11px; padding: 6px 16px; background-color: #1a1a1a; border-top: 1px solid #2a2a2a;")
        layout.addWidget(self._proj_status)

        return page

    def _create_history_page(self):
        """2026-06-16 新增：对话历史页面

        列出所有项目下的所有对话，按更新时间倒序。
        - 顶部：标题 + 搜索框 + 刷新按钮
        - 中部：滚动列表（每项显示：项目标签 + 对话标题 + 时间 + 消息数 + 删除按钮）
        - 底部：状态文字
        """
        page = QWidget()
        page.setStyleSheet("background-color: #121212;")
        layout = QVBoxLayout(page)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # 顶栏
        top = QFrame()
        top.setStyleSheet("QFrame { background-color: #1a1a1a; border-bottom: 1px solid #2a2a2a; }")
        top_layout = QHBoxLayout(top)
        top_layout.setContentsMargins(16, 12, 16, 12)
        top_layout.setSpacing(8)

        title = QLabel("📜 对话历史")
        title.setStyleSheet("color: #FFFFFF; font-size: 16px; font-weight: bold; background: transparent; border: none;")
        top_layout.addWidget(title)

        top_layout.addStretch()

        self._history_search = QLineEdit()
        self._history_search.setPlaceholderText("🔍 搜索对话标题...")
        self._history_search.setFixedWidth(220)
        self._history_search.setStyleSheet("""
            QLineEdit {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 6px;
                padding: 6px 10px; font-size: 12px;
            }
        """)
        self._history_search.textChanged.connect(self._history_filter_changed)
        top_layout.addWidget(self._history_search)

        btn_refresh = QPushButton("🔄 刷新")
        btn_refresh.setStyleSheet("""
            QPushButton {
                background-color: #1F1F1F; color: #E0E0E0;
                border: 1px solid #2A2A2A; border-radius: 6px;
                padding: 6px 14px; font-size: 12px;
            }
            QPushButton:hover { background-color: #2A2A2A; color: #FFFFFF; border-color: #3A3A3A; }
        """)
        btn_refresh.clicked.connect(self._refresh_history_list)
        top_layout.addWidget(btn_refresh)

        layout.addWidget(top)

        # 历史列表
        self._history_list = QListWidget()
        self._history_list.setStyleSheet("""
            QListWidget { background-color: #121212; border: none; color: #E0E0E0; font-size: 13px; }
            QListWidget::item { padding: 12px 16px; border-bottom: 1px solid #222; }
            QListWidget::item:hover { background-color: #1f1f1f; }
            QListWidget::item:selected { background-color: #1E3A8A; color: #fff; }
        """)
        # 双击跳转到任务对话页并加载该会话
        self._history_list.itemDoubleClicked.connect(self._history_open_item)
        layout.addWidget(self._history_list, 1)

        # 底部状态
        self._history_status = QLabel("加载中...")
        self._history_status.setStyleSheet("color: #888; font-size: 11px; padding: 6px 16px; background-color: #1a1a1a; border-top: 1px solid #2a2a2a;")
        layout.addWidget(self._history_status)

        # 缓存最近一次的全量历史，供搜索过滤使用
        self._history_cache = []

        return page

    def _refresh_history_list(self):
        """刷新历史列表（从 bridge 拉所有项目的对话）"""
        if not hasattr(self, "_history_list"):
            return
        try:
            result = self.bridge.listAllConversations()
            data = json.loads(result) if isinstance(result, str) else result
            if not data:
                self._history_cache = []
            else:
                self._history_cache = data
        except Exception as e:
            self._history_cache = []
            self._append_log(f"加载历史失败: {e}", "#F44336")
        self._render_history_list(self._history_cache)

    def _render_history_list(self, items):
        """渲染历史列表（已过滤过的 items）"""
        if not hasattr(self, "_history_list"):
            return
        self._history_list.clear()
        if not items:
            empty = QListWidgetItem("   📭 暂无对话历史。开始你的第一次对话吧～")
            empty.setFlags(Qt.ItemFlag.NoItemFlags)
            empty.setForeground(Qt.GlobalColor.gray)
            self._history_list.addItem(empty)
            if hasattr(self, "_history_status"):
                self._history_status.setText("共 0 条")
            return
        for c in items:
            title = c.get("title") or "（未命名对话）"
            project = c.get("project_name", "未分类")
            updated = c.get("updated_at", "")
            count = c.get("message_count", 0)
            # 时间显示
            ts = updated.replace("T", " ")
            if "." in ts:
                ts = ts.split(".")[0]
            line = f"💬 {title}\n   📁 {project}  ·  💭 {count} 条  ·  🕒 {ts or '未知时间'}"
            item = QListWidgetItem(line)
            item.setData(Qt.ItemDataRole.UserRole, {
                "project_id": c.get("project_id"),
                "session_id": c.get("session_id"),
                "title": title,
            })
            self._history_list.addItem(item)
        if hasattr(self, "_history_status"):
            self._history_status.setText(f"共 {len(items)} 条对话")

    def _history_filter_changed(self, text):
        """历史搜索框过滤"""
        kw = (text or "").strip().lower()
        if not kw:
            self._render_history_list(self._history_cache)
            return
        filtered = [
            c for c in self._history_cache
            if kw in (c.get("title") or "").lower()
            or kw in (c.get("project_name") or "").lower()
        ]
        self._render_history_list(filtered)

    def _history_open_item(self, item):
        """双击历史项：切换到该项目并把会话 id 暂存，聊天页会读取并加载"""
        data = item.data(Qt.ItemDataRole.UserRole)
        if not isinstance(data, dict):
            return
        pid = data.get("project_id")
        sid = data.get("session_id")
        if not pid or not sid:
            return
        try:
            # 1. 切换项目
            self.bridge.switchProject(pid)
            # 2. 暂存要加载的会话 id
            self._pending_load_session = sid
            # 3. 切换到任务对话页（page 0）
            self._switch_page(0)
            self._append_log(f"已加载历史对话: {data.get('title', sid)}", "#10B981")
        except Exception as e:
            self._append_log(f"打开历史对话失败: {e}", "#F44336")

    def _create_settings_page(self):
        """创建系统设置页面 - 原生 PyQt6 实现"""
        page = QWidget()
        page.setStyleSheet("background-color: #121212;")
        layout = QVBoxLayout(page)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        # 顶栏
        top = QFrame()
        top.setStyleSheet("QFrame { background-color: #1a1a1a; border-bottom: 1px solid #2a2a2a; }")
        top_layout = QHBoxLayout(top)
        top_layout.setContentsMargins(16, 12, 16, 12)
        title = QLabel("⚙️ 系统设置")
        title.setStyleSheet("color: #FFFFFF; font-size: 16px; font-weight: bold; background: transparent; border: none;")
        top_layout.addWidget(title)
        top_layout.addStretch()
        layout.addWidget(top)

        # 滚动设置区
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet("QScrollArea { background-color: #121212; border: none; }")
        scroll_body = QWidget()
        scroll_body.setStyleSheet("background-color: #121212;")
        body_layout = QVBoxLayout(scroll_body)
        body_layout.setSpacing(12)
        body_layout.setContentsMargins(16, 16, 16, 16)

        def make_setting_row(label_text, widget):
            row = QFrame()
            row.setStyleSheet("QFrame { background-color: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 6px; }")
            row_layout = QHBoxLayout(row)
            row_layout.setContentsMargins(14, 10, 14, 10)
            label = QLabel(label_text)
            label.setStyleSheet("color: #E0E0E0; font-size: 13px; background: transparent; border: none; min-width: 140px;")
            row_layout.addWidget(label)
            row_layout.addStretch()
            row_layout.addWidget(widget)
            return row

        # 主题
        self._settings_theme = QComboBox()
        self._settings_theme.addItems(["🌙 暗色", "☀️ 亮色", "🖥️ 跟随系统"])
        self._settings_theme.setStyleSheet("""
            QComboBox {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 4px;
                padding: 4px 10px; font-size: 12px; min-width: 140px;
            }
        """)
        body_layout.addWidget(make_setting_row("主题", self._settings_theme))

        # 默认模型供应商
        self._settings_default_provider = QComboBox()
        self._settings_default_provider.addItems(["qwen2api", "zhipu2api", "ollama", "openai", "anthropic"])
        self._settings_default_provider.setStyleSheet("""
            QComboBox {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 4px;
                padding: 4px 10px; font-size: 12px; min-width: 140px;
            }
        """)
        body_layout.addWidget(make_setting_row("默认供应商", self._settings_default_provider))

        # 自动启动
        self._settings_autostart = QCheckBox("开机自动启动")
        self._settings_autostart.setStyleSheet("color: #E0E0E0; font-size: 12px;")
        body_layout.addWidget(make_setting_row("启动", self._settings_autostart))

        # 声音提醒
        self._settings_sound = QCheckBox("新消息声音提醒")
        self._settings_sound.setChecked(True)
        self._settings_sound.setStyleSheet("color: #E0E0E0; font-size: 12px;")
        body_layout.addWidget(make_setting_row("提醒", self._settings_sound))

        # 端口
        self._settings_port = QLineEdit("18080")
        self._settings_port.setFixedWidth(120)
        self._settings_port.setStyleSheet("""
            QLineEdit {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 4px;
                padding: 4px 8px; font-size: 12px;
            }
        """)
        body_layout.addWidget(make_setting_row("API 端口", self._settings_port))

        # 路径
        self._settings_data_dir = QLineEdit(str(Path("data")))
        self._settings_data_dir.setFixedWidth(280)
        self._settings_data_dir.setStyleSheet("""
            QLineEdit {
                background-color: #0e0e0e; color: #E0E0E0;
                border: 1px solid #333; border-radius: 4px;
                padding: 4px 8px; font-size: 12px;
            }
        """)
        body_layout.addWidget(make_setting_row("数据目录", self._settings_data_dir))

        body_layout.addStretch()

        # 保存按钮
        btn_save = QPushButton("💾 保存设置")
        btn_save.setStyleSheet("""
            QPushButton {
                background-color: #10B981; color: #FFFFFF;
                border: none; border-radius: 6px;
                padding: 8px 18px; font-size: 13px; font-weight: bold;
            }
            QPushButton:hover { background-color: #059669; }
        """)
        btn_save.clicked.connect(self._settings_save_clicked)
        body_layout.addWidget(btn_save, 0, Qt.AlignmentFlag.AlignRight)

        scroll.setWidget(scroll_body)
        layout.addWidget(scroll, 1)

        return page

    def _create_home_page(self):
        """创建运行服务页面 - 仅管理服务状态、日志、启动/停止按钮
        不再包含并排 AI 编程工作台（那已在"任务对话"页面全屏显示）。
        """
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setSpacing(0)
        page_layout.setContentsMargins(0, 0, 0, 0)

        # ── 运行服务卡片区域 ──
        cards_frame = QFrame()
        cards_frame.setStyleSheet("QFrame { background-color: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 8px; margin: 12px; padding: 12px; }")
        cards_layout = QVBoxLayout(cards_frame)
        cards_layout.setSpacing(8)
        cards_layout.setContentsMargins(12, 12, 12, 12)

        # 卡片区域标题
        cards_title_row = QHBoxLayout()
        cards_title = QLabel("🚀 服务管理")
        cards_title.setFont(QFont("Microsoft YaHei", 14, QFont.Weight.Bold))
        cards_title.setStyleSheet("color: #FFFFFF; background: transparent; border: none;")
        cards_title_row.addWidget(cards_title)
        cards_title_row.addStretch()

        self.home_status = QLabel("🟢 就绪")
        self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")
        cards_title_row.addWidget(self.home_status)
        cards_layout.addLayout(cards_title_row)
        cards_layout.addSpacing(12)

        # ── 服务定义 ──
        self._services_info = {
            "qwen2api": {
                "label": "千问 API 代理",
                "icon": "🤖",
                "color": "#388E3C",
                "hover_color": "#4CAF50",
                "default_port": 7777,
                "desc": "Qwen 系列模型 API 代理服务",
            },
            "zhipu2api": {
                "label": "智谱 API 代理",
                "icon": "🧠",
                "color": "#1565C0",
                "hover_color": "#1976D2",
                "default_port": 7780,
                "desc": "GLM 系列模型 API 代理服务",
            },
        }

        # 创建各个服务卡片
        for sid, info in self._services_info.items():
            card = ServiceCard(sid, info, self)
            card.setStyleSheet("QFrame { background-color: #222222; border: 1px solid #333333; border-radius: 6px; }")
            card.restart_clicked.connect(self._on_service_restart)
            card.open_clicked.connect(self._on_service_open)
            card.stop_clicked.connect(self._on_service_stop)
            cards_layout.addWidget(card)
            self.service_cards[sid] = card

        # ── 全局操作按钮行 ──
        btn_row = QHBoxLayout()
        btn_row.setSpacing(12)

        self.btn_start_all = QPushButton("▶ 启动全部服务")
        self.btn_start_all.setStyleSheet("""
            QPushButton {
                background-color: #2E7D32; color: #FFFFFF;
                border: 1px solid #388E3C; border-radius: 8px;
                padding: 12px 0; font-size: 14px; font-weight: bold;
            }
            QPushButton:hover { background-color: #388E3C; }
            QPushButton:pressed { background-color: #1B5E20; }
            QPushButton:disabled { background-color: #1a1a1a; color: #555; border-color: #333; }
        """)
        self.btn_start_all.clicked.connect(self._start_all_services)
        btn_row.addWidget(self.btn_start_all)

        self.btn_stop_all = QPushButton("⏹ 停止全部服务")
        self.btn_stop_all.setEnabled(False)
        self.btn_stop_all.setStyleSheet("""
            QPushButton {
                background-color: #C62828; color: #FFFFFF;
                border: 1px solid #D32F2F; border-radius: 8px;
                padding: 12px 0; font-size: 14px; font-weight: bold;
            }
            QPushButton:hover { background-color: #D32F2F; }
            QPushButton:disabled { background-color: #1a1a1a; color: #555; border-color: #333; }
        """)
        self.btn_stop_all.clicked.connect(self._stop_all_services)
        btn_row.addWidget(self.btn_stop_all)

        cards_layout.addLayout(btn_row, 0)
        cards_layout.addSpacing(16)

        # ── 运行日志面板 ──
        log_frame = QFrame()
        log_frame.setStyleSheet("QFrame { background-color: #111; border: 1px solid #222; border-radius: 8px; margin: 0 12px 12px 12px; padding: 8px; }")
        log_layout = QVBoxLayout(log_frame)
        log_layout.setSpacing(4)
        log_layout.setContentsMargins(8, 6, 8, 6)

        log_header = QHBoxLayout()
        log_title = QLabel("📋 运行日志")
        log_title.setStyleSheet("color: #888; font-size: 11px; font-weight: bold; background: transparent; border: none;")
        log_header.addWidget(log_title)
        log_header.addStretch()

        clear_btn = QPushButton("🗑 清空")
        clear_btn.setStyleSheet("QPushButton { background: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 2px 8px; font-size: 10px; color: #888; } QPushButton:hover { background: #3a3a3a; color: #ccc; }")
        clear_btn.clicked.connect(lambda: self.log_text.clear() if self.log_text else None)
        log_header.addWidget(clear_btn)
        log_layout.addLayout(log_header)

        self.log_text = QTextEdit()
        self.log_text.setReadOnly(True)
        self.log_text.setMaximumHeight(120)
        self.log_text.setStyleSheet("""
            QTextEdit {
                background-color: #0a0a0a; color: #bbbbbb;
                border: 1px solid #1a1a1a; border-radius: 4px;
                font-family: Consolas, monospace; font-size: 11px;
                padding: 4px;
            }
        """)
        log_layout.addWidget(self.log_text, 0)

        page_layout.addWidget(cards_frame, 1)
        page_layout.addWidget(log_frame, 0)

        return page

    def _load_vue_into_chat_page(self):
        """将 Vue 前端界面加载到"任务对话"页面

        2026-06-16 修复：之前这里会 deleteLater() 任务对话页所有原生控件，
        然后塞入 QtWebView2 加载 Vue dist。但 QtWebView2 是第三方实验模块，
        经常失败/黑屏，导致整个任务对话页一片空白。
        现在改为：保留原生 PyQt6 任务对话页（带欢迎语 + 会话列表 + 输入框），
        放弃用 QtWebView2 替换。仅在 log 中告知。
        """
        dist_path = os.path.join(self.app_dir, "desktop", "dist", "index.html")
        if not os.path.exists(dist_path):
            self._append_log("前端未构建，已使用原生 PyQt6 任务对话页", "#FFC107")
        else:
            self._append_log(
                "✓ 使用原生 PyQt6 任务对话页（QtWebView2 实验模块不稳定，已弃用）",
                "#4CAF50",
            )
        # 关键：什么都不做！保留原生 PyQt6 任务对话页。
        return
        vue_layout.setSpacing(0)
        vue_layout.setContentsMargins(0, 0, 0, 0)

        # WebView2 控件
        try:
            from qtwebview2 import QtWebView2Widget

            # 准备 JS APIs dict
            js_apis = {}
            for name in dir(self.bridge):
                if name.startswith('_'):
                    continue
                attr = getattr(self.bridge, name)
                if callable(attr):
                    js_apis[name] = attr

            # 创建 WebView2 widget
            self._chat_vue2_widget = QtWebView2Widget(
                url=None,
                js_apis=js_apis,
                parent=vue_container,
                debug=False,
            )
            # 加载本地文件
            url = QUrl.fromLocalFile(dist_path)
            self._chat_vue2_widget.load_url(url.toString())

            # 添加到布局
            vue_layout.addWidget(self._chat_vue2_widget, 1)

            # ── 注入 JS 桥接层 ──
            shim_js = """
            (function() {
                function waitForBridge() {
                    if (window.qtwebview2 && window.qtwebview2.api) {
                        if (!window.pywebview) window.pywebview = {};
                        window.pywebview.api = window.qtwebview2.api;
                        window.desktopApi = window.qtwebview2.api;
                        if (typeof pywebviewReady === 'function') {
                            pywebviewReady();
                        }
                    } else {
                        setTimeout(waitForBridge, 100);
                    }
                }
                waitForBridge();
            })();
            """
            # 页面加载完成后注入 shim
            QTimer.singleShot(1000, lambda: self._chat_vue2_widget.evaluate_js(shim_js))

            # ── 将 Vue 容器放入任务对话页面 ──
            chat_page_layout = self.chat_page.layout()
            if chat_page_layout:
                # 清除原有的提示文字
                for i in reversed(range(chat_page_layout.count())):
                    w = chat_page_layout.itemAt(i).widget()
                    if w:
                        w.deleteLater()
                # 将 Vue 容器放入
                chat_page_layout.addWidget(vue_container, 1)

            self._chat_vue_container = vue_container
            self._chat_vue2_widget_ref = self._chat_vue2_widget
            self._webview2_initialized = True
            self._append_log("✓ Vue 前端界面已加载", "#4CAF50")

            # 同时更新运行服务页面的状态
            self.home_status.setText("🟢 Vue 前端已就绪")
            self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")

        except Exception as e:
            self._append_log(f"Vue 前端加载失败: {e}", "#F44336")
            import traceback
            self._append_log(traceback.format_exc(), "#FF5722")
            return

    def _start_all_services(self):
        """启动全部服务（用户手动触发）"""
        if self._is_starting_all:
            return
        self._is_starting_all = True
        self.btn_start_all.setEnabled(False)
        self.home_status.setText("🔄 启动中...")
        self.home_status.setStyleSheet("color: #FFC107; font-size: 12px; font-weight: bold; background: transparent; border: none;")

        self._append_log("正在启动全部服务...", "#4CAF50")

        # 设置所有卡片为"启动中"
        for card in self.service_cards.values():
            card.set_starting()

        # 在后台线程中依次启动服务
        def _do_start():
            # 1. 启动 qwen2api
            self._start_api_service("qwen2api")
            # 2. 启动 zhipu2api
            self._start_api_service("zhipu2api")
            # 3. 启动前端
            QTimer.singleShot(500, self._load_vue_into_chat_page)

        t = threading.Thread(target=_do_start, daemon=True)
        t.start()

        # 启动服务状态监控
        self.service_monitor_timer.start(3000)

    def _stop_all_services(self):
        """停止全部服务"""
        self._append_log("正在停止全部服务...", "#FFC107")
        self.btn_stop_all.setEnabled(False)

        def _do_stop():
            # 停止 API 服务
            try:
                backend.stop_qwen2api()
            except Exception as e:
                self._append_log(f"停止千问服务异常: {e}", "#F44336")

            try:
                backend.stop_zhipu2api()
            except Exception as e:
                self._append_log(f"停止智谱服务异常: {e}", "#F44336")

            # 停止前端服务
            self._stop_frontend_service()

            QTimer.singleShot(1000, self._on_all_services_stopped)

        t = threading.Thread(target=_do_stop, daemon=True)
        t.start()

    def _on_all_services_stopped(self):
        """所有服务停止后的回调"""
        self._is_starting_all = False
        for card in self.service_cards.values():
            card.set_running(False)
        self.btn_start_all.setEnabled(True)
        self.btn_stop_all.setEnabled(False)
        self.home_status.setText("🟢 就绪")
        self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")
        self.service_monitor_timer.stop()
        self._append_log("全部服务已停止", "#FFC107")

    def _start_api_service(self, service_id: str):
        """启动单个 API 服务（qwen2api 或 zhipu2api）"""
        reg = backend.API_SERVICE_REGISTRY.get(service_id)
        if not reg:
            self._append_log(f"未知服务: {service_id}", "#F44336")
            return

        label = reg["label"]
        port = reg["default_port"]

        # 先检查是否已经在运行
        base_url = f"http://127.0.0.1:{port}"
        chk = backend.check_api_service(base_url)
        if chk.get("running") and chk.get("serviceType") == reg["service_type"]:
            self._append_log(f"{label} 服务已在运行 (:{port})", "#4CAF50")
            QTimer.singleShot(0, lambda sid=service_id: self.service_cards[sid].set_running(True))
            return

        self._append_log(f"正在启动 {label} 服务...", "#2196F3")

        try:
            if service_id == "qwen2api":
                result = backend.start_qwen2api(port=port)
            elif service_id == "zhipu2api":
                result = backend.start_zhipu2api(port=port)
            else:
                return

            if result.get("ok"):
                self._append_log(f"✓ {label} 服务启动成功 (:{port})", "#4CAF50")
                QTimer.singleShot(0, lambda sid=service_id: self.service_cards[sid].set_running(True))
            else:
                error = result.get("error", "未知错误")
                self._append_log(f"✗ {label} 服务启动失败: {error}", "#F44336")
                QTimer.singleShot(0, lambda sid=service_id: self.service_cards[sid].set_running(False))
        except Exception as e:
            self._append_log(f"✗ {label} 服务启动异常: {e}", "#F44336")
            QTimer.singleShot(0, lambda sid=service_id: self.service_cards[sid].set_running(False))

    def _start_frontend_service(self):
        """启动前端 - 内嵌到首页（QtWebView2）"""
        # 设置前端卡片为运行中
        card = self.service_cards.get("frontend")
        if card:
            card.set_running(True)

        self._append_log("正在加载 Vue 前端界面...", "#2196F3")
        # 加载 Vue 到首页
        QTimer.singleShot(1000, self._load_vue_into_chat_page)

    def _stop_frontend_service(self):
        """停止前端服务 - 关闭 Vue 视图"""
        self._close_vue_view()

    def _finish_start_all(self):
        """启动全部服务后的最终状态更新"""
        self._is_starting_all = False
        self.btn_start_all.setEnabled(True)

        # 检查有多少服务真正在运行
        running_count = sum(1 for c in self.service_cards.values() if c.is_running)
        if running_count > 0:
            self.btn_stop_all.setEnabled(True)
            self.home_status.setText(f"🟢 {running_count} 个服务运行中")
            self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")
        else:
            self.home_status.setText("🟡 就绪（无服务运行）")
            self.home_status.setStyleSheet("color: #FFC107; font-size: 12px; font-weight: bold; background: transparent; border: none;")

    def _on_service_restart(self, service_id: str):
        """重启单个服务"""
        self._append_log(f"正在重启 {service_id}...", "#2196F3")
        card = self.service_cards.get(service_id)
        if card:
            card.set_starting()

        def _do_restart():
            if service_id == "qwen2api":
                backend.stop_qwen2api()
                time.sleep(1)
                self._start_api_service("qwen2api")
            elif service_id == "zhipu2api":
                backend.stop_zhipu2api()
                time.sleep(1)
                self._start_api_service("zhipu2api")
            elif service_id == "frontend":
                self._start_frontend_service()

        t = threading.Thread(target=_do_restart, daemon=True)
        t.start()

    def _on_service_open(self, service_id: str):
        """打开服务页面"""
        if service_id in ("qwen2api", "zhipu2api"):
            import webbrowser
            port = self._services_info[service_id]["default_port"]
            webbrowser.open(f"http://127.0.0.1:{port}")
        elif service_id == "frontend":
            # 前端现在内嵌在首页，直接切换到底部
            if self._webview2_initialized:
                self._vue_tab.setStyleSheet("""
                    QPushButton { background-color: #1565C0; color: #FFFFFF; border: none; border-radius: 4px; padding: 4px 12px; font-size: 12px; font-weight: bold; }
                """)

    def _on_service_stop(self, service_id: str):
        """停止单个服务"""
        self._append_log(f"正在停止 {service_id}...", "#FFC107")

        def _do_stop():
            if service_id == "qwen2api":
                backend.stop_qwen2api()
            elif service_id == "zhipu2api":
                backend.stop_zhipu2api()
            elif service_id == "frontend":
                self._stop_frontend_service()

            QTimer.singleShot(500, self._monitor_services)

        t = threading.Thread(target=_do_stop, daemon=True)
        t.start()

    def _monitor_services(self):
        """定时监控服务状态，更新卡片和按钮"""
        running_count = 0
        for sid, info in self._services_info.items():
            card = self.service_cards.get(sid)
            if not card:
                continue

            if sid in ("qwen2api", "zhipu2api"):
                port = info["default_port"]
                base_url = f"http://127.0.0.1:{port}"
                try:
                    chk = backend.check_api_service(base_url)
                    is_running = chk.get("running", False)
                except Exception:
                    is_running = False
            elif sid == "frontend":
                is_running = hasattr(self, '_webview_window') and self._webview_window is not None
            else:
                is_running = False

            card.set_running(is_running)
            if is_running:
                running_count += 1

        # 更新全局状态
        if not self._is_starting_all:
            if running_count > 0:
                self.btn_start_all.setEnabled(True)
                self.btn_stop_all.setEnabled(True)
                self.home_status.setText(f"🟢 {running_count} 个服务运行中")
                self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")
            else:
                self.btn_start_all.setEnabled(True)
                self.btn_stop_all.setEnabled(False)
                self.home_status.setText("🟢 就绪")
                self.home_status.setStyleSheet("color: #4CAF50; font-size: 12px; font-weight: bold; background: transparent; border: none;")

    def _launch_webview(self):
        """启动 pywebview 独立窗口，加载 Vue 前端"""
        if not WEBVIEW_AVAILABLE:
            QMessageBox.critical(self, "错误", "pywebview 未安装，请运行 pip install pywebview")
            return

        dist_path = os.path.join(self.app_dir, "desktop", "dist", "index.html")
        if not os.path.exists(dist_path):
            QMessageBox.warning(self, "前端未构建", "请先在「部署维护」中构建前端")
            return

        def _run_webview():
            w = webview.create_window(
                "云集智能编程工作站", url=dist_path,
                js_api=self.bridge,
                width=1280, height=860,
                confirm_close=True,
            )
            self._webview_window = w
            webview.start()

        t = threading.Thread(target=_run_webview, daemon=False)
        t.start()

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

        # 按钮区域：环境检测 + 部署维护（左右结构）
        btn_row = QHBoxLayout()
        btn_row.setSpacing(12)

        self.btn_env_check = QPushButton("🔍 环境检测")
        self.btn_env_check.setStyleSheet("""
            QPushButton { background-color: #1565C0; border: 2px solid #1E88E5; border-radius: 6px; padding: 10px 20px; font-size: 13px; font-weight: bold; }
            QPushButton:hover { background-color: #1E88E5; }
            QPushButton:disabled { background-color: #333; color: #666; border-color: #444; }
        """)
        self.btn_env_check.clicked.connect(self._refresh_deploy_env_status)
        btn_row.addWidget(self.btn_env_check)

        self.btn_install_all = QPushButton("🚀 部署维护")
        self.btn_install_all.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: 2px solid #388E3C; border-radius: 6px; padding: 10px 20px; font-size: 13px; font-weight: bold; }
            QPushButton:hover { background-color: #388E3C; }
            QPushButton:disabled { background-color: #333; color: #666; border-color: #444; }
        """)
        self.btn_install_all.clicked.connect(self._on_deploy)
        btn_row.addWidget(self.btn_install_all)
        deploy_layout.addLayout(btn_row)

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
        """创建软件更新页面 - 标签页架构（参考云集智能文件清理专家）"""
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        top_bar_widget = QWidget()
        top_bar_widget.setStyleSheet("background-color: #1e1e1e;")
        top_bar = QHBoxLayout(top_bar_widget)
        top_bar.setContentsMargins(12, 8, 12, 8)

        title = QLabel("🔄 软件更新与版本管理")
        title.setFont(QFont("Microsoft YaHei", 13, QFont.Weight.Bold))
        title.setStyleSheet("color: #E0E0E0; border: none; background: transparent;")
        top_bar.addWidget(title)

        ver_label = QLabel(f"v{VERSION}")
        ver_label.setFont(QFont("Consolas", 11))
        ver_label.setStyleSheet("color: #666; border: none; background: transparent;")
        top_bar.addWidget(ver_label)

        top_bar.addStretch()

        self._ver_status_label = QLabel("")
        self._ver_status_label.setFont(QFont("Microsoft YaHei", 11))
        self._ver_status_label.setStyleSheet("color: #888; border: none; background: transparent;")
        top_bar.addWidget(self._ver_status_label)

        btn_check_remote = QPushButton("🔄 检查远程更新")
        btn_check_remote.setStyleSheet("""
            QPushButton { background-color: #333; border: 1px solid #444; border-radius: 4px; padding: 6px 14px; font-size: 12px; color: #ccc; }
            QPushButton:hover { background-color: #CC0000; border-color: #E00000; color: #fff; }
        """)
        btn_check_remote.clicked.connect(self._check_remote_versions)
        top_bar.addWidget(btn_check_remote)

        layout.addWidget(top_bar_widget)

        tab_bar = QHBoxLayout()
        tab_bar.setContentsMargins(10, 0, 10, 0)
        tab_bar.setSpacing(4)
        tab_bar_widget = QWidget()
        tab_bar_widget.setFixedHeight(44)
        tab_bar_widget.setStyleSheet("background-color: #1e1e1e;")
        tab_bar_widget.setLayout(tab_bar)

        self._ver_tab_stable_btn = QPushButton("📦 EXE稳定版")
        self._ver_tab_stable_btn.setFixedWidth(160)
        self._ver_tab_stable_btn.setFixedHeight(34)
        self._ver_tab_stable_btn.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
            QPushButton:hover { background-color: #388E3C; }
        """)
        self._ver_tab_stable_btn.clicked.connect(lambda: self._switch_ver_tab("stable"))
        tab_bar.addWidget(self._ver_tab_stable_btn)

        self._ver_tab_git_btn = QPushButton("🔀 Git开发版")
        self._ver_tab_git_btn.setFixedWidth(160)
        self._ver_tab_git_btn.setFixedHeight(34)
        self._ver_tab_git_btn.setStyleSheet("""
            QPushButton { background-color: #CC0000; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
            QPushButton:hover { background-color: #E00000; }
        """)
        self._ver_tab_git_btn.clicked.connect(lambda: self._switch_ver_tab("git"))
        tab_bar.addWidget(self._ver_tab_git_btn)

        tab_bar.addStretch()

        self._ver_expand_btn = QPushButton("📋 全部展开")
        self._ver_expand_btn.setFixedWidth(100)
        self._ver_expand_btn.setFixedHeight(30)
        self._ver_expand_btn.setStyleSheet("""
            QPushButton { background-color: #333; border: none; border-radius: 4px; font-size: 11px; color: #ccc; }
            QPushButton:hover { background-color: #CC0000; color: #fff; }
        """)
        self._ver_expand_btn.clicked.connect(self._toggle_expand_all)
        tab_bar.addWidget(self._ver_expand_btn)

        layout.addWidget(tab_bar_widget)

        self._ver_scroll = QScrollArea()
        self._ver_scroll.setWidgetResizable(True)
        self._ver_scroll.setFrameShape(QFrame.Shape.NoFrame)
        self._ver_scroll.setStyleSheet("QScrollArea { background-color: #1a1a1a; border: none; }")

        self._ver_scroll_container = QWidget()
        self._ver_scroll_container.setStyleSheet("background-color: #1a1a1a;")
        self._ver_scroll_layout = QVBoxLayout(self._ver_scroll_container)
        self._ver_scroll_layout.setSpacing(0)
        self._ver_scroll_layout.setContentsMargins(10, 6, 10, 6)

        self._ver_scroll.setWidget(self._ver_scroll_container)
        layout.addWidget(self._ver_scroll, 1)

        self._ver_stable_data = []
        self._ver_git_data = []
        self._ver_current_version = VERSION
        self._ver_active_tab = "stable"
        self._ver_info_text = "点击「🔄 检查远程更新」获取远程仓库最新版本"
        self._ver_expanded = False
        self._ver_info_label = None
        self._ver_stable_container = None
        self._ver_git_container = None

        self.update_log_text = QTextEdit()
        self.update_log_text.setReadOnly(True)
        self.update_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        self.update_log_text.hide()

        return page

    # ── 页面切换 ──

    def _update_nav_active(self, kind: str):
        """2026-06-16：workbuddy 边栏导航按钮互斥高亮"""
        if not hasattr(self, "_workbuddy_nav_btns"):
            return
        for k, btn in self._workbuddy_nav_btns.items():
            btn.setChecked(k == kind)

    def _switch_page(self, index):
        """切换页面"""
        # 2026-06-16 修复：补齐 4（项目管理）、5（系统设置）、6（对话历史）的处理
        if index == 4:
            # 项目管理：原生 QStackedWidget 切换
            self.btn_chat.setChecked(False)
            self.btn_home.setChecked(False)
            self.btn_deploy_nav.setChecked(False)
            self.btn_update_nav.setChecked(False)
            self.btn_project_nav.setChecked(True)
            self.btn_settings_nav.setChecked(False)
            self.page_stack.setCurrentIndex(4)
            self._refresh_project_list()
        elif index == 5:
            # 系统设置：原生 QStackedWidget 切换
            self.btn_chat.setChecked(False)
            self.btn_home.setChecked(False)
            self.btn_deploy_nav.setChecked(False)
            self.btn_update_nav.setChecked(False)
            self.btn_project_nav.setChecked(False)
            self.btn_settings_nav.setChecked(True)
            self.page_stack.setCurrentIndex(5)
            self._refresh_settings_panel()
        elif index == 6:
            # 2026-06-16 新增：对话历史页
            self.btn_chat.setChecked(False)
            self.btn_home.setChecked(False)
            self.btn_deploy_nav.setChecked(False)
            self.btn_update_nav.setChecked(False)
            self.btn_project_nav.setChecked(False)
            self.btn_settings_nav.setChecked(False)
            self.page_stack.setCurrentIndex(6)
            self._refresh_history_list()
        elif index == 3:
            # 软件更新
            self.btn_chat.setChecked(False)
            self.btn_home.setChecked(False)
            self.btn_deploy_nav.setChecked(False)
            self.btn_update_nav.setChecked(True)
            self.btn_project_nav.setChecked(False)
            self.btn_settings_nav.setChecked(False)
            self.page_stack.setCurrentIndex(3)
            self._render_active_tab()
        else:
            self.btn_chat.setChecked(index == 0)
            self.btn_home.setChecked(index == 1)
            self.btn_deploy_nav.setChecked(index == 2)
            self.btn_update_nav.setChecked(False)
            self.btn_project_nav.setChecked(False)
            self.btn_settings_nav.setChecked(False)
            self.page_stack.setCurrentIndex(index)

            if index == 2:
                self._refresh_deploy_env_status()

    def _switch_to_project_view(self):
        """切换到项目管理视图"""
        if self._webview2_initialized and hasattr(self, '_chat_vue2_widget') and self._chat_vue2_widget:
            try:
                self._chat_vue2_widget.evaluate_js(f"if(window.switchNav) window.switchNav('project');")
            except Exception:
                pass

    def _switch_ver_tab(self, tab):
        if tab == self._ver_active_tab:
            return
        self._ver_active_tab = tab
        if tab == "stable":
            self._ver_tab_stable_btn.setStyleSheet("""
                QPushButton { background-color: #2E7D32; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
                QPushButton:hover { background-color: #388E3C; }
            """)
            self._ver_tab_git_btn.setStyleSheet("""
                QPushButton { background-color: #CC0000; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
                QPushButton:hover { background-color: #E00000; }
            """)
        else:
            self._ver_tab_git_btn.setStyleSheet("""
                QPushButton { background-color: #2E7D32; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
                QPushButton:hover { background-color: #388E3C; }
            """)
            self._ver_tab_stable_btn.setStyleSheet("""
                QPushButton { background-color: #CC0000; border: none; border-radius: 6px; font-size: 13px; font-weight: bold; color: #fff; }
                QPushButton:hover { background-color: #E00000; }
            """)
        self._render_active_tab()

    def _render_active_tab(self):
        if not hasattr(self, '_ver_scroll_layout') or self._ver_scroll_layout is None:
            return
        while self._ver_scroll_layout.count():
            item = self._ver_scroll_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()
            elif item.layout():
                while item.layout().count():
                    sub = item.layout().takeAt(0)
                    if sub.widget():
                        sub.widget().deleteLater()
        if self._ver_active_tab == "stable":
            self._render_stable_tab()
        else:
            self._render_git_tab()

    def _toggle_expand_all(self):
        self._ver_expanded = not self._ver_expanded
        if self._ver_expanded:
            self._ver_expand_btn.setText("📋 全部折叠")
            self._ver_expand_btn.setStyleSheet("""
                QPushButton { background-color: #2E7D32; border: none; border-radius: 4px; font-size: 11px; color: #fff; }
                QPushButton:hover { background-color: #388E3C; }
            """)
        else:
            self._ver_expand_btn.setText("📋 全部展开")
            self._ver_expand_btn.setStyleSheet("""
                QPushButton { background-color: #333; border: none; border-radius: 4px; font-size: 11px; color: #ccc; }
                QPushButton:hover { background-color: #CC0000; color: #fff; }
            """)
        self._render_active_tab()

    def _render_stable_tab(self):
        info_frame = QFrame()
        info_frame.setStyleSheet("QFrame { background-color: #1e1e1e; border: 1px solid #333; border-radius: 6px; } QLabel { border: none; background: transparent; } QPushButton { border: none; }")
        info_layout = QVBoxLayout(info_frame)
        info_layout.setContentsMargins(12, 8, 12, 8)
        info_header = QHBoxLayout()
        self._ver_info_label = QLabel(self._ver_info_text)
        self._ver_info_label.setFont(QFont("Microsoft YaHei", 11))
        self._ver_info_label.setStyleSheet("color: #888; border: none; background: transparent;")
        self._ver_info_label.setWordWrap(True)
        info_header.addWidget(self._ver_info_label)
        info_header.addStretch()
        self._btn_pull_update = QPushButton("📥 更新资源包")
        self._btn_pull_update.setFixedHeight(28)
        self._btn_pull_update.setEnabled(False)
        self._btn_pull_update.setStyleSheet("""
            QPushButton { background-color: #2E7D32; border: none; border-radius: 4px; padding: 4px 14px; font-size: 12px; color: #fff; }
            QPushButton:hover { background-color: #388E3C; }
            QPushButton:disabled { background-color: #1a1a1a; color: #555; }
        """)
        self._btn_pull_update.clicked.connect(self._do_pull_update)
        if getattr(self, '_has_remote_update', False):
            self._btn_pull_update.setEnabled(True)
        info_header.addWidget(self._btn_pull_update)
        info_layout.addLayout(info_header)
        self._ver_scroll_layout.addWidget(info_frame)

        self._ver_stable_container = QWidget()
        self._ver_stable_container.setStyleSheet("background-color: transparent;")
        stable_layout = QVBoxLayout(self._ver_stable_container)
        stable_layout.setSpacing(4)
        stable_layout.setContentsMargins(0, 4, 0, 0)
        self._render_stable_versions(self._ver_stable_data, self._ver_current_version, stable_layout)
        self._ver_scroll_layout.addWidget(self._ver_stable_container)
        self._ver_scroll_layout.addStretch()

    def _render_git_tab(self):
        git_header = QWidget()
        git_header.setStyleSheet("background-color: transparent;")
        git_header_layout = QHBoxLayout(git_header)
        git_header_layout.setContentsMargins(4, 6, 4, 2)
        git_title = QLabel("🔀 Git 版本切换")
        git_title.setFont(QFont("Microsoft YaHei", 12, QFont.Weight.Bold))
        git_title.setStyleSheet("color: #42A5F5; border: none; background: transparent;")
        git_header_layout.addWidget(git_title)
        git_header_layout.addStretch()
        git_refresh_btn = QPushButton("刷新历史")
        git_refresh_btn.setFixedHeight(24)
        git_refresh_btn.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 4px 12px; font-size: 11px; color: #aaa; }
            QPushButton:hover { background-color: #3a3a3a; color: #fff; }
        """)
        git_refresh_btn.clicked.connect(self._refresh_git_history)
        git_header_layout.addWidget(git_refresh_btn)
        self._ver_scroll_layout.addWidget(git_header)

        self._ver_git_container = QWidget()
        self._ver_git_container.setStyleSheet("background-color: transparent;")
        self.git_history_layout = QVBoxLayout(self._ver_git_container)
        self.git_history_layout.setSpacing(4)
        self.git_history_layout.setContentsMargins(0, 0, 0, 0)
        self._render_git_history(self._ver_git_data)
        self._ver_scroll_layout.addWidget(self._ver_git_container)
        self._ver_scroll_layout.addStretch()

    def _load_all_versions(self):
        if hasattr(self, '_ver_status_label') and self._ver_status_label:
            self._ver_status_label.setText("加载中...")
        threading.Thread(target=self._do_load_all_versions, daemon=True).start()

    def _do_load_all_versions(self):
        stable_exes = self.updater.list_stable_exes()
        exe_versions = {e["version"]: e for e in stable_exes}
        local_versions = self.updater.get_local_version_history()
        git_commits = self.updater.get_git_history(30)
        current_version = VERSION
        local_ver_set = set()
        all_versions = []
        import re
        for v in local_versions:
            ver = v.get("version", v.get("version_number", ""))
            m = re.search(r'v?(\d+\.\d+\.\d+\.\d+)', ver)
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
        self._ver_stable_data = all_versions
        self._ver_git_data = git_commits
        self._ver_current_version = current_version
        QTimer.singleShot(0, self._render_active_tab)
        if hasattr(self, '_ver_status_label') and self._ver_status_label:
            QTimer.singleShot(0, lambda: self._ver_status_label.setText(f"稳定版 {len(all_versions)} 个 | Git提交 {len(git_commits)} 条"))

    def _check_remote_versions(self):
        if hasattr(self, '_ver_status_label') and self._ver_status_label:
            self._ver_status_label.setText("正在检查远程更新...")
        threading.Thread(target=self._do_check_remote, daemon=True).start()

    def _do_check_remote(self):
        try:
            remote_versions = self.updater.fetch_remote_version_history()
            if remote_versions is None:
                raise Exception("无法获取远程版本信息（Gitee API/Git/HTTP均不可用）")
            stable_exes = self.updater.list_stable_exes()
            exe_versions = {e["version"]: e for e in stable_exes}
            git_commits = self.updater.fetch_remote_commits(30)
            if git_commits is None:
                git_commits = self.updater.get_git_history(30)
            import re
            current_version = VERSION
            local_ver_set = set()
            all_versions = []
            for v in remote_versions:
                ver = v.get("version", v.get("version_number", ""))
                m = re.search(r'v?(\d+\.\d+\.\d+\.\d+)', ver)
                ver_num = m.group(1) if m else ver
                if not ver_num or ver_num in local_ver_set:
                    continue
                local_ver_set.add(ver_num)
                is_current = (ver_num == current_version)
                all_versions.append({
                    "version": ver_num,
                    "name": v.get("name", f"v{ver_num}"),
                    "changes": v.get("changes", []),
                    "build_time": v.get("build_time", v.get("date", "")),
                    "git_commit": v.get("git_commit", ""),
                    "available": ver_num in exe_versions,
                    "exe_info": exe_versions.get(ver_num),
                    "is_remote_new": not is_current,
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
            remote_latest = all_versions[0]["version"] if all_versions else ""
            has_update = remote_latest and remote_latest != VERSION
            if has_update:
                self._ver_info_text = f"🆕 发现新版本 v{remote_latest}，点击「📥 更新资源包」获取"
                self._has_remote_update = True
            else:
                self._ver_info_text = "✅ 已是最新版本"
                self._has_remote_update = False
            self._ver_stable_data = all_versions
            self._ver_git_data = git_commits
            self._ver_current_version = current_version
            QTimer.singleShot(0, self._render_active_tab)
            if hasattr(self, '_ver_status_label') and self._ver_status_label:
                QTimer.singleShot(0, lambda: self._ver_status_label.setText(f"稳定版 {len(all_versions)} 个 | Git提交 {len(git_commits)} 条"))
        except Exception as e:
            if hasattr(self, '_ver_status_label') and self._ver_status_label:
                QTimer.singleShot(0, lambda: self._ver_status_label.setText(f"检查失败: {e}"))

    def _refresh_git_history(self):
        """刷新Git历史记录"""
        git_commits = self.updater.get_git_history(30)
        self._ver_git_data = git_commits
        if self._ver_active_tab == "git":
            self._render_active_tab()

    def _render_git_history(self, git_commits):
        if not hasattr(self, 'git_history_layout') or self.git_history_layout is None:
            return
        while self.git_history_layout.count():
            item = self.git_history_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()
        if not self.updater.is_git_repo():
            no_git = QLabel("当前不是 Git 仓库，无法使用 Git 版本切换")
            no_git.setStyleSheet("color: #555; padding: 20px; border: none; background: transparent;")
            no_git.setAlignment(Qt.AlignmentFlag.AlignCenter)
            self.git_history_layout.addWidget(no_git)
            return
        if not git_commits:
            no_commits = QLabel("暂无 Git 提交记录")
            no_commits.setStyleSheet("color: #555; padding: 20px; border: none; background: transparent;")
            no_commits.setAlignment(Qt.AlignmentFlag.AlignCenter)
            self.git_history_layout.addWidget(no_commits)
            return
        current_commit = self.updater.get_current_commit()
        expanded = self._ver_expanded
        for commit in git_commits:
            is_current = (commit["hash"] == current_commit)
            card = QFrame()
            card.setObjectName("gitCommitCard")
            if is_current:
                card.setStyleSheet("""
                    #gitCommitCard { background-color: #152015; border: 1px solid #1f3a1f; border-radius: 6px; }
                    #gitCommitCard:hover { background-color: #1a2a1a; border-color: #2a4a2a; }
                    QLabel { border: none; background: transparent; }
                    QPushButton { border: none; }
                """)
            else:
                card.setStyleSheet("""
                    #gitCommitCard { background-color: #161616; border: 1px solid #2a2a2a; border-radius: 6px; }
                    #gitCommitCard:hover { background-color: #1c1c1c; border-color: #3a3a3a; }
                    QLabel { border: none; background: transparent; }
                    QPushButton { border: none; }
                """)
            cl = QVBoxLayout(card)
            cl.setSpacing(3)
            cl.setContentsMargins(10, 6, 10, 6)
            header = QHBoxLayout()
            header.setSpacing(8)
            hash_label = QLabel(commit["hash"])
            hash_label.setFont(QFont("Consolas", 11, QFont.Weight.Bold))
            hash_label.setStyleSheet(f"color: {'#4CAF50' if is_current else '#42A5F5'};")
            header.addWidget(hash_label)
            time_label = QLabel(commit.get("time", ""))
            time_label.setFont(QFont("Consolas", 9))
            time_label.setStyleSheet("color: #666;")
            header.addWidget(time_label)
            header.addStretch()
            if is_current:
                current_tag = QLabel("● 当前")
                current_tag.setFont(QFont("Microsoft YaHei", 10))
                current_tag.setStyleSheet("color: #4CAF50;")
                header.addWidget(current_tag)
            else:
                switch_btn = QPushButton("切换")
                switch_btn.setFixedWidth(50)
                switch_btn.setFixedHeight(22)
                switch_btn.setStyleSheet("""
                    QPushButton { background-color: #1e1e1e; border: 1px solid #2a2a2a; border-radius: 4px; font-size: 11px; color: #aaa; }
                    QPushButton:hover { background-color: #2a2a2a; border-color: #3a3a3a; color: #fff; }
                """)
                switch_btn.clicked.connect(lambda checked, h=commit["hash"]: self._switch_git_commit(h))
                header.addWidget(switch_btn)
            toggle_text = "▼" if expanded else "▶"
            toggle_btn = QPushButton(toggle_text)
            toggle_btn.setFixedWidth(24)
            toggle_btn.setFixedHeight(22)
            card_bg = "#152015" if is_current else "#161616"
            toggle_btn.setStyleSheet(f"""
                QPushButton {{ background-color: {card_bg}; border: none; border-radius: 3px; font-size: 10px; color: #888; }}
                QPushButton:hover {{ background-color: #2a2a2a; color: #fff; }}
            """)
            toggle_btn.clicked.connect(lambda checked, c=card, d=commit: self._toggle_card_detail(c, d, "git"))
            header.addWidget(toggle_btn)
            cl.addLayout(header)
            if expanded:
                msg_label = QLabel(commit["message"])
                msg_label.setFont(QFont("Microsoft YaHei", 10))
                msg_label.setStyleSheet("color: #ccc;")
                msg_label.setWordWrap(True)
                cl.addWidget(msg_label)
                author_label = QLabel(f"👤 {commit.get('author', '')}")
                author_label.setFont(QFont("Microsoft YaHei", 9))
                author_label.setStyleSheet("color: #666;")
                cl.addWidget(author_label)
            self.git_history_layout.addWidget(card)

    def _render_stable_versions(self, all_versions, current_version, layout):
        while layout.count():
            item = layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()
        if not all_versions:
            no_ver = QLabel("暂无稳定版本")
            no_ver.setStyleSheet("color: #555; padding: 10px; border: none; background: transparent;")
            no_ver.setAlignment(Qt.AlignmentFlag.AlignCenter)
            layout.addWidget(no_ver)
            return
        expanded = self._ver_expanded
        for v in all_versions:
            ver = v["version"]
            is_current = (ver == current_version)
            is_available = v.get("available", False)
            is_remote_new = v.get("is_remote_new", False)
            changes = v.get("changes", [])
            exe_info = v.get("exe_info")
            if is_current:
                card_bg = "#152015"
                border_color = "#1f3a1f"
            elif is_remote_new:
                card_bg = "#161620"
                border_color = "#1f3a4f"
            elif is_available:
                card_bg = "#161616"
                border_color = "#222"
            else:
                card_bg = "#111"
                border_color = "#1a1a1a"
            card = QFrame()
            card.setObjectName("stableCard")
            card.setStyleSheet(f"""
                #stableCard {{ background-color: {card_bg}; border: 1px solid {border_color}; border-radius: 6px; }}
                #stableCard:hover {{ background-color: #1c1c1c; }}
                QLabel {{ border: none; background: transparent; }}
                QPushButton {{ border: none; }}
            """)
            cl = QVBoxLayout(card)
            cl.setSpacing(3)
            cl.setContentsMargins(10, 6, 10, 6)
            header = QHBoxLayout()
            header.setSpacing(8)
            ver_color = "#4CAF50" if is_current else ("#42A5F5" if is_remote_new else ("#E0E0E0" if is_available else "#555"))
            ver_label = QLabel(f"v{ver}")
            ver_label.setFont(QFont("Consolas", 12, QFont.Weight.Bold))
            ver_label.setStyleSheet(f"color: {ver_color};")
            header.addWidget(ver_label)
            build_time = v.get("build_time", "")
            if build_time:
                try:
                    from datetime import datetime as _dt
                    dt = _dt.fromisoformat(build_time)
                    date_str = dt.strftime("%Y-%m-%d %H:%M")
                except:
                    date_str = build_time[:16]
                date_label = QLabel(date_str)
                date_label.setFont(QFont("Consolas", 10))
                date_label.setStyleSheet("color: #555;")
                header.addWidget(date_label)
            if is_remote_new:
                remote_tag = QLabel("🆕 远程新版本")
                remote_tag.setFont(QFont("Microsoft YaHei", 10))
                remote_tag.setStyleSheet("color: #42A5F5;")
                header.addWidget(remote_tag)
            elif is_available and exe_info:
                exe_tag = QLabel("📦 含EXE稳定版")
                exe_tag.setFont(QFont("Microsoft YaHei", 10))
                exe_tag.setStyleSheet("color: #FF9800;")
                header.addWidget(exe_tag)
                if exe_info.get("size_mb"):
                    size_label = QLabel(f"{exe_info['size_mb']}MB")
                    size_label.setFont(QFont("Consolas", 10))
                    size_label.setStyleSheet("color: #555;")
                    header.addWidget(size_label)
            elif not is_available:
                status_label = QLabel("未提供")
                status_label.setFont(QFont("Microsoft YaHei", 10))
                status_label.setStyleSheet("color: #444;")
                header.addWidget(status_label)
            header.addStretch()
            if is_current:
                current_tag = QLabel("● 当前版本")
                current_tag.setFont(QFont("Microsoft YaHei", 10))
                current_tag.setStyleSheet("color: #4CAF50;")
                header.addWidget(current_tag)
            elif is_available and exe_info:
                switch_btn = QPushButton("切换")
                switch_btn.setFixedWidth(50)
                switch_btn.setFixedHeight(22)
                switch_btn.setStyleSheet("""
                    QPushButton { background-color: #1e1e1e; border: 1px solid #2a2a2a; border-radius: 4px; font-size: 11px; color: #aaa; }
                    QPushButton:hover { background-color: #2a2a2a; border-color: #3a3a3a; color: #fff; }
                """)
                gc = v.get("git_commit", "")
                switch_btn.clicked.connect(lambda checked, p=exe_info["path"], c=gc: self.updater.switch_to_exe(p, c))
                header.addWidget(switch_btn)
            has_detail = bool(changes) or bool(v.get("git_commit", ""))
            if has_detail:
                toggle_text = "▼" if expanded else "▶"
                toggle_btn = QPushButton(toggle_text)
                toggle_btn.setFixedWidth(24)
                toggle_btn.setFixedHeight(22)
                toggle_btn.setStyleSheet(f"""
                    QPushButton {{ background-color: {card_bg}; border: none; border-radius: 3px; font-size: 10px; color: #888; }}
                    QPushButton:hover {{ background-color: #2a2a2a; color: #fff; }}
                """)
                toggle_btn.clicked.connect(lambda checked, c=card, d=v: self._toggle_card_detail(c, d, "stable"))
                header.addWidget(toggle_btn)
            cl.addLayout(header)
            if expanded and has_detail:
                detail = QWidget()
                detail.setStyleSheet("border: none; background: transparent;")
                dl = QVBoxLayout(detail)
                dl.setSpacing(2)
                dl.setContentsMargins(0, 4, 0, 0)
                git_commit = v.get("git_commit", "")
                if git_commit:
                    commit_label = QLabel(f"🔗 commit: {git_commit}")
                    commit_label.setFont(QFont("Consolas", 9))
                    commit_label.setStyleSheet("color: #555;")
                    dl.addWidget(commit_label)
                if changes:
                    for ch in changes[:3]:
                        ch_label = QLabel(f"· {ch}")
                        ch_label.setFont(QFont("Microsoft YaHei", 10))
                        ch_label.setStyleSheet("color: #777;")
                        ch_label.setWordWrap(True)
                        dl.addWidget(ch_label)
                    if len(changes) > 3:
                        more_label = QLabel(f"  +{len(changes)-3}项更多...")
                        more_label.setFont(QFont("Microsoft YaHei", 10))
                        more_label.setStyleSheet("color: #444;")
                        dl.addWidget(more_label)
                else:
                    no_ch = QLabel("暂无修改记录")
                    no_ch.setFont(QFont("Microsoft YaHei", 10))
                    no_ch.setStyleSheet("color: #3a3a3a;")
                    dl.addWidget(no_ch)
                cl.addWidget(detail)
            layout.addWidget(card)

    def _toggle_card_detail(self, card, data, card_type):
        cl = card.layout()
        if cl is None:
            return
        for i in range(cl.count()):
            item = cl.itemAt(i)
            if item and item.widget() and item.widget().property("_is_detail"):
                item.widget().deleteLater()
                for j in range(cl.count()):
                    h_item = cl.itemAt(j)
                    if h_item and h_item.layout():
                        for k in range(h_item.layout().count()):
                            btn_item = h_item.layout().itemAt(k)
                            if btn_item and btn_item.widget() and isinstance(btn_item.widget(), QPushButton):
                                btn_text = btn_item.widget().text()
                                if btn_text in ("▼", "▶"):
                                    btn_item.widget().setText("▶")
                return
        card_bg = "#152015" if card_type == "stable" and data.get("version") == self._ver_current_version else "#161616"
        detail = QWidget()
        detail.setProperty("_is_detail", True)
        detail.setStyleSheet("border: none; background: transparent;")
        dl = QVBoxLayout(detail)
        dl.setSpacing(2)
        dl.setContentsMargins(0, 4, 0, 0)
        if card_type == "stable":
            git_commit = data.get("git_commit", "")
            if git_commit:
                commit_label = QLabel(f"🔗 commit: {git_commit}")
                commit_label.setFont(QFont("Consolas", 9))
                commit_label.setStyleSheet("color: #555;")
                dl.addWidget(commit_label)
            changes = data.get("changes", [])
            if changes:
                for ch in changes[:3]:
                    ch_label = QLabel(f"· {ch}")
                    ch_label.setFont(QFont("Microsoft YaHei", 10))
                    ch_label.setStyleSheet("color: #777;")
                    ch_label.setWordWrap(True)
                    dl.addWidget(ch_label)
                if len(changes) > 3:
                    more_label = QLabel(f"  +{len(changes)-3}项更多...")
                    more_label.setFont(QFont("Microsoft YaHei", 10))
                    more_label.setStyleSheet("color: #444;")
                    dl.addWidget(more_label)
            else:
                no_ch = QLabel("暂无修改记录")
                no_ch.setFont(QFont("Microsoft YaHei", 10))
                no_ch.setStyleSheet("color: #3a3a3a;")
                dl.addWidget(no_ch)
        else:
            msg_label = QLabel(data["message"])
            msg_label.setFont(QFont("Microsoft YaHei", 10))
            msg_label.setStyleSheet("color: #ccc;")
            msg_label.setWordWrap(True)
            dl.addWidget(msg_label)
            author_label = QLabel(f"👤 {data.get('author', '')}")
            author_label.setFont(QFont("Microsoft YaHei", 9))
            author_label.setStyleSheet("color: #666;")
            dl.addWidget(author_label)
        cl.addWidget(detail)
        for j in range(cl.count()):
            h_item = cl.itemAt(j)
            if h_item and h_item.layout():
                for k in range(h_item.layout().count()):
                    btn_item = h_item.layout().itemAt(k)
                    if btn_item and btn_item.widget() and isinstance(btn_item.widget(), QPushButton):
                        btn_text = btn_item.widget().text()
                        if btn_text in ("▼", "▶"):
                            btn_item.widget().setText("▼")

    def _switch_git_commit(self, commit_hash):
        """切换到指定Git提交"""
        self._append_log(f"正在切换到 Git commit {commit_hash}...", "#FF9800")
        success = self.updater.switch_git_commit(commit_hash)
        if success:
            self._append_log("✓ Git 版本切换成功，请重启应用以加载新资源包", "#4CAF50")
            self._refresh_git_history()

    def _do_pull_update(self):
        """执行资源包更新 - 双通道：git pull → Gitee API下载"""
        self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
        self.updater.progress = lambda p, l: self.progress_signal.emit(p, l)
        if hasattr(self, '_btn_pull_update') and self._btn_pull_update:
            self._btn_pull_update.setEnabled(False)
        self._append_log("正在更新资源包...", "#FF9800")

        def _pull():
            success = False
            if self.updater.is_git_repo():
                self.log_signal.emit("使用 Git 更新...", "#FF9800")
                success = self.updater.pull_update()
            if not success:
                self.log_signal.emit("Git 不可用，使用 Gitee API 下载更新包...", "#FF9800")
                success = self.updater.download_update_via_gitee(
                    progress_func=lambda p, l: self.progress_signal.emit(p, l)
                )
            if success:
                self.log_signal.emit("✓ 资源包更新完成", "#4CAF50")
                self._has_remote_update = False
                self._ver_info_text = "✅ 已是最新版本"
            else:
                self.log_signal.emit("✗ 资源包更新失败", "#F44336")
            QTimer.singleShot(500, self._check_remote_versions)

        threading.Thread(target=_pull, daemon=True).start()

    # ── 环境检查与自动加载 ──
    def _auto_check_and_load(self):
        def _check():
            checks = self.installer.check_all()

            parts = []
            for k, v in checks.items():
                parts.append(f"{k}:{'✓' if v else '✗'}")
            self.log_signal.emit("环境检查: " + " ".join(parts), "#666")

            # 自动启动基础服务（千问 + 智谱 API）
            QTimer.singleShot(500, self._auto_start_services)
            
            QTimer.singleShot(100, self._load_frontend)
            QTimer.singleShot(200, self._refresh_deploy_env_status)

        t = threading.Thread(target=_check, daemon=True)
        t.start()

    def _auto_start_services(self):
        """自动启动基础 API 服务（千问 + 智谱）"""
        try:
            self._start_api_service("qwen2api")
            self._start_api_service("zhipu2api")
            # 延迟 1.5 秒后加载 Vue 前端
            QTimer.singleShot(1500, self._load_vue_into_chat_page)
            self.home_status.setText("🟢 服务已就绪")
        except Exception as e:
            self._append_log(f"自动启动服务失败: {e}", "#FF9800")

    def _splash_fallback(self):
        if self._splash and self._splash.isVisible():
            self.show()
            self._splash.finish(self)
            self._splash = None

    def _load_frontend(self):
        """检查前端是否已构建并更新状态"""
        dist_path = os.path.join(self.app_dir, "desktop", "dist", "index.html")
        if not os.path.exists(dist_path):
            self._update_status("⚠ 前端未构建")
            self.log_signal.emit("前端未构建，请先运行部署维护", "#FFC107")
            return
        self._update_status("🟢 就绪（点击首页「一键启动」启动工作台）")

        # 更新环境状态栏
        self._update_env_status()

    def _update_env_status(self):
        checks = self.installer.check_all()
        parts = []
        labels = {"node": "Node", "bun": "Bun", "deps": "依赖", "dist": "前端", "electron": "Electron", "qwen2api": "API"}
        for k, v in checks.items():
            parts.append(f"{labels.get(k, k)}:{'✓' if v else '✗'}")
        self.env_status.setText("环境: " + " | ".join(parts))

    def _on_js_console(self, level, message, line, source):
        level_map = {0: "INFO", 1: "WARN", 2: "ERROR", 3: "DEBUG"}
        lvl = level_map.get(level, str(level))
        color = "#F44336" if level == 2 else "#FF9800" if level == 1 else "#888"
        short_src = source.split("/")[-1] if source else "?"
        self.debug_log_signal.emit(f"[JS {lvl}] {message} ({short_src}:{line})", color)

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

    def _on_voice_result(self, text: str):
        """语音识别结果回调"""
        self._pending_voice_result = text
        try:
            escaped = json.dumps(text)
            if self._webview_window:
                self._webview_window.evaluate_js(f"if(window.setVoiceResult) window.setVoiceResult({escaped});")
        except Exception:
            pass

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
    # ── 2026-06-16 新增：原生任务对话/项目管理/系统设置 事件处理 ──

    def _chat_new_session(self):
        """新建会话：弹输入框取标题"""
        title, ok = QInputDialog.getText(self, "新建对话", "请输入对话标题：", text="新对话")
        if not ok or not title.strip():
            return
        sid = datetime.now().strftime("%Y%m%d%H%M%S")
        sess = {"id": sid, "title": title.strip(), "created_at": datetime.now().isoformat(timespec="seconds"), "messages": []}
        self._chat_sessions.insert(0, sess)
        self._chat_session_list.addItem(QListWidgetItem(f"💬 {sess['title']}\n   {sid}"))
        self._chat_current_session_id = sid
        self._chat_current_title.setText(sess["title"])
        self._chat_messages_area.clear()
        self._chat_input.setFocus()
        self._append_log(f"已创建对话: {sess['title']}", "#10B981")

    def _chat_load_session(self, item):
        """加载左侧选中的会话"""
        try:
            idx = self._chat_session_list.row(item)
            sess = self._chat_sessions[idx]
        except (IndexError, AttributeError):
            return
        self._chat_current_session_id = sess["id"]
        self._chat_current_title.setText(sess["title"])
        # 渲染历史消息
        html = ""
        for m in sess.get("messages", []):
            role = m.get("role", "assistant")
            text = m.get("text", "")
            if role == "user":
                html += f'<div style="margin: 8px 0; padding: 10px 14px; background: #1E3A8A; color: #fff; border-radius: 8px; max-width: 75%; margin-left: auto;">{text}</div>'
            else:
                html += f'<div style="margin: 8px 0; padding: 10px 14px; background: #1f1f1f; color: #E0E0E0; border-radius: 8px; max-width: 75%;">{text}</div>'
        self._chat_messages_area.setHtml(html or '<div style="color: #888; text-align: center; margin-top: 40px;">（空对话）</div>')

    def _chat_send_message(self):
        """发送消息：先存到当前会话，再用后端做 AI 响应（如果有）"""
        text = self._chat_input.toPlainText().strip()
        if not text:
            return
        if not self._chat_current_session_id:
            # 没有选中的会话，先创建一个
            self._chat_new_session()
        # 找到当前会话
        sess = None
        for s in self._chat_sessions:
            if s["id"] == self._chat_current_session_id:
                sess = s
                break
        if sess is None:
            self._append_log("当前会话无效，请重新选择", "#FFC107")
            return
        sess.setdefault("messages", []).append({"role": "user", "text": text, "time": datetime.now().isoformat(timespec="seconds")})
        # 渲染
        self._chat_load_session(self._chat_session_list.currentItem() or self._chat_session_list.item(0))
        self._chat_input.clear()
        # AI 响应占位
        sess["messages"].append({
            "role": "assistant",
            "text": f"（本地占位响应）你刚说了：{text}\n\n要启用真实 AI 响应，请在「运行服务」页启动 qwen2api / zhipu2api 后端服务。",
            "time": datetime.now().isoformat(timespec="seconds"),
        })
        self._chat_load_session(self._chat_session_list.currentItem() or self._chat_session_list.item(0))
        self._append_log(f"[chat] {text[:30]}...", "#9CA3AF")

    def _refresh_project_list(self):
        """刷新项目列表（占位实现，列出 data/ 下的子目录作为项目）"""
        if not hasattr(self, "_project_list"):
            return
        self._project_list.clear()
        data_dir = Path("data")
        candidates = []
        if data_dir.exists():
            for child in sorted(data_dir.iterdir()):
                if child.is_dir() and not child.name.startswith("."):
                    candidates.append(child)
        # 兜底：根目录 + desktop
        for p in [Path("app"), Path("ver")]:
            if p.exists() and p not in candidates:
                candidates.append(p)
        if not candidates:
            self._proj_status.setText("未发现项目。可点击右上角「+ 新建项目」创建。")
            placeholder = QListWidgetItem("（暂无项目）")
            placeholder.setFlags(Qt.ItemFlag.NoItemFlags)
            self._project_list.addItem(placeholder)
            return
        for proj in candidates:
            stat = proj.stat()
            item = QListWidgetItem(f"📁 {proj.name}/\n   {proj}")
            item.setData(Qt.ItemDataRole.UserRole, str(proj))
            self._project_list.addItem(item)
        self._proj_status.setText(f"共 {len(candidates)} 个项目")

    def _project_filter_changed(self, text):
        """项目搜索过滤"""
        if not hasattr(self, "_project_list"):
            return
        for i in range(self._project_list.count()):
            item = self._project_list.item(i)
            item.setHidden(bool(text) and text.lower() not in item.text().lower())

    def _project_create_dialog(self):
        """新建项目对话框"""
        name, ok = QInputDialog.getText(self, "新建项目", "项目名：", text="")
        if not ok or not name.strip():
            return
        target = Path("data") / name.strip()
        try:
            target.mkdir(parents=True, exist_ok=True)
            (target / "README.md").write_text(f"# {name}\n\n由云集智能编程工作站在 {datetime.now().isoformat(timespec='seconds')} 创建。\n", encoding="utf-8")
            self._append_log(f"已创建项目: {name}", "#10B981")
            self._refresh_project_list()
        except Exception as e:
            QMessageBox.critical(self, "创建失败", str(e))

    def _refresh_settings_panel(self):
        """刷新系统设置面板（从配置文件加载）"""
        # 占位实现：未来接入 config_service
        self._append_log("已切换到系统设置", "#9CA3AF")

    def _settings_save_clicked(self):
        """保存设置"""
        cfg = {
            "theme": self._settings_theme.currentText(),
            "default_provider": self._settings_default_provider.currentText(),
            "autostart": self._settings_autostart.isChecked(),
            "sound": self._settings_sound.isChecked(),
            "api_port": self._settings_port.text(),
            "data_dir": self._settings_data_dir.text(),
        }
        try:
            cfg_path = Path("data") / "settings.json"
            cfg_path.parent.mkdir(parents=True, exist_ok=True)
            cfg_path.write_text(json.dumps(cfg, ensure_ascii=False, indent=2), encoding="utf-8")
            self._append_log(f"设置已保存到 {cfg_path}", "#10B981")
            QMessageBox.information(self, "保存成功", f"设置已保存到\n{cfg_path}")
        except Exception as e:
            QMessageBox.critical(self, "保存失败", str(e))

    def closeEvent(self, event):
        _cleanup_single_instance()
        event.accept()


# ══════════════════════════════════════════════════════════════
# ── 自部署机制 (参考云集智能视频创意站) ──
# ══════════════════════════════════════════════════════════════
# 首次运行: 单 EXE → 自部署到同级目录 → 启动稳定入口 EXE
# 后续运行: 直接启动已部署的稳定 EXE
#
# 部署后目录结构:
#   云集智能编程工作站/
#   ├── .yunji.lock
#   ├── 云集智能编程工作站.exe              ← 稳定入口 (硬链接)
#   ├── ver/
#   │   └── 云集智能编程工作站-v2026.xx.xx.xxxx.exe
#   ├── app/                      ← 从 EXE 内嵌释放的资源
#   │   ├── desktop/dist/
#   │   ├── backend.py
#   │   ├── api/  services/  routes/  platformkit/
#   │   └── ...
#   ├── data/
#   └── temp/
# ══════════════════════════════════════════════════════════════

BRAND_NAME = "云集智能编程工作站"
LOCK_FILE_NAME = ".yunji.lock"


def _create_hardlink(src, dst):
    """创建硬链接，失败则回退到复制"""
    try:
        if os.path.exists(dst):
            os.remove(dst)
        os.link(src, dst)
        return True
    except OSError:
        pass
    try:
        if os.path.exists(dst):
            os.remove(dst)
        shutil.copy2(src, dst)
        return True
    except Exception:
        return False


def _find_install_root(start_dir=None):
    """从指定目录向上查找包含 app/ 子目录的安装根"""
    d = start_dir or (
        os.path.dirname(sys.executable) if getattr(sys, 'frozen', False)
        else os.path.dirname(os.path.abspath(__file__))
    )
    for _ in range(5):
        if os.path.isdir(os.path.join(d, "app")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            break
        d = parent
    return None


def _self_deploy(exe_dir):
    """
    自部署: 将单 EXE 展开为完整目录结构

    首次运行时:
    1. 创建 deploy_dir/ver/ 和 data/、temp/
    2. 从 _MEIPASS 释放嵌入资源到 deploy_dir/app/
    3. 写 .yunji.lock 标记
    4. 复制 EXE 到 ver/ 并创建硬链接作为稳定入口
    5. 启动稳定入口 EXE (带 --cleanup 清理原始 EXE)

    已部署时:
    直接返回 deploy_dir 路径
    """
    src_exe = os.path.abspath(sys.executable)
    exe_basename = os.path.basename(src_exe)

    if BRAND_NAME not in exe_basename:
        return None

    deploy_dir = os.path.join(exe_dir, BRAND_NAME)
    lock_path = os.path.join(deploy_dir, LOCK_FILE_NAME)
    already_deployed = os.path.isdir(deploy_dir) and os.path.isfile(lock_path)

    if already_deployed:
        entry_exe = os.path.join(deploy_dir, f"{BRAND_NAME}.exe")
        if os.path.isfile(entry_exe) and os.path.normpath(src_exe) != os.path.normpath(entry_exe):
            # 启动稳定入口，让它清理当前 EXE
            subprocess.Popen(
                f'ping -n 3 127.0.0.1 >nul & start "" "{entry_exe}" --cleanup="{src_exe}"',
                shell=True,
                creationflags=subprocess.CREATE_NO_WINDOW
            )
            # 释放单实例资源
            try:
                _cleanup_single_instance()
            except Exception:
                pass
            os._exit(0)
        return deploy_dir

    # ── 首次部署 ──
    os.makedirs(deploy_dir, exist_ok=True)

    ver_dir = os.path.join(deploy_dir, "ver")
    os.makedirs(ver_dir, exist_ok=True)
    app_dir = os.path.join(deploy_dir, "app")
    os.makedirs(app_dir, exist_ok=True)
    os.makedirs(os.path.join(deploy_dir, "data"), exist_ok=True)
    os.makedirs(os.path.join(deploy_dir, "temp"), exist_ok=True)

    # 从 _MEIPASS 释放嵌入资源
    meipass = getattr(sys, '_MEIPASS', '')
    if meipass:
        _IGNORE = shutil.ignore_patterns("__pycache__", "*.pyc", "*.pyo")

        # 目录资源
        _DIR_RESOURCES = [
            "desktop/dist",
            "api/qwen2api",
            "api/zhipu2api",
            "services",
            "routes",
            "platformkit",
            "shims",
            "scripts",
        ]
        for rel in _DIR_RESOURCES:
            src = os.path.join(meipass, rel)
            dst = os.path.join(app_dir, rel)
            if os.path.isdir(src):
                try:
                    if os.path.exists(dst):
                        shutil.rmtree(dst, ignore_errors=True)
                    shutil.copytree(src, dst, ignore=_IGNORE)
                except Exception:
                    pass

        # 文件资源
        _FILE_RESOURCES = [
            "backend.py", "package.json",
            "icon.ico", "icon.png",
            "version_info.txt", "version_history.json",
            "versions.json", "gitlog.json", "project.json",
        ]
        for rel in _FILE_RESOURCES:
            src = os.path.join(meipass, rel)
            dst = os.path.join(app_dir, rel)
            if os.path.isfile(src) and not os.path.isfile(dst):
                try:
                    shutil.copy2(src, dst)
                except Exception:
                    pass

    # 写锁文件
    with open(lock_path, "w", encoding="utf-8") as f:
        f.write("yunji")

    # 确定版本化 EXE 文件名
    if not exe_basename.startswith(BRAND_NAME + "-v"):
        m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', exe_basename)
        ver_str = m.group(1) if m else datetime.now().strftime("%Y.%m.%d.%H%M")
        new_name = f"{BRAND_NAME}-v{ver_str}.exe"
    else:
        new_name = exe_basename

    # 复制 EXE 到 ver/
    target_exe = os.path.join(ver_dir, new_name)
    if os.path.normpath(src_exe) != os.path.normpath(target_exe):
        shutil.copy2(src_exe, target_exe)

    # 创建硬链接作为稳定入口
    entry_exe = os.path.join(deploy_dir, f"{BRAND_NAME}.exe")
    if not os.path.isfile(entry_exe):
        _create_hardlink(target_exe, entry_exe)

    # 启动稳定入口，让它清理原始 EXE
    if os.path.normpath(src_exe) != os.path.normpath(entry_exe):
        if os.path.isfile(entry_exe):
            subprocess.Popen(
                f'ping -n 3 127.0.0.1 >nul & start "" "{entry_exe}" --cleanup="{src_exe}"',
                shell=True,
                creationflags=subprocess.CREATE_NO_WINDOW
            )
        # 释放单实例资源
        try:
            _cleanup_single_instance()
        except Exception:
            pass
        os._exit(0)

    return deploy_dir


def _find_dev_dir():
    """
    查找或创建部署目录

    冻结模式 (EXE):
    - 已部署: 返回部署目录
    - 未部署: 触发自部署

    开发模式:
    - 返回 1.PC/ 目录
    """
    if getattr(sys, 'frozen', False):
        exe_dir = os.path.dirname(os.path.abspath(sys.executable))
        # 检查是否已在部署目录中运行
        d = exe_dir
        for _ in range(5):
            if os.path.isfile(os.path.join(d, LOCK_FILE_NAME)):
                if os.path.isdir(os.path.join(d, "ver")) and os.path.isfile(
                    os.path.join(d, f"{BRAND_NAME}.exe")
                ):
                    return d
            parent = os.path.dirname(d)
            if parent == d:
                break
            d = parent

        # 检查当前 EXE 是否就是稳定入口
        exe_basename = os.path.basename(sys.executable)
        if exe_basename == f"{BRAND_NAME}.exe" and os.path.isfile(
            os.path.join(exe_dir, LOCK_FILE_NAME)
        ):
            return exe_dir

        # 触发自部署
        result = _self_deploy(exe_dir)
        return result or exe_dir

    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main():
    # ── 处理 --cleanup 参数 (自部署后清理原始 EXE) ──
    for arg in sys.argv[1:]:
        if arg.startswith("--cleanup="):
            old_exe = arg.split("=", 1)[1].strip('"')
            def _delayed_cleanup():
                try:
                    time.sleep(3)
                    if os.path.isfile(old_exe):
                        os.remove(old_exe)
                except Exception:
                    pass
            threading.Thread(target=_delayed_cleanup, daemon=True).start()
            break

    # ── 自部署 (冻结模式) ──
    if getattr(sys, 'frozen', False):
        deploy_dir = _find_dev_dir()
        if deploy_dir:
            entry_exe = os.path.join(deploy_dir, f"{BRAND_NAME}.exe")
            if os.path.isfile(entry_exe) and os.path.normpath(
                os.path.abspath(sys.executable)
            ) != os.path.normpath(entry_exe):
                # 当前不是稳定入口，启动稳定入口
                subprocess.Popen(
                    [entry_exe] + sys.argv[1:],
                    creationflags=subprocess.CREATE_NO_WINDOW
                )
                os._exit(0)

    _ensure_single_instance()

    try:
        import ctypes
        app_id = "YunJi.SmartIDE.Workstation"
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(app_id)
        print(f"[APP] SetCurrentProcessExplicitAppUserModelID: {app_id}")
    except Exception as _e:
        print(f"[APP] SetCurrentProcessExplicitAppUserModelID failed: {_e}")

    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )

    app = QApplication(sys.argv)
    app.setStyle('Fusion')

    try:
        if hasattr(sys, '_MEIPASS'):
            icon_path = os.path.join(sys._MEIPASS, 'icon.ico')
        elif hasattr(sys, 'frozen'):
            icon_path = os.path.join(os.path.dirname(sys.executable), 'app', 'icon.ico')
        else:
            icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'icon.ico')
        if os.path.exists(icon_path):
            app.setWindowIcon(QIcon(icon_path))
    except Exception:
        pass

    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor("#0d0d0d"))
    palette.setColor(QPalette.ColorRole.WindowText, QColor("#f0f0f0"))
    palette.setColor(QPalette.ColorRole.Base, QColor("#0d0d0d"))
    palette.setColor(QPalette.ColorRole.Text, QColor("#f0f0f0"))
    palette.setColor(QPalette.ColorRole.Button, QColor("#1a1a1a"))
    palette.setColor(QPalette.ColorRole.ButtonText, QColor("#f0f0f0"))
    app.setPalette(palette)

    font = QFont("Microsoft YaHei", 10)
    app.setFont(font)

    splash = SplashScreen()
    screen = app.primaryScreen().geometry()
    x = (screen.width() - splash.width()) // 2
    y = (screen.height() - splash.height()) // 2
    splash.move(x, y)
    splash.show()
    splash.repaint()
    app.processEvents()

    try:
        import pyi_splash
        pyi_splash.close()
    except Exception:
        pass

    splash.set_progress(0.1, "正在创建主窗口...")
    app.processEvents()

    window = MainWindow(splash=splash)
    window.resize(1260, 860)

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
