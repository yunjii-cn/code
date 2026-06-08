#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
云集智能编程工作站 - 版本化构建工具

使用方法：
  python build-version.py                    # 从 dev/app/ 源码直接构建新版本
  python build-version.py --desc "描述内容"  # 指定版本描述
  python build-version.py --push             # 构建完成后自动推送到远程仓库

架构说明 (v3.0+)：
  - PyQt6 + QWebEngineView 替代 Electron
  - QWebChannel 替代 Electron IPC
  - backend.py 提供 Ollama 代理 / CLI 管理 / 配置管理
  - --onedir 模式打包（QtWebEngine 不支持 --onefile）
  - PyInstaller 工作目录: build/ (仅构建用，不推送)
  - 目录架构（对齐参考项目）:
    dev/*.exe         = 开发测试 EXE（gitignore）
    dev/_internal/    = PyInstaller 运行时（gitignore）
    dev/app/          = 资源目录（main.py, desktop/, nodejs/ 等，git 管理）
    dev/ver/*.exe     = 稳定版 EXE（git 跟踪）
    dev/              = Git 仓库根目录
  - 部署维护功能自动安装 nodejs/bun/node_modules 等运行时
"""
import os
import sys
import subprocess
import shutil
import re
import json
from pathlib import Path
from datetime import datetime
import time

if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

VERSION = datetime.now().strftime("%Y.%m.%d.%H%M")
ROOT_DIR = Path(__file__).resolve().parent  # build-version.py 在 dev/app/ 下
DEV_APP_DIR = ROOT_DIR
DEV_DIR = ROOT_DIR.parent                   # dev/ 根目录
BUILD_DIR = ROOT_DIR.parent.parent / "build"  # 项目根/build/ (PyInstaller 工作目录)
VERSION_HISTORY_FILE = ROOT_DIR / "version_history.json"
VERSION_JSON_FILE = DEV_DIR / "ver" / "version.json"


def load_version_history():
    if VERSION_HISTORY_FILE.exists():
        try:
            with open(VERSION_HISTORY_FILE, 'r', encoding='utf-8') as f:
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
        except Exception:
            pass
    return []


def save_version_history(history):
    try:
        with open(VERSION_HISTORY_FILE, 'w', encoding='utf-8') as f:
            json.dump(history, f, ensure_ascii=False, indent=2)
    except Exception as e:
        print(f"警告：保存版本历史失败：{e}")


def get_version_description():
    print()
    print("=" * 60)
    print("  自动生成版本描述")
    print("=" * 60)
    print()

    default_changes = [
        "前端集成到EXE (QWebEngineView替代Electron)",
        "统一终端架构，部署维护成为EXE内置功能",
        "QWebChannel替代Electron IPC",
        "backend.py替代main.cjs后端逻辑",
    ]

    print("使用自动生成的修改内容：")
    for i, change in enumerate(default_changes, 1):
        print(f"  {i}. {change}")
    print()

    return default_changes


# ── 构建前端 ──
def build_frontend():
    """在 dev/app/ 中构建 Vue 前端到 desktop/dist/"""
    dist_path = DEV_APP_DIR / "desktop" / "dist" / "index.html"
    src_dir = DEV_APP_DIR / "desktop" / "renderer" / "src"

    need_rebuild = not dist_path.exists()
    if not need_rebuild and src_dir.exists():
        dist_mtime = dist_path.stat().st_mtime
        for src_file in src_dir.rglob("*"):
            if src_file.is_file() and src_file.stat().st_mtime > dist_mtime:
                need_rebuild = True
                break

    if not need_rebuild:
        print("  ✓ 前端已构建且为最新 (desktop/dist/)")
        return True

    if dist_path.exists():
        print("  检测到前端源码变更，重新构建...")
    else:
        print("  构建前端 (vite build)...")

    bun_dir = DEV_APP_DIR / "bun" / "bun-windows-x64"
    node_dir = DEV_APP_DIR / "nodejs"
    # 查找 node 目录
    node_subdirs = list(node_dir.glob("node-v*-win-x64")) if node_dir.exists() else []
    node_exe_dir = node_subdirs[0] if node_subdirs else None

    bun_exe = bun_dir / "bun.exe" if bun_dir.exists() else None

    env = dict(os.environ)
    if node_exe_dir and node_exe_dir.exists():
        env["PATH"] = str(node_exe_dir) + ";" + env.get("PATH", "")
    if bun_exe and bun_exe.parent.exists():
        env["PATH"] = str(bun_exe.parent) + ";" + env.get("PATH", "")

    si = subprocess.STARTUPINFO()
    si.dwFlags |= subprocess.STARTF_USESHOWWINDOW

    try:
        vite_js = str(DEV_APP_DIR / "node_modules" / "vite" / "bin" / "vite.js")
        node_exe = str(node_exe_dir / "node.exe") if node_exe_dir and node_exe_dir.exists() else "node"

        if node_exe_dir and node_exe_dir.exists() and os.path.isfile(vite_js):
            r = subprocess.run(
                [node_exe, vite_js, "build", "--config", "desktop/vite.config.ts"],
                cwd=str(DEV_APP_DIR), env=env,
                startupinfo=si, capture_output=True, text=True, timeout=120,
            )
        elif bun_exe and bun_exe.exists():
            r = subprocess.run(
                [str(bun_exe), "run", "desktop:build"],
                cwd=str(DEV_APP_DIR), env=env,
                startupinfo=si, capture_output=True, text=True, timeout=120,
            )
        else:
            r = subprocess.run(
                ["npx", "vite", "build", "--config", "desktop/vite.config.ts"],
                cwd=str(DEV_APP_DIR), env=env,
                startupinfo=si, capture_output=True, text=True, timeout=120,
            )

        if r.returncode == 0 and dist_path.exists():
            print("  ✓ 前端构建完成")
            return True
        else:
            print(f"  ✗ 前端构建失败 (rc={r.returncode})")
            if r.stderr:
                print(f"    stderr: {r.stderr[:300]}")
            return False
    except Exception as e:
        print(f"  ✗ 前端构建异常: {e}")
        return False


# ── PyInstaller 打包 ──
def build_exe():
    """用 PyInstaller --onefile 模式打包，输出单个 EXE"""
    print(f"  PyInstaller 打包 (v{VERSION})...")

    release_name = f"云集智能编程工作站v{VERSION}"
    release_dir = BUILD_DIR / release_name

    if release_dir.exists():
        print(f"  清理旧发布目录: {release_name}")
        shutil.rmtree(str(release_dir), ignore_errors=True)

    BUILD_DIR.mkdir(parents=True, exist_ok=True)

    ver_parts = VERSION.split(".")
    ver_tuple = ", ".join(str(int(p)) for p in ver_parts)
    version_file_content = f"""VSVersionInfo(
  ffi=FixedFileInfo(
    filevers=({ver_tuple}),
    prodvers=({ver_tuple}),
    mask=0x3f,
    flags=0x0,
    OS=0x40004,
    fileType=0x1,
    subtype=0x0,
    date=(0, 0),
  ),
  kids=[
    StringFileInfo(
      [
        StringTable(
          u'080404B0',
          [
            StringStruct(u'CompanyName', u'YunJi'),
            StringStruct(u'FileDescription', u'YunJi Smart IDE Workstation'),
            StringStruct(u'FileVersion', u'{VERSION}'),
            StringStruct(u'InternalName', u'YunJiSmartIDE'),
            StringStruct(u'LegalCopyright', u'Copyright 2026 YunJi'),
            StringStruct(u'OriginalFilename', u'YunJiSmartIDE.exe'),
            StringStruct(u'ProductName', u'YunJi Smart IDE Workstation'),
            StringStruct(u'ProductVersion', u'{VERSION}'),
          ]
        )
      ]
    ),
    VarFileInfo([VarStruct(u'Translation', [2052, 1200])])
  ]
)
"""
    version_file_path = DEV_APP_DIR / "version_info.txt"
    with open(str(version_file_path), "w", encoding="utf-8") as vf:
        vf.write(version_file_content)

    os.chdir(str(DEV_APP_DIR))

    icon_path = str(DEV_APP_DIR / "icon.ico")

    pyinstaller_args = [
        sys.executable, "-m", "PyInstaller",
        "--name", release_name,
        "--onefile", "--windowed",
        "--icon", icon_path,
        "--distpath", str(BUILD_DIR),
        "--workpath", str(BUILD_DIR / "_pyinstaller_work"),
        "--specpath", str(BUILD_DIR / "_pyinstaller_work"),
        "--clean", "--noconfirm",
        "--hidden-import", "PyQt6",
        "--hidden-import", "PyQt6.QtCore",
        "--hidden-import", "PyQt6.QtGui",
        "--hidden-import", "PyQt6.QtWidgets",
        "--hidden-import", "PyQt6.QtWebEngineWidgets",
        "--hidden-import", "PyQt6.QtWebEngineCore",
        "--hidden-import", "PyQt6.QtWebChannel",
        "--hidden-import", "backend",
        "--hidden-import", "http.server",
        "--hidden-import", "socketserver",
        "--hidden-import", "urllib.request",
        "--hidden-import", "urllib.error",
        "--hidden-import", "urllib.parse",
        "--hidden-import", "psutil",
        "--hidden-import", "uuid",
        "--exclude-module", "matplotlib",
        "--exclude-module", "scipy",
        "--exclude-module", "numpy",
        "--exclude-module", "tkinter",
        "--exclude-module", "tensorflow",
        "--exclude-module", "torch",
        "--add-data", f"{icon_path};.",
        "main.py"
    ]

    icon_png = str(DEV_APP_DIR / "icon.png")
    if os.path.exists(icon_png):
        pyinstaller_args.insert(-1, "--add-data")
        pyinstaller_args.insert(-1, f"{icon_png};.")

    print("  运行 PyInstaller (--onefile)...")
    subprocess.run(pyinstaller_args, check=True)

    release_dir.mkdir(parents=True, exist_ok=True)
    exe_src = BUILD_DIR / f"{release_name}.exe"
    if exe_src.exists():
        shutil.copy2(str(exe_src), str(release_dir / f"{release_name}.exe"))
        exe_size = exe_src.stat().st_size / (1024 * 1024)
        print(f"  ✓ EXE: {exe_src.name} ({exe_size:.1f} MB)")

    return release_dir


# ── 打包后处理：将运行时文件复制到发布目录（build/ 下的整合包）──
def post_build(release_dir: Path):
    """将资源组织到发布目录中（--onefile 模式，无 _internal/）

    发布目录结构：
      发布目录/
      ├── *.exe              # 单文件 EXE（包含所有运行时）
      ├── app/               # 应用程序（只读，纯净可发布）
      ├── data/              # 用户数据（可写，需备份）
      └── temp/              # 临时文件（可删除）
    """
    print("  打包后处理（--onefile 模式）...")

    app_dir = release_dir / "app"
    data_dir = release_dir / "data"
    temp_dir = release_dir / "temp"

    _ignore_cache = shutil.ignore_patterns(
        "__pycache__", "*.pyc", ".venv", ".uv_cache", ".bun_cache",
        "node_modules", "nodejs", "bun", ".env", ".env.*",
        "*.log", "*.tmp", "*.bak",
    )

    app_dir.mkdir(parents=True, exist_ok=True)

    dist_src = DEV_APP_DIR / "desktop" / "dist"
    dist_dst = app_dir / "desktop" / "dist"
    if dist_src.exists():
        if dist_dst.exists():
            shutil.rmtree(str(dist_dst), ignore_errors=True)
        shutil.copytree(str(dist_src), str(dist_dst), ignore=_ignore_cache)
        print("  ✓ 复制 desktop/dist/ -> app/desktop/dist/ (前端)")
    else:
        print("  ✗ desktop/dist/ 不存在，前端将不可用")

    icon_src = DEV_APP_DIR / "icon.ico"
    if icon_src.exists():
        shutil.copy2(str(icon_src), str(app_dir / "icon.ico"))
    icon_png_src = DEV_APP_DIR / "icon.png"
    if icon_png_src.exists():
        shutil.copy2(str(icon_png_src), str(app_dir / "icon.png"))

    bin_src = DEV_APP_DIR / "bin"
    bin_dst = app_dir / "bin"
    if bin_src.exists():
        if bin_dst.exists():
            shutil.rmtree(str(bin_dst), ignore_errors=True)
        shutil.copytree(str(bin_src), str(bin_dst), ignore=_ignore_cache)
        print("  ✓ 复制 bin/ -> app/bin/ (CLI)")

    stubs_src = DEV_APP_DIR / "stubs"
    stubs_dst = app_dir / "stubs"
    if stubs_src.exists():
        if stubs_dst.exists():
            shutil.rmtree(str(stubs_dst), ignore_errors=True)
        shutil.copytree(str(stubs_src), str(stubs_dst), ignore=_ignore_cache)
        print("  ✓ 复制 stubs/ -> app/stubs/")

    for fname in ["package.json", "bunfig.toml", "preload.ts"]:
        fsrc = DEV_APP_DIR / fname
        if fsrc.exists():
            shutil.copy2(str(fsrc), str(app_dir / fname))

    scripts_src = DEV_APP_DIR / "scripts"
    scripts_dst = app_dir / "scripts"
    if scripts_src.exists():
        if scripts_dst.exists():
            shutil.rmtree(str(scripts_dst), ignore_errors=True)
        shutil.copytree(str(scripts_src), str(scripts_dst), ignore=_ignore_cache)
        print("  ✓ 复制 scripts/ -> app/scripts/")

    api_src = DEV_APP_DIR / "api"
    api_dst = app_dir / "api"
    if api_src.exists():
        if api_dst.exists():
            shutil.rmtree(str(api_dst), ignore_errors=True)
        shutil.copytree(str(api_src), str(api_dst), ignore=_ignore_cache)
        print("  ✓ 复制 api/ -> app/api/ (API 服务代码)")

    src_src = DEV_APP_DIR / "src"
    src_dst = app_dir / "src"
    if src_src.exists():
        if src_dst.exists():
            shutil.rmtree(str(src_dst), ignore_errors=True)
        shutil.copytree(str(src_src), str(src_dst), ignore=_ignore_cache)
        print("  ✓ 复制 src/ -> app/src/ (CLI 代码)")

    uv_src = DEV_APP_DIR / "uv"
    uv_dst = app_dir / "uv"
    if uv_src.exists():
        if uv_dst.exists():
            shutil.rmtree(str(uv_dst), ignore_errors=True)
        shutil.copytree(str(uv_src), str(uv_dst), ignore=_ignore_cache)
        print("  ✓ 复制 uv/ -> app/uv/")

    vh_src = DEV_APP_DIR / "version_history.json"
    if vh_src.exists():
        shutil.copy2(str(vh_src), str(app_dir / "version_history.json"))

    data_dir.mkdir(parents=True, exist_ok=True)
    
    (data_dir / "public" / "models").mkdir(parents=True, exist_ok=True)
    (data_dir / "public" / "templates").mkdir(parents=True, exist_ok=True)
    (data_dir / "public" / "plugins").mkdir(parents=True, exist_ok=True)
    (data_dir / "public" / "api" / "qwen2api").mkdir(parents=True, exist_ok=True)
    (data_dir / "public" / "api" / "zhipu2api").mkdir(parents=True, exist_ok=True)
    
    (data_dir / "users" / "default" / "projects").mkdir(parents=True, exist_ok=True)
    (data_dir / "users" / "default" / "sessions").mkdir(parents=True, exist_ok=True)
    (data_dir / "users" / "default" / "webdata").mkdir(parents=True, exist_ok=True)
    
    print("  ✓ 创建 data/ 目录结构 (public/ + users/)")

    temp_dir.mkdir(parents=True, exist_ok=True)
    (temp_dir / "__pycache__").mkdir(parents=True, exist_ok=True)
    (temp_dir / "logs").mkdir(parents=True, exist_ok=True)
    (temp_dir / "cache").mkdir(parents=True, exist_ok=True)
    (temp_dir / "debug").mkdir(parents=True, exist_ok=True)
    (temp_dir / "tmp").mkdir(parents=True, exist_ok=True)
    print("  ✓ 创建 temp/ 目录结构")

    total_size = sum(f.stat().st_size for f in release_dir.rglob("*") if f.is_file())
    size_mb = total_size / (1024 * 1024)
    print(f"  发布目录大小: {size_mb:.1f} MB")


# ── 清理 ──
def cleanup():
    """清理 PyInstaller 临时文件"""
    work_dir = BUILD_DIR / "_pyinstaller_work"
    if work_dir.exists():
        try:
            shutil.rmtree(str(work_dir), ignore_errors=True)
            print("  清理 PyInstaller 临时文件")
        except:
            pass


# ── 部署到 dev/ ──
def _kill_running_exe():
    """尝试终止正在运行的旧版 EXE 进程，避免文件锁定"""
    import signal as sig_module
    current_pid = os.getpid()
    killed = []
    try:
        import psutil as _ps
        for proc in _ps.process_iter(['pid', 'name', 'exe']):
            try:
                pname = (proc.info.get('name') or '').lower()
                pexe = proc.info.get('exe') or ''
                if pname.startswith('云集智能编程工作站') and proc.info['pid'] != current_pid:
                    proc.terminate()
                    killed.append(pname)
            except (_ps.NoSuchProcess, _ps.AccessDenied):
                pass
    except ImportError:
        pass
    if killed:
        print(f"  已终止旧版进程: {', '.join(killed)}")
        time.sleep(1)
    return len(killed)


def _set_hidden_attribute(path: str):
    """设置 Windows 文件/文件夹为隐藏属性"""
    if os.name == 'nt':
        try:
            import ctypes
            ctypes.windll.kernel32.SetFileAttributesW(path, 0x02)  # FILE_ATTRIBUTE_HIDDEN
            return True
        except:
            pass
    return False


def _deploy_to_dev(release_dir: Path):
    """将构建产物复制到 dev/ 下（--onefile 模式，无 _internal/）

    目录架构:
    - EXE 直接放在 dev/ 下（dev/云集智能编程工作站vX.X.exe）
    - app/ 在 dev/ 下（源代码目录，Git管理）
    - data/ 在 dev/ 下（用户数据，Git不管理）
    - temp/ 在 dev/ 下（临时文件，Git不管理）
    """
    release_name = release_dir.name

    _kill_running_exe()

    new_exe = release_dir / f"{release_name}.exe"
    if new_exe.exists():
        existing = DEV_DIR / new_exe.name
        if existing.exists():
            try:
                existing.unlink()
            except PermissionError:
                print(f"  ⚠ EXE 被占用，尝试重命名旧文件...")
                backup_name = existing.stem + "_old" + existing.suffix
                backup_path = DEV_DIR / backup_name
                if backup_path.exists():
                    try:
                        backup_path.unlink()
                    except PermissionError:
                        pass
                try:
                    existing.rename(str(backup_path))
                    print(f"  旧 EXE 重命名为: {backup_name}")
                except PermissionError:
                    print(f"  ✗ 无法重命名旧 EXE，请手动关闭正在运行的应用后重试")
                    return
            print(f"  复制 EXE: {new_exe.name}")
        shutil.copy2(str(new_exe), str(DEV_DIR / new_exe.name))
        print(f"  ✓ 复制 EXE: {new_exe.name}")

    old_internal = DEV_DIR / "_internal"
    if old_internal.exists():
        try:
            shutil.rmtree(str(old_internal), ignore_errors=True)
            print(f"  ✓ 清理旧 _internal/ (--onefile 模式不再需要)")
        except Exception:
            pass

    icon_src = release_dir / "app" / "icon.ico"
    if not icon_src.exists():
        icon_src = release_dir / "icon.ico"
    if icon_src.exists():
        shutil.copy2(str(icon_src), str(DEV_DIR / "icon.ico"))
        print(f"  ✓ 复制 icon.ico")

    icon_png_src = release_dir / "app" / "icon.png"
    if not icon_png_src.exists():
        icon_png_src = release_dir / "icon.png"
    if icon_png_src.exists():
        shutil.copy2(str(icon_png_src), str(DEV_DIR / "icon.png"))
        print(f"  ✓ 复制 icon.png")

    dev_data_dir = DEV_DIR / "data"
    dev_temp_dir = DEV_DIR / "temp"
    dev_data_dir.mkdir(parents=True, exist_ok=True)
    dev_temp_dir.mkdir(parents=True, exist_ok=True)
    
    for sub in ["public/models", "public/templates", "public/plugins", "public/api/qwen2api", "public/api/zhipu2api"]:
        (dev_data_dir / sub).mkdir(parents=True, exist_ok=True)
    
    for sub in ["users/default/projects", "users/default/sessions", "users/default/webdata"]:
        (dev_data_dir / sub).mkdir(parents=True, exist_ok=True)
    
    for sub in ["__pycache__", "logs", "cache", "debug", "tmp"]:
        (dev_temp_dir / sub).mkdir(parents=True, exist_ok=True)
    print(f"  ✓ 确保 data/ 和 temp/ 目录结构 (数据分级)")

    print(f"  ✓ 部署完成，EXE 在 {DEV_DIR}")


# ── 记录版本 ──
def record_version(release_name, changes):
    history = load_version_history()

    git_commit = ""
    try:
        r = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(DEV_DIR),
            capture_output=True, text=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        if r.returncode == 0:
            git_commit = r.stdout.strip()
    except Exception:
        pass

    version_info = {
        "version": VERSION,
        "name": release_name,
        "changes": changes,
        "build_time": datetime.now().isoformat(),
        "git_commit": git_commit,
    }

    existing = [v for v in history if v.get("version") == VERSION]
    if existing:
        existing[0].update(version_info)
    else:
        history.insert(0, version_info)

    save_version_history(history)

    update_version_json(VERSION, release_name, changes)

    return version_info


def update_version_json(version, release_name, changes):
    ver_json_path = VERSION_JSON_FILE
    ver_dir = ver_json_path.parent
    ver_dir.mkdir(parents=True, exist_ok=True)

    data = {}
    if ver_json_path.exists():
        try:
            with open(str(ver_json_path), 'r', encoding='utf-8') as f:
                data = json.load(f)
        except Exception:
            data = {}

    data["latest"] = version

    entry = {
        "version": version,
        "date": datetime.now().strftime("%Y-%m-%d"),
        "exe": f"{release_name}.exe",
        "changes": changes or [],
    }

    existing = [v for v in data.get("versions", []) if v.get("version") != version]
    existing.insert(0, entry)
    data["versions"] = existing

    try:
        with open(str(ver_json_path), 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"  ✓ 更新 version.json (latest={version})")
    except Exception as e:
        print(f"  ⚠ 更新 version.json 失败: {e}")


# ── 主流程 ──
def main():
    print("=" * 60)
    print("  云集智能编程工作站 - 版本化构建工具 v3.0")
    print("=" * 60)
    print()
    print(f"  版本: {VERSION}")
    print(f"  源码: {DEV_APP_DIR}")
    print(f"  输出: {BUILD_DIR}")
    print()

    # 解析参数
    auto_push = False
    changes = None

    args = sys.argv[1:]
    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ["--push", "-p"]:
            auto_push = True
        elif arg in ["--no-push", "-np"]:
            auto_push = False
        elif arg in ["--desc", "-d"] and i + 1 < len(args):
            changes = args[i + 1].split('|')
            i += 1
        i += 1

    try:
        # Step 1: 构建前端
        print("── Step 1: 构建前端 ──")
        if not build_frontend():
            print("\n前端构建失败，终止打包")
            sys.exit(1)
        print()

        # Step 2: PyInstaller 打包
        print("── Step 2: PyInstaller 打包 ──")
        release_dir = build_exe()
        print()

        # Step 3: 打包后处理
        print("── Step 3: 打包后处理 ──")
        post_build(release_dir)
        print()

        # Step 4: 清理
        print("── Step 4: 清理 ──")
        cleanup()
        print()

        # Step 5: 记录版本
        if changes is None:
            changes = get_version_description()
        release_name = release_dir.name
        record_version(release_name, changes)

        # Step 6: 将 EXE + _internal/ 复制到 dev/ 下
        print("── Step 6: 部署到 dev/ ──")
        _deploy_to_dev(release_dir)

        # 完成
        exe_path = DEV_DIR / f"{release_name}.exe"
        print("=" * 60)
        print("  构建完成！")
        print(f"  发布目录: {release_dir}")
        print(f"  运行目录: {DEV_DIR}")
        if exe_path.exists():
            print(f"  EXE 文件: {exe_path}")
            size_mb = exe_path.stat().st_size / (1024 * 1024)
            print(f"  EXE 大小: {size_mb:.1f} MB")
        print(f"  资源目录: {DEV_APP_DIR}")
        print(f"  版本历史: {VERSION_HISTORY_FILE}")
        print("=" * 60)

    except subprocess.CalledProcessError as e:
        print(f"\n打包失败：{e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n错误：{e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
