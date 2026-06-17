# 云集智能体工作台 - 详细路线图

> **版本**：v3.0
> **更新日期**：2026-06-17
> **状态**：M1 进行中（W1 已完成）
> **配套**：[AGENTS.md](../AGENTS.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) · [AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) · [AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) · [VERIFICATION-GUARDRAILS.md](VERIFICATION-GUARDRAILS.md) · [tier.yaml](../tier.yaml)

本文档将 20 周目标拆解到**周级任务**。每周结束做一次回顾，必要时调整后续计划。

> **v3.0 重大变更**：从 v2.0 的 12 周扩展为 **20 周**，融入三大壁垒级能力：
> 1. **AI 团队协作引擎**（多 Agent DAG 协作 + 异构模型绑定）— 详见 [AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md)
> 2. **AST-Native 代码认知引擎**（Tree-sitter + LanceDB 知识图谱）— 详见 [AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md)
> 3. **编译期验证护栏**（五级验证管道 + 零幻觉交付）— 详见 [VERIFICATION-GUARDRAILS.md](VERIFICATION-GUARDRAILS.md)
>
> 这三大能力让 AgentWork 从"AI 编程工具"升级为"**AI 研发团队协作平台**"，技术壁垒显著提升，开发周期延长 8 周是值得的投入。

---

## 0. 团队假设

| 角色 | 数量 | 投入 | 备注 |
|---|:---:|:---:|---|
| **架构师 / 全栈** | 1 | 100% | 主开发（Trae） |
| **前端** | 1 | 兼职 50% | 桌面端 UI + Web 端 |
| **Rust 后端** | 1 | 兼职 50% | TimeFlow / agent-team / ast-native |
| **DevOps** | 0.5 | 兼职 25% | CI / 沙箱 / 部署 |

**单干场景适配**：如无人手，前端/Rust 可由全栈兼顾，相应延期 30-50%。

---

## 1. 20 周时间线总览（v3.0）

```
=== M1: TimeFlow 本地版本控制（W1-W4）✅ 进行中 ===
W1  (06-22~28)  ▓▓░░░ M1.1 Tauri 骨架        ✅ 已完成
W2  (06-29~07-05) ░░▓▓ M1.2 TimeFlow 内核     目标：快照/回滚/分支
W3  (07-06~12)  ░░▓▓ M1.3 时间轴 UI          目标：可视化版本控制
W4  (07-13~19)  ░░▓▓ M1.4 Git 双模兼容       目标：隐身/同步/发布模式

=== M2: AI 语义版本管理（W5-W6）===
W5  (07-20~26)  ░░▓▓ M2.1 AI commit msg      目标：自动生成提交信息
W6  (07-27~08-02) ░░▓▓ M2.2 AI 版本整理      目标：候选版本识别+语义搜索

=== M3: AI 团队协作引擎（W7-W10）⭐王炸 ===
W7  (08-03~09)  ░░▓▓ M3.1 团队模板+模型路由   目标：角色定义+异构模型绑定
W8  (08-10~16)  ░░▓▓ M3.2 DAG 调度引擎       目标：任务依赖图+并行调度
W9  (08-17~23)  ░░▓▓ M3.3 双 Agent MVP       目标：架构师+Coder 协作跑通
W10 (08-24~30)  ░░▓▓ M3.4 Git 分支隔离+冲突  目标：独立分支+冲突预检

=== M4: AST-Native 代码认知（W11-W14）⭐壁垒 ===
W11 (08-31~09-06) ░░▓▓ M4.1 Tree-sitter 集成 目标：多语言 AST 解析
W12 (09-07~13)  ░░▓▓ M4.2 LanceDB 图谱      目标：代码知识图谱构建
W13 (09-14~20)  ░░▓▓ M4.3 智能上下文提取     目标：Token 降低 70%
W14 (09-21~27)  ░░▓▓ M4.4 调用链+契约监听   目标：跨语言契约监听

=== M5: 验证护栏 + 工作流 + 发布（W15-W17）⭐可信 ===
W15 (09-28~10-04) ░░▓▓ M5.1 验证护栏管道     目标：五级验证+错误反馈
W16 (10-05~11)  ░░▓▓ M5.2 工作流+沙箱       目标：触发词+Docker 沙箱
W17 (10-12~18)  ░░▓▓ M5.3 多平台发布         目标：构建矩阵+多平台分发

=== M6: 整合 + Beta（W18-W20）===
W18 (10-19~25)  ░░▓▓ M6.1 全链路整合         目标：五大支柱串联
W19 (10-26~11-01) ░░▓▓ M6.2 内部 Beta        目标：100 内部用户跑通
W20 (11-02~08)  ░░▓▓ M6.3 公开 Beta         目标：1000 外部用户

🔵 M1 ✅进行中  🟡 M2-M3 计划  🟢 M4-M6 远期
```

**v3.0 关键变化**：
- **新增 M3（4 周）**：AI 团队协作引擎（多 Agent DAG + 异构模型）
- **新增 M4（4 周）**：AST-Native 代码认知引擎（Tree-sitter + LanceDB）
- **新增 M5.1（1 周）**：编译期验证护栏（五级验证管道）
- **M5/M6 重新定义**：工作流+沙箱+发布 + 全链路整合 + Beta
- **总周期**：12 周 → 20 周（+8 周，三大壁垒值得）

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
| D4 | ROADMAP.md 详细路线图 | `docs/ROADMAP.md` | 本文 | ✅ |
| D4 | CONTRIBUTING.md 贡献指南 | `docs/CONTRIBUTING.md` | PR 流程规范 | ⏳ |
| D5 | 工具链准备：rustc + pnpm + tauri-cli + docker | 命令可用 | `cargo --version` 等 | ✅ |
| D5 | 双 workspace 根：package.json + Cargo.toml | 4.AgentWork/ 根 | `pnpm install` 通 | ✅ |

**M0 验收**：4.AgentWork/ 仓库具备完整文档体系 + 工具链就绪 + 工程脚手架 ✅

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

## 9. W7 M3.1 团队模板 + 模型路由（2026-08-03 ~ 08-09）

> **目标**：定义团队角色 + 异构模型绑定 + 模型路由
> **设计参考**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) § 3.1, 3.2

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `agent-team` crate 初始化 | `platformkit/crates/agent-team/` | 编译通过 |
| D2 | 团队模板定义（YAML 解析 + 角色结构） | `src/team_template.rs` | `.yunji/team.yml` 可加载 |
| D3 | 模型路由层（主模型 + 备用模型 + 故障转移） | `src/model_router.rs` | 路由可用 |
| D4 | Agent 工作池（每个角色一个 Worker） | `src/agent_worker.rs` | 4 角色 Worker |
| D5 | 桌面端团队配置 UI | `apps/desktop/src/views/Team.tsx` | 可视化配置 |

**M3.1 验收**：用户能配置团队（PM/后端/前端/测试）+ 绑定异构模型

---

## 10. W8 M3.2 DAG 调度引擎（2026-08-10 ~ 08-16）

> **目标**：任务依赖图 + 并行调度 + 状态追踪
> **设计参考**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) § 3.3

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | TaskDag 数据结构 + 依赖解析 | `src/dag.rs` | DAG 解析正确 |
| D2 | DagScheduler 调度循环（找 ready 任务） | `src/scheduler.rs` | 调度逻辑通 |
| D3 | 任务状态机（Pending/Running/Verifying/Merged/Failed） | `src/task_state.rs` | 状态流转正确 |
| D4 | 共享上下文池（Git 仓库作为上下文总线） | `src/context.rs` | 上下文读写 |
| D5 | 调度引擎单测 + 集成测试 | `tests/` | 10+ 用例通过 |

**M3.2 验收**：DAG 调度引擎能解析任务依赖 + 并行分配 + 状态追踪

---

## 11. W9 M3.3 双 Agent MVP（2026-08-17 ~ 08-23）

> **目标**：架构师 + Coder 双 Agent 协作跑通
> **设计参考**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) § 9

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | Orchestrator Agent（Qwen3.7）任务拆解 | `src/orchestrator.rs` | 能输出 JSON DAG |
| D2 | Coder Agent（GLM5.2）代码生成 | `src/coder.rs` | 能在分支写代码 |
| D3 | 双 Agent 协作 E2E（"添加健康检查接口"） | `tests/e2e/dual_agent.rs` | 任务跑通 |
| D4 | 桌面端任务看板 UI（DAG 可视化） | `apps/desktop/src/views/TaskBoard.tsx` | 看板可用 |
| D5 | 错误处理 + 重试机制 | `src/retry.rs` | 失败自动重试 |

**M3.3 验收**：用户输入需求 → Orchestrator 拆分 → Coder 执行 → 任务完成

---

## 12. W10 M3.4 Git 分支隔离 + 冲突预检（2026-08-24 ~ 08-30）

> **目标**：每个 Agent 独立分支 + 冲突预检 + Merge Resolver
> **设计参考**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) § 3.5

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | TimeFlow 分支隔离集成（task/{id} 分支） | `agent-team` 集成 | 独立分支工作 |
| D2 | git merge-tree 冲突预检 | `src/conflict_check.rs` | 冲突检测准确 |
| D3 | Merge Resolver Agent（LLM 解决冲突） | `src/merge_resolver.rs` | 冲突自动解决 |
| D4 | 多 Agent 并行 E2E（3+ Agent 同时工作） | `tests/e2e/multi_agent.rs` | 并行无冲突 |
| D5 | M3 Demo 录制 + 团队对齐 | `docs/demo/m3-demo.mp4` | Demo 通过 |

**M3 验收**：多 Agent 在独立分支并行工作 + 冲突自动解决 + 合并到 main

---

## 13. W11 M4.1 Tree-sitter 集成（2026-08-31 ~ 09-06）

> **目标**：多语言 AST 解析器
> **设计参考**：[AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) § 3.1

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `ast-native` crate 初始化 + Tree-sitter 依赖 | `platformkit/crates/ast-native/` | 编译通过 |
| D2 | 多语言解析器（Rust/TS/Python/Go/Java） | `src/parser.rs` | 5 语言解析 |
| D3 | 增量解析（文件保存后 ≤200ms） | `src/incremental.rs` | 增量更新通 |
| D4 | 符号提取（类/方法/函数/变量） | `src/symbols.rs` | 符号提取准确 |
| D5 | 解析器单测 + 性能基准 | `tests/` | 5 语言覆盖 |

**M4.1 验收**：Tree-sitter 能解析 5 种语言 + 增量更新 ≤200ms

---

## 14. W12 M4.2 LanceDB 代码知识图谱（2026-09-07 ~ 09-13）

> **目标**：代码知识图谱构建（类-方法-调用链）
> **设计参考**：[AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) § 3.2

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | LanceDB 集成 + 节点/边表结构 | `src/graph.rs` | DB 初始化 |
| D2 | 代码节点提取（CodeNode） | `src/graph.rs` | 节点入库 |
| D3 | 代码边提取（Calls/CalledBy/Implements） | `src/graph.rs` | 边入库 |
| D4 | 增量更新（文件保存触发图谱更新） | `src/graph.rs` | 增量更新通 |
| D5 | 图谱查询 API（get_callers/get_callees） | `src/graph.rs` | 查询 ≤50ms |

**M4.2 验收**：代码知识图谱能构建 + 增量更新 + 调用链查询

---

## 15. W13 M4.3 智能上下文提取（2026-09-14 ~ 09-20）

> **目标**：为 Agent 提取任务相关代码，Token 降低 70%
> **设计参考**：[AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) § 3.3

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | ContextExtractor 实现 | `src/context_extractor.rs` | 提取逻辑通 |
| D2 | 语义搜索（embedding + 向量查询） | `src/search.rs` | Top-10 搜索 |
| D3 | 调用链扩展（上下游符号） | `src/context_extractor.rs` | 上下文完整 |
| D4 | Token 节省基准测试（vs 全文件读取） | `benchmarks/` | 节省 ≥70% |
| D5 | 与 agent-team 整合（Agent 用 AST 上下文） | 集成 | Agent Token 降 |

**M4.3 验收**：Agent 上下文 Token 降低 70% + 相关符号召回率 ≥90%

---

## 16. W14 M4.4 调用链分析 + 契约监听（2026-09-21 ~ 09-27）

> **目标**：跨语言契约监听 + 影响范围分析
> **设计参考**：[AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) § 3.4

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 影响范围分析（改函数 → 列出受影响文件） | `src/impact.rs` | 影响分析准确 |
| D2 | Proto/OpenAPI 契约解析 | `src/contract_listener.rs` | 契约解析通 |
| D3 | 跨语言任务触发（后端改 API → 前端任务） | `src/contract_listener.rs` | 自动触发任务 |
| D4 | 与 agent-team 整合（契约变更触发 DAG） | 集成 | 跨语言协作 |
| D5 | M4 Demo 录制 + 团队对齐 | `docs/demo/m4-demo.mp4` | Demo 通过 |

**M4 验收**：后端改 API → AST 检测 → 自动触发前端更新任务 → 验证通过

---

## 17. W15 M5.1 验证护栏管道（2026-09-28 ~ 10-04）

> **目标**：五级验证管道 + 错误反馈 + 白名单
> **设计参考**：[VERIFICATION-GUARDRAILS.md](VERIFICATION-GUARDRAILS.md) § 3

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `verification-guardrails` crate 初始化 | `platformkit/crates/verification-guardrails/` | 编译通过 |
| D2 | Stage 1: Lint 检查 + Stage 2: 类型检查 | `src/lint.rs` `src/type_check.rs` | 2 级验证 |
| D3 | Stage 3: 编译检查 + Stage 4: 契约匹配 | `src/compiler.rs` `src/contract.rs` | 4 级验证 |
| D4 | Stage 5: 调用链完整性 + 错误反馈器 | `src/call_chain.rs` `src/feedback.rs` | 5 级验证 |
| D5 | 白名单机制 + 与 agent-team 整合 | `src/whitelist.rs` | 误杀率 <5% |

**M5.1 验收**：AI 代码必须通过五级验证才能合并 + 失败自动重试

---

## 18. W16 M5.2 工作流引擎 + Docker 沙箱（2026-10-05 ~ 10-11）

> **目标**：触发词工作流 + Docker 沙箱隔离
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 5

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `timeflow-workflow` crate + 触发器系统 | `platformkit/crates/timeflow-workflow/` | 3 种触发器 |
| D2 | YAML 工作流定义解析 + DAG 执行 | `src/engine.rs` | `.yunji/workflows.yml` |
| D3 | `sandbox-image/Dockerfile` + aw-sandbox | `sandbox-image/` `aw-sandbox/` | 沙箱启动 |
| D4 | Agent 在沙箱内执行 E2E | `tests/e2e/sandbox.rs` | 沙箱隔离 |
| D5 | 桌面端工作流管理 UI | `apps/desktop/src/views/Workflows.tsx` | 可视化配置 |

**M5.2 验收**：触发词触发工作流 + Agent 在 Docker 沙箱内执行

---

## 19. W17 M5.3 多平台发布（2026-10-12 ~ 10-18）

> **目标**：多目标构建 + 多平台分发 + Release Notes
> **设计参考**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) § 6

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | `timeflow-release` crate + 构建矩阵 | `platformkit/crates/timeflow-release/` | 4 目标构建 |
| D2 | GitHub Release + Gitee 适配器 | `src/github.rs` `src/gitee.rs` | 上传成功 |
| D3 | 网盘适配器（蓝奏云/阿里云盘） | `src/netdisk.rs` | 上传成功 |
| D4 | AI Release Notes 生成 | `src/release_notes.rs` | Markdown 输出 |
| D5 | `/release` 全流程 E2E | `tests/e2e/release.rs` | 一句话发布 |

**M5 验收**：输入 `/release` → 构建多平台 → 分发多平台 → 生成 Release Notes

---

## 20. W18 M6.1 全链路整合（2026-10-19 ~ 10-25）

> **目标**：五大支柱串联（TimeFlow + AI 团队 + AST + 验证护栏 + 发布）

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 五大模块整合 + 端到端流程打通 | 集成 | 全链路通 |
| D2 | 性能优化（启动 <3s，快照 <500ms，发布 <5min） | benchmarks | 达标 |
| D3 | Bug triage + 修复 | issues | 严重 bug = 0 |
| D4 | 用户文档（5 篇：版本控制/团队协作/AST/护栏/发布） | `docs/user/*.md` | 文档齐 |
| D5 | 全链路 Demo 录制 | `docs/demo/full-pipeline.mp4` | Demo 通过 |

**M6.1 验收**：用户输入需求 → AI 团队协作 → AST 优化 → 验证护栏 → 自动发布

---

## 21. W19 M6.2 内部 Beta（2026-10-26 ~ 11-01）

> **目标**：100 个内部用户跑通全链路

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 100 个内部用户招募 + onboarding | Discord/Slack 群 | 100 用户活跃 |
| D2 | 用户反馈收集 + Bug 分类 | issues | 反馈整理 |
| D3 | 关键 Bug 修复 + 性能调优 | 代码 | P0 全解决 |
| D4 | Beta 周报 + 改进计划 | `docs/internal/beta-report.md` | 周报发 |
| D5 | M6.2 验收 + M6.3 计划 | 计划文档 | 计划发布 |

**M6.2 验收**：100 内部用户至少 50 人跑通"需求→AI 团队→发布"全流程

---

## 22. W20 M6.3 公开 Beta（2026-11-02 ~ 11-08）

> **目标**：1000 个外部用户

| Day | 任务 | 产出 | 验收 |
|:---:|---|---|---|
| D1 | 营销：Twitter/HN/ProductHunt 发布 | 推文 + Landing page | 发布 |
| D2 | 公开渠道：Discord/GitHub Discussions | 社区 | 1000 注册 |
| D3 | 文档站（VitePress） | `docs.yunji.ai/aw/` | 上线 |
| D4 | 反馈分类 + 关键问题修复 | issues | P0 全部解决 |
| D5 | M6 报告 + GA 计划 | `docs/internal/m6-report.md` | GA 计划 |

**M6 验收**：1000 外部用户，至少 200 人跑通任务

---

## 23. M7-M10 长期规划（2026 Q4 - 2027 Q1）

> M7+ 不在 20 周硬计划内，按季度推进

| Milestone | 时间 | 目标 |
|---|---|---|
| **M7** | 2026-11 | GA 1.0：付费用户 100+，Pro/Team 档位上线 |
| **M8** | 2026-12 | Enterprise 档位：私有化部署 + SLA |
| **M9** | 2027-01 | Skills 市场公测：第三方 Skills 发布 |
| **M10** | 2027-02 | Temporal 替换内存 DAG（生产级调度） |
| **M11** | 2027-03 | 自定义 LLM 接入（私有模型） |
| **M12** | 2027-04 | 移动端 PoC（iOS/Android） |

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
| 2026-06-17 | **v3.0 重大升级**：12 周 → 20 周，融入三大壁垒级能力（AI 团队协作 + AST-Native + 验证护栏），产品定位升级为"AI 研发团队协作平台" | Trae |

---

> **本文档是执行宪法**。每周回顾时更新（任务完成、风险触发、应急启动）。
