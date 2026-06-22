# AW 降维打击 Hermes · 全民 AI 计划书

> **版本**：v1.0
> **日期**：2026-06-22
> **作者**：Trae + 用户共同制定
> **目标**：全面碾压 Hermes Agent，做出差异化革新，面向全行业、全民 AI
> **状态**：待用户审批

---

## 0. 一句话定位

> **Hermes 是"工程师的 Agent"，AW 要做"全民的 Agent"。**

Hermes 的用户画像：会写 YAML/Markdown、会用命令行、懂 MCP 协议的开发者。
AW 的用户画像：餐饮店老板、早教老师、证券分析师、穿搭博主、法律顾问——**所有不会写代码的人**。

这不是功能差距，是**维度差距**。

---

## 1. 降维打击的 6 个维度

| # | 维度 | Hermes | AW 降维打击 |
|---|------|--------|------------|
| 1 | **用户群体** | 开发者（需写 YAML/MD） | 全民（对话式创建，零代码） |
| 2 | **进化机制** | 被动沉淀（Memory/Sessions/Skills） | 主动进化 + 可视化 + 可导出 + 可交易 |
| 3 | **隔离模型** | Profile 目录隔离（静态、手动配置） | 动态角色 + 行业模板市场 + 团队共享 |
| 4 | **工具生态** | MCP + 自注册工具（工程师接入） | 可视化工具市场 + 无代码工具构建器 |
| 5 | **入口场景** | CLI + 消息平台（技术入口） | 桌面 + Web + 移动 + API + 消息平台（全入口） |
| 6 | **商业模式** | 开源免费 | 免费 + Pro + Enterprise + 行业模板市场（分润） |

---

## 2. 差异化革新：5 大独创概念

### 2.1 革新 1：用户可选模式（User-Selectable Modes）

**Hermes 没有"模式"概念**——它只有一种运行方式：Agent Loop 自动循环。

**AW 独创 5 种模式**，用户可随时切换：

| 模式 | 适合场景 | Agent 行为 | 用户参与度 |
|------|---------|-----------|-----------|
| **学习模式** 🧠 | 新手用户、新行业 | Agent 主动观察用户操作，记录偏好，逐步建立用户画像 | 低（用户正常工作） |
| **教学模式** 👨‍🏫 | 老板培训 AI 员工 | 用户教 Agent 怎么做，Agent 记录流程为 Skill | 高（手把手教） |
| **协作模式** 🤝 | 日常办公 | Agent + 用户一起完成任务，Agent 主动建议 | 中（互动） |
| **自动模式** 🚀 | 重复性任务 | Agent 全自动执行，完成后汇报 | 极低（只看结果） |
| **审批模式** 🔒 | 高风险场景 | 每步都要用户确认后才执行 | 极高（每步审批） |

**实现方式**：
- 每个员工（EmployeeDefinition）可独立设置模式
- 同一员工在不同任务中可切换模式
- 模式由 `WorkMode` enum 定义，注入 system_prompt

**降维打击点**：Hermes 是"一刀切"的自动循环，AW 是"用户掌控节奏"的弹性协作。

### 2.2 革新 2：进化仪表盘（Evolution Dashboard）

**Hermes 的自进化是黑盒**——用户不知道 Agent 学到了什么。

**AW 独创"进化仪表盘"**，让进化可见、可控、可交易：

```
┌─────────────────────────────────────────────────┐
│  📈 小明的客服员工 · 进化报告（最近 30 天）       │
├─────────────────────────────────────────────────┤
│  🧠 记忆：42 条事实                              │
│     • 用户偏好：喜欢简洁回复（✅ 自动学习）       │
│     • 项目约定：金额>500 转人工（✅ 用户教学）    │
│     • 工具坑点：PDF 解析慢，建议转文本（自动）    │
│                                                 │
│  📚 技能：8 个                                   │
│     • 退款流程处理（成功 23 次 / 失败 1 次）      │
│     • 投诉安抚话术（成功 15 次 / 失败 0 次）      │
│     • [新] 订单查询优化（自动生成，待用户确认）   │
│                                                 │
│  💬 会话：156 次                                 │
│     • 平均满意度：4.6/5                          │
│     • 常见问题 TOP3：退款、物流、产品咨询         │
│                                                 │
│  📊 能力雷达图                                    │
│     退款处理 ████████████ 95%                    │
│     投诉处理 ███████████  88%                    │
│     产品咨询 ████████      72%                   │
│     物流查询 █████████     80%                   │
│                                                 │
│  [📥 导出进化包]  [📤 上传到市场]  [⏪ 回滚]      │
└─────────────────────────────────────────────────┘
```

**3 个独家功能**：
1. **导出进化包**：把员工的全部记忆+技能+话术打包成 `.aw-evolution` 文件，可迁移到其他设备
2. **上传到市场**：把训练好的员工上架到"AI 员工市场"，其他用户付费下载（创作者分润 70%）
3. **回滚**：进化出问题时，可回滚到任意历史快照（基于 TimeFlow 版本控制）

**降维打击点**：Hermes 的进化是"被动沉淀 + 不可见"，AW 是"主动进化 + 可视化 + 可交易"。

### 2.3 革新 3：行业模板市场 + 无代码自定义（Industry Template Marketplace）

**Hermes 的 Profile 是工程师手动配置**——写 SOUL.md、config.yaml、.env。

**AW 独创"行业模板市场 + 无代码自定义向导"**：

```
用户想开一家奶茶店：
1. 打开 AW → 模板市场 → 搜索"奶茶"
2. 找到"奶茶店全套 AI 员工"模板（由其他奶茶店老板上传）
3. 一键安装 → 自动创建 5 个员工：
   • 前台客服（处理点单咨询、推荐新品）
   • 库存管理员（监控原料、提醒补货）
   • 排班助手（根据客流排班）
   • 财务对账（每日营收统计）
   • 营销策划（设计促销活动）
4. 每个员工自带：
   • 奶茶行业知识库（产品、原料、设备）
   • 奶茶行业话术（50 条 few-shot）
   • 奶茶行业规则（食品安全、卫生标准）
   • 奶茶行业评估用例（20 条）
5. 用户可通过"自定义向导"调整：
   • "我们店主打水果茶" → 知识库自动聚焦水果茶
   • "我们在上海" → 话术加入上海话元素
   • "我们用小程序点单" → 工具接入微信小程序 API
```

**无代码自定义向导**（5 步）：
1. 选行业（10 核心 + 社区 N 个）
2. 描述业务（自然语言，AI 自动生成员工定义）
3. 上传资料（产品手册、话术录音、历史对话）
4. 调整风格（滑块：正式 ↔ 亲切，简洁 ↔ 详细）
5. 试运行 + 评估（AI 自动跑 20 条测试用例，给出能力评分）

**降维打击点**：Hermes 的 Profile 是"工程师的玩具"，AW 是"老板的一键安装"。

### 2.4 革新 4：全民工具构建器（No-Code Tool Builder）

**Hermes 的工具需要写代码注册**——`registry.register(MyTool)`。

**AW 独创"无代码工具构建器"**：

```
用户：我想让客服员工能查订单状态
AW：好的，我来帮你创建这个工具

┌─────────────────────────────────────────────────┐
│  🔧 工具构建器 · 查询订单状态                     │
├─────────────────────────────────────────────────┤
│  第 1 步：工具信息                                │
│  名称：查询订单状态                               │
│  描述：根据订单号查询物流和状态                    │
│                                                 │
│  第 2 步：输入参数                                │
│  • order_id（订单号，必填，字符串）               │
│  • phone（手机号，选填，字符串）                  │
│                                                 │
│  第 3 步：数据源（选一个）                         │
│  ○ 调用 API（填 URL + 方法 + Headers）           │
│  ○ 查询数据库（填连接串 + SQL）                   │
│  ○ 读取文件（填路径 + 格式）                      │
│  ○ 调用另一个 AI 员工（选员工 + 传参）            │
│  ● 接入 MCP 服务器（填服务器地址）                │
│                                                 │
│  第 4 步：输出格式                                │
│  • 返回物流公司、运单号、状态、预计到达时间        │
│                                                 │
│  第 5 步：测试                                    │
│  输入：order_id = "DD20260622001"                │
│  输出：✅ 顺丰快递，SF1234567890，运输中，明日达  │
│                                                 │
│  [💾 保存并启用]  [🔄 再测一次]                   │
└─────────────────────────────────────────────────┘
```

**支持的数据源**：
- HTTP API（GET/POST/PUT/DELETE）
- 数据库（MySQL/PostgreSQL/SQLite）
- 文件（CSV/Excel/JSON/Markdown）
- 其他 AI 员工（Agent-to-Agent 调用）
- MCP 服务器（兼容 Hermes 生态）
- 系统命令（白名单内）

**降维打击点**：Hermes 的工具是"开发者写代码"，AW 是"用户填表单"。

### 2.5 革新 5：团队知识共享（Team Knowledge Sharing）

**Hermes 是单机工具**——Profile 在本地目录，无法团队共享。

**AW 独创"团队知识共享"**：

```
公司：星巴克中国
├── .aw/                          ← 整个目录可 git 跟踪
│   ├── memory/                   ← 公司级事实记忆
│   │   ├── company.json          ← "我们是星巴克，主打咖啡"
│   │   └── user-profiles/        ← 每个用户的偏好
│   ├── skills/                   ← 公司级技能库
│   │   ├── 退款流程.md
│   │   ├── 新品推荐.md
│   │   └── 会员升级.md
│   ├── employees/                ← 公司级员工定义
│   │   ├── barista.yaml          ← 咖啡师员工
│   │   └── cashier.yaml          ← 收银员工
│   ├── knowledge/                ← 公司级知识库
│   │   ├── menu.md               ← 菜单
│   │   └── recipes/              ← 配方
│   └── agents.md                 ← 公司规范
│
├── 上海分部/.aw/                  ← 分部级覆盖
│   └── memory/
│       └── shanghai.json         ← "上海用户偏好少糖"
│
└── 张三/.aw/                      ← 个人级覆盖
    └── memory/
        └── zhangsan.json         ← "张三是新员工，需要详细指导"
```

**3 层覆盖机制**：
1. **公司级**（`.aw/`）：全员共享的基础知识
2. **部门级**（`部门/.aw/`）：部门特化知识
3. **个人级**（`用户/.aw/`）：个人偏好

**同步机制**：
- 基于 Git（隐身/同步/发布三模式）
- 加密同步（AES-256-GCM，已有 `sync.rs`）
- 冲突解决（3-way merge）

**降维打击点**：Hermes 是"一个人的 Agent"，AW 是"一个团队的 AI 员工队伍"。

---

## 3. 实施计划：4 期 18 周

### 3.1 总览

```
=== 第 1 期：自进化基础（W1-W4，4 周）===
目标：让 Agent 能记忆、能学习、能沉淀
新增模块：memory.rs / sessions.rs / skills_v2.rs / prompt_builder.rs

=== 第 2 期：全民可用（W5-W9，5 周）===
目标：让非技术用户能零代码创建 AI 员工
新增模块：work_mode.rs / evolution_dashboard.rs / no_code_tool_builder.rs
新增前端：进化仪表盘 / 工具构建器 / 模式切换器

=== 第 3 期：行业生态（W10-W14，5 周）===
目标：建立行业模板市场 + 团队知识共享
新增模块：team_knowledge.rs / template_marketplace.rs / profile_manager.rs
新增前端：模板市场 / 团队知识中心 / Profile 管理器

=== 第 4 期：降维打击（W15-W18，4 周）===
目标：MCP 兼容 + 多入口 + 安全增强
新增模块：mcp_client.rs / multi_entry.rs / sandbox_v2.rs / security.rs
```

### 3.2 第 1 期：自进化基础（W1-W4）

#### W1：Memory 模块（长期事实记忆）

**新增文件**：`platformkit/crates/agent-team/src/memory.rs`

**核心结构**：
```rust
/// 长期事实记忆（对应 Hermes MEMORY.md + USER.md）
pub struct Memory {
    pub facts: Vec<MemoryFact>,
    pub user_profile: UserProfile,
    pub tool_pitfalls: Vec<ToolPitfall>,
    pub project_conventions: Vec<Convention>,
}

pub struct MemoryFact {
    pub id: String,
    pub category: FactCategory,        // User / Project / Environment / Tool
    pub content: String,
    pub confidence: f32,               // 0.0-1.0
    pub source: FactSource,            // Learned / Taught / Imported
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub use_count: u32,
}

pub struct UserProfile {
    pub communication_style: String,   // "简洁" / "详细" / "正式" / "亲切"
    pub expertise_level: ExpertiseLevel, // Novice / Intermediate / Expert
    pub preferences: HashMap<String, String>,
    pub language: String,
}

pub enum FactSource {
    /// Agent 自动学习（学习模式）
    Learned,
    /// 用户主动教学（教学模式）
    Taught,
    /// 从进化包导入
    Imported,
    /// 团队共享
    Shared,
}
```

**存储**：`<repo>/.aw/memory/memory.json`（可 git 跟踪）

**注入**：每次 Agent 启动时，自动拼入 system_prompt

**自进化触发条件**：
- 用户连续 3 次纠正同一类错误 → 记录为 ToolPitfall
- 用户连续 5 次采用某种回复风格 → 更新 UserProfile
- Agent 执行任务成功 → confidence +0.1（上限 1.0）
- Agent 执行任务失败 → confidence -0.2（下限 0.0，低于 0.3 自动标记"待复核"）

#### W2：Sessions 模块（跨会话过程记忆）

**新增文件**：`platformkit/crates/agent-team/src/sessions.rs`

**核心结构**：
```rust
/// 跨会话过程记忆（对应 Hermes Sessions + FTS5）
pub struct SessionStore {
    db: rusqlite::Connection,
}

impl SessionStore {
    pub fn save_session(&self, session: &Session) -> Result<()>;
    pub fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSnippet>>;
    pub fn get_session(&self, id: &str) -> Result<Option<Session>>;
    pub fn list_recent_sessions(&self, limit: usize) -> Result<Vec<SessionSummary>>;
    pub fn export_session(&self, id: &str) -> Result<String>;  // 导出为 JSON
}

pub struct Session {
    pub id: String,
    pub employee_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub messages: Vec<SessionMessage>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub token_stats: TokenStats,
    pub satisfaction_score: Option<f32>,  // 用户评分
    pub tags: Vec<String>,                // 自动打标签
}

pub struct SessionMessage {
    pub role: MessageRole,               // User / Assistant / Tool
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_call_id: Option<String>,
}
```

**存储**：`<repo>/.aw/sessions/state.db`（SQLite + FTS5）

**能力**：
- 全文检索历史会话（"上次怎么解决退款问题的？"）
- 按员工/时间/标签筛选
- 导出会话为 JSON（可分享、可分析）
- 自动打标签（基于 LLM 分类）

#### W3：Skills V2 模块（程序性记忆 + 自进化）

**新增文件**：`platformkit/crates/agent-team/src/skills_v2.rs`（升级现有 `skill_market.rs`）

**核心结构**：
```rust
/// 程序性记忆（对应 Hermes Skills，升级版）
pub struct SkillV2 {
    pub id: String,
    pub name: String,
    pub description: String,            // 仅描述进 prompt（progressive disclosure）
    pub content: String,                // 完整内容按需加载
    pub trigger_keywords: Vec<String>,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub success_count: u32,
    pub failure_count: u32,
    pub success_rate: f32,
    pub last_used: Option<DateTime<Utc>>,
    pub created_by: SkillAuthor,        // Human / Agent / Imported
    pub version: u32,
    pub status: SkillStatus,            // Draft / Active / Deprecated / PendingReview
}

pub enum TriggerCondition {
    KeywordMatch(String),
    TaskType(String),
    UserMode(WorkMode),
    ConfidenceBelow(f32),
}

impl SkillV2 {
    /// 自动生成新技能（自进化核心）
    pub async fn auto_generate(
        session: &Session,
        llm: &dyn LlmEngine,
    ) -> Result<Option<SkillV2>>;

    /// 记录使用结果（自进化反馈）
    pub fn record_usage(&mut self, success: bool);

    /// 评估技能质量
    pub fn quality_score(&self) -> f32;
}
```

**存储**：`<repo>/.aw/skills/*.md`（Markdown 驱动，git 友好）

**自进化闭环**：
1. Agent 完成任务后，LLM 判断"这是否是新流程"
2. 若是，自动生成 SkillV2（status: Draft）
3. 用户在进化仪表盘确认（status: Active）
4. 后续遇到类似任务，自动加载该 Skill
5. 记录成功/失败，动态调整 success_rate
6. success_rate < 0.3 自动标记 Deprecated

#### W4：Prompt Builder 模块（分层 Prompt 引擎）

**新增文件**：`platformkit/crates/agent-team/src/prompt_builder.rs`

**核心结构**：
```rust
/// 分层 Prompt 引擎（对应 Hermes prompt_builder.py）
pub struct PromptBuilder {
    layers: Vec<PromptLayer>,
    token_budget: usize,
}

pub struct PromptLayer {
    pub name: String,                    // SOUL / AGENTS / MEMORY / USER / SKILLS / TOOLS / CONTEXT
    pub priority: u8,                    // 0-255，高优先级先保留
    pub content: String,
    pub token_count: usize,
    pub dynamic: bool,                   // 是否动态加载
}

impl PromptBuilder {
    pub fn new() -> Self;
    pub fn add_layer(&mut self, layer: PromptLayer);
    pub fn build(&self, budget: usize) -> String;  // 按预算裁剪

    /// 渐进发现（对应 Hermes 子目录 AGENTS.md）
    pub fn discover_context(&mut self, path: &Path);
}
```

**7 层 Prompt**：
1. **SOUL**：角色人格 + 语气（最高优先级）
2. **AGENTS**：项目规范（根 AGENTS.md）
3. **MEMORY**：长期事实记忆
4. **USER**：用户画像
5. **SKILLS_INDEX**：技能描述（仅描述，不展开）
6. **TOOLS**：工具 schema（仅启用的）
7. **CONTEXT**：RAG 检索结果 + 当前任务上下文

**Token 预算管理**：
- 总预算 = 模型上下文窗口 - 任务预留 - 输出预留
- 按优先级裁剪：CONTEXT > SKILLS_INDEX > TOOLS > MEMORY > USER > AGENTS > SOUL
- 渐进发现：读到子目录时动态加载该目录的 AGENTS.md

### 3.3 第 2 期：全民可用（W5-W9）

#### W5：WorkMode 模块（用户可选模式）

**新增文件**：`platformkit/crates/agent-team/src/work_mode.rs`

```rust
/// 用户可选模式（AW 独创）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkMode {
    /// 学习模式：Agent 主动观察用户操作
    Learning,
    /// 教学模式：用户教 Agent 怎么做
    Teaching,
    /// 协作模式：Agent + 用户一起完成
    Collaborative,
    /// 自动模式：Agent 全自动执行
    Autonomous,
    /// 审批模式：每步都要用户确认
    Approval,
}

impl WorkMode {
    /// 根据模式生成 system_prompt 补充
    pub fn system_prompt_hint(&self) -> &str;

    /// 根据模式决定是否需要用户确认
    pub fn requires_approval(&self, action: &Action) -> bool;

    /// 根据模式决定是否记录学习
    pub fn should_learn(&self) -> bool;
}
```

**前端**：每个员工卡片右上角加模式切换器（5 个图标按钮）

#### W6-W7：进化仪表盘

**新增前端**：`apps/desktop/src/views/EvolutionDashboard.tsx`

**功能**：
- 记忆列表（可编辑、可删除、可导出）
- 技能列表（可启用/禁用、可编辑、可导出）
- 会话历史（可搜索、可回放、可导出）
- 能力雷达图（基于评估结果）
- 进化时间线（可视化成长历程）
- 导出进化包 / 上传到市场 / 回滚

**新增后端命令**：
- `list_memories(employee_id)`
- `delete_memory(id)`
- `export_evolution_pack(employee_id) -> path`
- `import_evolution_pack(path) -> employee_id`
- `rollback_evolution(employee_id, snapshot_id)`
- `get_evolution_report(employee_id) -> EvolutionReport`

#### W8-W9：无代码工具构建器

**新增前端**：`apps/desktop/src/views/ToolBuilder.tsx`

**新增后端**：`platformkit/crates/agent-team/src/tool_builder.rs`

**支持的数据源**：
- HTTP API
- 数据库（MySQL/PostgreSQL/SQLite）
- 文件（CSV/Excel/JSON/Markdown）
- 其他 AI 员工
- MCP 服务器
- 系统命令（白名单内）

**工具定义格式**（YAML，用户无感知）：
```yaml
id: query_order_status
name: 查询订单状态
description: 根据订单号查询物流和状态
parameters:
  - name: order_id
    type: string
    required: true
    description: 订单号
data_source:
  type: http_api
  url: https://api.example.com/orders/{order_id}
  method: GET
  headers:
    Authorization: Bearer ${env.ORDER_API_KEY}
output_format:
  fields:
    - name: logistics_company
    - name: tracking_number
    - name: status
    - name: estimated_arrival
```

### 3.4 第 3 期：行业生态（W10-W14）

#### W10-W11：团队知识共享

**新增后端**：`platformkit/crates/agent-team/src/team_knowledge.rs`

**3 层覆盖机制**：
- 公司级（`.aw/`）
- 部门级（`部门/.aw/`）
- 个人级（`用户/.aw/`）

**冲突解决**：3-way merge（基于 TimeFlow）

#### W12-W13：模板市场增强

**升级**：`platformkit/crates/agent-team/src/template_market.rs`

**新增**：
- AI 员工市场（不只是模板，是训练好的完整员工）
- 进化包交易（用户付费下载别人训练的员工）
- 创作者分润（70%）
- 评分 + 评论 + 试用

#### W14：Profile 管理器

**新增后端**：`platformkit/crates/agent-team/src/profile_manager.rs`

**借鉴 Hermes Profile**，但升级为：
- 可视化创建（不写 YAML）
- 一键克隆（基于现有员工派生）
- 团队共享（git 同步）
- 权限隔离（独立密钥、独立工具集）

### 3.5 第 4 期：降维打击（W15-W18）

#### W15：MCP 客户端

**新增后端**：`platformkit/crates/agent-team/src/mcp_client.rs`

**兼容 Hermes 生态**：
- 支持 stdio MCP Server
- 支持 HTTP MCP Server
- Per-server 工具过滤
- 自动发现 + 注册

**降维打击点**：AW 用户可以**无代码接入** Hermes 生态的所有 MCP 服务器。

#### W16：多入口支持

**新增**：
- API Server（外部系统调用 AW 员工）
- Webhook（接收外部事件）
- 消息平台接入（微信/钉钉/飞书，通过 MCP）
- 定时任务（Cron）

#### W17：安全增强

**新增后端**：`platformkit/crates/agent-team/src/security.rs`

- 危险命令审批（对应 Hermes）
- 容器隔离（Docker 后端）
- MCP 凭证过滤
- 跨 Session 隔离
- 输入净化

#### W18：打磨 + 发布

- 性能优化
- 文档完善
- 演示视频
- Beta 发布

---

## 4. 技术架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        AW 全民 AI 平台                           │
├─────────────────────────────────────────────────────────────────┤
│  入口层（多入口）                                                 │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐     │
│  │桌面 UI│ Web  │移动端 │ API  │Webhook│微信  │钉钉  │ Cron │     │
│  └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘     │
├─────────────────────────────────────────────────────────────────┤
│  模式层（用户可选）                                               │
│  ┌────────┬────────┬──────────┬────────┬────────┐               │
│  │学习模式│教学模式│协作模式  │自动模式│审批模式│               │
│  └────────┴────────┴──────────┴────────┴────────┘               │
├─────────────────────────────────────────────────────────────────┤
│  核心循环层（Agent Loop）                                         │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  Prompt Builder（7 层分层）                           │       │
│  │  ┌─────┬──────┬──────┬─────┬──────┬─────┬───────┐  │       │
│  │  │SOUL │AGENTS│MEMORY│USER│SKILLS│TOOLS│CONTEXT│  │       │
│  │  └─────┴──────┴──────┴─────┴──────┴─────┴───────┘  │       │
│  │  Agent Loop（迭代预算 + Fallback + 压缩 + 回调）     │       │
│  └──────────────────────────────────────────────────────┘       │
├─────────────────────────────────────────────────────────────────┤
│  能力扩展层                                                       │
│  ┌──────────────┬──────────────┬──────────────┬──────────┐      │
│  │工具运行时     │无代码工具构建器│ MCP 客户端   │沙箱后端  │      │
│  │（自注册）     │（表单式）     │（兼容 Hermes）│（多后端）│      │
│  └──────────────┴──────────────┴──────────────┴──────────┘      │
├─────────────────────────────────────────────────────────────────┤
│  长期沉淀层（4 类记忆 = AI 知识库）                                │
│  ┌──────────┬──────────┬──────────┬──────────┐                  │
│  │声明性知识 │情景性记忆 │历史性记忆 │程序性记忆 │                  │
│  │knowledge │ memory   │ sessions │ skills_v2│                  │
│  │  (RAG)   │(事实偏好) │(FTS5检索) │(流程方法论)│                 │
│  └──────────┴──────────┴──────────┴──────────┘                  │
├─────────────────────────────────────────────────────────────────┤
│  生态层                                                           │
│  ┌──────────┬──────────┬──────────┬──────────┐                  │
│  │行业模板市场│AI员工市场 │进化包交易 │团队知识共享│                 │
│  │(10核心+N) │(训练好的) │(70%分润) │(3层覆盖)  │                 │
│  └──────────┴──────────┴──────────┴──────────┘                  │
├─────────────────────────────────────────────────────────────────┤
│  安全运行层                                                       │
│  ┌────────┬────────┬────────┬────────┬────────┐                 │
│  │危险命令 │容器隔离 │MCP凭证 │跨Session│输入净化 │                 │
│  │审批    │(Docker) │过滤    │隔离    │        │                 │
│  └────────┴────────┴────────┴────────┴────────┘                 │
├─────────────────────────────────────────────────────────────────┤
│  持久化层                                                         │
│  ┌──────────────────────────────────────────────┐               │
│  │  <repo>/.aw/                                  │               │
│  │  ├── memory/        (JSON)                    │               │
│  │  ├── sessions/      (SQLite + FTS5)           │               │
│  │  ├── skills/        (Markdown)                │               │
│  │  ├── employees/     (YAML)                    │               │
│  │  ├── knowledge/     (RAG 向量)                │               │
│  │  ├── profiles/      (目录隔离)                │               │
│  │  └── state.db       (定时任务 + 状态)         │               │
│  │  可 git 跟踪 + 加密同步 + 团队共享             │               │
│  └──────────────────────────────────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. 与 Hermes 的最终对比

| 维度 | Hermes v0.17 | AW 全民 AI 平台 | 降维打击点 |
|------|-------------|----------------|-----------|
| **用户群体** | 开发者 | 全民 | 对话式创建 vs 写 YAML |
| **自进化** | 被动沉淀 | 主动进化 + 可视化 | 进化仪表盘 + 可交易 |
| **子代理** | 动态生成 | DAG + 动态生成 + 递归 | 更强 |
| **Profile** | 目录隔离 | 目录 + 团队共享 + 市场 | 可交易 + 可共享 |
| **分层 Prompt** | 7 层 | 7 层 + Token 预算 + 渐进发现 | 持平 |
| **Tools Runtime** | 自注册 + MCP | 自注册 + MCP + **无代码构建器** | 用户可自建工具 |
| **入口** | CLI + 消息平台 | 桌面 + Web + 移动 + API + 消息 | 全入口 |
| **安全** | 审批 + 容器 | 审批 + 容器 + 凭证过滤 | 持平 |
| **行业覆盖** | 通用 | 10 核心 + 社区 N 个 | 行业模板市场 |
| **商业模式** | 开源免费 | 免费 + Pro + Enterprise + 市场分润 | 可持续 |
| **模式选择** | 无 | 5 种模式 | 用户掌控节奏 |
| **团队协作** | 单机 | 3 层知识共享 | 团队 AI 员工队伍 |

---

## 6. 风险与对策

| 风险 | 概率 | 影响 | 对策 |
|------|------|------|------|
| 18 周工期太紧 | 高 | 高 | 分 4 期发布，每期独立可用 |
| SQLite + FTS5 在 Windows 兼容性 | 中 | 中 | 用 rusqlite bundled feature |
| 无代码工具构建器复杂度高 | 高 | 高 | 先支持 HTTP API + 文件，其他后端迭代 |
| 模板市场冷启动 | 高 | 中 | 自营 10 个核心模板 + 创作者激励 |
| MCP 兼容性 | 中 | 中 | 优先支持 stdio，HTTP 后置 |

---

## 7. 成功指标

### 7.1 第 1 期结束（W4）
- ✅ Agent 能记住用户偏好（跨会话）
- ✅ Agent 能搜索历史会话
- ✅ Agent 能自动生成新技能
- ✅ Prompt 分层装配，Token 预算可控

### 7.2 第 2 期结束（W9）
- ✅ 非技术用户能 5 分钟内创建一个 AI 员工
- ✅ 用户能查看进化报告
- ✅ 用户能无代码创建工具
- ✅ 5 种模式可切换

### 7.3 第 3 期结束（W14）
- ✅ 10 个行业模板上线
- ✅ 团队可共享知识库
- ✅ 进化包可导出/导入
- ✅ 模板市场可交易

### 7.4 第 4 期结束（W18）
- ✅ 兼容 Hermes MCP 生态
- ✅ 多入口可用
- ✅ 安全边界完整
- ✅ Beta 发布

---

## 8. 下一步

**等待用户审批本计划书**。

审批通过后，立即开始第 1 期 W1：`memory.rs` 模块开发。

---

> **最后**：本计划书的目标不是"追上 Hermes"，而是"让 Hermes 成为我们的子集"。Hermes 的所有优势（自进化、子代理、Profile、分层 Prompt、Tools Runtime、MCP）我们都要吸收，同时用 5 大独创概念（用户可选模式、进化仪表盘、行业模板市场、无代码工具构建器、团队知识共享）实现降维打击。

> **面向全行业、全民 AI**——这是 AW 的终极定位。
