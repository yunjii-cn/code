"""Platform Shared Model Utils - 模型数据规范化工具

跨进程共享的纯模型处理工具，**无状态 / 无副作用**。

**使用规则**:
    - ✅ 新代码: `from platformkit.shared import normalize_model_entries, guess_tool_support_by_name`
    - ✅ 旧代码: `from backend import normalize_model_entries, guess_tool_support_by_name`（仍工作）
    - ❌ 禁止: 在本模块中重新定义（避免双源漂移）

**历史来源**:
    - 2026-06-08 从 dev/app/backend.py L182-L199 提取（TASK-1.3 Phase 1 收尾）
    - 旧位置: backend.py 顶层函数
    - 兼容期: 至少 4 周
"""

import importlib.util
import os
import sys
from pathlib import Path

# ── 加载主 backend.py（按绝对路径，避开 sys.path 冲突）──
_HERE = Path(__file__).resolve().parent
_BACKEND_PATH = _HERE.parent.parent / "backend.py"


def _load_main_backend():
    """按绝对路径加载 dev/app/backend.py，避开 sys.path 冲突

    共享 sys.modules['yunji_main_backend']，避免重复加载。
    """
    module_name = "yunji_main_backend"
    if module_name in sys.modules:
        return sys.modules[module_name]
    if not _BACKEND_PATH.exists():
        raise ImportError(f"Cannot find backend.py at {_BACKEND_PATH}")
    spec = importlib.util.spec_from_file_location(module_name, str(_BACKEND_PATH))
    if spec is None or spec.loader is None:
        raise ImportError(f"Cannot load backend spec from {_BACKEND_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


_backend = _load_main_backend()

# Re-export 模式（保持与 backend.py 同一对象）
TOOL_CALLING_MODEL_PATTERNS = _backend.TOOL_CALLING_MODEL_PATTERNS


def normalize_model_entries(lst: list, provider: str) -> list:
    """规范化多供应商的模型列表为统一格式

    Args:
        lst: 原始模型列表（dict 数组）
        provider: 提供商标识（'ollama' / 'openrouter' / 'anthropic' 等）

    Returns:
        list: [{"id": str, "name": str, "provider": str}, ...]
    """
    if not isinstance(lst, list):
        return []
    result = []
    for item in lst[:120]:
        mid = str(item.get("id", "") or "").strip()
        if not mid:
            continue
        name = str(item.get("name", "") or item.get("display_name", "") or mid).strip()
        result.append({"id": mid, "name": name, "provider": provider})
    return result


def guess_tool_support_by_name(name: str) -> bool:
    """根据模型名称猜测是否支持 tool/function calling

    Args:
        name: 模型名称（如 "llama3.1:8b", "qwen2.5-coder:32b"）

    Returns:
        bool: True = 可能支持，False = 不支持
    """
    lower = (name or "").lower()
    if lower.endswith(":cloud"):
        return False
    return any(p in lower for p in TOOL_CALLING_MODEL_PATTERNS)


__all__ = [
    "normalize_model_entries",
    "guess_tool_support_by_name",
    "TOOL_CALLING_MODEL_PATTERNS",
]
