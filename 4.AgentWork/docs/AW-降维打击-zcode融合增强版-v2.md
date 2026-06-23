# AW 降维打击 · zcode 融合增强版计划书 v2.0

> **版本**：v2.0（基于 v1.0《AW-降维打击Hermes-全民AI计划书》深化）
> **日期**：2026-06-22
> **作者**：ZCode + Trae 联合制定
> **目标**：在 v1.0"全民 AI"基础上，吸收 zcode 自身工程化优势 + OpenHands / Claude Code / OpenClaw 等开源 Agent 的成熟范式，形成对 Hermes 的差异化降维打击
> **状态**：已分解到 M6.1-M6.4 任务级，骨架代码已落地
> **配套**：
> - [AW-降维打击Hermes-全民AI计划书.md](AW-降维打击Hermes-全民AI计划书.md) — v1.0 原始战略
> - [AW-差异化竞争分析-v2.md](AW-差异化竞争分析-v2.md) — 竞品矩阵
> - [M6.1-自进化基础-任务分解.md](M6.1-自进化基础-任务分解.md) — W1-W4 任务清单
> - [M6.2-全民可用-任务分解.md](M6.2-全民可用-任务分解.md) — W5-W9 任务清单
> - [M6.3-M6.4-行业生态与降维打击-任务分解.md](M6.3-M6.4-行业生态与降维打击-任务分解.md) — W10-W18 任务清单

---

## 0. v2.0 相比 v1.0 的三大升级

### 升级一：从"功能降维"升级为"工程降维"

v1.0 提出的 5 大革新（用户可选模式 / 进化仪表盘 / 行业模板市场 / 无代码工具构建器 / 团队知识共享）是**产品维度**的降维打击。

v2.0 在此基础上追加**工程维度**的降维打击——把 zcode 自身在工程化上的 7 大优势内化为 AW 的"出厂能力"，让 AW 不只是"用户友好"，更是"开发者无脑上手"：

| zcode 优势 | 在 AW 中的内化 | Hermes 缺口 |
|---|---|---|
| **Skill 系统**（Markdown 驱动 + 渐进发现） | 升级 `skill_market.rs` → `skills_v2.rs`，技能即 Markdown 文件 | Hermes Skills 强 schema，普通用户难写 |
| **子代理并行**（Agent 工具 + Explore agent） | DAG 调度器天然支持并行子任务，无需用户配置 | Hermes 子代理串行 |
| **计划模式**（Plan Mode + ExitPlanMode 审批） | 对应 v1.0 "审批模式"的工程化落地 | Hermes 无显式计划/执行分离 |
| **TodoWrite 任务追踪** | 每个 AI 员工自带可可视化任务清单 | Hermes 无 |
| **工作树并行**（git worktree） | TimeFlow 分支隔离的工程化基础 | Hermes 单仓 |
| **验证护栏**（verification-before-completion） | 已有 `verification-guardrails` crate 五级验证 | Hermes 仅运行时审批 |
| **TDD / 系统化调试 / 收发代码评审** Skills | 作为"员工培训包"出厂，每个员工自带工程方法论 | Hermes 用户需自学 |

### 升级二：吸收开源 Agent 的成熟范式

调研 OpenHands、Claude Code、OpenClaw、Aider、Continue 等开源 Agent 后，提取 5 个范式融入 v2.0：

| 开源范式 | 来源 | 在 AW 中的落地 |
|---|---|---|
| **事件流架构**（EventStream） | OpenHands | `stream.rs` 已有 StreamEvent，扩展为完整事件总线，支持回放 |
| **MCP 客户端兼容** | Claude Code | v1.0 W15 `mcp_client.rs`，**v2.0 提前到第 1 期就做协议层**，让生态接入从"D4 远期"变成"D1 出厂" |
| **可观测 trace** | LangSmith / Langfuse | `sessions.rs` 每条会话记录 token / 耗时 / 工具调用链，导出为 OpenTelemetry |
| **Diff-first 修改** | Aider | `coder.rs` 改为先生成 diff、再 apply，配合五级验证护栏 |
| **Skills 即文档** | zcode / Claude Code | 技能 = Markdown（front-matter + 步骤），git 友好，用户可读可改 |

### 升级三：差异化矩阵——AW / Hermes / OpenHands / Claude Code

详见 [AW-差异化竞争分析-v2.md](AW-差异化竞争分析-v2.md)。一句话总结：

> **Hermes = 工程师的 Agent；OpenHands = 研究员的 Agent；Claude Code = 单兵作战的 Agent；AW = 全民 + 团队 + 工程 + 生态 的 Agent。**

---

## 1. 6 维降维打击（继承 v1.0，新增"工程化"维度）

| # | 维度 | Hermes | OpenHands | Claude Code | **AW v2.0** |
|---|------|--------|-----------|-------------|------------|
| 1 | 用户群体 | 开发者 | 研究员 | 开发者 | **全民（对话式 + 工程化双形态）** |
| 2 | 进化机制 | 被动沉淀 | 手动配置 | 项目记忆 | **主动进化 + 可视化 + 可交易 + 可观测** |
| 3 | 隔离模型 | Profile 目录 | 单会话 | 单仓 | **动态角色 + 行业模板 + 团队共享 + worktree 并行** |
| 4 | 工具生态 | MCP | 内置 + MCP | MCP + Skills | **MCP（出厂兼容）+ 无代码构建器 + Skill 市场** |
| 5 | 入口场景 | CLI + 消息 | Web/CLI | CLI | **桌面 + Web + 移动 + API + Webhook + 消息** |
| 6 | 商业模式 | 开源 | 开源 + 云 | 商业 | **免费 + Pro + Enterprise + 市场分润** |
| **7** 🆕 | **工程化** | 手工 | 实验性 | 单兵 | **TDD + 系统化调试 + 代码评审 + 验证护栏** |

---

## 2. 7 大革新（v1.0 的 5 个 + v2.0 的 2 个工程化革新）

### 革新 1-5：继承 v1.0（详见 v1.0 计划书 §2.1-2.5）

1. **用户可选模式**（学习/教学/协作/自动/审批）
2. **进化仪表盘**（可视化 + 可导出 + 可交易 + 可回滚）
3. **行业模板市场 + 无代码自定义**
4. **全民工具构建器**
5. **团队知识共享**（3 层覆盖 + Git 同步）

### 革新 6 🆕：工程化出厂包（Engineering First-Class）

**核心洞察**：Hermes / OpenHands 都假设用户"懂工程"。AW 把"懂工程"这件事**打包成员工的内置能力**。

**3 个子能力**：

1. **每个员工自带工程方法论 Skill**：
   - 客服员工 → 自带"投诉处理流程"Skill（步骤化）
   - 财务员工 → 自带"对账五步法"Skill
   - 开发员工 → 自带"TDD + 系统化调试 + 代码评审"Skill（zcode 同款）

2. **计划/执行分离**（zcode Plan Mode 范式）：
   - "审批模式"= Plan Mode，员工先出计划，用户 ExitPlanMode 才执行
   - "自动模式"= 一步到位
   - 同一员工同一任务可在两种模式间切换

3. **可观测 trace**（LangSmith 范式）：
   - 每次会话产出 trace.json：token、耗时、工具调用链、子任务 DAG
   - 进化仪表盘里可点开任意会话看完整回放
   - 导出 OpenTelemetry，对接 Grafana / Jaeger

### 革新 7 🆕：生态兼容优先级前置

**核心洞察**：v1.0 把 MCP 放在 W15（第 4 期），太晚。v2.0 把"MCP 协议层"提到第 1 期就做接口骨架，让 AW **从第一周就具备接入 Hermes / Claude Code / OpenHands 生态的能力**。

- W1-W2：定义 `ToolProvider` trait（HTTP / Database / File / MCP / Agent-to-Agent / Command 统一抽象）
- W3-W4：实现 MCP stdio client（最低成本兼容）
- W5+：无代码工具构建器基于 `ToolProvider` 构建，**自动支持所有 6 种数据源**

---

## 3. 实施计划：4 期 18 周（v2.0 节奏调整）

### 3.1 总览（相比 v1.0 的调整）

```
=== 第 1 期：自进化基础（W1-W4，4 周）===
v1.0：memory / sessions / skills_v2 / prompt_builder
v2.0 新增：tool_provider trait（统一工具抽象）+ mcp_client 协议层骨架
→ 让 AW 从第 1 期就具备生态接入能力

=== 第 2 期：全民可用（W5-W9，5 周）===
v1.0：work_mode / evolution_dashboard / no_code_tool_builder
v2.0 新增：trace 可观测 + 工程化 Skill 出厂包
→ 让 AI 员工"开箱即懂工程"

=== 第 3 期：行业生态（W10-W14，5 周）===
v1.0：team_knowledge / template_marketplace / profile_manager
v2.0 强化：与已有 template_market.rs / template_loader.rs / sync.rs 整合（避免重复造轮子）
→ 复用 M4.2 已完成的行业模板基础设施

=== 第 4 期：降维打击（W15-W18，4 周）===
v1.0：mcp_client（已前移）/ multi_entry / sandbox_v2 / security
v2.0 调整：mcp_client 前移后，本期重点改为 multi_entry + security 打磨 + Beta
```

### 3.2 详细的任务级分解

详见三份配套文档：

- [M6.1-自进化基础-任务分解.md](M6.1-自进化基础-任务分解.md)（W1-W4，4 模块 × 5 天 = 20 任务）
- [M6.2-全民可用-任务分解.md](M6.2-全民可用-任务分解.md)（W5-W9，3 模块 × 5 天 = 25 任务）
- [M6.3-M6.4-行业生态与降维打击-任务分解.md](M6.3-M6.4-行业生态与降维打击-任务分解.md)（W10-W18）

---

## 4. v2.0 关键设计决策

### 决策 1：不复用现有 `skill_market.rs`，新建 `skills_v2.rs`

**理由**：
- 现有 `skill_market.rs`（1359 行）是 M5.2 精简版，强 schema，2 个内置 Skill
- v1.0 要求 Markdown 驱动 + 自进化（success_count / failure_count）+ 渐进发现
- 强行改造会破坏 M5.2 已稳定的 API
- **方案**：新建 `skills_v2.rs`，保留 `skill_market.rs` 作为兼容层，v2.0 用户用 V2，老用户用 V1

### 决策 2：Memory 不依赖 SQLite，纯 JSON

**理由**：
- v1.0 要求"可 git 跟踪"
- SQLite 二进制文件 git diff 无意义
- Memory 数据量小（千条级），JSON 足够
- Sessions 才用 SQLite + FTS5（量大、需检索）

### 决策 3：PromptBuilder 与现有 `context.rs` 共存

**理由**：
- `context.rs`（740 行）是 DAG 调度期的"共享上下文池"，按角色过滤
- `prompt_builder.rs` 是"分层 prompt 引擎"，按优先级裁剪 + Token 预算
- 两者职责不同：context 管"有哪些内容"，prompt_builder 管"如何装配进 prompt"
- **方案**：PromptBuilder 内部调用 context.rs 的 read_for_role，不替代

### 决策 4：MCP 协议层独立成 `tool_provider.rs`

**理由**：
- 无代码工具构建器、MCP client、Agent-to-Agent 都需要统一的工具抽象
- 抽出 `ToolProvider` trait，6 种数据源各自实现
- MCP 只是 6 种之一，不特殊对待

---

## 5. 与 zcode 自身能力的双向赋能

### 5.1 zcode → AW（zcode 优势内化为 AW 出厂能力）

| zcode 能力 | AW 落地 | 完成里程碑 |
|---|---|---|
| Skill 工具（Markdown SKILL.md） | `skills_v2.rs` 的 Skill 格式 | M6.1 W3 |
| Agent 工具（子代理派发） | DAG 调度器 + Agent-to-Agent 工具 | M6.1 W4 |
| EnterPlanMode / ExitPlanMode | 审批模式的工程化 | M6.2 W5 |
| TodoWrite | 每个员工的可视化任务清单 | M6.2 W6 |
| using-git-worktrees | TimeFlow 分支隔离基础 | M6.3 W11 |
| verification-before-completion | 五级验证护栏（已有） | 复用 M5.1 |
| systematic-debugging / TDD / 代码评审 Skill | 内置工程方法论 Skill 包 | M6.2 W7 |

### 5.2 AW → zcode（AW 反哺 zcode）

AW 在 M6 阶段沉淀的以下能力，可作为 zcode 的 Skill 反向输出：

- **行业 Skill 包**（电商/教育/金融等）→ 作为 zcode Skill 分发
- **无代码工具构建器**→ 让 zcode 用户也能图形化建工具
- **进化仪表盘**→ zcode 用户的"AI 成长档案"

---

## 6. 验收标准（v2.0 新增工程化指标）

继承 v1.0 §7 的全部指标，新增：

### 6.1 工程化指标

| 指标 | 目标 | 测量方法 |
|---|---|---|
| 4 大核心模块单测覆盖 | ≥ 70% | `cargo tarpaulin` |
| `cargo check` 通过 | 100% | CI |
| `cargo clippy` 无 warning | 100% | CI |
| MCP echo server 互通测试 | 通过 | 集成测试 |
| Trace 导出 OpenTelemetry 格式 | 通过 | 单测 |

### 6.2 差异化指标（对标 Hermes）

| 指标 | Hermes | AW 目标 |
|---|---|---|
| 非技术用户创建员工耗时 | N/A（需写 YAML） | ≤ 5 分钟（无代码向导） |
| 进化报告可读性 | 黑盒 | 全可视化 + 可导出 |
| 工具生态接入门槛 | 写 Rust 代码 | 填表单 |
| 团队知识共享 | 不支持 | Git 三层覆盖 |

---

## 7. 风险与对策（v2.0 新增）

继承 v1.0 §6 的全部风险，新增：

| 风险 | 概率 | 影响 | 对策 |
|---|---|---|---|
| zcode 优势内化工作量被低估 | 中 | 中 | 第 1 期只做接口骨架，实现延后 |
| MCP 协议前移导致第 1 期膨胀 | 中 | 中 | 第 1 期只做 stdio + echo 测试，HTTP 后置 |
| skills_v2 与 skill_market 双轨混乱 | 中 | 低 | 文档明确："新功能用 V2，老功能不破坏" |
| trace 性能开销 | 低 | 中 | 异步落盘，采样率可配 |

---

## 8. 下一步（v2.0 已启动）

✅ **已完成**（本计划书 v2.0 发布即完成）：

- [x] 分析 v1.0 计划书
- [x] 调研现有 21 个 Rust 模块实现状况
- [x] 撰写 v2.0 战略（本文档）
- [x] 产出差异化竞争分析报告
- [x] 产出 M6.1 / M6.2 / M6.3-M6.4 三份任务分解
- [x] 为 4 大核心模块创建 Rust 接口骨架（memory.rs / sessions.rs / skills_v2.rs / prompt_builder.rs + work_mode.rs / tool_provider.rs）
- [x] 接入 lib.rs，确保 `cargo check` 通过
- [x] 更新 ROADMAP.md / IMPROVEMENT-PLAN.md

🔜 **W1 启动条件**：开发者读完本文档 + 三份任务分解，即可开始 M6.1 W1 D1（memory.rs 接口细化）。

---

> **最后**：v1.0 让 AW "面向全民"；v2.0 让 AW "全民 + 工程化 + 生态兼容"。
> 三者叠加，AW 不是追上 Hermes，而是让 **Hermes / OpenHands / Claude Code 的能力都成为 AW 的子集**——它们的工具能被 AW 无代码接入，它们的方法论能被 AW 的员工 Skill 化吸收，它们的用户能被 AW 的全民体验转化。
>
> **降维打击的本质，不是功能多，而是让对手的优势成为你的输入。**
