# 云集智能体工作台 (AW / AgentWork)

> **AI 研发团队协作平台** — 多 Agent DAG 协作 + 异构模型绑定 + 本地版本控制 + 编译期验证 + 全链路发布。
> 
> - **`AW`** = 简称（**A**I-**W**ork），CLI 命令、crate 名、commit scope 用此
> - **`AgentWork`** = 正式品牌名，仓库、官网、商标、对外营销统一用此

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blueviolet.svg)](https://tauri.app)
[![Status](https://img.shields.io/badge/status-M1%20%E8%BF%9B%E8%A1%8C%E4%B8%AD-yellow.svg)]()

## ✨ 核心特性

### 五大差异化支柱

- 🤝 **AI 团队协作引擎** ⭐王炸 — 多 Agent DAG 真并行 + 异构模型差异化绑定（Qwen3.7 推理/GLM5.2 代码/MiniMax3 视觉）+ Git 分支隔离 + 冲突自动解决
- 🧠 **AST-Native 代码认知** ⭐壁垒 — Tree-sitter + LanceDB 构建"类-方法-调用链"知识图谱，Token 消耗降低 70%，跨语言契约监听
- 🛡️ **编译期验证护栏** ⭐可信 — 五级验证管道（Lint/类型/编译/契约/调用链），破坏类型安全的代码直接拒绝合并，零幻觉交付
- 🔄 **TimeFlow 本地版本控制** — 不依赖 git 的本地优先 VCS，自动快照、自由回滚、分支管理，类似云端文档的版本控制体验
- 🚀 **全链路自动发布** — 从 AI 整理正式版本 → 多目标构建 → 多平台分发（GitHub/Gitee/网盘/官网）→ 自动生成 Release Notes

### 完整能力矩阵

- 🔀 **Git 双模兼容** — 隐身模式（纯本地，代码永不上云）/ 同步模式（自动镜像 git）/ 发布模式（只推正式版本）
- 🧠 **AI 语义版本管理** — 自动生成 commit msg、版本号建议、changelog、语义搜索版本
- 🏷️ **AI 版本整理** — 自动区分 WIP/进度/候选/正式版本，识别"可发布版本"
- 🦾 **OpenHands Runtime** — 基于开源 Apache 2.0 工业级 Agent Runtime
- 🔌 **MCP 标准协议** — 工具接入生态化（vs Cursor 私有协议）
- 🦀 **Tauri 2 桌面** — Rust + 系统 WebView，体积比 Electron 小 10x
- 🏪 **Skills 市场** — 可复用任务模板，团队/社区共享
- 🐳 **Docker 沙箱** — 任务执行环境完全隔离，安全可控
- ⚙️ **工作流引擎** — 触发词/定时/事件驱动，YAML 定义自动化流程
- 🌍 **跨平台** — Windows / macOS / Linux 桌面 + Web + CLI

## 🎯 适用场景

### AI 团队协作场景（王炸）
- 🤝 **多 Agent 并行开发**：项目负责人拆解任务 → 后端/前端/测试 Agent 在独立分支并行工作 → 自动合并
- 🎯 **异构模型分工**：Qwen3.7 做推理规划，GLM5.2 写后端代码，MiniMax3 做前端视觉，各展所长
- 🛡️ **零幻觉交付**：AI 代码必须通过五级验证（Lint/类型/编译/契约/调用链）才能合并
- 🔗 **跨语言协作**：后端改 API → AST 自动检测 → 触发前端类型更新任务
- 📊 **任务看板**：DAG 可视化，实时查看多 Agent 工作进度

### 版本控制场景
- 🔄 **自动版本控制**：写代码 → 自动快照 → 任意回滚，无需手动 commit
- 🔍 **语义搜索版本**："回到我加登录功能的那次" → AI 找到匹配版本
- 🏷️ **版本整理**：AI 自动区分 WIP/进度/候选/正式版本，识别可发布版本
- 🔒 **隐私保护**：隐身模式下代码永不上云，企业数据安全

### 自动发布场景
- 🚀 **一句话发布**：输入 `/release` → 自动构建多平台 → 分发 GitHub/Gitee/网盘
- 📝 **自动 Release Notes**：AI 汇总 commit 生成 changelog
- 🏷️ **智能版本号**：基于 Conventional Commits 自动建议 semver
- 📦 **多平台分发**：GitHub Release / Gitee / 蓝奏云 / 阿里云盘 / 官网 CDN

### 开发协作场景
- 🚀 **快速原型**：自然语言描述需求 → Agent 团队生成项目脚手架 → 提 PR
- 🐛 **自动化修 bug**：Agent 读 issue → 定位代码 → 改代码 → 提 PR
- 📝 **代码重构**：Agent 分析代码 → 生成 diff → 人类审核 → 合并
- 🧪 **测试生成**：Agent 自动写单测 → 跑通 → 提 PR
- 📚 **文档维护**：Agent 读代码 → 生成文档 → 同步到仓库
- 🔍 **代码审查**：Agent 自动 review PR → 给出建议

## 🛠️ 技术栈

| 层 | 技术 |
|---|---|
| **桌面端** | Tauri 2 + React 18 + TypeScript 5.5 + shadcn/ui + Tailwind 3.4 |
| **Web 端** | Next.js 14 + React Server Components |
| **CLI** | Rust 1.78+ + clap 4 |
| **后端** | Rust + Axum 0.7 + SQLx + PostgreSQL 16 |
| **数据库（本地）** | SQLite (rusqlite) |
| **TimeFlow VCS** | 自研内容寻址存储 + 快照链（`timeflow-core` crate） |
| **Git 兼容层** | libgit2 (git2-rs) |
| **AI 语义层** | 本地 bge-small + 可选 OpenAI embedding |
| **工作流引擎** | 自研 DAG 执行器（`timeflow-workflow` crate） |
| **发布引擎** | 多平台适配器（`timeflow-release` crate） |
| **Agent Runtime** | OpenHands (Apache 2.0, fork + patch) |
| **工具协议** | MCP (Model Context Protocol) |
| **文件监控** | notify (Rust) |
| **沙箱** | Docker + gVisor |

## 🚀 快速开始

### 环境要求

- **Rust** 1.78+ — [rustup.rs](https://rustup.rs)
- **Node.js** 20+ + **pnpm** 9+
- **Docker** 24+（沙箱功能需要）
- **Git** 2.40+

### 克隆 + 安装

```bash
git clone https://github.com/yunji/agentwork.git
cd agentwork

# 安装依赖
pnpm install

# 启动桌面端（开发模式）
cd apps/desktop
pnpm tauri dev
```

### 第一个 Agent 任务

```bash
# 使用 CLI
cd apps/cli
cargo run -- task "在 examples/hello-world 中添加一个 README.md"

# 或在桌面端输入框直接说：
# "在 examples/hello-world 中添加一个 README.md"
```

## 📂 项目结构

```
4.AgentWork/
├── apps/                4 个端应用
│   ├── desktop/         Tauri 2 桌面端
│   ├── web/             Next.js 14 Web 端
│   ├── cli/             Rust CLI
│   └── mcp-bridge/      MCP 协议桥
├── platformkit/         共享层
│   ├── crates/          Rust 核心库
│   │   ├── timeflow-core/      ⭐ 本地 VCS 引擎（内容寻址+快照链）
│   │   ├── timeflow-ai/        ⭐ AI 语义层（commit msg/版本号/语义搜索）
│   │   ├── timeflow-workflow/  ⭐ 工作流引擎（触发词+DAG）
│   │   ├── timeflow-release/   ⭐ 全链路发布引擎（多平台分发）
│   │   ├── aw-git/             Git 兼容层（libgit2）
│   │   ├── aw-runtime/         Agent Runtime 适配
│   │   └── aw-sandbox/         Docker 沙箱
│   └── packages/        TypeScript 包
├── runtime/             OpenHands 集成层
├── skills/              Agent Skills 市场
├── sandbox-image/       Docker 沙箱镜像
├── docs/                文档
│   ├── TIMEFLOW-DESIGN.md  ⭐ TimeFlow 详细设计
│   ├── ARCHITECTURE.md     系统架构
│   └── ROADMAP.md          路线图
├── tests/               E2E + 集成测试
├── scripts/             构建脚本
└── examples/            示例项目
```

详见 [AGENTS.md](AGENTS.md) § 1.1。

## 🗺️ 路线图

- [x] **M0（2026-06）**：仓库初始化 + 团队对齐 + 工具链就绪
- [ ] **M1（2026-07 上）**：Tauri 桌面骨架 + **TimeFlow VCS 内核**（快照/回滚/分支）
- [ ] **M2（2026-07 下）**：Git 双模兼容 + AI commit msg 自动生成
- [ ] **M3（2026-08 上）**：Skills 市场 v1 + **AI 版本整理**（语义搜索/候选版本识别）
- [ ] **M4（2026-08 下）**：沙箱 + 安全审计 + **工作流引擎**（触发词/DAG）
- [ ] **M5（2026-09）**：内部 Beta + **全链路发布 MVP**（多目标构建/多平台分发）
- [ ] **M6（2026-10）**：公开 Beta
- [ ] **M7（2026-11）**：GA 1.0
- [ ] **M8（2026-12）**：企业版私有化

详见 [docs/ROADMAP.md](docs/ROADMAP.md) 和 [docs/TIMEFLOW-DESIGN.md](docs/TIMEFLOW-DESIGN.md)。

## 💰 商业模式

详见 [tier.yaml](tier.yaml)。

| 档位 | 价格 | 定位 |
|------|:---:|------|
| **Community** | ¥0 | 个人开发者 / 学习者 |
| **Pro** | ¥99/月 | 独立开发者 / 小团队 |
| **Team** | ¥299/月/席位 | 5-50 人研发团队 |
| **Enterprise** | ¥999/月/席位 | 50+ 人企业 / 上市公司 |

## 🆚 竞品对比

| 项目 | 我们 | Cursor | Devin | CrewAI | MetaGPT | OpenHands |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| **开源** | ✅ Apache 2.0 | ❌ 闭源 | ❌ 闭源 | ✅ | ✅ | ✅ |
| **多 Agent DAG 真并行** | ✅ | ⚠️Planner/Worker | ⚠️有限 | ✅ | ❌伪并行 | ⚠️单 Agent |
| **异构模型差异化绑定** | ✅ | ❌固定 | ❌固定 | ✅ | ⚠️有限 | ⚠️单模型 |
| **AST-Native 代码认知** | ✅ Tree-sitter+LanceDB | ❌ | ❌ | ❌ | ❌ | ❌ |
| **编译期验证护栏** | ✅ 五级管道 | ❌ | ❌ | ❌ | ❌ | ❌ |
| **本地 VCS（不依赖 git）** | ✅ TimeFlow | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Git 分支隔离协作** | ✅ | ❌ | ⚠️弱 | ❌ | ❌ | ⚠️基础 |
| **AI 语义搜索版本** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **全链路自动发布** | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **桌面原生** | ✅ Tauri | ✅ | ❌云端 | ❌CLI/Web | ❌ | ❌云端IDE |
| **MCP 协议** | ✅ | ❌ | ❌ | ❌ | ❌ | ✅Server |
| **企业私有化** | ✅ | ❌ | ❌ | ⚠️自建 | ⚠️自建 | ⚠️自建 |

**唯一组合**：多 Agent DAG 真并行 + 异构模型差异化绑定 + AST-Native 代码认知 + 编译期验证护栏 + TimeFlow 版本控制 + 全链路发布。

> 行业论断：将"异构模型 + 真并行 + Git-Native + 桌面原生"同时做到生产级，目前开源和商业产品中没有一家完全实现——这正是 AgentWork 的差异化窗口。

## 📜 许可

- **核心代码**：Apache 2.0
- **企业模块**：商业授权
- **Skills 市场**：双许可（Apache 2.0 + 商业）
- **依赖声明**：详见各 crate / package 的 NOTICE 文件

## 🤝 贡献

欢迎贡献！详见 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)。

贡献流程：
1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feat/amazing-feature`)
3. 提交改动 (`git commit -m 'feat(desktop): add amazing feature'`)
4. 推送分支 (`git push origin feat/amazing-feature`)
5. 创建 Pull Request

## 📞 联系方式

- **GitHub**：https://github.com/yunji/agentwork
- **Gitee 镜像**：https://gitee.com/yunji/agentwork
- **Email**：agentwork@yunji.ai
- **官网**（筹备中）：https://agentwork.yunji.ai

## 🙏 致谢

本项目站在以下开源巨人的肩膀上：

### 核心依赖

- [OpenHands](https://github.com/All-Hands-AI/OpenHands) — Agent Runtime（Apache 2.0）
- [OpenClaw](https://github.com/openclaw/openclaw) — WorkBuddy 设计灵感（Apache 2.0）
- [MCP](https://modelcontextprotocol.io) — 工具协议（MIT）
- [Tauri](https://tauri.app) — 桌面框架
- [libgit2](https://libgit2.org) — Git 操作

### 架构参考

- [UI-TARS-desktop](https://github.com/bytedance/UI-TARS-desktop) by 字节跳动（34K+ Stars，Apache 2.0）
  - 借鉴：Monorepo 布局（apps/ + packages/ + multimodal/ + infra/）
  - 借鉴：MCP 协议集成模式
  - 借鉴：pnpm workspace + Turbo 构建加速
  - 差异化：他们做 GUI Agent，我们做 Git-Native 代码 Agent

### 设计灵感

- [Continue](https://github.com/continuedev/continue) — 开源 AI 编程助手
- [Aider](https://github.com/Aider-AI/aider) — 命令行 AI 编程工具
- [Cursor](https://cursor.com) — AI 编辑器产品形态参考
- [Devin](https://devin.ai) — Agent 任务模式参考

---

> **云集智能体工作台（AgentWork），让 AI 智能体替你写代码、提 PR、跑流水线。**
> *AgentWork — AI Agent Work. Where AI Commits Code Like a Human.*
