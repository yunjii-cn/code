import os
import sys
import json
import shutil
import subprocess
import threading
import urllib.request
import urllib.error
from pathlib import Path

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"

GIT_BRANCH = "main"

GITEE_OWNER = "yunjii"
GITEE_REPO = "code"
GITHUB_OWNER = "yunjii-cn"
GITHUB_REPO = "code"

GITEE_API_BASE = f"https://gitee.com/api/v5/repos/{GITEE_OWNER}/{GITEE_REPO}"
GITHUB_API_BASE = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}"

GITEE_RAW_URL = f"https://gitee.com/{GITEE_OWNER}/{GITEE_REPO}/raw/{GIT_BRANCH}/dev/ver/version.json"
GITHUB_RAW_URL = f"https://raw.githubusercontent.com/{GITHUB_OWNER}/{GITHUB_REPO}/{GIT_BRANCH}/dev/ver/version.json"


class SoftwareUpdater:
    """软件更新器 - 双源并行：Gitee（国内直连）+ GitHub（需代理），谁先返回用谁"""

    def __init__(self, dev_dir: str, log_func=None, progress_func=None):
        self.dev_dir = dev_dir
        self.app_dir = os.path.join(dev_dir, "app")
        self.ver_dir = os.path.join(dev_dir, "ver")
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func
        self._active_source = None

    def _si(self):
        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        return si

    def _run_git(self, *args, cwd=None, timeout=60):
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
        r = self._run_git("rev-parse", "--is-inside-work-tree")
        return r["ok"] and r["stdout"] == "true"

    def get_current_commit(self):
        r = self._run_git("rev-parse", "--short", "HEAD")
        return r["stdout"] if r["ok"] else "unknown"

    def get_remote_commit(self):
        r = self._run_git("fetch", "origin", GIT_BRANCH, timeout=30)
        if not r["ok"]:
            return None
        r2 = self._run_git("rev-parse", "--short", f"origin/{GIT_BRANCH}")
        return r2["stdout"] if r2["ok"] else None

    def check_update(self):
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
        if not self.is_git_repo():
            self.log("[错误] 不是 Git 仓库，无法更新", "#F44336")
            return False

        self.log("正在更新资源包...")
        self.progress(10, "正在拉取远程更新...")

        r = self._run_git("stash")
        stashed = r["ok"] and "Saved" in r["stdout"]

        r = self._run_git("pull", "origin", GIT_BRANCH, timeout=120)
        if not r["ok"]:
            self.log(f"[错误] 更新失败: {r['stderr'][:200]}", "#F44336")
            if stashed:
                self._run_git("stash", "pop")
            return False

        self.progress(70, "正在恢复本地配置...")

        if stashed:
            self._run_git("stash", "pop")

        new_commit = self.get_current_commit()
        self.log(f"✓ 资源包已更新到 {new_commit}", "#4CAF50")
        self.progress(100, "更新完成")
        return True

    def list_stable_exes(self):
        if not os.path.isdir(self.ver_dir):
            return []

        exes = []
        for f in os.listdir(self.ver_dir):
            if f.endswith(".exe"):
                path = os.path.join(self.ver_dir, f)
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

        exes.sort(key=lambda x: x["version"], reverse=True)
        return exes

    def get_git_history(self, limit=20):
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
        req = urllib.request.Request(url)
        req.add_header('User-Agent', 'Mozilla/5.0')
        if headers:
            for k, v in headers.items():
                req.add_header(k, v)
        resp = urllib.request.urlopen(req, timeout=timeout)
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
        source = self._active_source or "gitee"
        if source == "gitee":
            url = f"{GITEE_API_BASE}/zipball/{GIT_BRANCH}"
            self.log("正在从 Gitee 下载更新包...", "#FF9800")
        else:
            url = f"{GITHUB_API_BASE}/zipball/{GIT_BRANCH}"
            self.log("正在从 GitHub 下载更新包...", "#FF9800")
        return self._download_and_apply_zip(url, progress_func, source)

    def download_exe_from_release(self, version, filename, progress_func=None):
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
        try:
            os.makedirs(self.ver_dir, exist_ok=True)
            dest_path = os.path.join(self.ver_dir, filename)
            if progress_func:
                progress_func(5, "正在连接下载服务器...")
            req = urllib.request.Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            resp = urllib.request.urlopen(req, timeout=120)
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
        try:
            import tempfile
            import zipfile
            token = self._get_gitee_token()
            if progress_func:
                progress_func(10, "正在下载更新包...")
            req = urllib.request.Request(url)
            req.add_header('User-Agent', 'Mozilla/5.0')
            if token:
                if source == "gitee":
                    pass
                else:
                    req.add_header('Authorization', f'token {token}')
            resp = urllib.request.urlopen(req, timeout=120)
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


def _get_dev_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    return str(Path(__file__).resolve().parent.parent.parent)


class UpdateService:
    def __init__(self, dev_dir=None):
        self._updater = SoftwareUpdater(dev_dir or _get_dev_dir())

    async def get_current_version(self):
        commit = self._updater.get_current_commit()
        is_repo = self._updater.is_git_repo()
        return {
            "ok": True,
            "commit": commit,
            "isGitRepo": is_repo,
        }

    async def get_version_history(self):
        local = self._updater.get_local_version_history()
        return {"ok": True, "versions": local}

    async def fetch_remote_versions(self):
        remote = self._updater.fetch_remote_version_history()
        return {"ok": True, "versions": remote}

    async def check_update(self):
        return self._updater.check_update()

    async def pull_update(self):
        ok = self._updater.pull_update()
        return {"ok": ok}

    async def fetch_remote_commits(self, limit: int = 30):
        commits = self._updater.fetch_remote_commits(limit)
        return {"ok": True, "commits": commits}

    async def get_git_history(self, limit: int = 20):
        commits = self._updater.get_git_history(limit)
        return {"ok": True, "commits": commits}

    async def switch_commit(self, commit_hash: str):
        ok = self._updater.switch_git_commit(commit_hash)
        return {"ok": ok}

    async def switch_exe(self, exe_path: str, git_commit: str = None):
        import shutil
        if not os.path.exists(exe_path):
            return {"ok": False, "error": f"EXE 文件不存在: {exe_path}"}
        try:
            ver_dir = self._updater.ver_dir
            os.makedirs(ver_dir, exist_ok=True)
            dest = os.path.join(ver_dir, os.path.basename(exe_path))
            if exe_path != dest:
                shutil.copy2(exe_path, dest)
            return {"ok": True, "path": dest, "gitCommit": git_commit or ""}
        except Exception as e:
            return {"ok": False, "error": str(e)}

    async def list_stable_exes(self):
        exes = self._updater.list_stable_exes()
        return {"ok": True, "exes": exes}

    async def download_update(self, source: str = "gitee"):
        ok = self._updater.download_update_via_gitee()
        return {"ok": ok}
