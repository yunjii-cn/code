# 云集智能体工作台 - 系统架构

> **版本**：v1.0
> **更新日期**：2026-06-17
> **状态**：M0 设计阶段

本文档定义 **AgentWork** 项目的整体架构。任何架构级变更必须先在此文档更新，再实施。

---

## 0. 设计目标

| 目标 | 优先级 | 度量 |
|---|:---:|---|
| **Git-Native** | P0 | 100% 工作流以 PR 形式交付 |
| **可审计** | P0 | 所有操作有日志、可回放 |
| **沙箱安全** | P0 | Agent 执行环境完全隔离 |
| **跨平台** | P1 | Windows / macOS / Linux 桌面 + Web + CLI |
| **轻量** | P1 | 桌面安装包 < 10MB，内存 < 50MB |
| **可扩展** | P1 | Skills / MCP 工具 / 自定义 Agent |
| **高性能** | P2 | Agent 任务平均 5 分钟内完成 |

---

## 0.1 产品战略：四源融合

> **4.AgentWork 不是从零发明**，而是 **四源融合** 的产品：
> **市场差异化 + 我们的特色 + 市面产品优点 + 前三代价值提取**。

### 战略 1：市场差异化（vs 头部竞品）

| 竞品 | 他们的短板 | 我们的差异化 |
|---|---|---|
| **Devin** ($500/月) | 闭源、黑盒沙箱 | ✅ Apache 2.0 + gVisor 开源沙箱 + 价格低 5x |
| **Cursor** ($40/月) | 编辑器插件、无 PR 自动化 | ✅ 完整 IDE + Git-Native PR 自动化 |
| **Copilot Workspace** ($39/月) | 仅云端、无本地 | ✅ 桌面端本地优先 + 云端可选 |
| **OpenHands Cloud** | 云端为主、桌面弱 | ✅ Tauri 2 桌面优先 |
| **Aider** (开源) | 仅 CLI、无 GUI | ✅ GUI + CLI + Web 三端 |

### 战略 2：我们的特色（核心壁垒）

| 特色 | 价值 |
|---|---|
| **Git-Native 工作流** | PR 即交付，永久审计 |
| **MCP 标准协议** | 工具生态可互认（不孤岛） |
| **Apache 2.0 开源** | 商业可信赖、可私有化 |
| **Tauri 2 桌面** | 体积 < 10MB（vs Electron 100MB+） |
| **Skills 市场** | 任务模板可复用、团队共享 |
| **Docker + gVisor 沙箱** | 任务执行完全隔离 |

### 战略 3：市面产品优点（要吸收的）

| 来源 | 优点 | 4.AgentWork 落地 |
|---|---|---|
| **UI-TARS-desktop** (34K⭐) | Monorepo 结构、rfcs/ 目录、MCP 集成 | ✅ 已借鉴（AGENTS.md §4） |
| **OpenClaw** (310K⭐) | 通用 Agent 框架、Skill 系统 | ✅ runtime/ + skills/ 目录 |
| **Devin** | 端到端任务、PR 自动化 | ✅ 数据流 A 场景 |
| **Cursor** | Composer 模式、丝滑体验 | ✅ @aw/ui 组件库 |
| **Aider** | CLI 设计、diff 可读 | ✅ apps/cli + PR diff 视图 |
| **WorkBuddy** | 多端形态（GUI/CLI/Web） | ✅ apps/{desktop,web,cli,mcp-bridge} |

### 战略 4：前三代产品价值提取

> 1.PC / 2.WEB / 3.dev 已经积累的代码资产，不能浪费。

| 来源 | 价值资产 | 4.AgentWork 提取方式 | 优先级 |
|---|---|---|:---:|
| **1.PC** | 单实例控制、UI 布局经验 | → aw-config + @aw/ui | P1 |
| **2.WEB** | 17 个 API 设计模式、Vue→React 经验 | → aw-sdk + apps/web | P1 |
| **3.dev** `skill_market` | Skill 任务模板市场 | ⭐ → skills/ 目录（核心） | P0 |
| **3.dev** `github_core` | GitHub 集成 | ⭐ → GitHub MCP server | P0 |
| **3.dev** `auth_core` | 认证 + RBAC | ⭐ → aw-auth + SSO | P0 |
| **3.dev** `audit_core` | 审计日志 | ⭐ → aw-log | P0 |
| **3.dev** `durable_store` | 持久化存储 | ⭐ → aw-store | P0 |
| **3.dev** `billing_core` | 订阅计费 | → aw-billing | P1 |
| **3.dev** `tenant_core` | 多租户 | → aw-tenant | P1 |
| **3.dev** `whisper_core` | 语音转写 | → 未来语音输入 | P3 |
| **3.dev** `ai_engine` | AI 引擎 | ⚠️ → OpenHands 替代 | 替换 |
| **3.dev** `model_router` | 多模型路由 | ⚠️ → MCP 标准替代 | 替换 |
| **3.dev** `yjs_server` | Yjs 实时协作 | ❌ 暂不需要（异步为主） | 跳过 |
| **3.dev** `team_core` | 团队协作 | ❌ 暂不需要 | 跳过 |
| **3.dev** `responsive_core` | 响应式核心 | ❌ React 自带 | 跳过 |
| **3.dev** `tailscale_core` | 内网穿透 | ❌ 暂不需要 | 跳过 |

**统计**：3.dev 共 16 个 platformkit 模块 → 7 个核心复用 + 2 个评估 + 2 个替换 + 4 个跳过

**提取原则**：
- ✅ **复用**：技术栈兼容、4.AgentWork 需要的
- ⚠️ **替换**：4.AgentWork 用更好的方案（MCP 替代 model_router，OpenHands 替代 ai_engine）
- ❌ **跳过**：4.AgentWork 场景不需要的（异步为主，不需要 Yjs；React 自带响应式）

### 4 个来源的执行优先级

| 来源 | 阶段 | 负责人 |
|---|---|---|
| **战略 1（差异化）** | 全程贯穿 | 全体 |
| **战略 2（我们的特色）** | M0-M5 重点 | 架构组 |
| **战略 3（市面优点）** | M1-M3 调研 + 借鉴 | 调研组 |
| **战略 4（前三代价值）** | M1-M2 提取 + 适配 | 跨代协作组 |

---

## 1. 架构总览（5 层架构）

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Layer 5: 端应用 (apps/)                                                  │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐             │
│ │  Desktop   │ │    Web     │ │    CLI     │ │ MCP Bridge │             │
│ │  Tauri 2   │ │  Next.js   │ │   Rust     │ │   Rust     │             │
│ │  (R+W)     │ │  (W)       │ │  (R)       │ │  (R)       │             │
│ └──────┬─────┘ └──────┬─────┘ └──────┬─────┘ └──────┬─────┘             │
│        │              │              │              │                    │
│        │ R: Rust 前端 │              │              │                    │
│        │ W: WebView   │              │              │                    │
├────────┼──────────────┼──────────────┼──────────────┼────────────────────┤
│        ▼              ▼              ▼              ▼                    │
│ Layer 4: 共享业务层 (platformkit/)                                        │
│ ┌──────────────────────────────────────────────────────────────────┐    │
│ │ Rust crates:                                                       │    │
│ │  aw-core       核心抽象 (Task, Skill, Agent traits)                │    │
│ │  aw-git        Git 操作 (libgit2 绑定, clone/commit/PR/branch)     │    │
│ │  aw-store      持久化 (rusqlite 本地 / sqlx 服务端)                │    │
│ │  aw-mcp        MCP 协议实现 (stdio/HTTP transport)                 │    │
│ │  aw-runtime    Agent 适配层 (封装 OpenHands)                       │    │
│ │  aw-config     配置管理 (TOML + 环境变量)                          │    │
│ │  aw-log        审计日志 (结构化 + 加密)                            │    │
│ ├──────────────────────────────────────────────────────────────────┤    │
│ │ TS packages:                                                       │    │
│ │  @aw/types         共享 TypeScript 类型 (Task, Skill, Agent...)    │    │
│ │  @aw/ui            共享 React 组件库 (Button, Dialog, Tree...)    │    │
│ │  @aw/sdk           浏览器/Node SDK (HTTP client)                  │    │
│ │  @aw/api-client    API 调用封装 (TanStack Query 适配)              │    │
│ │  @aw/mcp-client    MCP 客户端 (stdin/stdout JSON-RPC)              │    │
│ └──────────────────────────────────────────────────────────────────┘    │
│        │                                                                │
├────────┼────────────────────────────────────────────────────────────────┤
│        ▼                                                                │
│ Layer 3: Agent Runtime (runtime/)                                       │
│ ┌──────────────────────────────────────────────────────────────────┐    │
│ │  OpenHands (Apache 2.0, fork + patch)                             │    │
│ │  - 任务调度 (Task Scheduler)                                       │    │
│ │  - 工具调用 (MCP Tool Router)                                       │    │
│ │  - 上下文管理 (Context Manager)                                     │    │
│ │  - 错误恢复 (Retry + Rollback)                                      │    │
│ │  - 人机协作 (Human-in-the-loop)                                     │    │
│ │  upstream: github.com/All-Hands-AI/OpenHands                      │    │
│ └──────────────────────────────────────────────────────────────────┘    │
│        │                                                                │
├────────┼────────────────────────────────────────────────────────────────┤
│        ▼                                                                │
│ Layer 2: Skills (skills/)                                                │
│ ┌──────────────────────────────────────────────────────────────────┐    │
│ │  5 个内置 Skills:                                                   │    │
│ │   code-review    代码审查 (PR Review Bot)                          │    │
│ │   test-gen       测试生成 (auto unit test)                         │    │
│ │   refactor       代码重构 (lint + 重构)                            │    │
│ │   doc-gen        文档生成 (从代码生成 README/JSDoc)                 │    │
│ │   bug-fix        Bug 修复 (读 issue → 修代码 → 提 PR)               │    │
│ │  + 用户自定义 Skills (按目录加载)                                   │    │
│ │  + Skills 注册表 (registry.json)                                   │    │
│ └──────────────────────────────────────────────────────────────────┘    │
│        │                                                                │
├────────┼────────────────────────────────────────────────────────────────┤
│        ▼                                                                │
│ Layer 1: 基础设施 (sandbox-image/)                                       │
│ ┌──────────────────────────────────────────────────────────────────┐    │
│ │  - Docker 沙箱 (镜像: agentwork/sandbox:0.1)                       │    │
│ │  - gVisor 隔离 (runsc)                                              │    │
│ │  - 网络隔离 (egress proxy + 域名白名单)                              │    │
│ │  - 资源限制 (CPU/Mem/Disk cgroup)                                  │    │
│ │  - 审计日志 (stdout → Loki)                                         │    │
│ │  - 一键清理 (任务结束销毁容器)                                       │    │
│ └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘

图例：R = Rust 实现, W = Web/TypeScript 实现
```

### 1.1 关键设计原则

| 原则 | 落地 |
|---|---|
| **端应用与共享层分离** | `apps/*` 可独立部署；`platformkit/*` 不可独立运行 |
| **双 workspace** | `package.json` 管 TS/Node，`Cargo.toml` 管 Rust |
| **第三方代码独立** | `runtime/openhands/` 便于 upstream sync |
| **Skills 即产品** | `skills/` 是核心资产，独立目录+版本管理 |
| **沙箱与代码解耦** | `sandbox-image/` Docker 镜像定义独立维护 |
| **可审计优先** | 所有跨进程调用都有 trace ID + 日志 |

---

## 2. 核心模块设计

### 2.1 aw-core（核心抽象）

**职责**：定义所有核心 trait 和数据结构

```rust
// platformkit/crates/aw-core/src/lib.rs

/// Agent 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: Uuid,
    pub prompt: String,           // 用户输入的自然语言
    pub repo: RepoRef,            // 目标仓库
    pub skills: Vec<SkillRef>,    // 使用的 Skills
    pub timeout: Duration,        // 超时
    pub sandbox: SandboxConfig,   // 沙箱配置
    pub created_at: DateTime<Utc>,
}

/// Skill 抽象
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, ctx: &SkillContext) -> Result<SkillOutput>;
}

/// MCP 工具抽象
#[async_trait]
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn invoke(&self, args: Value) -> Result<Value>;
}
```

### 2.2 aw-git（Git 操作）

**职责**：封装 libgit2，提供 Git 高层 API

**关键 API**：
```rust
impl GitRepo {
    pub async fn clone(&self, url: &str, path: &Path) -> Result<Self>;
    pub async fn commit(&self, msg: &str, sign: bool) -> Result<Oid>;
    pub async fn push(&self, remote: &str, branch: &str) -> Result<()>;
    pub async fn create_branch(&self, name: &str) -> Result<Branch>;
    pub async fn create_pr(&self, opts: &PrOptions) -> Result<PullRequest>;
    pub async fn diff(&self, base: &str, head: &str) -> Result<Diff>;
}
```

**依赖**：`git2-rs` 0.19（libgit2 1.8 绑定）

### 2.3 aw-mcp（MCP 协议）

**职责**：实现 Model Context Protocol 客户端和服务端

**传输方式**：
- **stdio**：本地进程间通信（Desktop 默认）
- **HTTP/SSE**：云端 Agent ↔ MCP servers
- **WebSocket**：实时双向（未来）

**实现**：
```rust
// platformkit/crates/aw-mcp/src/lib.rs

pub struct McpClient { /* ... */ }
pub struct McpServer { /* ... */ }

impl McpClient {
    pub async fn connect_stdio(&mut self, cmd: &Command) -> Result<()>;
    pub async fn list_tools(&self) -> Result<Vec<ToolSchema>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
}
```

### 2.4 aw-runtime（Agent 适配层）

**职责**：封装 OpenHands Runtime，提供统一接口

**为什么不直接用 OpenHands**：
1. 上游 API 不稳定，需要本地封装层
2. 我们要替换/扩展部分行为（如 Git-Native 工作流）
3. 避免 OpenHands 大版本升级破坏我们代码

**接口**：
```rust
#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn submit_task(&self, task: AgentTask) -> Result<TaskHandle>;
    async fn wait(&self, handle: TaskHandle) -> Result<TaskResult>;
    async fn cancel(&self, handle: TaskHandle) -> Result<()>;
}
```

### 2.5 @aw/ui（共享 React 组件）

**职责**：跨 desktop/web 共享的 UI 组件

**核心组件**：
- `<TaskCard>` — Agent 任务卡片
- `<PrDiffView>` — PR diff 可视化
- `<SkillPicker>` — Skills 选择器
- `<RepoTree>` — 文件树
- `<AgentLogStream>` — 实时日志流
- `<SandboxStatus>` — 沙箱状态指示器

**技术栈**：React 18 + shadcn/ui + Tailwind CSS 3.4

### 2.6 apps/desktop（Tauri 2 桌面端）

**技术栈**：
- 前端：React 18 + Vite 5 + TypeScript 5.5
- 后端：Rust（src-tauri/）
- 状态：Zustand 4.5
- 存储：rusqlite + 本地文件

**子进程架构**：
```
┌────────────────────────────────────────┐
│  AgentWork Desktop (Tauri 主进程)      │
│  ┌──────────────┐  ┌──────────────┐    │
│  │  WebView     │  │ Rust 后端    │    │
│  │  (React UI)  │←→│ (Tauri API)  │    │
│  └──────────────┘  └──────┬───────┘    │
│                            │            │
│                            │ spawn      │
│                            ▼            │
│              ┌──────────────────────┐  │
│              │  aw-mcp-bridge       │  │
│              │  (独立子进程)         │  │
│              │  stdio JSON-RPC      │  │
│              └──────────────────────┘  │
│                            │            │
│                            │ docker run │
│                            ▼            │
│              ┌──────────────────────┐  │
│              │  aw-sandbox          │  │
│              │  (一次性容器)         │  │
│              └──────────────────────┘  │
└────────────────────────────────────────┘
```

### 2.7 apps/mcp-bridge（MCP 协议桥）

**职责**：独立 MCP 服务器进程，供所有端应用调用

**为什么独立**：
1. 沙箱安全（主进程被攻破不会影响 MCP）
2. 多语言客户端（Rust/Node/Python 都能调）
3. 跨进程隔离（崩溃不影响主程序）

**进程模型**：
- **stdio JSON-RPC**（Desktop 默认，最简单）
- **HTTP + SSE**（Web/远程，云端部署）

---

## 3. 数据流（3 个核心场景）

### 3.1 场景 A：自然语言 → PR（核心流程）

```
用户输入: "在 src/api 加个 /users 接口"
   ↓
[Desktop UI] 解析自然语言
   ↓
[aw-sdk] 创建 AgentTask
   {
     prompt: "在 src/api 加个 /users 接口",
     repo: { url: "github.com/user/app", branch: "main" },
     skills: ["code-gen"],
     sandbox: { image: "agentwork/sandbox:node-20" }
   }
   ↓
[aw-mcp-bridge] 接收任务，写入任务队列
   ↓
[aw-runtime] 调度 Agent Worker
   ↓
[OpenHands] 执行循环:
  1. 思考: "需要先读 src/api/ 现有结构"
  2. 调 aw-git.read_dir (MCP tool)
  3. 思考: "现在写代码"
  4. 调 aw-fs.write_file (MCP tool)
  5. 调 aw-test.run_tests (MCP tool)
  6. 调 aw-git.commit (MCP tool)
  7. 调 aw-git.push (MCP tool)
  8. 调 github.create_pr (MCP tool, 关联到 origin)
   ↓
[aw-sandbox] 在隔离容器内执行（gVisor）
  - 写文件、跑测试都在沙箱
  - 网络只能访问白名单域名
  - 完成后容器销毁
   ↓
[aw-runtime] 返回结果
  {
    pr_url: "https://github.com/user/app/pull/42",
    pr_number: 42,
    files_changed: 2,
    tests_passed: true
  }
   ↓
[Desktop UI] 弹出通知
  ┌────────────────────────────────────┐
  │ ✅ PR 已创建: #42                  │
  │ 📁 2 个文件，+47 -3 行             │
  │ 🧪 测试通过                         │
  │ 🔗 https://github.com/.../pull/42 │
  │ [查看 PR] [立即 Review]            │
  └────────────────────────────────────┘
```

### 3.2 场景 B：Issue → 自动修复 PR

```
GitHub Issue #123 创建（用户手动 / 自动）
   ↓
[aw-mcp-bridge] 接收 GitHub webhook (push event)
   ↓
[aw-runtime] 创建任务
   {
     type: "auto-fix",
     issue_url: "github.com/user/app/issues/123",
     prompt: "读 issue → 修代码 → 提 PR"
   }
   ↓
[OpenHands] 复用场景 A 的执行循环
  + 额外: PR description 自动加 "Closes #123"
   ↓
PR 创建后，aw-mcp-bridge 监听 PR CI
   ↓
CI 通过 → 评论 issue: "修复完成，详见 #124"
CI 失败 → 自动重试 1 次 → 失败则评论 issue: "Agent 尝试失败，需要人工"
```

### 3.3 场景 C：PR 评论 → Agent 修复

```
用户在 GitHub PR #42 评论: "password 字段没加密"
   ↓
[aw-mcp-bridge] 接收 PR review comment webhook
   ↓
[aw-runtime] 找到原 Agent 任务（按 PR 关联）
   ↓
[OpenHands] 新一轮执行:
  prompt: "用户反馈: password 字段没加密，请修复"
  上下文: 之前的所有代码改动
   ↓
[aw-git] 推新 commit 到原 PR
   ↓
[aw-mcp-bridge] 评论回复: "已修复，请重新 review"
   ↓
CI 重跑 → 通过 → 通知用户
```

---

## 4. 部署架构

### 4.1 桌面端（本地部署）

```
用户电脑
├── AgentWork Desktop (Tauri 2 主程序)
│   ├── 前端 (React, 渲染在 WebView2/WKWebView)
│   ├── 后端 (Rust, 单进程)
│   └── 调 aw-mcp-bridge (子进程, stdio)
├── aw-mcp-bridge (独立 Rust 二进制, 9090)
│   └── 调 aw-git, aw-store, aw-mcp servers
├── Docker Desktop (必需, 沙箱执行)
│   └── agentwork/sandbox:0.1 (按任务启动/销毁)
└── ~/.agentwork/ (本地数据目录)
    ├── config.toml
    ├── db.sqlite (任务历史)
    └── repos/ (本地 clone 的仓库缓存)
```

### 4.2 云端（自部署 Web 版）

```
云服务器 / Kubernetes
├── aw-web (Next.js 14, 3000) — Web UI
├── aw-api (Axum 0.7, 8080) — API server
├── aw-runtime (OpenHands 包装, 8081) — Agent 调度
├── aw-mcp-bridge-cluster (9090) — MCP 集群
├── aw-sandbox-pool (Docker Swarm / k8s) — 沙箱池
├── PostgreSQL 16 (数据)
├── Redis 7 (缓存 + 任务队列)
├── S3 / 阿里云 OSS (Skills 市场、文件)
├── Prometheus + Grafana (监控)
└── Loki (日志聚合)
```

### 4.3 私有化部署（Enterprise 客户）

```
客户内网
├── Kubernetes 集群 (or 虚拟机)
│   ├── 上述云端组件 (全部)
│   ├── 私有 LLM (Qwen / DeepSeek / 智谱)
│   └── 私有 Git (GitLab / Gitea)
├── 防火墙
│   ├── 允许: 客户内网 ↔ 云更新服务
│   └── 阻断: 所有外网（除更新和白名单）
└── 审计 → SIEM (Splunk / 国产化方案)
```

---

## 5. ADR（架构决策记录）

> 任何重大架构变更必须新增一条 ADR。

### ADR-001：选择 Tauri 2 而非 Electron

**状态**：✅ 已采纳
**日期**：2026-06-17

**背景**：需要选桌面应用框架。

**决策**：使用 Tauri 2。

**理由**：
- 体积小 10x（< 10MB vs Electron ~100MB）
- 内存占用少 1/3（10-50MB vs 100-300MB）
- Rust 后端性能更好
- 系统集成更紧密

**代价**：
- macOS 用 WKWebView（Safari 引擎），需注意 CSS 兼容性
- Linux 用 WebKitGTK，少部分 Linux 发行版需要装依赖

### ADR-002：选择 Rust 后端而非 Python

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- 性能（编译型 vs 解释型）
- 内存安全（无 GC 暂停）
- 沙箱隔离天然适合（gVisor + Rust 内存安全）
- 与 Tauri 2 同语言，无 FFI 开销

**代价**：
- 开发速度比 Python 慢 1.5x
- 需要 Rust 学习曲线

### ADR-003：双 workspace（pnpm + Cargo）

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- Tauri 桌面端有 Rust（src-tauri/）
- MCP Bridge 是 Rust
- 共享业务层同时有 Rust + TS
- 各管各的，依赖清晰

**替代方案**：纯 TS（不行，需要 Rust）；纯 Rust（不行，前端还是要 TS）

### ADR-004：OpenHands fork 而非自研

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- 节省 1 年研发时间
- Apache 2.0 许可，可商用
- 社区活跃，问题修复快

**风险与对策**：
- 风险：上游 API 变化 → 对策：aw-runtime 适配层隔离
- 风险：上游停止维护 → 对策：本地 fork 永久可用

### ADR-005：MCP 作为工具协议

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- Anthropic 推动的事实标准
- 生态丰富（数百个 MCP server）
- 跨语言互操作

**替代方案**：自研协议（不推荐，孤岛）

### ADR-006：libgit2 直接绑定而非 git CLI

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- 性能（in-process vs fork+exec）
- 跨平台一致（不依赖系统 git 版本）
- 原子操作（事务性更好）

**风险**：libgit2 GPL v2 with linking exception
**对策**：仅 binary link，应用层保持 Apache 2.0，保留依赖声明

### ADR-007：Skills 目录与代码分离

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- Skills 是产品核心资产
- 独立目录便于版本管理（类似 npm packages）
- 用户可热加载/卸载 Skills

**结构**：
```
skills/
├── core/          内置 5 个 Skills
├── registry.json  Skills 注册表
└── user/          用户自定义（运行时下载）
```

### ADR-008：Docker 沙箱 + gVisor

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- Docker 普及度高
- gVisor 提供内核级隔离（防止容器逃逸）
- 网络隔离可配置（egress proxy）

**风险**：性能开销 ~10%（gVisor syscall 拦截）
**对策**：仅 Agent 执行时用沙箱，用户编辑时不进沙箱

### ADR-009：Git-Native 优先（所有工作流 = PR）

**状态**：✅ 已采纳
**日期**：2026-06-17

**理由**：
- 可审计（PR 永久记录）
- 可回放（PR 关联完整代码改动）
- 可回滚（PR 不合并就不影响主分支）
- 可审核（人类 review 是最后一道关）
- 可 CI（PR 触发自动测试）

**这是核心差异化**，不可妥协。

---

## 6. 模块依赖图

```
                    ┌─────────────────┐
                    │  apps/desktop   │ ──┐
                    │  (Tauri 2)      │   │
                    └─────────────────┘   │
                            │              │
                    ┌─────────────────┐   │
                    │   apps/web      │ ──┤
                    │  (Next.js)      │   │
                    └─────────────────┘   │
                            │              │
                    ┌─────────────────┐   │
                    │   apps/cli      │ ──┤
                    │    (Rust)       │   │
                    └─────────────────┘   │
                            │              │
                            ▼              │
       ┌────────────────────────────────────┴────┐
       │         platformkit/                    │
       │  ┌─────────┐ ┌─────────┐ ┌──────────┐  │
       │  │ @aw/ui  │ │ @aw/sdk │ │ @aw/types│  │  (TS)
       │  └────┬────┘ └────┬────┘ └────┬─────┘  │
       │       │           │           │         │
       │       └───────────┼───────────┘         │
       │                   │                     │
       │  ┌─────────┐ ┌────┴────┐ ┌──────────┐  │
       │  │ aw-git  │ │ aw-mcp  │ │ aw-store │  │  (Rust)
       │  └────┬────┘ └────┬────┘ └────┬─────┘  │
       │       │           │           │         │
       │       └───────────┼───────────┘         │
       │                   │                     │
       │            ┌──────┴──────┐              │
       │            │  aw-core    │              │
       │            │ aw-runtime  │              │
       │            └──────┬──────┘              │
       └───────────────────┼─────────────────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │   runtime/           │
                │  OpenHands fork      │
                └──────────┬───────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │  skills/ │ │sandbox-  │ │   MCP    │
        │  (5+ N)  │ │ image/   │ │ servers  │
        └──────────┘ └──────────┘ └──────────┘
```

**关键约束**：
- `platformkit/` 不能依赖 `apps/`
- `runtime/` 只能被 `aw-runtime` 包装
- `skills/` 是数据，不是代码
- `sandbox-image/` 是镜像定义

---

## 7. 风险与限制

| 风险 | 等级 | 对策 |
|------|:---:|------|
| OpenHands 大版本变更破坏 API | 🟡 中 | aw-runtime 适配层隔离 |
| libgit2 GPL 传染 | 🟡 中 | 仅 binary link，Apache 2.0 应用层 |
| Tauri 2 仍在演进 | 🟡 中 | 锁版本 2.x，月度评估升级 |
| Docker Desktop 用户未装 | 🟡 中 | 启动时检测 + 引导安装 |
| 沙箱逃逸 | 🔴 高 | gVisor + 渗透测试 + bounty |
| LLM 成本失控 | 🟡 中 | 月度预算告警 + 任务配额 |
| MCP 协议 RFC 变更 | 🟢 低 | 锁 v0.x，跟踪上游 |

---

## 8. 未来扩展

| 方向 | 时间 | 状态 |
|---|---|---|
| Web 端 GA | M5-M7 | 设计中 |
| 移动端（iOS/Android） | M12+ | 评估中 |
| 多 Agent 协作 | M9+ | 暂不做 |
| 自定义 LLM 接入 | M4+ | 计划中 |
| Skills 市场公测 | M6 | 计划中 |
| 国际化 (i18n) | M5 | 计划中 |

---

## 9. 变更历史

| 日期 | 变更 | 作者 |
|---|---|---|
| 2026-06-17 | 初始架构 v1.0（5 层 + 9 ADR） | Trae |

---

> **本文档是架构宪法**。修改前请先讨论，再更新本文档，再实施代码。
