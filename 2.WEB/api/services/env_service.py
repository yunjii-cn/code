import os
import sys
import shutil
import subprocess
import urllib.request
import zipfile

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"

NODE_VERSION = "v24.11.1"
NODE_DIR_NAME = f"node-{NODE_VERSION}-win-x64"
BUN_VERSION = "1.1.42"
BUN_DIR_NAME = "bun-windows-x64"
UV_PYTHON_VERSION = "3.12"

MIRROR_SOURCES = {
    "official": {
        "label": "🌐 官方源",
        "node": "https://nodejs.org/dist/",
        "github_proxy": "",
        "uv_python_mirror": "",
        "pypi_index": "https://pypi.org/simple/",
    },
    "china": {
        "label": "🇨🇳 国内镜像",
        "node": "https://npmmirror.com/mirrors/node/",
        "github_proxy": "https://gh-proxy.com/",
        "uv_python_mirror": "https://registry.npmmirror.com/-/binary/python-build-standalone/",
        "pypi_index": "https://pypi.tuna.tsinghua.edu.cn/simple/",
    },
}

MIRROR_SETTINGS_FILE = "mirror_source.json"


class EnvInstaller:
    """便携版环境下载与安装"""

    def __init__(self, base_dir: str, log_func=None, progress_func=None, data_dir: str = None, temp_dir: str = None):
        self.base_dir = base_dir
        self.data_dir = data_dir or base_dir
        self.temp_dir = temp_dir or os.path.join(os.path.dirname(base_dir), "temp")
        self.log = log_func or (lambda *a: None)
        self.progress = progress_func
        self._mirror_key = "china"

    def _load_mirror(self):
        try:
            fp = os.path.join(self.data_dir, MIRROR_SETTINGS_FILE)
            if os.path.isfile(fp):
                with open(fp, "r", encoding="utf-8") as f:
                    key = f.read().strip()
                if key in MIRROR_SOURCES:
                    self._mirror_key = key
        except Exception:
            pass

    def _save_mirror(self, key: str):
        self._mirror_key = key
        try:
            fp = os.path.join(self.data_dir, MIRROR_SETTINGS_FILE)
            with open(fp, "w", encoding="utf-8") as f:
                f.write(key)
        except Exception:
            pass

    @property
    def mirror(self) -> dict:
        return MIRROR_SOURCES.get(self._mirror_key, MIRROR_SOURCES["china"])

    def _github_url(self, original_url: str) -> str:
        proxy = self.mirror.get("github_proxy", "")
        if proxy and "github.com" in original_url:
            return proxy + original_url
        return original_url

    @property
    def nodejs_dir(self): return os.path.join(self.base_dir, "nodejs")
    @property
    def node_extract(self): return os.path.join(self.nodejs_dir, NODE_DIR_NAME)
    @property
    def node_exe(self): return os.path.join(self.node_extract, "node.exe")
    @property
    def bun_dir(self): return os.path.join(self.base_dir, "bun")
    @property
    def bun_extract(self): return os.path.join(self.bun_dir, BUN_DIR_NAME)
    @property
    def bun_exe(self): return os.path.join(self.bun_extract, "bun.exe")
    @property
    def uv_dir(self): return os.path.join(self.base_dir, "uv")
    @property
    def uv_exe(self): return os.path.join(self.uv_dir, "uv.exe")
    @property
    def uv_python_dir(self): return os.path.join(self.base_dir, "python")
    @property
    def scripts_dir(self): return os.path.join(self.base_dir, "scripts")
    @property
    def venv_dir(self): return os.path.join(self.data_dir, ".venv")
    @property
    def venv_python(self):
        return os.path.join(self.venv_dir, "Scripts", "python.exe") if os.name == "nt" else os.path.join(self.venv_dir, "bin", "python")

    def check_node(self): return os.path.exists(self.node_exe)
    def check_bun(self): return os.path.exists(self.bun_exe)
    def check_deps(self):
        tsx = os.path.join(self.base_dir, "node_modules", "tsx", "dist", "loader.mjs")
        return os.path.exists(os.path.join(self.base_dir, "node_modules")) and os.path.exists(tsx)
    def check_dist(self): return os.path.exists(os.path.join(self.base_dir, "desktop", "dist"))
    def check_electron(self):
        return os.path.exists(os.path.join(self.base_dir, "node_modules", ".bin", "electron.cmd"))

    def check_uv(self): return os.path.exists(self.uv_exe)

    def check_qwen2api(self):
        if not os.path.isfile(self.venv_python):
            return False
        try:
            r = subprocess.run(
                [self.venv_python, "-c",
                 "import fastapi, uvicorn, httpx, pydantic_settings, tiktoken, curl_cffi; print('ok')"],
                capture_output=True, text=True, timeout=15,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            return r.returncode == 0 and "ok" in (r.stdout or "")
        except Exception:
            return False

    def check_all(self):
        return {
            "node": self.check_node(), "bun": self.check_bun(),
            "deps": self.check_deps(), "dist": self.check_dist(),
            "electron": self.check_electron(),
            "uv": self.check_uv(), "qwen2api": self.check_qwen2api(),
        }

    def _download(self, url: str, dest: str, label: str):
        mirror_label = self.mirror.get("label", "")
        self.log(f"正在下载 {label}... [{mirror_label}]")
        try:
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            resp = urllib.request.urlopen(req, timeout=120)
            total = int(resp.headers.get("Content-Length", 0))
            dl = 0
            with open(dest, "wb") as f:
                while True:
                    chunk = resp.read(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    dl += len(chunk)
                    if total > 0 and self.progress:
                        self.progress(int(dl * 100 / total), f"下载 {label} {int(dl*100/total)}%")
            self.log(f"✓ {label} 下载完成")
            return True
        except Exception as e:
            self.log(f"[错误] 下载 {label} 失败: {e}", "#F44336")
            return False

    def _unzip(self, zip_path, dest_dir, label):
        self.log(f"正在解压 {label}...")
        try:
            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(dest_dir)
            try:
                os.remove(zip_path)
            except:
                pass
            self.log(f"✓ {label} 解压完成")
            return True
        except Exception as e:
            self.log(f"[错误] 解压 {label} 失败: {e}", "#F44336")
            return False

    def _si(self):
        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        return si

    def install_node(self):
        if self.check_node():
            self.log("✓ Node.js 已安装")
            return True
        zip_path = os.path.join(self.nodejs_dir, f"{NODE_DIR_NAME}.zip")
        node_base = self.mirror.get("node", "https://nodejs.org/dist/")
        url = f"{node_base}{NODE_VERSION}/node-{NODE_VERSION}-win-x64.zip"
        if not os.path.exists(zip_path):
            if not self._download(url, zip_path, f"Node.js {NODE_VERSION}"):
                return False
        if os.path.exists(self.node_extract):
            shutil.rmtree(self.node_extract, ignore_errors=True)
        return self._unzip(zip_path, self.nodejs_dir, "Node.js")

    def install_bun(self):
        if self.check_bun():
            self.log("✓ Bun 已安装")
            return True
        zip_path = os.path.join(self.bun_dir, f"{BUN_DIR_NAME}.zip")
        url = self._github_url(f"https://github.com/oven-sh/bun/releases/download/bun-v{BUN_VERSION}/bun-windows-x64.zip")
        if not os.path.exists(zip_path):
            if not self._download(url, zip_path, f"Bun {BUN_VERSION}"):
                return False
        if os.path.exists(self.bun_extract):
            shutil.rmtree(self.bun_extract, ignore_errors=True)
        return self._unzip(zip_path, self.bun_dir, "Bun")

    def install_deps(self):
        if self.check_deps():
            self.log("✓ 依赖已安装")
            return True
        if not self.check_bun():
            self.log("[错误] Bun 未安装", "#F44336")
            return False
        try:
            env = dict(os.environ)
            if os.path.exists(self.node_extract):
                env["PATH"] = self.node_extract + ";" + env.get("PATH", "")
            if os.path.exists(self.bun_extract):
                env["PATH"] = self.bun_extract + ";" + env.get("PATH", "")
            bun_cache_dir = os.path.join(self.temp_dir, "cache", "bun_cache")
            os.makedirs(bun_cache_dir, exist_ok=True)
            env["BUN_INSTALL_CACHE_DIR"] = bun_cache_dir
            subprocess.run([self.bun_exe, "config", "set", "registry", "https://registry.npmmirror.com"],
                           cwd=self.base_dir, env=env, startupinfo=self._si(), capture_output=True, timeout=30,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            r = subprocess.run([self.bun_exe, "install"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=300,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                self.log("✓ 依赖安装完成")
                self._fix_jsonc_parser_esm()
                return True
            self.log(f"[警告] bun install 返回码: {r.returncode}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] 依赖安装失败: {e}", "#F44336")
            return False

    def _fix_jsonc_parser_esm(self):
        esm_dir = os.path.join(self.base_dir, "node_modules", "jsonc-parser", "lib", "esm")
        if not os.path.isdir(esm_dir):
            return
        pkg_path = os.path.join(esm_dir, "package.json")
        if not os.path.exists(pkg_path):
            try:
                with open(pkg_path, 'w', encoding='utf-8') as f:
                    f.write('{"type":"module"}')
                self.log("  修复 jsonc-parser ESM 类型声明")
            except Exception:
                pass
        targets = ['scanner', 'parser', 'format', 'edit', 'string-intern']
        count = 0
        for root, dirs, files in os.walk(esm_dir):
            for fname in files:
                if not fname.endswith('.js'):
                    continue
                fpath = os.path.join(root, fname)
                try:
                    with open(fpath, 'r', encoding='utf-8') as f:
                        content = f.read()
                    original = content
                    for t in targets:
                        content = content.replace(f"from './{t}'", f"from './{t}.js'")
                        content = content.replace(f"from '../{t}'", f"from '../{t}.js'")
                    if content != original:
                        with open(fpath, 'w', encoding='utf-8') as f:
                            f.write(content)
                        count += 1
                except Exception:
                    pass
        if count:
            self.log(f"  修复 jsonc-parser ESM 导入 ({count} 文件)")

    def build_frontend(self):
        dist = os.path.join(self.base_dir, "desktop", "dist")
        if os.path.exists(dist):
            self.log("✓ 前端已构建")
            return True
        if not self.check_bun():
            self.log("[错误] Bun 未安装", "#F44336")
            return False
        try:
            env = dict(os.environ)
            if os.path.exists(self.node_extract):
                env["PATH"] = self.node_extract + ";" + env.get("PATH", "")
            if os.path.exists(self.bun_extract):
                env["PATH"] = self.bun_extract + ";" + env.get("PATH", "")
            bun_cache_dir = os.path.join(self.temp_dir, "cache", "bun_cache")
            os.makedirs(bun_cache_dir, exist_ok=True)
            env["BUN_INSTALL_CACHE_DIR"] = bun_cache_dir
            r = subprocess.run([self.bun_exe, "run", "desktop:build"], cwd=self.base_dir, env=env,
                               startupinfo=self._si(), capture_output=True, text=True, timeout=120,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                self.log("✓ 前端构建完成")
                return True
            self.log(f"[警告] 前端构建返回码: {r.returncode}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] 前端构建失败: {e}", "#F44336")
            return False

    def install_all(self):
        return all([
            self.install_node(),
            self.install_bun(),
            self.install_deps(),
            self.build_frontend(),
            self.install_uv(),
            self.install_qwen2api_deps(),
        ])

    def install_uv(self):
        if self.check_uv():
            self.log("✓ uv 已安装")
            return True
        os.makedirs(self.uv_dir, exist_ok=True)
        try:
            env = os.environ.copy()
            env["UV_INSTALL_DIR"] = self.uv_dir
            mirror = self.mirror
            github_proxy = mirror.get("github_proxy", "")
            proxy_urls = []
            if github_proxy:
                proxy_urls.append(github_proxy + "https://astral.sh/uv/install.ps1")
            proxy_urls.append("https://astral.sh/uv/install.ps1")
            si = self._si()
            success = False
            for url in proxy_urls:
                source_label = "代理" if url != proxy_urls[-1] else "直连"
                self.log(f"正在下载 uv 安装脚本 ({source_label})... [{mirror.get('label', '')}]")
                ps_script = f"""
$ProgressPreference = 'SilentlyContinue'
$env:UV_INSTALL_DIR = '{self.uv_dir}'
try {{
    Invoke-WebRequest -Uri '{url}' -OutFile '$env:TEMP\\uv-install.ps1' -TimeoutSec 30
    powershell -ExecutionPolicy Bypass -File '$env:TEMP\\uv-install.ps1'
    Remove-Item '$env:TEMP\\uv-install.ps1' -ErrorAction SilentlyContinue
}} catch {{
    Write-Output "DOWNLOAD_FAILED"
    Exit 1
}}
"""
                proc = subprocess.Popen(
                    ["powershell.exe", "-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", ps_script],
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                    env=env, startupinfo=si, creationflags=subprocess.CREATE_NO_WINDOW,
                )
                output_lines = []
                while proc.poll() is None:
                    line = proc.stdout.readline()
                    if line and line.strip():
                        self.log(f"  {line.strip()}")
                        output_lines.append(line.strip())
                if "DOWNLOAD_FAILED" in output_lines:
                    self.log(f"  {source_label}下载失败，尝试下一个源...", "#FF9800")
                    continue
                if proc.returncode == 0:
                    success = True
                    break
                self.log(f"  {source_label}安装失败 (返回码: {proc.returncode})", "#FF9800")
            if not success and os.path.exists(os.path.expanduser("~/.local/bin/uv.exe")):
                src = os.path.expanduser("~/.local/bin/uv.exe")
                shutil.copy2(src, self.uv_exe)
                self.log("✓ uv 安装完成（从默认路径复制）")
                return True
            if success and os.path.exists(self.uv_exe):
                self.log("✓ uv 安装完成")
                return True
            if not success:
                self.log("[错误] uv 安装失败，所有源均不可用", "#F44336")
                return False
            self.log("✓ uv 安装完成")
            return True
        except Exception as e:
            self.log(f"[错误] uv 安装异常: {e}", "#F44336")
            return False

    def _uv_env(self):
        env = os.environ.copy()
        env["UV_PYTHON_INSTALL_DIR"] = self.uv_python_dir
        env["UV_PYTHON_DOWNLOADS"] = "auto"
        uv_cache_dir = os.path.join(self.temp_dir, "cache", "uv_cache")
        os.makedirs(uv_cache_dir, exist_ok=True)
        env["UV_CACHE_DIR"] = uv_cache_dir
        python_mirror = self.mirror.get("uv_python_mirror", "")
        if python_mirror:
            env["UV_PYTHON_INSTALL_MIRROR"] = python_mirror
        pypi_index = self.mirror.get("pypi_index", "")
        if pypi_index:
            env["UV_INDEX_URL"] = pypi_index
            env["UV_DEFAULT_INDEX"] = pypi_index
        return env

    def install_qwen2api_deps(self):
        if self.check_qwen2api():
            self.log("✓ API 服务依赖已安装")
            return True
        if not self.install_uv():
            self.log("[错误] uv 安装失败，无法继续", "#F44336")
            return False
        try:
            env = self._uv_env()

            uv_exe_abs = os.path.abspath(self.uv_exe)
            venv_dir_abs = os.path.abspath(self.venv_dir)
            venv_python_abs = os.path.abspath(self.venv_python)
            base_dir_abs = os.path.abspath(self.base_dir)

            os.makedirs(os.path.dirname(venv_dir_abs), exist_ok=True)

            old_venv_dir = os.path.join(self.data_dir, "venvs", ".venv")
            if os.path.isdir(old_venv_dir) and not os.path.isdir(venv_dir_abs):
                self.log("  迁移旧虚拟环境: data/venvs/.venv → data/.venv")
                try:
                    shutil.move(old_venv_dir, venv_dir_abs)
                    self.log("  ✓ 虚拟环境迁移完成")
                except Exception as e:
                    self.log(f"  迁移失败: {e}，将重新创建", "#FF9800")

            old_venvs_dir = os.path.join(self.data_dir, "venvs")
            if os.path.isdir(old_venvs_dir):
                try:
                    shutil.rmtree(old_venvs_dir, ignore_errors=True)
                    self.log("  ✓ 清理旧目录: data/venvs/")
                except Exception:
                    pass

            venv_valid = False
            if os.path.isfile(venv_python_abs):
                pip_exe = os.path.join(venv_dir_abs, "Scripts", "pip.exe")
                if not os.path.exists(pip_exe):
                    pip_exe = os.path.join(venv_dir_abs, "bin", "pip")
                if os.path.exists(pip_exe):
                    venv_valid = True
                    self.log(f"✓ 虚拟环境已存在且有效: {venv_python_abs}")
                else:
                    self.log(f"  警告: 虚拟环境已存在但缺少 pip.exe，需要重建", "#FF9800")

            if not venv_valid:
                self.log("正在创建虚拟环境...")
                self.log(f"  虚拟环境目录: {venv_dir_abs}")
                if os.path.exists(venv_dir_abs):
                    try:
                        shutil.rmtree(venv_dir_abs)
                        self.log(f"  已清理旧的虚拟环境目录")
                    except Exception as e:
                        self.log(f"  警告：无法清理旧的虚拟环境目录: {e}")
                try:
                    import venv
                    self.log("  使用 Python venv 创建虚拟环境...")
                    venv.create(venv_dir_abs, with_pip=True)
                    self.log("  ✓ venv 创建成功")
                except Exception as e:
                    self.log(f"  警告: venv 创建失败: {e}, 尝试用 uv...", "#FF9800")
                    r = subprocess.run(
                        [uv_exe_abs, "venv", self.venv_dir, "--python", UV_PYTHON_VERSION, "--clear"],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=base_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if r.returncode != 0:
                        self.log(f"  uv 创建也失败 (返回码: {r.returncode})", "#FF9800")
                        if r.stdout:
                            self.log(f"  stdout: {r.stdout[:500]}")
                        if r.stderr:
                            self.log(f"  stderr: {r.stderr[:500]}", "#FF9800")
                if not os.path.exists(venv_python_abs):
                    self.log(f"[错误] 找不到 python.exe: {venv_python_abs}", "#F44336")
                    if os.path.exists(venv_dir_abs):
                        contents = os.listdir(venv_dir_abs)
                        self.log(f"  虚拟环境目录内容: {contents}")
                    return False
                self.log("✓ 虚拟环境已创建")

            self.log("正在安装 API 服务依赖...")
            pip_exe = os.path.join(venv_dir_abs, "Scripts", "pip.exe")
            if not os.path.exists(pip_exe):
                pip_exe = os.path.join(venv_dir_abs, "bin", "pip")

            req_files = []
            for svc in ["qwen2api", "zhipu2api"]:
                for candidate in [
                    os.path.join(self.base_dir, "api", svc, "backend", "requirements.txt"),
                    os.path.join(self.base_dir, "api", svc, "requirements.txt"),
                ]:
                    if os.path.exists(candidate):
                        req_files.append(candidate)
                        break

            if not req_files:
                self.log("⚠ 未找到任何 API 服务的 requirements.txt", "#FF9800")
                return True

            last_r = None
            if os.path.exists(pip_exe):
                self.log(f"  使用 pip: {pip_exe}")
                self.log("  升级 pip...")
                subprocess.run(
                    [pip_exe, "install", "--upgrade", "pip"],
                    env=env, capture_output=True, text=True, timeout=300,
                    cwd=base_dir_abs,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                )
                for rf in req_files:
                    self.log(f"  安装 {os.path.basename(os.path.dirname(rf))} 依赖...")
                    last_r = subprocess.run(
                        [pip_exe, "install", "-r", rf],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=base_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if last_r.returncode != 0:
                        break
            else:
                self.log(f"  警告: pip.exe 不存在，尝试用 uv pip")
                for rf in req_files:
                    self.log(f"  安装 {os.path.basename(os.path.dirname(rf))} 依赖...")
                    last_r = subprocess.run(
                        [uv_exe_abs, "pip", "install", "-r", rf],
                        env=env, capture_output=True, text=True, timeout=600,
                        cwd=venv_dir_abs,
                        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
                    )
                    if last_r.returncode != 0:
                        break

            if last_r and last_r.returncode == 0:
                self.log("✓ API 服务依赖安装完成")
                return True
            if last_r:
                self.log(f"[警告] API 服务依赖安装返回码: {last_r.returncode}", "#FF9800")
                if last_r.stdout:
                    self.log(f"  stdout: {last_r.stdout[:500]}")
                if last_r.stderr:
                    self.log(f"  stderr: {last_r.stderr[:500]}", "#FF9800")
            return False
        except Exception as e:
            self.log(f"[错误] API 服务依赖安装失败: {e}", "#F44336")
            import traceback
            self.log(f"  堆栈: {traceback.format_exc()[:500]}", "#F44336")
            return False


def _get_base_dir() -> str:
    if getattr(sys, 'frozen', False):
        dev_dir = os.environ.get("YUNJI_DEV_DIR", "")
        if dev_dir:
            return os.path.abspath(dev_dir)
        return os.path.abspath(os.path.dirname(sys.executable))
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


class EnvService:
    def __init__(self):
        base_dir = _get_base_dir()
        app_dir = os.path.join(base_dir, "app")
        data_dir = os.path.join(base_dir, "data")
        temp_dir = os.path.join(base_dir, "temp")
        self._installer = EnvInstaller(
            base_dir=app_dir,
            data_dir=data_dir,
            temp_dir=temp_dir,
        )
        self._base_dir = base_dir

    async def check_environment(self):
        from services.ai_service import check_environment as _check_env
        return _check_env()

    async def get_status(self):
        return self._installer.check_all()

    async def install_component(self, component: str):
        install_map = {
            "node": self._installer.install_node,
            "bun": self._installer.install_bun,
            "deps": self._installer.install_deps,
            "uv": self._installer.install_uv,
            "qwen2api": self._installer.install_qwen2api_deps,
        }
        fn = install_map.get(component)
        if not fn:
            return {"ok": False, "error": f"未知组件: {component}"}
        ok = fn()
        return {"ok": ok, "component": component}

    async def install_all(self):
        ok = self._installer.install_all()
        return {"ok": ok}

    async def get_mirror(self):
        self._installer._load_mirror()
        return {"key": self._installer._mirror_key, "mirror": self._installer.mirror}

    async def set_mirror(self, mirror_key: str):
        if mirror_key not in MIRROR_SOURCES:
            return {"ok": False, "error": f"未知镜像源: {mirror_key}"}
        self._installer._save_mirror(mirror_key)
        return {"ok": True, "key": mirror_key}

    async def list_services(self):
        from services.ai_service import list_api_services
        return list_api_services()

    async def get_service_info(self, service_name: str):
        from services.ai_service import get_api_service_info
        return get_api_service_info(service_name)

    async def start_service(self, service_name: str, project_dir=None, port=None, admin_key=None):
        from services.ai_service import start_qwen2api, start_zhipu2api, API_SERVICE_REGISTRY
        if service_name not in API_SERVICE_REGISTRY:
            return {"ok": False, "error": f"未知服务: {service_name}"}
        reg = API_SERVICE_REGISTRY[service_name]
        p = port or reg["default_port"]
        ak = admin_key or "admin"
        pd = project_dir or ""
        if service_name == "qwen2api":
            return start_qwen2api(pd, p, ak)
        elif service_name == "zhipu2api":
            return start_zhipu2api(pd, p, ak)
        return {"ok": False, "error": f"不支持的服务: {service_name}"}

    async def stop_service(self, service_name: str, base_url=None):
        from services.ai_service import stop_qwen2api, stop_zhipu2api, API_SERVICE_REGISTRY
        if service_name not in API_SERVICE_REGISTRY:
            return {"ok": False, "error": f"未知服务: {service_name}"}
        if service_name == "qwen2api":
            return stop_qwen2api(base_url or "")
        elif service_name == "zhipu2api":
            return stop_zhipu2api(base_url or "")
        return {"ok": False, "error": f"不支持的服务: {service_name}"}

    async def start_all_services(self):
        from services.ai_service import start_all_api_services
        return start_all_api_services()
