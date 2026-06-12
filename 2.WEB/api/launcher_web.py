import sys
import os
import ctypes
import ctypes.wintypes
import time
import shutil
import json
import re
import subprocess as _sp

BRAND_NAME = "云集智能编程工作站"
VALID_NAMES = [BRAND_NAME]

is_frozen = getattr(sys, 'frozen', False)
is_win = sys.platform == 'win32'

if is_win and is_frozen:
    class _NullWriter:
        def write(self, *args, **kwargs):
            return 0
        def flush(self):
            pass
        def isatty(self):
            return False
    sys.stdout = _NullWriter()
    sys.stderr = _NullWriter()


def _load_project_config():
    _DEFAULTS = {
        "brand_name": BRAND_NAME,
        "paths": {"dev": "dev", "app": "app", "dist": "dist", "ver": "ver"}
    }
    if is_frozen:
        config_path = os.path.join(sys._MEIPASS, "project.json")
    else:
        _root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        config_path = os.path.join(_root, "project.json")
    if os.path.isfile(config_path):
        try:
            with open(config_path, 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception:
            pass
    return _DEFAULTS


def _verify_brand():
    if not is_win or not is_frozen:
        return
    exe_name = os.path.basename(sys.executable)
    if not any(v in exe_name for v in VALID_NAMES):
        correct_name = f"{BRAND_NAME}.exe"
        ctypes.windll.user32.MessageBoxW(
            0,
            f"可执行文件名已被修改，无法运行。\n\n当前文件名: {exe_name}\n正确文件名: {correct_name}\n\n请将文件名改回「{correct_name}」后重试。",
            "品牌校验失败",
            0x10
        )
        sys.exit(1)


_verify_brand()


def _kill_old_instances():
    if not is_win or not is_frozen:
        return

    my_pid = ctypes.windll.kernel32.GetCurrentProcessId()
    kernel32 = ctypes.windll.kernel32
    TH32CS_SNAPPROCESS = 0x00000002

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

    snap = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snap == ctypes.c_void_p(-1).value:
        return

    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
    pids_to_kill = []

    if kernel32.Process32FirstW(snap, ctypes.byref(entry)):
        while True:
            pid = entry.th32ProcessID
            en = entry.szExeFile.lower()
            if pid != my_pid and any(en.startswith(v.lower()) for v in VALID_NAMES) and en.endswith('.exe'):
                pids_to_kill.append(pid)
            entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
            if not kernel32.Process32NextW(snap, ctypes.byref(entry)):
                break

    kernel32.CloseHandle(snap)

    PROCESS_TERMINATE = 0x0001
    for pid in pids_to_kill:
        handle = kernel32.OpenProcess(PROCESS_TERMINATE, False, pid)
        if handle:
            kernel32.TerminateProcess(handle, 0)
            kernel32.CloseHandle(handle)

    if pids_to_kill:
        time.sleep(0.5)


_kill_old_instances()


_cleanup_path = ""
_new_argv = [sys.argv[0]]
for arg in sys.argv[1:]:
    if arg.startswith("--cleanup="):
        _cleanup_path = arg[len("--cleanup="):]
    else:
        _new_argv.append(arg)
sys.argv = _new_argv

if _cleanup_path and os.path.isfile(_cleanup_path):
    for _ in range(10):
        try:
            os.remove(_cleanup_path)
            break
        except PermissionError:
            time.sleep(0.5)
    if os.path.isfile(_cleanup_path):
        MOVEFILE_DELAY_UNTIL_REBOOT = 0x4
        try:
            ctypes.windll.kernel32.MoveFileExW(
                _cleanup_path, None, MOVEFILE_DELAY_UNTIL_REBOOT
            )
        except Exception:
            pass


def _find_dev_dir():
    if not is_frozen:
        return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    exe_dir = os.path.dirname(os.path.abspath(sys.executable))

    for _ in range(5):
        lock_path = os.path.join(exe_dir, ".yunji.lock")
        app_dir = os.path.join(exe_dir, "app")
        if os.path.isfile(lock_path) or os.path.isdir(app_dir):
            return exe_dir
        parent = os.path.dirname(exe_dir)
        if parent == exe_dir:
            break
        exe_dir = parent

    return None


def _self_deploy():
    if not is_frozen:
        return None

    exe_path = os.path.abspath(sys.executable)
    exe_dir = os.path.dirname(exe_path)
    exe_name = os.path.basename(exe_path)

    config = _load_project_config()
    brand = config.get("brand_name", BRAND_NAME)
    paths = config.get("paths", {})

    deploy_root = os.path.join(exe_dir, brand)
    os.makedirs(deploy_root, exist_ok=True)

    ver_dir = os.path.join(deploy_root, paths.get("ver", "ver"))
    os.makedirs(ver_dir, exist_ok=True)

    lock_path = os.path.join(deploy_root, ".yunji.lock")
    with open(lock_path, 'w', encoding='utf-8') as f:
        f.write("")

    version_suffix = ""
    m = re.search(r'-v(\d{4}\.\d{2}\.\d{2}\.\d{4})', exe_name)
    if m:
        version_suffix = m.group(1)

    ver_exe_name = f"{brand}-v{version_suffix}.exe" if version_suffix else f"{brand}.exe"
    ver_exe_path = os.path.join(ver_dir, ver_exe_name)

    if os.path.normcase(exe_path) != os.path.normcase(ver_exe_path):
        if os.path.isfile(ver_exe_path):
            try:
                os.remove(ver_exe_path)
            except PermissionError:
                pass
        shutil.copy2(exe_path, ver_exe_path)

    entry_exe = os.path.join(deploy_root, f"{brand}.exe")
    if not os.path.isfile(entry_exe):
        try:
            _sp.run(
                ['mklink', '/H', entry_exe, ver_exe_path],
                check=True, capture_output=True,
                creationflags=0x08000000,
            )
        except Exception:
            try:
                shutil.copy2(ver_exe_path, entry_exe)
            except Exception:
                pass

    if os.path.isfile(entry_exe):
        _sp.Popen(
            [entry_exe, f"--cleanup={exe_path}"],
            cwd=deploy_root,
            creationflags=0x08000000,
        )
        sys.exit(0)
    else:
        return deploy_root


def _setup_env():
    if not is_frozen:
        _script_dir = os.path.dirname(os.path.abspath(__file__))
        _dev_dir = os.path.dirname(_script_dir)
        os.environ["YUNJI_DEV_DIR"] = _dev_dir
        return

    dev_dir = _find_dev_dir()
    if dev_dir is None:
        dev_dir = _self_deploy()
    if dev_dir is None:
        dev_dir = os.path.dirname(os.path.abspath(sys.executable))

    os.environ["YUNJI_DEV_DIR"] = dev_dir

    _static_dir = os.path.join(dev_dir, "static")
    if not os.path.isdir(_static_dir):
        _static_dir = os.path.join(sys._MEIPASS, "static")
    if os.path.isdir(_static_dir):
        os.environ["YUNJI_STATIC_DIR"] = _static_dir


_setup_env()

try:
    import api_main
    api_main.main()
except Exception as _e:
    import traceback
    _tb = traceback.format_exc()
    try:
        _exe_dir = os.path.dirname(os.path.abspath(sys.executable)) if is_frozen else os.path.dirname(os.path.abspath(__file__))
        with open(os.path.join(_exe_dir, "crash.log"), "w", encoding="utf-8") as _lf:
            _lf.write(_tb)
    except Exception:
        pass
    if is_win:
        ctypes.windll.user32.MessageBoxW(
            0,
            f"程序启动失败:\n\n{_tb[:2000]}\n\n错误日志已保存到: {_exe_dir}\\crash.log",
            "启动错误",
            0x10
        )
    sys.exit(1)