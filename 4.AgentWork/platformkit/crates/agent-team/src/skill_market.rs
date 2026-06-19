// Skills 市场（M5.2 精简版）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M5.2
//
// v2.0 调整：
//   - v1.0 写"5 个内置 Skills" → v2.0 砍到 2 个（developer / customer_service）
//   - 不做"Skills 市场下载"（宝塔 8G 内存跑不动 marketplace）
//
// Skills = 员工能力插件（类似 WordPress 插件）
//   - 扩展员工能力（如代码生成 / 工单路由 / 情感分析）
//   - 可被任意员工加载（通过 skill_ids 字段）
//   - 内置 2 个：developer（开发）+ customer_service（客服）
//
// 与 EmployeeDefinition 的关系：
//   - EmployeeDefinition.tools = 工具 ID 列表（如 "code_gen" / "ticket_route"）
//   - Skill.tools = 该 Skill 提供的工具列表
//   - 员工加载 Skill 后，获得 Skill 提供的所有工具

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 类型定义
// ============================================================================

/// Skill ID 类型（如 "developer" / "customer_service"）
pub type SkillId = String;

/// Tool ID 类型（如 "code_gen" / "ticket_route"）
pub type ToolId = String;

/// Skill 分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SkillCategory {
    /// 开发类（代码生成 / 审查 / 调试）
    Developer,
    /// 客服类（FAQ / 工单 / 情感分析）
    CustomerService,
    /// 通信类（邮件 / 即时消息 / 通知）
    Communication,
    /// 数据类（数据库 / API / 文件）
    Data,
    /// 生产力类（日历 / 待办 / 笔记）
    Productivity,
}

impl SkillCategory {
    /// 获取分类的中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::Developer => "开发",
            Self::CustomerService => "客服",
            Self::Communication => "通信",
            Self::Data => "数据",
            Self::Productivity => "生产力",
        }
    }

    /// 获取分类的英文标识
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::CustomerService => "customer_service",
            Self::Communication => "communication",
            Self::Data => "data",
            Self::Productivity => "productivity",
        }
    }

    /// 从字符串解析分类
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "developer" => Some(Self::Developer),
            "customer_service" => Some(Self::CustomerService),
            "communication" => Some(Self::Communication),
            "data" => Some(Self::Data),
            "productivity" => Some(Self::Productivity),
            _ => None,
        }
    }
}

/// Skill 工具定义
///
/// 描述 Skill 提供的一个具体工具（function calling）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    /// 工具 ID（唯一，如 "code_gen" / "ticket_route"）
    pub id: ToolId,
    /// 工具名称（展示用，如"代码生成"）
    pub name: String,
    /// 工具描述（给 LLM 看，告诉它何时使用此工具）
    pub description: String,
    /// 输入参数（JSON Schema 格式，简化版）
    #[serde(default)]
    pub parameters: Vec<ToolParameter>,
    /// 输出格式描述
    #[serde(default)]
    pub output_format: String,
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// 参数名
    pub name: String,
    /// 参数类型（string / number / boolean / object / array）
    pub param_type: String,
    /// 参数描述
    pub description: String,
    /// 是否必填
    pub required: bool,
    /// 默认值（可选）
    #[serde(default)]
    pub default: Option<String>,
}

/// Skill 定义
///
/// 一个 Skill = 一组相关工具的集合，可被员工加载以扩展能力。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill ID（唯一，如 "developer" / "customer_service"）
    pub id: SkillId,
    /// Skill 名称（展示用，如"开发技能包"）
    pub name: String,
    /// Skill 描述
    pub description: String,
    /// 分类
    pub category: SkillCategory,
    /// 版本号（语义化版本）
    pub version: String,
    /// 提供的工具列表
    pub tools: Vec<SkillTool>,
    /// 适用员工角色（空 = 通用，所有员工可加载）
    #[serde(default)]
    pub applicable_roles: Vec<String>,
    /// 系统提示词补充（加载此 Skill 时追加到员工 system_prompt）
    #[serde(default)]
    pub system_prompt_supplement: String,
    /// 是否内置（true = 内置，false = 用户自定义）
    #[serde(default)]
    pub builtin: bool,
    /// 作者（用户自定义 Skill 时填写）
    #[serde(default)]
    pub author: Option<String>,
    /// 标签（便于搜索）
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Skill {
    /// 创建新的 Skill
    pub fn new(id: impl Into<String>, name: impl Into<String>, category: SkillCategory) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category,
            version: "1.0.0".to_string(),
            tools: Vec::new(),
            applicable_roles: Vec::new(),
            system_prompt_supplement: String::new(),
            builtin: false,
            author: None,
            tags: Vec::new(),
        }
    }

    /// 添加工具
    pub fn with_tool(mut self, tool: SkillTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置系统提示词补充
    pub fn with_prompt_supplement(mut self, supplement: impl Into<String>) -> Self {
        self.system_prompt_supplement = supplement.into();
        self
    }

    /// 设置适用角色
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.applicable_roles = roles;
        self
    }

    /// 设置标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 标记为内置
    pub fn mark_builtin(mut self) -> Self {
        self.builtin = true;
        self
    }

    /// 获取所有工具 ID（返回 owned String 便于使用）
    pub fn tool_ids(&self) -> Vec<ToolId> {
        self.tools.iter().map(|t| t.id.clone()).collect()
    }

    /// 检查是否适用于指定角色
    pub fn is_applicable_to(&self, role: &str) -> bool {
        // applicable_roles 为空 = 通用，适用于所有角色
        if self.applicable_roles.is_empty() {
            return true;
        }
        self.applicable_roles.iter().any(|r| r == role)
    }

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

// ============================================================================
// Skill 市场
// ============================================================================

/// Skill 市场注册表
///
/// 管理所有可用的 Skills（内置 + 用户自定义）。
/// MVP 阶段不做"市场下载"，仅本地注册。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarket {
    /// 已注册的 Skills（按 ID 索引）
    skills: HashMap<SkillId, Skill>,
}

impl Default for SkillMarket {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillMarket {
    /// 创建空的 Skill 市场
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// 创建包含所有内置 Skills 的市场
    pub fn with_builtins() -> Self {
        let mut market = Self::new();
        for skill in builtin_skills() {
            market.register(skill);
        }
        market
    }

    /// 注册一个 Skill
    pub fn register(&mut self, skill: Skill) -> bool {
        let id = skill.id.clone();
        if self.skills.contains_key(&id) {
            return false; // 已存在
        }
        self.skills.insert(id, skill);
        true
    }

    /// 注销一个 Skill（内置 Skill 不可注销）
    pub fn unregister(&mut self, id: &str) -> Option<Skill> {
        if let Some(skill) = self.skills.get(id) {
            if skill.builtin {
                return None; // 内置不可删除
            }
        }
        self.skills.remove(id)
    }

    /// 获取 Skill
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// 获取所有 Skills
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// 按分类筛选
    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| &s.category == category)
            .collect()
    }

    /// 按角色筛选（返回适用于该角色的 Skills）
    pub fn list_by_role(&self, role: &str) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.is_applicable_to(role))
            .collect()
    }

    /// 搜索 Skills（按名称 / 描述 / 标签匹配）
    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let q = query.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// 获取所有工具 ID（从所有 Skills 聚合）
    pub fn all_tool_ids(&self) -> Vec<ToolId> {
        self.skills
            .values()
            .flat_map(|s| s.tools.iter().map(|t| t.id.clone()))
            .collect()
    }

    /// 根据工具 ID 查找所属 Skill
    pub fn find_skill_by_tool(&self, tool_id: &str) -> Option<&Skill> {
        self.skills
            .values()
            .find(|s| s.tools.iter().any(|t| t.id == tool_id))
    }

    /// 根据多个 Skill ID 加载完整 Skills
    pub fn load_skills(&self, skill_ids: &[SkillId]) -> Vec<&Skill> {
        skill_ids
            .iter()
            .filter_map(|id| self.skills.get(id))
            .collect()
    }

    /// 获取统计信息
    pub fn stats(&self) -> SkillMarketStats {
        let total = self.skills.len();
        let builtin = self.skills.values().filter(|s| s.builtin).count();
        let custom = total - builtin;
        let total_tools: usize = self.skills.values().map(|s| s.tool_count()).sum();

        let mut by_category: HashMap<SkillCategory, usize> = HashMap::new();
        for skill in self.skills.values() {
            *by_category.entry(skill.category.clone()).or_default() += 1;
        }

        SkillMarketStats {
            total_skills: total,
            builtin_skills: builtin,
            custom_skills: custom,
            total_tools,
            by_category,
        }
    }
}

/// Skill 市场统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketStats {
    /// Skill 总数
    pub total_skills: usize,
    /// 内置 Skill 数
    pub builtin_skills: usize,
    /// 自定义 Skill 数
    pub custom_skills: usize,
    /// 工具总数
    pub total_tools: usize,
    /// 按分类统计
    pub by_category: HashMap<SkillCategory, usize>,
}

// ============================================================================
// 内置 Skills（M5.2 精简版：2 个）
// ============================================================================

/// 获取所有内置 Skills
pub fn builtin_skills() -> Vec<Skill> {
    vec![builtin_developer_skill(), builtin_customer_service_skill()]
}

/// 按 ID 获取内置 Skill
pub fn builtin_skill_by_id(id: &str) -> Option<Skill> {
    builtin_skills().into_iter().find(|s| s.id == id)
}

/// 内置 Skill 1：开发技能包（developer）
///
/// 提供代码生成 / 审查 / 调试 / 重构等工具，适用于开发工程师角色。
pub fn builtin_developer_skill() -> Skill {
    let mut skill = Skill::new("developer", "开发技能包", SkillCategory::Developer)
        .with_description("为开发工程师提供代码生成、审查、调试、重构等能力。包含 6 个工具。")
        .with_prompt_supplement(
            "你是一个专业的开发工程师。你可以使用以下工具来辅助开发工作：\n\
             - code_gen: 根据需求生成代码\n\
             - code_review: 审查代码质量\n\
             - debug: 调试代码问题\n\
             - refactor: 重构代码结构\n\
             - test_gen: 生成单元测试\n\
             - doc_gen: 生成代码文档\n\
             在使用工具前，先理解用户需求，选择最合适的工具。"
        )
        .with_roles(vec![
            "write_code".to_string(),
            "code_review".to_string(),
            "debug".to_string(),
        ])
        .with_tags(vec![
            "开发".to_string(),
            "代码".to_string(),
            "官方".to_string(),
        ])
        .mark_builtin();

    // 添加 6 个开发工具
    skill.tools.push(SkillTool {
        id: "code_gen".to_string(),
        name: "代码生成".to_string(),
        description: "根据自然语言需求描述生成代码。支持多种编程语言。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "language".to_string(),
                param_type: "string".to_string(),
                description: "目标编程语言（如 rust / python / typescript）".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "requirement".to_string(),
                param_type: "string".to_string(),
                description: "需求描述（自然语言）".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "context".to_string(),
                param_type: "string".to_string(),
                description: "上下文代码（可选，用于参考现有代码风格）".to_string(),
                required: false,
                default: None,
            },
        ],
        output_format: "生成的代码（带注释）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "code_review".to_string(),
        name: "代码审查".to_string(),
        description: "审查代码质量，发现潜在 bug、安全漏洞、性能问题。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "code".to_string(),
                param_type: "string".to_string(),
                description: "待审查的代码".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "focus".to_string(),
                param_type: "string".to_string(),
                description: "审查重点（如 security / performance / readability）".to_string(),
                required: false,
                default: Some("all".to_string()),
            },
        ],
        output_format: "审查报告（问题列表 + 严重程度 + 修复建议）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "debug".to_string(),
        name: "代码调试".to_string(),
        description: "分析错误信息，定位 bug 根因，提供修复方案。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "error_message".to_string(),
                param_type: "string".to_string(),
                description: "错误信息".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "code".to_string(),
                param_type: "string".to_string(),
                description: "相关代码".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "stack_trace".to_string(),
                param_type: "string".to_string(),
                description: "堆栈跟踪（可选）".to_string(),
                required: false,
                default: None,
            },
        ],
        output_format: "调试报告（根因分析 + 修复方案 + 修复后代码）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "refactor".to_string(),
        name: "代码重构".to_string(),
        description: "重构代码结构，提升可读性、可维护性，不改变功能。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "code".to_string(),
                param_type: "string".to_string(),
                description: "待重构的代码".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "goal".to_string(),
                param_type: "string".to_string(),
                description: "重构目标（如 extract_function / simplify / deduplicate）".to_string(),
                required: true,
                default: None,
            },
        ],
        output_format: "重构后代码 + 重构说明".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "test_gen".to_string(),
        name: "测试生成".to_string(),
        description: "为指定代码生成单元测试用例。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "code".to_string(),
                param_type: "string".to_string(),
                description: "待测试的代码".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "framework".to_string(),
                param_type: "string".to_string(),
                description: "测试框架（如 pytest / jest / cargo test）".to_string(),
                required: false,
                default: Some("auto".to_string()),
            },
        ],
        output_format: "单元测试代码（含测试用例说明）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "doc_gen".to_string(),
        name: "文档生成".to_string(),
        description: "为代码生成文档注释 / API 文档 / README。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "code".to_string(),
                param_type: "string".to_string(),
                description: "代码内容".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "format".to_string(),
                param_type: "string".to_string(),
                description: "文档格式（如 docstring / markdown / openapi）".to_string(),
                required: false,
                default: Some("docstring".to_string()),
            },
        ],
        output_format: "文档内容".to_string(),
    });

    skill
}

/// 内置 Skill 2：客服技能包（customer_service）
///
/// 提供 FAQ 检索 / 工单路由 / 情感分析 / 退换货处理等工具，适用于客服角色。
pub fn builtin_customer_service_skill() -> Skill {
    let mut skill = Skill::new("customer_service", "客服技能包", SkillCategory::CustomerService)
        .with_description("为客服人员提供 FAQ 检索、工单路由、情感分析、退换货处理等能力。包含 6 个工具。")
        .with_prompt_supplement(
            "你是一个专业的客服人员。你可以使用以下工具来提升服务效率：\n\
             - faq_search: 检索常见问题库\n\
             - ticket_route: 智能工单路由\n\
             - sentiment_analyze: 情感分析\n\
             - refund_process: 退换货流程\n\
             - order_query: 订单查询\n\
             - escalation: 转人工\n\
             始终保持友好、专业的态度。遇到无法解决的问题时，及时使用 escalation 转人工。"
        )
        .with_roles(vec![
            "answer_questions".to_string(),
            "customer_service".to_string(),
            "after_sale".to_string(),
        ])
        .with_tags(vec![
            "客服".to_string(),
            "售后".to_string(),
            "官方".to_string(),
        ])
        .mark_builtin();

    // 添加 6 个客服工具
    skill.tools.push(SkillTool {
        id: "faq_search".to_string(),
        name: "FAQ 检索".to_string(),
        description: "从常见问题库中检索最匹配的答案。".to_string(),
        parameters: vec![ToolParameter {
            name: "query".to_string(),
            param_type: "string".to_string(),
            description: "用户问题".to_string(),
            required: true,
            default: None,
        }],
        output_format: "匹配的 FAQ 列表（含问题、答案、相关度）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "ticket_route".to_string(),
        name: "工单路由".to_string(),
        description: "根据用户问题内容，自动路由到合适的处理团队。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "content".to_string(),
                param_type: "string".to_string(),
                description: "用户问题描述".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "priority".to_string(),
                param_type: "string".to_string(),
                description: "优先级（low / medium / high / urgent）".to_string(),
                required: false,
                default: Some("medium".to_string()),
            },
        ],
        output_format: "路由结果（目标团队 + 优先级 + 预计处理时间）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "sentiment_analyze".to_string(),
        name: "情感分析".to_string(),
        description: "分析用户文本的情感倾向，辅助判断是否需要转人工。".to_string(),
        parameters: vec![ToolParameter {
            name: "text".to_string(),
            param_type: "string".to_string(),
            description: "待分析的文本".to_string(),
            required: true,
            default: None,
        }],
        output_format: "情感分析结果（positive / neutral / negative + 置信度 + 建议）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "refund_process".to_string(),
        name: "退换货处理".to_string(),
        description: "处理退换货请求，查询订单状态，生成退换货工单。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "order_id".to_string(),
                param_type: "string".to_string(),
                description: "订单号".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "reason".to_string(),
                param_type: "string".to_string(),
                description: "退换货原因".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "type".to_string(),
                param_type: "string".to_string(),
                description: "类型（refund / exchange / repair）".to_string(),
                required: false,
                default: Some("refund".to_string()),
            },
        ],
        output_format: "退换货工单（工单号 + 状态 + 预计处理时间）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "order_query".to_string(),
        name: "订单查询".to_string(),
        description: "查询订单状态、物流信息、支付状态。".to_string(),
        parameters: vec![ToolParameter {
            name: "order_id".to_string(),
            param_type: "string".to_string(),
            description: "订单号".to_string(),
            required: true,
            default: None,
        }],
        output_format: "订单详情（状态 + 物流 + 支付 + 商品列表）".to_string(),
    });

    skill.tools.push(SkillTool {
        id: "escalation".to_string(),
        name: "转人工".to_string(),
        description: "当 AI 无法处理或用户明确要求时，转接人工客服。".to_string(),
        parameters: vec![
            ToolParameter {
                name: "reason".to_string(),
                param_type: "string".to_string(),
                description: "转人工原因".to_string(),
                required: true,
                default: None,
            },
            ToolParameter {
                name: "priority".to_string(),
                param_type: "string".to_string(),
                description: "优先级（low / medium / high / urgent）".to_string(),
                required: false,
                default: Some("medium".to_string()),
            },
        ],
        output_format: "转人工工单（工单号 + 预计等待时间）".to_string(),
    });

    skill
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------
    // SkillCategory 测试
    // ------------------------------------------------------------

    #[test]
    fn test_category_label() {
        assert_eq!(SkillCategory::Developer.label(), "开发");
        assert_eq!(SkillCategory::CustomerService.label(), "客服");
        assert_eq!(SkillCategory::Communication.label(), "通信");
        assert_eq!(SkillCategory::Data.label(), "数据");
        assert_eq!(SkillCategory::Productivity.label(), "生产力");
    }

    #[test]
    fn test_category_as_str() {
        assert_eq!(SkillCategory::Developer.as_str(), "developer");
        assert_eq!(SkillCategory::CustomerService.as_str(), "customer_service");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(SkillCategory::from_str("developer"), Some(SkillCategory::Developer));
        assert_eq!(SkillCategory::from_str("customer_service"), Some(SkillCategory::CustomerService));
        assert_eq!(SkillCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_category_roundtrip() {
        for cat in [
            SkillCategory::Developer,
            SkillCategory::CustomerService,
            SkillCategory::Communication,
            SkillCategory::Data,
            SkillCategory::Productivity,
        ] {
            let s = cat.as_str();
            assert_eq!(SkillCategory::from_str(s), Some(cat.clone()));
        }
    }

    // ------------------------------------------------------------
    // Skill 测试
    // ------------------------------------------------------------

    #[test]
    fn test_skill_new() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer);
        assert_eq!(skill.id, "test");
        assert_eq!(skill.name, "测试");
        assert_eq!(skill.category, SkillCategory::Developer);
        assert_eq!(skill.version, "1.0.0");
        assert!(skill.tools.is_empty());
        assert!(!skill.builtin);
    }

    #[test]
    fn test_skill_with_tool() {
        let tool = SkillTool {
            id: "tool1".to_string(),
            name: "工具1".to_string(),
            description: "测试工具".to_string(),
            parameters: vec![],
            output_format: "text".to_string(),
        };
        let skill = Skill::new("test", "测试", SkillCategory::Developer).with_tool(tool);
        assert_eq!(skill.tool_count(), 1);
        assert_eq!(skill.tool_ids(), vec!["tool1".to_string()]);
    }

    #[test]
    fn test_skill_with_description() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_description("这是一个测试 Skill");
        assert_eq!(skill.description, "这是一个测试 Skill");
    }

    #[test]
    fn test_skill_with_prompt_supplement() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_prompt_supplement("补充提示词");
        assert_eq!(skill.system_prompt_supplement, "补充提示词");
    }

    #[test]
    fn test_skill_with_roles() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_roles(vec!["role1".to_string(), "role2".to_string()]);
        assert_eq!(skill.applicable_roles.len(), 2);
    }

    #[test]
    fn test_skill_with_tags() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(skill.tags.len(), 2);
    }

    #[test]
    fn test_skill_mark_builtin() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer).mark_builtin();
        assert!(skill.builtin);
    }

    #[test]
    fn test_skill_is_applicable_to_empty_roles() {
        // applicable_roles 为空 = 通用，适用于所有角色
        let skill = Skill::new("test", "测试", SkillCategory::Developer);
        assert!(skill.is_applicable_to("any_role"));
        assert!(skill.is_applicable_to("developer"));
    }

    #[test]
    fn test_skill_is_applicable_to_specific_roles() {
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_roles(vec!["write_code".to_string(), "debug".to_string()]);
        assert!(skill.is_applicable_to("write_code"));
        assert!(skill.is_applicable_to("debug"));
        assert!(!skill.is_applicable_to("customer_service"));
    }

    #[test]
    fn test_skill_tool_ids() {
        let tool1 = SkillTool {
            id: "tool1".to_string(),
            name: "工具1".to_string(),
            description: "".to_string(),
            parameters: vec![],
            output_format: "".to_string(),
        };
        let tool2 = SkillTool {
            id: "tool2".to_string(),
            name: "工具2".to_string(),
            description: "".to_string(),
            parameters: vec![],
            output_format: "".to_string(),
        };
        let skill = Skill::new("test", "测试", SkillCategory::Developer)
            .with_tool(tool1)
            .with_tool(tool2);
        assert_eq!(skill.tool_ids().len(), 2);
        assert!(skill.tool_ids().contains(&"tool1".to_string()));
        assert!(skill.tool_ids().contains(&"tool2".to_string()));
    }

    // ------------------------------------------------------------
    // SkillMarket 测试
    // ------------------------------------------------------------

    #[test]
    fn test_market_new() {
        let market = SkillMarket::new();
        assert_eq!(market.list().len(), 0);
    }

    #[test]
    fn test_market_with_builtins() {
        let market = SkillMarket::with_builtins();
        assert_eq!(market.list().len(), 2);
        assert!(market.get("developer").is_some());
        assert!(market.get("customer_service").is_some());
    }

    #[test]
    fn test_market_register() {
        let mut market = SkillMarket::new();
        let skill = Skill::new("custom", "自定义", SkillCategory::Data);
        assert!(market.register(skill));
        assert_eq!(market.list().len(), 1);
    }

    #[test]
    fn test_market_register_duplicate() {
        let mut market = SkillMarket::new();
        let skill1 = Skill::new("custom", "自定义1", SkillCategory::Data);
        let skill2 = Skill::new("custom", "自定义2", SkillCategory::Data);
        assert!(market.register(skill1));
        assert!(!market.register(skill2)); // 重复 ID
        assert_eq!(market.list().len(), 1);
    }

    #[test]
    fn test_market_unregister_custom() {
        let mut market = SkillMarket::new();
        let skill = Skill::new("custom", "自定义", SkillCategory::Data);
        market.register(skill);
        assert!(market.unregister("custom").is_some());
        assert_eq!(market.list().len(), 0);
    }

    #[test]
    fn test_market_unregister_builtin_fails() {
        let mut market = SkillMarket::with_builtins();
        // 内置 Skill 不可删除
        assert!(market.unregister("developer").is_none());
        assert!(market.get("developer").is_some());
    }

    #[test]
    fn test_market_get() {
        let market = SkillMarket::with_builtins();
        let dev = market.get("developer").unwrap();
        assert_eq!(dev.name, "开发技能包");
        assert_eq!(dev.category, SkillCategory::Developer);
    }

    #[test]
    fn test_market_get_not_found() {
        let market = SkillMarket::with_builtins();
        assert!(market.get("nonexistent").is_none());
    }

    #[test]
    fn test_market_list_by_category() {
        let market = SkillMarket::with_builtins();
        let dev_skills = market.list_by_category(&SkillCategory::Developer);
        let cs_skills = market.list_by_category(&SkillCategory::CustomerService);
        assert_eq!(dev_skills.len(), 1);
        assert_eq!(cs_skills.len(), 1);
    }

    #[test]
    fn test_market_list_by_role() {
        let market = SkillMarket::with_builtins();
        // developer skill 适用于 write_code 角色
        let code_skills = market.list_by_role("write_code");
        assert_eq!(code_skills.len(), 1);
        assert_eq!(code_skills[0].id, "developer");

        // customer_service skill 适用于 answer_questions 角色
        let cs_skills = market.list_by_role("answer_questions");
        assert_eq!(cs_skills.len(), 1);
        assert_eq!(cs_skills[0].id, "customer_service");

        // 未知角色应返回空
        let unknown = market.list_by_role("unknown_role");
        assert_eq!(unknown.len(), 0);
    }

    #[test]
    fn test_market_search_by_name() {
        let market = SkillMarket::with_builtins();
        let results = market.search("开发");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "developer");
    }

    #[test]
    fn test_market_search_by_description() {
        let market = SkillMarket::with_builtins();
        let results = market.search("客服");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "customer_service");
    }

    #[test]
    fn test_market_search_by_tag() {
        let market = SkillMarket::with_builtins();
        let results = market.search("官方");
        assert_eq!(results.len(), 2); // 两个内置 Skill 都有"官方"标签
    }

    #[test]
    fn test_market_search_no_results() {
        let market = SkillMarket::with_builtins();
        let results = market.search("不存在的关键词");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_market_all_tool_ids() {
        let market = SkillMarket::with_builtins();
        let tool_ids = market.all_tool_ids();
        // developer 6 个 + customer_service 6 个 = 12 个
        assert_eq!(tool_ids.len(), 12);
        assert!(tool_ids.contains(&"code_gen".to_string()));
        assert!(tool_ids.contains(&"faq_search".to_string()));
    }

    #[test]
    fn test_market_find_skill_by_tool() {
        let market = SkillMarket::with_builtins();
        let skill = market.find_skill_by_tool("code_gen").unwrap();
        assert_eq!(skill.id, "developer");

        let skill = market.find_skill_by_tool("faq_search").unwrap();
        assert_eq!(skill.id, "customer_service");

        assert!(market.find_skill_by_tool("nonexistent").is_none());
    }

    #[test]
    fn test_market_load_skills() {
        let market = SkillMarket::with_builtins();
        let loaded = market.load_skills(&["developer".to_string(), "customer_service".to_string()]);
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_market_load_skills_with_unknown() {
        let market = SkillMarket::with_builtins();
        let loaded = market.load_skills(&[
            "developer".to_string(),
            "unknown".to_string(),
            "customer_service".to_string(),
        ]);
        // 未知的会被过滤掉
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_market_stats() {
        let market = SkillMarket::with_builtins();
        let stats = market.stats();
        assert_eq!(stats.total_skills, 2);
        assert_eq!(stats.builtin_skills, 2);
        assert_eq!(stats.custom_skills, 0);
        assert_eq!(stats.total_tools, 12); // 6 + 6
        assert_eq!(stats.by_category.get(&SkillCategory::Developer), Some(&1));
        assert_eq!(stats.by_category.get(&SkillCategory::CustomerService), Some(&1));
    }

    #[test]
    fn test_market_stats_with_custom() {
        let mut market = SkillMarket::with_builtins();
        let custom = Skill::new("custom", "自定义", SkillCategory::Data)
            .with_tool(SkillTool {
                id: "custom_tool".to_string(),
                name: "自定义工具".to_string(),
                description: "".to_string(),
                parameters: vec![],
                output_format: "".to_string(),
            });
        market.register(custom);

        let stats = market.stats();
        assert_eq!(stats.total_skills, 3);
        assert_eq!(stats.builtin_skills, 2);
        assert_eq!(stats.custom_skills, 1);
        assert_eq!(stats.total_tools, 13); // 12 + 1
    }

    // ------------------------------------------------------------
    // 内置 Skills 测试
    // ------------------------------------------------------------

    #[test]
    fn test_builtin_skills_count() {
        let skills = builtin_skills();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn test_builtin_by_id_found() {
        let skill = builtin_skill_by_id("developer").unwrap();
        assert_eq!(skill.name, "开发技能包");
        assert!(skill.builtin);
    }

    #[test]
    fn test_builtin_by_id_not_found() {
        assert!(builtin_skill_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_developer_skill() {
        let skill = builtin_developer_skill();
        assert_eq!(skill.id, "developer");
        assert_eq!(skill.name, "开发技能包");
        assert_eq!(skill.category, SkillCategory::Developer);
        assert!(skill.builtin);
        assert_eq!(skill.tool_count(), 6);

        // 验证工具 ID
        let tool_ids = skill.tool_ids();
        assert!(tool_ids.contains(&"code_gen".to_string()));
        assert!(tool_ids.contains(&"code_review".to_string()));
        assert!(tool_ids.contains(&"debug".to_string()));
        assert!(tool_ids.contains(&"refactor".to_string()));
        assert!(tool_ids.contains(&"test_gen".to_string()));
        assert!(tool_ids.contains(&"doc_gen".to_string()));

        // 验证适用角色
        assert!(skill.is_applicable_to("write_code"));
        assert!(skill.is_applicable_to("code_review"));
        assert!(skill.is_applicable_to("debug"));
        assert!(!skill.is_applicable_to("answer_questions"));

        // 验证系统提示词补充
        assert!(!skill.system_prompt_supplement.is_empty());
        assert!(skill.system_prompt_supplement.contains("code_gen"));

        // 验证标签
        assert!(skill.tags.contains(&"开发".to_string()));
        assert!(skill.tags.contains(&"官方".to_string()));
    }

    #[test]
    fn test_builtin_customer_service_skill() {
        let skill = builtin_customer_service_skill();
        assert_eq!(skill.id, "customer_service");
        assert_eq!(skill.name, "客服技能包");
        assert_eq!(skill.category, SkillCategory::CustomerService);
        assert!(skill.builtin);
        assert_eq!(skill.tool_count(), 6);

        // 验证工具 ID
        let tool_ids = skill.tool_ids();
        assert!(tool_ids.contains(&"faq_search".to_string()));
        assert!(tool_ids.contains(&"ticket_route".to_string()));
        assert!(tool_ids.contains(&"sentiment_analyze".to_string()));
        assert!(tool_ids.contains(&"refund_process".to_string()));
        assert!(tool_ids.contains(&"order_query".to_string()));
        assert!(tool_ids.contains(&"escalation".to_string()));

        // 验证适用角色
        assert!(skill.is_applicable_to("answer_questions"));
        assert!(skill.is_applicable_to("customer_service"));
        assert!(skill.is_applicable_to("after_sale"));
        assert!(!skill.is_applicable_to("write_code"));

        // 验证系统提示词补充
        assert!(!skill.system_prompt_supplement.is_empty());
        assert!(skill.system_prompt_supplement.contains("faq_search"));
        assert!(skill.system_prompt_supplement.contains("escalation"));

        // 验证标签
        assert!(skill.tags.contains(&"客服".to_string()));
        assert!(skill.tags.contains(&"官方".to_string()));
    }

    #[test]
    fn test_builtin_skills_all_have_tools() {
        for skill in builtin_skills() {
            assert!(skill.tool_count() > 0, "Skill {} 应该有工具", skill.id);
        }
    }

    #[test]
    fn test_builtin_skills_all_have_prompt_supplement() {
        for skill in builtin_skills() {
            assert!(
                !skill.system_prompt_supplement.is_empty(),
                "Skill {} 应该有系统提示词补充",
                skill.id
            );
        }
    }

    #[test]
    fn test_builtin_skills_all_marked_builtin() {
        for skill in builtin_skills() {
            assert!(skill.builtin, "Skill {} 应该标记为内置", skill.id);
        }
    }

    #[test]
    fn test_builtin_skills_unique_ids() {
        let skills = builtin_skills();
        let mut ids: Vec<_> = skills.iter().map(|s| s.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), skills.len(), "内置 Skill ID 应该唯一");
    }

    #[test]
    fn test_builtin_skills_unique_tool_ids() {
        let skills = builtin_skills();
        let mut all_tool_ids: Vec<_> = skills
            .iter()
            .flat_map(|s| s.tools.iter().map(|t| t.id.clone()))
            .collect();
        all_tool_ids.sort();
        all_tool_ids.dedup();
        let total_tools: usize = skills.iter().map(|s| s.tool_count()).sum();
        assert_eq!(all_tool_ids.len(), total_tools, "工具 ID 应该全局唯一");
    }

    // ------------------------------------------------------------
    // SkillTool 测试
    // ------------------------------------------------------------

    #[test]
    fn test_skill_tool_serialization() {
        let tool = SkillTool {
            id: "test_tool".to_string(),
            name: "测试工具".to_string(),
            description: "用于测试".to_string(),
            parameters: vec![ToolParameter {
                name: "input".to_string(),
                param_type: "string".to_string(),
                description: "输入".to_string(),
                required: true,
                default: None,
            }],
            output_format: "text".to_string(),
        };

        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: SkillTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test_tool");
        assert_eq!(deserialized.parameters.len(), 1);
        assert!(deserialized.parameters[0].required);
    }

    #[test]
    fn test_skill_serialization() {
        let skill = builtin_developer_skill();
        let json = serde_json::to_string(&skill).unwrap();
        let deserialized: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "developer");
        assert_eq!(deserialized.tool_count(), 6);
        assert!(deserialized.builtin);
    }

    #[test]
    fn test_skill_market_serialization() {
        let market = SkillMarket::with_builtins();
        let json = serde_json::to_string(&market).unwrap();
        let deserialized: SkillMarket = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.list().len(), 2);
        assert!(deserialized.get("developer").is_some());
    }

    // ------------------------------------------------------------
    // 集成测试
    // ------------------------------------------------------------

    #[test]
    fn test_integration_register_custom_and_use() {
        let mut market = SkillMarket::with_builtins();

        // 注册自定义 Skill
        let custom = Skill::new("custom_analytics", "数据分析", SkillCategory::Data)
            .with_description("自定义数据分析技能")
            .with_tool(SkillTool {
                id: "data_query".to_string(),
                name: "数据查询".to_string(),
                description: "查询数据库".to_string(),
                parameters: vec![ToolParameter {
                    name: "sql".to_string(),
                    param_type: "string".to_string(),
                    description: "SQL 查询".to_string(),
                    required: true,
                    default: None,
                }],
                output_format: "查询结果".to_string(),
            })
            .with_tags(vec!["数据".to_string(), "自定义".to_string()]);

        assert!(market.register(custom));
        assert_eq!(market.list().len(), 3);

        // 搜索自定义 Skill
        let results = market.search("数据分析");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "custom_analytics");

        // 查找工具所属 Skill
        let skill = market.find_skill_by_tool("data_query").unwrap();
        assert_eq!(skill.id, "custom_analytics");

        // 删除自定义 Skill
        assert!(market.unregister("custom_analytics").is_some());
        assert_eq!(market.list().len(), 2);
    }

    #[test]
    fn test_integration_employee_role_matching() {
        let market = SkillMarket::with_builtins();

        // 开发工程师角色
        let dev_skills = market.list_by_role("write_code");
        assert_eq!(dev_skills.len(), 1);
        assert_eq!(dev_skills[0].id, "developer");

        // 客服角色
        let cs_skills = market.list_by_role("answer_questions");
        assert_eq!(cs_skills.len(), 1);
        assert_eq!(cs_skills[0].id, "customer_service");

        // 售后角色
        let after_sale_skills = market.list_by_role("after_sale");
        assert_eq!(after_sale_skills.len(), 1);
        assert_eq!(after_sale_skills[0].id, "customer_service");
    }

    #[test]
    fn test_integration_full_workflow() {
        // 1. 创建市场
        let mut market = SkillMarket::with_builtins();

        // 2. 验证初始状态
        let stats = market.stats();
        assert_eq!(stats.total_skills, 2);
        assert_eq!(stats.total_tools, 12);

        // 3. 加载多个 Skills
        let loaded = market.load_skills(&["developer".to_string(), "customer_service".to_string()]);
        assert_eq!(loaded.len(), 2);

        // 4. 获取所有工具 ID
        let all_tools = market.all_tool_ids();
        assert_eq!(all_tools.len(), 12);

        // 5. 按分类筛选
        let dev_skills = market.list_by_category(&SkillCategory::Developer);
        assert_eq!(dev_skills.len(), 1);

        // 6. 搜索
        let search_results = market.search("代码");
        assert!(!search_results.is_empty());

        // 7. 添加自定义 Skill
        let custom = Skill::new("my_skill", "我的技能", SkillCategory::Productivity)
            .with_description("自定义生产力工具");
        market.register(custom);

        // 8. 验证统计更新
        let new_stats = market.stats();
        assert_eq!(new_stats.total_skills, 3);
        assert_eq!(new_stats.custom_skills, 1);
    }
}
