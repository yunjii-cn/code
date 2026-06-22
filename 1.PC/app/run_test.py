"""调试 main.py 启动"""
import sys
import os
import traceback
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    print("[TEST] Starting main.py...")
    # 直接 import 会执行模块级代码但不运行 main()
    import main as m
    print(f"[TEST] Import OK, VERSION={m.VERSION}")
    print("[TEST] Calling main()...")
    m.main()
except SystemExit as e:
    print(f"[TEST] SystemExit: {e.code}")
except Exception as e:
    print(f"[TEST] ERROR: {type(e).__name__}: {e}")
    traceback.print_exc()
