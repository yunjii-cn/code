# 云集智能体工作台 (AW / AgentWork)

> **AI Agent Workbench** — 让 AI 智能体替你写代码、提 PR、跑流水线。
> 
> - **`AW`** = 简称（**A**I-**W**ork），CLI 命令、crate 名、commit scope 用此
> - **`AgentWork`** = 正式品牌名，仓库、官网、商标、对外营销统一用此

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blueviolet.svg)](https://tauri.app)
[![Status](https://img.shields.io/badge/status-M0%20%E5%90%AF%E5%8A%A8-yellow.svg)]()

## ✨ 核心特性

- 🤖 **AI-Agent-Work** — AI 智能体替你做工作（Writing / Reviewing / Refactoring / Testing）
- 🔀 **Git-Native** — Agent 工作流 = PR 提交，可审计、可回放、可回滚
- 🦾 **OpenHands Runtime** — 基于开源 Apache 2.0 工业级 Agent Runtime
- 🔌 **MCP 标准协议** — 工具接入生态化（vs Cursor 私有协议）
- 🦀 **Tauri 2 桌面** — Rust + 系统 WebView，体积比 Electron 小 10x
- 🏪 **Skills 市场** — 可复用任务模板，团队/社区共享
- 🐳 **Docker 沙箱** — 任务执行环境完全隔离，安全可控
- 🌍 **跨平台** — Windows / macOS / Linux 桌面 + Web + CLI

## 🎯 适用场景

- 🚀 **快速原型**：自然语言描述需求 → Agent 生成项目脚手架 → 提 PR
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
| **Agent Runtime** | OpenHands (Apache 2.0, fork + patch) |
| **工具协议** | MCP (Model Context Protocol) |
| **Git 操作** | libgit2 (git2-rs) |
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
│   └── packages/        TypeScript 包
├── runtime/             OpenHands 集成层
├── skills/              Agent Skills 市场
├── sandbox-image/       Docker 沙箱镜像
├── docs/                文档
├── tests/               E2E + 集成测试
├── scripts/             构建脚本
└── examples/            示例项目
```

详见 [AGENTS.md](AGENTS.md) § 1.1。

## 🗺️ 路线图

- [x] **M0（2026-06）**：仓库初始化 + 团队对齐
- [ ] **M1（2026-07 上）**：Tauri 桌面骨架 + Git 集成
- [ ] **M2（2026-07 下）**：Agent Runtime + MCP Bridge
- [ ] **M3（2026-08 上）**：Skills 市场 v1
- [ ] **M4（2026-08 下）**：沙箱 + 安全审计
- [ ] **M5（2026-09）**：内部 Beta（100 用户）
- [ ] **M6（2026-10）**：公开 Beta
- [ ] **M7（2026-11）**：GA 1.0
- [ ] **M8（2026-12）**：企业版私有化

## 💰 商业模式

详见 [tier.yaml](tier.yaml)。

| 档位 | 价格 | 定位 |
|------|:---:|------|
| **Community** | ¥0 | 个人开发者 / 学习者 |
| **Pro** | ¥99/月 | 独立开发者 / 小团队 |
| **Team** | ¥299/月/席位 | 5-50 人研发团队 |
| **Enterprise** | ¥999/月/席位 | 50+ 人企业 / 上市公司 |

## 🆚 竞品对比

| 项目 | 我们 | GitHub Copilot | Cursor | Devin |
|------|:---:|:---:|:---:|:---:|
| **开源** | ✅ Apache 2.0 | ❌ 闭源 | ❌ 闭源 | ❌ 闭源 |
| **Git-Native** | ✅ PR 即交付 | ⚠️ 弱 | ❌ 无 | ⚠️ 弱 |
| **本地运行** | ✅ 桌面端 | ❌ 云端 | ✅ 编辑器 | ❌ 云端 |
| **MCP 协议** | ✅ | ❌ | ❌ | ❌ |
| **Skills 市场** | ✅ | ⚠️ 弱 | ❌ | ❌ |
| **桌面体积** | 🟢 < 10MB | - | - | - |

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
