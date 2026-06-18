# AI 员工定义指南

> **目的**：教你如何自定义 AI 员工，打造符合自己公司业务的专属 AI 员工。
> **适用版本**：M4.0 D2（v3.2）
> **配套**：[IMPROVEMENT-PLAN.md](IMPROVEMENT-PLAN.md) § M4.0 D2

---

## 1. 什么是 AI 员工？

AI 员工（EmployeeDefinition）是 AgentWork 中**单个 AI 智能体**的完整定义。

一个 AI 员工可以：
- **独立工作**（单 Agent 场景）：客服、销售、财务、教育助教……
- **加入团队**（多 Agent 协作）：作为 TeamTemplate 中的一个 Role

### 1.1 与 TeamTemplate 的关系

| 概念 | 说明 | 示例 |
|------|------|------|
| **EmployeeDefinition** | 单个 AI 员工 | 客服、开发、销售 |
| **TeamTemplate** | 一组 Role 协作 | 全栈团队（PM + 后端 + 前端 + 测试）|
| **关系** | Employee 可转为 Role 加入团队 | 客服员工 → 团队的客服角色 |

---

## 2. 内置员工（6 个）

AgentWork 内置 6 个员工模板（3 通用 + 3 行业）：

### 2.1 通用员工（3 个）

| ID | 岗位名 | 角色 | 模型 | 说明 |
|----|--------|------|------|------|
| `customer_service` | 客服 | answer_questions | qwen3.7 | 售前咨询 + 售后处理 + 评价管理 |
| `developer` | 开发工程师 | write_code | glm5.2 | 写代码 + 修 bug + 重构 + 测试 |
| `assistant` | 通用助手 | general_assistant | qwen3.7 | 日程 + 邮件 + 文档 + 决策辅助 |

### 2.2 行业员工（3 个）

| ID | 岗位名 | 行业 | 模型 | 说明 |
|----|--------|------|------|------|
| `education_teacher` | 教育助教 | education | qwen3.7 | 学科答疑 + 作业批改 + 学情分析 |
| `sales_followup` | 销售跟进 | sales | qwen3.7 | 线索筛选 + 客户跟进 + 报价管理 |
| `finance_auditor` | 财务对账 | finance | glm5.2 | 票据识别 + 账目核对 + 报表生成 |

---

## 3. 员工定义结构（YAML）

```yaml
# 员工 ID（唯一标识）
id: "customer_service"

# 岗位名（展示用）
name: "客服"

# 角色标识（功能分类）
role: "answer_questions"

# 职责描述（给用户看）
description: "通用客服员工，处理售前咨询、售后问题、评价管理"

# 使用的 LLM 模型 ID
model_id: "qwen3.7"

# 备用模型列表（故障转移顺序）
model_fallback:
  - "glm5.2"

# 系统提示词（基础 prompt，最重要）
system_prompt: |
  你是一名专业的客服员工。你的职责是：
  1. 友好地回答客户问题
  2. 处理售前咨询
  3. 处理售后问题
  4. 主动邀评，负面评价及时预警

# 技能标签（能力描述）
capabilities:
  - pre_sales_consultation
  - after_sales_handling
  - review_management

# MCP 工具列表（可调用的工具）
tools:
  - order_query
  - logistics_query
  - refund_process

# 知识库 ID（M4.1 培训引擎用，null = 无知识库）
knowledge_base: null

# 规则列表（M4.1 规则引擎用）
rules:
  - "金额>500转人工"
  - "投诉>2次升级主管"

# 话术示例（M4.1 few-shot 训练用）
examples:
  - user: "这个衣服有 XL 码吗？"
    assistant: "亲，有的！XL 码适合 70-80kg 的亲，建议参考尺码表哦~"
    tag: "售前"
  - user: "我要退款"
    assistant: "好的亲，请提供订单号，我帮您查询退款流程~"
    tag: "售后"

# 行业分类（null = 通用，"ecommerce" / "education" / "finance" / ...）
industry: null

# 自定义元数据（扩展字段）
metadata:
  author: "yunji"
  version: "1.0"
```

---

## 4. 字段详解

### 4.1 必填字段

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `id` | String | 员工唯一标识 | `customer_service` |
| `name` | String | 岗位名（展示用）| `客服` |
| `role` | String | 角色标识 | `answer_questions` |
| `model_id` | String | LLM 模型 ID | `qwen3.7` |
| `system_prompt` | String | 系统提示词 | `你是一名客服...` |

### 4.2 可选字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `description` | String | `""` | 职责描述 |
| `model_fallback` | Vec<String> | `[]` | 备用模型列表 |
| `capabilities` | Vec<String> | `[]` | 技能标签 |
| `tools` | Vec<String> | `[]` | MCP 工具列表 |
| `knowledge_base` | Option<String> | `null` | 知识库 ID |
| `rules` | Vec<String> | `[]` | 规则列表 |
| `examples` | Vec<Example> | `[]` | 话术示例 |
| `industry` | Option<String> | `null` | 行业分类 |
| `metadata` | HashMap<String,String> | `{}` | 自定义元数据 |

### 4.3 Example 结构

```yaml
examples:
  - user: "用户输入"           # 必填
    assistant: "期望的 AI 回复" # 必填
    tag: "售前"                # 可选，示例标签
```

---

## 5. 如何自定义员工

### 5.1 基于内置模板修改（推荐）

1. 复制内置员工 YAML
2. 修改 `id` / `name` / `system_prompt`
3. 保存到 `.yunji/employees/{your_id}.yml`

```yaml
# 基于 customer_service 修改的穿搭师员工
id: "fashion_stylist"
name: "穿搭师"
role: "style_recommendation"
description: "电商穿搭行业员工，提供搭配推荐和尺码建议"
model_id: "qwen3.7"
model_fallback:
  - "glm5.2"
system_prompt: |
  你是一名专业的穿搭师。你的职责是：
  1. 根据客户身高体重推荐尺码
  2. 根据场景（通勤/约会/宴会）推荐搭配
  3. 处理退换货（尺码不符/质量问题）
  4. 生成小红书种草文案
  
  沟通原则：
  - 亲切友好，使用"亲""姐妹"等用语
  - 不夸大面料功效
  - 涉及金额>500转人工
capabilities:
  - size_recommendation
  - style_matching
  - after_sales_handling
  - content_generation
tools:
  - order_query
  - size_chart
industry: "ecommerce"
metadata:
  author: "your_company"
  version: "1.0"
```

### 5.2 从空白创建

直接编写 YAML 文件，确保 5 个必填字段都有值。

### 5.3 加载自定义员工

员工文件放在 `.yunji/employees/` 目录下，AgentWork 启动时自动加载：

```
你的项目/
└── .yunji/
    └── employees/
        ├── fashion_stylist.yml
        ├── tech_advisor.yml
        └── custom_assistant.yml
```

---

## 6. 系统提示词编写指南

系统提示词（system_prompt）是 AI 员工的"灵魂"，直接决定员工的行为。

### 6.1 推荐结构

```
你是一名[岗位名]。你的职责是：
1. [职责1]
2. [职责2]
3. [职责3]

[工作/沟通/教学/财务]原则：
- [原则1]
- [原则2]
- [原则3]
```

### 6.2 示例

**客服员工**：
```
你是一名专业的客服员工。你的职责是：
1. 友好地回答客户问题
2. 处理售前咨询
3. 处理售后问题

沟通原则：
- 语气亲切友好
- 不夸大商品功效
- 涉及金额>500转人工
```

**教育助教**：
```
你是一名教育助教。你的职责是：
1. 解答学生学科问题
2. 批改作业
3. 分析学情

教学原则：
- 启发式引导，不直接给答案
- 鼓励式反馈
- 涉及自伤/家暴立即转人工
```

---

## 7. 行业分类参考

| 行业 ID | 说明 | 内置员工 |
|---------|------|---------|
| `null` | 通用 | 客服 / 开发 / 助手 |
| `education` | 教育 | 教育助教 |
| `sales` | 销售 | 销售跟进 |
| `finance` | 财务 | 财务对账 |
| `ecommerce` | 电商 | （M4.2 添加）|
| `legal` | 法律 | （M4.2 候补）|
| `medical` | 医疗 | （M4.2 候补）|
| `hr` | 人力资源 | （M4.2 候补）|

---

## 8. API 参考（Rust）

```rust
use agent_team::employee::{EmployeeDefinition, builtin_employees};

// 获取所有内置员工
let employees = builtin_employees();
assert_eq!(employees.len(), 6);

// 按 ID 查找
let cs = agent_team::employee::builtin_by_id("customer_service").unwrap();

// 从 YAML 加载
let yaml = std::fs::read_to_string("my_employee.yml")?;
let emp = EmployeeDefinition::from_yaml(&yaml)?;

// 保存到文件
emp.save_to_file(std::path::Path::new("my_employee.yml"))?;

// 从目录加载所有员工
let all = EmployeeDefinition::load_from_dir(std::path::Path::new(".yunji/employees"))?;

// 转为团队角色
let role = emp.to_role();
```

---

## 9. 下一步（M4.1 培训引擎）

M4.0 D2 只做员工**定义**，M4.1 培训引擎会让员工**变强**：

| M4.0 D2 | M4.1 培训引擎 |
|---------|--------------|
| `knowledge_base: null` | 接入知识库（RAG 检索）|
| `rules: []` | 规则引擎（关键词匹配 + LLM 确认）|
| `examples: []` | 话术训练（few-shot + 评估）|

详见 [IMPROVEMENT-PLAN.md](IMPROVEMENT-PLAN.md) § M4.1。

---

## 10. FAQ

**Q: 一个员工可以用多个模型吗？**
A: 可以。`model_id` 是主模型，`model_fallback` 是备用模型列表。主模型失败时按顺序尝试备用模型。

**Q: 员工和团队模板有什么区别？**
A: 员工 = 单个 AI 智能体；团队模板 = 一组角色协作。员工可通过 `to_role()` 转为团队角色。

**Q: 如何让员工调用外部工具？**
A: 在 `tools` 字段列出 MCP 工具名。M4.0 D3 会实现工具调用流式输出。

**Q: 系统提示词多长合适？**
A: 建议 100-500 字。太短行为不明确，太长消耗 token。核心是"职责 + 原则"。

**Q: 可以导入其他公司的员工定义吗？**
A: 可以。YAML 文件可直接复制分享。M4.2 D4 社区模板市场会支持一键导入。
