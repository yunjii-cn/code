"""Platform Shared Config Core - .env 读写核心

EnvFileManager 是项目唯一的 .env 读写入口。

**使用规则**:
    - ✅ 新代码: `from platformkit.shared import EnvFileManager`
    - ✅ 旧代码: `from backend import EnvFileManager`（仍工作）
    - ❌ 禁止: 在本模块中重新定义 EnvFileManager（会双源漂移）

**为什么 re-export 而不是直接定义**:
    1. **零风险迁移**：backend.py 完全不变，100% 向后兼容
    2. **单一权威源**：未来真正独立时，删 backend.py 定义即可
    3. **同时支持 app 和未来 daemon**

**历史来源**:
    - 2026-06-08 从 dev/app/backend.py L84-L150 提取（TASK-1.3）
    - 旧位置: backend.py 顶层类
    - 兼容期: 至少 4 周
"""

import importlib.util
import sys
from pathlib import Path

# ── 加载主 backend.py（按绝对路径，避开 sys.path 冲突）──
# 本文件位于: dev/app/platformkit/shared/config_core.py
_HERE = Path(__file__).resolve().parent
_BACKEND_PATH = _HERE.parent.parent / "backend.py"


def _load_main_backend():
    """按绝对路径加载 dev/app/backend.py，避开 sys.path 冲突

    共享 sys.modules['yunji_main_backend']，避免重复加载。
    """
    module_name = "yunji_main_backend"
    if module_name in sys.modules:
        return sys.modules[module_name]
    spec = importlib.util.spec_from_file_location(module_name, str(_BACKEND_PATH))
    if spec is None or spec.loader is None:
        raise ImportError(f"Cannot load backend spec from {_BACKEND_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


_backend = _load_main_backend()

# Re-export 模式：让 platformkit.shared.EnvFileManager === backend.EnvFileManager
EnvFileManager = _backend.EnvFileManager

__all__ = ["EnvFileManager"]
