"""
云集智能编程工作站 - Web版构建脚本（自部署模式）

流程:
1. npm run build 构建前端 (Vue.js -> dist/)
2. 复制 dist/ 到 dev/app/static/
3. PyInstaller --onefile 打包 (含 static/ + project.json)
4. EXE 输出到 dev/dist/
5. 创建硬链接入口 dev/云集智能编程工作站.exe → dist 中 EXE

自部署机制:
- EXE 首次运行时自动检测 .yunji.lock / app/ 目录
- 未找到则自动创建目录结构，将自身复制到 ver/ 并创建硬链接入口

用法:
    python build/build_web.py
    python build/build_web.py --skip-frontend   # 跳过前端构建（已构建过）
"""

import os
import sys
import re
import shutil
import subprocess
from datetime import datetime

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEV_DIR = os.path.join(PROJECT_ROOT, "dev")
APP_DIR = os.path.join(DEV_DIR, "app")
WEB_DIR = os.path.join(DEV_DIR, "web")
LAUNCHER_PY = os.path.join(APP_DIR, "launcher_web.py")
API_MAIN_PY = os.path.join(APP_DIR, "api_main.py")
ICON_ICO = os.path.join(APP_DIR, "icon.ico")
ICON_PNG = os.path.join(APP_DIR, "icon.png")
VERSION_INFO = os.path.join(APP_DIR, "version_info.txt")
BUILD_ROOT = os.path.join(PROJECT_ROOT, "build")
STATIC_DIR = os.path.join(APP_DIR, "static")
PROJECT_JSON = os.path.join(PROJECT_ROOT, "project.json")
DIST_DIR = os.path.join(DEV_DIR, "dist")
VER_DIR = os.path.join(DEV_DIR, "ver")
BRAND_NAME = "云集智能编程工作站"

NODEJS_DIR = r"D:\Programs\nodejs"
NPM_CMD = os.path.join(NODEJS_DIR, "npm.cmd")

EXCLUDED_PATTERNS = [
    'matplotlib', 'scipy', 'numpy', 'tensorflow', 'torch',
    'tkinter', '_tkinter', 'CustomTkinter',
    'PyQt6', 'PyQt5',
    'PIL', 'Pillow',
    'cv2', 'opencv',
]


def get_version():
    return datetime.now().strftime("%Y.%m.%d.%H%M")


def patch_version(api_main_path, version, version_info_path):
    with open(api_main_path, 'r', encoding='utf-8') as f:
        content = f.read()
    original = content
    content = re.sub(
        r'VERSION = "[^"]*"',
        f'VERSION = "{version}"',
        content
    )
    with open(api_main_path, 'w', encoding='utf-8') as f:
        f.write(content)

    original_vi = ""
    if os.path.isfile(version_info_path):
        with open(version_info_path, 'r', encoding='utf-8') as f:
            vi_content = f.read()
        original_vi = vi_content
        parts = [int(p) for p in version.split('.')]
        vi_content = re.sub(r'filevers=\(\d+,\s*\d+,\s*\d+,\s*\d+\)',
                            f'filevers=({parts[0]}, {parts[1]}, {parts[2]}, {parts[3]})', vi_content)
        vi_content = re.sub(r'prodvers=\(\d+,\s*\d+,\s*\d+,\s*\d+\)',
                            f'prodvers=({parts[0]}, {parts[1]}, {parts[2]}, {parts[3]})', vi_content)
        vi_content = re.sub(r"StringStruct\(u'FileVersion',\s*u'[^']*'\)",
                            f"StringStruct(u'FileVersion', u'{version}')", vi_content)
        vi_content = re.sub(r"StringStruct\(u'ProductVersion',\s*u'[^']*'\)",
                            f"StringStruct(u'ProductVersion', u'{version}')", vi_content)
        with open(version_info_path, 'w', encoding='utf-8') as f:
            f.write(vi_content)

    return original, original_vi


def restore_version(api_main_path, original_content, version_info_path, original_vi):
    with open(api_main_path, 'w', encoding='utf-8') as f:
        f.write(original_content)
    if original_vi and os.path.isfile(version_info_path):
        with open(version_info_path, 'w', encoding='utf-8') as f:
            f.write(original_vi)


def build_frontend():
    print("\n" + "=" * 60)
    print("  Step 1: Build Frontend (Vue.js)")
    print("=" * 60)

    if not os.path.isdir(WEB_DIR):
        print(f"[ERROR] Web directory not found: {WEB_DIR}")
        return False

    if not os.path.isfile(os.path.join(WEB_DIR, "package.json")):
        print(f"[ERROR] package.json not found in: {WEB_DIR}")
        return False

    node_modules = os.path.join(WEB_DIR, "node_modules")
    if not os.path.isdir(node_modules):
        print("  Installing npm dependencies...")
        result = subprocess.run(
            [NPM_CMD, "install"],
            cwd=WEB_DIR,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"[ERROR] npm install failed:\n{result.stderr}")
            return False
        print("  npm install done.")

    print("  Running npm run build...")
    result = subprocess.run(
        [NPM_CMD, "run", "build"],
        cwd=WEB_DIR,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"[ERROR] npm run build failed:\n{result.stderr}")
        return False

    dist_dir = os.path.join(WEB_DIR, "dist")
    if not os.path.isdir(dist_dir):
        print(f"[ERROR] dist directory not found after build: {dist_dir}")
        return False

    file_count = sum(len(files) for _, _, files in os.walk(dist_dir))
    print(f"  Frontend build done: {file_count} files in dist/")
    return True


def copy_static_files():
    print("\n" + "=" * 60)
    print("  Step 2: Copy static files")
    print("=" * 60)

    src_dist = os.path.join(WEB_DIR, "dist")
    if not os.path.isdir(src_dist):
        print(f"[ERROR] Frontend dist not found: {src_dist}")
        return False

    if os.path.isdir(STATIC_DIR):
        shutil.rmtree(STATIC_DIR)
        print(f"  Cleaned old static/")

    shutil.copytree(src_dist, STATIC_DIR)
    file_count = sum(len(files) for _, _, files in os.walk(STATIC_DIR))
    total_size = sum(
        os.path.getsize(os.path.join(dirpath, f))
        for dirpath, _, filenames in os.walk(STATIC_DIR)
        for f in filenames
    )
    print(f"  Copied {file_count} files ({total_size / 1024:.0f} KB) -> {STATIC_DIR}")
    return True


def generate_icon():
    if os.path.isfile(ICON_ICO):
        print(f"  Icon exists: {ICON_ICO}")
        return True

    if not os.path.isfile(ICON_PNG):
        print("  Warning: No icon files found, skipping icon generation")
        return True

    try:
        from PIL import Image
        img = Image.open(ICON_PNG)
        ico_sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]
        img.save(ICON_ICO, format='ICO', sizes=ico_sizes)
        print(f"  Generated: icon.ico")
        return True
    except ImportError:
        print("  Warning: Pillow not installed, cannot generate .ico from .png")
        return True
    except Exception as e:
        print(f"  Warning: Icon generation failed: {e}")
        return True


def generate_spec(version, exe_name, pyinstaller_build_dir):
    datas_entries = []

    if os.path.isdir(STATIC_DIR):
        datas_entries.append(f"(r'{STATIC_DIR}', 'static')")

    if os.path.isfile(ICON_ICO):
        datas_entries.append(f"(r'{ICON_ICO}', '.')")
    if os.path.isfile(ICON_PNG):
        datas_entries.append(f"(r'{ICON_PNG}', '.')")

    if os.path.isfile(PROJECT_JSON):
        datas_entries.append(f"(r'{PROJECT_JSON}', '.')")

    datas_str = ",\n    ".join(datas_entries) if datas_entries else ""

    excluded_str = ", ".join(f"'{p}'" for p in EXCLUDED_PATTERNS)

    hidden_imports = [
        'uvicorn.logging', 'uvicorn.loops', 'uvicorn.loops.auto',
        'uvicorn.protocols', 'uvicorn.protocols.http', 'uvicorn.protocols.http.auto',
        'uvicorn.protocols.websockets', 'uvicorn.protocols.websockets.auto',
        'uvicorn.lifespan', 'uvicorn.lifespan.on',
        'fastapi', 'fastapi.responses', 'fastapi.staticfiles',
        'starlette', 'starlette.routing', 'starlette.middleware',
        'webview', 'webview.platforms', 'webview.platforms.edgechromium',
        'anyio._back._eventloop',
        'httpcore', 'httpcore._async', 'httpcore._sync',
        'httpx', 'httpx._transports', 'httpx._transports.default',
        'sniffio', 'h11',
        'ai_service', 'cli_service', 'config_service',
        'env_service', 'project_service', 'update_service',
        'routes.ai', 'routes.env', 'routes.project',
        'routes.system', 'routes.version', 'routes.ws',
    ]

    hidden_str = ",\n    ".join(f"'{m}'" for m in hidden_imports)

    spec_content = f"""# -*- mode: python ; coding: utf-8 -*-
import sys
import os
from PyInstaller.building.build_main import Analysis as _OrigAnalysis

block_cipher = None

_excluded_patterns = [{excluded_str}]

class FilteredAnalysis(_OrigAnalysis):
    def assemble(self):
        result = super().assemble()
        filtered_binaries = []
        for dest, src, typecode in self.binaries:
            skip = False
            for ex in _excluded_patterns:
                if ex.lower() in dest.lower() or ex.lower() in src.lower():
                    skip = True
                    break
            if not skip:
                filtered_binaries.append((dest, src, typecode))
        self.binaries = filtered_binaries

        filtered_datas = []
        for item in self.datas:
            dest = item[0] if len(item) > 0 else ""
            src = item[1] if len(item) > 1 else ""
            skip = False
            for ex in _excluded_patterns:
                if ex.lower() in dest.lower() or ex.lower() in src.lower():
                    skip = True
                    break
            if not skip:
                filtered_datas.append(item)
        self.datas = filtered_datas
        return result

_datas = [
    {datas_str}
]

_binaries = []

_hiddenimports = [
    {hidden_str}
]

a = FilteredAnalysis([r'{LAUNCHER_PY}'],
             pathex=[r'{APP_DIR}'],
             binaries=_binaries,
             datas=_datas,
             hiddenimports=_hiddenimports,
             hookspath=[],
             hooksconfig={{}},
             runtime_hooks=[],
             excludes=['matplotlib', 'scipy', 'numpy', 'tkinter', 'tensorflow',
                       'torch', 'CustomTkinter', 'PyQt6', 'PyQt5', 'PIL', 'cv2'],
             win_no_prefer_redirects=False,
             win_private_assemblies=False,
             cipher=block_cipher,
             noarchive=False)

pyz = PYZ(a.pure, a.zipped_data,
             cipher=block_cipher)

exe = EXE(pyz,
          a.scripts,
          a.binaries,
          a.zipfiles,
          a.datas,
          [],
          name='{exe_name}',
          debug=False,
          bootloader_ignore_signals=False,
          strip=False,
          upx=False,
          upx_exclude=[],
          runtime_tmpdir=None,
          console=False,
          disable_windowed_traceback=True,
          argv_emulation=False,
          target_arch=None,
          codesign_identity=None,
          entitlements_file=None,
          icon=r'{ICON_ICO}',
          version=r'{VERSION_INFO}')
"""
    spec_path = os.path.join(pyinstaller_build_dir, f"{exe_name}.spec")
    os.makedirs(pyinstaller_build_dir, exist_ok=True)
    with open(spec_path, 'w', encoding='utf-8') as f:
        f.write(spec_content)
    return spec_path


def create_hardlink_entry(dist_exe):
    entry_exe = os.path.join(DEV_DIR, f"{BRAND_NAME}.exe")
    if os.path.isfile(entry_exe):
        try:
            os.remove(entry_exe)
        except PermissionError:
            print(f"  Warning: Cannot remove old entry EXE (may be running): {entry_exe}")
            return
    try:
        subprocess.run(
            ['mklink', '/H', entry_exe, dist_exe],
            check=True, capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        print(f"  Hardlink created: {entry_exe} -> {dist_exe}")
    except Exception:
        try:
            shutil.copy2(dist_exe, entry_exe)
            print(f"  Hardlink failed, fallback copy: {entry_exe}")
        except Exception as e:
            print(f"  Warning: Cannot create entry {entry_exe}: {e}")


def build_exe(version, skip_frontend=False):
    exe_name = f"{BRAND_NAME}-v{version}"

    version_build_dir = os.path.join(BUILD_ROOT, f"v{version}")
    pyinstaller_build_dir = os.path.join(version_build_dir, "build")
    pyinstaller_dist_dir = os.path.join(version_build_dir, "dist")

    os.makedirs(version_build_dir, exist_ok=True)

    print(f"\n{'=' * 60}")
    print(f"  Build {BRAND_NAME} v{version}")
    print(f"  EXE: {exe_name}.exe")
    print(f"  Build dir: {version_build_dir}")
    print(f"{'=' * 60}")

    if not skip_frontend:
        if not build_frontend():
            return False

    if not copy_static_files():
        return False

    generate_icon()

    original_content, original_vi = patch_version(API_MAIN_PY, version, VERSION_INFO)
    print(f"\n  Version injected: {version}")

    try:
        spec_path = generate_spec(version, exe_name, pyinstaller_build_dir)
        print(f"\n{'=' * 60}")
        print(f"  Step 3: PyInstaller --onefile")
        print(f"{'=' * 60}")

        cmd = [
            sys.executable, "-m", "PyInstaller",
            "--noconfirm",
            "--clean",
            f"--distpath={pyinstaller_dist_dir}",
            f"--workpath={pyinstaller_build_dir}",
            spec_path,
        ]

        result = subprocess.run(cmd, cwd=APP_DIR)

        if result.returncode != 0:
            print(f"\n[ERROR] PyInstaller failed! Exit code: {result.returncode}")
            return False

        src_exe = os.path.join(pyinstaller_dist_dir, f"{exe_name}.exe")
        os.makedirs(DIST_DIR, exist_ok=True)
        dist_exe = os.path.join(DIST_DIR, f"{exe_name}.exe")

        if os.path.isfile(src_exe):
            if os.path.isfile(dist_exe):
                os.remove(dist_exe)
            shutil.move(src_exe, dist_exe)
            exe_size = os.path.getsize(dist_exe) / (1024 * 1024)

            create_hardlink_entry(dist_exe)

            self_path = os.path.abspath(__file__)
            version_build_script = os.path.join(version_build_dir, "build_web.py")
            shutil.copy2(self_path, version_build_script)

            print(f"\n{'=' * 60}")
            print(f"  BUILD SUCCESS!")
            print(f"  Version:  v{version}")
            print(f"  EXE:      {dist_exe}")
            print(f"  EXE size: {exe_size:.1f} MB")
            print(f"  Entry:    {os.path.join(DEV_DIR, f'{BRAND_NAME}.exe')}")
            print(f"  Build:    {version_build_dir}")
            print(f"{'=' * 60}")
            return True
        else:
            print(f"[ERROR] EXE not found: {src_exe}")
            return False

    finally:
        restore_version(API_MAIN_PY, original_content, VERSION_INFO, original_vi)
        print(f"  Source version restored.")

        if os.path.isdir(STATIC_DIR):
            shutil.rmtree(STATIC_DIR)
            print(f"  Cleaned temp static/")


def main():
    skip_frontend = "--skip-frontend" in sys.argv

    version = get_version()
    success = build_exe(version, skip_frontend=skip_frontend)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
