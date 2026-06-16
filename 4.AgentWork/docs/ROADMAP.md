# 云集智能体工作台 - 详细路线图

> **版本**：v1.0
> **更新日期**：2026-06-17
> **状态**：M0 启动中
> **配套**：[AGENTS.md](file:///e:/软件开发/云集智能编程工作站/4.AgentWork/AGENTS.md) · [ARCHITECTURE.md](file:///e:/软件开发/云集智能编程工作站/4.AgentWork/docs/ARCHITECTURE.md) · [tier.yaml](file:///e:/软件开发/云集智能编程工作站/4.AgentWork/tier.yaml)

本文档将 12 周目标拆解到**天级任务**。每周结束做一次回顾，必要时调整后续计划。

---

## 0. 团队假设

| 角色 | 数量 | 投入 | 备注 |
|---|:---:|:---:|---|
| **架构师 / 全栈** | 1 | 100% | 主开发（Trae） |
| **前端** | 1 | 兼职 50% | 桌面端 UI + Web 端 |
| **Rust 后端** | 1 | 兼职 50% | aw-git / aw-mcp / aw-runtime |
| **DevOps** | 0.5 | 兼职 25% | CI / 沙箱 / 部署 |

**单干场景适配**：如无人手，前端/Rust 可由全栈兼顾，相应延期 30-50%。

---

## 1. 12 周时间线总览

```
W0 (06-15~21)  ▓▓░░░ M0 启动周      ✅ 已完成 60%
W1 (06-22~28)  ░░▓▓▓ M1.1 Tauri 骨架  目标：Hello World 窗口
W2 (06-29~07-05) ░░▓▓ M1.2 libgit2    目标：Rust 端 clone/commit
W3 (07-06~12)  ░░▓▓ M1.3 端到端       目标：桌面端按钮触发 Git
W4 (07-13~19)  ░░▓▓ M1.4 PR 自动化    目标：M1 Demo
W5 (07-20~26)  ░░▓▓ M2.1 OpenHands    目标：Agent 跑通
W6 (07-27~08-02) ░░▓▓ M2.2 MCP Bridge 目标：3 个内置工具
W7 (08-03~09)  ░░▓▓ M3.1 Skills 基础  目标：5 个内置 Skills
W8 (08-10~16)  ░░▓▓ M3.2 Skills UI   目标：Skills 管理界面
W9 (08-17~23)  ░░▓▓ M4.1 Docker 沙箱  目标：沙箱跑通
W10 (08-24~30) ░░▓▓ M4.2 安全审计     目标：审计 + gVisor
W11 (08-31~09-06) ░░▓▓ M5 内部 Beta   目标：100 内部用户
W12 (09-07~13) ░░▓▓ M6 公开 Beta     目标：1000 外部用户

🔵 M0 ✅  🟡 M1-M2 进行中  🟢 M3-M6 计划
```

---

## 2. W0 启动周（2026-06-15 ~ 06-21）

| Day | 任务 | 产出 | 验收 | 状态 |
|:---:|---|---|---|:---:|
| D1 | AGENTS.md 起草 + 4 代产品线契约 | `4.AgentWork/AGENTS.md` | 11 章节无重号 | ✅ |
| D1 | README.md 项目门面 | `4.AgentWork/README.md` | 5 分钟看懂 | ✅ |
| D1 | tier.yaml 商业模式 | `4.AgentWork/tier.yaml` | 4 档位 + reference | ✅ |
| D1 | .gitignore 基础 | `4.AgentWork/.gitignore` | Rust/TS/Docker 全覆盖 | ✅ |
| D2 | 命名层次定稿（AW/AgentWork/agentwork/aw） | 同上文件 | 简称/品牌/仓库/代码 4 层 | ✅ |
| D2 | UI-TARS-desktop 升级为 reference | AGENTS.md §4 | 34K Stars 借鉴 | ✅ |
| D3 | ARCHITECTURE.md 5 层架构 | `docs/ARCHITECTURE.md` | 5 层 + 9 ADR | ✅ |
| D3 | 四源融合战略矩阵 | ARCHITECTURE.md §0.1 | 4 源 × N 行动 | ✅ |
| D4 | **本任务**：ROADMAP.md 详细路线图 | `docs/ROADMAP.md` | 本文 | ⏳ |
| D4 | CONTRIBUTING.md 贡献指南 | `docs/CONTRIBUTING.md` | PR 流程规范 | ⏳ |
| D5 | 工具链准备：rustc + pnpm + tauri-cli + docker | 命令可用 | `cargo --version` 等 | ⏳ |
| D5 | 双 workspace 根：package.json + Cargo.toml | 4.AgentWork/ 根 | `pnpm install` 通 | ⏳ |

**M0 验收**：4.AgentWork/ 仓库具备完整文档体系 + 工具链就绪 + 工程脚手架

---

## 3. W1 M1.1 Tauri 桌面骨架（2026-06-22 ~ 06-28）

> **目标**：apps/desktop 跑出 Hello World 窗口

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `apps/desktop/` 用 `pnpm create tauri-app` 初始化 | 完整 Tauri 2 项目结构 | `pnpm tauri dev` 弹窗 |
| D2 | 配置 Cargo workspace 把 apps/desktop 纳入 | `Cargo.toml` workspace | `cargo build` 通 |
| D3 | 前端基础：3 个页面（任务列表/任务详情/设置） | `apps/desktop/src/views/*` | 路由通 |
| D4 | shadcn/ui 初始化 + Tailwind 配置 | `@aw/ui` 雏形 | 按钮/对话框组件可用 |
| D5 | 前后端通信：Tauri command `greet` 测试 | Rust → JS 调用 | 弹出对话框 |

**M1.1 验收**：Tauri 窗口 + 3 个空页面 + 1 个 command 跑通

---

## 4. W2 M1.2 libgit2 集成（2026-06-29 ~ 07-05）

> **目标**：platformkit/crates/aw-git 跑通 clone + commit

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `cargo new --lib aw-git` 初始化 | `platformkit/crates/aw-git/` | 编译通过 |
| D2 | 加 `git2` 依赖到 Cargo.toml | `Cargo.toml` | `cargo build` 拉依赖 |
| D3 | 实现 `GitRepo::clone(url, path)` | `src/clone.rs` | 单测通过 |
| D4 | 实现 `GitRepo::commit(msg, sign)` | `src/commit.rs` | 单测通过 |
| D5 | 实现 `GitRepo::current_branch()` + `list_branches()` | `src/branch.rs` | 单测 5+ 用例 |

**M1.2 验收**：aw-git 库 80% 单元测试覆盖 + 3 个核心 API 可用

---

## 5. W3 M1.3 端到端（2026-07-06 ~ 07-12）

> **目标**：桌面端按钮触发 Git 操作

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 桌面端 Tauri command `clone_repo` 包装 | `apps/desktop/src-tauri/src/commands/git.rs` | 按钮触发 |
| D2 | 前端按钮：输入 URL → 调 `clone_repo` | `apps/desktop/src/views/TaskList.tsx` | UI 显示进度 |
| D3 | commit UI：textarea 输入 + 按钮提交 | `apps/desktop/src/views/TaskDetail.tsx` | commit hash 显示 |
| D4 | 集成测试：clone → 改文件 → commit | `tests/integration/git_e2e.rs` | E2E 通 |
| D5 | Playwright 桌面端 E2E | `tests/e2e/desktop.spec.ts` | 跨平台 E2E |

**M1.3 验收**：用户能在桌面端点 2 次按钮完成 clone + commit

---

## 6. W4 M1.4 PR 自动化（2026-07-13 ~ 07-19）

> **目标**：M1 Demo 跑通（自然语言 → PR）

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | GitHub API 集成：加 `octocrab` 依赖 | `platformkit/crates/aw-github/` | crate 编译 |
| D2 | 实现 `create_pr(opts)` + `list_prs()` | `src/pr.rs` | 单测 5+ |
| D3 | push 实现：`aw-git.push(remote, branch)` | `src/push.rs` | E2E 通 |
| D4 | 桌面端"提交 PR"按钮（一键） | `apps/desktop/src/views/TaskDetail.tsx` | PR 链接显示 |
| D5 | M1 Demo 录制 + 内部 demo 给团队 | `docs/demo/m1-demo.mp4` | 团队对齐 |

**M1 验收**：用户输入 "fix login bug" → Agent 提 PR → 用户在 GitHub 看到 PR

---

## 7. W5 M2.1 OpenHands 集成（2026-07-20 ~ 07-26）

> **目标**：Agent Runtime 跑通简单任务

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `runtime/` 目录 + OpenHands fork（git submodule 或直接 clone） | `runtime/openhands/` | 目录在 |
| D2 | aw-runtime 适配层：`AgentRuntime` trait | `platformkit/crates/aw-runtime/src/lib.rs` | trait 编译 |
| D3 | 适配 OpenHands API：`start/submit/wait/cancel` | `src/openhands_adapter.rs` | 单测 |
| D4 | 简单任务跑通："列出仓库所有文件" | 测试用例 | 输出正确 |
| D5 | 错误恢复：网络中断重试 | `src/retry.rs` | 3 次重试 |

**M2.1 验收**：OpenHands 跑通 1 个简单任务

---

## 8. W6 M2.2 MCP Bridge（2026-07-27 ~ 08-02）

> **目标**：3 个内置 MCP tools 可用

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `apps/mcp-bridge/` 初始化 | Rust 二进制 | 编译通过 |
| D2 | MCP 协议实现：stdio JSON-RPC | `mcp-bridge/src/protocol.rs` | 协议通 |
| D3 | MCP tool 1: `aw-fs.read_file` | `tools/read_file.rs` | 工具可用 |
| D4 | MCP tool 2: `aw-fs.write_file` + 3: `aw-shell.run` | `tools/{write_file,run}.rs` | 3 工具 |
| D5 | OpenHands 调 MCP 工具的 E2E 测试 | `tests/e2e/openhands_mcp.rs` | E2E 通 |

**M2 验收**：OpenHands + 3 个 MCP 工具可调用，Agent 能读/写文件/跑 shell

---

## 9. W7 M3.1 Skills 基础（2026-08-03 ~ 08-09）

> **目标**：5 个内置 Skills 可加载

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `skills/core/code-review/` Skill 定义 | `code-review/SKILL.md` + `main.py` | 加载通 |
| D2 | 4 个其他 Skills: test-gen/refactor/doc-gen/bug-fix | `skills/core/*/SKILL.md` | 5 个 Skills |
| D3 | `skills/registry.json` 注册表 | `registry.json` | 5 个条目 |
| D4 | Skills 加载机制（aw-core 读 registry） | `platformkit/crates/aw-core/src/skill.rs` | 加载通 |
| D5 | Skills 单测：5 个 Skills 都能加载 | 单测 | 5/5 通 |

**M3.1 验收**：5 个内置 Skills 在程序中可枚举和加载

---

## 10. W8 M3.2 Skills UI（2026-08-10 ~ 08-16）

> **目标**：桌面端 Skills 管理界面

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 桌面端 Skills 列表页面 | `apps/desktop/src/views/Skills.tsx` | 显示 5 个 |
| D2 | Skills 详情页：参数配置 | `Skills/[id].tsx` | 路由通 |
| D3 | Skills 市场 v1：本地浏览 | `views/SkillsMarket.tsx` | 本地列表 |
| D4 | Skills 热加载测试 | `tests/integration/skill_reload.rs` | 改文件自动加载 |
| D5 | Skills 创建向导（用户自定义） | `views/SkillCreate.tsx` | 流程通 |

**M3 验收**：用户能浏览/选择/创建 Skills

---

## 11. W9 M4.1 Docker 沙箱（2026-08-17 ~ 08-23）

> **目标**：Agent 在沙箱内执行

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `sandbox-image/Dockerfile`（基于 ubuntu 22.04） | `Dockerfile` | 镜像构建通 |
| D2 | docker-compose.yml（资源限制 + 网络隔离） | `docker-compose.yml` | compose up 通 |
| D3 | aw-sandbox crate 包装 Docker SDK | `platformkit/crates/aw-sandbox/` | crate 编译 |
| D4 | 沙箱启动/销毁/状态查询 | `src/lifecycle.rs` | 单元测试 |
| D5 | 沙箱内跑 Agent 任务 E2E | `tests/e2e/sandbox.rs` | Agent 在沙箱跑通 |

**M4.1 验收**：Agent 在隔离沙箱内执行，文件改动不污染主机

---

## 12. W10 M4.2 安全审计（2026-08-24 ~ 08-30）

> **目标**：审计日志 + gVisor

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `aw-log` crate（结构化审计日志） | `platformkit/crates/aw-log/` | 编译通 |
| D2 | 所有 Agent 操作埋点 | 各模块集成 | 日志完整 |
| D3 | gVisor (runsc) 集成 | `sandbox-image/gvisor/` | runsc 配置 |
| D4 | 网络隔离（egress proxy + 域名白名单） | `infra/egress-proxy/` | 阻断外网 |
| D5 | 沙箱 + 审计 E2E | `tests/e2e/sandbox_audit.rs` | E2E 通 |

**M4 验收**：所有 Agent 操作有审计，沙箱有 gVisor 隔离

---

## 13. W11 M5 内部 Beta（2026-08-31 ~ 09-06）

> **目标**：100 个内部用户跑通

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | Bug triage + 修复 | GitHub issues 关闭 80% | 严重 bug = 0 |
| D2 | 性能优化（启动 < 3s，操作 < 500ms） | benchmarks | 达标 |
| D3 | 用户文档（5 篇快速上手） | `docs/user/*.md` | 文档齐 |
| D4 | 100 个内部用户招募 + onboarding | Discord/Slack 群 | 100 用户活跃 |
| D5 | 反馈收集 + 周报 | `docs/internal/m5-report.md` | 周报发 |

**M5 验收**：100 个内部用户至少有 50 人成功跑过 1 个 Agent 任务

---

## 14. W12 M6 公开 Beta（2026-09-07 ~ 09-13）

> **目标**：1000 个外部用户

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 营销：Twitter/HN/ProductHunt 发布 | 推文 + Landing page | 发布 |
| D2 | 公开渠道：Discord/GitHub Discussions | 社区 | 1000 注册 |
| D3 | 文档站（VitePress） | `docs.yunji.ai/aw/` | 上线 |
| D4 | 反馈分类 + 关键问题修复 | issues 关闭 | P0 全部解决 |
| D5 | M6 报告 + M7 计划 | `docs/internal/m6-report.md` | 计划发布 |

**M6 验收**：1000 外部用户，至少 200 人跑通任务

---

## 15. M7-M8 长期规划（2026 Q4）

> M7-M8 不在 12 周硬计划内，按季度推进

| Milestone | 时间 | 目标 |
|---|---|---|
| **M7** | 2026-10 | GA 1.0：付费用户 100+，Pro/Team 档位上线 |
| **M8** | 2026-11 | Enterprise 档位：私有化部署 + SLA |
| **M9** | 2026-12 | Skills 市场公测：第三方 Skills 发布 |
| **M10** | 2027-01 | 多 Agent 协作（2+ Agent 协同） |
| **M11** | 2027-02 | 自定义 LLM 接入（私有模型） |
| **M12** | 2027-03 | 移动端 PoC（iOS/Android） |

---

## 16. 风险登记册

| # | 风险 | 等级 | 概率 | 影响 | 触发条件 | 应急方案 |
|:---:|---|:---:|:---:|:---:|---|---|
| R1 | OpenHands 大版本变更破坏 API | 🟡 | 中 | 高 | M5-W5 | aw-runtime 适配层隔离（已设计） |
| R2 | Tauri 2 在 Windows 上 WebView2 兼容问题 | 🟡 | 中 | 中 | M1-W1 | 退路：PyQt6 + QWebEngineView（用 1.PC 经验） |
| R3 | libgit2 GPL 传染 | 🟡 | 低 | 中 | M1-W2 | 仅 binary link，应用层 Apache 2.0 |
| R4 | Docker Desktop 用户未装 | 🟡 | 中 | 中 | M4-W9 | 启动时检测 + 引导安装 |
| R5 | LLM API 成本失控 | 🟡 | 中 | 高 | M5-W11 | 月度预算告警 + 任务配额 |
| R6 | 沙箱逃逸 | 🔴 | 低 | 极高 | M4-W10 | 立即停服 + bounty + gVisor 兜底 |
| R7 | 单干人手不足延期 | 🟡 | 高 | 中 | 全程 | 砍非核心功能（自定义 Skills / Web 端） |
| R8 | MCP 协议 RFC 变更 | 🟢 | 低 | 中 | M2-W6 | 锁 v0.x，跟踪上游 |
| R9 | 微软/Google 突然发布竞品 | 🟢 | 低 | 高 | 任何时候 | 加快 Skills 市场 + 私有化壁垒 |
| R10 | 用户对 AW 缩写混淆 AWS | 🟢 | 中 | 低 | 全程 | 对外营销统一用 AgentWork |

**风险审查频率**：每周一次（每周一 09:00）

---

## 17. 应急方案（如果某周延期）

| 场景 | 触发 | 应急 |
|---|---|---|
| **M1 延期到 W5** | Tauri 2 兼容问题 | 砍 MCP Bridge（先单 OpenHands），W5 集中做 M2 |
| **M2 延期到 W7** | OpenHands API 变动 | 简化 Adapter，先跑通"读文件 + 写文件" |
| **M3 延期到 W9** | Skills 抽象设计卡住 | 先做 1 个内置 Skill 验证模式 |
| **M4 延期到 W11** | Docker 沙箱性能问题 | 退到 gVisor only（无 Docker） |
| **M5 严重延期** | 核心 bug 未解决 | 砍 M6 公开 Beta，先把内部用户做稳 |

**核心原则**：**砍功能不砍质量**。延期的代价是砍掉次要功能，不是"快速修 bug"。

---

## 18. 资源需求

| 资源 | 数量 | 用途 | 备注 |
|---|:---:|---|---|
| **GitHub 仓库** | 1 | `yunji/agentwork` | M0 建 |
| **Gitee 镜像** | 1 | 国内加速 | M0 建 |
| **域名** | 1 | `agentwork.yunji.ai` | M5 公开 Beta |
| **Docker Hub 账号** | 1 | 镜像分发 | M4-W9 |
| **LLM API 预算** | ¥3000/月 | OpenAI/Anthropic | M3-W7 开始 |
| **测试机** | 1 台 | macOS/Linux E2E | M1-W3（借用） |
| **Grafana Cloud** | 1 | 日志聚合 | M4-W10 |

---

## 19. 度量指标（每周跟踪）

| 指标 | 目标 | 测量方法 |
|---|---|---|
| **代码覆盖率** | ≥ 80% | `cargo tarpaulin` |
| **CI 通过率** | ≥ 95% | GitHub Actions 历史 |
| **任务完成率** | ≥ 90% | 本路线图 vs 实际 |
| **E2E 测试数** | ≥ 50 | Playwright 统计 |
| **文档完整度** | 100% 章节 | 文档审计 |
| **用户活跃度** (M5+) | ≥ 50% DAU/WAU | 内部统计 |
| **Agent 任务成功率** (M5+) | ≥ 70% | 任务日志 |
| **PR 自动通过率** (M5+) | ≥ 60% | 任务日志 |

---

## 20. 变更历史

| 日期 | 变更 | 作者 |
|---|---|---|
| 2026-06-17 | 初始路线图 v1.0（12 周 × 天级任务 + 10 风险 + 8 应急） | Trae |

---

> **本文档是执行宪法**。每周回顾时更新（任务完成、风险触发、应急启动）。
