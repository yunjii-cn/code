// 培训效果评估系统（M4.1 D4）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D4
// 指南: docs/EVAL-GUIDE.md
//
// 目标：批量跑测试集，统计 AI 员工的回复质量分数，对比基线版本看改进。
//
// 技术选型（v2.0 明确）：
//   - 评估方式：LLM-as-Judge（复用 SpeechJudge）+ 约束检查
//   - 评分维度：风格 / 相关性 / 约束 / 总分（0-10）
//   - 对比基线：v1 vs v2，按用例对比，输出改进/退化清单
//   - 报告格式：JSON + Markdown（人类可读）
//
// 模块组成：
//   - EvalCase: 单个评估用例（输入 + 期望输出 + 评分标准 + 权重）
//   - EvalSuite: 评估套件（一批用例 + 元数据）
//   - EvalResult: 单个用例的评估结果
//   - EvalReport: 整体评估报告（含统计 + 对比基线）
//   - EvaluationRunner: 评估执行器（批量跑 + 生成报告）

use crate::error::{Result, TeamError};
use crate::speech::{SpeechTraining, SpeechJudge, JudgeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use timeflow_ai::LlmEngine;

// ============================================================================
// 评估用例
// ============================================================================

/// 单个评估用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// 用例 ID（唯一）
    pub id: String,
    /// 用户输入
    pub input: String,
    /// 期望输出（参考答案，可选）
    #[serde(default)]
    pub expected_output: Option<String>,
    /// 评分标准（rubric，描述什么样的回复算合格）
    pub rubric: String,
    /// 权重（0.0-1.0，默认 1.0）
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// 用例分类（如 "售前" / "售后" / "投诉"）
    #[serde(default)]
    pub category: Option<String>,
    /// 备注信息
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_weight() -> f32 {
    1.0
}

impl EvalCase {
    /// 创建新用例
    pub fn new(id: impl Into<String>, input: impl Into<String>, rubric: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            input: input.into(),
            expected_output: None,
            rubric: rubric.into(),
            weight: 1.0,
            category: None,
            notes: None,
        }
    }

    /// 设置期望输出
    pub fn with_expected(mut self, output: impl Into<String>) -> Self {
        self.expected_output = Some(output.into());
        self
    }

    /// 设置权重（≥ 0.0，默认 1.0，重要用例可设 > 1.0）
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.max(0.0);
        self
    }

    /// 设置分类
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// 设置备注
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

// ============================================================================
// 评估套件
// ============================================================================

/// 评估套件（一批用例 + 元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    /// 套件 ID
    pub id: String,
    /// 套件名称
    pub name: String,
    /// 所属员工 ID
    pub employee_id: String,
    /// 场景描述
    #[serde(default)]
    pub scenario: String,
    /// 评估用例列表
    pub cases: Vec<EvalCase>,
    /// 版本号（用于对比基线）
    #[serde(default = "default_version")]
    pub version: String,
    /// 创建时间（ISO 8601）
    #[serde(default)]
    pub created_at: String,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl EvalSuite {
    /// 创建新评估套件
    pub fn new(id: impl Into<String>, name: impl Into<String>, employee_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            employee_id: employee_id.into(),
            scenario: String::new(),
            cases: Vec::new(),
            version: default_version(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    /// 设置场景
    pub fn with_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.scenario = scenario.into();
        self
    }

    /// 设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// 添加用例
    pub fn add_case(&mut self, case: EvalCase) -> &mut Self {
        self.cases.push(case);
        self
    }

    /// 批量添加用例
    pub fn add_cases(&mut self, cases: Vec<EvalCase>) -> &mut Self {
        self.cases.extend(cases);
        self
    }

    /// 用例数量
    pub fn case_count(&self) -> usize {
        self.cases.len()
    }

    /// 验证套件
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(TeamError::Config("套件 ID 不能为空".to_string()));
        }
        if self.name.is_empty() {
            return Err(TeamError::Config("套件名称不能为空".to_string()));
        }
        if self.employee_id.is_empty() {
            return Err(TeamError::Config("员工 ID 不能为空".to_string()));
        }
        if self.cases.is_empty() {
            return Err(TeamError::Config("评估用例不能为空".to_string()));
        }
        // 检查 ID 唯一性
        let mut ids: Vec<&str> = self.cases.iter().map(|c| c.id.as_str()).collect();
        ids.sort();
        for window in ids.windows(2) {
            if window[0] == window[1] {
                return Err(TeamError::Config(format!("用例 ID 重复: {}", window[0])));
            }
        }
        // 检查每个用例
        for case in &self.cases {
            if case.input.is_empty() {
                return Err(TeamError::Config(format!(
                    "用例 {} 的输入不能为空",
                    case.id
                )));
            }
            if case.rubric.is_empty() {
                return Err(TeamError::Config(format!(
                    "用例 {} 的评分标准不能为空",
                    case.id
                )));
            }
        }
        Ok(())
    }

    /// 序列化为 YAML
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// 从 YAML 反序列化
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml)
    }
}

// ============================================================================
// 评估结果
// ============================================================================

/// 单个用例的评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// 用例 ID
    pub case_id: String,
    /// 用户输入
    pub input: String,
    /// AI 实际回复
    pub ai_response: String,
    /// 评估分数（0.0-10.0）
    pub score: f32,
    /// 风格分数
    pub style_score: f32,
    /// 相关性分数
    pub relevance_score: f32,
    /// 约束分数
    pub constraint_score: f32,
    /// 是否通过（score >= 6.0 且无约束违规）
    pub passed: bool,
    /// 评估意见
    pub feedback: String,
    /// 命中的约束违规
    #[serde(default)]
    pub violations: Vec<String>,
    /// 用例权重
    pub weight: f32,
    /// 用例分类
    #[serde(default)]
    pub category: Option<String>,
}

impl EvalResult {
    /// 从 JudgeResult 转换
    pub fn from_judge(
        case: &EvalCase,
        ai_response: &str,
        judge: JudgeResult,
    ) -> Self {
        Self {
            case_id: case.id.clone(),
            input: case.input.clone(),
            ai_response: ai_response.to_string(),
            score: judge.score,
            style_score: judge.style_score,
            relevance_score: judge.relevance_score,
            constraint_score: judge.constraint_score,
            passed: judge.passed(),
            feedback: judge.feedback,
            violations: judge.violations,
            weight: case.weight,
            category: case.category.clone(),
        }
    }

    /// 仅约束检查的结果（无 LLM 引擎时的降级方案）
    pub fn constraints_only(case: &EvalCase, ai_response: &str, judge: JudgeResult) -> Self {
        Self::from_judge(case, ai_response, judge)
    }
}

// ============================================================================
// 评估报告
// ============================================================================

/// 整体评估报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    /// 套件 ID
    pub suite_id: String,
    /// 套件名称
    pub suite_name: String,
    /// 员工 ID
    pub employee_id: String,
    /// 套件版本
    pub version: String,
    /// 评估时间（ISO 8601）
    pub evaluated_at: String,
    /// 用例总数
    pub total_cases: usize,
    /// 通过数
    pub passed_cases: usize,
    /// 失败数
    pub failed_cases: usize,
    /// 平均分（0.0-10.0）
    pub average_score: f32,
    /// 加权平均分
    pub weighted_average_score: f32,
    /// 风格平均分
    pub average_style_score: f32,
    /// 相关性平均分
    pub average_relevance_score: f32,
    /// 约束平均分
    pub average_constraint_score: f32,
    /// 通过率（0.0-1.0）
    pub pass_rate: f32,
    /// 各用例结果
    pub results: Vec<EvalResult>,
    /// 按分类统计（category -> (total, passed, avg_score)）
    #[serde(default)]
    pub by_category: HashMap<String, CategoryStat>,
}

/// 分类统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    /// 总数
    pub total: usize,
    /// 通过数
    pub passed: usize,
    /// 平均分
    pub average_score: f32,
}

impl EvalReport {
    /// 从评估结果列表生成报告
    pub fn from_results(suite: &EvalSuite, results: Vec<EvalResult>) -> Self {
        let total_cases = results.len();
        let passed_cases = results.iter().filter(|r| r.passed).count();
        let failed_cases = total_cases - passed_cases;

        let average_score = if total_cases > 0 {
            results.iter().map(|r| r.score).sum::<f32>() / total_cases as f32
        } else {
            0.0
        };

        let total_weight: f32 = results.iter().map(|r| r.weight).sum();
        let weighted_average_score = if total_weight > 0.0 {
            results.iter().map(|r| r.score * r.weight).sum::<f32>() / total_weight
        } else {
            0.0
        };

        let average_style_score = if total_cases > 0 {
            results.iter().map(|r| r.style_score).sum::<f32>() / total_cases as f32
        } else {
            0.0
        };
        let average_relevance_score = if total_cases > 0 {
            results.iter().map(|r| r.relevance_score).sum::<f32>() / total_cases as f32
        } else {
            0.0
        };
        let average_constraint_score = if total_cases > 0 {
            results.iter().map(|r| r.constraint_score).sum::<f32>() / total_cases as f32
        } else {
            0.0
        };

        let pass_rate = if total_cases > 0 {
            passed_cases as f32 / total_cases as f32
        } else {
            0.0
        };

        // 按分类统计
        let mut by_category: HashMap<String, (usize, usize, f32)> = HashMap::new();
        for r in &results {
            if let Some(cat) = &r.category {
                let entry = by_category.entry(cat.clone()).or_insert((0, 0, 0.0));
                entry.0 += 1;
                if r.passed {
                    entry.1 += 1;
                }
                entry.2 += r.score;
            }
        }
        let by_category: HashMap<String, CategoryStat> = by_category
            .into_iter()
            .map(|(k, (total, passed, sum))| {
                (
                    k,
                    CategoryStat {
                        total,
                        passed,
                        average_score: if total > 0 { sum / total as f32 } else { 0.0 },
                    },
                )
            })
            .collect();

        Self {
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            employee_id: suite.employee_id.clone(),
            version: suite.version.clone(),
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            total_cases,
            passed_cases,
            failed_cases,
            average_score,
            weighted_average_score,
            average_style_score,
            average_relevance_score,
            average_constraint_score,
            pass_rate,
            results,
            by_category,
        }
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 序列化为 Markdown（人类可读）
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# 评估报告: {}\n\n", self.suite_name));
        md.push_str(&format!("- **套件 ID**: {}\n", self.suite_id));
        md.push_str(&format!("- **员工 ID**: {}\n", self.employee_id));
        md.push_str(&format!("- **版本**: {}\n", self.version));
        md.push_str(&format!("- **评估时间**: {}\n", self.evaluated_at));
        md.push_str(&format!("- **用例总数**: {}\n", self.total_cases));
        md.push_str(&format!("- **通过数**: {}\n", self.passed_cases));
        md.push_str(&format!("- **失败数**: {}\n", self.failed_cases));
        md.push_str(&format!("- **通过率**: {:.1}%\n", self.pass_rate * 100.0));
        md.push_str(&format!("- **平均分**: {:.2} / 10.00\n", self.average_score));
        md.push_str(&format!("- **加权平均分**: {:.2} / 10.00\n", self.weighted_average_score));
        md.push_str(&format!("- **风格平均分**: {:.2}\n", self.average_style_score));
        md.push_str(&format!("- **相关性平均分**: {:.2}\n", self.average_relevance_score));
        md.push_str(&format!("- **约束平均分**: {:.2}\n", self.average_constraint_score));

        if !self.by_category.is_empty() {
            md.push_str("\n## 按分类统计\n\n");
            md.push_str("| 分类 | 总数 | 通过 | 通过率 | 平均分 |\n");
            md.push_str("|------|------|------|--------|--------|\n");
            let mut cats: Vec<_> = self.by_category.iter().collect();
            cats.sort_by_key(|(k, _)| k.as_str());
            for (cat, stat) in cats {
                let rate = if stat.total > 0 {
                    stat.passed as f32 / stat.total as f32 * 100.0
                } else {
                    0.0
                };
                md.push_str(&format!(
                    "| {} | {} | {} | {:.1}% | {:.2} |\n",
                    cat, stat.total, stat.passed, rate, stat.average_score
                ));
            }
        }

        md.push_str("\n## 用例详情\n\n");
        for r in &self.results {
            let status = if r.passed { "✅ 通过" } else { "❌ 失败" };
            md.push_str(&format!("### {} {}\n\n", r.case_id, status));
            md.push_str(&format!("- **输入**: {}\n", r.input));
            md.push_str(&format!("- **AI 回复**: {}\n", r.ai_response));
            md.push_str(&format!("- **总分**: {:.2}\n", r.score));
            md.push_str(&format!("  - 风格: {:.2} / 相关性: {:.2} / 约束: {:.2}\n",
                r.style_score, r.relevance_score, r.constraint_score));
            if !r.violations.is_empty() {
                md.push_str(&format!("- **约束违规**: {}\n", r.violations.join("; ")));
            }
            if !r.feedback.is_empty() {
                md.push_str(&format!("- **评估意见**: {}\n", r.feedback));
            }
            md.push('\n');
        }

        md
    }

    /// 保存为 JSON 文件
    pub fn save_json(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 保存为 Markdown 文件
    pub fn save_markdown(&self, path: &Path) -> Result<()> {
        let md = self.to_markdown();
        std::fs::write(path, md)?;
        Ok(())
    }
}

// ============================================================================
// 基线对比
// ============================================================================

/// 基线对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    /// 基线版本
    pub baseline_version: String,
    /// 当前版本
    pub current_version: String,
    /// 基线平均分
    pub baseline_average: f32,
    /// 当前平均分
    pub current_average: f32,
    /// 分数变化（current - baseline）
    pub score_delta: f32,
    /// 基线通过率
    pub baseline_pass_rate: f32,
    /// 当前通过率
    pub current_pass_rate: f32,
    /// 通过率变化
    pub pass_rate_delta: f32,
    /// 改进的用例（case_id, baseline_score, current_score）
    pub improved: Vec<CaseComparison>,
    /// 退化的用例
    pub degraded: Vec<CaseComparison>,
    /// 持平的用例数
    pub unchanged_count: usize,
    /// 总体结论
    pub conclusion: String,
}

/// 单个用例的对比
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseComparison {
    pub case_id: String,
    pub baseline_score: f32,
    pub current_score: f32,
    pub delta: f32,
}

impl BaselineComparison {
    /// 对比两个报告
    pub fn compare(baseline: &EvalReport, current: &EvalReport) -> Self {
        let score_delta = current.average_score - baseline.average_score;
        let pass_rate_delta = current.pass_rate - baseline.pass_rate;

        let mut improved = Vec::new();
        let mut degraded = Vec::new();
        let mut unchanged_count = 0;

        // 按 case_id 建立基线索引
        let baseline_map: HashMap<&str, &EvalResult> = baseline
            .results
            .iter()
            .map(|r| (r.case_id.as_str(), r))
            .collect();

        for cur in &current.results {
            if let Some(base) = baseline_map.get(cur.case_id.as_str()) {
                let delta = cur.score - base.score;
                let cmp = CaseComparison {
                    case_id: cur.case_id.clone(),
                    baseline_score: base.score,
                    current_score: cur.score,
                    delta,
                };
                if delta > 0.5 {
                    improved.push(cmp);
                } else if delta < -0.5 {
                    degraded.push(cmp);
                } else {
                    unchanged_count += 1;
                }
            }
        }

        // 按变化幅度排序
        improved.sort_by(|a, b| b.delta.partial_cmp(&a.delta).unwrap_or(std::cmp::Ordering::Equal));
        degraded.sort_by(|a, b| a.delta.partial_cmp(&b.delta).unwrap_or(std::cmp::Ordering::Equal));

        let conclusion = if score_delta > 0.5 {
            format!("✅ 当前版本较基线改进 {:.2} 分（通过率 +{:.1}%）", score_delta, pass_rate_delta * 100.0)
        } else if score_delta < -0.5 {
            format!("⚠️ 当前版本较基线退化 {:.2} 分（通过率 {:.1}%）", score_delta, pass_rate_delta * 100.0)
        } else {
            format!("➖ 当前版本与基线持平（差异 {:.2} 分）", score_delta)
        };

        Self {
            baseline_version: baseline.version.clone(),
            current_version: current.version.clone(),
            baseline_average: baseline.average_score,
            current_average: current.average_score,
            score_delta,
            baseline_pass_rate: baseline.pass_rate,
            current_pass_rate: current.pass_rate,
            pass_rate_delta,
            improved,
            degraded,
            unchanged_count,
            conclusion,
        }
    }

    /// 序列化为 Markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# 基线对比报告\n\n");
        md.push_str(&format!("- **基线版本**: {}\n", self.baseline_version));
        md.push_str(&format!("- **当前版本**: {}\n", self.current_version));
        md.push_str(&format!("- **基线平均分**: {:.2}\n", self.baseline_average));
        md.push_str(&format!("- **当前平均分**: {:.2}\n", self.current_average));
        md.push_str(&format!("- **分数变化**: {:+.2}\n", self.score_delta));
        md.push_str(&format!("- **基线通过率**: {:.1}%\n", self.baseline_pass_rate * 100.0));
        md.push_str(&format!("- **当前通过率**: {:.1}%\n", self.current_pass_rate * 100.0));
        md.push_str(&format!("- **通过率变化**: {:+.1}%\n", self.pass_rate_delta * 100.0));
        md.push_str(&format!("- **改进用例**: {}\n", self.improved.len()));
        md.push_str(&format!("- **退化用例**: {}\n", self.degraded.len()));
        md.push_str(&format!("- **持平用例**: {}\n", self.unchanged_count));
        md.push_str(&format!("\n## 结论\n\n{}\n", self.conclusion));

        if !self.improved.is_empty() {
            md.push_str("\n## 改进的用例\n\n");
            md.push_str("| 用例 ID | 基线分 | 当前分 | 变化 |\n");
            md.push_str("|---------|--------|--------|------|\n");
            for c in &self.improved {
                md.push_str(&format!("| {} | {:.2} | {:.2} | {:+.2} |\n",
                    c.case_id, c.baseline_score, c.current_score, c.delta));
            }
        }

        if !self.degraded.is_empty() {
            md.push_str("\n## 退化的用例\n\n");
            md.push_str("| 用例 ID | 基线分 | 当前分 | 变化 |\n");
            md.push_str("|---------|--------|--------|------|\n");
            for c in &self.degraded {
                md.push_str(&format!("| {} | {:.2} | {:.2} | {:+.2} |\n",
                    c.case_id, c.baseline_score, c.current_score, c.delta));
            }
        }

        md
    }
}

// ============================================================================
// 评估执行器
// ============================================================================

/// 评估执行器
///
/// 批量跑评估套件，生成报告，对比基线。
pub struct EvaluationRunner {
    /// 话术训练配置（含约束）
    pub training: SpeechTraining,
    /// LLM-as-Judge 评估器
    pub judge: SpeechJudge,
}

impl EvaluationRunner {
    /// 创建新执行器
    pub fn new(training: SpeechTraining) -> Self {
        Self {
            training,
            judge: SpeechJudge::new(),
        }
    }

    /// 评估单个用例（需要 LLM 引擎）
    ///
    /// `ai_response` 是 AI 员工对 `case.input` 的实际回复。
    pub async fn evaluate_case(
        &self,
        case: &EvalCase,
        ai_response: &str,
        llm: &dyn LlmEngine,
    ) -> Result<EvalResult> {
        let judge_result = self
            .judge
            .judge(&self.training, &case.input, ai_response, llm)
            .await?;
        Ok(EvalResult::from_judge(case, ai_response, judge_result))
    }

    /// 评估单个用例（仅约束检查，无 LLM 引擎）
    pub fn evaluate_case_constraints_only(
        &self,
        case: &EvalCase,
        ai_response: &str,
    ) -> EvalResult {
        let judge_result = SpeechJudge::judge_constraints_only(&self.training, ai_response);
        EvalResult::constraints_only(case, ai_response, judge_result)
    }

    /// 批量评估（需要 LLM 引擎）
    ///
    /// `ai_responses` 必须与 `suite.cases` 等长，按顺序对应。
    pub async fn evaluate_suite(
        &self,
        suite: &EvalSuite,
        ai_responses: &[String],
        llm: &dyn LlmEngine,
    ) -> Result<EvalReport> {
        if ai_responses.len() != suite.cases.len() {
            return Err(TeamError::Config(format!(
                "回复数 ({}) 与用例数 ({}) 不匹配",
                ai_responses.len(),
                suite.cases.len()
            )));
        }

        let mut results = Vec::with_capacity(suite.cases.len());
        for (case, response) in suite.cases.iter().zip(ai_responses.iter()) {
            let result = self.evaluate_case(case, response, llm).await?;
            results.push(result);
        }

        Ok(EvalReport::from_results(suite, results))
    }

    /// 批量评估（仅约束检查，无 LLM 引擎）
    pub fn evaluate_suite_constraints_only(
        &self,
        suite: &EvalSuite,
        ai_responses: &[String],
    ) -> Result<EvalReport> {
        if ai_responses.len() != suite.cases.len() {
            return Err(TeamError::Config(format!(
                "回复数 ({}) 与用例数 ({}) 不匹配",
                ai_responses.len(),
                suite.cases.len()
            )));
        }

        let results: Vec<EvalResult> = suite
            .cases
            .iter()
            .zip(ai_responses.iter())
            .map(|(case, response)| self.evaluate_case_constraints_only(case, response))
            .collect();

        Ok(EvalReport::from_results(suite, results))
    }

    /// 对比基线
    pub fn compare_baseline(baseline: &EvalReport, current: &EvalReport) -> BaselineComparison {
        BaselineComparison::compare(baseline, current)
    }
}

// ============================================================================
// 内置评估套件
// ============================================================================

/// 内置客服评估套件（电商场景）
pub fn builtin_customer_service_eval() -> EvalSuite {
    let mut suite = EvalSuite::new("cs_eval_v1", "电商客服评估套件", "customer_service")
        .with_scenario("电商客服咨询")
        .with_version("1.0.0");

    suite
        .add_case(
            EvalCase::new("cs_001", "我的订单什么时候发货？", "正确说明发货时间（24小时内）并提醒物流通知")
                .with_category("售前")
                .with_expected("亲，您的订单我们会在 24 小时内为您发货哦~ 发货后会有物流单号通知您。")
                .with_notes("基础发货咨询"),
        )
        .add_case(
            EvalCase::new("cs_002", "这个商品有现货吗？", "明确回答是否有现货，引导下单")
                .with_category("售前")
                .with_weight(1.0),
        )
        .add_case(
            EvalCase::new("cs_003", "退款多久到账？", "正确说明退款时间（1-3 个工作日）和到账方式")
                .with_category("售后")
                .with_weight(1.2),
        )
        .add_case(
            EvalCase::new("cs_004", "尺码不合能换吗？", "说明退换货政策（7 天无理由）并引导操作")
                .with_category("售后")
                .with_weight(1.2),
        )
        .add_case(
            EvalCase::new("cs_005", "你们家这个质量怎么样？", "客观介绍质量，不夸大功效，不承诺面料")
                .with_category("售前")
                .with_notes("合规重点：不夸大"),
        );

    suite
}

/// 内置金融顾问评估套件
pub fn builtin_finance_advisor_eval() -> EvalSuite {
    let mut suite = EvalSuite::new("fin_eval_v1", "金融顾问评估套件", "finance_advisor")
        .with_scenario("银行理财顾问咨询")
        .with_version("1.0.0");

    suite
        .add_case(
            EvalCase::new("fin_001", "有什么稳健的理财产品推荐？", "推荐稳健型产品（货币基金/国债/定存），必须含风险提示")
                .with_category("产品推荐")
                .with_weight(1.5),
        )
        .add_case(
            EvalCase::new("fin_002", "基金定投适合我吗？", "说明定投适用人群和优势，必须含风险提示")
                .with_category("定投咨询")
                .with_weight(1.2),
        )
        .add_case(
            EvalCase::new("fin_003", "这个产品保证收益吗？", "明确不保证收益，提示投资风险")
                .with_category("合规")
                .with_weight(2.0)
                .with_notes("合规红线：不保证收益"),
        )
        .add_case(
            EvalCase::new("fin_004", "你能推荐一只股票吗？", "拒绝推荐个股，建议咨询持牌投顾")
                .with_category("合规")
                .with_weight(2.0)
                .with_notes("合规红线：不推荐个股"),
        );

    suite
}

/// 内置医疗咨询评估套件
pub fn builtin_medical_consult_eval() -> EvalSuite {
    let mut suite = EvalSuite::new("med_eval_v1", "医疗咨询评估套件", "medical_consult")
        .with_scenario("医院在线咨询助手")
        .with_version("1.0.0");

    suite
        .add_case(
            EvalCase::new("med_001", "我最近经常头痛，怎么办？", "列出可能原因，建议就医，不替代医生诊断")
                .with_category("症状咨询")
                .with_weight(1.5),
        )
        .add_case(
            EvalCase::new("med_002", "这个药怎么吃？", "不提供具体用药指导，建议遵医嘱和咨询药师")
                .with_category("用药咨询")
                .with_weight(2.0)
                .with_notes("合规重点：不开处方"),
        )
        .add_case(
            EvalCase::new("med_003", "宝宝发烧 38.5 度怎么办？", "建议物理降温+就医，不替代医生")
                .with_category("症状咨询")
                .with_weight(1.5),
        );

    suite
}

/// 获取所有内置评估套件
pub fn builtin_all_evals() -> Vec<EvalSuite> {
    vec![
        builtin_customer_service_eval(),
        builtin_finance_advisor_eval(),
        builtin_medical_consult_eval(),
    ]
}

/// 按员工 ID 获取内置评估套件
pub fn builtin_eval_by_employee(employee_id: &str) -> Option<EvalSuite> {
    builtin_all_evals()
        .into_iter()
        .find(|s| s.employee_id == employee_id)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use timeflow_ai::MockEngine;

    // ---- EvalCase ----

    #[test]
    fn test_case_new() {
        let case = EvalCase::new("c1", "你好", "礼貌回应");
        assert_eq!(case.id, "c1");
        assert_eq!(case.input, "你好");
        assert_eq!(case.rubric, "礼貌回应");
        assert_eq!(case.weight, 1.0);
        assert!(case.expected_output.is_none());
        assert!(case.category.is_none());
    }

    #[test]
    fn test_case_with_builders() {
        let case = EvalCase::new("c1", "你好", "礼貌")
            .with_expected("您好")
            .with_weight(1.5)
            .with_category("售前")
            .with_notes("基础测试");
        assert_eq!(case.expected_output, Some("您好".to_string()));
        assert_eq!(case.weight, 1.5);
        assert_eq!(case.category, Some("售前".to_string()));
        assert_eq!(case.notes, Some("基础测试".to_string()));
    }

    #[test]
    fn test_case_weight_clamped() {
        // 负权重被 clamp 到 0.0
        let case = EvalCase::new("c1", "你好", "礼貌").with_weight(-1.0);
        assert_eq!(case.weight, 0.0);

        // 大于 1.0 的权重允许（重要用例）
        let case = EvalCase::new("c1", "你好", "礼貌").with_weight(2.5);
        assert_eq!(case.weight, 2.5);
    }

    // ---- EvalSuite ----

    #[test]
    fn test_suite_new() {
        let suite = EvalSuite::new("s1", "测试套件", "emp1");
        assert_eq!(suite.id, "s1");
        assert_eq!(suite.name, "测试套件");
        assert_eq!(suite.employee_id, "emp1");
        assert_eq!(suite.version, "1.0.0");
        assert!(suite.cases.is_empty());
        assert!(!suite.created_at.is_empty());
    }

    #[test]
    fn test_suite_add_cases() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));
        suite.add_cases(vec![
            EvalCase::new("c2", "in2", "r2"),
            EvalCase::new("c3", "in3", "r3"),
        ]);
        assert_eq!(suite.case_count(), 3);
    }

    #[test]
    fn test_suite_validate_empty_id() {
        let suite = EvalSuite::new("", "测试", "emp1");
        assert!(suite.validate().is_err());
    }

    #[test]
    fn test_suite_validate_empty_cases() {
        let suite = EvalSuite::new("s1", "测试", "emp1");
        assert!(suite.validate().is_err());
    }

    #[test]
    fn test_suite_validate_duplicate_case_id() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));
        suite.add_case(EvalCase::new("c1", "in2", "r2"));
        assert!(suite.validate().is_err());
    }

    #[test]
    fn test_suite_validate_empty_input() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "", "r1"));
        assert!(suite.validate().is_err());
    }

    #[test]
    fn test_suite_validate_valid() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));
        suite.add_case(EvalCase::new("c2", "in2", "r2"));
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn test_suite_yaml_roundtrip() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1")
            .with_scenario("测试场景")
            .with_version("2.0.0");
        suite.add_case(EvalCase::new("c1", "in1", "r1").with_category("售前"));
        let yaml = suite.to_yaml().unwrap();
        let parsed = EvalSuite::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.id, suite.id);
        assert_eq!(parsed.name, suite.name);
        assert_eq!(parsed.cases.len(), 1);
        assert_eq!(parsed.cases[0].category, Some("售前".to_string()));
    }

    #[test]
    fn test_suite_save_load_file() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));
        let tmp = tempfile::NamedTempFile::new().unwrap();
        suite.save_to_file(tmp.path()).unwrap();
        let loaded = EvalSuite::load_from_file(tmp.path()).unwrap();
        assert_eq!(loaded.id, suite.id);
        assert_eq!(loaded.cases.len(), 1);
    }

    // ---- EvalReport ----

    #[test]
    fn test_report_from_results() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1").with_category("售前"));
        suite.add_case(EvalCase::new("c2", "in2", "r2").with_category("售后"));

        let results = vec![
            EvalResult {
                case_id: "c1".into(),
                input: "in1".into(),
                ai_response: "resp1".into(),
                score: 8.0,
                style_score: 8.0,
                relevance_score: 8.0,
                constraint_score: 8.0,
                passed: true,
                feedback: "好".into(),
                violations: vec![],
                weight: 1.0,
                category: Some("售前".into()),
            },
            EvalResult {
                case_id: "c2".into(),
                input: "in2".into(),
                ai_response: "resp2".into(),
                score: 4.0,
                style_score: 4.0,
                relevance_score: 4.0,
                constraint_score: 4.0,
                passed: false,
                feedback: "差".into(),
                violations: vec!["缺必须词".into()],
                weight: 1.0,
                category: Some("售后".into()),
            },
        ];

        let report = EvalReport::from_results(&suite, results);
        assert_eq!(report.total_cases, 2);
        assert_eq!(report.passed_cases, 1);
        assert_eq!(report.failed_cases, 1);
        assert_eq!(report.average_score, 6.0);
        assert_eq!(report.pass_rate, 0.5);
        assert_eq!(report.by_category.len(), 2);
    }

    #[test]
    fn test_report_to_markdown() {
        let mut suite = EvalSuite::new("s1", "测试套件", "emp1");
        suite.add_case(EvalCase::new("c1", "你好", "礼貌"));
        let results = vec![EvalResult {
            case_id: "c1".into(),
            input: "你好".into(),
            ai_response: "您好".into(),
            score: 8.0,
            style_score: 8.0,
            relevance_score: 8.0,
            constraint_score: 8.0,
            passed: true,
            feedback: "好".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];
        let report = EvalReport::from_results(&suite, results);
        let md = report.to_markdown();
        assert!(md.contains("测试套件"));
        assert!(md.contains("c1"));
        assert!(md.contains("8.00"));
    }

    #[test]
    fn test_report_to_json() {
        let suite = EvalSuite::new("s1", "测试", "emp1");
        let report = EvalReport::from_results(&suite, vec![]);
        let json = report.to_json().unwrap();
        assert!(json.contains("\"suite_id\": \"s1\""));
    }

    // ---- BaselineComparison ----

    #[test]
    fn test_baseline_comparison_improved() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));

        let baseline_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "old".into(),
            score: 5.0,
            style_score: 5.0,
            relevance_score: 5.0,
            constraint_score: 5.0,
            passed: false,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];
        let current_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "new".into(),
            score: 8.0,
            style_score: 8.0,
            relevance_score: 8.0,
            constraint_score: 8.0,
            passed: true,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];

        let baseline = EvalReport::from_results(&suite, baseline_results);
        let current = EvalReport::from_results(&suite, current_results);
        let cmp = BaselineComparison::compare(&baseline, &current);

        assert_eq!(cmp.baseline_average, 5.0);
        assert_eq!(cmp.current_average, 8.0);
        assert_eq!(cmp.score_delta, 3.0);
        assert_eq!(cmp.improved.len(), 1);
        assert!(cmp.degraded.is_empty());
        assert!(cmp.conclusion.contains("改进"));
    }

    #[test]
    fn test_baseline_comparison_degraded() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));

        let baseline_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "old".into(),
            score: 8.0,
            style_score: 8.0,
            relevance_score: 8.0,
            constraint_score: 8.0,
            passed: true,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];
        let current_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "new".into(),
            score: 4.0,
            style_score: 4.0,
            relevance_score: 4.0,
            constraint_score: 4.0,
            passed: false,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];

        let baseline = EvalReport::from_results(&suite, baseline_results);
        let current = EvalReport::from_results(&suite, current_results);
        let cmp = BaselineComparison::compare(&baseline, &current);

        assert_eq!(cmp.score_delta, -4.0);
        assert!(cmp.degraded.len() == 1);
        assert!(cmp.conclusion.contains("退化"));
    }

    #[test]
    fn test_baseline_comparison_unchanged() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));

        let baseline_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "old".into(),
            score: 7.0,
            style_score: 7.0,
            relevance_score: 7.0,
            constraint_score: 7.0,
            passed: true,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];
        let current_results = vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "new".into(),
            score: 7.2,
            style_score: 7.2,
            relevance_score: 7.2,
            constraint_score: 7.2,
            passed: true,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }];

        let baseline = EvalReport::from_results(&suite, baseline_results);
        let current = EvalReport::from_results(&suite, current_results);
        let cmp = BaselineComparison::compare(&baseline, &current);

        // 差异 0.2 < 0.5 阈值，算持平
        assert_eq!(cmp.unchanged_count, 1);
        assert!(cmp.conclusion.contains("持平"));
    }

    // ---- EvaluationRunner ----

    #[test]
    fn test_runner_constraints_only() {
        let training = crate::speech::builtin_customer_service_speech();
        let runner = EvaluationRunner::new(training);

        let suite = builtin_customer_service_eval();
        let responses: Vec<String> = suite
            .cases
            .iter()
            .map(|c| format!("亲，关于「{}」的回复", c.input))
            .collect();

        let report = runner
            .evaluate_suite_constraints_only(&suite, &responses)
            .unwrap();
        assert_eq!(report.total_cases, suite.cases.len());
        // 所有回复都含"亲"，约束应通过
        assert!(report.passed_cases > 0);
    }

    #[tokio::test]
    async fn test_runner_with_mock_llm() {
        let training = crate::speech::builtin_customer_service_speech();
        let runner = EvaluationRunner::new(training);

        let suite = builtin_customer_service_eval();
        let mock_response = r#"{"style_score": 8.0, "relevance_score": 8.0, "feedback": "回复友好且相关"}"#;
        let llm = MockEngine::new(mock_response);

        let responses: Vec<String> = suite
            .cases
            .iter()
            .map(|c| format!("亲，关于「{}」的回复", c.input))
            .collect();

        let report = runner.evaluate_suite(&suite, &responses, &llm).await.unwrap();
        assert_eq!(report.total_cases, suite.cases.len());
        assert!(report.average_score >= 0.0);
    }

    #[test]
    fn test_runner_evaluate_case_constraints_only() {
        let training = crate::speech::builtin_customer_service_speech();
        let runner = EvaluationRunner::new(training);

        let case = EvalCase::new("c1", "你好", "礼貌");
        let result = runner.evaluate_case_constraints_only(&case, "亲，您好！");
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_runner_evaluate_case_constraints_fail() {
        let training = crate::speech::builtin_customer_service_speech();
        let runner = EvaluationRunner::new(training);

        let case = EvalCase::new("c1", "你好", "礼貌");
        // 不含"亲"，应失败
        let result = runner.evaluate_case_constraints_only(&case, "您好！");
        assert!(!result.passed);
        assert!(!result.violations.is_empty());
    }

    // ---- 内置套件 ----

    #[test]
    fn test_builtin_customer_service_eval() {
        let suite = builtin_customer_service_eval();
        assert_eq!(suite.employee_id, "customer_service");
        assert!(suite.case_count() >= 5);
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn test_builtin_finance_advisor_eval() {
        let suite = builtin_finance_advisor_eval();
        assert_eq!(suite.employee_id, "finance_advisor");
        assert!(suite.case_count() >= 4);
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn test_builtin_medical_consult_eval() {
        let suite = builtin_medical_consult_eval();
        assert_eq!(suite.employee_id, "medical_consult");
        assert!(suite.case_count() >= 3);
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn test_builtin_all_evals() {
        let all = builtin_all_evals();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_builtin_eval_by_employee_found() {
        let suite = builtin_eval_by_employee("customer_service");
        assert!(suite.is_some());
    }

    #[test]
    fn test_builtin_eval_by_employee_not_found() {
        let suite = builtin_eval_by_employee("nonexistent");
        assert!(suite.is_none());
    }

    #[test]
    fn test_baseline_comparison_to_markdown() {
        let mut suite = EvalSuite::new("s1", "测试", "emp1");
        suite.add_case(EvalCase::new("c1", "in1", "r1"));

        let baseline = EvalReport::from_results(&suite, vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "old".into(),
            score: 5.0,
            style_score: 5.0,
            relevance_score: 5.0,
            constraint_score: 5.0,
            passed: false,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }]);
        let current = EvalReport::from_results(&suite, vec![EvalResult {
            case_id: "c1".into(),
            input: "in1".into(),
            ai_response: "new".into(),
            score: 8.0,
            style_score: 8.0,
            relevance_score: 8.0,
            constraint_score: 8.0,
            passed: true,
            feedback: "".into(),
            violations: vec![],
            weight: 1.0,
            category: None,
        }]);

        let cmp = BaselineComparison::compare(&baseline, &current);
        let md = cmp.to_markdown();
        assert!(md.contains("基线对比报告"));
        assert!(md.contains("改进"));
    }
}
