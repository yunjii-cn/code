# 云集智能体工作台 - AGENTS.md（4 代架构契约）

> **目的**：所有 AI 协作（Trae / Cursor / Claude Code / 任意 agent）必须遵守的项目契约。
> **生效日期**：2026-06-17
> **简称**：`AW`（**A**I-**W**ork 缩写，3 字母好记）
> **正式品牌名**：`AgentWork`（**A**gent**W**ork 全称，无 AWS 冲突）
> **目录名**：`4.AgentWork/`（保留首字母大写可读形式）
> **GitHub 仓库名**：`agentwork`（避免 AW/AWS 视觉撞名）
> **CLI 命令 / crate 名 / npm scope**：`aw`（与简称一致）
> **中文名**：`云集智能体工作台`

> **命名风险声明**：AW 简称与 AWS (Amazon Web Services) 字符串相近。所有对外营销、域名、商标都使用 `AgentWork` 全称；机器/技术标识符（CLI 命令、crate 名、commit scope、npm scope）使用小写 `aw`（Rust/npm 规范强制小写）。

---

## 0. 4 代产品线定位

本项目是 **云集智能**（Yunji）旗下 **第 4 代** 产品线：

| 代 | 目录 | 中文名 | 英文名 | 档位 | 价格 | 状态 |
|:---:|:---|:---|:---:|:---:|:---:|------|
| 1 | `1.PC/` | 云集桌面 | Yunji Desktop | Free | ¥0 | ✅ 完整可跑 |
| 2 | `2.WEB/` | 云集 Web | Yunji Web | Pro | ¥9.9/月 | 🟡 半成品 |
| 3 | `3.dev/` | 云集旗舰 | Yunji Flagship | Business | ¥99/月 | 🟢 全力开发 |
| **4** | **`4.AgentWork/`** | **云集智能体工作台** | **AW (AgentWork)** | **Enterprise** | **¥999/月** | 🆕 启动中 |

**核心差异化（五大支柱，v3.2 重写）**：
- 🤝 **AI 团队协作引擎** ⭐王炸 — 多 Agent DAG 真并行 + 异构模型差异化绑定（Qwen3.7 推理 / GLM5.2 代码 / MiniMax3 视觉）+ 邮件式异步协作
- 🧠 **AI 员工培训引擎** ⭐壁垒 — 知识库 + 规则 + 话术 + 评估四要素，纯 prompt + few-shot，**不依赖 GPU 即可行业化**
- 🛡️ **行业模板 + 自定义** ⭐B 端关键 — 5 核心细分行业（电商-穿搭/电子 + 教育-早教/素质 + 金融-证券）+ 5 候补（金融-建筑/法律/财务/HR/医疗）+ 用户自定义模板向导
- 💬 **任务驱动 + 互动双引擎** — 维护阶段用 ChatPanel 自然语言互动，开发/执行阶段用 TaskBoard DAG 可视化
- 🔄 **本地优先 + 云端协同** — 4 核 8G 宝塔服务器即可跑通反代 + 同步 + 备份，**不需要 GPU 集群**

**完整能力**：
- 🔀 **Git 双模兼容** — 隐身模式（纯本地）/ 同步模式（自动镜像 git）/ 发布模式（只推正式版本）
- 🧠 **AI 语义版本管理** — 自动 commit msg、版本号建议、changelog、语义搜索版本
- ⚙️ **工作流引擎** — 触发词/定时/事件驱动，YAML 定义自动化流程
- 🤖 **OpenHands Runtime** — 基于开源 Apache 2.0 工业级 Agent Runtime
- 🔌 **MCP 标准协议** — 工具接入生态化（vs Cursor 私有协议）
- 🦀 **Tauri 2 桌面** — Rust + WebView，体积小 10x（vs Electron）
- 🏪 **Skills 市场** — 可复用任务模板，团队/社区共享
- 📦 **10 行业模板**（5 核心 + 5 候补）— 电商-穿搭/电子、教育-早教/素质、金融-证券 5 核心 + 5 候补
- 🛠️ **模板自定义向导**（v3.2 P0）— 用户可基于核心模板 fork 或从空白创建专属模板

> **v3.2 演进方向**：从"AI 研发团队"全面升级为"AI 智能体工作台"——
> - 不再以"写代码"为核心场景，扩展到 5 大行业（电商/教育/金融/法律/财务）的 AI 员工场景
> - 新增 AI 员工培训引擎（壁垒核心）+ 行业模板（B 端关键）+ 模板自定义（P0 需求）
> - 新增 ChatPanel 互动机制（维护阶段友好）+ 任务驱动 + 互动双引擎
> - 新增 6 个内置 AI 员工（3 通用 + 3 行业，扩展到 18 个 = 3 通用 + 15 行业）
> - 详见 [docs/IMPROVEMENT-PLAN.md](docs/IMPROVEMENT-PLAN.md) v2.2
> - 未来规划归档到 [docs/FUTURE-PLANS.md](docs/FUTURE-PLANS.md) v1.1

**所有 agent 必须**：
1. **明确自己工作在哪个代**（1.PC / 2.WEB / 3.dev / 4.AgentWork / 跨代）
2. **明确自己工作在哪个层**（apps / platformkit / runtime / skills / sandbox-image）
3. **新功能先在 4.AgentWork 落地**（本目录是 2026 下半年新主力）
4. **明确自己工作在哪个场景**（代码协作 / AI 员工独立工作 / 5 行业模板 / 互动面板）
5. **5 行业模板是 B 端获客主力**——任何新功能优先考虑行业模板的兼容性

---

## 1. 路径契约

### 1.1 项目根结构

```
云集智能体工作台/                    <-- 项目根 (4.AgentWork/)
├── apps/                            <-- 4 个端应用
│   ├── desktop/                     Tauri 2 桌面端（Windows/macOS/Linux）
│   ├── cli/                         Rust CLI（aw 命令）
│   ├── web/                         Next.js 14 Web 端
│   ├── mcp-bridge/                  MCP 协议桥（独立二进制）
├── platformkit/                     <-- 共享层
│   ├── crates/                      Rust 核心库
│   │   ├── timeflow-core/           ⭐ 本地 VCS 引擎（内容寻址+快照链+分支）
│   │   ├── timeflow-ai/             ⭐ AI 语义层（commit msg/版本号/语义搜索/分类）
│   │   ├── timeflow-workflow/       ⭐ 工作流引擎（触发词+DAG 执行器）
│   │   ├── timeflow-release/        ⭐ 全链路发布引擎（多目标构建+多平台分发）
│   │   ├── agent-team/              ⭐🆕 AI 团队协作引擎（多 Agent DAG+异构模型+分支隔离）
│   │   ├── ast-native/              ⭐🆕 AST 代码认知引擎（Tree-sitter+LanceDB 知识图谱）
│   │   ├── verification-guardrails/ ⭐🆕 编译期验证护栏（五级验证管道+零幻觉交付）
│   │   ├── aw-git/                  Git 兼容层（libgit2，三模式切换）
│   │   ├── aw-runtime/              Agent Runtime 适配（OpenHands）
│   │   ├── aw-sandbox/              Docker 沙箱
│   │   └── aw-log/                  审计日志
│   └── packages/                    TypeScript 包（@aw/ui, @aw/sdk...）
├── runtime/                         OpenHands 集成层（fork + patch）
├── skills/                          Agent Skills 市场内容
├── sandbox-image/                   Docker 沙箱镜像 + compose 文件
├── docs/                            架构/API/用户文档
│   ├── TIMEFLOW-DESIGN.md           ⭐ TimeFlow 详细设计
│   ├── AGENT-TEAM-DESIGN.md         ⭐🆕 AI 团队协作引擎设计
│   ├── AST-NATIVE-DESIGN.md         ⭐🆕 AST 代码认知引擎设计
│   ├── VERIFICATION-GUARDRAILS.md   ⭐🆕 编译期验证护栏设计
│   ├── IMPROVEMENT-PLAN.md          🆕 完善与增强计划（W9.5+ 演进路线）
│   ├── ARCHITECTURE.md              系统架构
│   ├── ROADMAP.md                   路线图（v3.0，20 周）
│   └── CONTRIBUTING.md              贡献指南
├── tests/                           E2E + 集成测试（Playwright + cargo test）
├── scripts/                         构建/部署/工具脚本
├── examples/                        示例项目（demo 仓库）
├── .github/workflows/               CI（GitHub Actions）
├── .gitignore
├── AGENTS.md                        本文件
├── README.md
├── tier.yaml
└── package.json                     pnpm workspace 根
```

### 1.2 与其他代产品线的关系

- ✅ **不替代** 3.dev（云集旗舰仍是主力，3.dev 走 SaaS 路线）
- ✅ **不依赖** 3.dev 的 `platformkit/`（独立技术栈：Tauri 2 + Rust 1.78+）
- ✅ **可借鉴** 3.dev 的 API 设计、UI 思想（仅思想，不代码复用）
- ✅ **企业服务**（私有化 / 授权 / 培训）独立议价
- ⚠️ **与 1.PC 完全独立**：1.PC 是 PyQt6 桌面，本项目是 Tauri 2 桌面，不混用

---

## 2. 技术栈契约

### 2.1 桌面端（apps/desktop）

- **Tauri 2.x**（Rust + 系统 WebView，体积比 Electron 小 10x）
- 前端：**React 18 + Vite 5 + TypeScript 5.5+**
- UI：**shadcn/ui + Tailwind CSS 3.4**
- 状态：**Zustand 4.5**（轻量、TS 友好）
- 数据：**TanStack Query 5 + rusqlite**（本地 SQLite）
- 编辑器：**Monaco Editor**（代码展示/编辑）
- 终端：**xterm.js**（嵌入 shell）

### 2.2 后端（platformkit/crates/api）

- **Rust 1.78+**（稳定版，edition 2021）
- Web 框架：**Axum 0.7 + tower 0.5**
- 数据库：**SQLx 0.8 + PostgreSQL 16**（服务端）/ **rusqlite 0.32**（桌面端）
- 认证：**JWT (jsonwebtoken 9) + OAuth 2.1**
- 序列化：**serde 1 + Protocol Buffers (prost 0.13)**
- 异步运行时：**tokio 1.40**
- HTTP 客户端：**reqwest 0.12**

### 2.3 关键依赖

| 依赖 | 许可证 | 用途 |
|------|:---:|------|
| **OpenHands** | Apache 2.0 | Agent Runtime（fork + 定制） |
| **OpenClaw** | Apache 2.0 | WorkBuddy 基座思想借鉴 |
| **MCP** | MIT | Model Context Protocol 工具协议 |
| **libgit2** | GPL v2 with linking exception | Git 操作（git2-rs 绑定） |
| **ripgrep** | MIT | 代码搜索（rg 绑定） |
| **tree-sitter** | MIT | 代码解析（多语言 AST） |

---

## 3. 工作流契约

### 3.1 修改本项目（4.AgentWork）

- ✅ 新功能优先在本目录落地
- ✅ platformkit 独立维护，不引用 3.dev/platformkit
- ⚠️ 借鉴 3.dev 时**复制思想不复制代码**（避免强耦合）
- ⚠️ 任何 Rust 改动必须通过 `cargo clippy -- -D warnings` + `cargo test`
- ⚠️ 任何 TypeScript 改动必须通过 `pnpm lint` + `pnpm test`
- ⚠️ 任何 PR 必须有至少 1 个 E2E 测试覆盖

### 3.2 修改其他代

- ✅ 修改 1.PC / 2.WEB / 3.dev 时**不动** `4.AgentWork/`
- ✅ 跨代协作通过 `doc/` 根级文档约定

### 3.3 企业服务（enterprise-services，未来扩展）

- 🟣 私有化部署 / 商业授权 / SLA / 培训
- 💰 价格单独议价，不进 `tier.yaml`
- 📞 涉及金额时回到根级 `doc/产品线矩阵.md`

---

## 4. 参考项目（reference projects）

> **本节目的**：列出业界成熟的同方向项目，我们**借鉴架构/工程实践**，但**不做竞品**。
> 详见 `tier.yaml` § `reference_projects`。

### 4.1 UI-TARS-desktop（bytedance）

| 字段 | 值 |
|---|---|
| **GitHub** | https://github.com/bytedance/UI-TARS-desktop |
| **License** | Apache-2.0 |
| **Stars** | 34K+（2026-05） |
| **Commits** | 1,109 |
| **首发** | 2025-01-21 |
| **技术栈** | TypeScript + pnpm + Turbo + Electron + MCP |

**为什么是参考项目而不是竞品**：
- 赛道不同：他们做 **GUI Agent**（操控电脑），我们做 **Git-Native 代码 Agent**（写代码提 PR）
- 许可证一致：都是 Apache-2.0，理论上可互操作
- 生态协同：都用 MCP 协议，工具可互认

**借鉴要点**（写入本目录结构决策）：

| 借鉴 | 落地到 4.AgentWork |
|---|---|
| Monorepo 分层（apps/ + packages/ + multimodal/ + infra/） | ✅ 已采用类似结构（apps/ + platformkit/crates/ + platformkit/packages/） |
| pnpm workspace + Turbo 加速 CI | ✅ `package.json` 用 pnpm workspace |
| rfcs/ 目录管设计文档 | ✅ `docs/` 目录承担此角色 |
| MCP 作为核心依赖 | ✅ 独立 `apps/mcp-bridge/` 进程 |
| 多 apps 共享 packages | ✅ `platformkit/packages/` 跨 desktop/web 共享 |

**避免复制的坑**：
- ❌ 不学他们用 Electron（我们用 Tauri 2，体积小 10x）
- ❌ 不学他们做 GUI 操控（我们做 Git 仓库交互，方向不同）

### 4.2 持续跟踪

新参考项目需满足：
- Apache-2.0 / MIT 许可证
- 同方向（Agent / 代码生成 / Git 集成）
- 5K+ Stars 或 3+ 大厂采用
- 与我们无直接商业冲突

候选跟踪列表（季度评估）：
- [OpenHands](https://github.com/All-Hands-AI/OpenHands) — Agent Runtime（我们 fork 它）
- [Tarko](https://github.com/bytedance/UI-TARS-desktop) — Agent 框架（UI-TARS 子项目）
- [Continue](https://github.com/continuedev/continue) — 开源 AI 编程助手
- [Aider](https://github.com/Aider-AI/aider) — 命令行 AI 编程工具

---

## 5. 构建契约

### 5.1 开发模式命令

| 应用 | 命令 | 默认端口 |
|------|------|:---:|
| **Desktop** | `cd apps/desktop && pnpm tauri dev` | - |
| **Web** | `cd apps/web && pnpm dev` | 3000 |
| **API** | `cd platformkit/crates/api && cargo run` | 8080 |
| **CLI** | `cd apps/cli && cargo run -- --help` | - |
| **MCP Bridge** | `cd apps/mcp-bridge && cargo run` | 9090 |

### 5.2 打包命令

| 应用 | 命令 | 输出位置 |
|------|------|----------|
| **Desktop (Windows)** | `cd apps/desktop && pnpm tauri build` | `apps/desktop/target/release/bundle/msi/*.msi` |
| **Desktop (macOS)** | 同上（需在 macOS 上执行） | `apps/desktop/target/release/bundle/dmg/*.dmg` |
| **Desktop (Linux)** | 同上 | `apps/desktop/target/release/bundle/appimage/*.AppImage` |
| **Web** | `cd apps/web && pnpm build` | `apps/web/dist/` |
| **CLI** | `cd apps/cli && cargo build --release` | `apps/cli/target/release/aw` |

### 5.3 打包格式

- **Desktop** → `.msi` (Windows) / `.dmg` (macOS) / `.AppImage` (Linux)
- **Web** → Docker image (`aw-web:vYYYYMMDD`)
- **CLI** → 单二进制静态链接 + 交叉编译（cargo-cross）

### 5.4 版本号规范

- 统一格式：`vYYYY.MM.DD.HHMM`（例：`v2026.06.17.1430`）
- 桌面端版本号通过 `apps/desktop/src-tauri/tauri.conf.json` 的 `version` 字段管理
- 每次 release 自动从 git tag 注入

---

## 6. 测试契约

| 层级 | 命令 | 覆盖率目标 |
|------|------|:---:|
| **Rust 单元测试** | `cargo test --workspace` | ≥ 80% |
| **TypeScript 测试** | `pnpm test` | ≥ 70% |
| **E2E 测试** | `pnpm test:e2e`（Playwright） | 关键路径 100% |
| **沙箱集成测试** | `docker compose -f sandbox-image/docker-compose.test.yml up` | - |
| **Linting (Rust)** | `cargo clippy --all-targets -- -D warnings` | 0 warning |
| **Linting (TS)** | `pnpm lint` | 0 error |
| **Formatting** | `cargo fmt --check && pnpm format:check` | - |

### 6.1 测试分层

- **Unit**：纯函数、工具方法（无 IO）
- **Integration**：模块间交互（mock 外部依赖）
- **E2E**：真实用户场景（Playwright + 真实 Tauri 窗口）
- **Sandbox**：完整 Docker 环境，跑 Agent 任务

---

## 7. Git 契约

### 7.1 提交范围

- ✅ `4.AgentWork/` 全部代码
- ✅ `4.AgentWork/docs/`、`4.AgentWork/scripts/`、`4.AgentWork/examples/`
- ❌ `target/`、`node_modules/`、`dist/`、`build/`、`.cargo/`、`.next/`

### 7.2 远程仓库

- `origin` → GitHub（`git@github.com:yunji/agentwork.git`）
- `gitee` → Gitee（`git@gitee.com:yunji/agentwork.git`，国内加速）

### 7.3 Commit 格式（Conventional Commits）

```
<type>(<scope>): <subject>

<body>

<footer>
```

- **type**：`feat` / `fix` / `docs` / `style` / `refactor` / `test` / `chore` / `perf`
- **scope**：`desktop` / `web` / `cli` / `api` / `platformkit` / `runtime` / `skills` / `sandbox` / `docs` / `ci`
- **subject**：中文，50 字以内，动词开头
- **body**：可选，详细说明
- **footer**：可选，关联 issue / 破坏性变更标记

### 7.4 分支策略

- `main` — 主分支，受保护，必须通过 CI
- `feat/*` — 功能开发
- `fix/*` — bug 修复
- `release/*` — 发布准备
- `hotfix/*` — 紧急修复

---

## 8. 里程碑（12 周路线图）

| 里程碑 | 时间 | 内容 | 验收 |
|:---:|:---:|------|------|
| **M0** | 2026-06 | 仓库初始化 + 团队对齐 | 本文件落地、CI 通 |
| **M1** | 2026-07 上 | Tauri 桌面骨架 + Git 集成 | libgit2 跑通 clone/commit/PR |
| **M2** | 2026-07 下 | Agent Runtime + MCP Bridge | OpenHands 跑通简单任务 |
| **M3** | 2026-08 上 | Skills 市场 v1 | 5 个内置 Skills 可用 |
| **M4** | 2026-08 下 | 沙箱 + 安全审计 | Docker 沙箱隔离 + 审计日志 |
| **M5** | 2026-09 | Beta 发布 | 100 个内部用户跑通 |
| **M6** | 2026-10 | 公开 Beta | 1000 外部用户 |
| **M7** | 2026-11 | GA 1.0 | 付费用户 100+ |
| **M8** | 2026-12 | 企业版 | 私有化部署跑通 |

---

## 9. 文档契约

### 9.1 必须文档

- `README.md` — 项目门面，5 分钟看懂项目
- `AGENTS.md` — 本文件，AI 协作契约
- `tier.yaml` — 商业模式
- `启动.bat` — Windows 启动菜单（dev / build / test / clippy）
- `docs/ARCHITECTURE.md` — 系统架构图
- `docs/ROADMAP.md` — 详细路线图
- `docs/IMPROVEMENT-PLAN.md` — 完善+增强计划（v2.2）
- `docs/FUTURE-PLANS.md` — 未来规划参考库（v1.1）
- `docs/QUICKSTART.md` — 5 分钟快速开始
- `docs/USER-GUIDE.md` — 详细用户指南
- `docs/EMPLOYEE-GUIDE.md` — AI 员工定义指南
- `docs/CONTRIBUTING.md` — 贡献指南
- `CHANGELOG.md` — 变更日志（自动生成）

### 9.2 文档语言

- 用户文档 → 中文
- 代码注释 → 中文（关键逻辑）
- API 文档 → 英文（面向全球）
- Commit message → 中文

---

## 10. 风险与对策

| 风险 | 等级 | 对策 |
|------|:---:|------|
| libgit2 GPL 传染 | 🟡 中 | 仅 binary link，应用层 Apache 2.0，保留依赖声明 |
| OpenHands API 不稳定 | 🟡 中 | fork 固定版本，本地 patch，独立升级 |
| MCP 协议变更 | 🟢 低 | 锁定 v0.x，跟踪上游 RFC |
| Tauri 2 仍在演进 | 🟡 中 | 固定 2.x 小版本，月度评估升级 |
| 沙箱逃逸 | 🔴 高 | 多层隔离 + 审计 + 渗透测试 |

---

## 11. 变更历史

| 日期 | 变更 | 负责人 |
|------|------|--------|
| 2026-06-17 | 新建 4.AgentWork 仓库，4 代产品线契约 v1.0 | Trae |
| 2026-06-17 | 新增 §4 参考项目（UI-TARS-desktop 升级为 reference project） | Trae |
| 2026-06-17 | **v2.0 重大升级**：产品定位升级为"AI-Native 全链路开发发布工作台"，新增 TimeFlow 引擎（4 个 crate：timeflow-core/ai/workflow/release），更新核心差异化、项目结构、发布边界契约 | Trae |
| 2026-06-17 | **v3.0 重大升级**：产品定位升级为"AI 研发团队协作平台"，新增三大壁垒级 crate（agent-team/ast-native/verification-guardrails），核心差异化升级为五大支柱，路线图扩展为 20 周 | Trae |
| 2026-06-18 | **v3.1 演进方向**：基于 W9 完工后的反思，新增 [docs/IMPROVEMENT-PLAN.md](docs/IMPROVEMENT-PLAN.md)，明确"任务驱动 + 互动机制"双引擎战略，引入 AI 员工定义 + 培训系统（壁垒核心），为跨行业（客服/销售/财务/...）打基础 | Trae + 用户 |
| 2026-06-18 | **v3.2 演进方向**：IMPROVEMENT-PLAN 升级 v2.2（5 细分行业：电商-穿搭/电子 + 教育-早教/素质 + 金融-证券）+ 5 候补（含金融-建筑投资）+ M4.2 D6 模板自定义向导（P0 用户硬需求）；并加入 v2.3 远期愿景（M6 2027-05+：WordPress 式产品周边 + 6 层用户生态）| 用户 + Trae |

---

> **最后**：本文件是项目宪法。所有 agent 修改前请**先读**本文件，**再行动**。
