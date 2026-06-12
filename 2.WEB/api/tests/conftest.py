"""pytest 配置

2026-06-08 TASK-1.6 引入
"""
import sys
from pathlib import Path

# 把 dev/app 加入 sys.path，让 tests 能 import services/platformkit/routes
_BACKEND_ROOT = Path(__file__).resolve().parent.parent
if str(_BACKEND_ROOT) not in sys.path:
    sys.path.insert(0, str(_BACKEND_ROOT))
