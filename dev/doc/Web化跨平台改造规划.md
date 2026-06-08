# 云集智能编程工作站 — Web 化改造总体规划

> 版本：v2.1 | 日期：2026-05-25

---

## 一、改造目标

将当前 PyQt6 + QWebEngineView 桌面版改造为 **Web 前端 + Python 后端** 架构。

### 六大改造价值

| # | 价值 | 说明 | 与跨平台无关 |
|---|------|------|------------|
| 1 | **体积缩小 5 倍** | EXE 从 ~200MB 降至 ~38MB | ✅ 纯桌面端受益 |
| 2 | **架构质量提升** | API-first，前后端解耦，Swagger 文档 | ✅ 开发质量受益 |
| 3 | **核心层可复用** | AI 引擎/感知/知识系统独立，未来产品共用 | ✅ 鸡生蛋基础 |
| 4 | **开发效率提升** | Vite HMR 秒级热更新，Vue 组件化 | ✅ 开发速度受益 |
| 5 | **远程编码** | 电脑做后端，手机做前端，手机远程写代码 | 🆕 杀手级功能 |
| 6 | **跨平台扩展** | 未来产品（AI世界/创意工坊）手机端可用 | 未来受益 |

**Web 化改造的本质不是"跨平台"，而是"架构升级"。** 即使永远只做桌面端，前4个价值也足以支撑改造决策。

### 远程编码（电脑做后端，手机做前端）

```
📱 手机浏览器 → http://电脑IP:18080 → 💻 电脑 FastAPI 后端

手机变成电脑的远程控制器：
- 躺床上让 AI 写代码，手机上审查 DiffView
- 通勤路上收到感知通知，一键修复安全漏洞
- 开会时远程查看日志和项目状态
- 外出时监控 AI 团队模式的工作进度
```

技术实现：FastAPI 监听 0.0.0.0 + Token 配对认证 + 响应式 UI（已实现）

---

## 二、技术选型决策

### 2.1 桌面壳方案对比

| 维度 | PyQt6 + QWebEngineView (当前) | pywebview | Tauri 2.0 | Electron |
|------|------|------|------|------|
| **打包体积** | ~200MB+ (含Chromium) | ~38MB (复用系统WebView2) | ~3-8MB (Rust二进制) | ~80-150MB (含Chromium) |
| **后端语言** | Python | Python | Rust | Node.js |
| **手机端支持** | ❌ | ❌ (需Capacitor补充) | ✅ 原生支持iOS/Android | ❌ (需Capacitor补充) |
| **前端框架** | 任意 | 任意 | 任意 | 任意 |
| **学习成本** | 已掌握 | 极低 | 高(需学Rust) | 低 |
| **Python生态** | ✅ 完整 | ✅ 完整 | ❌ 需侧载 | ❌ 需侧载 |
| **成熟度** | 成熟 | 成熟 | 2.0稳定版 | 非常成熟 |
| **Windows兼容** | ✅ | ✅ (需WebView2) | ✅ | ✅ |
| **macOS/Linux** | ✅ | ✅ | ✅ | ✅ |

### 2.2 选型结论：pywebview + Capacitor（混合方案）

**选择理由**：

1. **Python 生态不可替代** — 本项目核心后端（Ollama代理、CLI管理、环境部署、配置管理）全部是 Python 实现，迁移到 Rust 成本极高且无必要
2. **pywebview 体积优势明显** — 从 200MB+ 降到 ~38MB，已达到目标
3. **参考项目已验证** — 云集智能网联代理专家已成功采用此方案，桌面版和手机版均已上线
4. **Tauri 不适合本项目** — 虽然体积更小(3-8MB)且原生支持手机端，但要求后端用 Rust 重写，Python 只能通过 sidecar 侧载，架构复杂度大增
5. **Capacitor 手机端成熟** — Ionic 团队维护，Android/iOS 生态完善，参考项目已有完整实现

**未来演进路径**：如果后续需要极致体积（<10MB），可以考虑 Tauri 2.0 方案，将 Python 后端作为 sidecar 进程运行。当前阶段优先保证开发效率和功能完整性。

### 2.3 技术栈总览

| 层 | 技术 | 版本 | 说明 |
|----|------|------|------|
| 前端框架 | Vue 3 | 3.5+ | 组合式 API + TypeScript |
| UI 组件库 | Vant 4 | 4.9+ | 移动端优先，桌面端也美观 |
| 构建工具 | Vite | 6.x | 极速热更新 |
| 状态管理 | Pinia | 3.x | Vue 3 官方推荐 |
| 路由 | Vue Router | 4.x | SPA 路由 |
| 后端框架 | FastAPI | 0.115+ | 异步、自动文档、WebSocket |
| 桌面壳 | pywebview | 5.3+ | 调用系统 WebView，~1MB |
| HTTP 服务 | uvicorn | 0.34+ | ASGI 服务器 |
| 手机壳 | Capacitor | 6.x | 原生 APP 打包 |
| 打包工具 | PyInstaller | 6.x | EXE 打包（--onefile 模式） |

---

## 三、当前改造进度

> v2.0 新增：跟踪各阶段实际完成情况

### 3.1 总体进度

| 阶段 | 状态 | 完成度 | 说明 |
|------|------|--------|------|
| M1 后端 API 化 | ✅ 已完成 | 95% | 6个路由模块 + 7个服务模块已实现，API 远超原规划 |
| M2 前端界面 | 🔄 进行中 | 70% | 5个核心页面已实现，缺少组件拆分和高级功能 |
| M3 桌面端整合 | ✅ 已完成 | 90% | pywebview + FastAPI + 构建流程已跑通 |
| M4 手机端开发 | ⏳ 未开始 | 10% | Capacitor 配置已有，YunJiPlugin 未实现 |

### 3.2 后端已实现功能清单

| 模块 | 路由文件 | 服务文件 | API数量 | 状态 |
|------|---------|---------|---------|------|
| AI 对话/模型 | routes/ai.py | ai_service.py | 7 | ✅ |
| 环境部署 | routes/env.py | env_service.py | 9 | ✅ |
| 项目管理 | routes/project.py | project_service.py | 22 | ✅ 远超原规划 |
| 系统设置 | routes/system.py | config_service.py | 19 | ✅ 远超原规划 |
| 版本更新 | routes/version.py | update_service.py | 9 | ✅ |
| WebSocket | routes/ws.py | ws.py | 3 | ✅ |
| **合计** | | | **69** | |

### 3.3 前端已实现功能清单

| 文件 | 功能 | 状态 | 备注 |
|------|------|------|------|
| App.vue | 桌面/手机布局切换 | ✅ | 顶部导航 + 底部Tabbar |
| router/index.ts | 5条路由 | ✅ | chat/models/projects/env/settings |
| api/index.ts | API 客户端 | ⚠️ | 与后端存在接口不一致 |
| platform/index.ts | 平台检测 | ✅ | Capacitor 原生检测 |
| platform/desktop.ts | 桌面版实现 | ✅ | SSE 流式对话 |
| platform/mobile.ts | 手机版实现 | ✅ | YunJiPlugin 桥接 |
| stores/chat.ts | 对话状态 | ✅ | 流式输出 + 停止 |
| stores/model.ts | 模型状态 | ✅ | 提供商/模型切换 |
| stores/project.ts | 项目状态 | ✅ | 项目/会话管理 |
| stores/settings.ts | 设置状态 | ✅ | 主题/语言/字体 |
| views/ChatView.vue | AI 对话页 | ✅ | 桌面侧边栏 + 手机弹窗 |
| views/ModelsView.vue | 模型管理页 | ✅ | 桌面分栏 + 手机标签页 |
| views/ProjectView.vue | 项目管理页 | ✅ | 桌面分栏 + 手机卡片 |
| views/EnvView.vue | 环境部署页 | ✅ | 仅桌面端 |
| views/SettingsView.vue | 设置页 | ✅ | 桌面分栏 + 手机表单 |
| styles/main.css | 暗黑主题 | ✅ | CSS变量 + Vant覆盖 |
| vite.config.ts | 构建配置 | ✅ | 代理 /api → :18080 |
| capacitor.config.ts | Capacitor配置 | ✅ | 基础配置 |

### 3.4 前端待完善清单

| 优先级 | 内容 | 说明 |
|--------|------|------|
| 🔴 高 | 修复前后端 API 接口不一致 | 详见第五章对照表 |
| 🔴 高 | 创建 YunJiPlugin.ts | 手机版 mobile.ts 引用但文件不存在 |
| 🔴 高 | 对话页会话保存/删除 | 后端已有API，前端未对接 |
| 🟡 中 | 拆分可复用组件 | ChatBubble/CodeBlock/MarkdownRenderer |
| 🟡 中 | 创建 composables | useWebSocket/useStream/useDevice |
| 🟡 中 | API Key 配置 UI | 设置页缺少 API Key 管理 |
| 🟡 中 | 版本管理 UI | 版本历史/切换/下载前端未实现 |
| 🟡 中 | CLAUDE.md / 记忆管理 | 后端已实现，前端未对接 |
| 🟡 中 | 项目模板功能 | 后端已实现，前端未对接 |
| 🟢 低 | 虚拟滚动优化 | AI 对话长文本性能 |
| 🟢 低 | 终端/命令执行 UI | 后端已有 terminal/run |
| 🟢 低 | Git 状态/提交 UI | 后端已有 git/status, git/commit |
| 🟢 低 | 插件管理 UI | 后端已有 plugins 相关API |
| 🟢 低 | 离线模型管理 UI | 后端已有 models 相关API |

---

## 四、架构设计

### 4.1 整体架构

```
┌──────────────────────────────────────────────────────┐
│              Vue 3 + Vant 4 前端（一套代码）            │
│         响应式布局，手机竖屏 ←→ 桌面宽屏自动适配          │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │           平台适配层 (platform/)                 │  │
│  │  isDesktop → FastAPI HTTP API + SSE             │  │
│  │  isMobile  → Capacitor Plugin 原生桥接           │  │
│  └────────────────────────────────────────────────┘  │
└────────────────────┬─────────────────────────────────┘
                     │ HTTP API + WebSocket (标准协议)
┌────────────────────▼─────────────────────────────────┐
│             FastAPI 后端 (Python)                      │
│                                                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐  │
│  │ AI 服务   │ │ 环境服务  │ │ 项目服务  │ │系统服务 │  │
│  │ Ollama   │ │ 部署维护  │ │ 项目管理  │ │版本更新 │  │
│  │ 模型管理  │ │ CLI管理   │ │ 会话管理  │ │设置管理 │  │
│  │          │ │ API服务   │ │ CLAUDE.md │ │插件管理 │  │
│  │          │ │          │ │ 记忆管理   │ │Git集成  │  │
│  │          │ │          │ │ 模板系统   │ │终端执行 │  │
│  └──────────┘ └──────────┘ └──────────┘ └────────┘  │
└────────────────────┬─────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
   ┌──────────────┐     ┌──────────────┐
   │  桌面端       │     │  手机端       │
   │  pywebview    │     │  Capacitor   │
   │  窗口加载前端  │     │  APP壳加载前端│
   │  + 本地后端   │     │  + 云端API    │
   │  ~38MB       │     │  ~10-20MB    │
   └──────────────┘     └──────────────┘
```

### 4.2 桌面端工作方式

1. EXE 启动 → FastAPI 后端在 `127.0.0.1:18080` 监听
2. pywebview 打开窗口，加载 `http://127.0.0.1:18080`
3. 前端通过 HTTP API + WebSocket 与后端通信
4. 后端直接调用本地能力（Ollama代理、CLI管理、文件操作、环境部署）

### 4.3 手机端工作方式

1. Capacitor APP 启动 → WebView 加载前端页面
2. 前端通过 Capacitor Plugin 桥接原生功能
3. AI 对话通过云端 API（Anthropic/OpenRouter/智谱等）直连
4. 本地 AI 通过远程 Ollama 服务连接
5. 不需要 Python 后端，不需要 FastAPI

### 4.4 与当前架构的关键差异

| 维度 | 当前 PyQt6 架构 | 改造后 Web 架构 |
|------|----------------|----------------|
| 前后端通信 | QWebChannel (Qt专有) | HTTP API + WebSocket (标准协议) |
| 桌面窗口 | QWebEngineView (内嵌Chromium) | pywebview (系统WebView2) |
| EXE体积 | ~200MB+ | ~38MB |
| 打包模式 | --onedir (必须) | --onefile (可以) |
| 手机端 | ❌ 不支持 | ✅ Capacitor |
| API文档 | 无 | FastAPI 自动 Swagger |
| 调试方式 | Qt日志 | 浏览器 DevTools + Swagger |
| 前端热更新 | 需重启 | Vite HMR 秒级 |

---

## 五、API 设计（对齐实际后端实现）

> v2.0 更新：根据实际后端代码修正，标注前端对接状态

### 5.1 AI 对话 `/api/ai`

| 方法 | 路径 | 说明 | 前端对接 |
|------|------|------|---------|
| POST | /chat | 流式对话（SSE） | ⚠️ 前端用 /chat/stream，需统一 |
| POST | /chat/stop | 停止对话 | ⚠️ 前端用 /stop，需对齐 |
| GET | /models/{provider} | 获取模型列表 | ⚠️ 前端用 GET /models?provider=，需改路径参数 |
| GET | /providers | 获取提供商列表 | ✅ |
| POST | /providers/check | 检测API连接 | ⚠️ 前端用 GET，后端是 POST |
| GET | /ollama/capabilities | Ollama模型能力 | ⚠️ 前端缺少 model_name 参数 |
| POST | /ollama/health | Ollama健康检测 | ⚠️ 前端用 GET，后端是 POST |

**ChatRequest 参数**（实际后端）：

```python
class ChatRequest(BaseModel):
    prompt: str           # 用户输入（非 messages 数组）
    provider: str | None  # 提供商
    model: str | None     # 模型
    session_id: str | None
    workspace_path: str | None
    ai_language: str | None
    ai_temperature: float | None  # 0-2
    ai_max_tokens: int | None
    system_prompt: str | None
    auto_approve: bool = False
```

### 5.2 环境部署 `/api/env`

| 方法 | 路径 | 说明 | 前端对接 |
|------|------|------|---------|
| GET | /check | 检测运行环境 | ❌ 前端未对接 |
| GET | /status | 获取安装状态 | ✅ |
| POST | /install?component=xxx | 安装组件 | ⚠️ 前端用 POST body，后端用 query |
| POST | /install-all | 一键安装 | ✅ |
| GET | /mirror | 获取镜像源 | ✅ |
| POST | /mirror | 设置镜像源 | ⚠️ 前端用 {mirror}，后端用 {mirror_key} |
| GET | /services | 列出API服务 | ❌ 前端未对接 |
| GET | /services/{name} | 服务详情 | ❌ |
| POST | /services/start | 启动服务 | ❌ |
| POST | /services/stop | 停止服务 | ❌ |
| POST | /services/start-all | 启动全部 | ❌ |

### 5.3 项目管理 `/api/project`

| 方法 | 路径 | 说明 | 前端对接 |
|------|------|------|---------|
| GET | /list | 项目列表 | ✅ |
| GET | /active | 当前活跃项目 | ✅ |
| POST | /create | 创建项目 | ✅ |
| POST | /switch/{project_id} | 切换项目 | ⚠️ 前端用 POST body，后端用路径参数 |
| POST | /rename/{project_id} | 重命名项目 | ❌ |
| POST | /update/{project_id} | 更新项目 | ❌ |
| DELETE | /{project_id} | 删除项目 | ⚠️ 前端用 POST，后端用 DELETE |
| GET | /{project_id}/conversations | 会话列表 | ⚠️ 前端用 GET /conversations?projectId= |
| GET | /{project_id}/conversations/{sid} | 会话详情 | ❌ |
| POST | /conversations/save | 保存会话 | ❌ |
| POST | /conversations/copy | 复制会话 | ❌ |
| POST | /conversations/rename | 重命名会话 | ❌ |
| DELETE | /{project_id}/conversations/{sid} | 删除会话 | ❌ |
| GET | /{project_id}/conversations/search | 搜索会话 | ❌ |
| GET | /{project_id}/context | 项目上下文 | ❌ |
| GET | /{project_id}/claude-md | 获取CLAUDE.md | ❌ |
| POST | /claude-md/save | 保存CLAUDE.md | ❌ |
| GET | /claude-md/global | 全局CLAUDE.md | ❌ |
| POST | /claude-md/global | 保存全局CLAUDE.md | ❌ |
| GET | /{project_id}/memories | 记忆列表 | ❌ |
| POST | /memories/save | 保存记忆 | ❌ |
| DELETE | /{project_id}/memories/{filename} | 删除记忆 | ❌ |
| GET | /{project_id}/memories/search | 搜索记忆 | ❌ |
| GET | /{project_id}/memories/stats | 记忆统计 | ❌ |
| GET | /{project_id}/memories/relevant | 相关记忆 | ❌ |
| GET | /templates | 模板列表 | ❌ |
| POST | /templates/custom | 保存自定义模板 | ❌ |
| DELETE | /templates/custom/{id} | 删除自定义模板 | ❌ |
| POST | /create-from-template | 从模板创建 | ❌ |

### 5.4 系统设置 `/api/system`

| 方法 | 路径 | 说明 | 前端对接 |
|------|------|------|---------|
| GET | /state | 应用状态 | ✅ |
| GET | /settings | 获取设置 | ✅ |
| POST | /settings | 保存设置 | ⚠️ 前端用扁平对象，后端用 {settings: {}} |
| POST | /settings/clear-model | 清除模型设置 | ❌ |
| GET | /hardware | 硬件信息 | ✅ |
| GET | /workspace | 工作目录 | ❌ |
| POST | /workspace/choose | 选择工作目录 | ❌ |
| POST | /terminal/run | 执行终端命令 | ⚠️ 前端用 /command，后端用 /terminal/run |
| GET | /app-data | 加载应用数据 | ❌ |
| POST | /app-data | 保存应用数据 | ❌ |
| GET | /git/status | Git状态 | ❌ |
| POST | /git/commit | Git提交 | ❌ |
| GET | /git/log | Git日志 | ❌ |
| GET | /file-tree | 文件树 | ❌ |
| POST | /notification | 桌面通知 | ❌ |
| POST | /open-url | 打开外部链接 | ❌ |
| POST | /open-explorer | 打开资源管理器 | ❌ |
| POST | /select-directory | 选择目录对话框 | ❌ |
| GET | /plugins | 插件列表 | ❌ |
| POST | /plugins/install | 安装插件 | ❌ |
| DELETE | /plugins/{id} | 卸载插件 | ❌ |
| POST | /plugins/{id}/execute | 执行插件 | ❌ |
| GET | /models/offline | 离线模型列表 | ❌ |
| POST | /models/download | 下载模型 | ❌ |
| POST | /models/delete | 删除模型 | ❌ |
| GET | /models/ollama-search | 搜索Ollama库 | ❌ |
| POST | /models/pull | 拉取Ollama模型 | ❌ |
| GET | /models/recommend | 推荐模型 | ❌ |

### 5.5 版本更新 `/api/version`

| 方法 | 路径 | 说明 | 前端对接 |
|------|------|------|---------|
| GET | /current | 当前版本 | ✅ |
| GET | /history | 版本历史 | ✅ |
| GET | /remote | 远程版本信息 | ❌ |
| GET | /check-update | 检查更新 | ⚠️ 前端用 /check，后端是 /check-update |
| POST | /pull-update | 拉取更新 | ❌ |
| GET | /commits | 远程提交记录 | ❌ |
| GET | /git-history | 本地Git历史 | ❌ |
| POST | /switch-commit | 切换到指定commit | ❌ |
| POST | /switch-exe | 切换到指定EXE | ❌ |
| GET | /stable-exes | 稳定版EXE列表 | ❌ |
| POST | /download-update | 下载更新包 | ⚠️ 前端用 /download，后端是 /download-update |

### 5.6 WebSocket `/ws`

| 端点 | 说明 | 前端对接 |
|------|------|---------|
| /ws | 主WebSocket端点 | ❌ |
| /ws/ai/{session_id} | AI流式对话 | ❌ |
| /ws/terminal | 终端交互 | ❌ |

### 5.7 前后端接口不一致汇总（需修复）

| # | 前端 api/index.ts | 后端 routes/*.py | 修复方向 |
|---|-------------------|-------------------|---------|
| 1 | POST /ai/chat (body: messages数组) | POST /api/ai/chat (body: prompt字符串) | 前端改为发送 prompt |
| 2 | POST /ai/stop (body: {sessionId}) | POST /api/ai/chat/stop (body: {session_id}) | 前端对齐路径和字段名 |
| 3 | GET /ai/models?provider=xxx | GET /api/ai/models/{provider} | 前端改为路径参数 |
| 4 | GET /ai/provider/check | POST /api/ai/providers/check | 前端改为 POST |
| 5 | GET /ai/ollama/health | POST /api/ai/ollama/health | 前端改为 POST |
| 6 | POST /env/deploy | POST /api/env/install-all | 前端对齐路径 |
| 7 | POST /env/install (body: {component}) | POST /api/env/install?component=xxx | 前端改为 query 参数 |
| 8 | POST /env/mirror (body: {mirror}) | POST /api/env/mirror (body: {mirror_key}) | 前端对齐字段名 |
| 9 | POST /project/switch (body: {projectId}) | POST /api/project/switch/{project_id} | 前端改为路径参数 |
| 10 | POST /project/delete (body: {projectId}) | DELETE /api/project/{project_id} | 前端改为 DELETE + 路径参数 |
| 11 | GET /project/conversations?projectId= | GET /api/project/{id}/conversations | 前端改为路径参数 |
| 12 | POST /project/conversation/save | POST /api/project/conversations/save | 前端对齐路径 |
| 13 | POST /project/conversation/delete | DELETE /api/project/{id}/conversations/{sid} | 前端改为 DELETE |
| 14 | POST /system/command | POST /api/system/terminal/run | 前端对齐路径 |
| 15 | POST /system/settings (扁平对象) | POST /api/system/settings ({settings: {}}) | 前端对齐请求体 |
| 16 | GET /version/check | GET /api/version/check-update | 前端对齐路径 |
| 17 | POST /version/download | POST /api/version/download-update | 前端对齐路径 |

---

## 六、目录结构（对齐实际代码）

```
云集智能编程工作站/
│
│ # ========== Git管理 ==========
├── dev/                        # Git仓库根目录
│   ├── .gitignore
│   ├── app/                    # 后端（Python）
│   │   ├── api_main.py         # ⭐ FastAPI 入口 + pywebview 窗口
│   │   ├── main.py             # 旧版入口（过渡期保留）
│   │   ├── backend.py          # 旧版后端（过渡期保留）
│   │   ├── launcher.py         # EXE入口（杀旧进程 + 自部署 + --cleanup）
│   │   ├── launcher_stub.py    # 自部署启动器源码
│   │   ├── build-version.py    # 版本化构建工具
│   │   ├── routes/             # FastAPI 路由
│   │   │   ├── ai.py           #   AI对话 / 模型管理 / 流式输出 (7个API)
│   │   │   ├── env.py          #   环境部署 / API服务管理 (9个API)
│   │   │   ├── project.py      #   项目/会话/CLAUDE.md/记忆/模板 (22个API)
│   │   │   ├── system.py       #   系统/设置/Git/终端/插件/模型 (19个API)
│   │   │   ├── version.py      #   版本更新 / Git历史 (9个API)
│   │   │   └── ws.py           #   WebSocket 端点 (3个端点)
│   │   ├── services/           # 业务逻辑
│   │   │   ├── ai_service.py   #   Ollama代理 / 模型列表 / 流式对话
│   │   │   ├── env_service.py  #   环境部署 / API服务管理
│   │   │   ├── cli_service.py  #   Claude CLI 子进程管理
│   │   │   ├── project_service.py # 项目/会话/CLAUDE.md/记忆/模板
│   │   │   ├── config_service.py  # .env配置/硬件/Git/终端/插件/模型
│   │   │   ├── update_service.py  # 软件更新 / 版本切换
│   │   │   └── ws.py           #   WebSocket 管理器
│   │   ├── static/             # 前端构建产物（构建时生成）
│   │   ├── desktop/            # 旧版桌面端代码
│   │   ├── bin/                # Claude Code CLI 工具
│   │   ├── src/                # Claude Code 核心源码
│   │   ├── scripts/            # 启动/安装脚本
│   │   ├── icon.ico / icon.png
│   │   ├── pyproject.toml
│   │   ├── requirements.txt
│   │   └── package.json
│   │
│   ├── web/                    # 前端（Vue 3 + Vant 4）
│   │   ├── src/
│   │   │   ├── views/          #   页面组件（5个已实现）
│   │   │   │   ├── ChatView.vue      # ✅ AI 对话页
│   │   │   │   ├── ModelsView.vue    # ✅ 模型管理页
│   │   │   │   ├── ProjectView.vue   # ✅ 项目管理页
│   │   │   │   ├── EnvView.vue       # ✅ 环境部署页
│   │   │   │   └── SettingsView.vue  # ✅ 设置页
│   │   │   ├── components/     #   通用组件（待拆分）
│   │   │   │   └── HelloWorld.vue    # 占位组件
│   │   │   ├── platform/       #   ✅ 平台适配层
│   │   │   │   ├── index.ts    #     平台检测 + 统一导出
│   │   │   │   ├── desktop.ts  #     桌面版 API 实现（SSE流式）
│   │   │   │   └── mobile.ts   #     手机版 API 实现（Capacitor）
│   │   │   ├── api/            #   ✅ 后端 API 调用
│   │   │   │   └── index.ts    #     ⚠️ 需修复接口不一致
│   │   │   ├── stores/         #   ✅ Pinia 状态管理
│   │   │   │   ├── chat.ts     #     对话状态 + 流式
│   │   │   │   ├── model.ts    #     模型/提供商状态
│   │   │   │   ├── project.ts  #     项目/会话状态
│   │   │   │   └── settings.ts #     设置状态
│   │   │   ├── composables/    #   ❌ 待创建
│   │   │   ├── plugins/        #   ❌ YunJiPlugin.ts 待创建
│   │   │   ├── styles/         #   ✅ 暗黑主题
│   │   │   │   └── main.css
│   │   │   ├── router/         #   ✅ 路由配置
│   │   │   │   └── index.ts
│   │   │   ├── App.vue         #   ✅ 主布局
│   │   │   └── main.ts         #   ✅ 入口
│   │   ├── android/            #   Capacitor Android 项目
│   │   ├── public/
│   │   ├── capacitor.config.ts #   ✅ Capacitor 配置
│   │   ├── package.json
│   │   ├── vite.config.ts      #   ✅ 含 /api 代理
│   │   └── tsconfig.json
│   │
│   ├── ver/                    # 稳定版 EXE
│   └── dist/                   # 测试输出
│
├── build/                      # 构建脚本
│   ├── build_web.py            #   ⭐ Web版构建（前端+后端+PyInstaller）
│   ├── build_stub.py           #   自部署启动器构建
│   └── build.py                #   旧版构建
│
├── doc/                        # 文档
│   ├── Web化跨平台改造规划.md
│   ├── 开发指南.md
│   ├── 单实例控制机制-技术研究.md
│   └── 纯净整合包目录结构说明.md
│
└── BAK/                        # 旧版本备份
```

---

## 七、前端页面设计

### 7.1 页面结构

```
┌──────────────────────┐    ┌──────────────────────────────┐
│     手机模式 (9:16)    │    │       桌面模式 (宽屏)          │
│                      │    │                              │
│  ┌────────────────┐  │    │  ┌──────────────────────────┐│
│  │   状态栏        │  │    │  │  顶部导航 + 模型选择       ││
│  │   模型/状态     │  │    │  └──────────────────────────┘│
│  └────────────────┘  │    │                              │
│                      │    │  ┌──────────────────────────┐│
│  ┌────────────────┐  │    │  │                          ││
│  │                │  │    │  │      内容区域              ││
│  │   内容区域     │  │    │  │                          ││
│  │                │  │    │  │                          ││
│  │                │  │    │  │                          ││
│  └────────────────┘  │    │  └──────────────────────────┘│
│                      │    │                              │
│  ┌────────────────┐  │    │  ┌────┐┌────┐┌────┐┌────┐  │
│  │ 💬  🤖  📁  ⚙️ │  │    │  │💬 ││🤖 ││📁 ││⚙️ │  │
│  │   底部导航栏    │  │    │  │对话││模型││项目││设置│  │
│  └────────────────┘  │    │  └────┘└────┘└────┘└────┘  │
└──────────────────────┘    └──────────────────────────────┘
```

### 7.2 五个核心页面

| 页面 | 图标 | 手机模式 | 桌面模式 | 实现状态 |
|------|------|---------|---------|---------|
| AI 对话 | 💬 | 全屏对话 + 底部输入 | 左侧会话列表 + 右侧对话区 | ✅ 基础完成 |
| 模型管理 | 🤖 | 竖向卡片堆叠 | 左右分栏：提供商列表 + 模型详情 | ✅ 基础完成 |
| 项目管理 | 📁 | 项目卡片列表 | 项目列表 + 会话详情双栏 | ✅ 基础完成 |
| 环境部署 | 🔧 | 仅桌面端 | 部署状态面板 + 日志 | ✅ 基础完成 |
| 设置 | ⚙️ | 竖向表单 | 左右分栏设置 | ✅ 基础完成 |

### 7.3 响应式断点

| 断点 | 宽度 | 布局 |
|------|------|------|
| 手机 | 360-599px | 底部导航 + 竖向堆叠 |
| 平板/桌面 | ≥ 600px | 顶部导航 + 横向布局 |

### 7.4 窗口尺寸策略

| 平台 | 窗口行为 | 最小宽度 | 说明 |
|------|---------|---------|------|
| 桌面 (pywebview) | 可自由拖拽调整 | 360px | flex-wrap 自动适配 |
| 手机 (Capacitor) | 全屏，不可调 | — | 天然适配 |
| 平板 (Capacitor) | 全屏，不可调 | — | 横竖屏自动适配 |

pywebview 创建窗口时设置 `min_size=(360, 600)`。

### 7.5 暗黑主题设计

保持当前深色 + 蓝色强调色风格（已实现）：

```css
:root {
  --bg-primary: #0d0d0d;
  --bg-secondary: #111122;
  --bg-card: #1a1a2e;
  --bg-card-hover: #222240;
  --bg-input: #16162a;
  --text-primary: #E0E0E0;
  --text-secondary: #888888;
  --text-muted: #555555;
  --accent: #42A5F5;
  --accent-dark: #1565C0;
  --accent-light: #64B5F6;
  --success: #4CAF50;
  --warning: #FF9800;
  --error: #F44336;
  --border: #2a2a44;
}
```

### 7.6 AI 对话页面核心交互

对话页面是本项目最核心的页面，需要特别设计：

```
桌面模式：
┌─────────────────────────────────────────────────────┐
│  [模型: Claude Sonnet 4 ▾]  [提供商: Anthropic ▾]    │
├──────────────┬──────────────────────────────────────┤
│              │                                      │
│  会话列表     │  对话区域                             │
│              │                                      │
│  📅 今天      │  👤 用户: 帮我写一个Python脚本         │
│   ├ 聊天1    │                                      │
│   └ 聊天2    │  🤖 AI: 好的，这是一个...              │
│              │  ```python                           │
│  📅 昨天      │  print("Hello")                     │
│   ├ 聊天3    │  ```                                 │
│   └ 聊天4    │                                      │
│              │                                      │
│              │  ┌──────────────────────────────────┐│
│              │  │ 输入消息...          [发送] [停止] ││
│              │  └──────────────────────────────────┘│
└──────────────┴──────────────────────────────────────┘

手机模式：
┌──────────────────────┐
│  [Claude Sonnet 4 ▾] │
├──────────────────────┤
│                      │
│  👤 帮我写一个脚本    │
│                      │
│  🤖 好的，这是...    │
│  ```python           │
│  print("Hello")      │
│  ```                 │
│                      │
├──────────────────────┤
│ 输入消息...   [发送]  │
├──────────────────────┤
│ 💬  🤖  📁  ⚙️      │
└──────────────────────┘
```

**流式输出**：AI 回复通过 SSE (Server-Sent Events) 逐字推送，前端实时渲染 Markdown + 代码高亮。

---

## 八、前端组件与 Composables 规划

> v2.0 新增：细化前端架构，明确待创建的组件和组合式函数

### 8.1 待拆分的可复用组件

| 组件 | 来源 | 说明 | 优先级 |
|------|------|------|--------|
| ChatBubble.vue | ChatView.vue | 对话气泡（用户/AI），含头像、角色标识 | 🔴 高 |
| CodeBlock.vue | ChatView.vue | 代码块组件（语法高亮 + 复制按钮 + 语言标签） | 🔴 高 |
| MarkdownRenderer.vue | ChatView.vue | Markdown 渲染组件（整合 marked + highlight.js） | 🔴 高 |
| TopBar.vue | App.vue | 顶部导航栏（桌面模式标签 + 手机模式标题） | 🟡 中 |
| BottomNav.vue | App.vue | 底部导航栏（手机模式） | 🟡 中 |
| SessionList.vue | ChatView.vue | 会话列表（桌面侧边栏 + 手机弹窗共用） | 🟡 中 |
| ModelSelector.vue | ChatView.vue | 模型/提供商选择器（桌面下拉 + 手机弹窗） | 🟡 中 |
| VersionManager.vue | SettingsView.vue | 版本管理组件（检查/下载/切换/历史） | 🟡 中 |
| ApiKeyManager.vue | SettingsView.vue | API Key 配置管理 | 🟡 中 |
| TerminalOutput.vue | 新增 | 终端输出组件（WebSocket 实时输出） | 🟢 低 |
| FileTree.vue | 新增 | 文件树组件 | 🟢 低 |
| GitStatus.vue | 新增 | Git 状态组件 | 🟢 低 |

### 8.2 待创建的 Composables

| Composable | 说明 | 优先级 |
|-----------|------|--------|
| useStream.ts | SSE 流式请求封装（当前逻辑在 platform/desktop.ts） | 🔴 高 |
| useDevice.ts | 设备检测 + 响应式断点（当前各页面自行检测） | 🔴 高 |
| useWebSocket.ts | WebSocket 连接管理（重连、心跳、消息分发） | 🟡 中 |
| useNotification.ts | 通知提示（桌面通知 + 应用内 Toast） | 🟢 低 |

### 8.3 useStream 设计

```typescript
// composables/useStream.ts
export function useStream() {
  const isStreaming = ref(false)
  const abortController = ref<AbortController | null>(null)

  async function streamChat(
    params: ChatParams,
    onChunk: (content: string) => void,
    onDone: () => void,
    onError: (error: Error) => void,
  ) {
    isStreaming.value = true
    abortController.value = new AbortController()

    try {
      const response = await fetch('/api/ai/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
        signal: abortController.value.signal,
      })

      if (!response.ok) throw new Error(`请求失败: ${response.status}`)

      const reader = response.body!.getReader()
      const decoder = new TextDecoder()

      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        const text = decoder.decode(value, { stream: true })
        for (const line of text.split('\n')) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6)
            if (data === '[DONE]') { onDone(); return }
            try {
              const parsed = JSON.parse(data)
              if (parsed.content) onChunk(parsed.content)
            } catch { onChunk(data) }
          }
        }
      }
    } catch (e) {
      if ((e as Error).name !== 'AbortError') onError(e as Error)
    } finally {
      isStreaming.value = false
      abortController.value = null
    }
  }

  function stopStream() {
    abortController.value?.abort()
    isStreaming.value = false
  }

  return { isStreaming, streamChat, stopStream }
}
```

### 8.4 useDevice 设计

```typescript
// composables/useDevice.ts
export function useDevice() {
  const width = ref(window.innerWidth)
  const isMobile = computed(() => width.value < 600)
  const isDesktop = computed(() => !isMobile.value)

  function onResize() { width.value = window.innerWidth }

  onMounted(() => window.addEventListener('resize', onResize))
  onUnmounted(() => window.removeEventListener('resize', onResize))

  return { width, isMobile, isDesktop }
}
```

### 8.5 useWebSocket 设计

```typescript
// composables/useWebSocket.ts
export function useWebSocket(url: string) {
  const ws = ref<WebSocket | null>(null)
  const isConnected = ref(false)
  const lastMessage = ref<any>(null)
  let reconnectTimer: number | null = null

  function connect() {
    ws.value = new WebSocket(url)
    ws.value.onopen = () => { isConnected.value = true }
    ws.value.onclose = () => {
      isConnected.value = false
      reconnectTimer = window.setTimeout(connect, 3000)
    }
    ws.value.onmessage = (event) => {
      lastMessage.value = JSON.parse(event.data)
    }
  }

  function send(data: any) {
    ws.value?.send(JSON.stringify(data))
  }

  function disconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    ws.value?.close()
  }

  onMounted(connect)
  onUnmounted(disconnect)

  return { isConnected, lastMessage, send, disconnect }
}
```

---

## 九、平台适配层设计

### 9.1 核心思路（已实现）

参考云集智能网联代理专家的成熟方案，通过平台适配层实现一套前端代码适配桌面和手机：

```typescript
// src/platform/index.ts（已实现）
import { Capacitor } from '@capacitor/core'
import * as desktop from './desktop'
import * as mobile from './mobile'

export const isMobile = Capacitor.isNativePlatform()
export const isDesktop = !isMobile
export const platform: 'desktop' | 'mobile' = isMobile ? 'mobile' : 'desktop'

const impl = isMobile ? mobile : desktop

export const chat = impl.chat
export const loadSession = impl.loadSession
export const getProviders = impl.getProviders
export const getModels = impl.getModels
export const listProjects = impl.listProjects
export const createProject = impl.createProject
export const switchProject = impl.switchProject
export const deleteProject = impl.deleteProject
export const getActiveProject = impl.getActiveProject
export const getConversations = impl.getConversations
export const getSettings = impl.getSettings
export const saveSettings = impl.saveSettings
export const getSystemInfo = impl.getSystemInfo
```

### 9.2 桌面版实现（已实现）

通过 FastAPI HTTP API + SSE 实现所有功能。

### 9.3 手机版实现（已实现框架，缺少 YunJiPlugin.ts）

通过 Capacitor Plugin 桥接原生功能，AI 对话云端直连。

### 9.4 手机版与桌面版功能差异

| 功能 | 桌面版 | 手机版 |
|------|--------|--------|
| AI 对话 | ✅ 本地Ollama + 云端API | ✅ 仅云端API |
| 模型管理 | ✅ Ollama + OpenRouter + Anthropic + 智谱 | ✅ 仅云端提供商 |
| 项目管理 | ✅ 本地文件系统 | ✅ 本地存储 |
| 环境部署 | ✅ Node.js/Bun/CLI 安装 | ❌ 不适用 |
| CLI 管理 | ✅ Claude CLI 子进程 | ❌ 不适用 |
| Ollama 代理 | ✅ 本地代理服务器 | ✅ 远程Ollama连接 |
| 软件更新 | ✅ EXE 下载 + 版本切换 | ✅ 应用商店/下载页 |
| 系统代理 | ✅ Windows 系统代理 | ❌ 不适用 |
| CLAUDE.md | ✅ 本地文件读写 | ❌ 不适用 |
| 记忆管理 | ✅ 本地文件系统 | ❌ 不适用 |
| Git 集成 | ✅ 本地 Git 操作 | ❌ 不适用 |
| 终端执行 | ✅ 本地命令执行 | ❌ 不适用 |
| 插件系统 | ✅ 本地插件管理 | ❌ 不适用 |

---

## 十、后端 API 化迁移映射

### 10.1 从 backend.py 迁移（已完成 ✅）

| 旧代码 (backend.py) | 新位置 | 状态 |
|---------------------|--------|------|
| `EnvFileManager` | `services/config_service.py` → `routes/system.py` | ✅ |
| `OllamaProxyServer` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `ClaudeCliRunner` | `services/cli_service.py` → `routes/env.py` | ✅ |
| `list_openrouter_models()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `list_anthropic_models()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `list_ollama_models()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `list_zhipu_models()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `list_api_models()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `check_zhipu_api()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `check_api_service()` | `services/ai_service.py` → `routes/ai.py` | ✅ |
| `start_qwen2api()` | `services/env_service.py` → `routes/env.py` | ✅ |

### 10.2 从 main.py 迁移（已完成 ✅）

| 旧代码 (main.py) | 新位置 | 状态 |
|-------------------|--------|------|
| `ProjectManager` | `services/project_service.py` → `routes/project.py` | ✅ |
| `SoftwareUpdater` | `services/update_service.py` → `routes/version.py` | ✅ |
| `_ensure_single_instance()` | `api_main.py` 启动流程 | ✅ |
| `_self_deploy()` | `launcher.py`（保持不变） | ✅ |
| `_setup_webengine_env()` | 不再需要（pywebview 不用 QtWebEngine） | ✅ |
| `ChineseWebView` | 不再需要（前端自行处理国际化） | ✅ |
| `BackendBridge` (QWebChannel) | FastAPI 路由替代 | ✅ |
| UI 创建代码 | Vue 前端替代 | ✅ |

---

## 十一、流式对话实现（已实现）

### 11.1 SSE 方案（已实现）

**后端**（routes/ai.py）：

```python
@router.post("/chat")
async def chat(req: ChatRequest):
    async def event_generator():
        async for chunk in ai_service.stream_chat(req.model_dump()):
            yield chunk
    return EventSourceResponse(event_generator())
```

**前端**（platform/desktop.ts）：

```typescript
export async function chat(params, signal, onChunk) {
  const response = await fetch('/api/ai/chat/stream', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
    signal,
  })
  const reader = response.body!.getReader()
  const decoder = new TextDecoder()
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    const text = decoder.decode(value, { stream: true })
    for (const line of text.split('\n')) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6)
        if (data === '[DONE]') return
        onChunk(data)
      }
    }
  }
}
```

### 11.2 WebSocket 方案（后端已实现，前端待对接）

适用于终端交互、AI 对话等需要双向通信的场景。

**后端**（routes/ws.py）：

```python
@router.websocket("/ws/ai/{session_id}")
async def ai_websocket(websocket: WebSocket, session_id: str):
    await websocket.accept()
    # 双向流式对话
```

**前端**（待实现）：

```typescript
// 通过 useWebSocket composable 封装
const { isConnected, send, lastMessage } = useWebSocket(
  `ws://127.0.0.1:18080/ws/ai/${sessionId}`
)
```

---

## 十二、构建与打包

### 12.1 桌面版构建流程（已实现）

```bash
# 1. 构建前端
cd dev/web && npm run build    # → dist/

# 2. 复制前端到后端
# build_web.py 自动将 dist/ 复制到 app/static/

# 3. PyInstaller 打包
# build_web.py 自动执行 --onefile 模式打包
python build/build_web.py
```

**构建产物**：
- `dev/云集智能编程工作站-vYYYY.MM.DD.HHMM.exe`（~38MB 单文件）

### 12.2 手机版构建流程（待实现）

```bash
# 1. 构建前端
cd dev/web && npm run build

# 2. 同步到 Capacitor
npx cap sync

# 3. Android 构建
npx cap open android    # → Android Studio 构建 APK/AAB

# 4. iOS 构建（需要 macOS）
npx cap open ios        # → Xcode 构建 IPA
```

### 12.3 开发模式

```bash
# 后端开发模式（自动重载）
cd dev/app && python api_main.py --dev

# 前端开发模式（Vite HMR）
cd dev/web && npm run dev

# 前端访问 http://localhost:5173（自动代理 /api → :18080）
# 后端 API 文档 http://localhost:18080/docs
```

---

## 十三、错误处理策略

> v2.0 新增

### 13.1 后端错误处理

FastAPI 全局异常处理器，统一错误响应格式：

```python
# 统一错误响应格式
{
  "success": false,
  "error": "错误描述",
  "detail": "详细错误信息（仅开发模式）"
}
```

### 13.2 前端错误处理

| 层级 | 策略 | 实现 |
|------|------|------|
| API 请求 | axios/fetch 拦截器 | 统一 Toast 提示 |
| SSE 流式 | try-catch + onChunk 错误 | 流中断时显示错误消息 |
| WebSocket | 自动重连（3秒间隔） | useWebSocket 内置 |
| 路由 | Vue Router 导航守卫 | 未找到页面重定向 |
| 组件 | Vue onErrorCaptured | 组件级错误边界 |
| 全局 | Vue app.config.errorHandler | 兜底错误捕获 |

### 13.3 网络异常处理

| 场景 | 处理方式 |
|------|---------|
| 后端未启动 | 显示"正在启动..."引导页，轮询 /api/version/current |
| 请求超时 | 30秒超时，Toast 提示 |
| SSE 中断 | 显示"连接中断"提示，提供重试按钮 |
| WebSocket 断开 | 自动重连，最多3次 |

---

## 十四、性能优化策略

> v2.0 新增

### 14.1 前端性能

| 优化项 | 方案 | 优先级 |
|--------|------|--------|
| AI 对话长列表 | 虚拟滚动（vue-virtual-scroller） | 🟡 中 |
| Markdown 渲染 | 防抖渲染 + Web Worker | 🟡 中 |
| 代码高亮 | 懒加载 highlight.js 语言包 | 🟡 中 |
| 首屏加载 | 路由懒加载 + Vite 代码分割 | 🔴 高 |
| 图片资源 | 压缩 + CDN | 🟢 低 |
| 状态缓存 | Pinia 持久化插件（localStorage） | 🟡 中 |

### 14.2 后端性能

| 优化项 | 方案 | 状态 |
|--------|------|------|
| API 响应 | 异步 FastAPI + uvicorn | ✅ 已实现 |
| 文件操作 | 异步 aiofiles | ✅ 已实现 |
| SSE 流式 | async generator | ✅ 已实现 |
| 进程管理 | asyncio subprocess | ✅ 已实现 |

### 14.3 构建优化

| 优化项 | 方案 | 状态 |
|--------|------|------|
| 前端体积 | Vite tree-shaking + 压缩 | ✅ 已实现 |
| EXE 体积 | PyInstaller --onefile + UPX 压缩 | ✅ 已实现 |
| 静态资源 | gzip 压缩 | ✅ 已实现 |

---

## 十五、测试策略

> v2.0 新增

### 15.1 后端测试

| 类型 | 工具 | 覆盖范围 |
|------|------|---------|
| API 测试 | pytest + httpx | 所有 69 个 API 端点 |
| 服务测试 | pytest + mock | 7 个服务模块核心逻辑 |
| SSE 测试 | httpx + pytest-asyncio | 流式对话端点 |

### 15.2 前端测试

| 类型 | 工具 | 覆盖范围 |
|------|------|---------|
| 组件测试 | Vitest + @vue/test-utils | 核心组件 |
| Store 测试 | Vitest | Pinia stores |
| E2E 测试 | Playwright | 关键用户流程 |

### 15.3 集成测试

| 场景 | 验证内容 |
|------|---------|
| 桌面端启动 | EXE → FastAPI → pywebview → 前端加载 |
| AI 对话 | 发送消息 → SSE 流式 → Markdown 渲染 |
| 项目管理 | 创建 → 切换 → 会话保存/加载 |
| 环境部署 | 检测 → 安装 → 验证 |

---

## 十六、里程碑计划

### M1：后端 API 化 ✅ 已完成

- [x] FastAPI 路由框架搭建
- [x] AI 对话/模型管理 API（7个）
- [x] 环境部署 API（9个）
- [x] 项目管理 API（22个，远超原规划）
- [x] 系统设置 API（19个，远超原规划）
- [x] 版本更新 API（9个）
- [x] WebSocket 端点（3个）
- [x] pywebview 窗口集成
- [x] 构建流程（build_web.py）

### M2：前端界面 🔄 进行中（70%）

- [x] Vue 3 + Vant 4 项目搭建
- [x] 5 个核心页面实现
- [x] 平台适配层（desktop + mobile）
- [x] Pinia 状态管理（4个 store）
- [x] 暗黑主题 + 响应式布局
- [x] SSE 流式对话
- [ ] **修复 17 处前后端接口不一致** ← 当前重点
- [ ] 创建 YunJiPlugin.ts
- [ ] 拆分可复用组件（ChatBubble/CodeBlock/MarkdownRenderer）
- [ ] 创建 composables（useStream/useDevice/useWebSocket）
- [ ] 对接会话保存/删除功能
- [ ] 对接 CLAUDE.md / 记忆管理
- [ ] 对接版本管理 UI
- [ ] 对接 API Key 配置

### M3：桌面端整合 ✅ 已完成（90%）

- [x] pywebview + FastAPI 集成
- [x] 前端构建产物嵌入 EXE
- [x] --onefile 打包模式
- [x] 开发模式（Vite HMR + 后端热重载）
- [ ] 修复 API 接口不一致后的端到端测试

### M4：手机端开发 ⏳ 未开始

- [ ] YunJiPlugin.ts（Capacitor 原生插件）
- [ ] Android 项目配置
- [ ] AI 对话云端直连
- [ ] 手机端 UI 优化
- [ ] Android APK 构建
- [ ] iOS 项目配置（需要 macOS）

### M5：优化与发布 ⏳ 未开始

- [ ] 虚拟滚动优化
- [ ] 离线缓存
- [ ] 端到端测试
- [ ] 性能基准测试
- [ ] 用户文档
- [ ] 发布流程

---

## 十七、风险与对策

| 风险 | 影响 | 对策 | 状态 |
|------|------|------|------|
| WebView2 兼容性 | Win7 无 WebView2 | 安装时自动检测并安装 WebView2 Bootstrapper | ✅ 已处理 |
| SSE 流式兼容性 | 部分代理不支持 SSE | 备选方案：WebSocket 流式 | ✅ 已有备选 |
| 前后端接口不一致 | 功能无法正常使用 | 本文档第五章详细对照表，逐一修复 | 🔄 修复中 |
| EXE 启动速度 | PyInstaller --onefile 解压慢 | 使用 --onedir 或 UPX 压缩 | ✅ 已处理 |
| 手机端性能 | WebView 性能不如原生 | 使用 Vant 轻量组件 + 虚拟滚动 | ⏳ 待验证 |
| Capacitor 插件 | YunJiPlugin.ts 未实现 | 参考云集智能网联代理专家项目 | ⏳ 待开发 |
| AI 对话长文本 | DOM 节点过多导致卡顿 | 虚拟滚动 + 防抖渲染 | ⏳ 待实现 |

---

## 十八、参考项目

- **云集智能网联代理专家** — 已成功采用 pywebview + Capacitor 方案，桌面版和手机版均已上线
- **FastAPI 官方文档** — https://fastapi.tiangolo.com/
- **pywebview 官方文档** — https://pywebview.flowrl.com/
- **Capacitor 官方文档** — https://capacitorjs.com/
- **Vant 4 官方文档** — https://vant-ui.github.io/vant/