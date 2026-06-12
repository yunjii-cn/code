#!/usr/bin/env python3
"""
云集智能编程工作站 - 引导启动器 (Bootstrapper)

首次运行：创建目录 → 下载最新版本 → 启动（无缝衔接主程序启动画面）
后续运行：检测已有版本 → 直接启动（瞬间完成）

打包方式：
  python -m PyInstaller --onefile --windowed --icon icon.ico --name 云集智能编程工作站 bootstrapper.py
"""
import os
import sys
import json
import re
import subprocess
import threading
import time
import zipfile
import shutil
from pathlib import Path
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError

GITEE_OWNER = "yunjii"
GITEE_REPO = "code"
VERSION_JSON_URL = f"https://gitee.com/{GITEE_OWNER}/{GITEE_REPO}/raw/main/dev/ver/version.json"
GITEE_RELEASE_BASE = f"https://gitee.com/{GITEE_OWNER}/{GITEE_REPO}/releases/download"
GITHUB_RELEASE_BASE = f"https://github.com/yunjii-cn/code/releases/download"
APP_NAME = "云集智能编程工作站"


def get_dev_dir():
    if getattr(sys, 'frozen', False):
        return Path(sys.executable).parent
    script_dir = Path(__file__).resolve().parent
    if (script_dir / "main.py").exists() or (script_dir / "desktop").is_dir():
        return script_dir.parent
    return script_dir


def find_latest_exe(ver_dir):
    if not ver_dir.exists():
        return None
    exes = list(ver_dir.glob(f"{APP_NAME}v*.exe"))
    if not exes:
        return None

    def ver_key(p):
        m = re.search(r'v(\d+\.\d+\.\d+\.\d+)', p.name)
        return tuple(int(x) for x in m.group(1).split('.')) if m else (0, 0, 0, 0)

    exes.sort(key=ver_key, reverse=True)
    return exes[0]


def fetch_version_info():
    req = Request(VERSION_JSON_URL, headers={"User-Agent": "Mozilla/5.0"})
    resp = urlopen(req, timeout=15)
    return json.loads(resp.read().decode("utf-8"))


def _download_file(url, dest_path, progress_cb=None, cancelled_check=None):
    req = Request(url, headers={"User-Agent": "Mozilla/5.0"})
    resp = urlopen(req, timeout=600)
    total_size = int(resp.headers.get('Content-Length', 0))
    downloaded = 0
    chunk_size = 131072
    last_time = time.time()
    last_dl = 0

    with open(dest_path, "wb") as f:
        while True:
            if cancelled_check and cancelled_check():
                return False
            chunk = resp.read(chunk_size)
            if not chunk:
                break
            f.write(chunk)
            downloaded += len(chunk)
            if progress_cb and total_size > 0:
                now = time.time()
                elapsed = now - last_time
                if elapsed >= 0.3:
                    speed = (downloaded - last_dl) / elapsed / (1024 * 1024)
                    last_time = now
                    last_dl = downloaded
                    progress_cb(downloaded, total_size, speed)
                else:
                    progress_cb(downloaded, total_size, 0)
    return True


class BootstrapperUI:
    def __init__(self):
        import tkinter as tk
        from tkinter import ttk

        self.dev_dir = get_dev_dir()
        self.ver_dir = self.dev_dir / "ver"
        self._closed = False

        self.root = tk.Tk()
        self.root.title(APP_NAME)
        self.root.geometry("520x240+{}+{}".format(
            (self.root.winfo_screenwidth() - 520) // 2,
            (self.root.winfo_screenheight() - 240) // 2,
        ))
        self.root.resizable(False, False)
        self.root.configure(bg="#1a1a1a")

        icon_path = self.dev_dir / "icon.ico"
        if not icon_path.exists():
            icon_path = self.dev_dir / "app" / "icon.ico"
        if icon_path.exists():
            try:
                self.root.iconbitmap(str(icon_path))
            except Exception:
                pass

        tk.Label(
            self.root, text=APP_NAME,
            font=("Microsoft YaHei", 18, "bold"),
            fg="#E0E0E0", bg="#1a1a1a",
        ).pack(pady=(30, 5))

        self.status_var = tk.StringVar(value="正在初始化...")
        tk.Label(
            self.root, textvariable=self.status_var,
            font=("Microsoft YaHei", 11), fg="#888", bg="#1a1a1a",
        ).pack(pady=(0, 12))

        style = ttk.Style()
        style.theme_use('default')
        style.configure(
            "Boot.Horizontal.TProgressbar",
            troughcolor='#2a2a2a', background='#CC0000',
            darkcolor='#CC0000', lightcolor='#E00000',
            bordercolor='#1a1a1a', thickness=6,
        )
        self.progress = ttk.Progressbar(
            self.root, length=440, mode='determinate',
            style="Boot.Horizontal.TProgressbar",
        )
        self.progress.pack(pady=(0, 8))

        self.detail_var = tk.StringVar(value="")
        tk.Label(
            self.root, textvariable=self.detail_var,
            font=("Consolas", 9), fg="#555", bg="#1a1a1a",
        ).pack()

        self.root.protocol("WM_DELETE_WINDOW", self._on_close)

    def _on_close(self):
        self._closed = True
        self.root.destroy()

    def update(self, pct, status="", detail=""):
        if self._closed:
            return

        def _do():
            try:
                self.progress['value'] = pct * 100
                if status:
                    self.status_var.set(status)
                if detail:
                    self.detail_var.set(detail)
            except Exception:
                pass

        try:
            self.root.after(0, _do)
        except Exception:
            pass

    def launch_and_close(self, exe_path):
        if self._closed:
            return

        def _do():
            self.update(1.0, "启动完成！", "")
            self.root.update_idletasks()
            time.sleep(0.2)
            subprocess.Popen([str(exe_path)], cwd=str(self.dev_dir))
            self.root.after(300, self.root.destroy)

        self.root.after(0, _do)

    def run(self):
        latest_exe = find_latest_exe(self.ver_dir)
        app_dir = self.dev_dir / "app"
        app_main = app_dir / "main.py"

        if latest_exe and latest_exe.exists() and app_main.exists():
            self.update(0.5, "正在启动...", "")
            self.root.after(150, lambda: self.launch_and_close(latest_exe))
            self.root.mainloop()
            return

        def _setup():
            try:
                self._do_setup()
            except Exception as e:
                self.update(0, f"初始化失败: {e}", "请检查网络连接后重试")

        threading.Thread(target=_setup, daemon=True).start()
        self.root.mainloop()

    def _do_setup(self):
        self.update(0.02, "正在创建目录结构...", "")
        self.ver_dir.mkdir(parents=True, exist_ok=True)
        (self.dev_dir / "data").mkdir(parents=True, exist_ok=True)
        (self.dev_dir / "temp").mkdir(parents=True, exist_ok=True)
        for sub in ["public/models", "public/templates", "public/plugins",
                     "users/default/projects", "users/default/sessions"]:
            (self.dev_dir / "data" / sub).mkdir(parents=True, exist_ok=True)
        for sub in ["logs", "cache", "debug", "tmp"]:
            (self.dev_dir / "temp" / sub).mkdir(parents=True, exist_ok=True)

        self.update(0.05, "正在获取版本信息...", "连接 Gitee...")
        try:
            version_data = fetch_version_info()
        except Exception:
            self.update(0, "无法连接服务器", "请检查网络连接后重试")
            return

        latest_version = version_data.get("latest", "")
        if not latest_version:
            self.update(0, "无法获取版本信息", "请检查网络连接后重试")
            return

        versions = version_data.get("versions", [])
        filename = ""
        for v in versions:
            if v.get("version") == latest_version:
                filename = v.get("filename") or v.get("exe", "")
                break
        if not filename:
            filename = f"{APP_NAME}v{latest_version}.exe"

        exe_path = self.ver_dir / filename
        need_exe = not exe_path.exists()
        need_app = not (self.dev_dir / "app" / "main.py").exists()

        if not need_exe and not need_app:
            self.launch_and_close(exe_path)
            return

        if need_exe:
            self._download_exe(latest_version, filename, exe_path)
            if self._closed or not exe_path.exists():
                return

        if need_app:
            self._download_app_package(latest_version)
            if self._closed:
                return

        self.update(0.97, "正在准备启动...", "")
        time.sleep(0.1)
        if exe_path.exists():
            self.launch_and_close(exe_path)
        else:
            self.update(0, "启动失败", "主程序文件未找到")

    def _download_exe(self, version, filename, dest_path):
        urls = [
            f"{GITEE_RELEASE_BASE}/v{version}/{filename}",
            f"{GITHUB_RELEASE_BASE}/v{version}/{filename}",
        ]

        for url in urls:
            source = "Gitee" if "gitee.com" in url else "GitHub"
            self.update(0.08, f"正在从 {source} 下载...", f"v{version}")

            def progress_cb(dl, total, speed):
                pct = 0.08 + 0.60 * (dl / total)
                mb_dl = dl / (1024 * 1024)
                mb_total = total / (1024 * 1024)
                if speed > 0:
                    eta = (total - dl) / (speed * 1024 * 1024)
                    self.update(pct, f"正在下载 v{version}...",
                               f"{mb_dl:.1f}MB / {mb_total:.1f}MB  {speed:.1f}MB/s  剩余{eta:.0f}s")
                else:
                    self.update(pct, f"正在下载 v{version}...",
                               f"{mb_dl:.1f}MB / {mb_total:.1f}MB")

            try:
                ok = _download_file(url, dest_path, progress_cb, lambda: self._closed)
                if ok and dest_path.exists():
                    return
            except Exception:
                if dest_path.exists():
                    try:
                        dest_path.unlink()
                    except Exception:
                        pass
                continue

        self.update(0, "下载失败", "Gitee 和 GitHub 均不可用")

    def _download_app_package(self, version):
        pkg_names = [f"app_core.zip", f"app_core_v{version}.zip"]
        urls = []
        for name in pkg_names:
            urls.append(f"{GITEE_RELEASE_BASE}/v{version}/{name}")
            urls.append(f"{GITHUB_RELEASE_BASE}/v{version}/{name}")

        temp_zip = self.dev_dir / "temp" / "app_core_download.zip"

        for url in urls:
            source = "Gitee" if "gitee.com" in url else "GitHub"
            self.update(0.70, f"正在下载资源包 ({source})...", "")

            def progress_cb(dl, total, speed):
                pct = 0.70 + 0.22 * (dl / total)
                mb_dl = dl / (1024 * 1024)
                mb_total = total / (1024 * 1024)
                if speed > 0:
                    eta = (total - dl) / (speed * 1024 * 1024)
                    self.update(pct, "正在下载资源包...",
                               f"{mb_dl:.1f}MB / {mb_total:.1f}MB  {speed:.1f}MB/s  剩余{eta:.0f}s")
                else:
                    self.update(pct, "正在下载资源包...",
                               f"{mb_dl:.1f}MB / {mb_total:.1f}MB")

            try:
                ok = _download_file(url, temp_zip, progress_cb, lambda: self._closed)
                if ok and temp_zip.exists():
                    break
            except Exception:
                if temp_zip.exists():
                    try:
                        temp_zip.unlink()
                    except Exception:
                        pass
                continue
        else:
            self.update(0, "资源包下载失败", "请检查网络连接后重试")
            return

        self.update(0.93, "正在解压资源包...", "")
        app_dir = self.dev_dir / "app"
        try:
            with zipfile.ZipFile(str(temp_zip), 'r') as zf:
                members = zf.namelist()
                total = len(members)
                for i, name in enumerate(members):
                    if self._closed:
                        return
                    zf.extract(name, str(self.dev_dir))
                    if i % 50 == 0:
                        pct = 0.93 + 0.04 * (i / total)
                        self.update(pct, "正在解压资源包...", f"{i + 1}/{total} 文件")
        except Exception as e:
            self.update(0, f"解压失败: {e}", "")
            return
        finally:
            if temp_zip.exists():
                try:
                    temp_zip.unlink()
                except Exception:
                    pass


if __name__ == "__main__":
    app = BootstrapperUI()
    app.run()
