import os
import sys
import re
import shutil
import subprocess
import zipfile
from datetime import datetime

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEV_DIR = os.path.join(PROJECT_ROOT, "dev")
APP_DIR = os.path.join(DEV_DIR, "app")
MAIN_PY = os.path.join(APP_DIR, "main.py")
LAUNCHER_PY = os.path.join(APP_DIR, "launcher.py")
ICO_PNG = os.path.join(APP_DIR, "icon.png")
ICON_ICO = os.path.join(APP_DIR, "icon.ico")
ICON_PNG = os.path.join(APP_DIR, "icon.png")
VERSION_INFO = os.path.join(APP_DIR, "version_info.txt")
BUILD_ROOT = os.path.join(PROJECT_ROOT, "build")

VENV_DIR = os.path.join(DEV_DIR, "data", ".venv")
QT6_DIR = os.path.join(VENV_DIR, "Lib", "site-packages", "PyQt6", "Qt6")

WEBENGINE_FILES = [
    "bin/Qt6WebEngineCore.dll",
    "bin/Qt6WebEngineQuick.dll",
    "bin/Qt6WebEngineQuickDelegatesQml.dll",
    "bin/Qt6WebEngineWidgets.dll",
    "bin/QtWebEngineProcess.exe",
    "resources/icudtl.dat",
    "resources/qtwebengine_devtools_resources.debug.pak",
    "resources/qtwebengine_devtools_resources.pak",
    "resources/qtwebengine_resources.debug.pak",
    "resources/qtwebengine_resources.pak",
    "resources/qtwebengine_resources_100p.debug.pak",
    "resources/qtwebengine_resources_100p.pak",
    "resources/qtwebengine_resources_200p.debug.pak",
    "resources/qtwebengine_resources_200p.pak",
    "resources/v8_context_snapshot.bin",
    "resources/v8_context_snapshot.debug.bin",
    "translations/qtwebengine_locales",
    "translations/qtwebengine_ca.qm",
    "translations/qtwebengine_de.qm",
    "translations/qtwebengine_en.qm",
    "translations/qtwebengine_es.qm",
    "translations/qtwebengine_ka.qm",
    "translations/qtwebengine_ko.qm",
    "translations/qtwebengine_lg.qm",
    "translations/qtwebengine_pl.qm",
    "translations/qtwebengine_pt_BR.qm",
    "translations/qtwebengine_ru.qm",
    "translations/qtwebengine_sv.qm",
    "translations/qtwebengine_tr.qm",
    "translations/qtwebengine_uk.qm",
    "translations/qtwebengine_zh_CN.qm",
]

EXCLUDED_WE_PATTERNS = [
    'Qt6WebEngine',
    'QtWebEngineProcess',
    'icudtl.dat',
    'qtwebengine_',
    'v8_context_snapshot',
    'Qt6Qml',
    'Qt6Quick',
    'Qt6QmlModels',
    'Qt6QmlWorkerScript',
    'Qt6QuickWidgets',
    'Qt6QuickTest',
    'Qt63D',
    'Qt6Multimedia',
    'Qt6MultimediaWidgets',
    'Qt6SerialPort',
    'Qt6Sql',
    'Qt6Xml',
    'Qt6Test',
    'Qt6Svg',
    'Qt6SvgWidgets',
    'Qt6OpenGL',
    'Qt6OpenGLWidgets',
    'Qt6Nfc',
    'Qt6Bluetooth',
    'Qt6Positioning',
    'Qt6Location',
    'Qt6Sensors',
    'Qt6WebChannel',
    'Qt6WebView',
    'Qt6Pdf',
    'Qt6PdfWidgets',
    'Qt6TextToSpeech',
    'Qt6DataVisualization',
    'Qt6Charts',
    'Qt6NetworkAuth',
    'Qt6Help',
    'Qt6Designer',
    'Qt6UiTools',
    'Qt6AxContainer',
    'Qt6AxServer',
    'Qt6DBus',
    'Qt6PrintSupport',
    'Qt6Linguist',
    'qml',
    'plugins/sceneparsers',
    'plugins/renderers',
    'plugins/sqldrivers',
    'plugins/multimedia',
    'plugins/webview',
    'plugins/tls',
    'plugins/styles',
    'plugins/scxmldatamodel',
    'plugins/platforminputcontexts',
    'plugins/geometryloaders',
    'avcodec',
    'avformat',
    'avutil',
    'swresample',
    'swscale',
    'opengl32sw',
]


def get_version():
    return datetime.now().strftime("%Y.%m.%d.%H%M")


def patch_version(main_py_path, version, version_info_path):
    with open(main_py_path, 'r', encoding='utf-8') as f:
        content = f.read()
    original = content
    content = re.sub(
        r'VERSION = get_version_from_filename\(\)',
        f'VERSION = "{version}"',
        content
    )
    with open(main_py_path, 'w', encoding='utf-8') as f:
        f.write(content)

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


def restore_version(main_py_path, original_content, version_info_path, original_vi):
    with open(main_py_path, 'w', encoding='utf-8') as f:
        f.write(original_content)
    with open(version_info_path, 'w', encoding='utf-8') as f:
        f.write(original_vi)


def pack_webengine(version, version_build_dir):
    if not os.path.isdir(QT6_DIR):
        print(f"警告: Qt6目录不存在，跳过WebEngine打包: {QT6_DIR}")
        return None

    zip_name = f"WebEngine-v{version}.zip"
    zip_path = os.path.join(version_build_dir, zip_name)

    print(f"打包WebEngine运行时...")
    total_size = 0
    file_count = 0

    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED, compresslevel=1) as zf:
        for rel_path in WEBENGINE_FILES:
            full_path = os.path.join(QT6_DIR, rel_path.replace('/', os.sep))
            if os.path.isfile(full_path):
                arc_name = os.path.join("PyQt6", "Qt6", rel_path).replace(os.sep, '/')
                zf.write(full_path, arc_name)
                total_size += os.path.getsize(full_path)
                file_count += 1
            elif os.path.isdir(full_path):
                for root, dirs, files in os.walk(full_path):
                    for fname in files:
                        fpath = os.path.join(root, fname)
                        arel = os.path.relpath(fpath, QT6_DIR).replace(os.sep, '/')
                        arc_name = os.path.join("PyQt6", "Qt6", arel).replace(os.sep, '/')
                        zf.write(fpath, arc_name)
                        total_size += os.path.getsize(fpath)
                        file_count += 1

    zip_size = os.path.getsize(zip_path) / (1024 * 1024)
    print(f"WebEngine打包完成: {file_count}个文件, 原始{total_size/(1024*1024):.0f}MB → 压缩后{zip_size:.0f}MB")
    print(f"输出: {zip_path}")

    dist_dir = os.path.join(DEV_DIR, "dist")
    os.makedirs(dist_dir, exist_ok=True)
    dist_zip = os.path.join(dist_dir, zip_name)
    shutil.copy2(zip_path, dist_zip)
    print(f"已复制到: {dist_zip}")

    return zip_path


def generate_spec(version, exe_name, pyinstaller_build_dir, pyinstaller_dist_dir):
    datas_entries = []
    if os.path.isfile(ICON_ICO):
        datas_entries.append(f"(r'{ICON_ICO}', '.')")
    if os.path.isfile(ICON_PNG):
        datas_entries.append(f"(r'{ICON_PNG}', '.')")

    datas_str = ",\n    ".join(datas_entries) if datas_entries else ""

    excluded_binaries_str = ", ".join(f"'{p}'" for p in EXCLUDED_WE_PATTERNS)

    spec_content = f"""# -*- mode: python ; coding: utf-8 -*-
import sys
import os
from PyInstaller.building.build_main import Analysis as _OrigAnalysis

block_cipher = None

_excluded_patterns = [{excluded_binaries_str}]

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
    'PyQt6', 'PyQt6.QtCore', 'PyQt6.QtGui', 'PyQt6.QtWidgets',
    'PyQt6.QtWebEngineWidgets', 'PyQt6.QtWebEngineCore', 'PyQt6.QtWebChannel',
    'backend', 'http.server', 'socketserver',
    'urllib.request', 'urllib.error', 'urllib.parse',
    'psutil', 'uuid',
]

a = FilteredAnalysis([r'{LAUNCHER_PY}'],
             pathex=[r'{APP_DIR}'],
             binaries=_binaries,
             datas=_datas,
             hiddenimports=_hiddenimports,
             hookspath=[],
             hooksconfig={{}},
             runtime_hooks=[],
             excludes=['matplotlib', 'scipy', 'numpy', 'tkinter', 'tensorflow', 'torch'],
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


def build():
    version = get_version()
    exe_name = f"云集智能编程工作站-v{version}"

    version_build_dir = os.path.join(BUILD_ROOT, f"v{version}")
    pyinstaller_build_dir = os.path.join(version_build_dir, "build")
    pyinstaller_dist_dir = os.path.join(version_build_dir, "dist")

    os.makedirs(version_build_dir, exist_ok=True)

    print(f"=== 构建云集智能编程工作站 v{version} ===")
    print(f"EXE名称: {exe_name}.exe")
    print(f"构建目录: {version_build_dir}")

    if os.path.isfile(ICO_PNG):
        try:
            from PIL import Image
        except ImportError:
            print("Pillow 未安装，正在安装...")
            subprocess.run([sys.executable, "-m", "pip", "install", "Pillow", "-q"], check=True)
            from PIL import Image
        try:
            img = Image.open(ICO_PNG)
            ico_sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]
            img.save(ICON_ICO, format='ICO', sizes=ico_sizes)
            print(f"已生成: icon.ico (16/32/48/256)")
            img_64 = img.resize((64, 64), Image.LANCZOS)
            img_64.save(ICON_PNG, 'PNG')
            print(f"已生成: icon.png (64x64)")
        except Exception as e:
            print(f"警告: 从icon.png生成图标失败: {e}")
    elif os.path.isfile(ICON_ICO):
        print(f"使用现有图标: {ICON_ICO}")
    else:
        print(f"警告: 未找到图标文件")

    original_content, original_vi = patch_version(MAIN_PY, version, VERSION_INFO)
    print(f"已注入版本号: {version}")

    try:
        spec_path = generate_spec(version, exe_name, pyinstaller_build_dir, pyinstaller_dist_dir)

        cmd = [
            sys.executable, "-m", "PyInstaller",
            "--noconfirm",
            "--clean",
            f"--distpath={pyinstaller_dist_dir}",
            f"--workpath={pyinstaller_build_dir}",
            spec_path,
        ]

        print(f"运行 PyInstaller (spec文件, 排除WebEngine二进制)...")
        result = subprocess.run(cmd, cwd=APP_DIR)

        if result.returncode != 0:
            print(f"构建失败! 返回码: {result.returncode}")
            return False

        src_exe = os.path.join(pyinstaller_dist_dir, f"{exe_name}.exe")
        dist_dir = os.path.join(DEV_DIR, "dist")
        os.makedirs(dist_dir, exist_ok=True)
        dist_exe = os.path.join(dist_dir, f"{exe_name}.exe")

        if os.path.isfile(src_exe):
            if os.path.isfile(dist_exe):
                os.remove(dist_exe)
            shutil.move(src_exe, dist_exe)
            exe_size = os.path.getsize(dist_exe) / (1024 * 1024)
            print(f"EXE已移动到: {dist_exe}")
            print(f"EXE大小: {exe_size:.1f} MB")
        else:
            print(f"未找到构建产物: {src_exe}")
            return False

        webengine_zip = pack_webengine(version, version_build_dir)

        entry_exe = os.path.join(DEV_DIR, "云集智能编程工作站.exe")
        if os.path.isfile(entry_exe):
            try:
                os.remove(entry_exe)
            except PermissionError:
                print(f"警告: 无法删除旧入口EXE（可能正在运行）: {entry_exe}")
        try:
            subprocess.run(
                ['mklink', '/H', entry_exe, dist_exe],
                check=True, capture_output=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            print(f"硬链接已创建: {entry_exe} → {dist_exe}")
        except Exception:
            shutil.copy2(dist_exe, entry_exe)
            print(f"硬链接失败，回退复制: {entry_exe}")

        self_path = os.path.abspath(__file__)
        version_build_script = os.path.join(version_build_dir, "build.py")
        shutil.copy2(self_path, version_build_script)

        print(f"\n{'='*60}")
        print(f"  构建成功!")
        print(f"  版本: v{version}")
        print(f"  EXE: {dist_exe} ({exe_size:.1f} MB)")
        if webengine_zip:
            print(f"  WebEngine: {webengine_zip}")
        print(f"  入口: {entry_exe}")
        print(f"  构建记录: {version_build_dir}")
        print(f"{'='*60}")

        return True

    finally:
        restore_version(MAIN_PY, original_content, VERSION_INFO, original_vi)
        print(f"已恢复源代码版本号")


if __name__ == "__main__":
    success = build()
    sys.exit(0 if success else 1)
