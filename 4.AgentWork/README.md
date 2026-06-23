# 云集智能体工作台 (AW / AgentWork)

> **AI 智能体工作台** — 任务驱动 + 互动机制双引擎 + AI 员工定义与培训 + 跨行业模板 + WordPress 式生态愿景。
>
> - **`AW`** = 简称（**A**I-**W**ork），CLI 命令、crate 名、commit scope 用此
> - **`AgentWork`** = 正式品牌名，仓库、官网、商标、对外营销统一用此
> - **中文**：`云集智能体工作台`

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blueviolet.svg)](https://tauri.app)
[![Status](https://img.shields.io/badge/status-M3.3%20%E5%B7%B2%E5%AE%8C%E6%88%90-yellow.svg)]()

---

## ✨ 核心定位

> **AgentWork 不是"AI 编程工具"，而是"AI 智能体工作台"**。
>
> 一个公司能用的 AI 智能体远不止"写代码"——还有客服、销售、财务、法律、教育……
> AgentWork 让每个公司**自定义自己的 AI 员工 / AI 团队 / AI 行业模板**，
> 用任务驱动 + 互动机制双引擎，跑通"独立工作"和"团队协作"两种范式。

---

## 🎯 五大差异化支柱（v3.2 重写）

- 🤝 **AI 团队协作引擎** ⭐王炸 — 多 Agent DAG 真并行 + 异构模型差异化绑定（Qwen3.7 推理 / GLM5.2 代码 / MiniMax3 视觉）+ 邮件式异步协作
- 🧠 **AI 员工培训引擎** ⭐壁垒 — 知识库 + 规则 + 话术 + 评估四要素，纯 prompt + few-shot，**不依赖 GPU 即可行业化**
- 🛡️ **行业模板 + 自定义** ⭐B 端关键 — 5 核心细分行业（电商-穿搭/电子 + 教育-早教/素质 + 金融-证券）+ 5 候补 + 用户自定义模板向导
- 💬 **任务驱动 + 互动双引擎** — 维护阶段用 ChatPanel 自然语言互动，开发/执行阶段用 TaskBoard DAG 可视化
- 🔄 **本地优先 + 云端协同** — 4 核 8G 宝塔服务器即可跑通反代 + 同步 + 备份，**不需要 GPU 集群**
- 🧬 **全民 AI 自进化层** 🆕 — memory / sessions / skills_v2 / prompt_builder / work_mode，把 zcode 的 Skill、计划模式、子代理与验证护栏优势产品化

---

## 🎯 适用场景（v3.2 扩展：从"研发"到"5 行业"）

### 1️⃣ AI 团队协作（多 Agent 并行）
- **多 Agent DAG 真并行**：任务拆解 → 后端/前端/测试 Agent 在独立分支并行 → 自动合并
- **异构模型分工**：Qwen3.7 推理规划 / GLM5.2 写代码 / MiniMax3 处理视觉，各展所长
- **邮件式异步协作**：员工之间消息触发（不需长连接，宝塔友好）
- **任务看板**：DAG 可视化，实时查看多 Agent 工作进度

### 2️⃣ AI 员工独立工作（单 Agent 场景）⭐ v3.2 新增
- **客服员工**：售前咨询 / 售后处理 / 评价管理
- **销售员工**：线索筛选 / 客户跟进 / 报价管理
- **教育员工**：学科答疑 / 作业批改 / 学情分析
- **法律员工**：案件咨询 / 文档审核 / 法规查询
- **财务员工**：票据识别 / 账目核对 / 报表生成
- **设计员工**：海报生成 / 详情页文案 / 朋友圈种草
- **老板分身**：日程管理 / 邮件回复 / 决策辅助

### 3️⃣ 10 行业模板（5 核心 + 5 候补）⭐B 端获客主力

**总计 10 个行业**（5 核心先做 + 5 候补 Beta 后做，覆盖 60-70% 商业市场 + 千万级 C 端投资者）：

| # | 行业 | 阶段 | 目标客户 | 关键能力 |
|:-:|------|:---:|---------|---------|
| 1 | **电商-穿搭** | 🟢 核心 | 女装/男装/童装/鞋类商家 | 搭配推荐 + 退换货 + 朋友圈文案 |
| 2 | **电商-电子** | 🟢 核心 | 3C/家电/智能硬件商家 | 参数对比 + 故障排查 + 装机指导 |
| 3 | **教育-早教** | 🟢 核心 | 0-6 岁早教/托育/亲子号 | 育儿答疑 + 课程顾问 + 睡前故事 |
| 4 | **教育-素质培训** | 🟢 核心 | AI 启蒙/美术/书法/音乐机构 | 课程顾问 + AI 教师 + 作品点评 |
| 5 | **金融-证券股票** | 🟢 核心 | 券商投顾/私募/财富管理 | 行情分析 + 投顾助理 + 公告摘要 |
| 6 | **金融-建筑投资** | 🟡 候补（2027 H2）| 产业基金/基建/房地产投资 | 项目分析 + 投顾支持 + 风险评估 |
| 7 | **法律咨询** | 🟡 候补 | 律所/法务部/咨询公司 | 案件咨询 + 文档审核 + 法规查询 |
| 8 | **财务对账** | 🟡 候补 | 中小企业/代账公司 | 票据识别 + 账目核对 + 报表生成 |
| 9 | **HR 招聘助理** | 🟡 候补 | 中小企业 HR/猎头/RPO | 简历筛选 + 面试预约 + 候选人答疑 |
| 10 | **医疗健康咨询** | 🟡 候补 | 医美/口腔/中医诊所 | 预约客服 + 术前咨询 + 术后回访 |

**5 核心先做（2026-11~12，2 个月，Beta 主力）**
- 每个核心 1 周 = 12 工作日
- 每个 3 员工 + 5 文档 + 10 规则 + 30 话术 + 20 评估
- 合计 15 员工 / 25 文档 / 50 规则 / 150 话术 / 100 评估

**5 候补后做（2027 H2，Beta 后 3 个月）**
- 优先做 1-2 个：金融-建筑投资（用户点名）+ HR 招聘（付费意愿强）
- 其他 3 个视核心模板反馈决定
- 合计 15 员工 / 25 文档 / 50 规则 / 150 话术 / 100 评估

**10 行业 + 30 行业模板目标（1 年后核心 + 候补 + 社区）**
- 30 行业 = 10 内置 + 20 社区
- 覆盖 1000 万+ 商业主体
- 详见 [docs/IMPROVEMENT-PLAN.md](docs/IMPROVEMENT-PLAN.md) § M4.2

### 5️⃣ 模板自定义（v3.2 P0 用户硬需求）⭐
- 用户可基于 5 核心模板 fork 出自己的"穿搭-西装" / "电子-手机"子模板
- 用户可从空白创建完全自定义的"汽车 4S 店"模板
- **私有模板** = 公司内部用，不泄露
- **公开模板** = 一键提交社区市场，30% 收入分成

### 6️⃣ 互动场景（v3.2 新增）
- 维护阶段（修 bug / 调参数）通过 ChatPanel 自然语言互动
- 可指代上下文（"这个任务" / "这个文件"）
- 可触发小任务（聊天里说"修这个"→ 弹任务到 TaskBoard）
- 多轮对话 + 流式输出 + 打字指示器 + 可中断

---

## 🛠️ 技术栈（v3.2 更新）

| 层 | 技术 | 说明 |
|---|---|---|
| **桌面端** | Tauri 2 + React 18 + TypeScript 5.5 + shadcn/ui + Tailwind 3.4 | 跨平台桌面 |
| **后端核心** | Rust 1.88+ + Tokio + Axum | agent-team / timeflow-* / aw-* crates |
| **DAG 调度** | 自研 `agent-team::DagScheduler` | 6 态状态机 + 拓扑排序 |
| **AI 员工** | YAML 定义 + serde 序列化 | git 友好 + 可视化编辑 |
| **培训引擎** | hnsw_rs 内存索引 + bge-small-zh Embedding API | 纯 CPU，**无 GPU** |
| **规则引擎** | 关键词匹配 + LLM 二次确认 | 简单规则链，不引入 Rete |
| **话术训练** | few-shot prompt + LLM-as-Judge | 评估 6/10 准确率 |
| **LLM Gateway** | 自研反代 + 限流 + token 计数 | 宝塔 5G 内存单实例 |
| **云端协同** | 宝塔 4 核 8G / < 30G | 反代 + 同步 + 备份 |
| **任务调度** | DAG + 重试 + 错误分类（RetryPolicy）| 测试 190 个全通过 |
| **协议** | MCP（Model Context Protocol）| 工具接入生态化 |
| **数据** | 本地 SQLite（YAML 员工 / 培训 / 评估）| 简洁可审计 |

---

## 🚀 快速开始

### 环境要求

- **Rust** 1.88+ — [rustup.rs](https://rustup.rs)
- **Node.js** 20+ + **pnpm** 9+
- **Tauri 系统依赖**（macOS / Linux 见 [Tauri 官方文档](https://tauri.app/start/prerequisites/)）
- **宝塔服务器**（可选，云端协同用）— 4 核 8G / < 30G 磁盘 / 公网 IP

### 克隆 + 安装

```bash
git clone https://github.com/yunji/agentwork.git
cd agentwork
git checkout agentwork  # AgentWork 专属分支

# 安装依赖
pnpm install

# 启动桌面端（开发模式）
cd apps/desktop
pnpm tauri dev
```

### 启动脚本（Windows）

```bat
:: 根目录
启动.bat

:: 选择菜单：
::   [1] Desktop Dev
::   [2] Frontend
::   [3] Build
::   [4] Test
::   [5] Clippy
::   [6] Exit
```

### 第一个 AI 员工任务

```bash
# 在桌面端 TaskBoard 输入：
"创建一个电商客服员工，能回答尺码咨询"

# 系统自动：
# 1. 拆解为 DAG 任务（员工定义 + 知识库 + 规则 + 话术 + 评估）
# 2. 在 worktree/agent-customer-service 分支下执行
# 3. 完成后弹出 PR 预览
```

---

## 📂 项目结构（v3.2 现状）

```
4.AgentWork/
├── apps/                桌面端（v3.2 仅 desktop，Web 推迟）
│   └── desktop/         Tauri 2 桌面端
│       ├── src/         React 18 前端（views + components + lib）
│       └── src-tauri/   Rust 后端（commands + lib.rs + main.rs）
├── platformkit/         共享层（Rust crates）
│   └── crates/
│       ├── agent-team/      ⭐ AI 团队引擎（员工 + DAG + 调度 + 重试 + 模型路由）
│       ├── timeflow-core/   TimeFlow 本地 VCS 引擎（快照 + 分支 + 存储）
│       ├── timeflow-ai/     AI 语义层（embedding + 搜索 + 分类 + commit msg）
│       └── aw-git/          Git 兼容层（libgit2 + 3 模式：release/stealth/sync）
├── docs/                文档
│   ├── ROADMAP.md              ⭐ 主线 20 周路线图（v3.0）
│   ├── IMPROVEMENT-PLAN.md     ⭐ W9.5 起的完善+增强计划（v2.2）
│   ├── FUTURE-PLANS.md         🆕 未来规划参考库（沙箱/LoRA/私有化/M6 生态）
│   ├── ARCHITECTURE.md         系统架构
│   ├── AGENT-TEAM-DESIGN.md    AI 团队引擎设计
│   ├── TIMEFLOW-DESIGN.md      TimeFlow VCS 设计
│   ├── AST-NATIVE-DESIGN.md    AST 知识图谱设计
│   ├── VERIFICATION-GUARDRAILS.md  五级验证管道
│   └── CONTRIBUTING.md         贡献指南
├── AGENTS.md            项目宪法（v3.2）
├── tier.yaml            商业模式（v3.2 即将更新）
└── 启动.bat             启动菜单（v2 稳定版）
```

详见 [AGENTS.md](AGENTS.md) § 1。

---

## 🗺️ 路线图（v3.2 更新）

### 已完成（✅）
- **M0**（2026-06）：仓库初始化 + 团队对齐
- **W7 M3.1**：团队模型（角色定义 + 模板 + 模型路由）
- **W8 M3.2**：DAG 调度（TaskDag + 状态机 + 调度器 + 11 集成测试）
- **W9 M3.3**：双 Agent MVP（Orchestrator + Coder + 任务看板 UI + 重试）

### 进行中（🟡）
- **W9.5 - W10 M4.0**（2026-07~08）：互动机制 + AI 员工定义 + ChatPanel

### 未来规划（🟢 → 🔮）
- **M4.1**（2026-09~10）：AI 员工培训引擎（1.5 个月）
- **M4.2**（2026-11~12）：5 核心细分模板 + 5 候补 + 自定义向导（2 个月）⭐
- **M5**（2027-01~02）：Beta 准备（精简沙箱 + Skills + 内部 Beta）
- **GA 1.0**（2027-04）：付费用户 30+
- **M6 远期**（2027-05+）：WordPress 式产品周边 + 6 层用户生态 🔮

详见：
- [docs/ROADMAP.md](docs/ROADMAP.md) — 主线 20 周
- [docs/IMPROVEMENT-PLAN.md](docs/IMPROVEMENT-PLAN.md) — 完善+增强计划（v2.2）
- [docs/FUTURE-PLANS.md](docs/FUTURE-PLANS.md) — 未来规划参考库（含触发条件）

---

## 💰 商业模式（v3.2 草拟）

| 档位 | 价格 | 定位 | 关键功能 |
|------|:---:|------|---------|
| **Community** | ¥0 | 个人 / 学习者 | 桌面端 + 5 行业模板 + 1 私有模板 |
| **Pro** | ¥99/月 | 独立开发者 / 小团队 | + 10 私有模板 + 社区模板下载 |
| **Team** | ¥299/月/席位 | 5-50 人企业 | + 团队空间 + 自定义向导 + 审计 |
| **Enterprise** | ¥999/月/席位 | 50+ 上市公司 | + 私有化 + 私有 LLM + SLA |

详见 [tier.yaml](tier.yaml)。

---

## 🆚 竞品对比（v3.2 重新定位）

| 项目 | AgentWork | Cursor | Devin | OpenHands |
|------|:---:|:---:|:---:|:---:|
| **定位** | AI 智能体工作台 | AI 编辑器 | AI 程序员 | AI Agent Runtime |
| **开源** | ✅ Apache 2.0 | ❌ 闭源 | ❌ 闭源 | ✅ Apache 2.0 |
| **多 Agent DAG** | ✅ 真并行 | ⚠️ Planner/Worker | ⚠️ 有限 | ⚠️ 单 Agent |
| **异构模型** | ✅ Qwen/GLM/MiniMax | ❌ 固定 | ❌ 固定 | ⚠️ 单一 |
| **AI 员工培训** | ✅ 知识库+规则+话术+评估 | ❌ | ❌ | ❌ |
| **行业模板** | ✅ 5 核心+自定义 | ❌ | ❌ | ❌ |
| **互动面板** | ✅ ChatPanel 流式 | ✅ 编辑器内 | ❌ | ⚠️ CLI |
| **任务看板** | ✅ DAG 可视化 | ❌ | ⚠️ 步骤 | ❌ |
| **本地优先** | ✅ 桌面端 | ⚠️ 编辑器 | ❌ 云端 | ⚠️ CLI |
| **资源需求** | 4 核 8G 即可 | - | - | 16G+ |
| **MCP 协议** | ✅ | ❌ | ❌ | ✅ Server |
| **企业私有化** | 🔮 未来（FUTURE-PLANS）| ❌ | ❌ | ⚠️ 自建 |

**核心差异化**：**唯一同时具备"AI 团队协作 + AI 员工培训 + 行业模板 + 互动面板 + 本地优先"的产品**。

---

## 📜 许可

- **核心代码**：Apache 2.0
- **企业模块**：商业授权
- **Skills / 模板市场**：双许可（Apache 2.0 + 商业）
- **依赖声明**：详见各 crate / package 的 NOTICE 文件

---

## 🤝 贡献

欢迎贡献！详见 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)。

贡献流程：
1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feat/amazing-feature`)
3. 提交改动 (`git commit -m 'feat: add amazing feature'`)
4. 推送分支 (`git push origin feat/amazing-feature`)
5. 创建 Pull Request（使用 [PR 模板](.github/PULL_REQUEST_TEMPLATE.md)）

### 提交规范（Conventional Commits）

```bash
feat: 新功能
fix:  修 bug
docs: 文档变更
refactor: 重构
test: 测试
chore: 杂项

# 示例：
git commit -m "feat(agent-team): add template customize wizard"
```

---

## 📞 联系方式

- **GitHub**：https://github.com/yunji/agentwork
- **Gitee 镜像**：https://gitee.com/yunji/agentwork
- **Email**：agentwork@yunji.ai
- **官网**（筹备中）：https://agentwork.yunji.ai

---

## 🙏 致谢

本项目站在以下开源巨人的肩膀上：

- [OpenHands](https://github.com/All-Hands-AI/OpenHands) — Agent Runtime（Apache 2.0）
- [MCP](https://modelcontextprotocol.io) — 工具协议（MIT）
- [Tauri](https://tauri.app) — 桌面框架
- [libgit2](https://libgit2.org) — Git 操作
- [hnsw_rs](https://github.com/jean-pierreBoth/hnswlib-rs) — 内存向量索引
- [bge-small-zh](https://huggingface.co/BAAI/bge-small-zh) — 中文 Embedding 模型

架构参考：
- [UI-TARS-desktop](https://github.com/bytedance/UI-TARS-desktop) by 字节跳动（Monorepo + MCP + pnpm workspace）

设计灵感：
- [WordPress](https://wordpress.org) — "核心 + 主题 + 插件 + 应用市场"生态（终极愿景）
- [Cursor](https://cursor.com) / [Devin](https://devin.ai) / [Aider](https://aider.chat) — AI Agent 形态参考

---

> **云集智能体工作台（AgentWork），让每个公司打造自己的 AI 智能体团队。**
> *AgentWork — Where AI Becomes Your Workforce.*
