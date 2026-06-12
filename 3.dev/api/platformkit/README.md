# PlatformKit - 云集智能编程工作站共享核心层

> ⚠️ **重要变更（2026-06-08）**: 本目录原计划名为 `platform/`，但 Python 3.12 标准库有一个 `platform` 模块，**包名冲突会导致整个项目无法启动**（`webview` 失败：`ImportError: cannot import name 'python_implementation' from 'platform'`）。已重命名为 `platformkit/`。详见 [AGENTS.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md)。

---

## 一、定位

PlatformKit 是项目"**跨进程共享的领域核心**"——

| 组件 | 说明 |
|------|------|
| **FastAPI app**（api_main.py） | 当前主用入口 |
| **远程 daemon**（Phase 4 计划） | 独立进程，局域网/WAN 访问 |
| **子应用**（qwen2api / zhipu2api） | 独立 FastAPI 应用，挂载在 `/api/qwen`、`/api/zhipu` |

平台核心（PlatformKit）= 这些进程**共享**的代码：
- 不依赖 FastAPI（除 types 之外）
- 不依赖 webview / PyQt6
- 不依赖 subprocess / CLI
- **可被多个进程同时调用**（无状态/有锁状态）

---

## 二、目录结构

```
dev/app/platformkit/                    ← ⚠️ 注意是 platformkit 不是 platform
├── __init__.py                          # 包入口
├── README.md                            # 本文档
├── shared/                              # 跨进程共享的领域核心
│   ├── __init__.py                      # 统一导出
│   ├── constants.py                     # ✅ 常量（re-export 自 backend.py）
│   ├── config_core.py                   # ✅ EnvFileManager（re-export 自 backend.py）
│   ├── types.py                         # ⏳ 跨服务 Pydantic 模型（W2 占位）
│   ├── workspace_core.py                # ⏳ 项目/工作区核心（W3 计划）
│   └── knowledge_core.py                # ⏳ 知识系统核心（Phase 3 W9 计划）
└── app/                                 # FastAPI 应用层（占位）
    └── __init__.py                      # 未来放 platform/app/services/ + platform/app/routes/
```

---

## 三、当前可用 API

```python
# 统一从 platformkit.shared 导入
from platformkit.shared import (
    # 常量
    MODEL_KEYS,                # 哪些 .env 键是模型名
    SETTINGS_KEYS,             # 哪些 .env 键是用户设置
    OLLAMA_AGENT_MAX_STEPS,    # Ollama agent 最大步数
    TOOL_TEXT_LIMIT,           # 工具文本截断上限
    COMMAND_OUTPUT_LIMIT,      # 命令输出截断上限
    TOOL_CALLING_MODEL_PATTERNS,  # 支持工具调用的模型模式
    # 类
    EnvFileManager,            # .env 读写核心
)
```

---

## 四、设计原则

### 4.1 零重复定义（Single Source of Truth）

platformkit.shared.**X 永远 re-export 自 backend.py**，**禁止**在新模块中重新定义。
- ✅ `platformkit.shared.MODEL_KEYS is backend.MODEL_KEYS`（同一对象）
- ❌ `MODEL_KEYS = [...]` 在 platformkit 里独立定义（双源漂移）

未来真正独立时：
1. W4 末：所有调用方改用 `from platformkit.shared import X`
2. W5+：删除 backend.py 中的原始定义

### 4.2 跨进程无状态

platformkit.shared 的内容**不应该维护任何进程级状态**。如需要状态：
- 应用特定状态 → `platformkit/app/`
- 进程级状态（如 CLI 子进程）→ 留在 services/ 中

### 4.3 不破坏旧调用

任何时刻，**所有旧的 `from backend import X` 调用都必须继续工作**。
- 平台层是"加一层"，不是"替换一层"

---

## 五、为什么用 importlib.util 直接加载

**问题**：dev/app 同时有：
- `dev/app/backend.py`（主后端，PyQt6 启动器用）
- `dev/app/api/qwen2api/backend/`（子应用，被 api_main 用）

`import backend` 在 api_main 上下文里会**先匹配到子应用的 backend**（因为 sys.path 顺序）。

**解决**：platformkit/shared 使用 `importlib.util.spec_from_file_location` 按**绝对路径**加载 `dev/app/backend.py`，并注册为 `sys.modules['yunji_main_backend']`，与子应用隔离。

**好处**：
- ✅ 100% 确定加载的是主 backend
- ✅ 与子应用无冲突
- ✅ 未来 daemon 进程启动时也能用同一段代码

---

## 六、迁移进度（TASK-1.3）

| 文件 | 状态 | 来源 |
|------|------|------|
| `platformkit/shared/constants.py` | ✅ 完成 | re-export 自 [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) L35-L81 |
| `platformkit/shared/config_core.py` | ✅ 完成 | re-export 自 [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) L84-L150 (EnvFileManager) |
| `platformkit/shared/types.py` | ⏳ 占位 | 实际类型 W2 迁移 |
| `platformkit/shared/workspace_core.py` | ⏳ 计划中 | W3 计划 |
| `platformkit/shared/knowledge_core.py` | ⏳ 计划中 | Phase 3 W9 计划 |
| `platformkit/app/` | ⏳ 占位 | 未来放 services/ + routes/ |

---

## 七、备份与回滚

> 本次操作前已备份到 [BAK/app_pre_platform_20260608/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/BAK/app_pre_platform_20260608)

如需回滚：
```bash
rm -rf dev/app/platformkit
cp -r BAK/app_pre_platform_20260608/* dev/app/
```

---

## 八、相关文档

- [AGENTS.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md) — AI 协作者契约（包含 platformkit 命名原因）
- [doc/codebase-map.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/codebase-map.md) — 任务地图
- [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) — TASK-1.3 详细说明
