"""Platform App - FastAPI 应用层（占位）

未来这里会放：
    - routes/    （从 dev/app/routes/ 迁移过来）
    - services/  （从 dev/app/services/ 迁移过来）

迁移策略:
    - 路由层（routes/）: 直接平移，业务逻辑保持不变
    - 服务层（services/）:
        - 跨服务共享的核心逻辑 → platform/shared/
        - 应用特定的服务 → platform/app/services/

当前占位: 实际迁移从 W2-W3 开始

历史来源:
    - 2026-06-08 占位创建（TASK-1.3）
"""
