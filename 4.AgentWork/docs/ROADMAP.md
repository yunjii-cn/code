# 云集智能体工作台 - 详细路线图

> **版本**：v2.0
> **更新日期**：2026-06-17
> **状态**：M0 启动中
> **配套**：[AGENTS.md](../AGENTS.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) · [tier.yaml](../tier.yaml)

本文档将 12 周目标拆解到**天级任务**。每周结束做一次回顾，必要时调整后续计划。

> **v2.0 重大变更**：产品定位从"Git-Native 代码 Agent"升级为"AI-Native 全链路开发发布工作台"。新增 **TimeFlow** 引擎（本地 VCS + Git 双模 + AI 版本管理 + 全链路发布）作为核心差异化。详见 [TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md)。

---

## 0. 团队假设

| 角色 | 数量 | 投入 | 备注 |
|---|:---:|:---:|---|
| **架构师 / 全栈** | 1 | 100% | 主开发（Trae） |
| **前端** | 1 | 兼职 50% | 桌面端 UI + Web 端 |
| **Rust 后端** | 1 | 兼职 50% | TimeFlow / aw-git / aw-runtime |
| **DevOps** | 0.5 | 兼职 25% | CI / 沙箱 / 部署 |

**单干场景适配**：如无人手，前端/Rust 可由全栈兼顾，相应延期 30-50%。

---

## 1. 12 周时间线总览（v2.0）

```
W0 (06-15~21)  ▓▓░░░ M0 启动周           ✅ 已完成 80%
W1 (06-22~28)  ░░▓▓▓ M1.1 Tauri 骨架      目标：Hello World 窗口
W2 (06-29~07-05) ░░▓▓ M1.2 TimeFlow 内核  目标：快照/回滚/分支
W3 (07-06~12)  ░░▓▓ M1.3 时间轴 UI        目标：可视化版本控制
W4 (07-13~19)  ░░▓▓ M1.4 Git 双模兼容     目标：隐身/同步/发布模式
W5 (07-20~26)  ░░▓▓ M2.1 AI commit msg    目标：自动生成提交信息
W6 (07-27~08-02) ░░▓▓ M2.2 AI 版本整理    目标：候选版本识别+语义搜索
W7 (08-03~09)  ░░▓▓ M3.1 Skills + 工作流  目标：触发词+DAG 引擎
W8 (08-10~16)  ░░▓▓ M3.2 OpenHands 集成   目标：Agent 跑通
W9 (08-17~23)  ░░▓▓ M4.1 沙箱+多目标构建  目标：Docker 沙箱+构建矩阵
W10 (08-24~30) ░░▓▓ M4.2 多平台分发       目标：GitHub/Gitee/网盘发布
W11 (08-31~09-06) ░░▓▓ M5 内部 Beta       目标：100 内部用户跑通全链路
W12 (09-07~13) ░░▓▓ M6 公开 Beta          目标：1000 外部用户

🔵 M0 ✅  🟡 M1-M2 进行中  🟢 M3-M6 计划
```

**v2.0 关键变化**：
- M1 从"Git 集成"改为"**TimeFlow VCS 内核**"（本地优先版本控制）
- M2 从"Agent Runtime"改为"**AI 版本管理**"（commit msg + 版本整理）
- M3 新增"**工作流引擎**"（触发词 + DAG）
- M4 新增"**多目标构建 + 多平台分发**"
- M5 验收标准升级为"**全链路跑通**"（写代码→版本控制→自动发布）

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

## 4. W2 M1.2 TimeFlow VCS 内核（2026-06-29 ~ 07-05）

> **目标**：platformkit/crates/timeflow-core 跑通快照/回滚/分支
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 3

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `cargo new --lib timeflow-core` 初始化 | `platformkit/crates/timeflow-core/` | 编译通过 |
| D2 | 实现内容寻址存储（Blob + Tree） | `src/storage.rs` | SHA-256 哈希 + 去重 |
| D3 | 实现 Snapshot 链（parent + tree） | `src/snapshot.rs` | 快照创建 + 链式查询 |
| D4 | 实现 `snapshot()` + `rollback(snap_id)` | `src/operations.rs` | 单测：快照→改文件→回滚 |
| D5 | 实现 `branch(name)` + `list_snapshots()` | `src/branch.rs` | 单测 5+ 用例 |

**M1.2 验收**：timeflow-core 库 80% 单元测试覆盖，快照/回滚/分支三大核心 API 可用

---

## 5. W3 M1.3 时间轴 UI（2026-07-06 ~ 07-12）

> **目标**：桌面端可视化版本控制界面

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 桌面端 Tauri command `snapshot` / `rollback` 包装 | `apps/desktop/src-tauri/src/commands/timeflow.rs` | 命令可调 |
| D2 | 时间轴组件（垂直时间线 + 快照节点） | `apps/desktop/src/views/Timeline.tsx` | 显示快照列表 |
| D3 | 快照详情面板（diff 视图 + 元数据） | `apps/desktop/src/views/SnapshotDetail.tsx` | 点击查看 diff |
| D4 | 回滚按钮 + 确认对话框 | `apps/desktop/src/views/Timeline.tsx` | 一键回滚 |
| D5 | 文件监控集成（notify）→ 自动快照 | `timeflow-core/src/watcher.rs` | 保存文件自动快照 |

**M1.3 验收**：用户能在桌面端看到时间轴、点击查看 diff、一键回滚

---

## 6. W4 M1.4 Git 双模兼容（2026-07-13 ~ 07-19）

> **目标**：隐身/同步/发布三模式 + M1 Demo

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `aw-git` crate 初始化 + libgit2 集成 | `platformkit/crates/aw-git/` | 编译通过 |
| D2 | 隐身模式（纯 TimeFlow，不碰 git） | `src/modes/stealth.rs` | 默认模式可用 |
| D3 | 同步模式（TimeFlow 快照 → git commit 镜像） | `src/modes/sync.rs` | 快照自动转 commit |
| D4 | 发布模式（只推正式版本到 git） | `src/modes/release.rs` | 候选版本才推 |
| D5 | M1 Demo 录制：写代码→自动快照→回滚→切模式 | `docs/demo/m1-demo.mp4` | 团队对齐 |

**M1 验收**：用户能自动版本控制 + 任意回滚 + 三种模式切换

---

## 7. W5 M2.1 AI commit msg 自动生成（2026-07-20 ~ 07-26）

> **目标**：每次快照后 AI 自动生成 Conventional Commits 格式的提交信息
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 4

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `timeflow-ai` crate 初始化 | `platformkit/crates/timeflow-ai/` | 编译通过 |
| D2 | AI 引擎适配（支持 OpenAI / Ollama 本地） | `src/llm.rs` | 双模式调用 |
| D3 | diff 提取 + prompt 模板（Conventional Commits） | `src/prompts.rs` | 格式校验通过 |
| D4 | `generate_commit_msg(diff)` 实现 | `src/commit_msg.rs` | 单测 5+ 用例 |
| D5 | 集成到快照流程（快照后自动生成 msg） | `timeflow-core` 集成 | 同步模式自动 commit |

**M2.1 验收**：快照后自动生成符合规范的 commit msg，同步模式下自动提交到 git

---

## 8. W6 M2.2 AI 版本整理 + 语义搜索（2026-07-27 ~ 08-02）

> **目标**：AI 自动分类快照 + 语义搜索版本 + 候选版本识别

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 快照分类（WIP/进度/候选/正式） | `timeflow-ai/src/classify.rs` | AI 分类准确率 > 80% |
| D2 | Embedding 索引（bge-small 本地 + OpenAI 可选） | `timeflow-ai/src/embedding.rs` | 索引建立 |
| D3 | 语义搜索 `semantic_search(query)` | `timeflow-ai/src/search.rs` | "回到加登录的版本" 能找到 |
| D4 | 候选版本识别（编译通过+测试通过+改动充分） | `timeflow-ai/src/candidate.rs` | 自动标记候选版本 |
| D5 | 桌面端语义搜索 UI + 候选版本提示 | `apps/desktop/src/views/SemanticSearch.tsx` | 搜索框可用 |

**M2 验收**：用户能语义搜索版本 + AI 自动识别可发布版本

---

## 9. W7 M3.1 Skills 市场 + 工作流引擎（2026-08-03 ~ 08-09）

> **目标**：5 个内置 Skills + 触发词 + DAG 工作流引擎
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 5

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `timeflow-workflow` crate 初始化 + DAG 执行器 | `platformkit/crates/timeflow-workflow/` | 编译通过 |
| D2 | 触发器系统（keyword / schedule / file_change） | `src/triggers.rs` | 3 种触发器可用 |
| D3 | YAML 工作流定义解析 + 执行 | `src/engine.rs` | `.yunji/workflows.yml` 可加载 |
| D4 | 5 个内置 Skills（code-review/test-gen/refactor/doc-gen/bug-fix） | `skills/core/*/SKILL.md` | 5 个 Skills |
| D5 | 桌面端工作流管理 UI + Skills 市场 | `apps/desktop/src/views/Workflows.tsx` | 可视化配置 |

**M3.1 验收**：用户能用触发词触发工作流 + 浏览/选择 Skills

---

## 10. W8 M3.2 OpenHands 集成 + MCP Bridge（2026-08-10 ~ 08-16）

> **目标**：Agent Runtime 跑通 + 3 个 MCP 工具

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `runtime/` 目录 + OpenHands fork | `runtime/openhands/` | 目录在 |
| D2 | aw-runtime 适配层：`AgentRuntime` trait | `platformkit/crates/aw-runtime/` | trait 编译 |
| D3 | 适配 OpenHands API：`start/submit/wait/cancel` | `src/openhands_adapter.rs` | 单测 |
| D4 | MCP Bridge：3 个工具（read_file/write_file/run_shell） | `apps/mcp-bridge/` | 3 工具可用 |
| D5 | OpenHands 调 MCP 工具 E2E | `tests/e2e/openhands_mcp.rs` | E2E 通 |

**M3 验收**：OpenHands + MCP 工具可调用，Agent 能读/写文件/跑 shell

---

## 11. W9 M4.1 Docker 沙箱 + 多目标构建（2026-08-17 ~ 08-23）

> **目标**：沙箱隔离 + 多平台构建矩阵
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 6

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `sandbox-image/Dockerfile` + docker-compose | `sandbox-image/` | 镜像构建通 |
| D2 | aw-sandbox crate 包装 Docker SDK | `platformkit/crates/aw-sandbox/` | 沙箱启动/销毁 |
| D3 | `timeflow-release` crate 初始化 + 构建矩阵定义 | `platformkit/crates/timeflow-release/` | `.yunji/release.yml` 可解析 |
| D4 | 多目标构建执行器（Windows/macOS/Linux/Web） | `src/build_matrix.rs` | 4 目标并行构建 |
| D5 | 沙箱内跑构建 E2E | `tests/e2e/sandbox_build.rs` | 沙箱构建通 |

**M4.1 验收**：Agent 在沙箱内执行 + 多目标构建矩阵可用

---

## 12. W10 M4.2 多平台分发 + Release Notes（2026-08-24 ~ 08-30）

> **目标**：多平台自动分发 + 自动生成 Release Notes

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | GitHub Release 适配器（octocrab） | `timeflow-release/src/github.rs` | 上传 exe 到 Release |
| D2 | Gitee Release 适配器 | `timeflow-release/src/gitee.rs` | 上传到 Gitee |
| D3 | 网盘适配器（蓝奏云/阿里云盘） | `timeflow-release/src/netdisk.rs` | 上传到网盘 |
| D4 | AI Release Notes 生成（changelog + 下载链接） | `timeflow-ai/src/release_notes.rs` | Markdown 输出 |
| D5 | `/release` 触发词全流程 E2E | `tests/e2e/release_pipeline.rs` | 一句话发布 |

**M4 验收**：输入 `/release` → 自动构建 → 多平台分发 → 生成 Release Notes

---

## 13. W11 M5 内部 Beta（2026-08-31 ~ 09-06）

> **目标**：100 个内部用户跑通**全链路**（写代码→版本控制→自动发布）

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | Bug triage + 修复 | GitHub issues 关闭 80% | 严重 bug = 0 |
| D2 | 性能优化（启动 < 3s，快照 < 500ms，发布 < 5min） | benchmarks | 达标 |
| D3 | 用户文档（5 篇快速上手：版本控制/Git双模/工作流/发布/语义搜索） | `docs/user/*.md` | 文档齐 |
| D4 | 100 个内部用户招募 + onboarding | Discord/Slack 群 | 100 用户活跃 |
| D5 | 全链路 Demo + 反馈收集 + 周报 | `docs/internal/m5-report.md` | 周报发 |

**M5 验收**：100 个内部用户至少有 50 人成功跑通"写代码→自动快照→/release 发布"全流程

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
| 2026-06-17 | **v2.0 重大升级**：产品定位升级为"AI-Native 全链路开发发布工作台"，新增 TimeFlow 引擎（本地 VCS + Git 双模 + AI 版本管理 + 全链路发布），重写 M1-M4 里程碑 | Trae |

---

> **本文档是执行宪法**。每周回顾时更新（任务完成、风险触发、应急启动）。
