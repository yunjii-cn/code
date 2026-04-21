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
  - PyInstaller 工作目录: build/ (仅构建用)
  - 开发测试目录: dev/云集智能编程工作站vX.X/ (EXE + _internal/)
  - 用户拿到整合包后，EXE 直接在 dev/ 下运行
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

if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

VERSION = datetime.now().strftime("%Y.%m.%d.%H%M")
ROOT_DIR = Path(__file__).resolve().parent  # build-version.py 在 dev/app/ 下
DEV_APP_DIR = ROOT_DIR
DEV_DIR = ROOT_DIR.parent                   # dev/ 根目录
BUILD_DIR = ROOT_DIR.parent.parent / "build"  # 项目根/build/ (PyInstaller 工作目录)
VERSION_HISTORY_FILE = ROOT_DIR.parent.parent / "version_history.json"


def load_version_history():
    if VERSION_HISTORY_FILE.exists():
        try:
            with open(VERSION_HISTORY_FILE, 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception:
            pass
    return {}


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
    if dist_path.exists():
        print("  ✓ 前端已构建 (desktop/dist/)")
        return True

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
        if bun_exe and bun_exe.exists():
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
    """用 PyInstaller --onedir 模式打包，输出到 build/发布/xxx/"""
    print(f"  PyInstaller 打包 (v{VERSION})...")

    release_name = f"云集智能编程工作站v{VERSION}"
    release_dir = BUILD_DIR / release_name

    if release_dir.exists():
        print(f"  清理旧发布目录: {release_name}")
        shutil.rmtree(str(release_dir), ignore_errors=True)

    BUILD_DIR.mkdir(parents=True, exist_ok=True)

    os.chdir(str(DEV_APP_DIR))

    icon_path = str(DEV_APP_DIR / "icon.ico")

    pyinstaller_args = [
        sys.executable, "-m", "PyInstaller",
        "--name", release_name,
        # --onedir 模式：QtWebEngine 不支持 --onefile (DLL 路径问题)
        "--onedir", "--windowed",
        "--icon", icon_path,
        "--distpath", str(BUILD_DIR),
        "--workpath", str(BUILD_DIR / "_pyinstaller_work"),
        "--specpath", str(BUILD_DIR / "_pyinstaller_work"),
        "--clean", "--noconfirm",
        # PyQt6 核心
        "--hidden-import", "PyQt6",
        "--hidden-import", "PyQt6.QtCore",
        "--hidden-import", "PyQt6.QtGui",
        "--hidden-import", "PyQt6.QtWidgets",
        # PyQt6 WebEngine (替代 Electron)
        "--hidden-import", "PyQt6.QtWebEngineWidgets",
        "--hidden-import", "PyQt6.QtWebEngineCore",
        "--hidden-import", "PyQt6.QtWebChannel",
        # 后端模块
        "--hidden-import", "backend",
        "--hidden-import", "http.server",
        "--hidden-import", "socketserver",
        "--hidden-import", "urllib.request",
        "--hidden-import", "urllib.error",
        "--hidden-import", "urllib.parse",
        # 系统工具
        "--hidden-import", "psutil",
        "--hidden-import", "uuid",
        # 排除大型无用模块
        "--exclude-module", "matplotlib",
        "--exclude-module", "scipy",
        "--exclude-module", "numpy",
        "--exclude-module", "tkinter",
        "--exclude-module", "tensorflow",
        "--exclude-module", "torch",
        "main.py"
    ]

    print("  运行 PyInstaller (--onedir)...")
    subprocess.run(pyinstaller_args, check=True)

    return release_dir


# ── 打包后处理：将运行时文件复制到发布目录 ──
def post_build(release_dir: Path):
    """将 desktop/dist/ 和其他运行时资源复制到发布目录"""
    print("  打包后处理...")

    # 1. 复制 desktop/dist/ (Vue 前端)
    dist_src = DEV_APP_DIR / "desktop" / "dist"
    dist_dst = release_dir / "desktop" / "dist"
    if dist_src.exists():
        if dist_dst.exists():
            shutil.rmtree(str(dist_dst), ignore_errors=True)
        shutil.copytree(str(dist_src), str(dist_dst))
        print("  ✓ 复制 desktop/dist/ (前端)")
    else:
        print("  ✗ desktop/dist/ 不存在，前端将不可用")

    # 2. 复制 icon.ico (窗口图标)
    icon_src = DEV_APP_DIR / "icon.ico"
    if icon_src.exists():
        shutil.copy2(str(icon_src), str(release_dir / "icon.ico"))

    # 3. 复制 .env (配置文件，如果存在)
    env_src = DEV_APP_DIR / ".env"
    if env_src.exists():
        shutil.copy2(str(env_src), str(release_dir / ".env"))

    # 4. 复制 bin/ (CLI 工具)
    bin_src = DEV_APP_DIR / "bin"
    bin_dst = release_dir / "bin"
    if bin_src.exists() and not bin_dst.exists():
        shutil.copytree(str(bin_src), str(bin_dst))
        print("  ✓ 复制 bin/ (CLI)")

    # 5. 复制 stubs/ (类型定义)
    stubs_src = DEV_APP_DIR / "stubs"
    stubs_dst = release_dir / "stubs"
    if stubs_src.exists() and not stubs_dst.exists():
        shutil.copytree(str(stubs_src), str(stubs_dst))
        print("  ✓ 复制 stubs/")

    # 6. nodejs/ 和 bun/ 不复制 (发布版由用户点击部署维护自动下载)
    #    如果 dev/app/ 下有，也复制过去以方便使用
    for runtime_dir in ["nodejs", "bun"]:
        src = DEV_APP_DIR / runtime_dir
        dst = release_dir / runtime_dir
        if src.exists() and not dst.exists():
            try:
                shutil.copytree(str(src), str(dst_dst) if False else str(dst))
                print(f"  ✓ 复制 {runtime_dir}/")
            except Exception as e:
                print(f"  ⚠ 复制 {runtime_dir}/ 失败: {e}")

    # 7. node_modules/ (如果存在，CLI 依赖)
    nm_src = DEV_APP_DIR / "node_modules"
    nm_dst = release_dir / "node_modules"
    if nm_src.exists() and not nm_dst.exists():
        try:
            shutil.copytree(str(nm_src), str(nm_dst))
            print("  ✓ 复制 node_modules/")
        except Exception as e:
            print(f"  ⚠ 复制 node_modules/ 失败: {e}")

    # 8. package.json
    pkg_src = DEV_APP_DIR / "package.json"
    if pkg_src.exists():
        shutil.copy2(str(pkg_src), str(release_dir / "package.json"))

    # 计算发布目录大小
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


# ── 记录版本 ──
def record_version(release_name, changes):
    history = load_version_history()

    version_info = {
        "version": release_name,
        "changes": changes,
        "build_time": datetime.now().isoformat(),
        "version_number": VERSION,
    }

    history[release_name] = version_info
    save_version_history(history)
    return version_info


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

        # Step 6: 复制到 dev/ 目录（开发测试用）
        dev_release_dir = DEV_DIR / release_name
        print("── Step 6: 复制到 dev/ 目录 ──")
        if dev_release_dir.exists():
            print(f"  清理旧目录: {dev_release_dir}")
            shutil.rmtree(str(dev_release_dir), ignore_errors=True)
        shutil.copytree(str(release_dir), str(dev_release_dir))
        print(f"  ✓ 已复制到 {dev_release_dir}")

        # 完成
        exe_path = dev_release_dir / f"{release_name}.exe"
        print("=" * 60)
        print("  构建完成！")
        print(f"  发布目录: {release_dir}")
        print(f"  开发目录: {dev_release_dir}")
        if exe_path.exists():
            print(f"  EXE 文件: {exe_path}")
            size_mb = exe_path.stat().st_size / (1024 * 1024)
            total_size = sum(f.stat().st_size for f in dev_release_dir.rglob("*") if f.is_file())
            total_mb = total_size / (1024 * 1024)
            print(f"  EXE 大小: {size_mb:.1f} MB | 整合包大小: {total_mb:.1f} MB")
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
