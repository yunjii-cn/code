# 评估用例编写指南（EVAL-GUIDE）

> **目的**：教你如何为 AI 员工编写高质量的评估用例，量化培训效果。
> **适用版本**：M4.1 D4+（agent-team v0.4.0+）
> **最后更新**：2026-06-19

---

## 1. 为什么要写评估用例？

AI 员工培训后，怎么知道"学会了"？靠人主观判断不行，必须有**量化指标**：

- **回归保护**：改了 prompt / 加了示例后，确保旧场景不退化
- **基线对比**：v1 vs v2，看分数趋势
- **合规审计**：金融 / 医疗等行业的合规用例必须 100% 通过
- **客户验收**：B 端交付时，跑评估套件作为验收依据

---

## 2. 核心概念

### 2.1 EvalCase（评估用例）

单个测试点，包含：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 唯一标识（如 `cs_001`） |
| `input` | String | 用户输入 |
| `expected_output` | Option<String> | 期望输出（参考答案，可选） |
| `rubric` | String | 评分标准（描述什么样的回复算合格） |
| `weight` | f32 | 权重（≥ 0.0，默认 1.0，重要用例可设 2.0+） |
| `category` | Option<String> | 分类（如"售前"/"售后"/"合规"） |
| `notes` | Option<String> | 备注 |

### 2.2 EvalSuite（评估套件）

一批用例的集合，对应一个员工的一个场景：

```yaml
id: cs_eval_v1
name: 电商客服评估套件
employee_id: customer_service
scenario: 电商客服咨询
version: 1.0.0
cases:
  - id: cs_001
    input: 我的订单什么时候发货？
    rubric: 正确说明发货时间（24小时内）并提醒物流通知
    weight: 1.0
    category: 售前
```

### 2.3 EvalReport（评估报告）

跑完评估后的结果：

- **总分**：0.0-10.0
- **通过率**：0.0-1.0（通过 = 总分 ≥ 6.0 且无约束违规）
- **分类统计**：按 category 聚合
- **Markdown 报告**：人类可读

### 2.4 BaselineComparison（基线对比）

对比两个版本的报告：

- **改进用例**：分数提升 > 0.5
- **退化用例**：分数下降 > 0.5
- **持平用例**：差异在 ±0.5 内
- **结论**：自动生成"改进"/"退化"/"持平"

---

## 3. 如何写好评估用例

### 3.1 rubric（评分标准）的写法

**❌ 坏例子**（太模糊）：
```
rubric: 回答正确
```

**✅ 好例子**（具体可判）：
```
rubric: 正确说明发货时间（24小时内）并提醒物流通知，必须包含"亲"称呼
```

**rubric 三要素**：
1. **事实正确性**：发货时间 / 退款周期 / 政策细节
2. **风格要求**：必须含"亲" / 必须含风险提示
3. **禁止行为**：不夸大 / 不承诺收益 / 不替代医生

### 3.2 权重设置建议

| 用例类型 | 建议权重 | 说明 |
|----------|----------|------|
| 普通咨询 | 1.0 | 默认 |
| 售后争议 | 1.2 | 客户情绪敏感 |
| 合规红线 | 2.0 | 违规 = 法律风险 |
| 投诉处理 | 1.5 | 影响 NPS |

### 3.3 分类设置

按业务流程分类，便于定位薄弱环节：

- **售前**：咨询 / 推荐 / 比较
- **售后**：退换货 / 物流 / 投诉
- **合规**：金融 / 医疗 / 教育 红线

---

## 4. 代码示例

### 4.1 创建评估套件

```rust
use agent_team::{EvalCase, EvalSuite};

let mut suite = EvalSuite::new("cs_eval_v1", "电商客服评估套件", "customer_service")
    .with_scenario("电商客服咨询")
    .with_version("1.0.0");

suite.add_case(
    EvalCase::new("cs_001", "我的订单什么时候发货？", "正确说明发货时间（24小时内）并提醒物流通知")
        .with_category("售前")
        .with_weight(1.0)
);
```

### 4.2 跑评估（有 LLM 引擎）

```rust
use agent_team::{EvaluationRunner, SpeechTraining};

let training: SpeechTraining = load_training();
let runner = EvaluationRunner::new(training);

let ai_responses: Vec<String> = vec![
    "亲，24 小时内发货哦~".into(),
    // ... 每个用例对应一个 AI 回复
];

let report = runner.evaluate_suite(&suite, &ai_responses, &llm_engine).await?;
println!("{}", report.to_markdown());
```

### 4.3 跑评估（仅约束检查，无 LLM）

```rust
let report = runner.evaluate_suite_constraints_only(&suite, &ai_responses)?;
```

### 4.4 对比基线

```rust
let baseline = EvalReport::load_json("baseline_v1.json")?;
let current = runner.evaluate_suite(&suite, &ai_responses, &llm).await?;
let cmp = EvaluationRunner::compare_baseline(&baseline, &current);
println!("{}", cmp.to_markdown());
```

---

## 5. 内置评估套件

agent-team 提供 3 套内置评估套件：

| 套件 | 员工 ID | 用例数 | 重点 |
|------|---------|--------|------|
| 电商客服 | `customer_service` | 5+ | 售前 / 售后 / 合规 |
| 金融顾问 | `finance_advisor` | 4+ | 合规红线（不保证收益 / 不推荐个股） |
| 医疗咨询 | `medical_consult` | 3+ | 不替代医生 / 不开处方 |

```rust
use agent_team::builtin_all_evals;
let suites = builtin_all_evals();
```

---

## 6. 最佳实践

### 6.1 评估用例数量

- **MVP**：10-20 个用例（覆盖核心场景）
- **生产**：50+ 用例（覆盖边缘情况）
- **合规行业**：100+ 用例（每个合规红线至少 3 个）

### 6.2 版本管理

- 每次改 prompt / 加示例后，跑评估 + 保存报告
- 报告文件命名：`{suite_id}_v{version}_{timestamp}.json`
- 定期对比基线，发现退化立即回滚

### 6.3 合规用例

金融 / 医疗 / 教育行业的合规用例：

- **权重设 2.0+**：违规一次就拉低总分
- **rubric 明确**：必须含"风险提示" / 必须含"建议就医"
- **约束强制**：在 SpeechTraining.constraints 中设置 forbidden_words

---

## 7. 常见问题

### Q: 评估分数总是很低怎么办？

A: 检查：
1. rubric 是否过严（如要求"必须含 5 个关键词"）
2. AI 回复是否真的不达标（看 feedback）
3. 约束是否冲突（forbidden_words 误伤）

### Q: 如何评估多轮对话？

A: 当前版本（v1.0）只支持单轮评估。多轮对话评估在 M5 Beta 计划中。

### Q: 可以用不同的 LLM 当裁判吗？

A: 可以。`evaluate_suite` 接受任何实现 `LlmEngine` 的引擎。建议用比被评估模型更强的 LLM 当裁判（如用 GPT-4 评估 GLM-5.2 的回复）。

---

## 8. 相关文档

- [IMPROVEMENT-PLAN.md](./IMPROVEMENT-PLAN.md) § M4.1 D4
- [EMPLOYEE-GUIDE.md](./EMPLOYEE-GUIDE.md) - AI 员工定义
- [AGENT-TEAM-DESIGN.md](./AGENT-TEAM-DESIGN.md) - 团队协作设计
