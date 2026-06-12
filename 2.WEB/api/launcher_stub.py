import sys
import os
import json
import urllib.request
import ssl
import subprocess
import time
import shutil
import tempfile
import threading
import ctypes
from datetime import datetime

try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)
except Exception:
    try:
        ctypes.windll.user32.SetProcessDPIAware()
    except Exception:
        pass

APP_NAME = "云集智能编程工作站"
EXE_PREFIXES = [APP_NAME]
DEPLOY_DIR_NAME = APP_NAME
GITEE_API = "https://gitee.com/api/v5/repos/yunjii/code/releases/latest"
GITHUB_API = "https://api.github.com/repos/yunjii-cn/code/releases/latest"
LOCK_FILE = ".yunji.lock"
SETTINGS_FILE = "launcher_settings.json"

MIRROR_PREFIXES = [
    "https://gh-proxy.com/",
    "https://ghproxy.net/",
    "https://ghproxy.homeboycn.cn/",
]

COLOR_BG = "#0D0D0D"
COLOR_TEXT = "#E0E0E0"
COLOR_DIM = "#888888"
COLOR_RED = "#C62828"
COLOR_RED_LIGHT = "#EF5350"
COLOR_BORDER = "#2D2D2D"

_LOG_FILE = os.path.join(tempfile.gettempdir(), "yunji_code_launcher.log")
_LOG_LOCK = threading.Lock()


def _log(msg):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] {msg}"
    with _LOG_LOCK:
        try:
            with open(_LOG_FILE, "a", encoding="utf-8") as f:
                f.write(line + "\n")
        except Exception:
            pass


def _fix_tcl_tk_paths():
    if hasattr(sys, '_MEIPASS'):
        tcl_dir = os.path.join(sys._MEIPASS, 'tcl', 'tcl8.6')
        tk_dir = os.path.join(sys._MEIPASS, 'tcl', 'tk8.6')
    else:
        base = os.path.dirname(sys.executable)
        tcl_dir = os.path.join(base, 'tcl', 'tcl8.6')
        tk_dir = os.path.join(base, 'tcl', 'tk8.6')
        if not os.path.isdir(tcl_dir):
            tcl_dir = os.path.join(base, '..', 'tcl', 'tcl8.6')
            tk_dir = os.path.join(base, '..', 'tcl', 'tk8.6')
    if os.path.isdir(tcl_dir):
        os.environ['TCL_LIBRARY'] = tcl_dir
    if os.path.isdir(tk_dir):
        os.environ['TK_LIBRARY'] = tk_dir


def _find_deploy_dir():
    exe_dir = os.path.dirname(os.path.abspath(sys.argv[0]))

    if os.path.isfile(os.path.join(exe_dir, LOCK_FILE)):
        return exe_dir

    candidate = os.path.join(exe_dir, DEPLOY_DIR_NAME)
    if os.path.isdir(candidate):
        if os.path.isfile(os.path.join(candidate, LOCK_FILE)):
            return candidate
        ver_dir = os.path.join(candidate, "ver")
        if os.path.isdir(ver_dir) and any(f.endswith(".exe") for f in os.listdir(ver_dir)):
            return candidate

    return candidate


def _is_deployed(deploy_dir):
    if not os.path.isdir(deploy_dir):
        return False
    ver_dir = os.path.join(deploy_dir, "ver")
    if os.path.isdir(ver_dir):
        for f in os.listdir(ver_dir):
            if any(f.startswith(p) for p in EXE_PREFIXES) and f.endswith(".exe") and "-v" in f:
                return True
    for f in os.listdir(deploy_dir):
        if any(f.startswith(p) for p in EXE_PREFIXES) and f.endswith(".exe") and "-v" in f:
            return True
    return False


def _ensure_structure(deploy_dir):
    for sub in [
        "ver", "app",
        "data/public/models", "data/public/templates", "data/public/plugins",
        "data/public/api/qwen2api", "data/public/api/zhipu2api",
        "data/users/default/projects", "data/users/default/sessions", "data/users/default/webdata",
        "temp/__pycache__", "temp/logs", "temp/cache", "temp/debug", "temp/tmp",
    ]:
        os.makedirs(os.path.join(deploy_dir, sub.replace("/", os.sep)), exist_ok=True)
    lock_path = os.path.join(deploy_dir, LOCK_FILE)
    if not os.path.isfile(lock_path):
        with open(lock_path, "w") as f:
            f.write("yunji")


def _load_settings(deploy_dir):
    path = os.path.join(deploy_dir, "app", SETTINGS_FILE)
    if os.path.isfile(path):
        try:
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    path2 = os.path.join(deploy_dir, SETTINGS_FILE)
    if os.path.isfile(path2):
        try:
            with open(path2, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    return {}


def _save_settings(deploy_dir, settings):
    path = os.path.join(deploy_dir, "app", SETTINGS_FILE)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(settings, f, ensure_ascii=False, indent=2)


def _find_current_version_exe(deploy_dir):
    exes = []
    for f in os.listdir(deploy_dir):
        if any(f.startswith(p) for p in EXE_PREFIXES) and f.endswith(".exe") and "-v" in f:
            exes.append(f)
    if exes:
        exes.sort(reverse=True)
        return os.path.join(deploy_dir, exes[0])

    ver_dir = os.path.join(deploy_dir, "ver")
    if os.path.isdir(ver_dir):
        for f in os.listdir(ver_dir):
            if any(f.startswith(p) for p in EXE_PREFIXES) and f.endswith(".exe") and "-v" in f:
                exes.append(f)
        if exes:
            exes.sort(reverse=True)
            return os.path.join(ver_dir, exes[0])

    settings = _load_settings(deploy_dir)
    current_ver = settings.get("current_app_version", "")
    if current_ver:
        for prefix in EXE_PREFIXES:
            exe_name = f"{prefix}-v{current_ver}.exe"
            for d in [deploy_dir, os.path.join(deploy_dir, "ver")]:
                p = os.path.join(d, exe_name)
                if os.path.isfile(p):
                    return p
    return None


def _build_opener():
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return urllib.request.build_opener(urllib.request.HTTPSHandler(context=ctx))


def _fetch_json(url, timeout=20):
    opener = _build_opener()
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with opener.open(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _fetch_latest_release():
    try:
        data = _fetch_json(GITEE_API, timeout=20)
        if data and "tag_name" in data:
            return data, "gitee"
    except Exception:
        pass
    for prefix in MIRROR_PREFIXES:
        try:
            data = _fetch_json(prefix + GITHUB_API, timeout=20)
            if data and "tag_name" in data:
                return data, "github"
        except Exception:
            continue
    try:
        data = _fetch_json(GITHUB_API, timeout=30)
        if data and "tag_name" in data:
            return data, "github"
    except Exception:
        pass
    return None, None


def _download_file(url, dest_path, progress_cb=None):
    opener = _build_opener()
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with opener.open(req, timeout=300) as resp:
        total = int(resp.headers.get("Content-Length", 0))
        downloaded = 0
        with open(dest_path, "wb") as f:
            while True:
                chunk = resp.read(65536)
                if not chunk:
                    break
                f.write(chunk)
                downloaded += len(chunk)
                if total > 0 and progress_cb:
                    progress_cb(downloaded, total)


def _download_with_mirrors(download_url, dest_path, source="github", progress_cb=None):
    if source == "github":
        for prefix in MIRROR_PREFIXES:
            tmp_path = dest_path + ".tmp"
            try:
                _download_file(prefix + download_url, tmp_path, progress_cb)
                if os.path.isfile(tmp_path) and os.path.getsize(tmp_path) > 100000:
                    if os.path.isfile(dest_path):
                        os.remove(dest_path)
                    os.rename(tmp_path, dest_path)
                    return True
            except Exception:
                if os.path.isfile(tmp_path):
                    os.remove(tmp_path)
                continue
    tmp_path = dest_path + ".tmp"
    try:
        _download_file(download_url, tmp_path, progress_cb)
        if os.path.isfile(tmp_path) and os.path.getsize(tmp_path) > 100000:
            if os.path.isfile(dest_path):
                os.remove(dest_path)
            os.rename(tmp_path, dest_path)
            return True
    except Exception:
        if os.path.isfile(tmp_path):
            os.remove(tmp_path)
    return False


def _find_icon_path():
    if hasattr(sys, '_MEIPASS'):
        for name in ['icon.png', 'icon.ico']:
            p = os.path.join(sys._MEIPASS, name)
            if os.path.isfile(p):
                return p
    base = os.path.dirname(sys.executable) if getattr(sys, 'frozen', False) else os.path.dirname(os.path.abspath(__file__))
    for name in ['icon.png', 'icon.ico']:
        for d in [base, os.path.join(base, 'app')]:
            p = os.path.join(d, name)
            if os.path.isfile(p):
                return p
    return None


class SplashWindow:
    def __init__(self):
        self._root = None
        self._canvas = None
        self._w = 480
        self._h = 280
        self._progress = 0.0
        self._message = "正在初始化..."
        self._bar_item = None
        self._msg_item = None
        self._pct_item = None
        self._ok = False
        self._create()

    def _create(self):
        try:
            _fix_tcl_tk_paths()
            import tkinter as tk
            self._root = tk.Tk()
            self._root.title(APP_NAME)
            self._root.overrideredirect(True)
            self._root.attributes("-topmost", True)
            self._root.configure(bg=COLOR_BG)
            self._root.resizable(False, False)

            icon_path = _find_icon_path()
            if icon_path:
                try:
                    self._root.iconbitmap(icon_path)
                except Exception:
                    pass

            sw = self._root.winfo_screenwidth()
            sh = self._root.winfo_screenheight()
            x = (sw - self._w) // 2
            y = (sh - self._h) // 2
            self._root.geometry(f"{self._w}x{self._h}+{x}+{y}")

            self._canvas = tk.Canvas(
                self._root, width=self._w, height=self._h,
                bg=COLOR_BG, highlightthickness=0
            )
            self._canvas.pack()

            icon_photo = None
            if icon_path and icon_path.endswith('.png'):
                try:
                    from tkinter import PhotoImage
                    icon_photo = PhotoImage(file=icon_path)
                    iw = icon_photo.width()
                    ih = icon_photo.height()
                    if iw >= 128:
                        ratio = iw // 64
                        icon_display = icon_photo.subsample(ratio, ratio)
                    elif iw > 64:
                        zoom_r = 1
                        while iw * zoom_r < 128:
                            zoom_r += 1
                        icon_display = icon_photo.zoom(zoom_r, zoom_r)
                        sub_r = max(1, icon_display.width() // 64)
                        icon_display = icon_photo.subsample(sub_r, sub_r)
                    else:
                        zoom_r = max(1, 64 // iw)
                        icon_display = icon_photo.zoom(zoom_r, zoom_r)
                    self._canvas.create_image(self._w // 2, 50, image=icon_display)
                    self._icon_ref = icon_display
                except Exception:
                    self._icon_ref = None

            title_y = 120 if icon_photo else 45
            self._canvas.create_text(
                self._w // 2, title_y, text=APP_NAME, fill=COLOR_TEXT,
                font=("Microsoft YaHei", 18, "bold")
            )

            msg_y = title_y + 40
            self._msg_item = self._canvas.create_text(
                self._w // 2, msg_y, text=self._message, fill=COLOR_DIM,
                font=("Microsoft YaHei", 9)
            )

            bar_x, bar_y, bar_w, bar_h = 80, 190, self._w - 160, 6
            self._canvas.create_rectangle(
                bar_x, bar_y, bar_x + bar_w, bar_y + bar_h,
                fill=COLOR_BORDER, outline=""
            )
            self._bar_item = self._canvas.create_rectangle(
                bar_x, bar_y, bar_x, bar_y + bar_h,
                fill=COLOR_RED, outline=""
            )
            self._bar_x = bar_x
            self._bar_y = bar_y
            self._bar_w = bar_w
            self._bar_h = bar_h

            self._pct_item = self._canvas.create_text(
                self._w // 2, 220, text="0%", fill=COLOR_DIM,
                font=("Microsoft YaHei", 8)
            )

            self._root.update()
            self._ok = True
        except Exception as e:
            _log(f"SplashWindow create failed: {e}")
            self._root = None
            self._ok = False

    @property
    def ok(self):
        return self._ok

    def update(self, message="", progress=None):
        if not self._root or not self._ok:
            return
        try:
            if message:
                self._message = message
                self._canvas.itemconfig(self._msg_item, text=message)
            if progress is not None:
                self._progress = progress
                fill_w = int(self._bar_w * min(progress, 1.0))
                self._canvas.coords(
                    self._bar_item,
                    self._bar_x, self._bar_y,
                    self._bar_x + fill_w, self._bar_y + self._bar_h
                )
                self._canvas.itemconfig(self._pct_item, text=f"{int(progress * 100)}%")
            self._root.update()
        except Exception:
            pass

    def close(self):
        if self._root:
            try:
                self._root.destroy()
            except Exception:
                pass
            self._root = None


def _console_print(msg):
    sys.stdout.write(f"\r{msg}" + " " * 20)
    sys.stdout.flush()


def _create_shortcut(deploy_dir, main_exe_path):
    try:
        import ctypes
        from ctypes import wintypes

        CSIDL_DESKTOP = 0
        buf = ctypes.create_unicode_buffer(wintypes.MAX_PATH)
        ctypes.windll.shell32.SHGetFolderPathW(0, CSIDL_DESKTOP, 0, 0, buf)
        desktop = buf.value

        shortcut_path = os.path.join(desktop, f"{APP_NAME}.lnk")
        if os.path.isfile(shortcut_path):
            return

        powershell_script = f'''
$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut("{shortcut_path}")
$sc.TargetPath = "{main_exe_path}"
$sc.WorkingDirectory = "{deploy_dir}"
$sc.Description = "{APP_NAME}"
'''
        icon_path = os.path.join(deploy_dir, "app", "icon.ico")
        if not os.path.isfile(icon_path):
            icon_path = os.path.join(deploy_dir, "icon.ico")
        if os.path.isfile(icon_path):
            powershell_script += f'$sc.IconLocation = "{icon_path}"\n'
        powershell_script += '$sc.Save()\n'

        subprocess.run(
            ["powershell", "-NoProfile", "-NonInteractive", "-Command", powershell_script],
            capture_output=True, timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        if os.path.isfile(shortcut_path):
            _log(f"created desktop shortcut: {shortcut_path}")
    except Exception as e:
        _log(f"shortcut creation failed (non-critical): {e}")


def _launch_app(exe_path, self_exe=None):
    args = [exe_path]
    if self_exe and os.path.isfile(self_exe):
        args.append(f"--cleanup={self_exe}")
    _log(f"launching: {args}")
    subprocess.Popen(
        args,
        cwd=os.path.dirname(os.path.dirname(exe_path)),
    )


def main():
    _log("=" * 60)
    _log(f"launcher_stub: starting")
    _log(f"sys.executable={sys.executable}")
    _log(f"sys.frozen={getattr(sys, 'frozen', False)}")
    _log(f"sys._MEIPASS={getattr(sys, '_MEIPASS', 'N/A')}")

    deploy_dir = _find_deploy_dir()
    _log(f"deploy_dir={deploy_dir}")

    is_frozen = getattr(sys, 'frozen', False)
    self_exe = sys.executable if is_frozen else None

    if _is_deployed(deploy_dir):
        exe_path = _find_current_version_exe(deploy_dir)
        if exe_path:
            _log(f"already deployed, launching directly: {exe_path}")
            _launch_app(exe_path, self_exe=self_exe)
            return

    _ensure_structure(deploy_dir)

    exe_path = _find_current_version_exe(deploy_dir)
    if exe_path:
        _log(f"found existing exe, launching directly")
        _launch_app(exe_path, self_exe=self_exe)
        return

    splash = SplashWindow()
    use_console = not splash.ok

    try:
        if use_console:
            print(f"\n{APP_NAME} - 首次运行初始化")
            print("=" * 40)
            print("正在检查更新...")
        else:
            splash.update("正在检查更新...", 0.05)

        release, source = _fetch_latest_release()
        if not release:
            if use_console:
                print("\n无法连接更新服务器，请检查网络")
                input("按回车键退出...")
            else:
                splash.update("无法连接更新服务器，请检查网络", 0)
                time.sleep(3)
            return

        tag = release.get("tag_name", "")
        download_url = None
        download_name = ""
        for asset in release.get("assets", []):
            name = asset.get("name", "")
            if name.endswith(".exe") and any(p in name for p in EXE_PREFIXES) and "-v" in name:
                download_url = asset.get("browser_download_url", "")
                download_name = name
                break

        if not download_url:
            _log(f"no matching exe found in release assets, tag={tag}")
            if use_console:
                print("\n未找到可下载的版本")
                input("按回车键退出...")
            else:
                splash.update("未找到可下载的版本", 0)
                time.sleep(3)
            return

        dest_path = os.path.join(deploy_dir, "ver", f"{APP_NAME}-{tag}.exe")
        os.makedirs(os.path.dirname(dest_path), exist_ok=True)

        if use_console:
            print(f"正在下载 {tag}...")
        else:
            splash.update(f"正在下载 {tag}...", 0.1)

        def on_progress(downloaded, total):
            mb_d = downloaded / 1024 / 1024
            mb_t = total / 1024 / 1024
            pct = 0.1 + 0.8 * (downloaded / total) if total > 0 else 0.1
            if use_console:
                _console_print(f"下载进度: {mb_d:.1f}MB / {mb_t:.1f}MB ({int(pct*100)}%)")
            else:
                splash.update(f"正在下载 {tag}... {mb_d:.1f}MB / {mb_t:.1f}MB", pct)

        ok = _download_with_mirrors(download_url, dest_path, source, on_progress)

        if not ok or not os.path.isfile(dest_path):
            if use_console:
                print("\n下载失败，请检查网络后重试")
                input("按回车键退出...")
            else:
                splash.update("下载失败，请检查网络后重试", 0)
                time.sleep(3)
            return

        settings = _load_settings(deploy_dir)
        settings["current_app_version"] = tag.lstrip("v")
        _save_settings(deploy_dir, settings)

        main_exe = _find_current_version_exe(deploy_dir)
        if main_exe:
            _create_shortcut(deploy_dir, main_exe)

        if use_console:
            print(f"\n下载完成! 正在启动 {tag}...")
        else:
            splash.update("启动中...", 1.0)
            time.sleep(0.3)

        exe_path = _find_current_version_exe(deploy_dir)
        if exe_path:
            _launch_app(exe_path, self_exe=self_exe)
    finally:
        splash.close()


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        import traceback
        tb = traceback.format_exc()
        _log(f"launcher_stub FATAL:\n{tb}")
        log_path = os.path.join(
            os.path.dirname(os.path.abspath(sys.executable if getattr(sys, 'frozen', False) else __file__)),
            "launcher_error.log"
        )
        try:
            with open(log_path, "w", encoding="utf-8") as f:
                f.write(tb)
        except Exception:
            pass
        try:
            _fix_tcl_tk_paths()
            import tkinter as tk_
            from tkinter import messagebox
            root = tk_.Tk()
            root.withdraw()
            messagebox.showerror("启动器错误", f"错误已保存到:\n{log_path}\n\n详细日志:\n{_LOG_FILE}\n\n{tb[:300]}")
        except Exception:
            pass
