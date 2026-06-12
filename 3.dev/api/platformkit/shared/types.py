"""Platform Shared Types - 跨端共享类型

TODO (Phase 1 W2): 从 routes/*.py 提取共用的 Pydantic BaseModel
    - ChatMessage
    - ProjectMeta
    - GitStatus
    - FileNode
    - PluginManifest
    等等

**为什么需要这层**:
    - 未来 daemon 也需要这些类型
    - 避免 routes/ai.py 定义的 Pydantic 模型被 daemon 重复定义
    - 单一权威源，避免类型漂移

当前状态: 占位（确保模块可被 import 不报错）

历史来源:
    - 2026-06-08 占位创建（TASK-1.3）
    - 实际类型迁移到 W2 进行
"""
