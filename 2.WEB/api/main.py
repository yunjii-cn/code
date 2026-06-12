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

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
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
    QTabWidget, QScrollArea, QComboBox, QSplashScreen,
)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, pyqtSlot, QTimer, QUrl, QPropertyAnimation, pyqtProperty, QRectF
from PyQt6.QtGui import QFont, QIcon, QColor, QPixmap, QPainter, QLinearGradient, QPalette
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWebEngineCore import QWebEnginePage
from PyQt6.QtWebChannel import QWebChannel

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


class ChineseWebView(QWebEngineView):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.customContextMenuRequested.connect(self._show_translated_menu)

    def _show_translated_menu(self, pos):
        try:
            page = self.page()
            if page is None:
                return
            menu = page.createStandardContextMenu()
            if menu is None:
                return
            for action in menu.actions():
                text = action.text()
                if text in MENU_TRANSLATIONS:
                    action.setText(MENU_TRANSLATIONS[text])
                else:
                    for en, zh in MENU_TRANSLATIONS.items():
                        if en in text:
                            text = text.replace(en, zh)
                    action.setText(text)
            menu.exec(self.mapToGlobal(pos))
        except Exception:
            pass

# 导入后端模块
import backend
from backend import (
    EnvFileManager, ClaudeCliRunner,
    list_openrouter_models, list_anthropic_models, list_ollama_models, list_api_models,
    list_zhipu_models, check_zhipu_api, ZHIPU_DEFAULT_BASE_URL,
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

# ── 单实例控制：命名互斥体 + 命名事件 + 命名共享内存 ──
# 方案：
#   同版本 + 同路径 → 激活已运行实例的窗口
#   同版本 + 不同路径 → 通知旧实例退出
#   不同版本 → 通知旧版本退出
import ctypes
from ctypes import wintypes

_kernel32 = ctypes.windll.kernel32
_user32 = ctypes.windll.user32

ERROR_ALREADY_EXISTS = 183
PAGE_READWRITE = 0x04
FILE_MAP_ALL_ACCESS = 0xF001F
WM_ACTIVATE_INSTANCE = 0x0400 + 0x1001  # 自定义消息：激活窗口

# 全局句柄，MainWindow 关闭时需要释放
_instance_mutex = None
_shutdown_event = None
_path_mapping = None  # 共享内存句柄

MUTEX_NAME = f"YunJiCode_SingleInstance_v{VERSION}"
SHUTDOWN_EVENT_NAME = "YunJiCode_ShutdownEvent"
PATH_MAPPING_NAME = f"YunJiCode_Path_v{VERSION}"  # 存储当前 EXE 路径的共享内存
PATH_MAPPING_SIZE = 1024  # 足够存放路径


def _write_exe_path_to_shared_memory():
    """将当前 EXE 路径写入命名共享内存"""
    global _path_mapping
    _path_mapping = _kernel32.CreateFileMappingW(
        -1, None, PAGE_READWRITE, 0, PATH_MAPPING_SIZE, PATH_MAPPING_NAME
    )
    if not _path_mapping:
        return
    ptr = _kernel32.MapViewOfFile(_path_mapping, FILE_MAP_ALL_ACCESS, 0, 0, PATH_MAPPING_SIZE)
    if ptr:
        try:
            exe_path = sys.executable  # PyInstaller 打包后为 EXE 路径
            path_bytes = exe_path.encode("utf-16-le") + b"\x00\x00"
            ctypes.memmove(ptr, path_bytes, min(len(path_bytes), PATH_MAPPING_SIZE))
        finally:
            _kernel32.UnmapViewOfFile(ptr)


def _read_exe_path_from_shared_memory():
    """从命名共享内存读取已运行实例的 EXE 路径"""
    mapping = _kernel32.OpenFileMappingW(FILE_MAP_ALL_ACCESS, False, PATH_MAPPING_NAME)
    if not mapping:
        return ""
    try:
        ptr = _kernel32.MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, PATH_MAPPING_SIZE)
        if not ptr:
            return ""
        try:
            raw = ctypes.string_at(ptr, PATH_MAPPING_SIZE)
            # utf-16-le 编码，找双零终止
            end = raw.find(b"\x00\x00")
            if end > 0:
                raw = raw[:end + 2]
            return raw.decode("utf-16-le", errors="ignore").rstrip("\x00")
        finally:
            _kernel32.UnmapViewOfFile(ptr)
    finally:
        _kernel32.CloseHandle(mapping)


def _activate_running_instance():
    """激活已运行的实例窗口"""
    # 用窗口类名和标题查找
    hwnd = _user32.FindWindowW(None, f"云集智能编程工作站 v{VERSION}")
    if hwnd:
        # 如果窗口最小化，先恢复
        if _user32.IsIconic(hwnd):
            _user32.ShowWindow(hwnd, 9)  # SW_RESTORE
        _user32.SetForegroundWindow(hwnd)
        return True
    return False


def _ensure_single_instance():
    """单实例控制：
    1. 同版本 + 同路径 → 激活已运行实例窗口
    2. 同版本 + 不同路径 → 通知旧实例退出
    3. 不同版本 → 通知旧版本退出
    """
    global _instance_mutex, _shutdown_event

    # 1. 尝试创建版本互斥体（同版本检测）
    _instance_mutex = _kernel32.CreateMutexW(None, True, MUTEX_NAME)
    if ctypes.GetLastError() == ERROR_ALREADY_EXISTS:
        # 同版本已在运行
        existing_path = _read_exe_path_from_shared_memory()
        my_path = sys.executable
        same_path = existing_path and (
            existing_path.lower() == my_path.lower()
        )

        if same_path:
            # 同一 EXE，激活已运行窗口
            _kernel32.CloseHandle(_instance_mutex)
            _instance_mutex = None
            _activate_running_instance()
            sys.exit(0)
        else:
            # 不同路径的同版本，通知旧实例退出
            _kernel32.CloseHandle(_instance_mutex)
            _instance_mutex = None
            _shutdown_event = _kernel32.CreateEventW(None, True, False, SHUTDOWN_EVENT_NAME)
            _kernel32.SetEvent(_shutdown_event)
            time.sleep(0.5)
            # 旧实例退出后，重新创建互斥体
            _kernel32.CloseHandle(_shutdown_event)
            _shutdown_event = None
            _instance_mutex = _kernel32.CreateMutexW(None, True, MUTEX_NAME)
            if ctypes.GetLastError() == ERROR_ALREADY_EXISTS:
                # 旧实例还没退出，弹出提示
                _kernel32.CloseHandle(_instance_mutex)
                _instance_mutex = None
                ctypes.windll.user32.MessageBoxW(
                    0, "云集智能编程工作站旧实例正在退出中，请稍后重试。", "提示", 0x40
                )
                sys.exit(0)

    # 2. 通知所有旧版本优雅退出（设置全局关闭事件）
    #    无论事件是否已存在，都 SetEvent 一次
    _shutdown_event = _kernel32.CreateEventW(None, True, False, SHUTDOWN_EVENT_NAME)
    _kernel32.SetEvent(_shutdown_event)

    # 3. 短暂等待旧版本退出
    time.sleep(0.3)
    # 重置事件，新实例不会关闭自己
    _kernel32.ResetEvent(_shutdown_event)

    # 4. 将自身路径写入共享内存
    _write_exe_path_to_shared_memory()


def _is_shutdown_signaled() -> bool:
    """检查是否收到关闭信号（由更新的实例发出）"""
    if not _shutdown_event:
        return False
    return _kernel32.WaitForSingleObject(_shutdown_event, 0) == 0  # WAIT_OBJECT_0


def _cleanup_single_instance():
    """清理单实例资源（窗口关闭时调用）"""
    global _instance_mutex, _shutdown_event, _path_mapping
    if _instance_mutex:
        _kernel32.ReleaseMutex(_instance_mutex)
        _kernel32.CloseHandle(_instance_mutex)
        _instance_mutex = None
    if _shutdown_event:
        _kernel32.CloseHandle(_shutdown_event)
        _shutdown_event = None
    if _path_mapping:
        _kernel32.CloseHandle(_path_mapping)
        _path_mapping = None

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

# ── Git 仓库配置（双源：Gitee 公开仓库 + GitHub） ──
GIT_REMOTE = "https://github.com/yunjii-cn/code.git"
GIT_BRANCH = "main"

GITEE_OWNER = "yunjii"
GITEE_REPO = "code"
GITHUB_OWNER = "yunjii-cn"
GITHUB_REPO = "code"

GITEE_API_BASE = f"https://gitee.com/api/v5/repos/{GITEE_OWNER}/{GITEE_REPO}"
GITHUB_API_BASE = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}"

GITEE_RAW_URL = f"https://gitee.com/{GITEE_OWNER}/{GITEE_REPO}/raw/{GIT_BRANCH}/dev/ver/version.json"
GITHUB_RAW_URL = f"https://raw.githubusercontent.com/{GITHUB_OWNER}/{GITHUB_REPO}/{GIT_BRANCH}/dev/ver/version.json"

REMOTE_VERSIONS_URL = GITEE_RAW_URL


# ── 软件更新器 ──
class SoftwareUpdater:
    """软件更新器 - 双源并行：Gitee（国内直连）+ GitHub（需代理），谁先返回用谁"""

    def __init__(self, dev_dir: str, log_func=None, progress_func=None, main_window=None):
        self.dev_dir = dev_dir
        self.app_dir = os.path.join(dev_dir, "app")
        self.ver_dir = os.path.join(dev_dir, "ver")
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func
        self._active_source = None
        self._main_window = main_window

    def _get_main_window(self):
        if self._main_window:
            return self._main_window
        return QApplication.activeWindow()

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
        
        实现方式（硬链接机制，对齐参考项目）：
        1. ver/ 目录存储所有历史版本 EXE
        2. dev/ 目录的 EXE 是 ver/ 中某个版本的硬链接
        3. 切换版本 = 生成批处理脚本 → 等待进程退出 → 删除旧硬链接 → 创建新硬链接 → 启动新EXE
        """
        if not os.path.exists(exe_path):
            self.log(f"[错误] EXE 不存在: {exe_path}", "#F44336")
            return False

        exe_filename = os.path.basename(exe_path)
        dev_exe_path = os.path.join(self.dev_dir, exe_filename)

        dialog = QDialog(self._get_main_window())
        dialog.setWindowTitle("切换版本")
        dialog.setFixedSize(420, 220)
        dialog.setStyleSheet("QDialog { background-color: #1e1e1e; } QLabel { border: none; background: transparent; } QPushButton { border: none; }")
        dl = QVBoxLayout(dialog)
        dl.setSpacing(12)
        dl.setContentsMargins(24, 20, 24, 20)

        title_label = QLabel(f"🔄 切换到 {exe_filename}")
        title_label.setFont(QFont("Microsoft YaHei", 13, QFont.Weight.Bold))
        title_label.setStyleSheet("color: #E0E0E0;")
        dl.addWidget(title_label)

        info_parts = []
        if git_commit:
            info_parts.append(f"Git: {git_commit}")
        info_parts.append("硬链接切换，瞬间完成")
        info_label = QLabel("  |  ".join(info_parts))
        info_label.setFont(QFont("Microsoft YaHei", 10))
        info_label.setStyleSheet("color: #888;")
        info_label.setWordWrap(True)
        dl.addWidget(info_label)

        warning_label = QLabel("⚠️ 切换将关闭当前程序并启动新版本")
        warning_label.setFont(QFont("Microsoft YaHei", 10))
        warning_label.setStyleSheet("color: #FF9800;")
        dl.addWidget(warning_label)

        btn_row = QHBoxLayout()
        btn_row.addStretch()

        cancel_btn = QPushButton("取消")
        cancel_btn.setFixedSize(90, 34)
        cancel_btn.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; font-size: 13px; color: #aaa; }
            QPushButton:hover { background-color: #333; color: #fff; }
        """)
        cancel_btn.clicked.connect(dialog.reject)
        btn_row.addWidget(cancel_btn)

        pause_btn = QPushButton("⏸ 暂停回滚")
        pause_btn.setFixedSize(110, 34)
        pause_btn.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; font-size: 13px; color: #aaa; }
            QPushButton:hover { background-color: #FF9800; color: #fff; }
        """)
        pause_btn.setEnabled(bool(git_commit))
        self._switch_paused = False
        def _toggle_pause():
            self._switch_paused = not self._switch_paused
            if self._switch_paused:
                pause_btn.setText("▶ 继续回滚")
                pause_btn.setStyleSheet("""
                    QPushButton { background-color: #FF9800; border: none; border-radius: 4px; font-size: 13px; font-weight: bold; color: #fff; }
                    QPushButton:hover { background-color: #F57C00; }
                """)
            else:
                pause_btn.setText("⏸ 暂停回滚")
                pause_btn.setStyleSheet("""
                    QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; font-size: 13px; color: #aaa; }
                    QPushButton:hover { background-color: #FF9800; color: #fff; }
                """)
        pause_btn.clicked.connect(_toggle_pause)
        btn_row.addWidget(pause_btn)

        confirm_btn = QPushButton("✅ 确认切换")
        confirm_btn.setFixedSize(110, 34)
        confirm_btn.setStyleSheet("""
            QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 13px; font-weight: bold; color: #fff; }
            QPushButton:hover { background-color: #E00000; }
        """)
        btn_row.addWidget(confirm_btn)

        dl.addLayout(btn_row)

        def _do_switch():
            confirm_btn.setEnabled(False)
            cancel_btn.setEnabled(False)
            pause_btn.setEnabled(False)
            confirm_btn.setText("切换中...")
            confirm_btn.setStyleSheet("""
                QPushButton { background-color: #555; border: none; border-radius: 4px; font-size: 13px; color: #999; }
            """)
            title_label.setText(f"⏳ 正在切换到 {exe_filename}...")
            warning_label.setText("⏳ 程序即将退出，请稍候...")
            warning_label.setStyleSheet("color: #4CAF50;")

            if git_commit and self.is_git_repo():
                while self._switch_paused:
                    QApplication.processEvents()
                    import time
                    time.sleep(0.2)
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

            bat_path = os.path.join(self.dev_dir, "_switch_version.bat")
            current_pid = os.getpid()
            bat_content = f"""@echo off
chcp 65001 >nul 2>&1
title 版本切换
set "TARGET={exe_path}"
set "LINK={dev_exe_path}"
set "PID={current_pid}"
echo 正在等待进程退出...
set /a WAIT=0
:wait_loop
tasklist /fi "pid eq %PID%" 2>nul | find "%PID%" >nul
if not errorlevel 1 (
    set /a WAIT+=1
    if %WAIT% GEQ 30 (
        echo 等待超时，强制切换
        goto do_switch
    )
    timeout /t 1 /nobreak >nul
    goto wait_loop
)
:do_switch
if exist "%LINK%" del /f /q "%LINK%"
mklink /H "%LINK%" "%TARGET%"
if errorlevel 1 (
    echo 硬链接创建失败，尝试复制...
    copy /y "%TARGET%" "%LINK%"
)
if exist "%LINK%" (
    start "" "%LINK%"
)
del /f /q "%~f0"
"""
            try:
                with open(bat_path, "w", encoding="utf-8") as f:
                    f.write(bat_content)
            except Exception as e:
                self.log(f"[警告] 批处理脚本创建失败: {e}，回退到直接启动", "#FF9800")
                cmd = f'ping -n 3 127.0.0.1 >nul & start "" "{exe_path}"'
                subprocess.Popen(cmd, shell=True, creationflags=subprocess.CREATE_NO_WINDOW)
                dialog.accept()
                QApplication.quit()
                return

            self.log(f"正在切换到 {exe_filename}...", "#4CAF50")
            subprocess.Popen(
                f'cmd /c "{bat_path}"',
                shell=True,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            dialog.accept()
            QApplication.quit()

        confirm_btn.clicked.connect(_do_switch)
        dialog.exec()

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
                        for key in ("GITEE_TOKEN", "GITHUB_TOKEN"):
                            if line.startswith(f"{key}="):
                                val = line.split("=", 1)[1].strip()
                                if val:
                                    return val
            except Exception:
                pass
        return token

    def _fetch_json(self, url, headers=None, timeout=15):
        from urllib.request import urlopen, Request
        from urllib.error import URLError, HTTPError
        req = Request(url)
        req.add_header('User-Agent', 'Mozilla/5.0')
        if headers:
            for k, v in headers.items():
                req.add_header(k, v)
        resp = urlopen(req, timeout=timeout)
        return json.loads(resp.read().decode('utf-8'))

    def _parse_version_data(self, version_data):
        if isinstance(version_data, dict) and "versions" in version_data:
            remote_versions = version_data.get("versions", [])
        elif isinstance(version_data, list):
            remote_versions = version_data
        else:
            return None
        result = []
        for v in remote_versions:
            entry = dict(v)
            if "date" in entry and "build_time" not in entry:
                entry["build_time"] = entry["date"]
            result.append(entry)
        result.sort(key=lambda x: x.get("version", ""), reverse=True)
        return result

    def fetch_remote_version_history(self):
        """获取远程版本历史 - 双源并行：Gitee → GitHub → Git命令"""
        result, source = self._fetch_remote_parallel()
        if result is not None:
            self._active_source = source
            return result
        result = self._fetch_remote_via_git()
        if result is not None:
            self._active_source = "git"
            return result
        return []

    def _fetch_remote_parallel(self):
        """双源并行获取远程版本历史，谁先返回用谁"""
        gitee_result = [None]
        github_result = [None]
        gitee_source = [None]
        github_source = [None]

        def _gitee_worker():
            try:
                r = self._fetch_remote_via_gitee()
                gitee_result[0] = r
                gitee_source[0] = "gitee"
            except Exception:
                pass

        def _github_worker():
            try:
                r = self._fetch_remote_via_github()
                github_result[0] = r
                github_source[0] = "github"
            except Exception:
                pass

        t1 = threading.Thread(target=_gitee_worker, daemon=True)
        t2 = threading.Thread(target=_github_worker, daemon=True)
        t1.start()
        t2.start()
        t1.join(timeout=10)
        t2.join(timeout=10)

        if gitee_result[0] is not None:
            return gitee_result[0], "gitee"
        if github_result[0] is not None:
            return github_result[0], "github"
        return None, None

    def _fetch_remote_via_gitee(self):
        """通过 Gitee API 获取远程版本历史（公开仓库无需Token，国内直连）"""
        try:
            url = f"{GITEE_API_BASE}/contents/dev/ver/version.json?ref={GIT_BRANCH}"
            data = self._fetch_json(url, timeout=10)
            if isinstance(data, list):
                return None
            content_b64 = data.get("content", "")
            if not content_b64:
                return None
            import base64
            content = base64.b64decode(content_b64).decode('utf-8')
            version_data = json.loads(content)
            return self._parse_version_data(version_data)
        except Exception:
            try:
                data = self._fetch_json(GITEE_RAW_URL, timeout=10)
                return self._parse_version_data(data)
            except Exception:
                return None

    def _fetch_remote_via_github(self):
        """通过 GitHub API 获取远程版本历史（需代理）"""
        try:
            token = self._get_gitee_token()
            headers = {}
            if token:
                headers['Authorization'] = f'token {token}'
            url = f"{GITHUB_API_BASE}/contents/dev/ver/version.json?ref={GIT_BRANCH}"
            data = self._fetch_json(url, headers=headers, timeout=10)
            if isinstance(data, list):
                pass
            content_b64 = data.get("content", "")
            if not content_b64:
                return None
            import base64
            content = base64.b64decode(content_b64).decode('utf-8')
            version_data = json.loads(content)
            return self._parse_version_data(version_data)
        except Exception:
            try:
                data = self._fetch_json(GITHUB_RAW_URL, timeout=10)
                return self._parse_version_data(data)
            except Exception:
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
            return self._parse_version_data(data)
        except Exception:
            return None

    def fetch_remote_commits(self, limit=30):
        """获取远程Git提交历史 - 双源并行：Gitee API → GitHub API → Git命令"""
        commits, source = self._fetch_commits_parallel(limit)
        if commits is not None:
            return commits
        return self.get_git_history(limit)

    def _fetch_commits_parallel(self, limit=30):
        gitee_result = [None]
        github_result = [None]

        def _gitee_worker():
            try:
                gitee_result[0] = self._fetch_commits_via_gitee(limit)
            except Exception:
                pass

        def _github_worker():
            try:
                github_result[0] = self._fetch_commits_via_github(limit)
            except Exception:
                pass

        t1 = threading.Thread(target=_gitee_worker, daemon=True)
        t2 = threading.Thread(target=_github_worker, daemon=True)
        t1.start()
        t2.start()
        t1.join(timeout=10)
        t2.join(timeout=10)

        if gitee_result[0] is not None:
            return gitee_result[0], "gitee"
        if github_result[0] is not None:
            return github_result[0], "github"
        return None, None

    def _fetch_commits_via_gitee(self, limit=30):
        """通过 Gitee API 获取提交历史（公开仓库无需Token）"""
        try:
            url = f"{GITEE_API_BASE}/commits?sha={GIT_BRANCH}&per_page={limit}"
            data = self._fetch_json(url, timeout=10)
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
        except Exception:
            return None

    def _fetch_commits_via_github(self, limit=30):
        """通过 GitHub API 获取提交历史（需代理）"""
        try:
            token = self._get_gitee_token()
            headers = {}
            if token:
                headers['Authorization'] = f'token {token}'
            url = f"{GITHUB_API_BASE}/commits?sha={GIT_BRANCH}&per_page={limit}"
            data = self._fetch_json(url, headers=headers, timeout=10)
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
        except Exception:
            return None

    def download_update_via_gitee(self, progress_func=None):
        """下载资源包更新 - 双源：根据版本检查来源自动选择 Gitee/GitHub"""
        source = self._active_source or "gitee"
        if source == "gitee":
            url = f"{GITEE_API_BASE}/zipball/{GIT_BRANCH}"
            self.log("正在从 Gitee 下载更新包...", "#FF9800")
        else:
            url = f"{GITHUB_API_BASE}/zipball/{GIT_BRANCH}"
            self.log("正在从 GitHub 下载更新包...", "#FF9800")
        return self._download_and_apply_zip(url, progress_func, source)

    def download_exe_from_release(self, version, filename, progress_func=None):
        """从 Gitee/GitHub Releases 下载 EXE 稳定版到 ver/ 目录（双源回退）"""
        gitee_url = f"https://gitee.com/{GITEE_OWNER}/{GITEE_REPO}/releases/download/v{version}/{filename}"
        github_url = f"https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/v{version}/{filename}"
        source = self._active_source or "gitee"
        urls = [gitee_url, github_url] if source == "gitee" else [github_url, gitee_url]
        for i, url in enumerate(urls):
            source_name = "Gitee" if "gitee.com" in url else "GitHub"
            self.log(f"正在从 {source_name} Releases 下载 {filename}...", "#FF9800")
            if progress_func:
                progress_func(2, f"正在连接 {source_name}...")
            result = self._download_exe(url, filename, progress_func)
            if result:
                self._active_source = source_name.lower()
                return result
            if i < len(urls) - 1:
                self.log(f"{source_name} 下载失败，尝试备用源...", "#FF9800")
        return False

    def _download_exe(self, url, filename, progress_func=None):
        """下载 EXE 文件到 ver/ 目录"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            os.makedirs(self.ver_dir, exist_ok=True)
            dest_path = os.path.join(self.ver_dir, filename)
            if progress_func:
                progress_func(5, "正在连接下载服务器...")
            req = Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            resp = urlopen(req, timeout=120)
            total_size = int(resp.headers.get('Content-Length', 0))
            downloaded = 0
            chunk_size = 65536
            with open(dest_path, "wb") as f:
                while True:
                    chunk = resp.read(chunk_size)
                    if not chunk:
                        break
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total_size > 0 and progress_func:
                        pct = int(5 + 90 * downloaded / total_size)
                        progress_func(pct, f"正在下载... {downloaded // 1024}KB / {total_size // 1024}KB")
            if progress_func:
                progress_func(100, "下载完成")
            self.log(f"✓ 已下载 {filename} 到 ver/ 目录", "#4CAF50")
            return True
        except Exception as e:
            self.log(f"[错误] 下载 EXE 失败: {e}", "#F44336")
            return False

    def _download_and_apply_zip(self, url, progress_func=None, source="gitee"):
        """下载并应用 zip 更新包"""
        try:
            from urllib.request import urlopen, Request
            from urllib.error import URLError, HTTPError
            import tempfile
            import zipfile
            token = self._get_gitee_token()
            if progress_func:
                progress_func(10, "正在下载更新包...")
            req = Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            if token:
                if source == "gitee":
                    pass
                else:
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

    @pyqtSlot()
    def frontendReady(self):
        main = self._get_main()
        if main and hasattr(main, '_finish_splash'):
            QTimer.singleShot(300, main._finish_splash)

    # ── 用户管理 API (多用户登录架构) ──

    @pyqtSlot(result=str)
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

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(result=str)
    def listProjects(self):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_projects())

    @pyqtSlot(result=str)
    def getActiveProject(self):
        main = self._get_main()
        if not main:
            return json.dumps(None)
        proj = main.project_mgr.get_active_project()
        return json.dumps(proj)

    @pyqtSlot(str, str, result=str)
    def createProject(self, name: str, workspace_path: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.create_project(name, workspace_path)
        main.active_project_id = proj["id"]
        main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    @pyqtSlot(str, result=str)
    def switchProject(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.switch_project(project_id)
        if proj:
            main.active_project_id = proj["id"]
            main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    @pyqtSlot(str, str, result=bool)
    def renameProject(self, project_id: str, new_name: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.rename_project(project_id, new_name)

    @pyqtSlot(str, result=bool)
    def deleteProject(self, project_id: str):
        main = self._get_main()
        if not main:
            return False
        result = main.project_mgr.delete_project(project_id)
        if result and main.active_project_id == project_id:
            main.active_project_id = None
            main.current_workspace = main.app_dir
        return result

    @pyqtSlot(str, str, str, result=str)
    def updateProject(self, project_id: str, new_name: str, new_workspace_path: str):
        main = self._get_main()
        if not main:
            return json.dumps({"error": "no main"})
        proj = main.project_mgr.update_project(project_id, new_name or None, new_workspace_path or None)
        if proj and main.active_project_id == project_id:
            main.current_workspace = proj.get("workspace_path") or main.app_dir
        return json.dumps(proj)

    @pyqtSlot(str, result=str)
    def getDefaultProjectPath(self, name: str):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_default_path(name)

    @pyqtSlot(str, result=str)
    def listConversations(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_conversations(project_id))

    @pyqtSlot(str, str, result=str)
    def loadConversation(self, project_id: str, session_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        msgs = main.project_mgr.load_conversation(project_id, session_id)
        return json.dumps(msgs)

    @pyqtSlot(str, str, str, result=bool)
    def copyConversation(self, source_project_id: str, session_id: str, target_project_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.copy_conversation(source_project_id, session_id, target_project_id)

    @pyqtSlot(str, result=str)
    def getProjectContext(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        ctx = main.project_mgr.get_project_context(project_id)
        return json.dumps(ctx)

    @pyqtSlot(str, str, str, result=bool)
    def saveConversation(self, project_id: str, session_id: str, messages_json: str):
        main = self._get_main()
        if not main:
            return False
        try:
            msgs = json.loads(messages_json)
        except Exception:
            msgs = []
        return main.project_mgr.save_conversation(project_id, session_id, msgs)

    @pyqtSlot(str, str, result=str)
    def searchConversations(self, project_id: str, keyword: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.search_conversations(project_id, keyword))

    @pyqtSlot(str, str, result=bool)
    def deleteConversation(self, project_id: str, session_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_conversation(project_id, session_id)

    @pyqtSlot(str, str, str, result=bool)
    def renameConversation(self, project_id: str, session_id: str, new_title: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.rename_conversation(project_id, session_id, new_title)

    @pyqtSlot(str, result=str)
    def getClaudeMd(self, project_id: str):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_claude_md(project_id)

    @pyqtSlot(str, str, result=bool)
    def saveClaudeMd(self, project_id: str, content: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_claude_md(project_id, content)

    @pyqtSlot(result=str)
    def getGlobalClaudeMd(self):
        main = self._get_main()
        if not main:
            return ""
        return main.project_mgr.get_global_claude_md()

    @pyqtSlot(str, result=bool)
    def saveGlobalClaudeMd(self, content: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_global_claude_md(content)

    @pyqtSlot(str, result=str)
    def listMemories(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.list_memories(project_id))

    @pyqtSlot(str, str, str, str, result=bool)
    def saveMemory(self, project_id: str, filename: str, content: str, mem_type: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_memory(project_id, filename, content, mem_type)

    @pyqtSlot(str, str, result=bool)
    def deleteMemory(self, project_id: str, filename: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_memory(project_id, filename)

    @pyqtSlot(str, str, result=str)
    def searchMemories(self, project_id: str, keyword: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.search_memories(project_id, keyword))

    @pyqtSlot(str, str, result=str)
    def autoExtractMemories(self, project_id: str, messages_json: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        try:
            msgs = json.loads(messages_json)
        except Exception:
            msgs = []
        return json.dumps(main.project_mgr.auto_extract_memories(project_id, msgs))

    @pyqtSlot(str, result=str)
    def getMemoryStats(self, project_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"total": 0, "by_type": {}, "total_size": 0})
        return json.dumps(main.project_mgr.get_memory_stats(project_id))

    @pyqtSlot(str, str, result=str)
    def getRelevantMemories(self, project_id: str, query: str):
        main = self._get_main()
        if not main:
            return json.dumps([])
        return json.dumps(main.project_mgr.get_relevant_memories(project_id, query))

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, str, str, str, str, result=bool)
    def saveCustomTemplate(self, name: str, category: str, desc: str, prompt: str, files: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.save_custom_template(name, category, desc, prompt, files)

    @pyqtSlot(str, result=bool)
    def deleteCustomTemplate(self, template_id: str):
        main = self._get_main()
        if not main:
            return False
        return main.project_mgr.delete_custom_template(template_id)

    def _get_app_data_path(self) -> str:
        return os.path.join(self.user_dir, "app_data.json")

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, str, result=str)
    def createProjectFromTemplate(self, project_id: str, template_id: str):
        main = self._get_main()
        if not main:
            return json.dumps({"success": False, "error": "main not available"})
        return json.dumps(main.project_mgr.create_project_from_template(project_id, template_id))

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, str, result=bool)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(str, str, result=str)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(str, result=bool)
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

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, result=bool)
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

    # ── 前端可调用方法 (通过 pyqtSlot 暴露给 QWebChannel) ──

    @pyqtSlot(result=str)
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
        if payload.get("auto_approve"):
            settings["AUTO_APPROVE"] = True
        else:
            settings["AUTO_APPROVE"] = False
        if payload.get("workspace_path"):
            settings["WORKSPACE_PATH"] = payload["workspace_path"]
            main.current_workspace = payload["workspace_path"]

        main.is_busy = True
        self.statusReceived.emit(json.dumps({"busy": True}))

        t = threading.Thread(target=self._run_cli, args=(prompt, model, provider, settings), daemon=True)
        t.start()

        return json.dumps({"ok": True, "sessionId": main.active_session_id})

    @pyqtSlot(result=str)
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

    @pyqtSlot(str, result=bool)
    def openInExplorer(self, path: str):
        try:
            import subprocess
            if os.path.isdir(path):
                subprocess.Popen(f'explorer "{path}"')
                return True
            return False
        except Exception:
            return False

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
                api_base = payload.get("baseUrl", "") or settings.get("API_BASE_URL", "") or "http://127.0.0.1:7777"
                api_key = payload.get("apiKey", "") or settings.get("API_KEY", "")
                result = list_api_models(api_base, api_key, timeout)
            elif source == "zhipu":
                zhipu_key = payload.get("apiKey", "") or settings.get("ZHIPU_API_KEY", "")
                zhipu_base = payload.get("baseUrl", "") or settings.get("ZHIPU_BASE_URL", "") or ZHIPU_DEFAULT_BASE_URL
                result = list_zhipu_models(zhipu_key, zhipu_base, timeout)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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
                self.modelsLoaded.emit(json.dumps({"ok": True, "action": "pull_complete", "model": model_name}))
            except Exception as e:
                self.modelsLoaded.emit(json.dumps({"ok": False, "action": "pull_failed", "model": model_name, "error": str(e)[:200]}))

        t = threading.Thread(target=_do_pull, daemon=True)
        t.start()
        return json.dumps({"ok": True, "loading": True, "action": "pulling", "model": model_name})

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

    @pyqtSlot(result=str)
    def listApiServices(self):
        try:
            return json.dumps({"ok": True, "services": backend.list_api_services()})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    @pyqtSlot(str, result=str)
    def getApiServiceInfo(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            service_name = payload.get("service", "").strip()
            return json.dumps(backend.get_api_service_info(service_name))
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    @pyqtSlot(str, result=str)
    def stopServiceByPort(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            port = int(payload.get("port", 0))
            if port:
                backend._stop_port_service(port)
            return json.dumps({"ok": True})
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    @pyqtSlot(result=str)
    def startAllApiServices(self):
        try:
            result = backend.start_all_api_services()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    @pyqtSlot(result=str)
    def listAllModels(self):
        try:
            result = backend.list_all_models()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": str(e)})

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
    def listZhipuAccounts(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.list_zhipu_accounts(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listZhipuAccounts异常: {e}"})

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
    def fetchZhipuApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.fetch_zhipu_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"fetchZhipuApiKey异常: {e}"})

    @pyqtSlot(str, result=str)
    def createZhipuApiKey(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.create_zhipu_api_key(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"createZhipuApiKey异常: {e}"})

    @pyqtSlot(str, result=str)
    def startZhipuRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.start_zhipu_register(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"startZhipuRegister异常: {e}"})

    @pyqtSlot(str, result=str)
    def pollZhipuRegister(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.poll_zhipu_register(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollZhipuRegister异常: {e}"})

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(str, result=str)
    def clearStickyAccount(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            base_url = payload.get("baseUrl", "").strip()
            admin_key = payload.get("adminKey", "").strip()
            result = backend.clear_sticky_account(base_url, admin_key)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"clearStickyAccount异常: {e}"})

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(result=str)
    def pollQwenLogin(self):
        try:
            result = backend.poll_qwen_login()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollQwenLogin异常: {e}"})

    @pyqtSlot(str, result=str)
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

    @pyqtSlot(result=str)
    def pollQwenRegister(self):
        try:
            result = backend.poll_qwen_register()
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"pollQwenRegister异常: {e}"})

    @pyqtSlot(str, result=str)
    def checkZhipuApi(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            api_key = payload.get("apiKey", "").strip()
            base_url = payload.get("baseUrl", "").strip()
            result = check_zhipu_api(api_key, base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"checkZhipuApi异常: {e}"})

    @pyqtSlot(str, result=str)
    def listZhipuModels(self, payload_json: str = "{}"):
        try:
            payload = json.loads(payload_json) if payload_json else {}
            api_key = payload.get("apiKey", "").strip()
            base_url = payload.get("baseUrl", "").strip()
            result = list_zhipu_models(api_key, base_url)
            return json.dumps(result)
        except Exception as e:
            return json.dumps({"ok": False, "error": f"listZhipuModels异常: {e}"})

    @pyqtSlot(result=str)
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
                    zhipu_model = (settings.get("ZHIPU_MODEL", "") or "glm-4-flash").strip()
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
                    self.deltaReceived.emit(json.dumps({"text": result_text}))
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
                    self.log("  ✓ 虚拟环境迁移完成")
                except Exception as e:
                    self.log(f"  迁移失败: {e}，将重新创建", "#FF9800")

            old_venvs_dir = os.path.join(self.data_dir, "venvs")
            if os.path.isdir(old_venvs_dir):
                try:
                    shutil.rmtree(old_venvs_dir, ignore_errors=True)
                    self.log("  ✓ 清理旧目录: data/venvs/")
                except Exception:
                    pass

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
        self.updater = SoftwareUpdater(self.exe_dir, main_window=self)
        
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

        # 关闭信号检测：每 500ms 检查是否有更新版本要求本实例退出
        self._shutdown_timer = QTimer(self)
        self._shutdown_timer.timeout.connect(self._check_shutdown_signal)
        self._shutdown_timer.start(500)

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
        self.btn_update_nav = QPushButton("📋 软件更新")
        self.btn_update_nav.setCheckable(True)
        self.btn_update_nav.setStyleSheet(menu_button_style)
        self.btn_update_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_update_nav.clicked.connect(lambda: self._switch_page(2))
        nav_layout.addWidget(self.btn_update_nav)

        # 项目管理按钮
        self.btn_project_nav = QPushButton("📁 项目管理")
        self.btn_project_nav.setCheckable(True)
        self.btn_project_nav.setStyleSheet(menu_button_style)
        self.btn_project_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_project_nav.clicked.connect(lambda: self._switch_page(3))
        nav_layout.addWidget(self.btn_project_nav)

        # 系统设置按钮
        self.btn_settings_nav = QPushButton("⚙️ 系统设置")
        self.btn_settings_nav.setCheckable(True)
        self.btn_settings_nav.setStyleSheet(menu_button_style)
        self.btn_settings_nav.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.btn_settings_nav.clicked.connect(lambda: self._switch_page(4))
        nav_layout.addWidget(self.btn_settings_nav)

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

        # 页面3：项目管理（复用首页 WebEngineView）
        # 页面4：系统设置（复用首页 WebEngineView）
        # 这两个页面不创建独立页面，而是切换到首页并通过JS切换前端视图

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
        self.web_view = ChineseWebView()
        profile = self.web_view.page().profile()
        # Web存储路径: 用户级数据 (跟随用户隔离)
        storage_path = os.path.join(self.user_dir, "webdata")
        os.makedirs(storage_path, exist_ok=True)
        profile.setPersistentStoragePath(storage_path)
        profile.setHttpCacheMaximumSize(50 * 1024 * 1024)
        self.web_view.setStyleSheet("background-color: #0d0d0d;")

        self.web_view.page().setBackgroundColor(QColor("#0d0d0d"))

        self.web_view.loadFinished.connect(self._on_web_load_finished)
        self.web_view.page().javaScriptConsoleMessage = self._on_js_console

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
        """创建软件更新页面 - 单排工具栏架构"""
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setSpacing(0)
        layout.setContentsMargins(0, 0, 0, 0)

        toolbar_widget = QWidget()
        toolbar_widget.setFixedHeight(44)
        toolbar_widget.setStyleSheet("background-color: #1e1e1e;")
        toolbar = QHBoxLayout(toolbar_widget)
        toolbar.setContentsMargins(10, 5, 10, 5)
        toolbar.setSpacing(6)

        ver_label = QLabel(f"v{VERSION}")
        ver_label.setFont(QFont("Consolas", 11, QFont.Weight.Bold))
        ver_label.setStyleSheet("color: #888; border: none; background: transparent;")
        toolbar.addWidget(ver_label)

        self._ver_tab_stable_btn = QPushButton("📦 EXE稳定版")
        self._ver_tab_stable_btn.setFixedHeight(30)
        self._ver_tab_stable_btn.setStyleSheet("""
            QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 14px; }
            QPushButton:hover { background-color: #E00000; }
        """)
        self._ver_tab_stable_btn.clicked.connect(lambda: self._switch_ver_tab("stable"))
        toolbar.addWidget(self._ver_tab_stable_btn)

        self._ver_tab_git_btn = QPushButton("🔀 Git开发版")
        self._ver_tab_git_btn.setFixedHeight(30)
        self._ver_tab_git_btn.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #666; padding: 0 14px; }
            QPushButton:hover { background-color: #333; color: #999; }
        """)
        self._ver_tab_git_btn.clicked.connect(lambda: self._switch_ver_tab("git"))
        toolbar.addWidget(self._ver_tab_git_btn)

        toolbar.addStretch()

        self._ver_status_label = QLabel("")
        self._ver_status_label.setFont(QFont("Microsoft YaHei", 10))
        self._ver_status_label.setStyleSheet("color: #666; border: none; background: transparent;")
        toolbar.addWidget(self._ver_status_label)

        btn_check_remote = QPushButton("🔄 检查更新")
        btn_check_remote.setFixedHeight(28)
        btn_check_remote.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 0 10px; font-size: 11px; color: #aaa; }
            QPushButton:hover { background-color: #CC0000; border-color: #E00000; color: #fff; }
        """)
        btn_check_remote.clicked.connect(self._check_remote_versions)
        toolbar.addWidget(btn_check_remote)

        self._ver_expand_btn = QPushButton("📋 全部折叠")
        self._ver_expand_btn.setFixedHeight(28)
        self._ver_expand_btn.setStyleSheet("""
            QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 0 10px; font-size: 11px; color: #aaa; }
            QPushButton:hover { background-color: #333; color: #fff; }
        """)
        self._ver_expand_btn.clicked.connect(self._toggle_expand_all)
        toolbar.addWidget(self._ver_expand_btn)

        layout.addWidget(toolbar_widget)

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
        self._ver_info_text = "正在加载版本信息..."
        self._ver_expanded = True
        self._ver_info_label = None
        self._ver_stable_container = None
        self._ver_git_container = None
        self._ver_lazy_count = 10
        self._ver_lazy_batch_expanded = 10
        self._ver_lazy_batch_collapsed = 20

        self.update_log_text = QTextEdit()
        self.update_log_text.setReadOnly(True)
        self.update_log_text.setStyleSheet("QTextEdit { background-color: #0a0a0a; color: #aaa; border: none; font-family: Consolas, monospace; font-size: 11px; }")
        self.update_log_text.hide()

        return page

    # ── 页面切换 ──

    def _switch_page(self, index):
        """切换页面"""
        actual_page = 0 if index in (3, 4) else index
        self.btn_home.setChecked(index == 0)
        self.btn_deploy_nav.setChecked(index == 1)
        self.btn_update_nav.setChecked(index == 2)
        self.btn_project_nav.setChecked(index == 3)
        self.btn_settings_nav.setChecked(index == 4)
        self.page_stack.setCurrentIndex(actual_page)

        if index == 1:
            self._refresh_deploy_env_status()
        if index == 2:
            if not self._ver_stable_data:
                self._load_versions_from_cache()
            else:
                self._render_active_tab()
        if index in (0, 3, 4):
            nav_name = {0: "chat", 3: "project", 4: "settings"}.get(index, "chat")
            try:
                self.web_view.page().runJavaScript(f"if(window.switchNav) window.switchNav('{nav_name}');")
            except Exception:
                pass

    def _switch_ver_tab(self, tab):
        if tab == self._ver_active_tab:
            return
        self._ver_active_tab = tab
        self._ver_lazy_count = self._ver_lazy_batch_expanded if self._ver_expanded else self._ver_lazy_batch_collapsed
        if tab == "stable":
            self._ver_tab_stable_btn.setStyleSheet("""
                QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 14px; }
                QPushButton:hover { background-color: #E00000; }
            """)
            self._ver_tab_git_btn.setStyleSheet("""
                QPushButton { background-color: #2a2a2a; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #666; padding: 0 14px; }
                QPushButton:hover { background-color: #333; color: #999; }
            """)
        else:
            self._ver_tab_git_btn.setStyleSheet("""
                QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 14px; }
                QPushButton:hover { background-color: #E00000; }
            """)
            self._ver_tab_stable_btn.setStyleSheet("""
                QPushButton { background-color: #2a2a2a; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #666; padding: 0 14px; }
                QPushButton:hover { background-color: #333; color: #999; }
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
        self._ver_lazy_count = self._ver_lazy_batch_expanded if self._ver_expanded else self._ver_lazy_batch_collapsed
        if self._ver_expanded:
            self._ver_expand_btn.setText("📋 全部折叠")
            self._ver_expand_btn.setStyleSheet("""
                QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 0 10px; font-size: 11px; color: #aaa; }
                QPushButton:hover { background-color: #333; color: #fff; }
            """)
        else:
            self._ver_expand_btn.setText("📋 全部展开")
            self._ver_expand_btn.setStyleSheet("""
                QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 0 10px; font-size: 11px; color: #aaa; }
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
        info_layout.addLayout(info_header)
        self._ver_scroll_layout.addWidget(info_frame)

        current_card = self._build_current_version_card()
        if current_card:
            self._ver_scroll_layout.addWidget(current_card)

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

    def _load_versions_from_cache(self):
        """从本地缓存加载版本数据（秒显示），缓存过期则后台刷新"""
        cache_path = os.path.join(self.updater.dev_dir, "ver", "version_cache.json")
        loaded = False
        if os.path.exists(cache_path):
            try:
                with open(cache_path, "r", encoding="utf-8") as f:
                    cache = json.load(f)
                self._ver_stable_data = cache.get("stable_data", [])
                self._ver_git_data = cache.get("git_data", [])
                self._ver_current_version = cache.get("current_version", VERSION)
                self._ver_info_text = cache.get("info_text", "")
                self._has_remote_update = cache.get("has_remote_update", False)
                cache_time = cache.get("cache_time", 0)
                import time
                cache_age = time.time() - cache_time
                if cache_age < 300:
                    self._update_ver_info_text(self._ver_info_text or "✅ 版本数据已加载")
                    self._render_active_tab()
                    if hasattr(self, '_ver_status_label') and self._ver_status_label:
                        source_tag = cache.get("source_tag", "")
                        self._ver_status_label.setText(f"稳定版 {len(self._ver_stable_data)} 个 | Git提交 {len(self._ver_git_data)} 条{source_tag}")
                    loaded = True
                else:
                    self._update_ver_info_text("⏳ 缓存已过期，正在刷新...")
                    self._render_active_tab()
                    threading.Thread(target=self._do_check_remote, daemon=True).start()
                    loaded = True
            except Exception:
                pass
        if not loaded:
            self._update_ver_info_text("⏳ 正在加载版本数据...")
            self._render_active_tab()
            threading.Thread(target=self._do_check_remote, daemon=True).start()

    def _save_versions_to_cache(self, source_tag=""):
        """持久化版本数据到本地缓存文件"""
        cache_path = os.path.join(self.updater.dev_dir, "ver", "version_cache.json")
        try:
            os.makedirs(os.path.dirname(cache_path), exist_ok=True)
            import time
            cache = {
                "stable_data": self._ver_stable_data,
                "git_data": self._ver_git_data,
                "current_version": self._ver_current_version,
                "info_text": self._ver_info_text,
                "has_remote_update": getattr(self, '_has_remote_update', False),
                "cache_time": time.time(),
                "source_tag": source_tag,
            }
            with open(cache_path, "w", encoding="utf-8") as f:
                json.dump(cache, f, ensure_ascii=False, indent=2)
        except Exception:
            pass

    def _update_ver_info_text(self, text):
        self._ver_info_text = text
        if hasattr(self, '_ver_info_label') and self._ver_info_label:
            self._ver_info_label.setText(text)

    def _check_remote_versions(self):
        if hasattr(self, '_ver_status_label') and self._ver_status_label:
            self._ver_status_label.setText("正在检查远程更新...")
        self._update_ver_info_text("⏳ 正在检查远程更新...")
        threading.Thread(target=self._do_check_remote, daemon=True).start()

    def _do_check_remote(self):
        try:
            remote_versions = self.updater.fetch_remote_version_history()
            if remote_versions is None:
                raise Exception("无法获取远程版本信息（Gitee/GitHub/Git均不可用）")
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
                has_local = ver_num in exe_versions
                remote_filename = v.get("filename") or v.get("exe") or ""
                all_versions.append({
                    "version": ver_num,
                    "name": v.get("name", f"v{ver_num}"),
                    "changes": v.get("changes", []),
                    "build_time": v.get("build_time", v.get("date", "")),
                    "git_commit": v.get("git_commit", ""),
                    "available": has_local,
                    "exe_info": exe_versions.get(ver_num),
                    "is_remote_new": not is_current,
                    "remote_filename": remote_filename,
                    "remote_size_mb": v.get("size_mb", 0),
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
                self._ver_info_text = f"🆕 发现新版本 v{remote_latest}"
                self._has_remote_update = True
            else:
                self._ver_info_text = "✅ 已是最新版本"
                self._has_remote_update = False
            self._ver_stable_data = all_versions
            self._ver_git_data = git_commits
            self._ver_current_version = current_version
            source_tag = f"（via {self.updater._active_source}）" if self.updater._active_source else ""
            self._save_versions_to_cache(source_tag)
            QTimer.singleShot(0, self._render_active_tab)
            if hasattr(self, '_ver_status_label') and self._ver_status_label:
                QTimer.singleShot(0, lambda: self._ver_status_label.setText(f"稳定版 {len(all_versions)} 个 | Git提交 {len(git_commits)} 条{source_tag}"))
        except Exception as e:
            if not self._ver_stable_data:
                QTimer.singleShot(0, lambda: self._update_ver_info_text(f"❌ 远程检查失败: {e}"))
            else:
                QTimer.singleShot(0, lambda: self._update_ver_info_text("✅ 版本数据已加载（远程刷新失败）"))
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
        visible_commits = git_commits[:self._ver_lazy_count]
        for commit in visible_commits:
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
                switch_btn = QPushButton("🔄 切换")
                switch_btn.setFixedHeight(26)
                switch_btn.setStyleSheet("""
                    QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 12px; }
                    QPushButton:hover { background-color: #E00000; }
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

        if len(git_commits) > self._ver_lazy_count:
            load_more_btn = QPushButton(f"📋 加载更多（还有 {len(git_commits) - self._ver_lazy_count} 条提交）")
            load_more_btn.setFixedHeight(36)
            load_more_btn.setStyleSheet("""
                QPushButton { background-color: #222; border: 1px solid #333; border-radius: 6px; font-size: 12px; color: #888; }
                QPushButton:hover { background-color: #2a2a2a; border-color: #CC0000; color: #fff; }
            """)

            def _load_more():
                self._ver_lazy_count += self._ver_lazy_batch_expanded if self._ver_expanded else self._ver_lazy_batch_collapsed
                self._render_active_tab()

            load_more_btn.clicked.connect(_load_more)
            self.git_history_layout.addWidget(load_more_btn)

    def _build_current_version_card(self):
        """构建当前版本详情卡片（置顶显示）"""
        current_v = None
        for v in self._ver_stable_data:
            if v.get("version") == self._ver_current_version:
                current_v = v
                break
        if not current_v:
            return None
        changes = current_v.get("changes", [])
        build_time = current_v.get("build_time", "")
        git_commit = current_v.get("git_commit", "")
        exe_info = current_v.get("exe_info")
        card = QFrame()
        card.setObjectName("currentVersionCard")
        card.setStyleSheet("""
            #currentVersionCard { background-color: #0d1f0d; border: 2px solid #1f5a1f; border-radius: 8px; }
            #currentVersionCard:hover { border-color: #2a7a2a; }
            QLabel { border: none; background: transparent; }
        """)
        cl = QVBoxLayout(card)
        cl.setSpacing(4)
        cl.setContentsMargins(14, 10, 14, 10)
        header = QHBoxLayout()
        header.setSpacing(10)
        ver_label = QLabel(f"v{self._ver_current_version}")
        ver_label.setFont(QFont("Consolas", 14, QFont.Weight.Bold))
        ver_label.setStyleSheet("color: #4CAF50;")
        header.addWidget(ver_label)
        current_tag = QLabel("● 当前运行版本")
        current_tag.setFont(QFont("Microsoft YaHei", 11, QFont.Weight.Bold))
        current_tag.setStyleSheet("color: #4CAF50;")
        header.addWidget(current_tag)
        header.addStretch()
        if exe_info and exe_info.get("size_mb"):
            size_label = QLabel(f"{exe_info['size_mb']}MB")
            size_label.setFont(QFont("Consolas", 10))
            size_label.setStyleSheet("color: #555;")
            header.addWidget(size_label)
        cl.addLayout(header)
        detail_line = QHBoxLayout()
        detail_line.setSpacing(16)
        if build_time:
            try:
                from datetime import datetime as _dt
                dt = _dt.fromisoformat(build_time)
                date_str = dt.strftime("%Y-%m-%d %H:%M")
            except Exception:
                date_str = build_time[:16]
            time_label = QLabel(f"📅 {date_str}")
            time_label.setFont(QFont("Microsoft YaHei", 10))
            time_label.setStyleSheet("color: #666;")
            detail_line.addWidget(time_label)
        if git_commit:
            commit_label = QLabel(f"🔗 {git_commit}")
            commit_label.setFont(QFont("Consolas", 9))
            commit_label.setStyleSheet("color: #555;")
            detail_line.addWidget(commit_label)
        detail_line.addStretch()
        cl.addLayout(detail_line)
        if changes:
            sep = QFrame()
            sep.setFrameShape(QFrame.Shape.HLine)
            sep.setFixedHeight(1)
            sep.setStyleSheet("background-color: #1f3a1f; border: none;")
            cl.addWidget(sep)
            for ch in changes:
                ch_label = QLabel(f"· {ch}")
                ch_label.setFont(QFont("Microsoft YaHei", 10))
                ch_label.setStyleSheet("color: #8a8;")
                ch_label.setWordWrap(True)
                cl.addWidget(ch_label)
        return card

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
        visible = all_versions[:self._ver_lazy_count]
        expanded = self._ver_expanded
        for v in visible:
            ver = v["version"]
            is_current = (ver == current_version)
            is_available = v.get("available", False)
            is_remote_new = v.get("is_remote_new", False)
            changes = v.get("changes", [])
            exe_info = v.get("exe_info")
            if is_current:
                card_bg = "#0d1f0d"
                border_color = "#1f5a1f"
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
                remote_filename = v.get("remote_filename", "")
                if remote_filename:
                    cloud_tag = QLabel("☁ 远程可下载")
                    cloud_tag.setFont(QFont("Microsoft YaHei", 10))
                    cloud_tag.setStyleSheet("color: #42A5F5;")
                    header.addWidget(cloud_tag)
                    remote_size = v.get("remote_size_mb", 0)
                    if remote_size:
                        size_label = QLabel(f"{remote_size}MB")
                        size_label.setFont(QFont("Consolas", 10))
                        size_label.setStyleSheet("color: #555;")
                        header.addWidget(size_label)
                else:
                    status_label = QLabel("未提供")
                    status_label.setFont(QFont("Microsoft YaHei", 10))
                    status_label.setStyleSheet("color: #444;")
                    header.addWidget(status_label)
            header.addStretch()
            if is_current:
                current_tag = QLabel("● 运行中")
                current_tag.setFont(QFont("Microsoft YaHei", 10))
                current_tag.setStyleSheet("color: #4CAF50;")
                header.addWidget(current_tag)
            elif is_available and exe_info:
                switch_btn = QPushButton("🔄 切换")
                switch_btn.setFixedHeight(26)
                switch_btn.setStyleSheet("""
                    QPushButton { background-color: #CC0000; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 12px; }
                    QPushButton:hover { background-color: #E00000; }
                """)
                gc = v.get("git_commit", "")
                switch_btn.clicked.connect(lambda checked, p=exe_info["path"], c=gc: self.updater.switch_to_exe(p, c))
                header.addWidget(switch_btn)
            elif not is_available and v.get("remote_filename"):
                dl_btn = QPushButton("⬇ 下载")
                dl_btn.setFixedHeight(26)
                dl_btn.setStyleSheet("""
                    QPushButton { background-color: #1565C0; border: none; border-radius: 4px; font-size: 12px; font-weight: bold; color: #fff; padding: 0 12px; }
                    QPushButton:hover { background-color: #1976D2; }
                """)
                dl_ver = v["version"]
                dl_filename = v["remote_filename"]
                dl_git_commit = v.get("git_commit", "")
                dl_btn.clicked.connect(lambda checked, ver=dl_ver, fn=dl_filename, gc=dl_git_commit: self._download_and_switch(ver, fn, gc))
                header.addWidget(dl_btn)
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
                    for ch in changes:
                        ch_label = QLabel(f"· {ch}")
                        ch_label.setFont(QFont("Microsoft YaHei", 10))
                        ch_label.setStyleSheet("color: #777;")
                        ch_label.setWordWrap(True)
                        dl.addWidget(ch_label)
                else:
                    no_ch = QLabel("暂无修改记录")
                    no_ch.setFont(QFont("Microsoft YaHei", 10))
                    no_ch.setStyleSheet("color: #3a3a3a;")
                    dl.addWidget(no_ch)
                cl.addWidget(detail)
            layout.addWidget(card)

        if len(all_versions) > self._ver_lazy_count:
            load_more_btn = QPushButton(f"📋 加载更多（还有 {len(all_versions) - self._ver_lazy_count} 个版本）")
            load_more_btn.setFixedHeight(36)
            load_more_btn.setStyleSheet("""
                QPushButton { background-color: #222; border: 1px solid #333; border-radius: 6px; font-size: 12px; color: #888; }
                QPushButton:hover { background-color: #2a2a2a; border-color: #CC0000; color: #fff; }
            """)

            def _load_more():
                self._ver_lazy_count += self._ver_lazy_batch_expanded if self._ver_expanded else self._ver_lazy_batch_collapsed
                self._render_active_tab()

            load_more_btn.clicked.connect(_load_more)
            layout.addWidget(load_more_btn)

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
                for ch in changes:
                    ch_label = QLabel(f"· {ch}")
                    ch_label.setFont(QFont("Microsoft YaHei", 10))
                    ch_label.setStyleSheet("color: #777;")
                    ch_label.setWordWrap(True)
                    dl.addWidget(ch_label)
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

    def _download_and_switch(self, version, filename, git_commit=""):
        """从远程 Releases 下载 EXE 并切换版本"""
        from PyQt6.QtWidgets import QProgressDialog
        progress = QProgressDialog(f"正在下载 v{version}...", "取消", 0, 100, self._get_main_window())
        progress.setWindowTitle("下载稳定版")
        progress.setFixedSize(400, 120)
        progress.setStyleSheet("QProgressDialog { background-color: #1e1e1e; color: #E0E0E0; } QLabel { color: #E0E0E0; border: none; background: transparent; } QPushButton { background-color: #2a2a2a; border: 1px solid #3a3a3a; border-radius: 4px; padding: 4px 16px; color: #aaa; }")
        progress.setWindowModality(Qt.WindowModality.WindowModal)
        progress.setMinimumDuration(0)
        progress.setValue(0)

        self._download_cancelled = False

        def _on_cancel():
            self._download_cancelled = True

        progress.canceled.connect(_on_cancel)

        def _progress_func(pct, label):
            if self._download_cancelled:
                return
            QTimer.singleShot(0, lambda: progress.setValue(pct))
            QTimer.singleShot(0, lambda: progress.setLabelText(label))

        def _do_download():
            self.updater.log = lambda msg, color="#ccc": self.log_signal.emit(msg, color)
            self.updater.progress = _progress_func
            success = self.updater.download_exe_from_release(version, filename, _progress_func)
            if self._download_cancelled:
                QTimer.singleShot(0, lambda: progress.close())
                return
            if success:
                QTimer.singleShot(0, lambda: progress.setValue(100))
                QTimer.singleShot(0, lambda: progress.setLabelText("下载完成，准备切换..."))
                exe_path = os.path.join(self.updater.ver_dir, filename)
                if os.path.exists(exe_path):
                    QTimer.singleShot(500, lambda: self._after_download_switch(exe_path, git_commit))
                else:
                    self.log_signal.emit("✗ 下载完成但文件未找到", "#F44336")
                    QTimer.singleShot(0, lambda: progress.close())
            else:
                self.log_signal.emit("✗ 下载失败", "#F44336")
                QTimer.singleShot(0, lambda: progress.close())

        threading.Thread(target=_do_download, daemon=True).start()

    def _after_download_switch(self, exe_path, git_commit=""):
        """下载完成后的切换逻辑"""
        self._check_remote_versions()
        self.updater.switch_to_exe(exe_path, git_commit)

    def _switch_git_commit(self, commit_hash):
        """切换到指定Git提交"""
        self._append_log(f"正在切换到 Git commit {commit_hash}...", "#FF9800")
        success = self.updater.switch_git_commit(commit_hash)
        if success:
            self._append_log("✓ Git 版本切换成功，请重启应用以加载新资源包", "#4CAF50")
            self._refresh_git_history()

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
        if ok:
            self.web_view.setVisible(True)
            if self._splash and self._splash.isVisible():
                self._splash.set_progress(0.95, "正在渲染界面...")

    def _finish_splash(self):
        if self._splash and self._splash.isVisible():
            self._splash.set_progress(1.0, "加载完成！")
            self.show()
            self._splash.finish(self)
            self._splash = None

    def _splash_fallback(self):
        if self._splash and self._splash.isVisible():
            self.show()
            self._splash.finish(self)
            self._splash = None

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
            self.web_view.page().runJavaScript(f"if(window.setVoiceResult) window.setVoiceResult({escaped});")
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
    # ── 关闭 ──
    def _check_shutdown_signal(self):
        """检测是否有更新版本的实例要求本实例退出"""
        if _is_shutdown_signaled():
            print("[APP] 收到关闭信号，正在优雅退出...")
            self.close()

    def closeEvent(self, event):
        _cleanup_single_instance()
        event.accept()


BRAND_NAME = "云集智能编程工作站"


def main():
    if hasattr(sys, '_MEIPASS'):
        if 'QTWEBENGINEPROCESS_PATH' not in os.environ:
            we_process = os.path.join(sys._MEIPASS, 'PyQt6', 'Qt6', 'bin', 'QtWebEngineProcess.exe')
            if os.path.isfile(we_process):
                os.environ['QTWEBENGINEPROCESS_PATH'] = we_process
        os.environ.setdefault('QTWEBENGINE_CHROMIUM_FLAGS', '--disable-gpu')
        if not os.environ.get('QTWEBENGINE_RESOURCES_PATH'):
            qt6_base = os.path.join(sys._MEIPASS, 'PyQt6', 'Qt6')
            res_path = os.path.join(qt6_base, 'resources')
            if os.path.isdir(res_path):
                os.environ['QTWEBENGINE_RESOURCES_PATH'] = res_path
        if not os.environ.get('QTWEBENGINE_LOCALES_PATH'):
            qt6_base = os.path.join(sys._MEIPASS, 'PyQt6', 'Qt6')
            loc_path = os.path.join(qt6_base, 'translations', 'qtwebengine_locales')
            if os.path.isdir(loc_path):
                os.environ['QTWEBENGINE_LOCALES_PATH'] = loc_path

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
