"""Platform Shared Core - 跨 app/daemon 共享的领域核心

包含:
    - constants: 常量定义（MODEL_KEYS, SETTINGS_KEYS 等）
    - config_core: .env 读写核心（EnvFileManager）
    - http_client: 纯 HTTP 工具（fetch_json_with_timeout）
    - model_utils: 模型数据规范化（normalize / guess_tool_support）
    - types: 跨端共享类型（Pydantic 模型 - TODO）

设计原则:
    - 不依赖 FastAPI（types 除外）
    - 可被 FastAPI app 和远程 daemon 同时调用
    - 业务逻辑应放在这里，路由层只做参数解析
    - **零重复定义**：本模块 re-export 自 backend.py，避免双源漂移

迁移进度:
    - ✅ constants.py        （W1 末：从 backend.py 提取）
    - ✅ config_core.py     （W1 末：从 backend.py 提取 EnvFileManager）
    - ✅ http_client.py     （Phase 1 收尾：fetch_json_with_timeout，**真提取**非 re-export）
    - ✅ model_utils.py     （Phase 1 收尾：normalize_model_entries + guess_tool_support_by_name）
    - ⏳ types.py            （W2：跨服务共享的 Pydantic 模型）
    - ⏳ workspace_core.py   （W3：项目/工作区核心）
    - ✅ knowledge_core.py  （W9 TASK-3.1：四层知识 + 强度演化）
    - ✅ durable_store.py  （W11 TASK-3.7：6 文件持久化 + AGENTS 读取）
    - ✅ team_core.py      （W12 TASK-3.9：5 角色 + 边界治理 + 工具可见性）

新代码导入方式:
    from platformkit.shared import MODEL_KEYS, EnvFileManager, fetch_json_with_timeout
    from platformkit.shared.knowledge_core import KnowledgeEngine, Knowledge, KnowledgeLayer

旧代码兼容:
    from backend import MODEL_KEYS, EnvFileManager, fetch_json_with_timeout  # 仍工作
"""

from .constants import (
    MODEL_KEYS,
    SETTINGS_KEYS,
    OLLAMA_AGENT_MAX_STEPS,
    TOOL_TEXT_LIMIT,
    COMMAND_OUTPUT_LIMIT,
    TOOL_CALLING_MODEL_PATTERNS,
)
from .config_core import EnvFileManager
from .http_client import fetch_json_with_timeout
from .model_utils import (
    normalize_model_entries,
    guess_tool_support_by_name,
)
from . import github_core
from . import git_core
from . import whisper_core
from .knowledge_core import (
    KnowledgeEngine,
    Knowledge,
    KnowledgeLayer,
    KnowledgeScope,
    KnowledgeStrength,
    STRENGTH_THRESHOLDS,
    STRENGTH_WEIGHTS,
)

__all__ = [
    "MODEL_KEYS",
    "SETTINGS_KEYS",
    "OLLAMA_AGENT_MAX_STEPS",
    "TOOL_TEXT_LIMIT",
    "COMMAND_OUTPUT_LIMIT",
    "TOOL_CALLING_MODEL_PATTERNS",
    "EnvFileManager",
    "fetch_json_with_timeout",
    "normalize_model_entries",
    "guess_tool_support_by_name",
    "github_core",
    "git_core",
    "whisper_core",
    # 2026-06-09 TASK-3.1 引入：自进化知识系统
    "KnowledgeEngine",
    "Knowledge",
    "KnowledgeLayer",
    "KnowledgeScope",
    "KnowledgeStrength",
    "STRENGTH_THRESHOLDS",
    "STRENGTH_WEIGHTS",
    # 2026-06-09 TASK-3.4 引入：主动感知引擎
    "ResponsiveEngine",
    "Notification",
    "NotificationAction",
    "NotificationType",
    "Severity",
]
