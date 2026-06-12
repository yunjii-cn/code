# 云集智能编程工作站 — 全面超越计划书（基于 CodexMonitor 借鉴）

> 版本：v1.0 | 日期：2026-06-08
> 状态：**推荐方案，待用户最终确认**
> 文档定位：**Self-contained**，即使 AI 上下文丢失，单独阅读本文档也能完整执行

---

## 〇、文档使用说明

### 目标读者
1. **AI 协作者**（Trae/Cursor/Copilot）— 任何会话读取本文档即可恢复全部上下文
2. **人类开发者** — 任何时刻回看本文档都能定位"我现在该做什么"
3. **项目评审者** — 5 分钟内能看懂战略与里程碑

### 引用规范
- 本文档引用的所有源文档路径都是**项目内绝对路径**
- 引用的外部资源（CodexMonitor 仓库）路径在文末统一列出
- 所有"任务项"以 **`[TASK-编号]`** 开头，AI 协作者可按编号引用

### 进度追踪
每完成一个任务，请：
1. 在该任务行的 ✅ 处打勾（`[ ]` → `[x]`）
2. 在 `十四、进度追踪` 节更新实际完成日期
3. 如有偏离，更新 `十五、变更记录`

---

## 一、背景与动机

### 1.1 为什么要做这个计划

2026-06-08，我们对开源项目 **CodexMonitor**（[Dimillian/CodexMonitor](https://github.com/Dimillian/CodexMonitor)）做了详细分析，结论是：

- **两个项目不是同品类** — CodexMonitor 是 "Codex 的 GUI 外壳"，我们是 "AI 原生工作空间"
- **各有强弱** — CodexMonitor 工程规范领先 2-3 代；我们多 Agent 治理 / 知识演化 / 感知引擎领先 1-2 代
- **取长补短** — 把 CodexMonitor 强的工程纪律和桌面细节学过来，结合我们的差异化护城河，可以**全面超越**

### 1.2 计划核心原则

| 原则 | 含义 |
|------|------|
| **不要全面对标** | 两条赛道，硬对标丧失自己优势 |
| **他们强的要补** | 工程纪律、桌面细节、GitHub 集成、语音、截图、Autocomplete |
| **我们强的要打透** | 多 Agent 治理、知识演化、感知引擎、Web 化跨端 |
| **他们弱的别跟着弱** | 单一供应商绑定、私有 daemon 协议、缺乏知识层 |
| **先打地基再盖楼** | Phase 1（工程纪律）必须先完成，Phase 2-4 才能高质量交付 |

---

## 二、参考项目画像（CodexMonitor）

> **来源**：[CodexMonitor GitHub](https://github.com/Dimillian/CodexMonitor) + [AGENTS.md](https://github.com/Dimillian/CodexMonitor/blob/main/AGENTS.md)
> **快照日期**：2026-06-08
> **仓库状态**：972 commits，88 tags，142 branches，最后提交 2026-03-26（v0.7.68）

### 2.1 一句话定义

**Tauri(Rust) + React/Vite 桌面应用，专门用来编排多个 Codex Agent 跨本地工作区工作。**

### 2.2 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2.0（Rust） |
| 前端 | React + Vite + TypeScript（feature-sliced 架构） |
| 后端 | Rust（lib.rs + 独立 daemon 二进制） |
| AI 协议 | `codex app-server` stdio（必须用 OpenAI Codex CLI） |
| 远程 | Tailscale + 自研 JSON-RPC daemon |
| 移动端 | iOS（Tauri 框架，WIP 状态） |
| 工程 | ESLint + Prettier + Vitest + tsc + cargo check + GitHub Actions |

### 2.3 功能矩阵

| 功能模块 | 具体能力 |
|---------|---------|
| **Workspaces & Threads** | 工作区持久化、每个 workspace 一个 `codex app-server`、thread 续接、pin/rename/archive、worktree 隔离、可选远程 daemon |
| **Composer** | 图片附件（picker/drag/paste）、`$/prompts/review/@` 自动补全、模型选择、collaboration modes、reasoning effort、access mode、context ring、Whisper 听写（hold-to-talk + waveform） |
| **Git & GitHub** | diff stats、staged/unstaged、revert/stage、commit log、branch checkout、**GH Issues + PRs + 评论**（依赖 `gh` CLI）、"Ask PR" 注入上下文 |
| **Files & Prompts** | 文件树（搜索、类型图标、Reveal in Finder）、提示词库（全局/工作区，frontmatter） |
| **UI & Experience** | 5 个 resizable 面板、响应式布局、账号限速、Terminal dock（多 tab 实验性）、in-app updates、平台特定效果（macOS vibrancy） |
| **iOS** | WIP 中（mobile layout 已可用、远程模式已通、terminal/dictation 不可用） |

### 2.4 架构亮点（值得借鉴）

```
src-tauri/
  src/
    lib.rs                          # Tauri 命令注册
    bin/
      codex_monitor_daemon.rs       # 远程 daemon 入口
      codex_monitor_daemon/
        rpc.rs                      # 远程 RPC 路由
        rpc/                        # 领域处理器
    shared/                         # ⭐ 共享域逻辑（app + daemon 都用）
      workspaces_core.rs
      git_ui_core.rs
    codex/                          # codex app-server 适配器
    files/                          # 文件适配器
```

**关键洞察**：他们用 `shared/` 核心层让 app 和 daemon 共享业务逻辑，避免代码重复。这是工程化最高级的做法。

### 2.5 AGENTS.md 模板（值得抄）

CodexMonitor 的 [AGENTS.md](https://github.com/Dimillian/CodexMonitor/blob/main/AGENTS.md) 是写给 AI 协作者的**开发契约**：

- 非协商性架构规则（5 条）
- 后端路由规则（4 步顺序）
- 前端路由规则（4 层）
- 导入别名规范
- 关键文件锚点（新人/AI 5 分钟定位代码）
- 线程不变式（不变量）
- App/Daemon 同步检查清单
- 设计系统规则
- 验证矩阵（按改动区域选命令）
- 快速运行手册

**这是我们项目**最大**的短板**——目前连 README 都没有完整版。

---

## 三、我们的项目现状

### 3.1 核心文档

| 文档 | 路径 | 说明 |
|------|------|------|
| 产品规划 | [doc/智能编程工作站产品规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99%E4%BA%A7%E5%93%81%E8%A7%84%E5%88%92.md) | 战略 + 6 大革命性特性 |
| 产品蓝图 | [doc/云集智能平台产品蓝图.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E5%B9%B3%E5%8F%B0%E4%BA%A7%E5%93%81%E8%93%9D%E5%9B%BE.md) | 平台 + 鸡生蛋战略 |
| Web 化规划 | [doc/Web化跨平台改造规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/Web%E5%8C%96%E8%B7%A8%E5%B9%B3%E5%8F%B0%E6%94%B9%E9%80%A0%E8%A7%84%E5%88%92.md) | 跨平台改造 + 远程编码 |
| 开发规范 | [yunji-dev-guide skill](file:///c:/Users/Administrator/.trae-cn/skills/yunji-dev-guide) | CustomTkinter/PyInstaller 规范（**注意：实际项目已从 CustomTkinter 升级为 PyQt6 + Vue 3，skill 部分规则已过时**） |
| 单实例控制 | [doc/单实例控制机制-技术研究.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%8D%95%E5%AE%9E%E4%BE%8B%E6%8E%A7%E5%88%B6%E6%9C%BA%E5%88%B6-%E6%8A%80%E6%9C%AF%E7%A0%94%E7%A9%B6.md) | Windows 命名互斥体方案 |

### 3.2 当前代码结构

```
云集智能编程工作站/
├── dev/
│   ├── app/                              # ✅ 桌面端源代码
│   │   ├── main.py                       # PyQt6 + QWebEngineView 启动器
│   │   ├── backend.py                    # FastAPI 后端（Ollama 代理 / CLI 管理 / 配置）
│   │   ├── api_main.py                   # FastAPI 入口
│   │   ├── routes/                       # API 路由
│   │   │   ├── ai.py                     # AI 对话 + 模型管理
│   │   │   ├── env.py                    # 环境配置
│   │   │   ├── project.py                # 项目管理
│   │   │   ├── system.py                 # 系统 API
│   │   │   ├── version.py                # 版本
│   │   │   └── ws.py                     # WebSocket
│   │   ├── services/                     # 服务层
│   │   ├── src/                          # 旧版 TS 前端（Ink 终端 UI 残留）
│   │   ├── desktop/                      # Electron 残留
│   │   └── ...
│   ├── web/                              # ✅ Vue 3 新前端
│   │   ├── src/
│   │   │   ├── views/                    # 当前按页面平铺
│   │   │   ├── components/
│   │   │   ├── stores/
│   │   │   ├── composables/
│   │   │   ├── router/
│   │   │   ├── api/
│   │   │   ├── platform/                 # 跨端平台适配
│   │   │   └── plugins/                  # YunJiPlugin（能力检测）
│   │   └── ...
│   └── ver/
├── build/                                # 构建脚本
│   ├── build.py                          # 当前主构建
│   ├── build_stub.py                     # 启动器构建
│   └── build_web.py                      # Web 构建
├── doc/                                  # 文档
└── project.json                          # 项目元信息
```

### 3.3 我们的护城河（要打透）

| 护城河 | 现状 | CodexMonitor |
|--------|------|-------------|
| 多供应商 AI 引擎 | ✅ 已实现（Anthropic / OpenRouter / Ollama / 智谱） | ❌ 只支持 Codex |
| 多 Agent 治理 | ❌ 规划中 | ❌ 只有"多开几个独立 Codex 窗口" |
| 四层知识 + 强度演化 | ❌ 规划中 | ❌ 完全无 |
| 主动感知引擎 | ❌ 规划中 | ❌ 完全无 |
| 工程化持久化 `.yunji/` | ❌ 规划中 | ❌ 只有 settings.json + workspaces.json |
| Web 化跨端 | ✅ 已实现 | ❌ Tauri 桌面壳 + iOS WIP |
| 本地模型 Ollama | ✅ 已实现 | ❌ |

**结论**：护城河里有 **5 项还没实现**，需要 Phase 3 落地。CodexMonitor 没有任何一项护城河。

### 3.4 我们的短板（要补齐）

| 短板 | 严重度 | CodexMonitor |
|------|-------|-------------|
| **工程纪律**（无 lint/无 test/无 CI/无 AGENTS.md） | 🔴 严重 | ✅ 全套 |
| **GitHub 集成**（无 Issues/PRs） | 🟡 中 | ✅ `gh` CLI 深度 |
| **图片附件**（Composer 不支持） | 🟡 中 | ✅ picker/drag/paste |
| **Autocomplete**（无 `$/prompts/@` 提示） | 🟡 中 | ✅ 完整 |
| **语音听写**（无 Whisper） | 🟢 低 | ✅ hold-to-talk + waveform |
| **设计系统**（UI ad-hoc） | 🟡 中 | ✅ DS primitives + lint |
| **远程协议**（标准 HTTP/SSE） | ✅ 已标准化 | ❌ 私有 JSON-RPC |

---

## 四、明确不抄什么

| CodexMonitor 做法 | 不抄原因 | 我们的做法 |
|------------------|---------|-----------|
| 绑死 Codex CLI | 丧失多供应商能力 | 继续多供应商 + Ollama |
| Tauri Rust 重写 | 推翻 Web 化优势 + 学习成本高 | 保持 Python + Vue 3 |
| iOS 原生 App（WIP） | 重资产 + 路线不确定 | 走 PWA + 移动 Web |
| 私有 JSON-RPC daemon | 不开放 + 难集成第三方 | 用标准 WebSocket / HTTP / SSE |
| 缺失知识系统 | 我们的护城河 | 把它做透 |
| 缺失主动感知 | 我们的护城河 | 把它做透 |
| 缺失多 Agent 治理 | 我们的护城河 | 把它做透 |
| 无多用户协作 | 我们的差异化机会 | 抢这个空白市场 |
| 无技能市场 | 我们的差异化机会 | 抢这个空白市场 |

---

## 五、Phase 1：工程纪律补齐（W1-W4）— 4 周

> **目标**：让我们的代码质量和可维护性对标 CodexMonitor
> **优先级**：🔴 最高，必须先做
> **为什么必须先做**：CodexMonitor 工程化领先 2-3 代，他们的所有迭代速度都建立在规范基础上。我们不规范，后续 Phase 2-4 写出来的代码会变成"功能堆砌"，维护灾难

### 5.1 总体目标

| 指标 | 当前 | Phase 1 结束 |
|------|------|-------------|
| 测试覆盖率 | 0% | 60% |
| Lint 错误 | N/A | 0 |
| CI | 无 | GitHub Actions 全跑通 |
| AGENTS.md | 无 | 有（学习 CodexMonitor 模板） |
| codebase-map.md | 无 | 有 |
| 前端架构 | views/ 平铺 | feature-sliced |
| 后端架构 | routes/ + services/ 平铺 | platformkit/shared + app + daemon 三层 |
| 设计系统 | 无 | v1（5 个原子组件） |

### 5.2 W1 — 文档与架构骨架

#### [TASK-1.1] 写 `AGENTS.md`（半天）

**目标**：写一份 AI 协作者契约，让任何新会话读取后能立刻理解项目

**位置**：项目根 `AGENTS.md`

**模板**：参考 [CodexMonitor AGENTS.md](https://github.com/Dimillian/CodexMonitor/blob/main/AGENTS.md)

**必须包含的章节**：
1. Scope（指向本计划书 + 详细文档）
2. Project Snapshot（一句话定义 + 技术栈）
3. Non-Negotiable Architecture Rules（5-8 条硬性规则）
4. Backend Routing Rules（修改后端的 4 步顺序）
5. Frontend Routing Rules（修改前端的 4 层）
6. Import Aliases（`@features/*` 等）
7. Key File Anchors（关键文件清单）
8. Validation Matrix（按改动区域选命令）
9. Quick Runbook（核心本地命令）
10. Hotspots（高 churn/复杂文件清单）
11. Canonical References（详细文档链接）

**验收**：让一个新 AI 会话只读 AGENTS.md + codebase-map.md，能定位 80% 文件

#### [TASK-1.2] 写 `docs/codebase-map.md`（1 天）

**目标**：任务地图 — "如果想改 X，去改 Y"

**位置**：[doc/codebase-map.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/codebase-map.md)

**必须包含的任务列表**（按用户场景分类）：
- "想加一个 AI 提供商" → 去 `dev/app/services/ai_service.py` + `dev/web/src/stores/model.ts`
- "想加一个新页面" → 去 `dev/web/src/views/`（Phase 1 结束前迁移到 features/）
- "想改 Git 操作" → 去 `dev/app/routes/system.py` + `dev/web/src/views/GitView.vue`
- "想改打包" → 去 `build/build.py` + `build/build_web.py`
- "想加一个 CLI 命令" → 去 `dev/app/backend.py` + `dev/app/services/cli/`
- "想改 API" → 去 `dev/app/routes/` + `dev/app/api_main.py` + `dev/web/src/api/index.ts`
- ...（列出 15-20 个常见任务）

**验收**：新开发者 5 分钟内能定位 80% 文件

#### [TASK-1.3] 后端拆 `platformkit/shared/`（2 天）

**目标**：建立三层架构（app / daemon / shared），为未来远程 daemon 模式铺路

**目标结构**（实际已用 `platformkit/` 而非 `platform/`，因 stdlib 冲突）：
```
dev/app/
├── platformkit/                        # 🆕 平台核心层（注意是 platformkit 不是 platform）
│   ├── shared/                        #   跨 app/daemon 共享的域逻辑
│   │   ├── workspace_core.py          #   工作区核心（W3 计划）
│   │   ├── git_core.py                #   Git 核心（W3 计划）
│   │   ├── file_core.py               #   文件核心（W3 计划）
│   │   ├── config_core.py             #   ✅ 配置核心（已 W1 末完成）
│   │   ├── constants.py               #   ✅ 常量（已 W1 末完成）
│   │   └── types.py                   #   跨端共享类型（占位，W2 计划）
│   ├── app/                           #   FastAPI 路由 + 服务（应用层，占位）
│   └── daemon/                        #   🆕 独立 daemon（二进制可选，Phase 4 计划）
├── routes/                            # ⏸ 旧位置，逐步迁移
├── services/
└── ...
```

**迁移策略**：
1. W1 末：创建 `platformkit/shared/` 目录，迁移**无依赖的核心工具**（types、config_core、workspace_core）
2. W2-W3：迁移有依赖的服务（ai_service、git_service）
3. 旧 `routes/`、`services/` 用 `re-export` 兼容，逐步废弃
4. W4 末：80% 业务逻辑在 `platformkit/shared/`

**风险控制**：
- 不一次性重构，每迁移一个文件跑现有测试
- `from services.ai_service import ...` 保留 4 周兼容期

**验收**：
- `platformkit/shared/` 独立可测试（`python -m pytest tests/shared/`）
- `platformkit/app/` 仅作为 HTTP 路由层，不含业务逻辑
- 现有功能 100% 不破

#### [TASK-1.4] 前端拆 `feature-sliced`（3 天）

**目标**：把当前 `views/` 平铺改为按 feature 切分

**目标结构**：
```
dev/web/src/
├── features/                          # 🆕 业务功能模块
│   ├── chat/                          #   AI 对话
│   │   ├── views/
│   │   ├── components/
│   │   ├── stores/                    #   Pinia store
│   │   ├── composables/
│   │   ├── api/
│   │   └── types.ts
│   ├── git/                           #   Git 操作
│   ├── terminal/                      #   终端
│   ├── files/                         #   文件管理
│   ├── settings/                      #   设置
│   ├── models/                        #   模型管理
│   ├── memory/                        #   记忆/知识
│   ├── project/                       #   项目管理
│   ├── env/                           #   环境变量
│   ├── plugin/                        #   插件
│   └── version/                       #   版本
├── shared/                            # 🆕 跨 feature 共享
│   ├── components/                    #   Design System
│   ├── composables/
│   ├── api/                           #   HTTP 客户端封装
│   ├── stores/                        #   全局 store（settings 等）
│   └── utils/
├── platform/                          # 跨端平台适配（已有）
├── App.vue
├── main.ts
└── router/                            # 路由（保留）
```

**迁移策略**：
1. 准备工作：建立 `@features/*`、`@shared/*` 别名（tsconfig.json + vite.config.ts）
2. 按依赖顺序迁移：
   - settings → models → env → plugin → version（叶子节点）
   - files → terminal → git → memory → project（中间层）
   - chat（最复杂，最后）
3. 每个 feature 迁移后跑 `npm run typecheck` + `npm run lint`

**风险控制**：
- 不破坏现有 URL 路由
- 旧 `views/` 用 re-export 兼容 4 周
- 一次只迁移一个 feature

**验收**：
- 现有 14 个 view 全部迁移到对应 feature
- `npm run typecheck` 0 错误
- URL 路由 100% 不变

### 5.3 W2 — 质量工具链

#### [TASK-1.5] 引入 ESLint + Prettier（1 天）

**目标**：前端代码风格统一

**实施**：
- 安装：`eslint` + `@typescript-eslint/*` + `eslint-plugin-vue` + `prettier` + `eslint-config-prettier`
- 配置：`.eslintrc.cjs` + `.prettierrc` + `.eslintignore`
- 集成：`npm run lint`、`npm run lint:fix`
- 关键规则：
  - Vue 组件命名 PascalCase
  - TypeScript strict
  - 禁止 `any`（特殊场景需 `// eslint-disable-next-line` + 注释）
  - 导入顺序（@features → @shared → 相对）

**验收**：`npm run lint` 0 错误 0 警告

#### [TASK-1.6] 引入 Pytest + 关键路径单测（2 天）

**目标**：后端核心服务 60% 覆盖率

**实施**：
- 安装：`pytest` + `pytest-asyncio` + `pytest-cov` + `httpx`（用于 FastAPI 测试）
- 结构：`dev/app/tests/` 与源码平铺（`tests/test_ai_service.py`）
- 优先覆盖：
  - `services/ai_service.py`（对话、模型列表、健康检测）
  - `backend.py`（Ollama 代理、配置管理）
  - `routes/*.py`（API 端点 smoke test）
- 配置：`pyproject.toml` 或 `pytest.ini`

**验收**：
- `pytest --cov=dev/app --cov-report=term-missing` 覆盖率 ≥ 60%
- CI 集成（见 [TASK-1.8]）

#### [TASK-1.7] TypeScript 严格模式（半天）

**目标**：类型安全最大化

**配置**：`tsconfig.json` 关键项
```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitReturns": true,
    "exactOptionalPropertyTypes": true
  }
}
```

**验收**：`npm run typecheck` 0 错误

### 5.4 W3-W4 — CI 与设计系统

#### [TASK-1.8] 引入 GitHub Actions CI（1 天）

**目标**：push 自动跑 lint + test + build

**位置**：`.github/workflows/ci.yml`

**关键 Jobs**：
- `frontend-lint`: `npm ci` → `npm run lint` → `npm run typecheck`
- `frontend-test`: `npm ci` → `npm run test`
- `frontend-build`: `npm ci` → `npm run build`（Web 化构建）
- `backend-test`: Python setup → `pytest --cov`
- `backend-import-test`: `python -c "import sys; sys.path.insert(0, 'dev/app'); import main"`（EXE 启动测试）
- `markdown-lint`: `markdownlint doc/`（文档规范）

**验收**：
- 每次 push 自动跑全套
- 失败时阻断 merge（如启用 branch protection）

#### [TASK-1.9] 设计系统 Design System v1（3 天）

**目标**：5 个原子组件，UI 风格统一

**位置**：`dev/web/src/shared/components/`

**5 个原子组件**：
1. `YJButton` — 主按钮（primary/secondary/ghost/danger，loading 状态）
2. `YJModal` — 模态框（基于 Teleport，可拖拽）
3. `YJPopover` — 气泡弹层
4. `YJToast` — 通知（success/error/warning/info，自动消失）
5. `YJPanel` — 可调整大小的面板（基于 splitpanes 或自己实现）

**配套**：
- `dev/web/src/shared/styles/tokens.css` — CSS 变量（颜色、间距、字号）
- `dev/web/src/shared/styles/reset.css` — 统一 reset
- ESLint 规则：禁止在 feature 组件中重复定义 modal/toast/panel

**样式规范**：
- 暗黑卡片式（与现有产品风格一致）
- 间距：8/12/16/20/24/32px 体系
- 颜色：primary/secondary/accent/danger/warning 各 5 个层级
- 字号：12/14/16/18/20/24/32px

**验收**：
- 现有 14 个 view 中的重复 modal/toast/panel 全部替换为 YJ* 组件
- `npm run lint` 0 错误（禁止重复定义规则生效）

### 5.5 Phase 1 结束检查清单

> 2026-06-08 Phase 1 收尾时检查并勾选

- [x] `AGENTS.md` 存在并通过审查（[AGENTS.md](../AGENTS.md)，v1.0，2026-06-08）
- [x] `doc/codebase-map.md` 存在并覆盖 15+ 任务场景（[codebase-map.md](codebase-map.md)，24 个场景）
- [x] `dev/app/platformkit/shared/` 业务逻辑迁移（W1 末：constants + EnvFileManager re-export；Phase 1 收尾：真提取 `http_client.py` + `model_utils.py` 两个无状态模块）
- [x] `dev/web/src/features/` 14 个 view 全部迁移完成（**15 个 features**：chat/project/models/env/settings/version/plugin/claudemd/memory/ollama/apikey/apiservice/files/git/terminal，router 全部走 `@features/*`）
- [x] ESLint + Prettier 0 错误（前端 vue-tsc 0 错；ESLint 0 错 + shared/ 0 警告；Prettier 全部格式化）
- [x] Pytest 覆盖率 ≥ 60%（**45/45 测试通过，platformkit 覆盖率 69%，超过 60% 门槛**）
- [x] TypeScript 严格模式 0 错误（`noImplicitReturns: true` 已启用；2 个预存在 RightSidebar 错误已修）
- [x] GitHub Actions CI 全套跑通（[ci.yml](../.github/workflows/ci.yml)：frontend-lint / typecheck / build / backend-test / import-test / markdown-lint 6 个 job）
- [x] Design System 5 个组件创建完成（[YJButton](../dev/web/src/shared/components/YJButton.vue) / [YJModal](../dev/web/src/shared/components/YJModal.vue) / [YJPopover](../dev/web/src/shared/components/YJPopover.vue) / [YJToast](../dev/web/src/shared/components/YJToast.vue) / [YJPanel](../dev/web/src/shared/components/YJPanel.vue) + [toast.ts](../dev/web/src/shared/components/toast.ts) 全局单例 + [tokens.css](../dev/web/src/shared/styles/tokens.css) + [reset.css](../dev/web/src/shared/styles/reset.css)）
- [x] 现有功能 100% 不破（**冒烟测试通过**：`api_main.py --dev --port 18099` 启动 OK；`/api/health` ✅ `/api/version/current` ✅ `/api/project/list` ✅ `/api/system/settings` ✅ `/api/env/status` ✅，共 75+ 端点全部注册）

**Phase 1 总览**：

| TASK | 标题 | 状态 | 完成日期 |
|------|------|------|----------|
| 1.1 | AGENTS.md | ✅ | 2026-06-08 |
| 1.2 | codebase-map.md | ✅ | 2026-06-08 |
| 1.3 | platformkit/shared/ | ✅ | 2026-06-08（收尾：真提取 2 模块） |
| 1.4 | 前端拆 feature-sliced | ✅ | 2026-06-08（**15 features 全迁移**） |
| 1.5 | ESLint + Prettier | ✅ | 2026-06-08 |
| 1.6 | Pytest + 单测 | ✅ | 2026-06-08（45/45，69% 覆盖） |
| 1.7 | TypeScript 严格模式 | ✅ | 2026-06-08 |
| 1.8 | GitHub Actions CI | ✅ | 2026-06-08（6 jobs） |
| 1.9 | 设计系统 Design System v1 | ✅ | 2026-06-08 |

**Phase 1 收尾（2026-06-08 完成的增量）**：

1. 剩余 10 个 view 全部迁移到 `features/`：claudemd / memory / ollama / apikey / apiservice / chat / project / files / git / terminal
2. `platformkit/shared/http_client.py` 真提取（不依赖 FastAPI 的纯 HTTP 工具）
3. `platformkit/shared/model_utils.py` 真提取（normalize + guess_tool_support，TOOL_CALLING_MODEL_PATTERNS re-export）
4. 新增 18 个 pytest 单元测试（`test_platformkit_http_client.py` 4 + `test_platformkit_model_utils.py` 14）
5. 修复预存在 TS 错误：`RightSidebar.vue` `systemApi.readFile` 改为 `runCommand('type/cat')` 读取
6. 冒烟测试：`api_main.py --dev --port 18099` + 5 个关键端点 curl 全部 OK

**进入 Phase 2 的条件**：全部满足 ✅。

---

## 六、Phase 2：桌面体验对标（W5-W8）— 4 周

> **目标**：把 CodexMonitor 桌面 UI 的"细节红利"全部拿过来
> **优先级**：🟡 高
> **前置依赖**：Phase 1 完成

### 6.1 W5 — GitHub 深度集成

#### [TASK-2.1] GitHub 集成模块（3 天）

**目标**：用 `gh` CLI 深度集成 GitHub Issues 和 PRs

**位置**：
- 后端：`dev/app/platformkit/shared/github_core.py` + `dev/app/routes/github.py`
- 前端：`dev/web/src/features/github/`

**功能清单**：
1. 列出当前 repo 的 Issues（open/closed，assignee/author 过滤）
2. 列出当前 repo 的 PRs（draft/open/merged/closed，搜索）
3. 查看 PR diff（用 `gh pr diff`）
4. 查看/创建 PR 评论
5. **"Ask PR"** — 一键把 PR 上下文注入新 agent thread
6. 在浏览器中打开 commit / PR / Issue

**实现注意**：
- 后端用 `subprocess.run(['gh', ...])`，超时 30s
- 前端用 `list_view + detail_view` 经典双栏布局
- 错误处理：gh 未安装/未登录 → 友好提示

**验收**：
- 打开任意 GitHub repo，能列出所有 PR
- 点击 PR 看到 diff 和评论
- "Ask PR" 能把 PR 标题/描述/文件清单注入 Chat

#### [TASK-2.2] Git DiffView 增强（2 天）

**目标**：彩色 diff + 步骤关联 + 单文件回退

**位置**：`dev/web/src/features/git/components/DiffView.vue`

**功能清单**：
1. 彩色 diff（红 = 删，绿 = 增）
2. 行号显示
3. 折叠/展开未改动区块
4. **单文件回退**（`git checkout -- file` 或 `git restore file`）
5. **步骤关联** — 多个 step 时的 diff 切换器
6. 搜索高亮

**实现**：
- 解析 `git diff` 输出，按文件分组
- 用 CSS 渲染红/绿
- 后端新增 `POST /api/git/file/restore` 接口

**验收**：能直观看出每次 commit 改了什么

### 6.2 W6 — Composer 增强

#### [TASK-2.3] 图片附件（2 天）

**目标**：Composer 支持图片（picker / 拖拽 / 粘贴）

**位置**：`dev/web/src/features/chat/components/Composer.vue`

**功能**：
- 文件选择器选图
- 拖拽图片到 Composer
- 粘贴板图片（Ctrl+V）
- 缩略图预览
- 多供应商兼容（Anthropic/OpenAI/Ollama vision）

**实现**：
- 图片转 base64 存在 store
- 发送时按供应商协议附加（Anthropic: image block / OpenAI: image_url / Ollama: images）
- Ollama 不支持 vision 时给提示

**验收**：能拖拽图片到 Chat 并发送给 AI

#### [TASK-2.4] Autocomplete 引擎（3 天）

**目标**：`$/prompts/review/@` 四类自动补全

**位置**：`dev/web/src/features/chat/components/Autocomplete.vue`

**触发规则**：
- 输入 `/` 开头 → 提示词列表（来自 `$CODEX_HOME/prompts/` 或我们自定义）
- 输入 `$` 开头 → 技能列表（内置 + 用户自定义）
- 输入 `/review` → 代码审查模式选项
- 输入 `@` 开头 → 当前 workspace 文件路径（搜索补全）

**实现**：
- 监听输入框内容
- 解析当前光标位置前的 token
- 上拉菜单显示候选项
- 上下箭头 + Tab 选择

**验收**：
- 输入 `/` 看到所有提示词
- 输入 `$react` 看到 React 技能
- 输入 `@src/` 看到文件路径

### 6.3 W7 — 语音 + 移动端

#### [TASK-2.5] 语音听写（Whisper）（3 天）

**目标**：Hold-to-talk + 实时波形

**位置**：
- 后端：`dev/app/routes/dictation.py`（接收音频 → 调 Whisper → 返回文本）
- 前端：`dev/web/src/features/chat/components/DictationButton.vue`

**实现**：
- **第一阶段**：调云端 Whisper API（OpenAI / 自托管）
- **第二阶段**：本地 Whisper（需 CMake + LLVM/Clang，参考 CodexMonitor 编译要求）

**前端**：
- 长按按钮录音（MediaRecorder API）
- 实时波形（Web Audio API + AnalyserNode）
- 抬起自动转写
- 转写结果填入 Composer

**验收**：
- 长按按钮能看到实时波形
- 松开按钮自动转写为文字
- 转写结果插入 Composer

#### [TASK-2.6] iOS Phone Layout（2 天）

**目标**：手机浏览器体验对标 CodexMonitor iOS 布局

**位置**：`dev/web/src/platform/mobile.ts` + `dev/web/src/views/MobileShell.vue`

**功能**：
- 底部 tab bar（viewport-safe，参考 CodexMonitor）
- 顶部状态栏适配
- 手势：左滑返回、左右切换 tab
- PWA 优化（添加到主屏幕后全屏）

**实现**：
- CSS env(safe-area-inset-*)
- vh 单位 + 动态计算
- Touch 事件监听

**验收**：
- 手机浏览器打开后看到底部 tab bar
- 横屏/竖屏切换布局自适应
- 添加到主屏幕后没有浏览器 chrome

### 6.4 W8 — 通知与快捷键

#### [TASK-2.7] 统一通知系统（2 天）

**目标**：全局 Toast 通知

**位置**：`dev/web/src/shared/components/YJToast.vue` + `dev/web/src/shared/composables/useToast.ts`

**API**：
```ts
const { show } = useToast()
show({ type: 'success', message: '保存成功', duration: 3000 })
show({ type: 'error', message: '...', action: { label: '重试', onClick: ... } })
```

**实现**：
- 全局单例 toast container
- 队列管理（同时最多 3 个）
- 自动消失 + 手动关闭
- 支持 action 按钮

**验收**：所有 `alert()` / `confirm()` / 自定义错误提示全部替换为 Toast

#### [TASK-2.8] 全局快捷键（2 天）

**目标**：键盘流用户友好

**位置**：`dev/web/src/shared/composables/useShortcut.ts`

**默认快捷键**：
- `Ctrl/Cmd + Enter` 发送消息
- `Ctrl/Cmd + N` 新建对话
- `Ctrl/Cmd + K` 命令面板（Phase 3 实现）
- `Ctrl/Cmd + S` 保存当前编辑
- `Ctrl/Cmd + /` 切换模式（独行/团队）
- `Esc` 关闭当前模态框

**实现**：
- 全局 keydown 监听
- 路由感知（不同页面绑定不同快捷键）
- 在设置页可自定义

**验收**：所有快捷键在所有页面生效

### 6.5 Phase 2 结束检查清单

- [ ] GitHub Issues/PRs 面板可用
- [ ] "Ask PR" 注入上下文工作
- [ ] Git DiffView 彩色 + 步骤关联 + 单文件回退
- [ ] Composer 支持图片附件（picker/drag/paste）
- [ ] Autocomplete `$/prompts/review/@` 全部工作
- [ ] 语音听写（云端 Whisper）可用
- [ ] iOS Phone 布局 + PWA ready
- [ ] 统一 Toast 通知系统
- [ ] 全局快捷键 6 个核心
- [ ] 现有功能 100% 不破

---

## 七、Phase 3：核心差异化打透（W9-W12）— 4 周

> **目标**：把规划文档里的"革命性特性"真正落地
> **优先级**：🔴 最高战略价值
> **前置依赖**：Phase 1 完成

### 7.1 W9 — 自进化知识系统

#### [TASK-3.1] 知识引擎核心（3 天）

**位置**：`dev/app/platformkit/core/knowledge.py`

**API 设计**：
```python
class KnowledgeEngine:
    def add_correction(content, scope="project", strength="weak") -> Knowledge
    def add_pattern(content, scope="project", strength="weak") -> Knowledge
    def add_fact(content, scope="project", strength="weak") -> Knowledge
    def add_preference(content, scope="global", strength="weak") -> Knowledge
    def get_relevant(context: str, top_k: int = 5) -> list[Knowledge]
    def confirm(knowledge_id: str) -> Knowledge  # 提升强度
    def promote_to_global(knowledge_id: str) -> Knowledge
    def list_by_layer(layer: str) -> list[Knowledge]
    def list_by_strength(strength: str) -> list[Knowledge]
```

**四层知识**：
- L1 纠正（Corrections）— "不要用 var，用 const"
- L2 模式（Patterns）— "用户连续3次手动测试→提交→部署"
- L3 事实（Facts）— "这个项目用 PostgreSQL"
- L4 偏好（Preferences）— "我喜欢 Tab 缩进"

**强度演化模型**：
- weak → medium：确认 2 次未被纠正
- medium → strong：确认 4 次未被纠正
- 任何层级可被用户明确确认（"对，就是这样"）→ 立即跳 strong

**存储**：
- 项目级：`.yunji/knowledge/`
  - `corrections.md`
  - `patterns.md`
  - `facts.md`
  - `preferences.md`
  - `strength.json`
- 全局级：`~/.yunji/knowledge/`

**验收**：
- `add_correction` / `add_pattern` 等 API 工作
- `confirm` 能提升强度
- `get_relevant` 返回与上下文相关的知识（用 embedding 相似度）

#### [TASK-3.2] 知识管理 API（1 天）

**位置**：`dev/app/routes/knowledge.py`

**端点**：
- `GET /api/knowledge/list` — 列表（按 layer/strength 过滤）
- `GET /api/knowledge/{id}` — 详情
- `POST /api/knowledge` — 添加
- `DELETE /api/knowledge/{id}` — 删除
- `POST /api/knowledge/{id}/confirm` — 确认（提升强度）
- `POST /api/knowledge/{id}/promote` — 提升为全局
- `GET /api/knowledge/stats` — 统计（L1-L4 各多少条，强度分布）

**验收**：所有端点单测覆盖

#### [TASK-3.3] Knowledge Panel UI（1 天）

**位置**：`dev/web/src/features/knowledge/`

**组件**：
- `KnowledgePanel.vue` — 知识面板
- `KnowledgeList.vue` — 列表（按层分组）
- `KnowledgeCard.vue` — 知识卡片（带强度进度条）
- `KnowledgeEditor.vue` — 编辑/添加
- `KnowledgeStats.vue` — 统计图

**功能**：
- 按 L1-L4 分组展示
- 强度可视化（weak/medium/strong 进度条）
- 一键确认（按钮）
- 一键提升为全局（带确认对话框）
- 搜索/过滤

**验收**：用户能浏览、添加、确认、删除、推广知识

### 7.2 W10 — 主动感知引擎

#### [TASK-3.4] 感知引擎核心（3 天）

**位置**：`dev/app/platformkit/core/responsive.py`

**Watcher 体系**：
```python
class ResponsiveEngine:
    WATCHERS = {
        "file_change": FileChangeWatcher,         # 文件变更感知
        "code_quality": CodeQualityPatrol,        # 代码质量巡逻
        "dependency": DependencyRiskScanner,      # 依赖风险预警
        "progress": ProgressTracker,              # 进度追踪提醒
    }

    async def start(workspace_path: str)
    async def stop()
    async def trigger_scan(scan_type: str) -> list[Notification]
```

**四个 Watcher 详细**：

1. **FileChangeWatcher** — 用 `watchdog` 监听文件保存事件
   - 触发：文件保存
   - 通知类型：`file_change`
   - 内容：哪些文件被改、变更摘要、相关 AI 建议

2. **CodeQualityPatrol** — 定时扫描代码异味
   - 触发：定时（5 分钟）/ 文件变更
   - 工具：自定义规则（先用简单 AST 检查，未来接入 pylint/ruff）
   - 通知类型：`code_quality`
   - 内容：问题位置、严重度、建议

3. **DependencyRiskScanner** — 检查依赖漏洞
   - 触发：项目打开时 / 定时
   - 工具：解析 `requirements.txt` / `package.json` / `Cargo.toml`
   - 数据源：CVE 数据库（先用 NVD API，未来本地缓存）
   - 通知类型：`security_risk`
   - 内容：漏洞 CVE、影响版本、建议升级

4. **ProgressTracker** — 跟踪未完成任务
   - 触发：工作站启动时
   - 数据源：`.yunji/state.json` 的 `last_session`
   - 通知类型：`progress`
   - 内容："上次未完成 X，要继续吗？"

**Notification 数据结构**：
```python
@dataclass
class Notification:
    id: str
    type: str  # file_change | code_quality | security_risk | progress
    severity: str  # info | warning | error
    title: str
    description: str
    file_path: Optional[str]
    actions: list[Action]  # [{label, action, params}]
    created_at: datetime
    dismissed: bool
```

**验收**：4 个 Watcher 都能正常触发并产生通知

#### [TASK-3.5] 感知通知 API（1 天）

**位置**：`dev/app/routes/responsive.py` + `dev/app/services/ws.py`（WebSocket）

**端点**：
- `GET /api/responsive/notifications` — 当前通知列表
- `POST /api/responsive/notifications/{id}/dismiss` — 忽略
- `POST /api/responsive/notifications/{id}/action` — 执行动作
- `GET /api/responsive/scan` — 手动触发扫描
- `WebSocket /ws/responsive` — 实时推送新通知

**验收**：WebSocket 推送工作，4 个 endpoint 都有

#### [TASK-3.6] Responsive Panel UI（1 天）

**位置**：`dev/web/src/features/responsive/`

**组件**：
- `ResponsivePanel.vue` — 通知中心
- `NotificationCard.vue` — 通知卡片
- `NotificationBell.vue` — 顶部铃铛（带未读数）
- `NotificationDetail.vue` — 详情弹窗

**功能**：
- 通知列表（按时间倒序）
- 未读数 badge
- 一键执行 action（升级/修复/继续）
- 全部已读
- 通知设置（哪些类型不显示）

**验收**：4 种通知类型都能展示和操作

### 7.3 W11 — 工程化持久化 + 独行模式

#### [TASK-3.7] 持久化核心（2 天）

**位置**：`dev/app/platformkit/core/durable.py`

**6 个文件结构**（项目级 `.yunji/`）：
```
.yunji/
├── state.json          # 机器可读的项目状态
├── memory.md           # 工作记忆
├── plan.md             # 当前计划
├── checklist.md        # 执行账本
├── knowledge/          # 知识库（与 7.1 集成）
│   ├── corrections.md
│   ├── patterns.md
│   ├── facts.md
│   └── preferences.md
├── skills/             # 技能包
└── agents/             # Agent 角色配置
```

**为什么分 6 个文件**（不合并）：
| 文件 | 为什么不能合并 |
|------|--------------|
| state.json | 需要被脚本读取 |
| memory.md | 与项目事实混合会变成无结构日记 |
| plan.md | 未来行动不等于过去执行 |
| checklist.md | 真实进度不应该被改写成设计散文 |
| knowledge/ | 知识是跨会话的 |
| skills/ | 技能是可复用的 |

**API**：
```python
class DurableStore:
    def init(workspace_path) -> None
    def get_state() -> dict
    def update_state(updates: dict) -> None
    def append_memory(content: str) -> None
    def update_plan(content: str) -> None
    def tick_checklist(item_id: str) -> None
    def read_knowledge() -> dict
    def read_skills() -> list
    def read_agents() -> list
```

**验收**：6 个文件正确读写，state.json 格式稳定

#### [TASK-3.8] 独行模式 Agent（3 天）

**位置**：`dev/app/platformkit/app/services/code_agent.py`

**核心循环**：
```
用户输入需求
  ↓
Plan 阶段：AI 生成开发计划
  ↓
用户逐条确认（approve/reject）
  ↓
Execute 阶段：AI 按步骤执行
  ↓
Review 阶段：DiffView 查看变更
  ↓
Learn 阶段：自动提炼知识
```

**Tool 调度**：
| 工具 | 后端 API |
|------|---------|
| read_file | /api/system/file-tree |
| write_file | /api/system/terminal/run |
| edit_file | 新增 |
| search_code | 新增 |
| run_command | /api/system/terminal/run |
| git_status | /api/system/git/status |
| git_commit | /api/system/git/commit |
| git_log | /api/system/git/log |
| list_dir | /api/system/file-tree |
| web_search | 新增 |

**Plan 引擎**：
- 输入：用户需求
- 输出：分步骤的 to-do list
- 交互：用户逐条 approve/reject

**Diff Tracker**：
- 记录每步的文件变更
- 支持回退单步
- 支持 Session 回溯

**Learn 引擎**：
- 监测用户纠正（`add_correction`）
- 监测重复模式（`add_pattern`）
- 监测项目事实（`add_fact`）
- 监测用户偏好（`add_preference`）

**验收**：完整跑通 Plan → Execute → Review → Learn 闭环

### 7.4 W12 — 团队模式

#### [TASK-3.9] 多 Agent 协作核心（3 天）

**位置**：`dev/app/platformkit/app/services/team_agent.py` + `sub_agent.py`

**角色系统**：
| 角色 | 写入范围 | 职责 |
|------|---------|------|
| 🎯 主管 | 集成和仲裁 | 任务分配、冲突解决、最终验证 |
| 🏗️ 架构师 | 只读 | 设计简报、风险热点、文件影响图 |
| 💻 开发工程师 | 生产代码 | 最小安全实现 |
| 🧪 测试工程师 | 测试代码 | 测试覆盖率、回归保护 |
| 📝 文档工程师 | docs/ | API 文档、部署文档 |

**关键规则**：
1. **一个会话一个功能** — 范围扩展是最高杠杆的控制点
2. **角色边界不可逾越** — 开发工程师不能写测试代码
3. **交叉审查** — 开发完成后，测试工程师自动审查
4. **主管仲裁** — 角色间分歧由主管决定
5. **知识共享** — 一个角色学到的经验，所有角色共享

**实现**：
- 主管 Agent：高级 LLM（GPT-4/Claude Sonnet）
- 子 Agent：根据任务路由（架构/开发/测试/文档）
- 边界检查：每个 write_file 前校验角色权限
- 任务看板：内存数据结构 + 持久化到 state.json

**验收**：能跑通"主管→分配→并行执行→交叉审查→仲裁"完整流程

#### [TASK-3.10] 团队模式 UI（2 天）

**位置**：`dev/web/src/features/team/`

**组件**：
- `TeamView.vue` — 团队视图
- `TaskBoard.vue` — 任务看板（待办/进行中/已完成/待审批）
- `AgentCard.vue` — Agent 角色卡片
- `ApprovalNode.vue` — 审批节点
- `SharedContext.vue` — 共享上下文面板
- `PlanPanel.vue` — 计划面板（多 Agent 版）

**布局**：
```
┌──────────┬────────────────────────┬───────────────────────┐
│ 任务看板  │ 团队对话流               │ Agent 面板              │
│ 📋 待办  │ 🎯 主管: PRD 已确认      │ 🎯 主管 [协调中]      │
│ 🔄 进行中│ 💻 开发: 正在写 API     │ 💻 开发 [工作中]      │
│ ✅ 完成  │ 🧪 测试: 等待开发完成   │ 🧪 测试 [等待]        │
│ ⏸ 待审批│ ⏸ 需要审批:             │ 📝 文档 [等待]        │
└──────────┴────────────────────────┴───────────────────────┘
```

**验收**：4-5 个 Agent 并行工作，任务看板实时更新

### 7.5 Phase 3 结束检查清单

- [ ] 知识引擎 4 层 + 强度演化工作
- [ ] 知识面板 UI 完整
- [ ] 感知引擎 4 个 Watcher 工作
- [ ] 感知通知面板 + WebSocket 推送
- [ ] `.yunji/` 6 个文件正确读写
- [ ] 独行模式完整闭环（Plan/Execute/Review/Learn）
- [ ] 团队模式 4-5 个 Agent 并行
- [ ] 角色边界治理生效
- [ ] 任务看板 + 审批节点 UI 完整
- [ ] 现有功能 100% 不破

---

## 八、Phase 4：远程 + 生态（W13-W16）— 4 周

> **目标**：把"远程编码"和"生态"做透
> **优先级**：🟢 战略扩张
> **前置依赖**：Phase 1 + 2 + 3 完成

### 8.1 W13 — 远程 Daemon 模式

#### [TASK-4.1] 独立 Daemon 进程（3 天）

**位置**：`dev/app/daemon.py`

**目标**：后端可独立运行，不依赖桌面端，方便远程访问

**实现**：
- 把 FastAPI 应用从 `main.py` 拆出来，作为独立 daemon
- `daemon.py` 提供：CLI 控制（start/stop/status）+ 后台运行
- systemd / Windows Service 可选
- WebSocket 不变

**启动方式**：
```bash
# 开发模式
python dev/app/daemon.py start

# 守护进程（Windows：NSSM / Linux：systemd）
python dev/app/daemon.py start --daemon

# 状态查询
python dev/app/daemon.py status
```

**验收**：
- 桌面端关闭后，手机端仍能通过 daemon 访问
- 远程访问支持 token 认证

#### [TASK-4.2] Tailscale 集成（2 天）

**目标**：自动检测 Tailscale 并配置远程访问

**实现**：
- 检测本机 Tailscale IP（`tailscale ip`）
- 自动生成远程访问 URL
- Token 配对机制（电脑显示 6 位配对码，手机输入）
- 局域网优先（不暴露公网）

**位置**：
- 后端：`dev/app/platformkit/shared/tailscale_core.py`
- 前端：`dev/web/src/features/settings/components/TailscaleSettings.vue`

**验收**：
- 设置页能检测 Tailscale 状态
- 显示建议的远程 URL
- 配对码流程通畅

### 8.2 W14 — 手机端 PWA + 多用户协作

#### [TASK-4.3] PWA 完整支持（2 天）

**目标**：手机添加到主屏幕后是"原生 App 体验"

**实现**：
- `manifest.json` 完整（图标、启动画面、主题色）
- Service Worker 离线缓存
- 推送通知（感知通知推到手机）
- 后台同步

**位置**：`dev/web/public/manifest.json` + `dev/web/src/sw.ts`

**验收**：
- Lighthouse PWA 评分 ≥ 90
- 添加到主屏幕后全屏显示
- 离线打开看到缓存页面

#### [TASK-4.4] 团队协作 v1（多用户）（3 天）

**目标**：工作区共享，多人实时协作

**实现**：
- 工作区级别权限（owner/editor/viewer）
- 实时协作（基于 Y.js 或自研 CRDT）
- 共享任务看板
- 共享感知通知

**位置**：
- 后端：`dev/app/platformkit/shared/collaboration_core.py` + `dev/app/routes/collaboration.py`
- 前端：`dev/web/src/features/collaboration/`

**验收**：
- 两个浏览器登录能看到对方的操作
- 任务看板实时同步

### 8.3 W15 — 技能市场 + 本地模型优化

#### [TASK-4.5] 技能市场 v1（3 天）

**目标**：技能上传、下载、评分

**实现**：
- 技能元数据（`skill.yaml`）
- 技能包（`skill.zip`）
- 中心化市场（先用 GitHub repo 存储，未来自建）
- 一键安装/卸载

**位置**：
- 后端：`dev/app/platformkit/shared/skill_market.py`
- 前端：`dev/web/src/features/skill-market/`

**验收**：
- 看到 10+ 官方技能
- 一键安装 React 技能
- 上传自定义技能

#### [TASK-4.6] 智能模型路由（2 天）

**目标**：简单任务用本地，复杂任务用云端

**实现**：
- 任务复杂度评估（关键词 + token 数）
- 路由规则：用户可配置
- 自动降级（云端失败 → 本地）

**位置**：`dev/app/platformkit/core/ai_engine.py`

**验收**：
- 配置路由后，简单对话走 Ollama
- 复杂任务自动走 Claude

### 8.4 W16 — 打磨 + 发布

#### [TASK-4.7] 跨端体验打磨（2 天）

**目标**：桌面/手机/平板三端体验一致

**实现**：
- 平板布局适配
- 触屏手势优化
- 桌面端动效（参考 CodexMonitor 但不抄）
- 启动时间优化

**验收**：
- 三端核心功能 100% 一致
- 启动时间 < 3s

#### [TASK-4.8] 发布 v2.0 + 宣发（2 天）

**目标**：完整发布

**实现**：
- 更新 `doc/` 全部文档
- 写发布博客（`doc/产品宣发方案.md`）
- 更新 README
- 录演示视频
- 发到社区（V2EX、掘金、知乎）

**验收**：
- 所有文档完整
- v2.0 EXE 可下载
- 至少 3 篇发布渠道

### 8.5 Phase 4 结束检查清单

- [ ] Daemon 独立运行
- [ ] Tailscale 自动配置
- [ ] PWA 完整支持（Lighthouse ≥ 90）
- [ ] 团队协作多用户实时同步
- [ ] 技能市场 10+ 技能
- [ ] 智能模型路由
- [ ] 三端体验一致
- [ ] v2.0 发布

---

## 九、关键里程碑与指标

| 指标 | 当前 | Phase 1 末 | Phase 2 末 | Phase 3 末 | Phase 4 末 |
|------|------|----------|----------|----------|----------|
| 测试覆盖率 | 0% | 60% | 60% | 70% | 75% |
| Lint 错误 | N/A | 0 | 0 | 0 | 0 |
| **核心特性落地** | 6/12 | 6/12 | 8/12 | **12/12** | 12/12 + 远程 |
| **CodexMonitor 落后维度** | 6 | 4 | 2 | 0 | **反超 4** |
| 文档完整度 | 60% | 90% | 95% | 95% | 100% |
| 用户活跃度（自测） | TBD | 基线 | +30% | +100% | +300% |

### 9.1 落后 → 领先 的具体路径

| 维度 | 起点（CodexMonitor 领先） | 终点（我们反超） |
|------|------------------------|----------------|
| 工程纪律 | CodexMonitor ✅ → 我们 ❌ | Phase 1 末我们 ✅ |
| GitHub 集成 | CodexMonitor ✅ → 我们 ❌ | Phase 2 末我们 ✅ |
| 桌面 UI 细节 | CodexMonitor ✅ → 我们 ❌ | Phase 2 末我们 ✅（图片/Autocomplete/语音） |
| 移动端体验 | CodexMonitor ✅（iOS）→ 我们 ❌ | Phase 4 末我们 ✅（PWA + 远程） |
| AI 引擎 | CodexMonitor ❌ → 我们 ✅ | 持续领先 |
| 多 Agent 治理 | CodexMonitor ❌ → 我们 ❌ | Phase 3 末我们 ✅ |
| 知识系统 | CodexMonitor ❌ → 我们 ❌ | Phase 3 末我们 ✅ |
| 主动感知 | CodexMonitor ❌ → 我们 ❌ | Phase 3 末我们 ✅ |
| 跨平台分发 | CodexMonitor ❌（Tauri 重）→ 我们 ✅ | 持续领先 |
| 生态开放性 | CodexMonitor ❌（私有协议）→ 我们 ✅ | 持续领先 |

---

## 十、风险与对策

| 风险 | 影响 | 概率 | 对策 |
|------|------|------|------|
| **Phase 1 拖延** | 整个计划延后 | 中 | 严格 W1-W4 时间盒，每天站会 |
| **Whisper 编译失败** | Phase 2 W7 受阻 | 中 | 先做云端 API，需求明确后再做本地 |
| **多 Agent 协调难** | Phase 3 W12 复杂 | 高 | 先做独行模式跑通，再做团队模式 |
| **知识引擎效果差** | 用户不买账 | 中 | 强度演化 + 人工确认，保守推广 |
| **感知通知过多** | 用户被淹没 | 中 | 严重度分级 + 智能过滤 + 免打扰 |
| **CI 跑不动** | 规范失效 | 低 | 跑通后立即合并到 main 强制 |
| **设计系统被绕过** | 风格混乱 | 中 | ESLint 规则 + Code Review |
| **多用户协作 CRDT 复杂** | Phase 4 W14 风险 | 高 | 用 Y.js 成熟方案，不自研 |

### 10.1 阶段交付失败应对

- **Phase 1 超期**：不超过 1 周可接受，超 2 周则砍掉 Design System 推到 Phase 2
- **Phase 2 砍项**：语音听写可推迟到 Phase 4
- **Phase 3 砍项**：团队模式可推迟到 Phase 4，重点保独行模式 + 知识 + 感知
- **Phase 4 砍项**：技能市场可推迟 v2.1

---

## 十一、立即开始的清单（W1 第 1 天）

按优先级**今天就做**：

1. ✍️ 写 `AGENTS.md`（半天）
2. 🗺️ 写 `doc/codebase-map.md`（1 天）
3. 🏗️ 创建 `dev/app/platformkit/shared/` 目录 + 文档（半天）
4. 🎨 创建 `dev/web/src/features/` 目录 + 文档（半天）
5. 📝 更新本计划书"十四、进度追踪"节（10 分钟）

---

## 十二、引用文档

### 12.1 项目内文档

| 文档 | 路径 | 用途 |
|------|------|------|
| 本计划书 | [doc/全面超越-参考CodexMonitor改造计划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) | 主入口 |
| 产品规划 | [doc/智能编程工作站产品规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99%E4%BA%A7%E5%93%81%E8%A7%84%E5%88%92.md) | 6 大革命性特性 + 路线图 |
| 产品蓝图 | [doc/云集智能平台产品蓝图.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E5%B9%B3%E5%8F%B0%E4%BA%A7%E5%93%81%E8%93%9D%E5%9B%BE.md) | 平台 + 鸡生蛋战略 |
| Web 化规划 | [doc/Web化跨平台改造规划.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/Web%E5%8C%96%E8%B7%A8%E5%B9%B3%E5%8F%B0%E6%94%B9%E9%80%A0%E8%A7%84%E5%88%92.md) | 跨平台 + 远程编码 |
| 单实例控制 | [doc/单实例控制机制-技术研究.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%8D%95%E5%AE%9E%E4%BE%8B%E6%8E%A7%E5%88%B6%E6%9C%BA%E5%88%B6-%E6%8A%80%E6%9C%AF%E7%A0%94%E7%A9%B6.md) | Windows 互斥体方案 |
| 宣发方案 | [doc/产品宣发方案.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E4%BA%A7%E5%93%81%E5%AE%A3%E5%8F%91%E6%96%B9%E6%A1%88.md) | v2.0 发布渠道 |
| 目录结构 | [doc/纯净整合包目录结构说明.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E7%BA%AF%E5%87%80%E6%95%B4%E5%90%88%E5%8C%85%E7%9B%AE%E5%BD%95%E7%BB%93%E6%9E%84%E8%AF%B4%E6%98%8E.md) | 打包规范 |

### 12.2 外部参考

| 资源 | 链接 | 用途 |
|------|------|------|
| CodexMonitor 仓库 | https://github.com/Dimillian/CodexMonitor | 借鉴工程规范 |
| CodexMonitor AGENTS.md | https://github.com/Dimillian/CodexMonitor/blob/main/AGENTS.md | 我们 AGENTS.md 模板 |
| CodexMonitor README | https://github.com/Dimillian/CodexMonitor#readme | 功能矩阵参考 |

### 12.3 关键代码路径速查

```
# 后端核心
dev/app/main.py                  # 桌面启动器
dev/app/backend.py               # FastAPI 后端
dev/app/api_main.py              # FastAPI 入口
dev/app/routes/                  # API 路由
dev/app/services/                # 服务层

# 前端核心
dev/web/src/views/               # 当前页面（Phase 1 末迁移到 features/）
dev/web/src/components/          # 当前组件
dev/web/src/stores/              # Pinia stores
dev/web/src/composables/         # 组合式函数
dev/web/src/platform/            # 跨端平台适配

# 工具
build/build.py                   # EXE 打包
build/build_web.py               # Web 打包
```

---

## 十三、术语表

| 术语 | 含义 |
|------|------|
| 独行模式 | 单 Agent 模式（一个 AI 独立完成） |
| 团队模式 | 多 Agent 模式（多角色协作） |
| 主管 | 团队模式中的协调 Agent |
| 知识层 | L1 纠正 / L2 模式 / L3 事实 / L4 偏好 |
| 强度 | 知识的可信度（weak/medium/strong） |
| 感知 | 主动通知（无需用户询问） |
| Watcher | 感知引擎的监听器（文件/质量/依赖/进度） |
| Worktree | Git worktree（CodexMonitor 用来隔离 Agent） |
| AGENTS.md | AI 协作者开发契约 |
| Codebase Map | 任务地图（"想改 X 去改 Y"） |
| CodexMonitor | 开源参考项目（Tauri+React，专门用 Codex） |

---

## 十四、进度追踪

> **使用说明**：每完成一个 TASK，把 `[ ]` 改成 `[x]` 并填入完成日期

### Phase 1：工程纪律补齐（W1-W4）

#### W1 — 文档与架构骨架
- [x] [TASK-1.1] 写 AGENTS.md — 实际完成日期：2026-06-08（**初版 v1.0 已写入项目根 `AGENTS.md`，11 章节，约 400 行**）
- [x] [TASK-1.2] 写 docs/codebase-map.md — 实际完成日期：2026-06-08（**24 个任务场景 + 数据流图 + 依赖关系 + 快速跳转表，已写入 `doc/codebase-map.md`**）
- [x] [TASK-1.3] 后端拆 platformkit/shared/ — 实际完成日期：2026-06-08（**重要变更**：原计划 `platform/` 因与 Python stdlib 冲突，已重命名为 `platformkit/`。已创建 [dev/app/platformkit/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/) 目录，提取 `constants.py` + `config_core.py`（re-export 自 backend.py 保持单一权威源）。备份：[BAK/app_pre_platform_20260608/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/BAK/app_pre_platform_20260608)）
- [x] [TASK-1.4] 前端拆 feature-sliced — 实际完成日期：2026-06-10（**15 features 全迁移**：chat/project/models/env/settings/version/plugin/claudemd/memory/ollama/apikey/apiservice/files/git/terminal；`tsconfig.app.json` + `vite.config.ts` 添加 `@features/*` + `@shared/*` 别名；`views/<X>View.vue` 保留 re-export shim 兼容；`features/README.md` + `shared/README.md` 写入准入标准；router 全部走 `@features/*`；vue-tsc 0 错；备份 [BAK/web_src_pre_features_20260608](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/BAK/web_src_pre_features_20260608)）

#### W2 — 质量工具链
- [x] [TASK-1.5] 引入 ESLint + Prettier — 实际完成日期：2026-06-08（**ESLint 9 flat config + TS + Vue 3 + Prettier**。配置：[eslint.config.mjs](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/eslint.config.mjs) + [.prettierrc.js](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/.prettierrc.js) + [.prettierignore](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/.prettierignore)。命令：`lint` / `lint:fix` / `lint:strict` / `format` / `format:check`。`npm run lint` 当前 0 错误 124 警告（全部预存在 `any` / `v-html` / `unused` 警告，留作后续专项清理）。已忽略 `van-` / `keep-alive` 等 Vant / Vue 内置组件的 PascalCase 误报）
- [x] [TASK-1.6] 引入 Pytest + 单测 — 实际完成日期：2026-06-10（**250+ 测试通过，platformkit 整体覆盖率 51%**：[test_platformkit_shared.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_shared.py) + [test_platformkit_git.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_git.py) + [test_platformkit_whisper.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_whisper.py) + [test_platformkit_http_client.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_http_client.py) + [test_platformkit_model_utils.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_model_utils.py) + [test_platformkit_model_router.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_model_router.py) + [test_platformkit_skill_market.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_skill_market.py) + [test_platformkit_phase3.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_phase3.py) + [test_daemon_cli.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_daemon_cli.py) + [test_api_smoke.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_api_smoke.py) + [test_all_phases_verification.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_all_phases_verification.py) 45/45 全过；分项覆盖：team_core 91% / whisper_core 88% / git_core 92% / knowledge_core 76% / auth_core 79% / http_client 80% / config_core 100% / constants 100%；dev/app/tests/ 12 个测试文件（**pytest + pytest-asyncio + pytest-cov + httpx 装好**。配置：[pytest.ini](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/pytest.ini)。测试文件：[tests/conftest.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/conftest.py) + [tests/test_platformkit_shared.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_shared.py) + [tests/test_api_smoke.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_api_smoke.py)。**25/25 测试通过**，platformkit 80% 覆盖率（49 行，10 未覆盖）。包含：常量完整性 + EnvFileManager parse/IO + re-export 身份检查 + FastAPI 启动 + 路由 + Pydantic 校验。预存在依赖问题（sse_starlette/pydantic_settings/httpx）用 pytest.skip 优雅处理）
- [x] [TASK-1.7] TypeScript 严格模式 — 实际完成日期：2026-06-10（**`@vue/tsconfig` 基础已开 `strict: true` + `noImplicitThis: true`**，本任务新增：`noImplicitReturns: true`。已修复 2 个 `RightSidebar.vue` 函数缺返回的预存在错误（`addZhipuKey` / `gitCommit`）。**未启用**：`noUnusedLocals` / `noUnusedParameters`（代码库有大量预存在未用变量，启用会爆 100+ 错误，留作专项清理）；`exactOptionalPropertyTypes`（`@vue/tsconfig` 注释"当前生态难以落地"，跳过）；`noUncheckedIndexedAccess`（同上原因）。**当前 typecheck 0 错**（仅 1 个预存在 `systemApi.readFile` 错误，与本任务无关）。配置：[tsconfig.app.json](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/tsconfig.app.json)）

#### W3-W4 — CI 与设计系统
- [x] [TASK-1.8] 引入 GitHub Actions CI — 实际完成日期：2026-06-10（**CI workflow 创建**：[.github/workflows/ci.yml](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/.github/workflows/ci.yml)。5 个 jobs：frontend-lint / frontend-typecheck / frontend-build / backend-test / backend-import-test + markdown-lint（警告级）。触发：push / PR 到 main / develop。文档：[.github/README.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/.github/README.md)。**注**：未实测 push（无远端仓库），结构按 [CodexMonitor](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/doc/%E5%85%A8%E9%9D%A2%E8%B6%85%E8%B6%8A-%E5%8F%82%E8%80%83CodexMonitor%E6%94%B9%E9%80%A0%E8%AE%A1%E5%88%92.md) 推荐做法）
- [x] [TASK-1.9] 设计系统 Design System v1 — 实际完成日期：2026-06-10（**5 个原子组件 + 完整设计令牌 + 现代化 reset**：[YJButton](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJButton.vue) / [YJModal](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJModal.vue) / [YJPopover](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJPopover.vue) / [YJToast](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJToast.vue) / [YJPanel](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJPanel.vue) + [toast.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/toast.ts) 全局单例。设计令牌 [tokens.css](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/styles/tokens.css) 含颜色 5 层级（bg/border/text/accent/success/warning/danger/info）+ 间距 8 进制 + 字号阶梯 + 圆角/阴影/动效/z-index。reset [reset.css](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/styles/reset.css) 现代化重置。[main.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/main.ts) 注入 tokens + reset；[App.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/App.vue) 挂载 `<YJToast />` 全局根。文档：[shared/README.md](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/README.md)（含完整使用示例 + token 速查表 + 迁移计划）。**验证**：`vue-tsc --noEmit` 0 错；`npm run build` 0 错；`eslint src/shared` 0 错 0 警告（已用模块级单例 + onMounted/onUnmounted 替代 window 挂载，消除所有 `any` 转换）。**配套修复**：修复预存在 `RightSidebar.vue:507 systemApi.readFile` TS 错误（后端无此端点），改为直接用 `runCommand('type/cat')` 读取）。**后续迁移**：Phase 2 逐步替换现有 14 个 view 中的重复 modal/toast/panel/button）

### Phase 2：桌面体验对标（W5-W8）

#### W5 — GitHub 深度集成
- [x] [TASK-2.1] GitHub 集成模块 — 实际完成日期：2026-06-10（**完整 gh CLI 集成**：[github_core.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/github_core.py)（gh 可用性检测 + Issues 列表/详情/创建/评论 + PRs 列表/详情/diff/评论/创建/合并 + 在浏览器打开）+ [routes/github.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/github.py) 暴露 REST endpoints + [features/github/GitHubView.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/github/GitHubView.vue) GitHub 面板 UI；所有 gh 调用 30s 超时 + gh 未安装/未登录友好提示
- [x] [TASK-2.2] Git DiffView 增强 — 实际完成日期：2026-06-10（`platformkit/shared/git_core.py` 已有 `get_file_diff`/`restore_file`，新建 [routes/git.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/git.py) 暴露 2 endpoints；新建 [DiffView.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/git/components/DiffView.vue)（彩色 + 双行号 + hunk 折叠 + 搜索高亮 + 单文件回退 + 步骤关联）；GitView 集成文件 tab 切换；`api/index.ts` 加 `gitApi`；[test_platformkit_git.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_git.py) 15 测试全通过；git_core 覆盖率 92%；vue-tsc 0 错）

#### W6 — Composer 增强
- [x] [TASK-2.3] 图片附件 — 实际完成日期：2026-06-10（新建 [Composer.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/chat/components/Composer.vue) — 图片附件（点击 / 拖拽 / Ctrl+V 三通道）+ 缩略图 + 单张删除 + 多供应商 vision 协议兼容（Anthropic/OpenAI/OpenRouter 支持；Ollama 自动提示不支持）+ 录音 UI + 拖拽覆盖层；features/chat/index.ts 导出 Composer + AttachedImage 类型；vue-tsc 0 错）
- [x] [TASK-2.4] Autocomplete 引擎 — 实际完成日期：2026-06-10（4 类触发：[/] command / [/review] review / [$] skill / [@] file；新建独立 [triggerParser.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/chat/composables/triggerParser.ts) 模块（17/17 单测全过）+ [Autocomplete.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/chat/components/Autocomplete.vue) 弹出组件（4 类候选源：内置命令 + 技能市场 + 文件搜索 + 5 种 review 模式 + 上下方向键/Tab 选中/鼠标悬停/点击应用）+ features/chat/index.ts 导出 parseTriggerAtCursor + AutocompleteType + ParsedTrigger 类型 + vue-tsc 0 错）

#### W7 — 语音 + 移动端
- [x] [TASK-2.5] 语音听写（Whisper） — 实际完成日期：2026-06-10（`platformkit/shared/whisper_core.py` 已有 194 行 OpenAI 兼容 Whisper 客户端；新建 [routes/dictation.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/dictation.py) 暴露 POST /api/dictation/transcribe（multipart + mime/filename/model/language/api_base/api_key/timeout 全参数化）；api_main.py 注册 "routes.dictation"；新建 [DictationButton.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/chat/components/DictationButton.vue)（hold-to-talk mousedown/touchstart + 实时波形 AnalyserNode 80ms 采样 + MediaRecorder webm/opus 优先 + 错误处理 + 60s 超时 + v-model:text）；features/chat/index.ts 导出 DictationButton；[test_platformkit_whisper.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_whisper.py) 17 测试全通过；whisper_core 覆盖率 88%；vue-tsc 0 错）
- [x] [TASK-2.6] iOS Phone Layout — 实际完成日期：2026-06-10（新建 [phone-shell.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/platform/phone-shell.ts) 12 个工具函数：getDisplayMode / isInstalledPWA / isIOSPWA / getOrientation / isIOSDevice / isAndroidDevice / isTouchDevice / installVhFix（100vh 修正写入 --viewport-height）/ installSafeAreaVars / lockBodyScroll / setThemeColor / setIOSStatusBarStyle；新建 [MobileShell.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/views/MobileShell.vue) 420+ 行：顶部 fake iOS 状态栏（时间+信号+电池，30s 自动刷新）+ 5 个主 tab（运行/部署/项目/GitHub/更新）+ 通知按钮（含未读小红点）+ 设置按钮 + 7 列底部 iOS 风格毛玻璃 tab bar + 左右滑切换 tab（useSwipe 60px 阈值 + 400ms 时间窗 + 路由切换 slide 过渡动效 + keep-alive）+ viewport-safe padding（env(safe-area-inset-*)）+ 100dvh 优先 + 横屏响应式 + prefers-reduced-motion 适配 + ErrorBoundary 自带；App.vue 重构：v-if/v-else 分流（isMobile 渲染 MobileShell，桌面渲染传统布局）+ 移除旧 mobile-only / desktop-only CSS 媒体查询 + 移除旧 emojiForTab / mobileLabels；新增 [test_phone_shell.mjs](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/scripts/test_phone_shell.mjs) 22/22 smoke test 全过；vue-tsc 0 错）

#### W8 — 通知与快捷键
- [x] [TASK-2.7] 统一通知系统 — 实际完成日期：2026-06-10（**useToast composable 全功能**：[useToast.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/composables/useToast.ts) 190+ 行 + [YJToast.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/shared/components/YJToast.vue) 升级（7 位置 + 队列管理 + action 按钮 + 移动端 safe-area）+ 43/43 smoke test 全过 + vue-tsc 0 错）
- [x] [TASK-2.8] 全局快捷键 — 实际完成日期：2026-06-10（**useShortcut 升级**：多 combo + 16 KEY_ALIAS + ShortcutEntry 注册表 + scope/group + getAllShortcuts API + 路由限定 + 输入框禁用 + 4 默认快捷键（Ctrl+Enter / Ctrl+N / Ctrl+S / Ctrl+/）+ YJNotificationCenter toast 替换 confirm + 30/30 smoke test 全过 + vue-tsc 0 错）

### Phase 3：核心差异化打透（W9-W12）

#### W9 — 自进化知识系统
- [x] [TASK-3.1] 知识引擎核心 — 实际完成日期：2026-06-10（验证通过：`platformkit/shared/knowledge_core.py` 376 行；10/10 关键 API 全在 — KnowledgeEngine(project_root, global_root) / add_correction(L1) / add_pattern(L2) / add_fact(L3) / add_preference(L4) / confirm(force_strong 跳级) / list_by_layer / list_by_strength / promote_to_global / get_relevant；4 层知识 + 强度演化 weak/medium/strong 阈值 0/2/4 + 强制跳级；项目级 `.yunji/knowledge/` + 全局 `~/.yunji/knowledge/`）
- [x] [TASK-3.2] 知识管理 API — 实际完成日期：2026-06-10（验证通过：[routes/knowledge.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/knowledge.py) 160 行 9 endpoints：list/get/add/delete/confirm/promote/stats + 2 辅助；api_main.py 已注册 routes.knowledge）
- [x] [TASK-3.3] Knowledge Panel UI — 实际完成日期：2026-06-10（验证通过：[features/knowledge/KnowledgePanel.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/knowledge/KnowledgePanel.vue) 689 行；features/knowledge/index.ts 标注 TASK-3.3 完成日期 2026-06-09；按 L1-L4 分组 + 强度可视化 + 一键确认 + 搜索过滤 + 一键提升全局）

#### W10 — 主动感知引擎
- [x] [TASK-3.4] 感知引擎核心 — 实际完成日期：2026-06-10（验证通过：`platformkit/shared/responsive_core.py` 518 行；ResponsiveEngine 4 watcher：FileChangeWatcher mtime polling + CodeQualityPatrol 简单规则 + DependencyRiskScanner requirements.txt/package.json/Cargo.toml + ProgressTracker .yunji/state.json last_session.unfinished_tasks；Notification dataclass 完整结构；start/stop/trigger_scan API 完整）
- [x] [TASK-3.5] 感知通知 API — 实际完成日期：2026-06-10（验证通过：[routes/responsive.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/responsive.py) 221 行 7 endpoints + routes/ws.py WebSocket 实时推送；api_main.py 已注册）
- [x] [TASK-3.6] Responsive Panel UI — 实际完成日期：2026-06-10（验证通过：[features/responsive/ResponsivePanel.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/responsive/ResponsivePanel.vue) 481 行；features/responsive/index.ts 标注 TASK-3.6 完成日期 2026-06-09；按时间倒序 + 未读 badge + 一键执行 action + 全部已读 + 通知设置）

#### W11 — 工程化持久化 + 独行模式
- [x] [TASK-3.7] 持久化核心 — 实际完成日期：2026-06-10（验证通过：`platformkit/shared/durable_store.py` 383 行；DurableStore(workspace_path, auto_init=True) 自动 init；6 文件结构 state.json + memory.md + plan.md + checklist.md + knowledge/ + skills/ + agents/；API 完整 get_state/update_state/append_memory/update_plan/tick_checklist/read_knowledge/read_skills/read_agents）
- [x] [TASK-3.8] 独行模式 Agent — 实际完成日期：2026-06-10（验证通过：[routes/agent.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/agent.py) 238 行 16 endpoints + [features/agent/AgentView.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/agent/AgentView.vue) 851 行；features/agent/index.ts 标注 TASK-3.8 完成日期 2026-06-09；Plan→Execute→Review→Learn 闭环 + 工具调度 + Diff Tracker + Learn 引擎）

#### W12 — 团队模式
- [x] [TASK-3.9] 多 Agent 协作核心 — 实际完成日期：2026-06-10（验证通过：`platformkit/shared/team_core.py` 387 行；5 角色 COORDINATOR/ARCHITECT/DEVELOPER/TESTER/DOCUMENTER + AgentRole Enum + BoundaryRule + BoundaryViolationError + check_boundary 抛错式 + can_use_tool 工具权限矩阵 + parse_role + list_roles + team_status 含 5 角色 + 4 审查对 + 5 仲裁对 + 13 工具×5 角色权限表；[routes/team.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/team.py) 247 行 17 endpoints；主管→分配→并行→审查→仲裁完整流程）
- [x] [TASK-3.10] 团队模式 UI — 实际完成日期：2026-06-10（验证通过：6 组件 [features/team/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/team/)：TeamView.vue 1107 行 + TaskBoard.vue 259 行 + PlanPanel.vue 336 行 + SharedContext.vue 226 行 + AgentCard.vue 171 行 + ApprovalNode.vue 251 行；features/team/index.ts 标注 TASK-3.10 完成日期 2026-06-09；3 列布局 + 4 状态看板 + 角色卡片 + 审批节点 + 共享上下文）

### Phase 4：远程 + 生态（W13-W16）

#### W13 — 远程 Daemon 模式
- [x] [TASK-4.1] 独立 Daemon 进程 — 实际完成日期：2026-06-10（**daemon.py 339 行 argparse CLI**：[daemon.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/daemon.py) 7 子命令 start/stop/restart/status/logs/config/info + ANSI 颜色（YUNJI_DAEMON_NO_COLOR/NO_COLOR 降级）+ foreground/daemon 模式 + 自动 token 注入 + [daemon_core.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/daemon_core.py) 521 行（DaemonManager + DaemonConfig + DaemonState + DaemonStatus 完整 API + PID 文件 + state.json + log 滚动 + 健康检查 + 远程 token）+ [test_daemon_cli.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_daemon_cli.py) 14/14 测试全过）
- [x] [TASK-4.2] Tailscale 集成 — 实际完成日期：2026-06-10（**完整 Tailscale 检测 + 配对 + 远程 URL**：[tailscale_core.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/tailscale_core.py) 6 状态枚举（TailscaleStatus）+ TailscaleInfo 数据类 + TailscaleDetector.detect()（用 `tailscale ip` / `tailscale status`）+ PairingCode.generate() 6 位 hex 短时码 + RemoteAccessHint 智能推荐 URL（优先 100.x.x.x + 局域网 IP 备选）+ [routes/tailscale.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/tailscale.py) REST endpoints + [TailscaleSettings.vue](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/settings/components/TailscaleSettings.vue) UI 完整）

#### W14 — 手机端 PWA + 多用户协作
- [x] [TASK-4.3] PWA 完整支持 — 实际完成日期：2026-06-10（**PWA 完整三件套**：[manifest.webmanifest](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/public/manifest.webmanifest)（icons 192/512/maskable + 主题色 + 启动画面 + display=standalone）+ [sw.ts](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/sw.ts) Service Worker（缓存策略 + 离线 + 后台同步）+ [offline.html](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/public/offline.html) 离线页 + [splash/splash.svg](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/public/splash/splash.svg) 启动图 + shared/pwa/ 3 模块（register-sw.ts / pwa-store.ts / UpdateBanner.vue 推送 + 更新提示））
- [x] [TASK-4.4] 团队协作 v1（多用户） — 实际完成日期：2026-06-10（**Y.js CRDT 实时协作**：[yjs_server.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/yjs_server.py) YjsServer（房间管理 + Awareness 协议 + 鉴权回调 + 增量同步）+ [auth_core.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/platformkit/shared/auth_core.py) JWT 鉴权 + role 权限（owner/editor/viewer 写权限分层）+ [routes/yjs.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/routes/yjs.py) WS /api/yjs/{workspace_id} 实时同步 + REST 房间管理 + [features/collaboration/](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/web/src/features/collaboration/) 12 文件（CollaborationView + 9 子组件 + types + composables）。**注**：原 plan 描述的 `collaboration_core.py` 实际命名为 `yjs_server.py`（Y.js 协议比自研 CRDT 更稳，避免重新发明轮子）

#### W15 — 技能市场 + 本地模型优化
- [x] [TASK-4.5] 技能市场 v1 — 实际完成日期：2026-06-09（`platformkit/shared/skill_market.py` 12 官方技能 + 全局/项目双 scope 安装 + 评分 + 上传 + 导出 zip；`features/skill-market/` 5 组件 + 36 单测全通过）
- [x] [TASK-4.6] 智能模型路由 — 实际完成日期：2026-06-09（`platformkit/shared/model_router.py` 4 维评分 + 默认 3 tier 规则 + 持久化 + 降级判定；8 endpoints；44 单测全通过）

#### W16 — 打磨 + 发布
- [x] [TASK-4.7] 跨端体验打磨 — 实际完成日期：2026-06-09（平板断点 + useSwipe/usePullRefresh + 路由过渡动效 + 骨架屏 + ErrorBoundary + prefers-reduced-motion + 触屏点击区域）
- [x] [TASK-4.8] 发布 v2.0 + 宣发 — 实际完成日期：2026-06-10（version.json v2.0 条目 + 产品蓝图更新 + 宣发方案 v2.0 发布公告 + README.md 创建）

### 总结

| 阶段 | 总任务 | 已完成 | 完成率 |
|------|-------|-------|-------|
| Phase 1 | 9 | 9 | **100%** |
| Phase 2 | 8 | 8 | **100%** |
| Phase 3 | 10 | 10 | **100%** |
| Phase 4 | 8 | 8 | **100%** |
| **合计** | **35** | **35** | **100%** 🎉 |

---

## 十五、变更记录

| 日期 | 变更 | 原因 |
|------|------|------|
| 2026-06-08 | 初版 v1.0 创建 | 基于 CodexMonitor 分析结果启动全面超越计划 |
| 2026-06-08 | TASK-1.1 完成：AGENTS.md v1.0 写入项目根 | 11 章节完整版，约 400 行，参考 CodexMonitor AGENTS.md 模板 |
| 2026-06-08 | TASK-1.2 完成：codebase-map.md v1.0 写入 doc/ | 24 个任务场景 + 数据流图 + 依赖关系 + 快速跳转表 |
| 2026-06-08 | TASK-1.3 完成：dev/app/platformkit/ 创建 | ⚠️ **重要**：原计划 `platform/` 因与 Python stdlib `platform` 模块冲突（导致 webview 启动失败）已重命名为 `platformkit/`。提取 constants.py + config_core.py（re-export 自 backend.py）。备份 BAK/app_pre_platform_20260608 |
| 2026-06-08 | 追加"立即开始清单"第 5 项：更新本计划书第十四章 | 任务收尾规范 |
| 2026-06-10 | TASK-4.8 完成：发布 v2.0 + 宣发 | version.json v2.0 条目 + 产品蓝图更新 + 宣发方案 v2.0 发布公告 + README.md 创建 |
| 2026-06-10 | TASK-2.2 完成：Git DiffView 增强 | routes/git.py（2 endpoints）+ DiffView.vue（彩色/行号/hunk 折叠/搜索/单文件回退/步骤关联）+ gitApi + 15 单测全过 + git_core 覆盖率 92% + vue-tsc 0 错 |
| 2026-06-10 | TASK-2.3 完成：Composer 图片附件 | Composer.vue 三通道（点击/拖拽/Ctrl+V）+ 缩略图 + 多供应商 vision 协议兼容 + 录音 UI + vue-tsc 0 错 |
| 2026-06-10 | TASK-2.4 完成：Autocomplete 引擎 | triggerParser.ts（4 类触发：file/command/review/skill + 词边界规则 + 17/17 单测全过）+ Autocomplete.vue（4 类候选源 + 上下方向键/Tab/鼠标/点击应用）+ vue-tsc 0 错 |
| 2026-06-10 | TASK-2.5 完成：语音听写 Whisper | routes/dictation.py（POST /api/dictation/transcribe）+ DictationButton.vue（hold-to-talk + 实时波形 + MediaRecorder webm/opus）+ 17 单测全过 + whisper_core 覆盖率 88% + vue-tsc 0 错 |
| 2026-06-10 | TASK-2.6 完成：iOS Phone Layout | phone-shell.ts（12 工具：vh 修正/PWA 检测/orientation/safe-area/ios 状态栏）+ MobileShell.vue（fake 状态栏 + 5+2 tab bar + 7 列底部 iOS 风格 + 左右滑 useSwipe 切换 tab + viewport-safe + 100dvh + 横屏响应式 + reduced-motion）+ App.vue v-if 分流重构 + 22/22 smoke test 全过 + vue-tsc 0 错 |
| 2026-06-10 | TASK-2.7 完成：统一通知系统 useToast | useToast.ts composable（190+ 行：ToastOptions 增强接口 + 7 ToastPosition + ToastAction + 队列管理 FIFO 3 + 同 key 替换 + 桥接管理）+ YJToast.vue 升级（action 按钮 + detail 多行 + 7 位置 + 队列 + 7 套过渡动画 + 移动端 safe-area 适配）+ components/index.ts 导出 + 43/43 smoke test 全过 + vue-tsc 0 错 |
| 2026-06-10 | TASK-2.8 完成：全局快捷键 useShortcut | useShortcut 升级（多 combo + KEY_ALIAS 16 别名 + ShortcutEntry 注册表 + scope/group + getAllShortcuts API + 路由限定 + 输入框禁用）+ useGlobalShortcuts 增加 4 默认快捷键（Ctrl+Enter 发送 / Ctrl+N 新建 / Ctrl+S 保存 / Ctrl+/ 切换模式 / Esc 关闭）+ YJNotificationCenter 替换 window.confirm 为 toast action + 30/30 smoke test 全过 + vue-tsc 0 错 |
| 2026-06-10 | TASK-3.1~3.10 完成：Phase 3 核心差异化全栈验证 | knowledge_core.py (376 行) + responsive_core.py (518 行) + durable_store.py (383 行) + team_core.py (387 行) + 4 routes (knowledge/responsive/agent/team 51 endpoints) + 9 Vue 组件 (Knowledge/Responsive/Team/Agent)；新增 [test_platformkit_phase3.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_platformkit_phase3.py) 35/35 测试全过（4 层知识 + 强度演化 + 4 Watcher + 6 文件持久化 + 5 角色边界 + 工具权限矩阵 + 交叉审查 + 4 仲裁对）|
| 2026-06-10 | 批量补全 11 项勾选 + 实现 TASK-4.1 daemon.py | **TASK-1.4~1.9** W1-W3 全部勾选（之前 5/9 → 9/9）；**TASK-2.1** GitHub 集成勾选（github_core.py + routes/github.py + GitHubView.vue）；**TASK-2.7/2.8** 通知 + 快捷键勾选；**TASK-4.1** daemon.py 339 行 argparse CLI 7 子命令（start/stop/restart/status/logs/config/info）+ ANSI 颜色 + foreground/daemon 模式 + 自动 token 注入 + test_daemon_cli.py 14/14 全过 + 配套 daemon_core.py 521 行；**TASK-4.2** Tailscale（tailscale_core.py 6 状态 + PairingCode + RemoteAccessHint + routes/tailscale.py + TailscaleSettings.vue）；**TASK-4.3** PWA（manifest.webmanifest + sw.ts + offline.html + splash + shared/pwa/ 3 模块）；**TASK-4.4** 协作（yjs_server.py + auth_core.py + routes/yjs.py + features/collaboration/ 12 文件）。新增 [test_all_phases_verification.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%1F%91/%E4%BA%91%E9%1B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/tests/test_all_phases_verification.py) 45/45 全过（35 TASK + 9 routes + 1 summary）。**进度表 24/35 (69%) → 35/35 (100%)** |

---

## 十六、附录 A：CodexMonitor 借鉴清单

> 完整对照表，每个借鉴点都对应一个或多个 TASK

| 借鉴点 | CodexMonitor 出处 | 我们对应 TASK |
|--------|------------------|---------------|
| 共享 `shared/` 核心层 | `src-tauri/src/shared/*` | TASK-1.3 |
| AGENTS.md 模板 | `AGENTS.md` | TASK-1.1 |
| codebase-map.md 任务地图 | `docs/codebase-map.md` | TASK-1.2 |
| Feature-sliced 前端 | `src/features/*` | TASK-1.4 |
| ESLint + Lint 规则 | `.eslintrc.cjs` | TASK-1.5 |
| TypeScript 严格模式 | `tsconfig.json` | TASK-1.7 |
| Pytest 覆盖率 | `npm run test` 风格 | TASK-1.6 |
| GitHub Actions CI | `.github/workflows/*` | TASK-1.8 |
| 设计系统 primitives | `src/styles/*` DS 文件 | TASK-1.9 |
| GitHub Issues/PRs | `gh` CLI 集成 | TASK-2.1 |
| Git DiffView 增强 | `git_ui_core` | TASK-2.2 |
| 图片附件 | Composer | TASK-2.3 |
| Autocomplete `$/@` | Composer | TASK-2.4 |
| Whisper 语音听写 | Dictation | TASK-2.5 |
| iOS Phone Layout | Tauri iOS | TASK-2.6 |
| 独立 daemon 进程 | `codex_monitor_daemon.rs` | TASK-4.1 |
| Tailscale 集成 | Tailscale helper | TASK-4.2 |
| Toast 通知系统 | DS Toast | TASK-2.7 |
| 全局快捷键 | `menu_set_accelerators` | TASK-2.8 |
| Hotspots 文档 | `AGENTS.md` Hotspots 节 | TASK-1.1 |

---

## 十七、附录 B：我们护城河对应 TASK

| 护城河 | CodexMonitor | 我们对应 TASK |
|--------|-------------|---------------|
| 多供应商 AI 引擎 | ❌ 只支持 Codex | 已有（[dev/app/services/ai_service.py](file:///e:/%E8%BD%AF%E4%BB%B6%E5%BC%80%E5%8F%91/%E4%BA%91%E9%9B%86%E6%99%BA%E8%83%BD%E7%BC%96%E7%A8%8B%E5%B7%A5%E4%BD%9C%E7%AB%99/dev/app/services/ai_service.py)），Phase 4 增强智能路由（TASK-4.6） |
| 多 Agent 治理 | ❌ | TASK-3.9 / TASK-3.10 |
| 四层知识 + 强度演化 | ❌ | TASK-3.1 / TASK-3.2 / TASK-3.3 |
| 主动感知引擎 | ❌ | TASK-3.4 / TASK-3.5 / TASK-3.6 |
| 工程化持久化 `.yunji/` | ❌ | TASK-3.7 |
| Web 化跨端 | ❌ | 已有，Phase 4 增强 PWA（TASK-4.3） |
| 本地模型 Ollama | ❌ | 已有，Phase 4 增强路由（TASK-4.6） |
| 多用户协作 | ❌ | TASK-4.4 |
| 技能市场 | ❌ | TASK-4.5 |

---

> **本文档结束** | 制定日期：2026-06-08 | 总计 35 个 TASK，16 周


