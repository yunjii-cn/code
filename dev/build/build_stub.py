import os
import sys
import shutil
import subprocess

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEV_DIR = os.path.join(PROJECT_ROOT, "dev")
APP_DIR = os.path.join(DEV_DIR, "app")
STUB_PY = os.path.join(APP_DIR, "launcher_stub.py")
ICO_PNG = os.path.join(APP_DIR, "ico.png")
ICON_ICO = os.path.join(APP_DIR, "icon.ico")
ICON_PNG = os.path.join(APP_DIR, "icon.png")
BUILD_ROOT = os.path.join(PROJECT_ROOT, "build")
STUB_BUILD_DIR = os.path.join(BUILD_ROOT, "stub")

BRAND_NAME = "云集智能编程工作站"

VENV_DIR = os.path.join(DEV_DIR, "data", ".venv")
VENV_PYTHON = os.path.join(VENV_DIR, "Scripts", "python.exe")
PYTHON_DIR = os.path.dirname(VENV_PYTHON)
TCL_DIR = os.path.join(PYTHON_DIR, "tcl")
DLLS_DIR = os.path.join(PYTHON_DIR, "DLLs")
LIB_DIR = os.path.join(VENV_DIR, "Lib")

if not os.path.isdir(TCL_DIR):
    for candidate in [
        r"C:\Program Files\Python312\tcl",
        r"C:\Python312\tcl",
        os.path.join(os.path.dirname(PYTHON_DIR), "tcl"),
        os.path.join(os.path.dirname(VENV_DIR), "tcl"),
    ]:
        if os.path.isdir(candidate):
            TCL_DIR = candidate
            DLLS_DIR = os.path.join(os.path.dirname(candidate), "DLLs")
            LIB_DIR = os.path.join(os.path.dirname(candidate), "Lib")
            break


def build_stub():
    print(f"=== 构建{BRAND_NAME} 启动器 ===")

    if not os.path.isfile(STUB_PY):
        print(f"错误: launcher_stub.py 不存在: {STUB_PY}")
        return False

    os.makedirs(STUB_BUILD_DIR, exist_ok=True)

    icon_64_path = os.path.join(STUB_BUILD_DIR, "icon_64.png")
    source_icon = ICO_PNG if os.path.isfile(ICO_PNG) else (ICON_PNG if os.path.isfile(ICON_PNG) else ICON_ICO)
    if os.path.isfile(source_icon):
        try:
            from PIL import Image
        except ImportError:
            print("Pillow 未安装，正在安装...")
            subprocess.run([VENV_PYTHON, "-m", "pip", "install", "Pillow", "-q"], check=True)
            from PIL import Image
        try:
            img = Image.open(source_icon)
            ico_sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]
            img.save(ICON_ICO, format='ICO', sizes=ico_sizes)
            print(f"已生成: icon.ico (16/32/48/256, from {os.path.basename(source_icon)})")
            img_64 = img.resize((64, 64), Image.LANCZOS)
            img_64.save(ICON_PNG, 'PNG')
            img_64.save(icon_64_path, 'PNG')
            print(f"已生成: icon.png + icon_64.png (64x64, Lanczos)")
        except Exception as e:
            print(f"警告: 生成图标失败: {e}")
            icon_64_path = None
    else:
        icon_64_path = None

    cmd = [
        VENV_PYTHON, "-m", "PyInstaller",
        "--noconfirm",
        "--clean",
        f"--name={BRAND_NAME}",
        f"--icon={ICON_ICO}",
        f"--distpath={STUB_BUILD_DIR}/dist",
        f"--workpath={STUB_BUILD_DIR}/work",
        f"--specpath={STUB_BUILD_DIR}",
        "--onefile",
        "--windowed",
        "--exclude-module", "PyQt6",
        "--exclude-module", "PyQt6.QtCore",
        "--exclude-module", "PyQt6.QtGui",
        "--exclude-module", "PyQt6.QtWidgets",
        "--exclude-module", "PyQt6.QtWebEngineWidgets",
        "--exclude-module", "PyQt6.QtWebEngineCore",
        "--exclude-module", "PyQt6.QtWebChannel",
        "--exclude-module", "psutil",
        "--exclude-module", "numpy",
        "--exclude-module", "PIL",
        "--exclude-module", "fastapi",
        "--exclude-module", "uvicorn",
        "--exclude-module", "pywebview",
        "--exclude-module", "pythonnet",
        "--exclude-module", "clr_loader",
        "--exclude-module", "pydantic",
        "--exclude-module", "pydantic_core",
        "--exclude-module", "httpx",
        "--exclude-module", "httpcore",
        "--exclude-module", "sse_starlette",
        "--exclude-module", "starlette",
        "--exclude-module", "anyio",
        "--exclude-module", "sniffio",
        "--exclude-module", "watchfiles",
        "--exclude-module", "httptools",
        "--exclude-module", "orjson",
        "--exclude-module", "websockets",
    ]

    tkinter_dir = os.path.join(LIB_DIR, "tkinter")
    if os.path.isdir(tkinter_dir):
        cmd.extend(["--add-data", f"{tkinter_dir};tkinter"])
        print(f"已包含: tkinter 包 ({tkinter_dir})")
    else:
        print(f"警告: 未找到 tkinter 包 {tkinter_dir}")

    tcl86_dir = os.path.join(TCL_DIR, "tcl8.6")
    tk86_dir = os.path.join(TCL_DIR, "tk8.6")
    if os.path.isdir(tcl86_dir):
        cmd.extend(["--add-data", f"{tcl86_dir};tcl/tcl8.6"])
        print(f"已包含: tcl8.6 数据 ({tcl86_dir})")
    if os.path.isdir(tk86_dir):
        cmd.extend(["--add-data", f"{tk86_dir};tcl/tk8.6"])
        print(f"已包含: tk8.6 数据 ({tk86_dir})")

    tcl_dll = os.path.join(DLLS_DIR, "tcl86t.dll")
    tk_dll = os.path.join(DLLS_DIR, "tk86t.dll")
    tkinter_pyd = os.path.join(DLLS_DIR, "_tkinter.pyd")
    if os.path.isfile(tcl_dll):
        cmd.extend(["--add-binary", f"{tcl_dll};."])
        print(f"已包含: tcl86t.dll")
    if os.path.isfile(tk_dll):
        cmd.extend(["--add-binary", f"{tk_dll};."])
        print(f"已包含: tk86t.dll")
    if os.path.isfile(tkinter_pyd):
        cmd.extend(["--add-binary", f"{tkinter_pyd};."])
        print(f"已包含: _tkinter.pyd")

    if os.path.isfile(ICON_ICO):
        cmd.extend(["--add-data", f"{ICON_ICO};."])
        print(f"已包含: icon.ico")
    if icon_64_path and os.path.isfile(icon_64_path):
        cmd.extend(["--add-data", f"{icon_64_path};."])
        print(f"已包含: icon_64.png (预缩放64x64)")
    elif os.path.isfile(ICON_PNG):
        cmd.extend(["--add-data", f"{ICON_PNG};."])
        print(f"已包含: icon.png (原始尺寸)")

    cmd.append(STUB_PY)

    print(f"运行 PyInstaller...")

    result = subprocess.run(cmd, cwd=APP_DIR)

    if result.returncode != 0:
        print(f"构建失败! 返回码: {result.returncode}")
        return False

    src_exe = os.path.join(STUB_BUILD_DIR, "dist", f"{BRAND_NAME}.exe")
    if not os.path.isfile(src_exe):
        print(f"未找到构建产物: {src_exe}")
        return False

    dest_exe = os.path.join(PROJECT_ROOT, f"{BRAND_NAME}.exe")
    if os.path.isfile(dest_exe):
        try:
            os.remove(dest_exe)
        except PermissionError:
            print(f"警告: 无法删除旧启动器（可能正在运行）: {dest_exe}")
            print(f"请手动删除后复制: copy \"{src_exe}\" \"{dest_exe}\"")

    shutil.copy2(src_exe, dest_exe)
    size_mb = os.path.getsize(dest_exe) / 1024 / 1024

    print(f"\n=== 启动器构建成功! ===")
    print(f"输出: {dest_exe}")
    print(f"大小: {size_mb:.1f} MB")
    print(f"此文件为永久启动器，无需随版本更新")

    return True


if __name__ == "__main__":
    success = build_stub()
    sys.exit(0 if success else 1)
