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
  - --onedir 模式打包（EXE + _internal/，方便维护）
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
import zipfile
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
    """用 PyInstaller --onedir 模式打包，输出 EXE + _internal/"""
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
        "--onedir", "--windowed",
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

    print("  运行 PyInstaller (--onedir)...")
    subprocess.run(pyinstaller_args, check=True)

    exe_src = BUILD_DIR / release_name / f"{release_name}.exe"
    if exe_src.exists():
        exe_size = exe_src.stat().st_size / (1024 * 1024)
        print(f"  ✓ EXE: {exe_src.name} ({exe_size:.1f} MB)")

    return release_dir


# ── 打包后处理：将运行时文件复制到发布目录（build/ 下的整合包）──
def post_build(release_dir: Path):
    """将资源组织到发布目录中（--onedir 模式）

    发布目录结构：
      发布目录/
      ├── *.exe              # 主程序 EXE
      ├── _internal/         # PyInstaller 运行时
      ├── app/               # 应用程序（只读，纯净可发布）
      ├── data/              # 用户数据（可写，需备份）
      └── temp/              # 临时文件（可删除）
    """
    print("  打包后处理（--onedir 模式）...")

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
    """将构建产物部署到 dev/ 下（--onedir 模式，硬链接机制）

    目录架构（对齐参考项目）:
    - EXE 先存入 dev/ver/ 目录（版本仓库）
    - dev/ 下的 EXE 是 ver/ 中最新版本的硬链接
    - _internal/ 放在 dev/ 下
    - app/ 在 dev/ 下（源代码目录，Git管理）
    - data/ 在 dev/ 下（用户数据，Git不管理）
    - temp/ 在 dev/ 下（临时文件，Git不管理）
    """
    release_name = release_dir.name

    _kill_running_exe()

    ver_dir = DEV_DIR / "ver"
    ver_dir.mkdir(parents=True, exist_ok=True)

    new_exe = release_dir / f"{release_name}.exe"
    if new_exe.exists():
        ver_exe = ver_dir / new_exe.name
        if ver_exe.exists():
            try:
                ver_exe.unlink()
            except PermissionError:
                pass
        shutil.copy2(str(new_exe), str(ver_exe))
        print(f"  ✓ 复制 EXE 到 ver/: {new_exe.name}")

        dev_exe = DEV_DIR / new_exe.name
        if dev_exe.exists():
            try:
                dev_exe.unlink()
            except PermissionError:
                print(f"  ⚠ EXE 被占用，尝试重命名旧文件...")
                backup_name = dev_exe.stem + "_old" + dev_exe.suffix
                backup_path = DEV_DIR / backup_name
                if backup_path.exists():
                    try:
                        backup_path.unlink()
                    except PermissionError:
                        pass
                try:
                    dev_exe.rename(str(backup_path))
                    print(f"  旧 EXE 重命名为: {backup_name}")
                except PermissionError:
                    print(f"  ✗ 无法重命名旧 EXE，请手动关闭正在运行的应用后重试")
                    return

        try:
            subprocess.run(
                ["cmd", "/c", "mklink", "/H", str(dev_exe), str(ver_exe)],
                check=True, capture_output=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            print(f"  ✓ 创建硬链接: {dev_exe.name} → ver/{ver_exe.name}")
        except Exception:
            shutil.copy2(str(ver_exe), str(dev_exe))
            print(f"  ✓ 硬链接失败，回退复制: {dev_exe.name}")

    new_internal = release_dir / "_internal"
    dev_internal = DEV_DIR / "_internal"
    if new_internal.exists():
        if dev_internal.exists():
            try:
                shutil.rmtree(str(dev_internal), ignore_errors=True)
            except Exception:
                pass
        shutil.copytree(str(new_internal), str(dev_internal))
        print(f"  ✓ 复制 _internal/")

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

    exe_filename = f"{release_name}.exe"
    ver_exe_path = DEV_DIR / "ver" / exe_filename
    size_mb = 0.0
    if ver_exe_path.exists():
        size_mb = round(ver_exe_path.stat().st_size / (1024 * 1024), 1)

    entry = {
        "version": version,
        "date": datetime.now().strftime("%Y-%m-%d"),
        "filename": exe_filename,
        "exe": exe_filename,
        "size_mb": size_mb,
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


# ── 创建 app_core.zip 资源包 ──
def create_app_core_zip(version):
    """打包 app/ 核心文件为 zip，供引导器下载"""
    print(f"  创建 app_core.zip (v{version})...")
    zip_path = DEV_DIR / "ver" / f"app_core_v{version}.zip"

    _ignore = shutil.ignore_patterns(
        "__pycache__", "*.pyc", ".venv", ".uv_cache", ".bun_cache",
        "node_modules", "nodejs", "bun", ".env", ".env.*",
        "*.log", "*.tmp", "*.bak", "cli_debug.log",
        "package-lock.json",
    )

    app_src = DEV_APP_DIR
    file_count = 0

    with zipfile.ZipFile(str(zip_path), 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as zf:
        for item in ["main.py", "backend.py", "package.json", "bunfig.toml",
                      "icon.ico", "icon.png", "version_history.json"]:
            src = app_src / item
            if src.exists():
                zf.write(str(src), f"app/{item}")
                file_count += 1

        for subdir in ["desktop/dist", "bin", "stubs", "scripts", "api", "src", "uv",
                        "python", "shims", "desktop/renderer"]:
            src_dir = app_src / subdir
            if not src_dir.exists():
                continue
            for fpath in src_dir.rglob("*"):
                if fpath.is_file():
                    skip = False
                    for pattern in ["__pycache__", ".pyc", ".venv", "node_modules",
                                    "nodejs", "bun", ".env", ".uv_cache", ".bun_cache",
                                    "*.log", "*.tmp", "*.bak", "cli_debug.log",
                                    "package-lock.json"]:
                        if pattern in str(fpath) or fpath.name == pattern:
                            skip = True
                            break
                    if not skip:
                        arc_name = f"app/{subdir}/{fpath.relative_to(src_dir)}"
                        zf.write(str(fpath), arc_name)
                        file_count += 1

    zip_size = zip_path.stat().st_size / (1024 * 1024)
    print(f"  ✓ app_core.zip: {zip_size:.1f}MB ({file_count} 文件)")
    return zip_path


# ── 构建自部署启动器 ──
def build_bootstrapper():
    """构建自部署启动器 EXE（--onefile --windowed，基于 launcher_stub.py）

    启动器逻辑：用户点击 → 检测部署目录 → 已部署则直接启动 → 未部署则下载 deploy zip 解压
    """
    print(f"  构建自部署启动器 (launcher_stub)...")

    stub_src = DEV_APP_DIR / "launcher_stub.py"
    if not stub_src.exists():
        print("  ✗ launcher_stub.py 不存在")
        return None

    output_dir = BUILD_DIR / "bootstrapper"
    if output_dir.exists():
        shutil.rmtree(str(output_dir), ignore_errors=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    icon_path = str(DEV_APP_DIR / "icon.ico")

    args = [
        sys.executable, "-m", "PyInstaller",
        "--onefile", "--windowed",
        "--name", "云集智能编程工作站",
        "--icon", icon_path,
        "--distpath", str(output_dir),
        "--workpath", str(output_dir / "_work"),
        "--specpath", str(output_dir / "_work"),
        "--clean", "--noconfirm",
        "--exclude-module", "PyQt6",
        "--exclude-module", "PyQt6.QtCore",
        "--exclude-module", "PyQt6.QtGui",
        "--exclude-module", "PyQt6.QtWidgets",
        "--exclude-module", "PyQt6.QtWebEngineWidgets",
        "--exclude-module", "PyQt6.QtWebEngineCore",
        "--exclude-module", "PyQt6.QtWebChannel",
        "--exclude-module", "numpy",
        "--exclude-module", "scipy",
        "--exclude-module", "matplotlib",
        "--exclude-module", "PIL",
        "--exclude-module", "psutil",
        "--exclude-module", "fastapi",
        "--exclude-module", "uvicorn",
        "--exclude-module", "pydantic",
        "--exclude-module", "pydantic_core",
        "--exclude-module", "httpx",
        "--exclude-module", "httpcore",
        "--exclude-module", "starlette",
        "--exclude-module", "anyio",
        "--exclude-module", "sniffio",
        "--exclude-module", "watchfiles",
    ]

    VENV_DIR_LOCAL = DEV_DIR / "data" / ".venv"
    VENV_PYTHON_LOCAL = VENV_DIR_LOCAL / "Scripts" / "python.exe"
    PYTHON_DIR_LOCAL = VENV_PYTHON_LOCAL.parent
    TCL_DIR_LOCAL = PYTHON_DIR_LOCAL.parent / "tcl"
    DLLS_DIR_LOCAL = PYTHON_DIR_LOCAL / "DLLs"
    LIB_DIR_LOCAL = VENV_DIR_LOCAL / "Lib"

    if not TCL_DIR_LOCAL.is_dir():
        for candidate in [
            Path(r"C:\Program Files\Python312\tcl"),
            Path(r"C:\Python312\tcl"),
            PYTHON_DIR_LOCAL.parent / "tcl",
        ]:
            if candidate.is_dir():
                TCL_DIR_LOCAL = candidate
                DLLS_DIR_LOCAL = candidate.parent / "DLLs"
                LIB_DIR_LOCAL = candidate.parent / "Lib"
                break

    tkinter_dir = LIB_DIR_LOCAL / "tkinter"
    if tkinter_dir.is_dir():
        args.extend(["--add-data", f"{tkinter_dir};tkinter"])
        print(f"    已包含: tkinter ({tkinter_dir})")

    tcl86_dir = TCL_DIR_LOCAL / "tcl8.6"
    tk86_dir = TCL_DIR_LOCAL / "tk8.6"
    if tcl86_dir.is_dir():
        args.extend(["--add-data", f"{tcl86_dir};tcl/tcl8.6"])
    if tk86_dir.is_dir():
        args.extend(["--add-data", f"{tk86_dir};tcl/tk8.6"])

    tcl_dll = DLLS_DIR_LOCAL / "tcl86t.dll"
    tk_dll = DLLS_DIR_LOCAL / "tk86t.dll"
    tkinter_pyd = DLLS_DIR_LOCAL / "_tkinter.pyd"
    if tcl_dll.is_file():
        args.extend(["--add-binary", f"{tcl_dll};."])
    if tk_dll.is_file():
        args.extend(["--add-binary", f"{tk_dll};."])
    if tkinter_pyd.is_file():
        args.extend(["--add-binary", f"{tkinter_pyd};."])

    if os.path.isfile(icon_path):
        args.extend(["--add-data", f"{icon_path};."])
    icon_png = str(DEV_APP_DIR / "icon.png")
    if os.path.isfile(icon_png):
        args.extend(["--add-data", f"{icon_png};."])

    args.append(str(stub_src))

    print(f"    运行 PyInstaller...")
    subprocess.run(args, check=True)

    exe_path = output_dir / "云集智能编程工作站.exe"
    if exe_path.exists():
        size_mb = exe_path.stat().st_size / (1024 * 1024)
        print(f"  ✓ 自部署启动器: {exe_path.name} ({size_mb:.1f}MB)")

        deploy_path = DEV_DIR / f"launcher_云集智能编程工作站.exe"
        shutil.copy2(str(exe_path), str(deploy_path))
        print(f"  ✓ 复制到: {deploy_path}")
        return exe_path
    else:
        print("  ✗ 启动器构建失败")
        return None


# ── 发布到 Gitee Releases ──
def create_gitee_release(version, release_name, changes, exe_path, app_core_zip=None):
    """通过 Gitee API 创建 Release 并上传 EXE + app_core.zip"""
    from urllib.request import urlopen, Request
    from urllib.error import HTTPError
    import base64

    GITEE_OWNER = "yunjii"
    GITEE_REPO = "code"
    GITEE_API = f"https://gitee.com/api/v5/repos/{GITEE_OWNER}/{GITEE_REPO}"

    token = ""
    env_path = DEV_DIR / "data" / ".env"
    if not env_path.exists():
        env_path = DEV_APP_DIR / ".env"
    if env_path.exists():
        try:
            with open(str(env_path), "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line.startswith("GITEE_TOKEN="):
                        token = line.split("=", 1)[1].strip()
                        break
        except Exception:
            pass

    if not token:
        print("  ✗ 未找到 GITEE_TOKEN，无法发布 Release")
        print("    请在 .env 文件中设置 GITEE_TOKEN=your_token")
        return False

    tag_name = f"v{version}"
    body = "\n".join(f"- {c}" for c in changes) if changes else f"v{version} 发布"

    import json as _json

    release_url = f"{GITEE_API}/releases"
    release_data = {
        "access_token": token,
        "tag_name": tag_name,
        "name": release_name,
        "body": body,
        "target_commitish": "main",
        "prerelease": "false",
    }

    release_id = None
    try:
        req = Request(release_url, data=_json.dumps(release_data).encode("utf-8"))
        req.add_header("Content-Type", "application/json")
        req.add_header("User-Agent", "Mozilla/5.0")
        resp = urlopen(req, timeout=30)
        resp_data = _json.loads(resp.read().decode("utf-8"))
        release_id = resp_data.get("id")
        print(f"  ✓ 创建 Release: {tag_name} (id={release_id})")
    except HTTPError as e:
        if e.code == 422:
            print(f"  ⚠ Release {tag_name} 已存在，尝试获取现有 Release...")
            try:
                list_url = f"{GITEE_API}/releases?access_token={token}"
                req = Request(list_url)
                req.add_header("User-Agent", "Mozilla/5.0")
                resp = urlopen(req, timeout=30)
                releases = _json.loads(resp.read().decode("utf-8"))
                for r in releases:
                    if r.get("tag_name") == tag_name:
                        release_id = r["id"]
                        print(f"  ✓ 找到现有 Release: id={release_id}")
                        break
                else:
                    print("  ✗ 未找到现有 Release")
                    return False
            except Exception as e2:
                print(f"  ✗ 获取 Release 列表失败: {e2}")
                return False
        else:
            print(f"  ✗ 创建 Release 失败: {e.code} {e.reason}")
            return False
    except Exception as e:
        print(f"  ✗ 创建 Release 异常: {e}")
        return False

    if release_id is None:
        return False

    upload_url = f"{GITEE_API}/releases/{release_id}/attach_files"

    files_to_upload = [exe_path]
    if app_core_zip and app_core_zip.exists():
        files_to_upload.append(app_core_zip)

    for file_path in files_to_upload:
        file_size = file_path.stat().st_size
        print(f"  正在上传 {file_path.name} ({file_size / (1024*1024):.1f}MB)...")

        try:
            import uuid as _uuid
            boundary = _uuid.uuid4().hex
            filename = file_path.name

            with open(str(file_path), "rb") as f:
                file_data = f.read()

            body_parts = []
            body_parts.append(f"--{boundary}".encode())
            body_parts.append(f'Content-Disposition: form-data; name="file"; filename="{filename}"'.encode())
            body_parts.append(b"Content-Type: application/octet-stream")
            body_parts.append(b"")
            body_parts.append(file_data)
            body_parts.append(f"--{boundary}--".encode())
            body_bytes = b"\r\n".join(body_parts)

            upload_params = f"access_token={token}&name={filename}"
            req = Request(f"{upload_url}?{upload_params}", data=body_bytes)
            req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")
            req.add_header("User-Agent", "Mozilla/5.0")
            resp = urlopen(req, timeout=600)
            print(f"  ✓ 上传完成: {filename}")
        except Exception as e:
            print(f"  ✗ 上传失败: {filename} - {e}")
            return False

    return True


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
    auto_release = False
    build_bootstrap = False
    changes = None

    args = sys.argv[1:]
    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ["--push", "-p"]:
            auto_push = True
        elif arg in ["--no-push", "-np"]:
            auto_push = False
        elif arg in ["--release", "-r"]:
            auto_release = True
        elif arg in ["--bootstrapper", "-b"]:
            build_bootstrap = True
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

        if auto_release:
            print()
            print("── Step 7: 发布到 Gitee Releases ──")
            ver_exe = DEV_DIR / "ver" / f"{release_name}.exe"
            if ver_exe.exists():
                app_core_zip = create_app_core_zip(VERSION)
                create_gitee_release(VERSION, release_name, changes or [], ver_exe, app_core_zip)
            else:
                print("  ✗ EXE 文件不存在，跳过发布")

        if build_bootstrap:
            print()
            print("── Step 8: 构建自部署启动器 ──")
            build_bootstrapper()

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
