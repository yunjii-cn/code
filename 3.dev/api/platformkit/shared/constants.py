"""Platform Shared Constants - 常量定义

这些常量是**项目所有跨服务共享的常量**。本模块通过 `importlib.util` 直接加载
`dev/app/backend.py`，避免与子应用 `dev/app/api/qwen2api/backend/` 冲突。

**使用规则**:
    - ✅ 新代码: `from platformkit.shared import MODEL_KEYS, SETTINGS_KEYS`
    - ✅ 旧代码: `from backend import MODEL_KEYS, SETTINGS_KEYS`（仍工作）
    - ❌ 禁止: 在 platformkit/shared/constants.py 中重新定义（会双源漂移）

**为什么用 importlib 直接加载**:
    Python sys.path 中既有 `dev/app/api/qwen2api/backend/` 又有 `dev/app/backend.py`，
    `import backend` 会先匹配到子应用。所以这里用 importlib.util 按绝对路径加载主 backend。

**未来迁移路径**:
    1. Phase 1 W1 末：创建本模块，re-export 旧位置（当前）
    2. Phase 1 W4 末：所有调用方改用 `from platformkit.shared import X`
    3. Phase 2 W5+：删除 backend.py 中的原始定义，本模块变为唯一权威源

**为什么用 re-export**:
    1. **零风险迁移**：backend.py 完全不变，100% 向后兼容
    2. **单一权威源**：避免双源定义导致常量值漂移
    3. **同时支持 app 和未来 daemon**：daemon 可 import platformkit.shared
    4. **渐进式迁移**：不需要一次性改完所有调用方

**历史来源**:
    - 2026-06-08 从 dev/app/backend.py 提取（TASK-1.3）
    - 旧位置: backend.py 顶层模块变量
    - 兼容期: 至少 4 周
"""

import importlib.util
import os
import sys
from pathlib import Path

# ── 加载主 backend.py（按绝对路径）──
# 本文件位于: dev/app/platformkit/shared/constants.py
# 需要加载:   dev/app/backend.py
_HERE = Path(__file__).resolve().parent
_BACKEND_PATH = _HERE.parent.parent / "backend.py"

if not _BACKEND_PATH.exists():
    raise ImportError(f"Cannot find backend.py at {_BACKEND_PATH}")


def _load_main_backend():
    """按绝对路径加载 dev/app/backend.py，避开 sys.path 冲突

    同时把模块注册到 sys.modules['yunji_main_backend']，
    避免重复加载（确保 platformkit.shared.X 指向同一对象）。
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

# Re-export 模式：让 platformkit.shared.X 与 backend.X 指向同一对象
MODEL_KEYS = _backend.MODEL_KEYS
SETTINGS_KEYS = _backend.SETTINGS_KEYS
OLLAMA_AGENT_MAX_STEPS = _backend.OLLAMA_AGENT_MAX_STEPS
TOOL_TEXT_LIMIT = _backend.TOOL_TEXT_LIMIT
COMMAND_OUTPUT_LIMIT = _backend.COMMAND_OUTPUT_LIMIT
TOOL_CALLING_MODEL_PATTERNS = _backend.TOOL_CALLING_MODEL_PATTERNS

__all__ = [
    "MODEL_KEYS",
    "SETTINGS_KEYS",
    "OLLAMA_AGENT_MAX_STEPS",
    "TOOL_TEXT_LIMIT",
    "COMMAND_OUTPUT_LIMIT",
    "TOOL_CALLING_MODEL_PATTERNS",
]
