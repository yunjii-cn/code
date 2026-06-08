#!/usr/bin/env python3
"""
云集智能编程工作站 - FastAPI + pywebview 统一入口

架构:
- FastAPI 提供 API 服务
- uvicorn 运行 HTTP 服务器
- pywebview 加载前端界面
- 支持 --dev / --lan / --port 命令行参数
"""

import sys
import os
import argparse
import threading
import secrets
import ctypes

_is_dev_mode = '--dev' in sys.argv

if not _is_dev_mode:
    import webview

    if sys.platform == 'win32':
        _glm = sys.modules['webview.guilib']
        _orig_guilib_initialize = _glm.initialize
        def _patched_initialize(forced_gui=None):
            if forced_gui == 'edgechromium':
                from webview.platforms import edgechromium as _ec
                from webview.platforms import winforms as _wf
                _glm.guilib = _ec
                _glm.forced_gui_ = 'edgechromium'
                _wf.setup_app()
                return _ec
            return _orig_guilib_initialize(forced_gui)
        _glm.initialize = _patched_initialize

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
import uvicorn
import importlib

if hasattr(sys, 'frozen'):
    _dev_dir = os.environ.get("YUNJI_DEV_DIR", os.path.dirname(sys.executable))
    _temp_pycache = os.path.join(_dev_dir, "temp", "__pycache__")
else:
    _script_dir = os.path.dirname(os.path.abspath(__file__))
    _temp_pycache = os.path.join(os.path.dirname(_script_dir), "temp", "__pycache__")
os.makedirs(_temp_pycache, exist_ok=True)
os.environ["PYTHONPYCACHEPREFIX"] = _temp_pycache

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"

VERSION = "2026.05.25.0617"

app = FastAPI(title="云集智能编程工作站", version=VERSION)

LAN_TOKEN: str | None = None

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.middleware("http")
async def lan_auth_middleware(request, call_next):
    global LAN_TOKEN
    if LAN_TOKEN is None:
        return await call_next(request)
    client_host = request.client.host if request.client else ""
    if client_host in ("127.0.0.1", "::1", "localhost"):
        return await call_next(request)
    token = request.query_params.get("token") or request.headers.get("X-LAN-Token", "")
    if token != LAN_TOKEN:
        from fastapi.responses import JSONResponse
        return JSONResponse(status_code=401, content={"detail": "未授权访问，请输入正确的配对码"})
    return await call_next(request)

_routes_modules = [
    "routes.ai", "routes.env", "routes.project",
    "routes.system", "routes.version", "routes.ws",
    "routes.github", "routes.knowledge", "routes.responsive",
    "routes.agent", "routes.team",
]
for _mod_name in _routes_modules:
    try:
        _mod = importlib.import_module(_mod_name)
        _router = getattr(_mod, "router", None)
        if _router is not None:
            app.include_router(_router)
    except Exception as e:
        print(f"[API] 注册路由 {_mod_name} 失败: {e}")


def _mount_sub_apps():
    _api_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "api")

    _qwen_dir = os.path.join(_api_dir, "qwen2api")
    if os.path.isdir(_qwen_dir):
        try:
            os.environ.setdefault("QWEN_DATA_DIR", os.path.join(_qwen_dir, "data"))
            _qwen_sys_path = os.path.join(_qwen_dir)
            if _qwen_sys_path not in sys.path:
                sys.path.insert(0, _qwen_sys_path)
            from backend.main import app as qwen_app
            app.mount("/api/qwen", qwen_app)
            print("[API] qwen2api 子应用已挂载于 /api/qwen")
        except Exception as e:
            print(f"[API] 挂载 qwen2api 失败: {e}")

    _zhipu_dir = os.path.join(_api_dir, "zhipu2api")
    if os.path.isdir(_zhipu_dir):
        try:
            os.environ.setdefault("ZHIPU_DATA_DIR", os.path.join(_zhipu_dir, "data"))
            _zhipu_sys_path = _zhipu_dir
            if _zhipu_sys_path not in sys.path:
                sys.path.insert(0, _zhipu_sys_path)
            from main import app as zhipu_app
            app.mount("/api/zhipu", zhipu_app)
            print("[API] zhipu2api 子应用已挂载于 /api/zhipu")
        except Exception as e:
            print(f"[API] 挂载 zhipu2api 失败: {e}")

_mount_sub_apps()


@app.get("/api/health")
async def health_check():
    return {"status": "ok", "version": VERSION}


@app.get("/api/lan-info")
async def lan_info():
    import socket
    hostname = socket.gethostname()
    local_ip = socket.gethostbyname(hostname)
    return {
        "lan_mode": LAN_TOKEN is not None,
        "token": LAN_TOKEN,
        "ip": local_ip,
        "port": _current_port,
    }


_current_port = 18080


if hasattr(sys, 'frozen'):
    _base = os.path.dirname(sys.executable)
    _static_dir = os.path.join(_base, "static")
    if not os.path.isdir(_static_dir):
        _static_dir = os.path.join(sys._MEIPASS, "static")
elif os.environ.get("YUNJI_STATIC_DIR"):
    _static_dir = os.environ["YUNJI_STATIC_DIR"]
else:
    _static_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "web", "dist")

if os.path.isdir(_static_dir):
    app.mount("/", StaticFiles(directory=_static_dir, html=True), name="frontend")
else:
    print(f"[API] 未找到前端构建目录: {_static_dir}")


def parse_args():
    parser = argparse.ArgumentParser(description="云集智能编程工作站")
    parser.add_argument("--dev", action="store_true", help="开发模式：不启动 pywebview，仅启动 FastAPI")
    parser.add_argument("--lan", action="store_true", help="局域网模式：监听 0.0.0.0")
    parser.add_argument("--port", type=int, default=18080, help="自定义端口（默认 18080）")
    return parser.parse_args()


def main():
    global LAN_TOKEN, _current_port
    args = parse_args()

    host = "0.0.0.0" if args.lan else "127.0.0.1"
    port = args.port
    _current_port = port

    if args.lan:
        LAN_TOKEN = secrets.token_hex(3).upper()
        import socket
        hostname = socket.gethostname()
        local_ip = socket.gethostbyname(hostname)
        print(f"[API] 🔑 局域网配对码: {LAN_TOKEN}")
        print(f"[API] 📱 手机访问: http://{local_ip}:{port}?token={LAN_TOKEN}")

    print(f"[API] FastAPI 服务启动于 http://{host}:{port}")

    server_thread = threading.Thread(
        target=uvicorn.run,
        kwargs={
            "app": app,
            "host": host,
            "port": port,
            "log_level": "info",
        },
        daemon=True,
    )
    server_thread.start()

    if args.dev:
        print("[API] 开发模式：Vite 代理 /api → FastAPI，pywebview 未启动")
        print(f"[API] API 文档: http://{host}:{port}/docs")
        try:
            server_thread.join()
        except KeyboardInterrupt:
            print("[API] 收到中断信号，正在退出...")
    else:
        print("[API] 正在启动 pywebview 窗口...")

        if sys.platform == 'win32':
            _user32 = ctypes.windll.user32
            class _RECT(ctypes.Structure):
                _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long),
                            ("right", ctypes.c_long), ("bottom", ctypes.c_long)]
            _wa = _RECT()
            _user32.SystemParametersInfoW(0x30, 0, ctypes.byref(_wa), 0)
            _ww, _wh = 1200, 800
            _x = (_wa.right - _wa.left - _ww) // 2 + _wa.left
            _y = (_wa.bottom - _wa.top - _wh) // 2 + _wa.top
        else:
            _ww, _wh, _x, _y = 1200, 800, None, None

        def _on_window_ready(win):
            import time
            time.sleep(0.5)
            win.focus()
            time.sleep(0.3)
            win.on_top = False

        window = webview.create_window(
            title=f"云集智能编程工作站 v{VERSION}",
            url=f"http://127.0.0.1:{port}",
            width=_ww,
            height=_wh,
            x=_x,
            y=_y,
            min_size=(360, 600),
            on_top=True,
        )
        webview.start(func=_on_window_ready, args=(window,), gui='edgechromium')


if __name__ == "__main__":
    main()
