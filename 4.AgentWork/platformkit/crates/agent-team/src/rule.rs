// 规则引擎（M4.1 D2）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D2
//
// 目标：为 AI 员工提供行为约束规则，触发匹配后影响 AI 输出。
//
// 技术选型（v2.0 明确）：
//   - 简单规则链（不引入 Rete 算法）
//   - 关键词匹配 + 可选 LLM 二次确认（MVP 阶段仅关键词匹配）
//   - 规则按 priority 降序执行，命中后根据 action 决定是否继续
//
// 模块组成：
//   - RuleCondition: 触发条件（关键词 + 逻辑运算 AND/OR/NOT）
//   - RuleAction: 动作枚举（追加 prompt / 调用工具 / 拒绝 / 替换）
//   - Rule: 单条规则
//   - RuleSet: 规则集（按 priority 排序）
//   - RuleEngine: 规则引擎（匹配 + 执行）
//   - 内置模板：金融合规 / 教育守则 / 医疗保密

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 触发条件
// ============================================================================

/// 关键词逻辑运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicOp {
    /// 全部关键词都命中（AND）
    All,
    /// 任一关键词命中（OR）
    Any,
    /// 全部关键词都不命中（NOT）
    None,
}

impl Default for LogicOp {
    fn default() -> Self {
        Self::Any
    }
}

impl std::fmt::Display for LogicOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
        }
    }
}

/// 触发条件
///
/// 通过关键词列表 + 逻辑运算符定义触发条件。
/// 例如：`{ keywords: ["密码", "身份证"], op: All }` 表示文本同时包含"密码"和"身份证"才触发。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// 关键词列表（大小写不敏感）
    pub keywords: Vec<String>,
    /// 逻辑运算符
    #[serde(default)]
    pub op: LogicOp,
}

impl Default for RuleCondition {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            op: LogicOp::Any,
        }
    }
}

impl RuleCondition {
    /// 创建 OR 条件（任一关键词命中即触发）
    pub fn any(keywords: Vec<String>) -> Self {
        Self {
            keywords,
            op: LogicOp::Any,
        }
    }

    /// 创建 AND 条件（全部关键词命中才触发）
    pub fn all(keywords: Vec<String>) -> Self {
        Self {
            keywords,
            op: LogicOp::All,
        }
    }

    /// 创建 NOT 条件（全部关键词都不命中才触发）
    pub fn none(keywords: Vec<String>) -> Self {
        Self {
            keywords,
            op: LogicOp::None,
        }
    }

    /// 检查文本是否匹配条件
    pub fn matches(&self, text: &str) -> bool {
        if self.keywords.is_empty() {
            return false;
        }
        let lower = text.to_lowercase();
        let hits: Vec<bool> = self
            .keywords
            .iter()
            .map(|k| {
                let k_lower = k.to_lowercase();
                !k_lower.is_empty() && lower.contains(&k_lower)
            })
            .collect();

        match self.op {
            LogicOp::All => hits.iter().all(|&h| h),
            LogicOp::Any => hits.iter().any(|&h| h),
            LogicOp::None => hits.iter().all(|&h| !h),
        }
    }

    /// 返回命中的关键词列表
    pub fn matched_keywords(&self, text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        self.keywords
            .iter()
            .filter(|k| {
                let k_lower = k.to_lowercase();
                !k_lower.is_empty() && lower.contains(&k_lower)
            })
            .cloned()
            .collect()
    }
}

// ============================================================================
// 规则动作
// ============================================================================

/// 规则动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleAction {
    /// 追加 prompt（在系统提示词后追加约束）
    AppendPrompt {
        /// 追加的 prompt 文本
        prompt: String,
    },
    /// 前置 prompt（在用户消息前插入约束）
    PrependPrompt {
        /// 前置的 prompt 文本
        prompt: String,
    },
    /// 调用工具
    CallTool {
        /// 工具名
        tool: String,
        /// 工具参数（JSON 字符串）
        arguments: String,
    },
    /// 拒绝响应（直接返回拒绝消息，不调用 LLM）
    Refuse {
        /// 拒绝消息
        message: String,
    },
    /// 替换响应（将 LLM 响应中的敏感词替换为占位符）
    Replace {
        /// 需要替换的词列表
        words: Vec<String>,
        /// 替换为的占位符
        placeholder: String,
    },
}

impl RuleAction {
    /// 是否为拒绝动作（最高优先级，命中后应立即终止）
    pub fn is_refuse(&self) -> bool {
        matches!(self, Self::Refuse { .. })
    }

    /// 是否需要调用 LLM
    ///
    /// Refuse 动作不需要调用 LLM，直接返回拒绝消息。
    pub fn needs_llm(&self) -> bool {
        !self.is_refuse()
    }
}

// ============================================================================
// 规则
// ============================================================================

/// 单条规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 触发条件
    pub when: RuleCondition,
    /// 执行动作
    pub then: RuleAction,
    /// 优先级（数值越大越先执行，默认 0）
    #[serde(default)]
    pub priority: u32,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 规则描述
    #[serde(default)]
    pub description: String,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

impl Rule {
    /// 创建新规则
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        when: RuleCondition,
        then: RuleAction,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            when,
            then,
            priority: 0,
            enabled: true,
            description: String::new(),
            metadata: HashMap::new(),
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 检查文本是否触发规则
    pub fn matches(&self, text: &str) -> bool {
        self.enabled && self.when.matches(text)
    }

    /// 验证规则合法性
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(TeamError::Config("规则 ID 不能为空".to_string()));
        }
        if self.name.is_empty() {
            return Err(TeamError::Config("规则名称不能为空".to_string()));
        }
        if self.when.keywords.is_empty() {
            return Err(TeamError::Config(format!(
                "规则 {} 的触发条件关键词列表不能为空",
                self.id
            )));
        }
        match &self.then {
            RuleAction::AppendPrompt { prompt } | RuleAction::PrependPrompt { prompt } => {
                if prompt.is_empty() {
                    return Err(TeamError::Config(format!(
                        "规则 {} 的 prompt 动作内容不能为空",
                        self.id
                    )));
                }
            }
            RuleAction::CallTool { tool, .. } => {
                if tool.is_empty() {
                    return Err(TeamError::Config(format!(
                        "规则 {} 的工具名不能为空",
                        self.id
                    )));
                }
            }
            RuleAction::Refuse { message } => {
                if message.is_empty() {
                    return Err(TeamError::Config(format!(
                        "规则 {} 的拒绝消息不能为空",
                        self.id
                    )));
                }
            }
            RuleAction::Replace { words, .. } => {
                if words.is_empty() {
                    return Err(TeamError::Config(format!(
                        "规则 {} 的替换词列表不能为空",
                        self.id
                    )));
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// 规则集
// ============================================================================

/// 规则匹配结果
#[derive(Debug, Clone)]
pub struct RuleMatchResult {
    /// 命中的规则
    pub rule: Rule,
    /// 命中的关键词
    pub matched_keywords: Vec<String>,
}

/// 规则集
///
/// 管理一组规则，按 priority 降序执行。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleSet {
    /// 规则列表
    pub rules: Vec<Rule>,
    /// 所属员工 ID
    #[serde(default)]
    pub employee_id: String,
}

impl RuleSet {
    /// 创建空规则集
    pub fn new(employee_id: impl Into<String>) -> Self {
        Self {
            rules: Vec::new(),
            employee_id: employee_id.into(),
        }
    }

    /// 添加规则
    pub fn add(&mut self, rule: Rule) {
        self.rules.push(rule);
        self.sort_by_priority();
    }

    /// 批量添加
    pub fn add_all(&mut self, rules: Vec<Rule>) {
        self.rules.extend(rules);
        self.sort_by_priority();
    }

    /// 按 priority 降序排序
    fn sort_by_priority(&mut self) {
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 移除规则
    pub fn remove(&mut self, rule_id: &str) -> Option<Rule> {
        if let Some(idx) = self.rules.iter().position(|r| r.id == rule_id) {
            Some(self.rules.remove(idx))
        } else {
            None
        }
    }

    /// 按 ID 获取规则
    pub fn get(&self, rule_id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// 启用/禁用规则
    pub fn set_enabled(&mut self, rule_id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// 规则数量
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 已启用的规则数量
    pub fn enabled_count(&self) -> usize {
        self.rules.iter().filter(|r| r.enabled).count()
    }

    /// 验证所有规则
    pub fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            rule.validate()?;
        }
        // 检查 ID 唯一性
        let mut ids: Vec<&str> = self.rules.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        for window in ids.windows(2) {
            if window[0] == window[1] {
                return Err(TeamError::Config(format!("规则 ID 重复: {}", window[0])));
            }
        }
        Ok(())
    }

    /// 匹配文本，返回所有命中的规则（按 priority 降序）
    pub fn matches(&self, text: &str) -> Vec<RuleMatchResult> {
        self.rules
            .iter()
            .filter(|r| r.enabled)
            .filter(|r| r.when.matches(text))
            .map(|r| RuleMatchResult {
                matched_keywords: r.when.matched_keywords(text),
                rule: r.clone(),
            })
            .collect()
    }

    /// 匹配文本，返回第一个命中的拒绝规则（如有）
    ///
    /// 拒绝规则优先级最高，命中后应立即终止并返回拒绝消息。
    pub fn find_refuse(&self, text: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .filter(|r| r.enabled && r.then.is_refuse())
            .find(|r| r.when.matches(text))
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
// 规则引擎
// ============================================================================

/// 规则引擎执行结果
#[derive(Debug, Clone)]
pub struct EngineResult {
    /// 是否拒绝（命中 Refuse 动作）
    pub refused: bool,
    /// 拒绝消息（refused=true 时有值）
    pub refuse_message: Option<String>,
    /// 追加的 prompt 列表（按 priority 降序）
    pub append_prompts: Vec<String>,
    /// 前置的 prompt 列表（按 priority 降序）
    pub prepend_prompts: Vec<String>,
    /// 需要调用的工具列表
    pub tool_calls: Vec<(String, String)>, // (tool, arguments)
    /// 替换规则列表
    pub replacements: Vec<(Vec<String>, String)>, // (words, placeholder)
    /// 命中的规则 ID 列表
    pub matched_rule_ids: Vec<String>,
}

impl Default for EngineResult {
    fn default() -> Self {
        Self {
            refused: false,
            refuse_message: None,
            append_prompts: Vec::new(),
            prepend_prompts: Vec::new(),
            tool_calls: Vec::new(),
            replacements: Vec::new(),
            matched_rule_ids: Vec::new(),
        }
    }
}

impl EngineResult {
    /// 是否需要调用 LLM
    pub fn needs_llm(&self) -> bool {
        !self.refused
    }

    /// 应用替换规则到文本
    pub fn apply_replacements(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (words, placeholder) in &self.replacements {
            for word in words {
                // 大小写不敏感替换（按字符处理，避免 UTF-8 边界问题）
                let word_lower = word.to_lowercase();
                let placeholder_chars: Vec<char> = placeholder.chars().collect();
                let word_char_count = word.chars().count();

                let mut replaced = String::new();
                let result_chars: Vec<char> = result.chars().collect();
                let result_lower: String = result.to_lowercase();
                let result_lower_chars: Vec<char> = result_lower.chars().collect();

                let mut i = 0;
                while i < result_chars.len() {
                    // 检查从 i 开始是否匹配 word
                    if i + word_char_count <= result_lower_chars.len() {
                        let candidate: String = result_lower_chars[i..i + word_char_count].iter().collect();
                        if candidate == word_lower {
                            // 匹配，替换为 placeholder
                            replaced.extend(placeholder_chars.iter());
                            i += word_char_count;
                            continue;
                        }
                    }
                    replaced.push(result_chars[i]);
                    i += 1;
                }
                result = replaced;
            }
        }
        result
    }
}

/// 规则引擎
///
/// 执行流程：
///   1. 按 priority 降序匹配所有规则
///   2. 若命中 Refuse 动作 → 立即返回拒绝结果
///   3. 收集所有 AppendPrompt / PrependPrompt / CallTool / Replace 动作
///   4. 返回 EngineResult，调用方根据结果决定如何调用 LLM
pub struct RuleEngine {
    /// 规则集
    pub ruleset: RuleSet,
}

impl RuleEngine {
    /// 创建引擎
    pub fn new(ruleset: RuleSet) -> Self {
        Self { ruleset }
    }

    /// 从员工 ID 创建空引擎
    pub fn for_employee(employee_id: impl Into<String>) -> Self {
        Self::new(RuleSet::new(employee_id))
    }

    /// 执行规则匹配
    pub fn evaluate(&self, text: &str) -> EngineResult {
        let mut result = EngineResult::default();
        let matches = self.ruleset.matches(text);

        for m in &matches {
            result.matched_rule_ids.push(m.rule.id.clone());
            match &m.rule.then {
                RuleAction::Refuse { message } => {
                    // 拒绝动作：立即终止
                    result.refused = true;
                    result.refuse_message = Some(message.clone());
                    return result;
                }
                RuleAction::AppendPrompt { prompt } => {
                    result.append_prompts.push(prompt.clone());
                }
                RuleAction::PrependPrompt { prompt } => {
                    result.prepend_prompts.push(prompt.clone());
                }
                RuleAction::CallTool { tool, arguments } => {
                    result.tool_calls.push((tool.clone(), arguments.clone()));
                }
                RuleAction::Replace { words, placeholder } => {
                    result.replacements.push((words.clone(), placeholder.clone()));
                }
            }
        }

        result
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: Rule) {
        self.ruleset.add(rule);
    }

    /// 批量添加规则
    pub fn add_rules(&mut self, rules: Vec<Rule>) {
        self.ruleset.add_all(rules);
    }

    /// 规则数量
    pub fn len(&self) -> usize {
        self.ruleset.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.ruleset.is_empty()
    }
}

// ============================================================================
// 内置规则模板
// ============================================================================

/// 内置金融合规规则集
///
/// 场景：银行/证券/保险客服，防止泄露敏感信息。
pub fn builtin_finance_rules() -> RuleSet {
    let mut rs = RuleSet::new("builtin_finance");

    // 拒绝：询问密码
    rs.add(
        Rule::new(
            "fin_refuse_password",
            "拒绝询问密码",
            RuleCondition::any(vec!["密码".to_string(), "password".to_string()]),
            RuleAction::Refuse {
                message: "抱歉，出于安全考虑，我无法提供或验证任何密码。请联系银行客服热线处理。".to_string(),
            },
        )
        .with_priority(100)
        .with_description("用户询问密码时直接拒绝"),
    );

    // 拒绝：询问完整银行卡号
    rs.add(
        Rule::new(
            "fin_refuse_card_number",
            "拒绝询问完整银行卡号",
            RuleCondition::all(vec!["银行卡".to_string(), "卡号".to_string()]),
            RuleAction::Refuse {
                message: "抱歉，我无法提供完整的银行卡号。请通过银行官方 APP 查询。".to_string(),
            },
        )
        .with_priority(100)
        .with_description("用户询问完整银行卡号时拒绝"),
    );

    // 追加 prompt：涉及投资理财
    rs.add(
        Rule::new(
            "fin_append_investment",
            "投资理财风险提示",
            RuleCondition::any(vec![
                "投资".to_string(),
                "理财".to_string(),
                "基金".to_string(),
                "股票".to_string(),
            ]),
            RuleAction::AppendPrompt {
                prompt: "【合规要求】讨论投资理财时，必须附加风险提示：「投资有风险，入市需谨慎。过往业绩不代表未来表现。」".to_string(),
            },
        )
        .with_priority(50)
        .with_description("涉及投资理财时追加风险提示"),
    );

    // 替换：身份证号脱敏
    rs.add(
        Rule::new(
            "fin_replace_id_card",
            "身份证号脱敏",
            RuleCondition::any(vec!["身份证".to_string()]),
            RuleAction::Replace {
                words: vec!["身份证号".to_string()],
                placeholder: "***".to_string(),
            },
        )
        .with_priority(30)
        .with_description("将身份证号替换为占位符"),
    );

    rs
}

/// 内置教育守则规则集
///
/// 场景：学校/培训机构助教，防止泄露学生隐私。
pub fn builtin_education_rules() -> RuleSet {
    let mut rs = RuleSet::new("builtin_education");

    // 拒绝：询问学生成绩排名
    rs.add(
        Rule::new(
            "edu_refuse_ranking",
            "拒绝询问成绩排名",
            RuleCondition::all(vec!["排名".to_string(), "成绩".to_string()]),
            RuleAction::Refuse {
                message: "抱歉，根据教育部门规定，学校不得公开学生成绩排名。请直接联系班主任了解情况。".to_string(),
            },
        )
        .with_priority(100)
        .with_description("拒绝公开学生成绩排名"),
    );

    // 追加 prompt：涉及学生评价
    rs.add(
        Rule::new(
            "edu_append_evaluation",
            "学生评价客观性提示",
            RuleCondition::any(vec!["评价".to_string(), "表现".to_string(), "比较".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "【教育守则】评价学生时必须客观公正，避免与其他学生对比，关注学生个体进步。".to_string(),
            },
        )
        .with_priority(50)
        .with_description("评价学生时追加客观性提示"),
    );

    // 替换：家庭住址脱敏
    rs.add(
        Rule::new(
            "edu_replace_address",
            "家庭住址脱敏",
            RuleCondition::any(vec!["家庭住址".to_string(), "家庭地址".to_string()]),
            RuleAction::Replace {
                words: vec!["家庭住址".to_string(), "家庭地址".to_string()],
                placeholder: "[地址已隐藏]".to_string(),
            },
        )
        .with_priority(30)
        .with_description("家庭住址脱敏处理"),
    );

    rs
}

/// 内置医疗保密规则集
///
/// 场景：医院/诊所咨询助手，防止泄露患者隐私。
pub fn builtin_medical_rules() -> RuleSet {
    let mut rs = RuleSet::new("builtin_medical");

    // 拒绝：要求诊断具体疾病
    rs.add(
        Rule::new(
            "med_refuse_diagnosis",
            "拒绝在线诊断",
            RuleCondition::all(vec!["诊断".to_string(), "我".to_string()]),
            RuleAction::Refuse {
                message: "抱歉，在线咨询无法替代面诊。请前往正规医院就诊，由执业医师进行诊断。".to_string(),
            },
        )
        .with_priority(100)
        .with_description("拒绝在线诊断具体疾病"),
    );

    // 拒绝：要求开处方
    rs.add(
        Rule::new(
            "med_refuse_prescription",
            "拒绝在线开处方",
            RuleCondition::any(vec!["开处方".to_string(), "开药".to_string(), "处方".to_string()]),
            RuleAction::Refuse {
                message: "抱歉，处方药需凭执业医师处方购买。请前往医院就诊后开具处方。".to_string(),
            },
        )
        .with_priority(100)
        .with_description("拒绝在线开处方"),
    );

    // 追加 prompt：涉及用药建议
    rs.add(
        Rule::new(
            "med_append_medication",
            "用药建议免责声明",
            RuleCondition::any(vec!["用药".to_string(), "吃药".to_string(), "药物".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "【医疗免责】所有用药建议仅供参考，具体用药请遵医嘱。不可自行调整剂量或停药。".to_string(),
            },
        )
        .with_priority(50)
        .with_description("涉及用药建议时追加免责声明"),
    );

    // 替换：病历号脱敏
    rs.add(
        Rule::new(
            "med_replace_medical_record",
            "病历号脱敏",
            RuleCondition::any(vec!["病历号".to_string(), "病案号".to_string()]),
            RuleAction::Replace {
                words: vec!["病历号".to_string(), "病案号".to_string()],
                placeholder: "[病历号已隐藏]".to_string(),
            },
        )
        .with_priority(30)
        .with_description("病历号脱敏处理"),
    );

    rs
}

/// 获取所有内置规则模板
pub fn builtin_all_rules() -> Vec<RuleSet> {
    vec![
        builtin_finance_rules(),
        builtin_education_rules(),
        builtin_medical_rules(),
    ]
}

/// 按行业 ID 获取内置规则集
pub fn builtin_by_industry(industry: &str) -> Option<RuleSet> {
    match industry.to_lowercase().as_str() {
        "finance" | "金融" | "banking" => Some(builtin_finance_rules()),
        "education" | "教育" | "school" => Some(builtin_education_rules()),
        "medical" | "医疗" | "healthcare" | "hospital" => Some(builtin_medical_rules()),
        _ => None,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- RuleCondition ----

    #[test]
    fn test_condition_any_matches() {
        let cond = RuleCondition::any(vec!["密码".to_string(), "password".to_string()]);
        assert!(cond.matches("请告诉我密码"));
        assert!(cond.matches("what is the password"));
        assert!(!cond.matches("普通文本"));
    }

    #[test]
    fn test_condition_all_matches() {
        let cond = RuleCondition::all(vec!["银行卡".to_string(), "卡号".to_string()]);
        assert!(cond.matches("请告诉我银行卡卡号"));
        assert!(!cond.matches("请告诉我银行卡")); // 缺"卡号"
        assert!(!cond.matches("请告诉我卡号")); // 缺"银行卡"
    }

    #[test]
    fn test_condition_none_matches() {
        let cond = RuleCondition::none(vec!["密码".to_string(), "身份证".to_string()]);
        assert!(cond.matches("普通业务咨询"));
        assert!(!cond.matches("包含密码的文本"));
        assert!(!cond.matches("包含身份证的文本"));
    }

    #[test]
    fn test_condition_case_insensitive() {
        let cond = RuleCondition::any(vec!["password".to_string()]);
        assert!(cond.matches("my PASSWORD is 123"));
        assert!(cond.matches("my password is 123"));
    }

    #[test]
    fn test_condition_empty_keywords() {
        let cond = RuleCondition::default();
        assert!(!cond.matches("任何文本"));
    }

    #[test]
    fn test_condition_matched_keywords() {
        let cond = RuleCondition::any(vec!["密码".to_string(), "身份证".to_string(), "验证码".to_string()]);
        let matched = cond.matched_keywords("请提供身份证号和验证码");
        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&"身份证".to_string()));
        assert!(matched.contains(&"验证码".to_string()));
    }

    // ---- RuleAction ----

    #[test]
    fn test_action_is_refuse() {
        let action = RuleAction::Refuse {
            message: "拒绝".to_string(),
        };
        assert!(action.is_refuse());
        assert!(!action.needs_llm());

        let action = RuleAction::AppendPrompt {
            prompt: "提示".to_string(),
        };
        assert!(!action.is_refuse());
        assert!(action.needs_llm());
    }

    // ---- Rule ----

    #[test]
    fn test_rule_new() {
        let rule = Rule::new(
            "r1",
            "测试规则",
            RuleCondition::any(vec!["关键词".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "追加".to_string(),
            },
        );
        assert_eq!(rule.id, "r1");
        assert_eq!(rule.name, "测试规则");
        assert_eq!(rule.priority, 0);
        assert!(rule.enabled);
    }

    #[test]
    fn test_rule_with_priority() {
        let rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        )
        .with_priority(100);
        assert_eq!(rule.priority, 100);
    }

    #[test]
    fn test_rule_matches() {
        let rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["密码".to_string()]),
            RuleAction::Refuse {
                message: "拒绝".to_string(),
            },
        );
        assert!(rule.matches("包含密码"));
        assert!(!rule.matches("普通文本"));
    }

    #[test]
    fn test_rule_disabled_not_matches() {
        let mut rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["密码".to_string()]),
            RuleAction::Refuse {
                message: "拒绝".to_string(),
            },
        );
        rule.enabled = false;
        assert!(!rule.matches("包含密码"));
    }

    #[test]
    fn test_rule_validate_empty_id() {
        let rule = Rule::new(
            "",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        );
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_rule_validate_empty_keywords() {
        let rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::default(),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        );
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_rule_validate_empty_prompt() {
        let rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "".to_string(),
            },
        );
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_rule_validate_valid() {
        let rule = Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        );
        assert!(rule.validate().is_ok());
    }

    // ---- RuleSet ----

    #[test]
    fn test_ruleset_add_and_sort() {
        let mut rs = RuleSet::new("emp1");
        rs.add(
            Rule::new(
                "r1",
                "低优先级",
                RuleCondition::any(vec!["k".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "p1".to_string(),
                },
            )
            .with_priority(10),
        );
        rs.add(
            Rule::new(
                "r2",
                "高优先级",
                RuleCondition::any(vec!["k".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "p2".to_string(),
                },
            )
            .with_priority(100),
        );

        // 应按 priority 降序：r2 在前
        assert_eq!(rs.rules[0].id, "r2");
        assert_eq!(rs.rules[1].id, "r1");
    }

    #[test]
    fn test_ruleset_remove() {
        let mut rs = RuleSet::new("emp1");
        rs.add(Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        ));
        assert_eq!(rs.len(), 1);

        let removed = rs.remove("r1");
        assert!(removed.is_some());
        assert_eq!(rs.len(), 0);

        let not_found = rs.remove("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_ruleset_set_enabled() {
        let mut rs = RuleSet::new("emp1");
        rs.add(Rule::new(
            "r1",
            "测试",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        ));
        assert_eq!(rs.enabled_count(), 1);

        assert!(rs.set_enabled("r1", false));
        assert_eq!(rs.enabled_count(), 0);

        assert!(!rs.set_enabled("nonexistent", true));
    }

    #[test]
    fn test_ruleset_validate_duplicate_id() {
        let mut rs = RuleSet::new("emp1");
        rs.add(Rule::new(
            "r1",
            "测试1",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        ));
        rs.add(Rule::new(
            "r1",
            "测试2",
            RuleCondition::any(vec!["k".to_string()]),
            RuleAction::AppendPrompt {
                prompt: "p".to_string(),
            },
        ));
        assert!(rs.validate().is_err());
    }

    #[test]
    fn test_ruleset_matches() {
        let mut rs = RuleSet::new("emp1");
        rs.add(
            Rule::new(
                "r1",
                "密码规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100),
        );
        rs.add(
            Rule::new(
                "r2",
                "投资规则",
                RuleCondition::any(vec!["投资".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "风险提示".to_string(),
                },
            )
            .with_priority(50),
        );

        let matches = rs.matches("请告诉我密码和投资建议");
        assert_eq!(matches.len(), 2);
        // 高优先级在前
        assert_eq!(matches[0].rule.id, "r1");
        assert_eq!(matches[1].rule.id, "r2");
    }

    #[test]
    fn test_ruleset_find_refuse() {
        let mut rs = RuleSet::new("emp1");
        rs.add(
            Rule::new(
                "r1",
                "密码规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100),
        );

        let refuse = rs.find_refuse("包含密码");
        assert!(refuse.is_some());

        let no_refuse = rs.find_refuse("普通文本");
        assert!(no_refuse.is_none());
    }

    #[test]
    fn test_ruleset_yaml_roundtrip() {
        let mut rs = RuleSet::new("emp1");
        rs.add(
            Rule::new(
                "r1",
                "测试",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100)
            .with_description("测试规则"),
        );

        let yaml = rs.to_yaml().unwrap();
        let rs2 = RuleSet::from_yaml(&yaml).unwrap();
        assert_eq!(rs.rules.len(), rs2.rules.len());
        assert_eq!(rs.rules[0].id, rs2.rules[0].id);
        assert_eq!(rs.rules[0].name, rs2.rules[0].name);
    }

    // ---- RuleEngine ----

    #[test]
    fn test_engine_refuse_action() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "密码规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝提供密码".to_string(),
                },
            )
            .with_priority(100),
        );

        let result = engine.evaluate("请告诉我密码");
        assert!(result.refused);
        assert_eq!(result.refuse_message, Some("拒绝提供密码".to_string()));
        assert!(!result.needs_llm());
    }

    #[test]
    fn test_engine_append_prompt() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "投资规则",
                RuleCondition::any(vec!["投资".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "风险提示".to_string(),
                },
            )
            .with_priority(50),
        );

        let result = engine.evaluate("我想咨询投资理财");
        assert!(!result.refused);
        assert!(result.needs_llm());
        assert_eq!(result.append_prompts.len(), 1);
        assert_eq!(result.append_prompts[0], "风险提示");
    }

    #[test]
    fn test_engine_multiple_matches() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "投资规则",
                RuleCondition::any(vec!["投资".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "风险提示".to_string(),
                },
            )
            .with_priority(50),
        );
        engine.add_rule(
            Rule::new(
                "r2",
                "前置规则",
                RuleCondition::any(vec!["咨询".to_string()]),
                RuleAction::PrependPrompt {
                    prompt: "前置提示".to_string(),
                },
            )
            .with_priority(30),
        );

        let result = engine.evaluate("我想咨询投资");
        assert!(!result.refused);
        assert_eq!(result.append_prompts.len(), 1);
        assert_eq!(result.prepend_prompts.len(), 1);
        assert_eq!(result.matched_rule_ids.len(), 2);
    }

    #[test]
    fn test_engine_refuse_terminates() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "密码规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100),
        );
        engine.add_rule(
            Rule::new(
                "r2",
                "追加规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::AppendPrompt {
                    prompt: "追加".to_string(),
                },
            )
            .with_priority(50),
        );

        let result = engine.evaluate("包含密码");
        // Refuse 应立即终止，不执行后续规则
        assert!(result.refused);
        assert_eq!(result.append_prompts.len(), 0);
        // 只记录了拒绝规则
        assert_eq!(result.matched_rule_ids.len(), 1);
    }

    #[test]
    fn test_engine_no_match() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "密码规则",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100),
        );

        let result = engine.evaluate("普通业务咨询");
        assert!(!result.refused);
        assert!(result.append_prompts.is_empty());
        assert!(result.matched_rule_ids.is_empty());
    }

    #[test]
    fn test_engine_apply_replacements() {
        let mut engine = RuleEngine::for_employee("emp1");
        engine.add_rule(
            Rule::new(
                "r1",
                "身份证脱敏",
                RuleCondition::any(vec!["身份证".to_string()]),
                RuleAction::Replace {
                    words: vec!["身份证号".to_string()],
                    placeholder: "***".to_string(),
                },
            )
            .with_priority(30),
        );

        let result = engine.evaluate("请提供身份证号");
        assert!(!result.refused);
        assert_eq!(result.replacements.len(), 1);

        let replaced = result.apply_replacements("我的身份证号是 123456");
        assert!(replaced.contains("***"));
        assert!(!replaced.contains("身份证号"));
    }

    // ---- 内置规则模板 ----

    #[test]
    fn test_builtin_finance_rules() {
        let rs = builtin_finance_rules();
        assert!(rs.len() >= 3);
        assert!(rs.validate().is_ok());

        // 密码规则应触发拒绝
        let engine = RuleEngine::new(rs);
        let result = engine.evaluate("请告诉我银行卡密码");
        assert!(result.refused);
    }

    #[test]
    fn test_builtin_education_rules() {
        let rs = builtin_education_rules();
        assert!(rs.len() >= 2);
        assert!(rs.validate().is_ok());

        // 成绩排名应触发拒绝
        let engine = RuleEngine::new(rs);
        let result = engine.evaluate("请告诉我班级成绩排名");
        assert!(result.refused);
    }

    #[test]
    fn test_builtin_medical_rules() {
        rs_validate_medical();
    }

    fn rs_validate_medical() {
        let rs = builtin_medical_rules();
        assert!(rs.len() >= 3);
        assert!(rs.validate().is_ok());

        // 开处方应触发拒绝
        let engine = RuleEngine::new(rs);
        let result = engine.evaluate("请帮我开处方");
        assert!(result.refused);
    }

    #[test]
    fn test_builtin_by_industry() {
        assert!(builtin_by_industry("finance").is_some());
        assert!(builtin_by_industry("金融").is_some());
        assert!(builtin_by_industry("education").is_some());
        assert!(builtin_by_industry("教育").is_some());
        assert!(builtin_by_industry("medical").is_some());
        assert!(builtin_by_industry("医疗").is_some());
        assert!(builtin_by_industry("unknown").is_none());
    }

    #[test]
    fn test_builtin_all_rules() {
        let all = builtin_all_rules();
        assert_eq!(all.len(), 3);
        for rs in &all {
            assert!(rs.validate().is_ok());
        }
    }

    // ---- EngineResult ----

    #[test]
    fn test_engine_result_apply_replacements_multiple() {
        let mut result = EngineResult::default();
        result.replacements.push((
            vec!["密码".to_string(), "password".to_string()],
            "***".to_string(),
        ));
        result.replacements.push((
            vec!["身份证".to_string()],
            "[隐藏]".to_string(),
        ));

        let text = "密码是 123，password 是 456，身份证是 789";
        let replaced = result.apply_replacements(text);
        assert!(replaced.contains("***"));
        assert!(replaced.contains("[隐藏]"));
        assert!(!replaced.contains("密码"));
        assert!(!replaced.contains("password"));
        assert!(!replaced.contains("身份证"));
    }

    #[test]
    fn test_ruleset_save_load_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rules.yaml");

        let mut rs = RuleSet::new("emp1");
        rs.add(
            Rule::new(
                "r1",
                "测试",
                RuleCondition::any(vec!["密码".to_string()]),
                RuleAction::Refuse {
                    message: "拒绝".to_string(),
                },
            )
            .with_priority(100),
        );
        rs.save_to_file(&path).unwrap();

        let loaded = RuleSet::load_from_file(&path).unwrap();
        assert_eq!(rs.rules.len(), loaded.rules.len());
        assert_eq!(rs.rules[0].id, loaded.rules[0].id);
    }
}
