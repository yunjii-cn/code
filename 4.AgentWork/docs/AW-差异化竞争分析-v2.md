# AW 差异化竞争分析 v2.0

> **版本**：v2.0
> **日期**：2026-06-22
> **作者**：ZCode + Trae
> **目的**：通过横向对比 Hermes / OpenHands / Claude Code / OpenClaw / Aider，定位 AW 的差异化优势
> **配套**：[AW-降维打击-zcode融合增强版-v2.md](AW-降维打击-zcode融合增强版-v2.md)

---

## 0. 一句话定位

| 产品 | 一句话 |
|---|---|
| **Hermes** | "工程师的 Agent"——YAML/MD 驱动，自进化，单机 |
| **OpenHands** | "研究员的 Agent"——事件流架构，可复现实验，Web/CLI |
| **Claude Code** | "单兵的 Agent"——CLI，MCP + Skills，强单点能力 |
| **OpenClaw** | "极客的 Agent"——开源 Claude Code 复刻，强调可定制 |
| **Aider** | "Pair 编程 Agent"——终端 git 友好，diff-first |
| **AW** | **"全民 + 团队 + 工程 + 生态 的 Agent"**——多入口、多模式、多员工、多生态 |

---

## 1. 7 维度对比矩阵

### 1.1 用户群体与上手门槛

| 维度 | Hermes | OpenHands | Claude Code | OpenClaw | Aider | **AW** |
|---|---|---|---|---|---|---|
| 目标用户 | 开发者 | 研究员 | 开发者 | 极客 | 开发者 | **全民（含开发者）** |
| 创建 Agent 方式 | 写 YAML/MD | 写 Python | 写 Skill MD | 写 Skill MD | 命令行 | **对话式 / 表单 / YAML 三选** |
| 上手耗时 | 数小时 | 半天 | 1 小时 | 1 小时 | 30 分钟 | **5 分钟（无代码向导）** |
| 行业预置 | 通用 | 通用 | 通用 | 通用 | 仅编程 | **10 核心 + N 社区** |

**AW 差异化**：唯一面向"不会写代码的人"的 Agent 平台。

### 1.2 自进化与记忆

| 维度 | Hermes | OpenHands | Claude Code | Aider | **AW** |
|---|---|---|---|---|---|
| 长期事实记忆 | MEMORY.md + USER.md | 手动 | 项目级记忆 | 仅本次会话 | **memory.rs（结构化 JSON）** |
| 跨会话检索 | Sessions + FTS5 | 不强调 | 不强调 | 不支持 | **sessions.rs（SQLite + FTS5）** |
| 程序性技能 | Skills | 不强调 | Skills（MD） | 不支持 | **skills_v2.rs（MD + 自进化）** |
| 进化可视化 | ❌ 黑盒 | ❌ | ❌ | ❌ | **✅ 进化仪表盘** |
| 进化可导出 | ❌ | ❌ | ❌ | ❌ | **✅ .aw-evolution 包** |
| 进化可交易 | ❌ | ❌ | ❌ | ❌ | **✅ 市场分润** |

**AW 差异化**：唯一让进化"可见、可控、可交易"的 Agent。

### 1.3 隔离与协作

| 维度 | Hermes | OpenHands | Claude Code | **AW** |
|---|---|---|---|---|
| 隔离模型 | Profile 目录 | 单会话 | 单仓 | **Profile + 团队 + worktree** |
| 多 Agent | 子代理（动态） | AgentSkill | 单 Agent | **DAG 调度 + 子代理 + 员工队伍** |
| 并行执行 | ❌ 串行 | ✅ 事件流 | ❌ | **✅ DAG + worktree 并行** |
| 团队共享 | ❌ 单机 | ❌ | ❌ | **✅ 3 层覆盖 + Git 同步** |

**AW 差异化**：唯一支持"团队级 AI 员工队伍"的 Agent。

### 1.4 工具生态

| 维度 | Hermes | OpenHands | Claude Code | Aider | **AW** |
|---|---|---|---|---|---|
| 内置工具 | ✅ | ✅ | ✅ | ✅（仅编程）| **✅（编程 + 客服 + 通用）** |
| MCP 兼容 | ✅ | 部分 | ✅ | ❌ | **✅（出厂）** |
| 自定义工具 | 写 Rust | 写 Python | 写 Skill MD | 不支持 | **无代码构建器** |
| 工具市场 | ❌ | ❌ | ❌（社区分享）| ❌ | **✅ Skill 市场** |

**AW 差异化**：唯一让"用户填表单"就能建工具的 Agent。

### 1.5 入口场景

| 维度 | Hermes | OpenHands | Claude Code | Aider | **AW** |
|---|---|---|---|---|---|
| CLI | ✅ | ✅ | ✅ | ✅ | ✅ |
| Web | 部分 | ✅ | ❌ | ❌ | ✅ |
| 桌面 | ❌ | ❌ | ❌ | ❌ | **✅（Tauri）** |
| 移动 | ❌ | ❌ | ❌ | ❌ | ✅（远期） |
| API | ❌ | 部分 | ❌ | ❌ | **✅** |
| Webhook | 部分 | ❌ | ❌ | ❌ | **✅** |
| 消息平台 | ✅ | ❌ | ❌ | ❌ | ✅（MCP） |
| Cron | ❌ | ❌ | ❌ | ❌ | **✅** |

**AW 差异化**：入口最全，唯一覆盖"桌面 + Web + API + Webhook + Cron"全场景。

### 1.6 商业模式

| 维度 | Hermes | OpenHands | Claude Code | Aider | **AW** |
|---|---|---|---|---|---|
| 开源免费 | ✅ | ✅ | ❌（商业）| ✅ | ✅（Free）|
| 订阅 | ❌ | 云版 | ✅ | ❌ | ✅（Pro ¥9.9/月）|
| 企业版 | ❌ | ❌ | ✅ | ❌ | ✅（Enterprise ¥99/月）|
| 市场分润 | ❌ | ❌ | ❌ | ❌ | **✅（70% 创作者）** |

**AW 差异化**：唯一具备"创作者经济"的 Agent 平台。

### 1.7 工程化（v2.0 新增维度）

| 维度 | Hermes | OpenHands | Claude Code | Aider | **AW** |
|---|---|---|---|---|---|
| TDD 方法论 | 用户自备 | 用户自备 | 用户自备 | ❌ | **✅ 内置 Skill** |
| 系统化调试 | 用户自备 | 用户自备 | 用户自备 | ❌ | **✅ 内置 Skill** |
| 代码评审 | 用户自备 | 用户自备 | 用户自备 | ❌ | **✅ 内置 Skill** |
| 验证护栏 | ❌ | ❌ | ❌ | ❌ | **✅ 五级管道** |
| 可观测 trace | 部分 | 部分 | 部分 | ❌ | **✅ OpenTelemetry** |
| 计划/执行分离 | ❌ | ❌ | ✅（Plan Mode）| ❌ | **✅（审批模式）** |

**AW 差异化**：唯一把"工程方法论"打包成员工出厂能力的 Agent。

---

## 2. zcode 融合点矩阵

| zcode 能力 | 来源 Skill | AW 落地模块 | 落地里程碑 |
|---|---|---|---|
| Markdown 驱动 Skill | `skill-creator` / `writing-skills` | `skills_v2.rs` | M6.1 W3 |
| 子代理派发 | `dispatching-parallel-agents` / `subagent-driven-development` | DAG 调度器 + Agent-to-Agent 工具 | M6.1 W4 |
| 计划模式审批 | EnterPlanMode / ExitPlanMode | `work_mode.rs` 审批模式 | M6.2 W5 |
| 任务清单追踪 | TodoWrite | 每个员工的可视化任务 | M6.2 W6 |
| 工作树并行 | `using-git-worktrees` | TimeFlow 分支隔离 | M6.3 W11 |
| 验证护栏 | `verification-before-completion` | 五级验证管道（已有）| 复用 M5.1 |
| TDD | `test-driven-development` | 内置 Skill 包 | M6.2 W7 |
| 系统化调试 | `systematic-debugging` | 内置 Skill 包 | M6.2 W7 |
| 代码评审 | `requesting-code-review` / `receiving-code-review` | 内置 Skill 包 | M6.2 W7 |
| 头脑风暴 | `brainstorming` | 内置 Skill 包（可选） | M6.2 W8 |
| 多 Agent 协作 | `team-collab` / `multi-agent-dev-team` | collaboration.rs 已有，扩展 | M6.3 W10 |
| 编写计划 | `writing-plans` / `executing-plans` | 内置 Skill 包 | M6.2 W7 |

---

## 3. AW 的"独占象限"分析

用"用户门槛"和"协作能力"两个维度做四象限：

```
高协作
  │
  │   ┌─────────────────┐
  │   │  AW（独占）       │   ← 唯一"高协作 + 低门槛"
  │   │  全民 + 团队      │
  │   └─────────────────┘
  │
  │   Hermes / OpenHands
  │   高协作需高门槛
  │
  ├─────────────────────────── 用户门槛
  │   Aider
  │   低门槛但单兵
  │
  │   Claude Code / OpenClaw
  │   单兵 + 中门槛
  │
低协作
```

**AW 独占象限**：**高协作 + 低门槛**——这是 Hermes / OpenHands / Claude Code 都无法进入的蓝海。

---

## 4. 差异化降维打击的 3 个核心论断

### 论断 1：Hermes 的优势是 AW 的输入，不是对手

> Hermes 的 MCP 工具、自进化 Skills、Profile 隔离——这些都能被 AW 的 MCP 客户端 + skills_v2 + profile_manager 无代码接入。
> **AW 不和 Hermes 竞争功能，AW 把 Hermes 当生态源。**

### 论断 2：Claude Code 的能力是 AW 员工的一个 Skill

> Claude Code 的强单点能力（plan mode / skills / MCP）→ 在 AW 里就是一个"开发员工"员工的内置 Skill。
> **AW 不和 Claude Code 比"谁更强"，AW 让 Claude Code 的能力成为 AW 团队的一员。**

### 论断 3：OpenHands 的可复现实验，AW 用 trace + 进化包超越

> OpenHands 用事件流做可复现——但只能在研究员圈传播。
> AW 的 trace（OpenTelemetry）+ 进化包（.aw-evolution）让普通用户也能"看见 + 回放 + 交易" AI 的成长。
> **AW 把"可复现"从研究员特权变成全民能力。**

---

## 5. SWOT 小结

### Strengths（AW 独有）

- 唯一面向全民的 Agent 平台
- 唯一具备团队知识共享的 Agent
- 唯一具备创作者经济（市场分润）的 Agent
- 唯一把工程方法论出厂化的 Agent
- 4 代产品线（1.PC / 2.WEB / 3.dev / 4.AgentWork）共享技术沉淀

### Weaknesses

- 18 周工期紧（v2.0 通过骨架先行缓解）
- 行业模板冷启动（v2.0 通过 zcode Skills 反向填充缓解）
- MCP 生态依赖 Hermes / Claude Code 上游（v2.0 通过 ToolProvider 抽象降低耦合）

### Opportunities

- 国产 AI Agent 市场空白（Hermes / OpenHands 都是英文社区）
- 中小企业 AI 员工需求爆发
- 创作者经济在 AI 领域尚无平台

### Threats

- Hermes 大版本破坏 MCP 协议
- 大厂（字节/阿里）突然下场
- zcode 自身能力迭代节奏

---

## 6. 结论

> **AW 的差异化不是"功能多"，而是"维度不同"。**
>
> Hermes / OpenHands / Claude Code 都在"开发者 × 单机 × 工具"这个平面里竞争；
> AW 跳到"全民 × 团队 × 生态"这个立体空间，用降维的方式让对手的优势成为自己的输入。
>
> 这就是 v2.0 的核心论点：**不是追上对手，而是让对手成为你的子集。**
