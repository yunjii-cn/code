#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
YunJi SmartIDE v2.0 Launcher
- Single window, silent start
- Auto-detect & deploy environment
- Start backend + frontend + open browser
"""
import os
import sys
import time
import subprocess
import threading
import webbrowser
import urllib.request
import urllib.error

# ── Paths ──
HERE = os.path.dirname(os.path.abspath(__file__))
API_DIR = os.path.join(HERE, "api")
UI_DIR = os.path.join(HERE, "ui")
ROOT_DIR = os.path.dirname(HERE)  # project root (3.dev/..)
APP_DIR = os.path.join(ROOT_DIR, "1.PC", "app")

BACKEND_PORT = 18080
FRONTEND_PORT = 5173
STARTUP_TIMEOUT = 30  # seconds

# ── Colors (ANSI, disabled if NO_COLOR) ──
_NO_COLOR = os.environ.get("NO_COLOR") or os.environ.get("YUNJI_NO_COLOR")
if _NO_COLOR or not sys.stdout.isatty():
    C = lambda s: s
else:
    def C(s, code): return f"\033[{code}m{s}\033[0m"
    # shorthand
    class _C:
        def __getattr__(self, name):
            codes = {"green": "32", "red": "31", "yellow": "33", "cyan": "36", "dim": "2", "bold": "1"}
            return lambda s: f"\033[{codes.get(name, '0')}m{s}\033[0m"
    _c = _C()
    green = _c.green
    red = _c.red
    yellow = _c.yellow
    cyan = _c.cyan
    dim = _c.dim
    bold = _c.bold

if _NO_COLOR or not sys.stdout.isatty():
    green = red = yellow = cyan = dim = bold = lambda s: s


def log(msg, color=None):
    ts = time.strftime("%H:%M:%S")
    prefix = f"[{ts}]"
    if color:
        print(f"  {color(prefix)} {msg}")
    else:
        print(f"  {prefix} {msg}")


def check_port(host="127.0.0.1", port=80, timeout=2):
    """Check if a port is reachable."""
    import socket
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except (socket.timeout, ConnectionRefusedError, OSError):
        return False


def wait_for_port(port, timeout=STARTUP_TIMEOUT, label="service"):
    """Wait until a port becomes reachable."""
    t0 = time.time()
    while time.time() - t0 < timeout:
        if check_port(port=port):
            return True
        time.sleep(0.5)
    return False


def find_python():
    """Find a working Python interpreter."""
    # 1. System python
    for name in ("python", "python3"):
        try:
            r = subprocess.run([name, "--version"], capture_output=True, text=True, timeout=5,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
            if r.returncode == 0:
                return name
        except Exception:
            pass
    # 2. Portable uv
    uv_exe = os.path.join(APP_DIR, "uv", "uv.exe")
    if os.path.exists(uv_exe):
        return f'"{uv_exe}" run'
    # 3. Portable python
    py_exe = os.path.join(APP_DIR, "python", "python.exe")
    if os.path.exists(py_exe):
        return f'"{py_exe}"'
    return None


def find_npm():
    """Find a working npm."""
    # 1. System npm
    try:
        r = subprocess.run(["npm", "--version"], capture_output=True, text=True, timeout=5)
        if r.returncode == 0:
            return "npm"
    except Exception:
        pass
    # 2. System npx (npm is usually alongside)
    try:
        r = subprocess.run(["npx", "--version"], capture_output=True, text=True, timeout=5)
        if r.returncode == 0:
            return "npm"
    except Exception:
        pass
    # 3. Portable node
    node_dir = os.path.join(APP_DIR, "nodejs")
    npm_cmd = os.path.join(node_dir, "npm.cmd")
    if os.path.exists(npm_cmd):
        return f'"{npm_cmd}"'
    # 4. Search common locations
    for candidate in os.environ.get("PATH", "").split(os.pathsep):
        npm_path = os.path.join(candidate, "npm.cmd")
        if os.path.exists(npm_path):
            return f'"{npm_path}"'
    return None


def ensure_frontend_deps():
    """Install npm dependencies if node_modules is missing."""
    node_modules = os.path.join(UI_DIR, "node_modules")
    if os.path.isdir(node_modules):
        return True
    npm = find_npm()
    if not npm:
        log("npm not found, cannot install frontend deps", red)
        return False
    log("Installing frontend dependencies (first run)...", yellow)
    try:
        r = subprocess.run(f'{npm} install', shell=True, cwd=UI_DIR, capture_output=True, text=True, timeout=300,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        if r.returncode == 0:
            log("Frontend dependencies installed", green)
            return True
        log(f"npm install failed: {r.stderr[:200]}", red)
        return False
    except Exception as e:
        log(f"npm install error: {e}", red)
        return False


def ensure_backend_deps():
    """Install Python backend dependencies if needed."""
    py = find_python()
    if not py:
        return False
    try:
        r = subprocess.run(f'{py} -c "import fastapi,uvicorn"', shell=True, capture_output=True, text=True, timeout=10,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        if r.returncode == 0:
            return True
        log("Installing backend dependencies...", yellow)
        r = subprocess.run(f'{py} -m pip install fastapi uvicorn python-multipart sse-starlette pydantic-settings --quiet',
                           shell=True, capture_output=True, text=True, timeout=120,
                           creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        return r.returncode == 0
    except Exception:
        return False


def start_backend(py_cmd):
    """Start the FastAPI backend as a subprocess."""
    cmd = f'{py_cmd} api_main.py --dev --port {BACKEND_PORT}'
    log(f"Starting backend on port {BACKEND_PORT}...", cyan)
    proc = subprocess.Popen(
        cmd, shell=True, cwd=API_DIR,
    )
    return proc


def start_frontend(npm_cmd):
    """Start the Vite frontend as a subprocess."""
    cmd = f'{npm_cmd} run dev'
    log(f"Starting frontend on port {FRONTEND_PORT}...", cyan)
    proc = subprocess.Popen(
        cmd, shell=True, cwd=UI_DIR,
    )
    return proc


def _stream_output(proc, prefix, color_fn):
    """Stream subprocess output to console."""
    try:
        for line in iter(proc.stdout.readline, b""):
            text = line.decode("utf-8", errors="replace").rstrip()
            if text:
                print(f"  {color_fn(f'[{prefix}]')} {dim(text)}")
    except Exception:
        pass


def main():
    print()
    print(bold("  ============================================"))
    print(bold("    YunJi SmartIDE v2.0 - Launcher"))
    print(bold("  ============================================"))
    print()

    # ── Step 1: Find runtimes ──
    log("Checking environment...", cyan)

    py = find_python()
    if py:
        log(f"Python: {py}", green)
    else:
        log("No Python found! Please install Python or uv first.", red)
        log("Run: 一键环境维护.bat", yellow)
        try:
            input("\n  Press Enter to exit...")
        except EOFError:
            pass
        sys.exit(1)

    npm = find_npm()
    if npm:
        log(f"npm: {npm}", green)
    else:
        log("No npm/Node.js found! Please install Node.js first.", red)
        log("Run: 一键环境维护.bat", yellow)
        try:
            input("\n  Press Enter to exit...")
        except EOFError:
            pass
        sys.exit(1)

    # ── Step 2: Ensure dependencies ──
    log("Checking dependencies...", cyan)
    ensure_backend_deps()
    ensure_frontend_deps()

    # ── Step 3: Start services ──
    print()
    log("Starting services...", cyan)

    backend_proc = start_backend(py)
    frontend_proc = start_frontend(npm)

    # ── Step 4: Wait for services ──
    log(f"Waiting for backend (port {BACKEND_PORT})...", yellow)
    if wait_for_port(BACKEND_PORT, timeout=STARTUP_TIMEOUT):
        log(f"Backend is ready!", green)
    else:
        log(f"Backend did not start within {STARTUP_TIMEOUT}s", red)

    log(f"Waiting for frontend (port {FRONTEND_PORT})...", yellow)
    if wait_for_port(FRONTEND_PORT, timeout=STARTUP_TIMEOUT):
        log(f"Frontend is ready!", green)
    else:
        log(f"Frontend did not start within {STARTUP_TIMEOUT}s", red)

    # ── Step 5: Open browser ──
    time.sleep(1)
    url = f"http://localhost:{FRONTEND_PORT}"
    log(f"Opening browser: {url}", cyan)
    webbrowser.open(url)

    print()
    print(bold("  ============================================"))
    print(bold(green("    YunJi v2.0 is running!")))
    print(bold("  ============================================"))
    print(f"    Frontend:  http://localhost:{FRONTEND_PORT}")
    print(f"    Backend:   http://127.0.0.1:{BACKEND_PORT}/docs")
    print()
    print("    Press Ctrl+C to stop all services")
    print()

    # ── Step 6: Keep alive ──
    try:
        while True:
            # Check if processes are still alive
            if backend_proc.poll() is not None:
                log("Backend process exited unexpectedly!", red)
                break
            if frontend_proc.poll() is not None:
                log("Frontend process exited unexpectedly!", red)
                break
            time.sleep(1)
    except KeyboardInterrupt:
        print()
        log("Shutting down...", yellow)

    # Cleanup
    for proc in [backend_proc, frontend_proc]:
        try:
            proc.terminate()
            proc.wait(timeout=5)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass

    log("All services stopped.", green)
    print()


if __name__ == "__main__":
    main()
