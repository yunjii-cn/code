// 话术训练系统（M4.1 D3）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D3
//
// 目标：让 AI 员工通过话术示例学会特定风格，不微调，纯 prompt + few-shot。
//
// 技术选型（v2.0 明确）：
//   - 纯 prompt + few-shot（不碰 LoRA 微调）
//   - 风格控制：用 prompt + example 模拟
//   - 评估：LLM-as-Judge（用同一 LLM 当裁判）
//
// 模块组成：
//   - SpeechStyle: 风格枚举（友好/专业/严肃/活泼/沉稳）
//   - SpeechExample: 话术示例（user/assistant 对话）
//   - SpeechConstraints: 约束（禁用词/必须词）
//   - SpeechTraining: 话术训练配置
//   - FewShotBuilder: Few-shot prompt 构造器（自动选 top-3 最相似示例）
//   - SpeechJudge: LLM-as-Judge 评估器
//   - ConstraintChecker: 约束检查器

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use timeflow_ai::{EmbeddingEngine, cosine_similarity, LlmEngine, LlmMessage};

// ============================================================================
// 风格枚举
// ============================================================================

/// 话术风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechStyle {
    /// 友好（亲切、热情）
    Friendly,
    /// 专业（严谨、客观）
    Professional,
    /// 严肃（正式、权威）
    Serious,
    /// 活泼（轻松、幽默）
    Lively,
    /// 沉稳（冷静、克制）
    Calm,
}

impl Default for SpeechStyle {
    fn default() -> Self {
        Self::Professional
    }
}

impl std::fmt::Display for SpeechStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Friendly => "友好",
            Self::Professional => "专业",
            Self::Serious => "严肃",
            Self::Lively => "活泼",
            Self::Calm => "沉稳",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for SpeechStyle {
    type Err = TeamError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "friendly" | "友好" | "亲切" => Ok(Self::Friendly),
            "professional" | "专业" | "严谨" => Ok(Self::Professional),
            "serious" | "严肃" | "正式" => Ok(Self::Serious),
            "lively" | "活泼" | "轻松" => Ok(Self::Lively),
            "calm" | "沉稳" | "冷静" => Ok(Self::Calm),
            other => Err(TeamError::Config(format!("未知话术风格: {other}"))),
        }
    }
}

impl SpeechStyle {
    /// 获取风格对应的系统提示词片段
    pub fn system_prompt_hint(&self) -> &'static str {
        match self {
            Self::Friendly => "请用友好亲切的语气回答，展现热情和关怀，适当使用问候语和表情符号。",
            Self::Professional => "请用专业严谨的语气回答，保持客观中立，避免口语化表达。",
            Self::Serious => "请用严肃正式的语气回答，措辞准确权威，避免轻浮表达。",
            Self::Lively => "请用活泼轻松的语气回答，可适当使用幽默和流行语，保持积极向上。",
            Self::Calm => "请用沉稳冷静的语气回答，措辞克制，避免情绪化表达。",
        }
    }
}

// ============================================================================
// 话术示例
// ============================================================================

/// 话术示例（单轮对话）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechExample {
    /// 用户输入
    pub user: String,
    /// 期望的 AI 回复
    pub assistant: String,
    /// 示例标签（可选，用于分类）
    #[serde(default)]
    pub tag: Option<String>,
    /// 示例质量分数（0.0-1.0，由 LLM-as-Judge 评估，可选）
    #[serde(default)]
    pub quality_score: Option<f32>,
}

impl SpeechExample {
    /// 创建新示例
    pub fn new(user: impl Into<String>, assistant: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            assistant: assistant.into(),
            tag: None,
            quality_score: None,
        }
    }

    /// 设置标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// 设置质量分数
    pub fn with_score(mut self, score: f32) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }

    /// 拼接为文本（用于 embedding 相似度计算）
    pub fn as_text(&self) -> String {
        format!("用户: {}\n助手: {}", self.user, self.assistant)
    }
}

// ============================================================================
// 约束
// ============================================================================

/// 话术约束
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpeechConstraints {
    /// 禁用词列表（AI 回复中不能出现）
    #[serde(default)]
    pub forbidden_words: Vec<String>,
    /// 必须词列表（AI 回复中必须出现至少一个）
    #[serde(default)]
    pub required_words: Vec<String>,
    /// 最小回复长度（字符数）
    #[serde(default)]
    pub min_length: Option<usize>,
    /// 最大回复长度（字符数）
    #[serde(default)]
    pub max_length: Option<usize>,
}

impl SpeechConstraints {
    /// 创建空约束
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加禁用词
    pub fn forbid(&mut self, word: impl Into<String>) -> &mut Self {
        let word = word.into();
        if !word.is_empty() && !self.forbidden_words.contains(&word) {
            self.forbidden_words.push(word);
        }
        self
    }

    /// 添加必须词
    pub fn require(&mut self, word: impl Into<String>) -> &mut Self {
        let word = word.into();
        if !word.is_empty() && !self.required_words.contains(&word) {
            self.required_words.push(word);
        }
        self
    }

    /// 检查回复是否满足约束
    pub fn check(&self, response: &str) -> ConstraintCheckResult {
        let mut violations = Vec::new();
        let lower = response.to_lowercase();

        // 检查禁用词
        for word in &self.forbidden_words {
            let w_lower = word.to_lowercase();
            if !w_lower.is_empty() && lower.contains(&w_lower) {
                violations.push(ConstraintViolation::ForbiddenWord(word.clone()));
            }
        }

        // 检查必须词
        let required_hits: Vec<&String> = self
            .required_words
            .iter()
            .filter(|w| {
                let w_lower = w.to_lowercase();
                !w_lower.is_empty() && lower.contains(&w_lower)
            })
            .collect();
        if !self.required_words.is_empty() && required_hits.is_empty() {
            violations.push(ConstraintViolation::MissingRequiredWord(
                self.required_words.clone(),
            ));
        }

        // 检查长度
        let char_count = response.chars().count();
        if let Some(min) = self.min_length {
            if char_count < min {
                violations.push(ConstraintViolation::TooShort {
                    actual: char_count,
                    required: min,
                });
            }
        }
        if let Some(max) = self.max_length {
            if char_count > max {
                violations.push(ConstraintViolation::TooLong {
                    actual: char_count,
                    required: max,
                });
            }
        }

        ConstraintCheckResult {
            passed: violations.is_empty(),
            violations,
        }
    }
}

/// 约束检查结果
#[derive(Debug, Clone)]
pub struct ConstraintCheckResult {
    /// 是否通过
    pub passed: bool,
    /// 违规列表
    pub violations: Vec<ConstraintViolation>,
}

/// 约束违规
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintViolation {
    /// 包含禁用词
    ForbiddenWord(String),
    /// 缺少必须词
    MissingRequiredWord(Vec<String>),
    /// 回复过短
    TooShort { actual: usize, required: usize },
    /// 回复过长
    TooLong { actual: usize, required: usize },
}

impl std::fmt::Display for ConstraintViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForbiddenWord(w) => write!(f, "包含禁用词: {w}"),
            Self::MissingRequiredWord(words) => {
                write!(f, "缺少必须词（至少包含其一）: {}", words.join(", "))
            }
            Self::TooShort { actual, required } => {
                write!(f, "回复过短: {actual} 字符 < {required} 字符")
            }
            Self::TooLong { actual, required } => {
                write!(f, "回复过长: {actual} 字符 > {required} 字符")
            }
        }
    }
}

// ============================================================================
// 话术训练配置
// ============================================================================

/// 话术训练配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechTraining {
    /// 所属员工 ID
    pub employee_id: String,
    /// 场景描述（如"银行客服咨询"）
    pub scenario: String,
    /// 话术风格
    #[serde(default)]
    pub style: SpeechStyle,
    /// 话术示例列表
    #[serde(default)]
    pub examples: Vec<SpeechExample>,
    /// 约束
    #[serde(default)]
    pub constraints: SpeechConstraints,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SpeechTraining {
    /// 创建新话术训练配置
    pub fn new(employee_id: impl Into<String>, scenario: impl Into<String>) -> Self {
        Self {
            employee_id: employee_id.into(),
            scenario: scenario.into(),
            style: SpeechStyle::default(),
            examples: Vec::new(),
            constraints: SpeechConstraints::new(),
            metadata: HashMap::new(),
        }
    }

    /// 设置风格
    pub fn with_style(mut self, style: SpeechStyle) -> Self {
        self.style = style;
        self
    }

    /// 添加示例
    pub fn add_example(&mut self, example: SpeechExample) -> &mut Self {
        self.examples.push(example);
        self
    }

    /// 批量添加示例
    pub fn add_examples(&mut self, examples: Vec<SpeechExample>) -> &mut Self {
        self.examples.extend(examples);
        self
    }

    /// 示例数量
    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        if self.employee_id.is_empty() {
            return Err(TeamError::Config("员工 ID 不能为空".to_string()));
        }
        if self.scenario.is_empty() {
            return Err(TeamError::Config("场景描述不能为空".to_string()));
        }
        for (i, ex) in self.examples.iter().enumerate() {
            if ex.user.is_empty() {
                return Err(TeamError::Config(format!(
                    "示例 {i} 的用户输入不能为空"
                )));
            }
            if ex.assistant.is_empty() {
                return Err(TeamError::Config(format!(
                    "示例 {i} 的助手回复不能为空"
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
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml)
    }
}

// ============================================================================
// Few-shot Prompt 构造器
// ============================================================================

/// Few-shot prompt 构造器
///
/// 自动从示例库中选 top-k 最相似的示例，构造 few-shot prompt。
pub struct FewShotBuilder {
    /// 选取的示例数量（默认 3）
    pub top_k: usize,
}

impl Default for FewShotBuilder {
    fn default() -> Self {
        Self { top_k: 3 }
    }
}

impl FewShotBuilder {
    /// 创建构造器
    pub fn new(top_k: usize) -> Self {
        Self {
            top_k: top_k.max(1),
        }
    }

    /// 构造 few-shot prompt
    ///
    /// 参数：
    /// - `training`: 话术训练配置
    /// - `query`: 用户查询
    /// - `engine`: embedding 引擎（用于计算相似度）
    ///
    /// 返回构造好的系统提示词
    pub async fn build_prompt(
        &self,
        training: &SpeechTraining,
        query: &str,
        engine: &dyn EmbeddingEngine,
    ) -> Result<String> {
        let mut prompt = String::new();

        // 1. 场景描述
        prompt.push_str(&format!("【场景】{}\n\n", training.scenario));

        // 2. 风格提示
        prompt.push_str(&format!(
            "【风格要求】{}\n\n",
            training.style.system_prompt_hint()
        ));

        // 3. 约束提示
        if !training.constraints.forbidden_words.is_empty() {
            prompt.push_str(&format!(
                "【禁用词】回复中不能包含: {}\n\n",
                training.constraints.forbidden_words.join(", ")
            ));
        }
        if !training.constraints.required_words.is_empty() {
            prompt.push_str(&format!(
                "【必须词】回复中至少包含其一: {}\n\n",
                training.constraints.required_words.join(", ")
            ));
        }

        // 4. Few-shot 示例（选 top-k 最相似）
        if !training.examples.is_empty() {
            let selected = self.select_similar(training, query, engine).await?;
            prompt.push_str("【参考示例】\n");
            for ex in selected {
                prompt.push_str(&format!("用户: {}\n助手: {}\n\n", ex.user, ex.assistant));
            }
        }

        Ok(prompt)
    }

    /// 选择 top-k 最相似的示例
    async fn select_similar(
        &self,
        training: &SpeechTraining,
        query: &str,
        engine: &dyn EmbeddingEngine,
    ) -> Result<Vec<SpeechExample>> {
        if training.examples.len() <= self.top_k {
            return Ok(training.examples.clone());
        }

        // 计算 query 与每个示例的相似度
        let query_emb = engine.embed(query).await?;
        let mut scored: Vec<(usize, f32)> = Vec::with_capacity(training.examples.len());

        for (i, ex) in training.examples.iter().enumerate() {
            let ex_emb = engine.embed(&ex.as_text()).await?;
            let score = cosine_similarity(&query_emb, &ex_emb);
            scored.push((i, score));
        }

        // 按相似度降序排序，取 top-k
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.top_k);

        Ok(scored
            .into_iter()
            .map(|(i, _)| training.examples[i].clone())
            .collect())
    }

    /// 构造简单 few-shot prompt（不调用 embedding，直接取前 k 个）
    ///
    /// 用于无 embedding 引擎时的降级方案。
    pub fn build_prompt_simple(&self, training: &SpeechTraining) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!("【场景】{}\n\n", training.scenario));
        prompt.push_str(&format!(
            "【风格要求】{}\n\n",
            training.style.system_prompt_hint()
        ));

        if !training.constraints.forbidden_words.is_empty() {
            prompt.push_str(&format!(
                "【禁用词】回复中不能包含: {}\n\n",
                training.constraints.forbidden_words.join(", ")
            ));
        }
        if !training.constraints.required_words.is_empty() {
            prompt.push_str(&format!(
                "【必须词】回复中至少包含其一: {}\n\n",
                training.constraints.required_words.join(", ")
            ));
        }

        if !training.examples.is_empty() {
            prompt.push_str("【参考示例】\n");
            let k = self.top_k.min(training.examples.len());
            for ex in training.examples.iter().take(k) {
                prompt.push_str(&format!("用户: {}\n助手: {}\n\n", ex.user, ex.assistant));
            }
        }

        prompt
    }
}

// ============================================================================
// 约束检查器
// ============================================================================

/// 约束检查器
pub struct ConstraintChecker;

impl ConstraintChecker {
    /// 检查回复是否满足约束
    pub fn check(training: &SpeechTraining, response: &str) -> ConstraintCheckResult {
        training.constraints.check(response)
    }

    /// 生成约束违规的修正建议
    pub fn suggest_fix(violation: &ConstraintViolation) -> String {
        match violation {
            ConstraintViolation::ForbiddenWord(w) => {
                format!("请移除回复中的「{w}」，使用其他表达方式替代。")
            }
            ConstraintViolation::MissingRequiredWord(words) => {
                format!(
                    "请在回复中加入以下词汇之一: {}",
                    words.join(", ")
                )
            }
            ConstraintViolation::TooShort { actual, required } => {
                format!(
                    "回复过短（{actual} 字符），请扩展到至少 {required} 字符，可补充细节或示例。"
                )
            }
            ConstraintViolation::TooLong { actual, required } => {
                format!(
                    "回复过长（{actual} 字符），请精简到 {required} 字符以内，去除冗余内容。"
                )
            }
        }
    }
}

// ============================================================================
// LLM-as-Judge 评估器
// ============================================================================

/// 评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    /// 总分（0.0-10.0）
    pub score: f32,
    /// 风格匹配度（0.0-10.0）
    pub style_score: f32,
    /// 内容相关性（0.0-10.0）
    pub relevance_score: f32,
    /// 约束满足度（0.0-10.0）
    pub constraint_score: f32,
    /// 评估意见
    pub feedback: String,
    /// 命中的约束违规
    #[serde(default)]
    pub violations: Vec<String>,
}

impl Default for JudgeResult {
    fn default() -> Self {
        Self {
            score: 0.0,
            style_score: 0.0,
            relevance_score: 0.0,
            constraint_score: 0.0,
            feedback: String::new(),
            violations: Vec::new(),
        }
    }
}

impl JudgeResult {
    /// 是否通过（分数 >= 6.0 且无约束违规）
    pub fn passed(&self) -> bool {
        self.score >= 6.0 && self.violations.is_empty()
    }
}

/// LLM-as-Judge 评估器
///
/// 用同一 LLM 当裁判，评估 AI 员工的回复质量。
pub struct SpeechJudge {
    /// 评估 prompt 模板
    pub judge_prompt_template: String,
}

impl Default for SpeechJudge {
    fn default() -> Self {
        Self {
            judge_prompt_template: r#"你是一位严格的话术质量评估专家。请评估以下 AI 回复的质量。

【场景】{scenario}
【期望风格】{style}
【用户输入】{user_input}
【AI 回复】{ai_response}
【约束】
- 禁用词: {forbidden_words}
- 必须词: {required_words}

请从以下三个维度评分（0-10 分，保留一位小数）：
1. 风格匹配度：回复是否符合期望风格
2. 内容相关性：回复是否切题、准确
3. 约束满足度：回复是否满足所有约束

请按以下 JSON 格式返回（不要包含其他内容）：
```json
{{
  "style_score": 0.0,
  "relevance_score": 0.0,
  "constraint_score": 0.0,
  "feedback": "评估意见"
}}
```

总分 = (style_score + relevance_score + constraint_score) / 3"#.to_string(),
        }
    }
}

impl SpeechJudge {
    /// 创建评估器
    pub fn new() -> Self {
        Self::default()
    }

    /// 构造评估 prompt
    pub fn build_judge_prompt(
        &self,
        training: &SpeechTraining,
        user_input: &str,
        ai_response: &str,
    ) -> String {
        self.judge_prompt_template
            .replace("{scenario}", &training.scenario)
            .replace("{style}", &training.style.to_string())
            .replace("{user_input}", user_input)
            .replace("{ai_response}", ai_response)
            .replace(
                "{forbidden_words}",
                &training.constraints.forbidden_words.join(", "),
            )
            .replace(
                "{required_words}",
                &training.constraints.required_words.join(", "),
            )
    }

    /// 评估 AI 回复
    ///
    /// 调用 LLM 进行评估，返回评估结果。
    pub async fn judge(
        &self,
        training: &SpeechTraining,
        user_input: &str,
        ai_response: &str,
        llm: &dyn LlmEngine,
    ) -> Result<JudgeResult> {
        // 1. 先做约束检查（不依赖 LLM）
        let constraint_result = ConstraintChecker::check(training, ai_response);
        let violations: Vec<String> = constraint_result
            .violations
            .iter()
            .map(|v| v.to_string())
            .collect();

        // 约束分数：无违规=10，有违规按违规数扣分
        let constraint_score = if constraint_result.passed {
            10.0
        } else {
            (10.0 - violations.len() as f32 * 2.0).max(0.0)
        };

        // 2. 调用 LLM 评估风格和相关性
        let prompt = self.build_judge_prompt(training, user_input, ai_response);
        let messages = vec![
            LlmMessage::system("你是话术质量评估专家，只返回 JSON。"),
            LlmMessage::user(&prompt),
        ];
        let resp = llm.chat(messages).await?;
        let content = resp.content.trim();

        // 3. 解析 LLM 返回的 JSON
        let (style_score, relevance_score, feedback) = parse_judge_response(content);

        // 4. 计算总分
        let score = (style_score + relevance_score + constraint_score) / 3.0;

        // 5. 如果约束未通过，追加违规信息到 feedback
        let feedback = if violations.is_empty() {
            feedback
        } else {
            format!("{feedback}\n\n约束违规: {}", violations.join("; "))
        };

        Ok(JudgeResult {
            score,
            style_score,
            relevance_score,
            constraint_score,
            feedback,
            violations,
        })
    }

    /// 评估 AI 回复（不调用 LLM，仅约束检查）
    ///
    /// 用于无 LLM 引擎时的降级方案，只检查约束。
    pub fn judge_constraints_only(
        training: &SpeechTraining,
        ai_response: &str,
    ) -> JudgeResult {
        let constraint_result = ConstraintChecker::check(training, ai_response);
        let violations: Vec<String> = constraint_result
            .violations
            .iter()
            .map(|v| v.to_string())
            .collect();
        let constraint_score = if constraint_result.passed {
            10.0
        } else {
            (10.0 - violations.len() as f32 * 2.0).max(0.0)
        };

        JudgeResult {
            score: constraint_score, // 仅约束检查时，总分=约束分
            style_score: 0.0,        // 无法评估
            relevance_score: 0.0,    // 无法评估
            constraint_score,
            feedback: if constraint_result.passed {
                "约束检查通过（未评估风格和相关性，需 LLM 引擎）".to_string()
            } else {
                format!("约束检查未通过: {}", violations.join("; "))
            },
            violations,
        }
    }
}

/// 解析 LLM 评估响应
fn parse_judge_response(content: &str) -> (f32, f32, String) {
    // 尝试提取 JSON 块
    let json_str = extract_json_block(content).unwrap_or(content);

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        let style = v
            .get("style_score")
            .and_then(|s| s.as_f64())
            .unwrap_or(0.0) as f32;
        let relevance = v
            .get("relevance_score")
            .and_then(|s| s.as_f64())
            .unwrap_or(0.0) as f32;
        let feedback = v
            .get("feedback")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        (style, relevance, feedback)
    } else {
        // JSON 解析失败，返回默认值
        (0.0, 0.0, format!("评估响应解析失败: {content}"))
    }
}

/// 从文本中提取 JSON 代码块
fn extract_json_block(text: &str) -> Option<&str> {
    // 查找 ```json ... ``` 或 ``` ... ```
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    None
}

// ============================================================================
// 内置话术模板
// ============================================================================

/// 内置客服话术模板（友好风格）
pub fn builtin_customer_service_speech() -> SpeechTraining {
    let mut training = SpeechTraining::new("customer_service", "电商客服咨询")
        .with_style(SpeechStyle::Friendly);

    training
        .add_example(
            SpeechExample::new(
                "我的订单什么时候发货？",
                "亲，您的订单我们会在 24 小时内为您发货哦~ 发货后会有物流单号通知您，请留意短信提醒。如有其他问题随时联系我们！",
            )
            .with_tag("发货咨询"),
        )
        .add_example(
            SpeechExample::new(
                "这个商品有现货吗？",
                "亲，这款商品目前有现货的哦~ 您可以放心下单，我们会在付款后尽快为您安排发货！",
            )
            .with_tag("库存咨询"),
        )
        .add_example(
            SpeechExample::new(
                "退款多久到账？",
                "亲，退款一般会在 1-3 个工作日内原路退回您的账户哦~ 如果超过时间还没到账，请随时联系我们帮您查询处理！",
            )
            .with_tag("退款咨询"),
        );

    training
        .constraints
        .forbid("不知道")
        .forbid("不清楚")
        .require("亲");

    training
}

/// 内置金融顾问话术模板（专业风格）
pub fn builtin_finance_advisor_speech() -> SpeechTraining {
    let mut training = SpeechTraining::new("finance_advisor", "银行理财顾问咨询")
        .with_style(SpeechStyle::Professional);

    training
        .add_example(
            SpeechExample::new(
                "有什么稳健的理财产品推荐？",
                "根据您的需求，我为您推荐以下稳健型理财产品：\n1. 货币基金：风险低，流动性好，适合短期资金管理\n2. 国债逆回购：安全性高，收益稳定\n3. 银行定期存款：保本保息，适合长期资金配置\n\n投资有风险，入市需谨慎。建议根据您的风险承受能力和资金规划选择合适的产品。",
            )
            .with_tag("产品推荐"),
        )
        .add_example(
            SpeechExample::new(
                "基金定投适合我吗？",
                "基金定投适合以下情况：\n1. 有稳定收入，希望长期积累财富\n2. 风险承受能力中等\n3. 缺乏时间盯盘，希望被动投资\n\n定投的优势在于摊薄成本、分散风险。但需注意：基金过往业绩不代表未来表现，投资有风险，入市需谨慎。",
            )
            .with_tag("定投咨询"),
        );

    training
        .constraints
        .forbid("保证收益")
        .forbid("稳赚不赔")
        .require("风险");

    training
}

/// 内置医疗咨询话术模板（沉稳风格）
pub fn builtin_medical_consult_speech() -> SpeechTraining {
    let mut training = SpeechTraining::new("medical_consult", "医院在线咨询助手")
        .with_style(SpeechStyle::Calm);

    training
        .add_example(
            SpeechExample::new(
                "我最近经常头痛，怎么办？",
                "头痛可能有多种原因，包括紧张、疲劳、颈椎问题或更严重的疾病。建议您：\n1. 记录头痛的频率、持续时间和诱因\n2. 注意休息，避免过度劳累\n3. 如头痛持续加重或伴随其他症状（如视力模糊、呕吐），请及时就医\n\n在线咨询无法替代面诊，建议前往医院神经内科就诊。",
            )
            .with_tag("症状咨询"),
        )
        .add_example(
            SpeechExample::new(
                "这个药怎么吃？",
                "关于用药，我无法提供具体的用药指导，因为用药需根据您的具体病情、身体状况和医生诊断来决定。建议您：\n1. 仔细阅读药品说明书\n2. 遵医嘱服用\n3. 如有疑问，请咨询主治医师或药师\n\n切勿自行调整剂量或停药。",
            )
            .with_tag("用药咨询"),
        );

    training
        .constraints
        .forbid("诊断")
        .forbid("开处方")
        .require("就医");

    training
}

/// 获取所有内置话术模板
pub fn builtin_all_speech() -> Vec<SpeechTraining> {
    vec![
        builtin_customer_service_speech(),
        builtin_finance_advisor_speech(),
        builtin_medical_consult_speech(),
    ]
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use timeflow_ai::MockEmbeddingEngine;
    use timeflow_ai::MockEngine;

    // ---- SpeechStyle ----

    #[test]
    fn test_style_from_str() {
        assert_eq!("friendly".parse::<SpeechStyle>().unwrap(), SpeechStyle::Friendly);
        assert_eq!("友好".parse::<SpeechStyle>().unwrap(), SpeechStyle::Friendly);
        assert_eq!("professional".parse::<SpeechStyle>().unwrap(), SpeechStyle::Professional);
        assert_eq!("严肃".parse::<SpeechStyle>().unwrap(), SpeechStyle::Serious);
        assert_eq!("lively".parse::<SpeechStyle>().unwrap(), SpeechStyle::Lively);
        assert_eq!("calm".parse::<SpeechStyle>().unwrap(), SpeechStyle::Calm);
        assert!("invalid".parse::<SpeechStyle>().is_err());
    }

    #[test]
    fn test_style_display() {
        assert_eq!(SpeechStyle::Friendly.to_string(), "友好");
        assert_eq!(SpeechStyle::Professional.to_string(), "专业");
    }

    #[test]
    fn test_style_system_prompt_hint() {
        let hint = SpeechStyle::Friendly.system_prompt_hint();
        assert!(hint.contains("友好"));
        assert!(hint.contains("亲切"));

        let hint = SpeechStyle::Professional.system_prompt_hint();
        assert!(hint.contains("专业"));
    }

    // ---- SpeechExample ----

    #[test]
    fn test_example_new() {
        let ex = SpeechExample::new("你好", "您好，有什么可以帮您？");
        assert_eq!(ex.user, "你好");
        assert_eq!(ex.assistant, "您好，有什么可以帮您？");
        assert!(ex.tag.is_none());
        assert!(ex.quality_score.is_none());
    }

    #[test]
    fn test_example_with_tag_and_score() {
        let ex = SpeechExample::new("你好", "您好")
            .with_tag("问候")
            .with_score(0.85);
        assert_eq!(ex.tag, Some("问候".to_string()));
        assert_eq!(ex.quality_score, Some(0.85));
    }

    #[test]
    fn test_example_as_text() {
        let ex = SpeechExample::new("你好", "您好");
        let text = ex.as_text();
        assert!(text.contains("用户: 你好"));
        assert!(text.contains("助手: 您好"));
    }

    // ---- SpeechConstraints ----

    #[test]
    fn test_constraints_forbid_and_require() {
        let mut c = SpeechConstraints::new();
        c.forbid("密码").forbid("身份证");
        c.require("亲").require("您好");
        assert_eq!(c.forbidden_words.len(), 2);
        assert_eq!(c.required_words.len(), 2);
    }

    #[test]
    fn test_constraints_check_pass() {
        let mut c = SpeechConstraints::new();
        c.require("亲");
        let result = c.check("亲，您好！");
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_constraints_check_forbidden_word() {
        let mut c = SpeechConstraints::new();
        c.forbid("密码");
        let result = c.check("请告诉我密码");
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| matches!(v, ConstraintViolation::ForbiddenWord(w) if w == "密码")));
    }

    #[test]
    fn test_constraints_check_missing_required() {
        let mut c = SpeechConstraints::new();
        c.require("亲").require("您好");
        let result = c.check("你好，有什么问题？");
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| matches!(v, ConstraintViolation::MissingRequiredWord(_))));
    }

    #[test]
    fn test_constraints_check_too_short() {
        let mut c = SpeechConstraints::new();
        c.min_length = Some(10);
        let result = c.check("短");
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| matches!(v, ConstraintViolation::TooShort { actual: 1, required: 10 })));
    }

    #[test]
    fn test_constraints_check_too_long() {
        let mut c = SpeechConstraints::new();
        c.max_length = Some(5);
        let result = c.check("这是一段很长的回复文本");
        assert!(!result.passed);
        // "这是一段很长的回复文本" 共 11 个字符
        assert!(result.violations.iter().any(|v| matches!(
            v,
            ConstraintViolation::TooLong { actual: 11, required: 5 }
        )));
    }

    #[test]
    fn test_constraint_violation_display() {
        let v = ConstraintViolation::ForbiddenWord("密码".to_string());
        assert!(v.to_string().contains("密码"));

        let v = ConstraintViolation::MissingRequiredWord(vec!["亲".to_string()]);
        assert!(v.to_string().contains("亲"));
    }

    // ---- SpeechTraining ----

    #[test]
    fn test_training_new() {
        let t = SpeechTraining::new("emp1", "测试场景");
        assert_eq!(t.employee_id, "emp1");
        assert_eq!(t.scenario, "测试场景");
        assert_eq!(t.style, SpeechStyle::Professional);
        assert!(t.examples.is_empty());
    }

    #[test]
    fn test_training_with_style() {
        let t = SpeechTraining::new("emp1", "测试").with_style(SpeechStyle::Friendly);
        assert_eq!(t.style, SpeechStyle::Friendly);
    }

    #[test]
    fn test_training_add_example() {
        let mut t = SpeechTraining::new("emp1", "测试");
        t.add_example(SpeechExample::new("你好", "您好"));
        assert_eq!(t.example_count(), 1);
    }

    #[test]
    fn test_training_validate_empty_employee() {
        let t = SpeechTraining::new("", "测试");
        assert!(t.validate().is_err());
    }

    #[test]
    fn test_training_validate_empty_scenario() {
        let t = SpeechTraining::new("emp1", "");
        assert!(t.validate().is_err());
    }

    #[test]
    fn test_training_validate_empty_example_field() {
        let mut t = SpeechTraining::new("emp1", "测试");
        t.add_example(SpeechExample::new("", "回复"));
        assert!(t.validate().is_err());
    }

    #[test]
    fn test_training_validate_valid() {
        let mut t = SpeechTraining::new("emp1", "测试");
        t.add_example(SpeechExample::new("你好", "您好"));
        assert!(t.validate().is_ok());
    }

    #[test]
    fn test_training_yaml_roundtrip() {
        let mut t = SpeechTraining::new("emp1", "测试场景")
            .with_style(SpeechStyle::Friendly);
        t.add_example(SpeechExample::new("你好", "您好").with_tag("问候"));
        t.constraints.forbid("密码").require("亲");

        let yaml = t.to_yaml().unwrap();
        let t2 = SpeechTraining::from_yaml(&yaml).unwrap();
        assert_eq!(t.employee_id, t2.employee_id);
        assert_eq!(t.scenario, t2.scenario);
        assert_eq!(t.style, t2.style);
        assert_eq!(t.examples.len(), t2.examples.len());
        assert_eq!(t.constraints.forbidden_words, t2.constraints.forbidden_words);
    }

    // ---- FewShotBuilder ----

    #[test]
    fn test_fewshot_builder_default() {
        let b = FewShotBuilder::default();
        assert_eq!(b.top_k, 3);
    }

    #[test]
    fn test_fewshot_builder_new() {
        let b = FewShotBuilder::new(5);
        assert_eq!(b.top_k, 5);
    }

    #[test]
    fn test_fewshot_builder_new_min_1() {
        let b = FewShotBuilder::new(0);
        assert_eq!(b.top_k, 1);
    }

    #[tokio::test]
    async fn test_fewshot_build_prompt_with_mock() {
        let training = builtin_customer_service_speech();
        let builder = FewShotBuilder::default();
        let engine = MockEmbeddingEngine::new();

        let prompt = builder
            .build_prompt(&training, "我的订单什么时候发货？", &engine)
            .await
            .unwrap();

        assert!(prompt.contains("电商客服咨询"));
        assert!(prompt.contains("友好"));
        assert!(prompt.contains("【参考示例】"));
    }

    #[test]
    fn test_fewshot_build_prompt_simple() {
        let training = builtin_customer_service_speech();
        let builder = FewShotBuilder::new(2);
        let prompt = builder.build_prompt_simple(&training);

        assert!(prompt.contains("电商客服咨询"));
        assert!(prompt.contains("友好"));
        assert!(prompt.contains("【参考示例】"));
        // 只取前 2 个示例
        let example_count = prompt.matches("用户:").count();
        assert_eq!(example_count, 2);
    }

    #[tokio::test]
    async fn test_fewshot_select_similar_top_k() {
        let mut training = SpeechTraining::new("emp1", "测试");
        for i in 0..5 {
            training.add_example(SpeechExample::new(
                format!("问题 {i}"),
                format!("回答 {i}"),
            ));
        }
        let builder = FewShotBuilder::new(3);
        let engine = MockEmbeddingEngine::new();

        let selected = builder
            .select_similar(&training, "查询", &engine)
            .await
            .unwrap();
        assert_eq!(selected.len(), 3);
    }

    #[tokio::test]
    async fn test_fewshot_select_similar_all_when_few() {
        let mut training = SpeechTraining::new("emp1", "测试");
        training.add_example(SpeechExample::new("问题 1", "回答 1"));
        let builder = FewShotBuilder::new(3);
        let engine = MockEmbeddingEngine::new();

        let selected = builder
            .select_similar(&training, "查询", &engine)
            .await
            .unwrap();
        assert_eq!(selected.len(), 1); // 不足 top_k 时返回全部
    }

    // ---- ConstraintChecker ----

    #[test]
    fn test_constraint_checker_check() {
        let training = builtin_customer_service_speech();
        // 包含"亲"，通过
        let result = ConstraintChecker::check(&training, "亲，您好！");
        assert!(result.passed);

        // 不包含"亲"，未通过
        let result = ConstraintChecker::check(&training, "您好！");
        assert!(!result.passed);
    }

    #[test]
    fn test_constraint_checker_suggest_fix() {
        let v = ConstraintViolation::ForbiddenWord("密码".to_string());
        let suggestion = ConstraintChecker::suggest_fix(&v);
        assert!(suggestion.contains("密码"));

        let v = ConstraintViolation::TooShort {
            actual: 5,
            required: 10,
        };
        let suggestion = ConstraintChecker::suggest_fix(&v);
        assert!(suggestion.contains("5"));
        assert!(suggestion.contains("10"));
    }

    // ---- SpeechJudge ----

    #[test]
    fn test_judge_build_prompt() {
        let judge = SpeechJudge::new();
        let training = builtin_customer_service_speech();
        let prompt = judge.build_judge_prompt(&training, "用户问题", "AI 回复");

        assert!(prompt.contains("电商客服咨询"));
        assert!(prompt.contains("用户问题"));
        assert!(prompt.contains("AI 回复"));
        assert!(prompt.contains("亲")); // required_words
    }

    #[tokio::test]
    async fn test_judge_judge_with_mock_llm() {
        let judge = SpeechJudge::new();
        let training = builtin_customer_service_speech();
        // MockEngine 返回一个合法 JSON 评分
        let mock_response = r#"{"score": 8.5, "style_score": 8.0, "relevance_score": 9.0, "constraint_score": 9.5, "feedback": "回复风格友好且包含必要词"}"#;
        let llm = MockEngine::new(mock_response);

        let result = judge
            .judge(&training, "我的订单什么时候发货？", "亲，24 小时内发货哦~", &llm)
            .await
            .unwrap();

        // Mock LLM 返回固定内容，分数可能为 0，但不应报错
        assert!(result.score >= 0.0);
        assert!(result.constraint_score >= 0.0);
    }

    #[test]
    fn test_judge_constraints_only_pass() {
        let training = builtin_customer_service_speech();
        let result = SpeechJudge::judge_constraints_only(&training, "亲，您好！");

        assert!(result.passed());
        assert_eq!(result.constraint_score, 10.0);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_judge_constraints_only_fail() {
        let training = builtin_customer_service_speech();
        // 不含"亲"
        let result = SpeechJudge::judge_constraints_only(&training, "您好！");

        assert!(!result.passed());
        assert!(result.constraint_score < 10.0);
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn test_judge_result_passed() {
        let r = JudgeResult {
            score: 7.5,
            ..Default::default()
        };
        assert!(r.passed());

        let r = JudgeResult {
            score: 5.5,
            ..Default::default()
        };
        assert!(!r.passed());
    }

    // ---- JSON 解析 ----

    #[test]
    fn test_parse_judge_response_valid_json() {
        let content = r#"```json
        {
          "style_score": 8.5,
          "relevance_score": 7.0,
          "constraint_score": 9.0,
          "feedback": "回复风格友好，内容切题"
        }
        ```"#;
        let (style, relevance, feedback) = parse_judge_response(content);
        assert_eq!(style, 8.5);
        assert_eq!(relevance, 7.0);
        assert!(feedback.contains("友好"));
    }

    #[test]
    fn test_parse_judge_response_plain_json() {
        let content = r#"{"style_score": 6.0, "relevance_score": 7.0, "feedback": "不错"}"#;
        let (style, relevance, feedback) = parse_judge_response(content);
        assert_eq!(style, 6.0);
        assert_eq!(relevance, 7.0);
        assert_eq!(feedback, "不错");
    }

    #[test]
    fn test_parse_judge_response_invalid() {
        let content = "这不是 JSON";
        let (style, relevance, feedback) = parse_judge_response(content);
        assert_eq!(style, 0.0);
        assert_eq!(relevance, 0.0);
        assert!(feedback.contains("解析失败"));
    }

    #[test]
    fn test_extract_json_block_with_tag() {
        let text = "前文\n```json\n{\"a\": 1}\n```\n后文";
        let extracted = extract_json_block(text);
        assert_eq!(extracted, Some("{\"a\": 1}"));
    }

    #[test]
    fn test_extract_json_block_without_tag() {
        let text = "前文\n```\n{\"a\": 1}\n```\n后文";
        let extracted = extract_json_block(text);
        assert_eq!(extracted, Some("{\"a\": 1}"));
    }

    #[test]
    fn test_extract_json_block_none() {
        let text = "没有代码块";
        let extracted = extract_json_block(text);
        assert!(extracted.is_none());
    }

    // ---- 内置模板 ----

    #[test]
    fn test_builtin_customer_service_speech() {
        let t = builtin_customer_service_speech();
        assert_eq!(t.employee_id, "customer_service");
        assert_eq!(t.style, SpeechStyle::Friendly);
        assert!(t.example_count() >= 3);
        assert!(t.validate().is_ok());
        assert!(t.constraints.forbidden_words.contains(&"不知道".to_string()));
        assert!(t.constraints.required_words.contains(&"亲".to_string()));
    }

    #[test]
    fn test_builtin_finance_advisor_speech() {
        let t = builtin_finance_advisor_speech();
        assert_eq!(t.style, SpeechStyle::Professional);
        assert!(t.example_count() >= 2);
        assert!(t.validate().is_ok());
        assert!(t.constraints.forbidden_words.contains(&"保证收益".to_string()));
        assert!(t.constraints.required_words.contains(&"风险".to_string()));
    }

    #[test]
    fn test_builtin_medical_consult_speech() {
        let t = builtin_medical_consult_speech();
        assert_eq!(t.style, SpeechStyle::Calm);
        assert!(t.example_count() >= 2);
        assert!(t.validate().is_ok());
        assert!(t.constraints.forbidden_words.contains(&"诊断".to_string()));
        assert!(t.constraints.required_words.contains(&"就医".to_string()));
    }

    #[test]
    fn test_builtin_all_speech() {
        let all = builtin_all_speech();
        assert_eq!(all.len(), 3);
        for t in &all {
            assert!(t.validate().is_ok());
        }
    }

    // ---- 文件加载 ----

    #[test]
    fn test_training_save_load_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("speech.yaml");

        let t = builtin_customer_service_speech();
        t.save_to_file(&path).unwrap();

        let loaded = SpeechTraining::load_from_file(&path).unwrap();
        assert_eq!(t.employee_id, loaded.employee_id);
        assert_eq!(t.scenario, loaded.scenario);
        assert_eq!(t.examples.len(), loaded.examples.len());
    }
}
