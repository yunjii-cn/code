# Codebase Map — 任务地图

> 云集智能编程工作站 — 任务导向文件导航
> 版本：v1.0 | 制定日期：2026-06-08
> 模板来源：[CodexMonitor codebase-map.md](https://github.com/Dimillian/CodexMonitor/blob/main/docs/codebase-map.md)

---

## 使用说明

**这是任务地图，不是 API 文档**。每个场景告诉你**"想改 X，去改 Y"**，不重复代码细节。

**使用方法**：
1. 接到任务后，先在本表找最接近的场景
2. 顺着 `→` 看修改链路
3. 读被引用的文件即可（每个文件都在 30-50 行内的可读状态）
4. 不确定时回看 [AGENTS.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md) 关键文件锚点节

**场景分类**：
- 🟢 AI 与模型（4 个）
- 🟣 前端 UI 与跨端（4 个）
- 🔵 API 与路由（3 个）
- 🟠 数据与持久化（5 个）
- 🟡 Git 与项目（2 个）
- 🔴 打包与发布（3 个）
- ⚪ 调试与诊断（3 个）

**总计 24 个场景，覆盖 80% 日常任务**。

---

## 🟢 AI 与模型

### 场景 1：加一个新的 AI 提供商（如 DeepSeek）

```
backend.py MODEL_KEYS/SETTINGS_KEYS
   ↓
services/ai_service.py 新增 Provider 类 + register_provider()
   ↓
routes/ai.py 新增 Pydantic 模型 + endpoint
   ↓
web/src/api/index.ts aiApi 新增方法
   ↓
web/src/stores/model.ts 添加 Provider 类型
   ↓
web/src/views/ModelsView.vue UI 展示
```

**关键文件**：
- [dev/app/services/ai_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) — 供应商注册中心
- [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) — 模型列表聚合（`list_*_models()` 系列）
- [dev/app/routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) — `/api/ai/*` 端点
- [dev/web/src/api/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) — 前端 aiApi 命名空间
- [dev/web/src/views/ModelsView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ModelsView.vue) — 模型管理 UI

---

### 场景 2：修改 AI 对话流式响应逻辑

```
前端发起: ChatView.vue → chatStore.sendMessage()
   ↓
platform layer: platform/{desktop,mobile}.ts → api/index.ts aiApi.chat()
   ↓
后端路由: routes/ai.py /ai/chat → ai_service.stream_chat()
   ↓
供应商适配: ai_service.py 根据 provider/model 选具体实现
   ↓
SSE 输出: routes/ai.py 用 sse_starlette.EventSourceResponse
```

**关键文件**：
- [dev/web/src/views/ChatView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ChatView.vue) — 主聊天页
- [dev/web/src/stores/chat.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/chat.ts) — 对话状态 + 取消控制
- [dev/web/src/platform/desktop.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/desktop.ts) / [mobile.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/mobile.ts) — 跨端平台层
- [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) — `stream_chat()` / `stop_chat()` 方法
- [dev/app/routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) — `ChatRequest` Pydantic 模型 + `/chat` 端点

---

### 场景 3：调整 Ollama 代理（格式转换 + 自动降级）

```
backend.py EnvFileManager + list_ollama_models()
   ↓
services/ai_service.py ollama provider 实现
   ↓
backend.py Ollama HTTP 代理（_OllamaProxyHandler / OllamaProxyServer）
```

**关键文件**：
- [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) — Ollama 代理核心（`http.server` / `OllamaProxyServer`）
- [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) — `list_ollama_models()` / `check_ollama_health()` / `get_ollama_capabilities()`
- [dev/app/routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) — `/api/ai/ollama/*` 端点

---

### 场景 4：加 Claude CLI 子进程参数或换工具

```
services/cli_service.py ClaudeCliRunner（subprocess + stream-json 解析）
   ↓
backend.py ClaudeCliRunner 实例化 + 流式读取
   ↓
services/ai_service.py 调用 cli_service 调 Claude
```

**关键文件**：
- [dev/app/services/cli_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/cli_service.py) — CLI 子进程管理
- [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) — Claude CLI 调用入口
- [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) — 路由到 CLI provider

---

## 🟣 前端 UI 与跨端

### 场景 5：加一个新页面

```
web/src/views/NewView.vue 创建组件
   ↓
web/src/router/index.ts 注册路由
   ↓
web/src/App.vue 添加导航入口（如需要）
```

**关键文件**：
- [dev/web/src/views/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/) — 14 个 view 文件
- [dev/web/src/router/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/router/index.ts) — 6 条已注册路由
- [dev/web/src/App.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/App.vue) — 根组件（导航/布局）
- [dev/web/src/composables/useDevice.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/composables/useDevice.ts) — 设备检测

> ⚠️ 当前 14 个 view 只注册了 6 个路由。新页面建议先合并到现有 view，避免增加未用文件

---

### 场景 6：修改现有页面布局

```
对应 view 文件直接修改
   ↓
如需全局布局 → App.vue
   ↓
如需跨端差异化 → platform/{desktop,mobile}.ts
```

**当前 view 与路由对应**：
| View | 路由 | 主题 |
|------|------|------|
| [ChatView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ChatView.vue) | `/` | 主页（最复杂） |
| [ModelsView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ModelsView.vue) | `/models` | 模型管理 |
| [ProjectView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ProjectView.vue) | `/projects` | 项目管理 |
| [EnvView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/EnvView.vue) | `/env` | 环境部署 |
| [SettingsView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/SettingsView.vue) | `/settings` | 系统设置 |
| [VersionView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/VersionView.vue) | `/version` | 版本更新 |

---

### 场景 7：加一个 Pinia 全局状态 store

```
web/src/stores/new_store.ts 创建 store
   ↓
在需要用的 view 中 import 并 useStore()
```

**已有 store**（4 个）：
- [dev/web/src/stores/chat.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/chat.ts) — 对话状态 + 流式控制
- [dev/web/src/stores/model.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/model.ts) — 供应商 + 模型
- [dev/web/src/stores/project.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/project.ts) — 项目状态
- [dev/web/src/stores/settings.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/settings.ts) — 系统设置

---

### 场景 8：加跨端差异（桌面/手机不同行为）

```
web/src/platform/index.ts 导出新方法
   ↓
web/src/platform/desktop.ts 实现桌面端逻辑
   ↓
web/src/platform/mobile.ts 实现移动端逻辑
   ↓
在 view 中 import { newMethod } from '@/platform'
```

**关键文件**：
- [dev/web/src/platform/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/index.ts) — 平台检测 + 方法 re-export
- [dev/web/src/platform/desktop.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/desktop.ts) — 桌面实现（HTTP + WebSocket）
- [dev/web/src/platform/mobile.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/mobile.ts) — 移动实现（Capacitor）

---

## 🔵 API 与路由

### 场景 9：加一个新的 API 端点

```
api/routes/new_module.py 创建路由文件
   ↓
api_main.py _routes_modules 列表添加新模块
   ↓
services/ 创建对应服务类
   ↓
api/index.ts 添加 api 调用方法（web 端）
```

**模板**（参考 [routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py)）：
```python
from fastapi import APIRouter, Query
from pydantic import BaseModel, Field
from typing import Optional

from services.xxx_service import XxxService

router = APIRouter(prefix="/api/xxx", tags=["xxx模块"])
xxx_service = XxxService()


class XxxRequest(BaseModel):
    field: str = Field(..., description="...")


@router.get("/endpoint")
async def get_xxx():
    return await xxx_service.method()
```

**关键文件**：
- [dev/app/api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) — 路由注册中心（`_routes_modules` 列表，L85-L96）
- [dev/app/routes/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/) — 现有 6 个路由模块

---

### 场景 10：修改 SSE 流式响应

```
routes/ai.py /chat 端点
   ↓
sse_starlette.EventSourceResponse
   ↓
event_generator 异步生成器
   ↓
service 层的 stream_chat() 产生 chunk
```

**关键文件**：
- [dev/app/routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) — `EventSourceResponse` 用法（L41-L47）
- [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) — `stream_chat()` 实现
- [dev/web/src/composables/useStream.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/composables/useStream.ts) — 前端流式接收封装
- [dev/web/src/stores/chat.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/chat.ts) — 取消控制（AbortController）

---

### 场景 11：改鉴权/中间件

```
api_main.py lan_auth_middleware (L71-L83)
   ↓
CORS 中间件 (L62-L68)
   ↓
LAN_TOKEN 全局变量（L60, L177-L190）
```

**关键文件**：
- [dev/app/api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) — 中间件 + Token 生成

---

## 🟠 数据与持久化

### 场景 12：加一个新的配置项到 .env

```
backend.py SETTINGS_KEYS 列表添加新键
   ↓
backend.py EnvFileManager 读写逻辑（自动支持）
   ↓
services/config_service.py 业务方法（保存/读取）
   ↓
routes/system.py 端点（/api/system/settings）
   ↓
web 端 stores/settings.ts 调用
```

**关键文件**：
- [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) — `SETTINGS_KEYS` 列表（L43-L64）
- [dev/app/services/config_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/config_service.py) — `get_settings()` / `save_settings()`
- [dev/app/routes/system.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/system.py) — `/api/system/settings/*` 端点

---

### 场景 13：修改用户数据存储

⚠️ **高风险**：多用户数据隔离是核心架构规则

```
services/project_service.py ProjectManager / ProjectService
   ↓
data/users/{user_id}/ 子目录布局
   ↓
旧数据迁移逻辑 _migrate_old_projects()
```

**关键文件**：
- [dev/app/services/project_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) — `ProjectManager` 类（L13-L100+）
- [dev/app/routes/project.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/project.py) — `/api/project/*` 端点

> 严禁修改 `data/public/` 目录结构（[AGENTS.md Non-Negotiable Rule #5](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md)）

---

### 场景 14：加一个新模板/插件

**模板**：
```
data/public/templates/ 添加新模板
   ↓
services/project_service.py list_templates() 自动包含
   ↓
routes/project.py /project/templates
```

**插件**：
```
data/public/plugins/ 添加新插件
   ↓
services/config_service.py list_plugins()
   ↓
routes/system.py /api/system/plugins
```

**关键文件**：
- [dev/app/services/project_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) — `list_templates()` / `save_custom_template()`
- [dev/app/services/config_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/config_service.py) — `list_plugins()` / `install_plugin()` / `execute_plugin()`

---

### 场景 15：修改会话/记忆存储

```
会话：
  services/project_service.py save_conversation / load_conversation
  data/users/{user_id}/sessions/{project_id}/conversations/
  routes/project.py /project/{id}/conversations

记忆（Memory）：
  services/project_service.py list_memories / save_memory
  data/users/{user_id}/sessions/{project_id}/memories/
  routes/project.py /project/{id}/memories
```

**关键文件**：
- [dev/app/services/project_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) — 会话/记忆方法
- [dev/app/routes/project.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/project.py) — `/api/project/{id}/conversations` + `/memories`
- [dev/web/src/views/MemoryView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/MemoryView.vue) — 记忆管理 UI（**未路由**，待评估）

---

### 场景 16：加一个新的"功能设置"（AI_LANGUAGE 等）

```
backend.py SETTINGS_KEYS 添加键
   ↓
services/config_service.py 业务方法（可选）
   ↓
routes/system.py /api/system/settings
   ↓
web/src/views/SettingsView.vue 表单新增字段
```

参考 [AGENTS.md Rule 5](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md) — 数据隔离

---

## 🟡 Git 与项目

### 场景 17：修改 Git 操作（status/commit/log）

```
routes/system.py /api/system/git/*
   ↓
services/config_service.py git_status / git_commit / git_log
   ↓
web/src/views/GitView.vue UI
   ↓
web/src/api/index.ts systemApi
```

**关键文件**：
- [dev/app/routes/system.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/system.py) — `/api/system/git/*` 端点
- [dev/app/services/config_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/config_service.py) — Git 方法
- [dev/web/src/views/GitView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/GitView.vue) — Git UI（**未路由**）

---

### 场景 18：修改项目注册表/项目元数据

```
services/project_service.py ProjectManager
   ↓
data/users/{user_id}/projects/registry.json
   ↓
routes/project.py /api/project/*
```

**关键文件**：
- [dev/app/services/project_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) — 注册表 + 迁移逻辑
- [dev/app/routes/project.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/project.py) — 项目 CRUD
- [dev/web/src/views/ProjectView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ProjectView.vue) — 项目管理 UI
- [dev/web/src/stores/project.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/project.ts) — 项目状态

---

## 🔴 打包与发布

### 场景 19：修改打包配置

```
build/build_web.py 主打包脚本
   ↓
WEBENGINE_FILES 列表（PyQt6 兼容）
   ↓
EXCLUDED_PATTERNS（清理不必要模块）
   ↓
DIST_DIR / VER_DIR 输出常量
```

**关键文件**：
- [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) — 当前主用打包脚本
- [build/build.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build.py) — **Legacy** PyQt6 打包
- [build/build_stub.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_stub.py) — Stub 启动器

> ⚠️ 改打包后必须 `python build/build_web.py --skip-frontend` 验证不破

---

### 场景 20：修改版本号

```
api_main.py VERSION 变量（L56）
   ↓
build/build_web.py patch_version() 注入
   ↓
dev/ver/version.json
```

**关键文件**：
- [dev/app/api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) — VERSION 字符串
- [dev/app/services/update_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/update_service.py) — GitHub + Gitee 双源更新
- [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) — `patch_version()` / `get_version()`
- [dev/app/routes/version.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/version.py) — `/api/version/*` 端点

---

### 场景 21：改远程/局域网模式

```
api_main.py parse_args() (L168-L173)
   ↓
LAN_TOKEN 鉴权中间件 (L71-L83)
   ↓
main() 启动逻辑 (L176-L246)
```

**关键文件**：
- [dev/app/api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) — 全部 LAN 逻辑
- [dev/web/src/composables/useDevice.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/composables/useDevice.ts) — 前端设备检测

> 改完后必须 `python dev/app/api_main.py --lan --port 18080` 测试手机访问

---

## ⚪ 调试与诊断

### 场景 22：调试 AI 对话不返回 / 报错

按以下顺序检查：

1. **前端** — 浏览器 DevTools → Network → 找 `/api/ai/chat` 请求
2. **后端** — Python 启动控制台看异常
3. **服务层** — [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) 的 `stream_chat()` 加日志
4. **供应商** — 检查 `.env` 中 `MODEL_PROVIDER` / `ANTHROPIC_API_KEY` / `OLLAMA_BASE_URL` 等
5. **CLI 子进程** — Claude CLI 调试看 `cli_debug.log`

**关键日志文件**：
- 后端 stdout（启动 API 时打印）
- `app/cli_debug.log`（[dev/.gitignore](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/.gitignore) 已忽略）

---

### 场景 23：调试 EXE 启动失败

按以下顺序检查：

1. **单实例** — 看任务管理器是否有旧实例
2. **品牌校验** — EXE 文件名必须含"云集智能编程工作站"（[launcher.py#L7-L37](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py#L7-L37)）
3. **DLL 加载** — 看 `dev/云集智能编程工作站-v*/_internal/` 是否有 `Qt6WebEngineCore.dll` 等
4. **pywebview** — 看控制台是否报 Edge WebView2 缺失
5. **静态文件** — 看 `dev/app/static/` 是否在（构建后才会有）

**关键文件**：
- [dev/app/launcher.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py) — EXE 入口
- [dev/app/main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) — Legacy 启动器
- [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) — 打包脚本（生成 `_internal/`）

---

### 场景 24：调试 WebSocket 断连

```
routes/ws.py /ws 端点
   ↓
services/ws.py WebSocket 服务
   ↓
web/src/composables/useWebSocket.ts
```

**关键文件**：
- [dev/app/routes/ws.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ws.py) — WebSocket 路由
- [dev/app/services/ws.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ws.py) — WebSocket 业务
- [dev/web/src/composables/useWebSocket.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/composables/useWebSocket.ts) — 前端 WS 客户端
- [dev/web/vite.config.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/vite.config.ts) — 代理配置（`/ws` → FastAPI）

---

## 数据流总览

```
[Vue 组件 / View]
     ↓ import { xxxApi } from '@/api'
[web/src/api/index.ts]  ←  axios 封装，5 个命名空间
     ↓
[web/src/platform/{desktop|mobile}.ts]  ←  跨端适配
     ↓ HTTP / WebSocket
[FastAPI api_main.py]
     ↓
[web/src/router/index.ts] 路由分发
     ↓
[routes/xxx.py]  ←  Pydantic 校验 + 调服务
     ↓
[services/xxx_service.py]  ←  业务逻辑
     ↓
[backend.py]  ←  Ollama 代理 / .env / CLI 进程 / 模型聚合
     ↓
[外部 API / 本地 Ollama / Claude CLI 子进程]
```

---

## 关键依赖关系

### 后端依赖图

```
api_main.py
   ├─ routes/ai.py → services/ai_service.py → backend.py
   ├─ routes/project.py → services/project_service.py
   ├─ routes/system.py → services/config_service.py
   ├─ routes/env.py → services/env_service.py
   ├─ routes/version.py → services/update_service.py
   └─ routes/ws.py → services/ws.py

backend.py（被多处依赖）
   ├─ EnvFileManager（.env 读写）
   ├─ ClaudeCliRunner（CLI 子进程）
   ├─ OllamaProxyServer（HTTP 代理）
   └─ list_*_models()（多供应商模型聚合）

api/qwen2api/ → 挂载到 /api/qwen
api/zhipu2api/ → 挂载到 /api/zhipu
```

### 前端依赖图

```
main.ts
   ├─ App.vue → router/index.ts → views/*.vue
   ├─ pinia → stores/*.ts
   └─ vant（UI 组件库）

views/*.vue
   ├─ components/*.vue
   ├─ composables/*.ts
   └─ stores/*.ts → platform/*.ts → api/index.ts（HTTP）
```

---

## 未路由 View 处置决策（待 Phase 1 末）

| View | 建议处置 |
|------|---------|
| [OllamaView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/OllamaView.vue) | 合并到 ModelsView |
| [ApiKeyView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ApiKeyView.vue) | 合并到 SettingsView |
| [ApiServiceView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ApiServiceView.vue) | 合并到 SettingsView |
| [FilesView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/FilesView.vue) | **加路由**（功能完整） |
| [GitView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/GitView.vue) | **加路由**（功能完整） |
| [TerminalView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/TerminalView.vue) | **加路由**（功能完整） |
| [MemoryView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/MemoryView.vue) | 合并到 ProjectView |
| [PluginView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/PluginView.vue) | 合并到 SettingsView |
| [ClaudeMdView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ClaudeMdView.vue) | 合并到 ProjectView |

---

## 子应用（独立维护，**不要动**）

| 子应用 | 路径 | 用途 |
|--------|------|------|
| qwen2api | [dev/app/api/qwen2api/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api/qwen2api/) | 千问账户池 + API 转换 |
| zhipu2api | [dev/app/api/zhipu2api/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api/zhipu2api/) | 智谱账户池 + API 转换 |

> 它们的代码是独立维护的 FastAPI 应用。修改请联系对应的子模块负责人

---

## Legacy 区域（**不要修改**）

| 区域 | 状态 |
|------|------|
| [dev/app/main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) | 旧 PyQt6 + QWebEngineView 启动器 |
| [dev/app/desktop/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/desktop/) | 旧 Electron 壳 |
| [dev/app/src/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/src/) | 旧 TS 源码（Claude CLI 适配残留） |
| [build/build.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build.py) | 旧打包脚本（PyQt6 + Electron） |

> 详见 [AGENTS.md Hotspots & Legacy 节](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md)

---

## 快速跳转表

| 想做什么 | 改哪里 |
|---------|--------|
| 改 AI 对话 | [services/ai_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) + [routes/ai.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) |
| 改主页 UI | [views/ChatView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ChatView.vue) + [stores/chat.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/chat.ts) |
| 改项目数据 | [services/project_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) |
| 改 Git 操作 | [services/config_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/config_service.py) + [routes/system.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/system.py) |
| 改打包 | [build/build_web.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) |
| 改启动器 | [api_main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) + [launcher.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py) |
| 改跨端行为 | [platform/desktop.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/desktop.ts) + [mobile.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/mobile.ts) |
| 改 API 协议 | [api/index.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) + 对应 [routes/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/) + [api_main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) |
| 改版本管理 | [api_main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) + [services/update_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/update_service.py) |
| 改更新源 | [services/update_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/update_service.py) — `GITEE_OWNER` / `GITHUB_OWNER` 常量 |
| 改环境部署 | [services/env_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/env_service.py) + [routes/env.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/env.py) |
| 改单实例 | [main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) + [doc/单实例控制机制-技术研究.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%8D%95%E5%AE%9E%E4%BE%8B%E6%8E%A7%E5%88%B6%E6%9C%BA%E5%88%B6-%E6%8A%80%E6%9C%AF%E7%A0%94%E7%A9%B6.md) |
| 改浏览器右键菜单（中文翻译） | [main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) — `MENU_TRANSLATIONS` 字典（L62-L104） |

---

## 文档链接

- [AGENTS.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/AGENTS.md) — AI 协作者契约
- [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) — 35 个 TASK 路线图

---

## 更新历史

| 日期 | 变更 |
|------|------|
| 2026-06-08 | v1.0 初版，24 个任务场景 |
