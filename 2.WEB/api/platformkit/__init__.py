"""PlatformKit - 云集智能编程工作站共享核心层

⚠️ 本包原计划名为 `platform`，因与 Python 标准库 `platform` 模块冲突
（导致 `webview` 启动失败），重命名为 `platformkit/`。
详见 platformkit/README.md 和 AGENTS.md。

包含:
    - platformkit.shared: 跨进程共享的领域核心
    - platformkit.app: FastAPI 应用层（占位）

迁移策略（详见 platformkit/README.md）:
    - W1 末：创建骨架 + 提取零依赖常量/类型（本次 TASK-1.3）
    - W2-W3：迁移有依赖的服务到 platformkit/app/services
    - W4 末：80% 业务逻辑在 platformkit/shared/ 或 platformkit/app/
    - 旧 routes/、services/ 用 re-export 兼容 4 周，逐步废弃

详细路线图:
    doc/全面超越-参考CodexMonitor改造计划.md - TASK-1.3
"""
