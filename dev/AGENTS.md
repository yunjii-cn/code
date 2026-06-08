# AGENTS.md

> 云集智能编程工作站 — AI 协作者开发契约
> 版本：v1.0 | 制定日期：2026-06-08
> 模板来源：[CodexMonitor AGENTS.md](https://github.com/Dimillian/CodexMonitor/blob/main/AGENTS.md)

---

## Scope（范围）

本文件是 **AI 协作者**（Trae / Cursor / Copilot 等）在该仓库工作的契约。详细的导航/任务地图/runbook 见：

- [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) — **核心战略 + 35 个 TASK 路线图**（必读）
- [doc/智能编程工作站产品规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99%E4%BA%A7%E5%93%81%E8%A7%84%E5%88%92.md) — 6 大革命性特性（产品愿景）
- [doc/云集智能平台产品蓝图.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E5%B9%B3%E5%8F%B0%E4%BA%A7%E5%93%81%E8%93%9D%E5%9B%BE.md) — 平台 + 鸡生蛋战略
- [doc/Web化跨平台改造规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/Web%E5%8C%96%E8%B7%A8%E5%B9%B3%E5%8F%B0%E6%94%B9%E9%80%A0%E8%A7%84%E5%88%92.md) — 跨平台 + 远程编码
- [doc/codebase-map.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/codebase-map.md) — 任务地图"想改 X 去改 Y"（TODO 编写中）

---

## Project Snapshot（项目快照）

**云集智能编程工作站** — AI 原生的工作空间，不是 AI 工具。AI 是工作站的居民，主动感知、持续进化、协作创造。

| 层级 | 技术 |
|------|------|
| **桌面壳** | pywebview（复用系统 WebView2） + PyQt6（**legacy**） |
| **后端** | Python 3.12 + FastAPI + uvicorn |
| **前端** | Vue 3 + Vite + TypeScript + Pinia + Vant UI |
| **AI 引擎** | 多供应商：Anthropic / OpenRouter / Ollama / 智谱 GLM / 自定义 OpenAI 兼容 API |
| **子应用** | qwen2api（千问账户池）/ zhipu2api（智谱账户池） |
| **打包** | PyInstaller（`--onefile` 模式，输出到 `dev/dist/`） |
| **远程** | Web API + SSE + Token 配对（局域网） + Tailscale（计划中） |
| **移动端** | Capacitor Android（已实现，浏览器模式 PWA） |
| **持久化** | 多用户数据隔离：`data/public/` + `data/users/{user_id}/` |
| **Python 环境** | uv（按需自动安装 Python 3.12 + 依赖） |

**核心数据流**：
```
pywebview 窗口 → http://127.0.0.1:18080
  ↓
FastAPI（api_main.py）
  ↓
routes/ → services/ → backend.py（Ollama 代理、CLI 管理、配置管理）
  ↓
外部 API / 本地 Ollama / Claude CLI 子进程
```

---

## Non-Negotiable Architecture Rules（不可违反的架构规则）

> 违反任何一条，PR 必须被拒绝

1. **Python 字节码缓存** — 必须重定向到 `dev/temp/__pycache__/`，禁止污染 `dev/app/` 目录。已在 [main.py#L19-L27](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py#L19-L27) 和 [api_main.py#L44-L51](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py#L44-L51) 实现
2. **PlatformKit 命名** — 跨进程共享核心层目录名必须是 **`platformkit/`**（不是 `platform/`）。原因：Python 3.12 标准库有 `platform` 模块，**包名冲突会导致 `webview` 启动失败**（`ImportError: cannot import name 'python_implementation' from 'platform'`）。2026-06-08 已确认
3. **EXE 输出位置** — 必须输出到 `dev/dist/`，**禁止**项目根目录、`build/`、`ver/`（`build_web.py` 已配置）
4. **EXE 品牌校验** — EXE 文件名必须包含"云集智能编程工作站"（[launcher.py#L7-L37](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py#L7-L37)）
5. **单实例控制** — Windows 命名互斥体 + 命名事件 + 命名共享内存，遵循 [doc/单实例控制机制-技术研究.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%8D%95%E5%AE%9E%E4%BE%8B%E6%8E%A7%E5%88%B6%E6%9C%BA%E5%88%B6-%E6%8A%80%E6%9C%AF%E7%A0%94%E7%A9%B6.md)
6. **多用户数据隔离** — 用户数据写入 `data/users/{user_id}/`，**禁止**写入 `data/public/`（仅公共模板/插件/模型）
7. **前端别名** — 统一使用 `@/`，指向 `dev/web/src/`，**禁止**相对路径跨级（`../../../`）
8. **API 调用** — 一律通过 [dev/web/src/api/index.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) 封装，**禁止**组件内 `import axios from 'axios'`
9. **子应用挂载** — qwen2api / zhipu2api 挂载点固定为 `/api/qwen` 和 `/api/zhipu`，[api_main.py#L99-L128](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py#L99-L128) 不可改

---

## Backend Routing Rules（后端修改规则）

修改后端行为时，按以下顺序进行：

1. **平台核心层** — 跨进程共享的逻辑放在 [dev/app/platformkit/shared/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/)（无 FastAPI 依赖，未来可被 daemon 复用）
2. **服务层** — 业务逻辑放在 [dev/app/services/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/)，不要把业务逻辑写在路由层
3. **路由层** — 路由层只做参数解析 + 调服务 + 返回，禁止超过 30 行业务代码
4. **入口注册** — 新增路由必须在 [api_main.py#L85-L96](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py#L85-L96) 的 `_routes_modules` 列表中注册
5. **Pydantic 模型** — 复杂请求/响应必须用 Pydantic BaseModel，参考 [dev/app/routes/ai.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) 的 `ChatRequest` / `StopChatRequest`
6. **SSE 流式响应** — 流式对话必须用 `sse_starlette.EventSourceResponse`（参考 [routes/ai.py#L41-L47](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py#L41-L47)）

---

## Frontend Routing Rules（前端修改规则）

修改前端行为时，按以下顺序进行：

1. **页面** — 放在 [dev/web/src/views/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/)，对应路由在 [dev/web/src/router/index.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/router/index.ts)
2. **组件** — 跨页面通用组件放 `components/`，页面专用组件放在 views/ 同级子目录（Phase 1 末会拆 `features/`）
3. **状态** — 复杂页面状态用 Pinia store（[dev/web/src/stores/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/stores/)），简单 UI 状态用 `ref` / `reactive`
4. **API 调用** — 一律通过 [dev/web/src/api/index.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) 的 `aiApi` / `envApi` / `projectApi` / `systemApi` / `versionApi`，**禁止**在组件内直接 axios
5. **跨端适配** — 桌面/手机差异化逻辑放在 [dev/web/src/platform/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/)，用 `platform/desktop.ts` / `platform/mobile.ts`
6. **UI 组件** — 优先用 Vant 组件（[dev/web/src/main.ts#L6](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/main.ts#L6) 已引入），不要重复造轮子

---

## Import Aliases（导入别名）

### 前端

```ts
// ✅ 推荐
import { aiApi } from '@/api'
import { useChat } from '@/composables/useChat'
import ChatView from '@/views/ChatView.vue'

// ❌ 禁止
import ChatView from '../../../views/ChatView.vue'
import { aiApi } from '../../api'
```

| 别名 | 指向 |
|------|------|
| `@/*` | `dev/web/src/*` |
| `@platform/*` | `dev/web/src/platform/*`（计划中） |
| `@features/*` | `dev/web/src/features/*`（Phase 1 末启用） |
| `@shared/*` | `dev/web/src/shared/*`（Phase 1 末启用） |

> 当前 [tsconfig.app.json](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/tsconfig.app.json) 只配置了 `@/*`，更多别名在 Phase 1 末启用

### 后端

Python 走 `sys.path` 自动发现，模块导入按文件系统结构：

```python
# ✅ 推荐（绝对导入）
from services.ai_service import AiService
from routes.ai import router

# ❌ 禁止（相对导入跨包）
from ...services.ai_service import AiService
```

---

## Key File Anchors（关键文件锚点）

### 后端核心

| 文件 | 职责 |
|------|------|
| [dev/app/api_main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) | FastAPI 入口 + 路由注册 + pywebview 启动 + LAN Token 鉴权 |
| [dev/app/main.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) | **Legacy** PyQt6 + QWebEngineView 启动器，新代码别动这里 |
| [dev/app/launcher.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py) | EXE 入口点（品牌校验 + 杀旧实例） |
| [dev/app/launcher_web.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher_web.py) | Web 版 EXE 入口点 |
| [dev/app/backend.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) | **Ollama 代理 + .env 配置 + CLI 进程管理 + 模型列表聚合** |

### 服务层

| 文件 | 职责 |
|------|------|
| [dev/app/services/ai_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) | **多供应商 AI 对话**（Anthropic / OpenRouter / Ollama / 智谱） |
| [dev/app/services/cli_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/cli_service.py) | Claude CLI 子进程管理（stream-json 解析） |
| [dev/app/services/project_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) | 项目管理（多用户数据隔离） |
| [dev/app/services/config_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/config_service.py) | 配置管理（系统设置 + 插件 + 离线模型） |
| [dev/app/services/env_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/env_service.py) | 环境管理（Node.js / Bun / Python 安装 + 镜像源） |
| [dev/app/services/update_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/update_service.py) | 自动更新（GitHub / Gitee 远程版本） |
| [dev/app/services/ws.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ws.py) | WebSocket 服务 |

### 路由层

| 文件 | 端点前缀 |
|------|---------|
| [dev/app/routes/ai.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ai.py) | `/api/ai/*` — AI 对话 + 模型管理 |
| [dev/app/routes/project.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/project.py) | `/api/project/*` — 项目 + 会话 + 记忆 + 模板 |
| [dev/app/routes/system.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/system.py) | `/api/system/*` — 文件树 + 终端 + Git + 通知 + 插件 |
| [dev/app/routes/env.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/env.py) | `/api/env/*` — 环境管理 + 服务管理 |
| [dev/app/routes/version.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/version.py) | `/api/version/*` — 版本管理 + 更新 |
| [dev/app/routes/ws.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/ws.py) | WebSocket |

### 前端核心

| 文件 | 职责 |
|------|------|
| [dev/web/src/main.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/main.ts) | Vue 应用入口（Pinia + Router + Vant） |
| [dev/web/src/App.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/App.vue) | 根组件（路由出口） |
| [dev/web/src/router/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/router/index.ts) | 路由（6 条：chat/models/projects/env/settings/version） |
| [dev/web/src/api/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) | API 客户端（5 个命名空间） |
| [dev/web/src/platform/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/index.ts) | 跨端平台适配（desktop/mobile 检测） |
| [dev/web/src/plugins/YunJiPlugin.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/plugins/YunJiPlugin.ts) | YunJi 能力插件（YunJi.* 全局方法） |

### 页面（14 个 view，6 个已路由）

| 页面 | 路由 | 状态 |
|------|------|------|
| `ChatView.vue` | `/` | ✅ 主页 |
| `ModelsView.vue` | `/models` | ✅ |
| `ProjectView.vue` | `/projects` | ✅ |
| `EnvView.vue` | `/env` | ✅ |
| `SettingsView.vue` | `/settings` | ✅ |
| `VersionView.vue` | `/version` | ✅ |
| `OllamaView.vue` | ❌ | 待迁移/移除 |
| `ApiKeyView.vue` | ❌ | 待迁移/移除 |
| `ApiServiceView.vue` | ❌ | 待迁移/移除 |
| `FilesView.vue` | ❌ | 待迁移/移除 |
| `GitView.vue` | ❌ | 待迁移/移除 |
| `TerminalView.vue` | ❌ | 待迁移/移除 |
| `MemoryView.vue` | ❌ | 待迁移/移除 |
| `PluginView.vue` | ❌ | 待迁移/移除 |
| `ClaudeMdView.vue` | ❌ | 待迁移/移除 |

> 8 个未路由的 view 待 Phase 1 末评估：要么加路由（功能页），要么删除（重复/废弃）

### 构建 & 工具

| 文件 | 职责 |
|------|------|
| [build/build.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build.py) | **Legacy** PyQt6 + Electron 打包 |
| [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) | **当前主用** Web 打包（前端 + PyInstaller → `dev/dist/`） |
| [build/build_stub.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_stub.py) | Stub 启动器打包 |
| [dev/.gitignore](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/.gitignore) | Git 忽略配置（4 个目录纯净整合包架构） |

### Legacy（**不要动**）

| 目录 | 状态 |
|------|------|
| [dev/app/desktop/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/desktop/) | 旧 Electron 壳，**禁止修改** |
| [dev/app/src/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/src/) | 旧 TS 源码（Claude CLI 适配残留），**禁止修改** |
| [dev/app/main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) | 旧 PyQt6 启动器，**新功能不要加这里**（保留作为备份） |

---

## Validation Matrix（验证矩阵）

> 修改以下区域，必须跑对应命令。**未跑验证的 PR 必须被拒绝**。

| 改动区域 | 必跑命令（按顺序） |
|---------|-----------------|
| **后端 Python** | ① `cd dev/app && python -c "import sys; sys.path.insert(0, '.'); import main"`（导入测试）<br>② `cd dev/app && python -c "import api_main"`（API 启动测试）<br>③ 手动启动并访问 `http://127.0.0.1:18080/docs` 看 Swagger |
| **前端 Vue/TS** | ① `cd dev/web && npm run typecheck`（类型检查）<br>② `cd dev/web && npm run build`（完整构建）<br>③ `cd dev/web && npm run dev` 手动打开 5173 测试 |
| **API 协议** | ① 改前端 api + 改后端 route + Pydantic 模型<br>② `npm run build` + `python dev/app/api_main.py --dev` 联调 |
| **路由** | ① 改 [dev/web/src/router/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/router/index.ts)<br>② 6 个已路由页面都要测一遍 |
| **构建脚本** | ① 改 [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) 后必须 `python build/build_web.py --skip-frontend` 验证打包不破 |
| **单实例/品牌校验** | ① 改 [main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) / [launcher.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/launcher.py)<br>② 启动两个实例看是否冲突 |
| **远程 / LAN 模式** | ① 改 [api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) 的 LAN 逻辑<br>② `python dev/app/api_main.py --lan --port 18080` 测手机访问 |

### 通用冒烟测试

```bash
# 1. 后端导入测试（每次提交前必跑）
cd dev/app
python -c "import sys; sys.path.insert(0, '.'); import main, api_main, backend; print('OK')"

# 2. 前端构建测试
cd dev/web
npm run build

# 3. 启动开发服务器
cd dev
python app/api_main.py --dev   # 启动 FastAPI（不启动 pywebview）
# 新终端
cd dev/web
npm run dev                     # 启动 Vite，访问 http://localhost:5173
```

> 完整 Pytest + ESLint + GitHub Actions 在 Phase 1 W2-W3 上线

---

## Quick Runbook（核心命令）

### 开发模式

```bash
# 后端开发（FastAPI 热重载）
cd dev/app
python api_main.py --dev --port 18080

# 前端开发（Vite HMR）
cd dev/web
npm run dev

# 桌面端开发（pywebview 加载 127.0.0.1:18080）
cd dev/app
python api_main.py --port 18080   # 不加 --dev，会自动启动 pywebview

# 远程/局域网开发（手机访问）
cd dev/app
python api_main.py --lan --port 18080
# 启动后会打印：📱 手机访问: http://192.168.x.x:18080?token=XXXXXX
```

### 打包

```bash
# Web 版打包（前端构建 + PyInstaller --onefile）
cd dev/app/api_main.py 所在项目根
python build/build_web.py
# 输出: dev/dist/云集智能编程工作站-vYYYY.MM.DD.HHMM.exe
```

### 调试

```bash
# 查看后端日志（dev 模式 stdout）
# 查看前端日志：浏览器 DevTools Console
# 查看 WebSocket：DevTools Network → WS
# 查看局域网配对码：启动 --lan 后控制台打印
```

---

## Hotspots（高复杂度/高 churn 文件）

> 修改这些文件需**额外谨慎**，修改后必须充分测试

| 文件 | 原因 |
|------|------|
| [dev/app/api_main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api_main.py) | FastAPI 入口 + 路由注册 + LAN 鉴权 + 子应用挂载 + pywebview 启动 |
| [dev/app/backend.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/backend.py) | Ollama 代理 + 多供应商模型聚合 + CLI 进程管理（2000+ 行） |
| [dev/app/services/ai_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py) | 多供应商 AI 对话核心（2000+ 行） |
| [dev/app/main.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/main.py) | 单实例控制 + 品牌校验 + 启动器（**LEGACY**，慎改） |
| [dev/app/services/project_service.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/project_service.py) | 多用户数据隔离（500+ 行 + 数据迁移逻辑） |
| [dev/web/src/views/ChatView.vue](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/ChatView.vue) | 主页（最复杂，AI 对话 + 流式渲染 + 图片/代码块） |
| [dev/web/src/api/index.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/api/index.ts) | API 客户端（300+ 行，5 个命名空间） |
| [build/build_web.py](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/build/build_web.py) | 打包脚本（PyInstaller --onefile 配置复杂） |

---

## Sub-Apps（子应用）

挂载在主 API 之下的两个子应用，**禁止动**：

| 子应用 | 挂载点 | 路径 |
|--------|-------|------|
| **qwen2api** | `/api/qwen/*` | [dev/app/api/qwen2api/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api/qwen2api/) |
| **zhipu2api** | `/api/zhipu/*` | [dev/app/api/zhipu2api/](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/api/zhipu2api/) |

> 这两个子应用是独立的 FastAPI 应用，通过 `app.mount()` 挂载。它们的数据目录在 `data/public/qwen2api/data/` 和 `data/public/zhipu2api/data/`

---

## Roadmap Quick Reference（路线图速查）

> 完整 35 个 TASK 见 [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md)

| Phase | 范围 | 时间 |
|-------|------|------|
| **Phase 1** | 工程纪律（AGENTS.md / codebase-map / 平台拆 shared / feature-sliced / ESLint / Pytest / CI / Design System） | W1-W4 |
| **Phase 2** | 桌面体验对标（GitHub 集成 / DiffView / 图片附件 / Autocomplete / Whisper / iOS Layout / Toast / 快捷键） | W5-W8 |
| **Phase 3** | 核心差异化（自进化知识 / 主动感知 / 工程化持久化 / 独行模式 / 团队模式） | W9-W12 |
| **Phase 4** | 远程 + 生态（Daemon / Tailscale / PWA / 多用户协作 / 技能市场 / 模型路由 / v2.0 发布） | W13-W16 |

---

## Common Pitfalls（常见陷阱）

> 修改代码时**主动避免**这些坑

| 现象 | 原因 | 解决方案 |
|------|------|---------|
| EXE 启动报 `DLL load failed` | PyInstaller `--onefile` 模式 + Conda 环境 | 必须 `--onefile` 配 `uv` 管理的 venv，**禁止 conda** |
| EXE 启动后白屏 | 找不到 `static/` 前端 | 检查 `dev/app/static/` 是否在；构建后路径是否对齐 |
| 局域网手机访问 401 | Token 配对码输入错误 | 看启动日志 `[API] 🔑 局域网配对码: XXXXXX` |
| WebSocket 断连 | 局域网跨网段 | 确认手机和电脑在同一 WiFi/局域网 |
| Ollama 模型超时 | 默认 30s 不足 | `AI_TIMEOUT_MS=120000` 环境变量 |
| 多用户数据混乱 | 直接写 `data/` 根目录 | 必须用 `data/users/{user_id}/` |
| 旧 EXE 锁住端口 | 单实例控制不严 | 检查 `MUTEX_NAME` 版本号一致 |
| 前端 `Cannot find module '@/...'` | 路径别名未配置 | 查 [tsconfig.app.json](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/tsconfig.app.json) 和 [vite.config.ts](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/vite.config.ts) |
| 修改后 `npm run build` 报 TS 错误 | TypeScript 严格模式 | 跑 `npm run typecheck` 看具体错误 |
| 打包后 EXE 路径不对 | `build_web.py` 路径常量 | 检查 `BUILD_ROOT` / `DIST_DIR` 常量 |

---

## Design System Rule（设计系统规则，高级）

> Phase 1 W4 启用 Design System 后生效

1. **统一原子组件** — Button / Modal / Popover / Toast / Panel 5 个原子组件，**禁止**在 feature 组件中重复定义
2. **颜色 token** — 暗黑卡片式，`primary/secondary/accent/danger/warning` 各 5 层级
3. **间距 token** — 8/12/16/20/24/32px 体系
4. **字号 token** — 12/14/16/18/20/24/32px
5. **CSS 变量** — 全部从 `dev/web/src/styles/tokens.css` 取值，**禁止**硬编码颜色/间距

---

## Safety and Git Behavior（安全和 Git 行为）

1. **不要 commit** — `.env` / `data/users/` / `dev/dist/*.exe` / `app/static/`（[dev/.gitignore](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/.gitignore) 已配置）
2. **不要 reset/rebase 远端分支** — 除非用户明确要求
3. **不要 push** — 除非用户明确要求
4. **不要 commit 敏感信息** — API keys / 用户 token / 路径含用户名
5. **修复不破坏现有功能** — 改 Hotspot 文件后必须跑完整冒烟测试
6. **保守优于激进** — 不确定时问用户，不要擅自做决定
7. **遵循用户语言** — 文档/注释用中文（除非是技术名词）
8. **不要自创新文件** — 修改现有文件优先；新文件只在必要时创建

---

## Canonical References（详细文档）

| 文档 | 路径 |
|------|------|
| 战略 + 路线图（必读） | [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) |
| 产品规划 | [doc/智能编程工作站产品规划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99%E4%BA%A7%E5%93%81%E8%A7%84%E5%88%92.md) |
| 任务地图 | [doc/codebase-map.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/codebase-map.md)（TODO 编写中） |
| 产品蓝图 | [doc/云集智能平台产品蓝图.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E5%B9%B3%E5%8F%B0%E4%BA%A7%E5%93%81%E8%93%9D%E5%9B%BE.md) |
| Web 化规划 | [doc/Web化跨平台改造规划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/Web%E5%8C%96%E8%B7%A8%E5%B9%B3%E5%8F%B0%E6%94%B9%E9%80%A0%E8%A7%84%E5%88%92.md) |
| 单实例控制 | [doc/单实例控制机制-技术研究.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%8D%95%E5%AE%9E%E4%BE%8B%E6%8E%A7%E5%88%B6%E6%9C%BA%E5%88%B6-%E6%8A%80%E6%9C%AF%E7%A0%94%E7%A9%B6.md) |
| 宣发方案 | [doc/产品宣发方案.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%A7%E5%93%81%E5%AE%A3%E5%8F%91%E6%96%B9%E6%A1%88.md) |
| 目录结构 | [doc/纯净整合包目录结构说明.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E7%BA%AF%E5%87%80%E6%95%B4%E5%90%88%E5%8C%85%E7%9B%AE%E5%BD%95%E7%BB%93%E6%9E%84%E8%AF%B4%E6%98%8E.md) |

---

## Update History（更新历史）

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-06-08 | v1.0 初版，参考 CodexMonitor AGENTS.md 模板创建 | AI 协作者（Trae） |

---

> **最后提醒**：任何修改前先看 [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8/BD%AF%E4%BB%B6%E5%BC%80%E5%9F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) 是否有对应 TASK。如果有，按 TASK 编号走；如果没有，先和用户讨论再加到计划书
