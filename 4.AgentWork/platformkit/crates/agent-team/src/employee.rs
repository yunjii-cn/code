// AI 员工定义系统（M4.0 D2）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.0 D2
// 指南: docs/EMPLOYEE-GUIDE.md
//
// EmployeeDefinition = 单个 AI 员工的完整定义（独立于团队模板）
//   - 岗位名 + 角色 + 职责描述
//   - 使用的 LLM 模型
//   - 系统提示词（基础 prompt）
//   - 技能标签 + MCP 工具列表
//   - 知识库引用（M4.1 培训引擎用）
//   - 自定义元数据
//
// 与 TeamTemplate 的关系：
//   - TeamTemplate = 一组 Role 协作（多 Agent DAG）
//   - EmployeeDefinition = 单个 AI 员工独立工作（客服/销售/财务/...）
//   - 一个 Employee 可被加入 TeamTemplate 作为 Role
//
// 存储格式：YAML 文件（.yunji/employees/{id}.yml）
//   - 可读可改，git 友好
//   - 不依赖数据库（MVP 阶段）
//   - 用户可基于内置模板自定义

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// AI 员工定义
///
/// 一个 EmployeeDefinition 描述单个 AI 员工的所有信息，
/// 可独立工作（单 Agent 场景），也可加入团队（作为 Role）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeDefinition {
    /// 员工 ID（唯一标识，如 "customer_service" / "fashion_stylist"）
    pub id: String,
    /// 岗位名（展示用，如"客服" / "穿搭师"）
    pub name: String,
    /// 角色标识（如 "answer_questions" / "style_recommendation"）
    pub role: String,
    /// 职责描述（给用户看）
    #[serde(default)]
    pub description: String,
    /// 使用的 LLM 模型 ID
    pub model_id: String,
    /// 备用模型列表（故障转移顺序）
    #[serde(default)]
    pub model_fallback: Vec<String>,
    /// 系统提示词（基础 prompt）
    pub system_prompt: String,
    /// 技能标签（能力描述）
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// MCP 工具列表（可调用的工具）
    #[serde(default)]
    pub tools: Vec<String>,
    /// 知识库 ID（M4.1 培训引擎用，None = 无知识库）
    #[serde(default)]
    pub knowledge_base: Option<String>,
    /// 规则列表（M4.1 规则引擎用，如"金额>500转人工"）
    #[serde(default)]
    pub rules: Vec<String>,
    /// 话术示例（M4.1 few-shot 训练用）
    #[serde(default)]
    pub examples: Vec<Example>,
    /// 行业分类（如 "ecommerce" / "education" / "finance"）
    #[serde(default)]
    pub industry: Option<String>,
    /// 自定义元数据（扩展字段）
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 话术示例（few-shot 训练用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// 用户输入
    pub user: String,
    /// 期望的 AI 回复
    pub assistant: String,
    /// 示例标签（如 "售前" / "售后" / "投诉"）
    #[serde(default)]
    pub tag: Option<String>,
}

impl EmployeeDefinition {
    /// 从 YAML 字符串解析
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let employee: Self = serde_yaml::from_str(yaml)?;
        employee.validate()?;
        Ok(employee)
    }

    /// 从文件加载
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// 序列化为 YAML
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// 校验员工定义合法性
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(crate::error::TeamError::Config(
                "员工 ID 不能为空".to_string(),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(crate::error::TeamError::Config(
                "员工岗位名不能为空".to_string(),
            ));
        }
        if self.role.trim().is_empty() {
            return Err(crate::error::TeamError::Config(
                "员工角色不能为空".to_string(),
            ));
        }
        if self.model_id.trim().is_empty() {
            return Err(crate::error::TeamError::Config(format!(
                "员工 '{}' 的 model_id 不能为空",
                self.id
            )));
        }
        if self.system_prompt.trim().is_empty() {
            return Err(crate::error::TeamError::Config(format!(
                "员工 '{}' 的 system_prompt 不能为空",
                self.id
            )));
        }
        Ok(())
    }

    /// 获取所有模型（主 + 备用），按优先级排序
    pub fn all_models(&self) -> Vec<String> {
        let mut models = vec![self.model_id.clone()];
        models.extend(self.model_fallback.iter().cloned());
        models
    }

    /// 检查是否拥有某技能
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }

    /// 检查是否拥有某工具
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.iter().any(|t| t == tool)
    }

    /// 检查是否属于某行业
    pub fn is_industry(&self, industry: &str) -> bool {
        self.industry.as_deref() == Some(industry)
    }

    /// 转换为 TeamTemplate 中的 Role（用于加入团队）
    pub fn to_role(&self) -> crate::team_template::Role {
        crate::team_template::Role {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            model: self.model_id.clone(),
            model_fallback: self.model_fallback.clone(),
            skills: self.capabilities.clone(),
            knowledge: self
                .knowledge_base
                .as_ref()
                .map(|kb| vec![kb.clone()])
                .unwrap_or_default(),
            permissions: self.tools.clone(),
            system_prompt: Some(self.system_prompt.clone()),
        }
    }

    /// 从目录加载所有员工定义
    ///
    /// 目录结构：`{dir}/*.yml` 或 `{dir}/*.yaml`
    pub fn load_from_dir(dir: &Path) -> Result<Vec<EmployeeDefinition>> {
        let mut employees = Vec::new();
        if !dir.exists() {
            return Ok(employees);
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yml")
                || path.extension().and_then(|e| e.to_str()) == Some("yaml")
            {
                match Self::from_file(&path) {
                    Ok(emp) => employees.push(emp),
                    Err(e) => {
                        tracing::warn!(path = ?path, error = %e, "跳过无法解析的员工定义文件");
                    }
                }
            }
        }
        Ok(employees)
    }
}

// ===== 内置员工模板（3 通用 + 3 行业）=====

/// 内置客服员工
pub fn builtin_customer_service() -> EmployeeDefinition {
    let yaml = r#"
id: "customer_service"
name: "客服"
role: "answer_questions"
description: "通用客服员工，处理售前咨询、售后问题、评价管理"
model_id: "qwen3.7"
model_fallback: ["glm5.2"]
system_prompt: |
  你是一名专业的客服员工。你的职责是：
  1. 友好地回答客户问题
  2. 处理售前咨询（商品信息、价格、活动）
  3. 处理售后问题（退换货、物流查询、投诉）
  4. 主动邀评，负面评价及时预警
  
  沟通原则：
  - 语气亲切友好，使用"亲""您好"等礼貌用语
  - 不夸大商品功效，不承诺无法做到的事
  - 涉及金额超过 500 元的退款转人工
  - 涉及投诉纠纷立即升级主管
capabilities:
  - pre_sales_consultation
  - after_sales_handling
  - review_management
  - complaint_escalation
tools:
  - order_query
  - logistics_query
  - refund_process
industry: null
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置客服员工模板应合法")
}

/// 内置开发员工（兼容现有 Coder）
pub fn builtin_developer() -> EmployeeDefinition {
    let yaml = r#"
id: "developer"
name: "开发工程师"
role: "write_code"
description: "通用开发员工，编写代码、修复 bug、重构、写测试"
model_id: "glm5.2"
model_fallback: ["qwen-coder"]
system_prompt: |
  你是一名专业的开发工程师。你的职责是：
  1. 根据需求编写高质量代码
  2. 修复 bug 并编写回归测试
  3. 重构代码保持可维护性
  4. 编写单元测试和集成测试
  
  编码原则：
  - 遵循项目现有代码风格
  - 优先简单可读的方案
  - 不引入不必要的依赖
  - 每次改动都附上简短说明
capabilities:
  - code_writing
  - bug_fixing
  - refactoring
  - test_writing
tools:
  - file_read
  - file_write
  - shell_exec
  - git_operation
industry: null
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置开发员工模板应合法")
}

/// 内置通用助手
pub fn builtin_assistant() -> EmployeeDefinition {
    let yaml = r#"
id: "assistant"
name: "通用助手"
role: "general_assistant"
description: "通用 AI 助手，处理日程、邮件、文档、决策辅助等杂项工作"
model_id: "qwen3.7"
model_fallback: ["glm5.2"]
system_prompt: |
  你是一名通用 AI 助手。你的职责是：
  1. 管理日程安排
  2. 起草和回复邮件
  3. 整理文档和会议纪要
  4. 提供决策辅助信息
  
  工作原则：
  - 简洁高效，不啰嗦
  - 涉及机密信息提醒用户
  - 不确定的事明确说明
  - 主动提出下一步建议
capabilities:
  - schedule_management
  - email_drafting
  - document_summarization
  - decision_support
tools:
  - calendar
  - email
  - document
industry: null
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置通用助手模板应合法")
}

/// 内置教育助教员工（行业：教育）
pub fn builtin_education_teacher() -> EmployeeDefinition {
    let yaml = r#"
id: "education_teacher"
name: "教育助教"
role: "tutoring"
description: "教育行业助教员工，学科答疑、作业批改、学情分析"
model_id: "qwen3.7"
model_fallback: ["glm5.2"]
system_prompt: |
  你是一名教育助教。你的职责是：
  1. 解答学生学科问题（数学/语文/英语/物理/化学）
  2. 批改作业（客观题自动判，主观题给反馈）
  3. 分析学情（错题统计、进步曲线）
  4. 与家长沟通学习进展
  
  教学原则：
  - 启发式引导，不直接给答案
  - 学段适配（小学用比喻，初中用推理）
  - 鼓励式反馈，不打击信心
  - 涉及自伤/家暴/校园欺凌立即转人工
capabilities:
  - subject_qa
  - homework_grading
  - learning_analysis
  - parent_communication
tools:
  - knowledge_search
  - grade_book
industry: "education"
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置教育助教模板应合法")
}

/// 内置销售跟进员工（行业：销售）
pub fn builtin_sales_followup() -> EmployeeDefinition {
    let yaml = r#"
id: "sales_followup"
name: "销售跟进"
role: "sales"
description: "销售行业跟进员工，线索筛选、客户跟进、报价管理"
model_id: "qwen3.7"
model_fallback: ["glm5.2"]
system_prompt: |
  你是一名销售跟进员工。你的职责是：
  1. 初步筛选线索（BANT：预算/权威/需求/时间）
  2. 定期跟进客户，推进销售流程
  3. 管理报价单，超 8 折需主管审批
  4. 维护客户画像和沟通记录
  
  销售原则：
  - 专业不卑微，友好不讨好
  - 不诋毁竞争对手
  - 不承诺无法兑现的交付时间
  - 30 天未联系的客户自动降级
capabilities:
  - lead_qualification
  - customer_followup
  - quote_management
  - customer_profiling
tools:
  - crm
  - quote_generator
industry: "sales"
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置销售跟进模板应合法")
}

/// 内置财务对账员工（行业：财务）
pub fn builtin_finance_auditor() -> EmployeeDefinition {
    let yaml = r#"
id: "finance_auditor"
name: "财务对账"
role: "finance"
description: "财务行业对账员工，票据识别、账目核对、报表生成"
model_id: "glm5.2"
model_fallback: ["qwen3.7"]
system_prompt: |
  你是一名财务对账员工。你的职责是：
  1. 识别发票和银行回单（OCR + 字段提取）
  2. 核对银行流水与账目差异
  3. 生成月报/季报/年报
  4. 标记异常账目并转人工复核
  
  财务原则：
  - 严谨细致，不放过任何差异
  - 涉及金额超过 5 万的支出双重复核
  - 跨年账目转人工处理
  - 不处理涉密账目，提醒走线下流程
capabilities:
  - invoice_ocr
  - reconciliation
  - report_generation
  - anomaly_detection
tools:
  - ocr
  - accounting_system
  - bank_api
industry: "finance"
"#;
    EmployeeDefinition::from_yaml(yaml).expect("内置财务对账模板应合法")
}

/// 获取所有内置员工（3 通用 + 3 行业）
pub fn builtin_employees() -> Vec<EmployeeDefinition> {
    vec![
        builtin_customer_service(),
        builtin_developer(),
        builtin_assistant(),
        builtin_education_teacher(),
        builtin_sales_followup(),
        builtin_finance_auditor(),
    ]
}

/// 按 ID 查找内置员工
pub fn builtin_by_id(id: &str) -> Option<EmployeeDefinition> {
    builtin_employees().into_iter().find(|e| e.id == id)
}

/// 获取通用员工（industry = null）
pub fn builtin_general_employees() -> Vec<EmployeeDefinition> {
    builtin_employees()
        .into_iter()
        .filter(|e| e.industry.is_none())
        .collect()
}

/// 获取行业员工（industry = Some）
pub fn builtin_industry_employees() -> Vec<EmployeeDefinition> {
    builtin_employees()
        .into_iter()
        .filter(|e| e.industry.is_some())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 基础结构测试 =====

    #[test]
    fn test_parse_customer_service() {
        let emp = builtin_customer_service();
        assert_eq!(emp.id, "customer_service");
        assert_eq!(emp.name, "客服");
        assert_eq!(emp.role, "answer_questions");
        assert_eq!(emp.model_id, "qwen3.7");
        assert!(!emp.system_prompt.is_empty());
    }

    #[test]
    fn test_parse_developer() {
        let emp = builtin_developer();
        assert_eq!(emp.id, "developer");
        assert_eq!(emp.role, "write_code");
        assert_eq!(emp.model_id, "glm5.2");
        assert!(emp.has_capability("code_writing"));
        assert!(emp.has_tool("file_write"));
    }

    #[test]
    fn test_parse_assistant() {
        let emp = builtin_assistant();
        assert_eq!(emp.id, "assistant");
        assert_eq!(emp.industry, None);
        assert!(emp.has_capability("schedule_management"));
    }

    #[test]
    fn test_parse_education_teacher() {
        let emp = builtin_education_teacher();
        assert_eq!(emp.id, "education_teacher");
        assert_eq!(emp.industry, Some("education".to_string()));
        assert!(emp.is_industry("education"));
        assert!(!emp.is_industry("finance"));
    }

    #[test]
    fn test_parse_sales_followup() {
        let emp = builtin_sales_followup();
        assert_eq!(emp.id, "sales_followup");
        assert_eq!(emp.industry.as_deref(), Some("sales"));
        assert!(emp.has_capability("lead_qualification"));
    }

    #[test]
    fn test_parse_finance_auditor() {
        let emp = builtin_finance_auditor();
        assert_eq!(emp.id, "finance_auditor");
        assert_eq!(emp.industry.as_deref(), Some("finance"));
        assert!(emp.has_tool("ocr"));
    }

    // ===== 内置员工集合测试 =====

    #[test]
    fn test_builtin_employees_count() {
        let all = builtin_employees();
        assert_eq!(all.len(), 6, "应有 6 个内置员工（3 通用 + 3 行业）");
    }

    #[test]
    fn test_builtin_general_count() {
        let general = builtin_general_employees();
        assert_eq!(general.len(), 3, "应有 3 个通用员工");
        for emp in &general {
            assert!(emp.industry.is_none(), "通用员工 industry 应为 null");
        }
    }

    #[test]
    fn test_builtin_industry_count() {
        let industry = builtin_industry_employees();
        assert_eq!(industry.len(), 3, "应有 3 个行业员工");
        for emp in &industry {
            assert!(emp.industry.is_some(), "行业员工 industry 不应为 null");
        }
    }

    #[test]
    fn test_builtin_by_id() {
        assert!(builtin_by_id("customer_service").is_some());
        assert!(builtin_by_id("developer").is_some());
        assert!(builtin_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_ids_unique() {
        let all = builtin_employees();
        let mut ids: Vec<_> = all.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        let len_before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len_before, "内置员工 ID 不应重复");
    }

    // ===== YAML 序列化测试 =====

    #[test]
    fn test_yaml_roundtrip() {
        let emp = builtin_customer_service();
        let yaml = emp.to_yaml().unwrap();
        let parsed = EmployeeDefinition::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.id, emp.id);
        assert_eq!(parsed.name, emp.name);
        assert_eq!(parsed.model_id, emp.model_id);
        assert_eq!(parsed.capabilities.len(), emp.capabilities.len());
    }

    #[test]
    fn test_save_and_load_file() {
        let emp = builtin_developer();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path();
        emp.save_to_file(path).unwrap();
        let loaded = EmployeeDefinition::from_file(path).unwrap();
        assert_eq!(loaded.id, emp.id);
        assert_eq!(loaded.role, emp.role);
    }

    // ===== 校验测试 =====

    #[test]
    fn test_validate_empty_id() {
        let yaml = r#"
id: ""
name: "测试"
role: "test"
model_id: "m1"
system_prompt: "p"
"#;
        assert!(EmployeeDefinition::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_validate_empty_name() {
        let yaml = r#"
id: "test"
name: ""
role: "test"
model_id: "m1"
system_prompt: "p"
"#;
        assert!(EmployeeDefinition::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_validate_empty_model() {
        let yaml = r#"
id: "test"
name: "测试"
role: "test"
model_id: ""
system_prompt: "p"
"#;
        assert!(EmployeeDefinition::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_validate_empty_prompt() {
        let yaml = r#"
id: "test"
name: "测试"
role: "test"
model_id: "m1"
system_prompt: ""
"#;
        assert!(EmployeeDefinition::from_yaml(yaml).is_err());
    }

    // ===== 方法测试 =====

    #[test]
    fn test_all_models() {
        let emp = builtin_customer_service();
        let models = emp.all_models();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0], "qwen3.7");
        assert_eq!(models[1], "glm5.2");
    }

    #[test]
    fn test_has_capability_and_tool() {
        let emp = builtin_developer();
        assert!(emp.has_capability("code_writing"));
        assert!(!emp.has_capability("nonexistent"));
        assert!(emp.has_tool("git_operation"));
        assert!(!emp.has_tool("nonexistent"));
    }

    #[test]
    fn test_to_role_conversion() {
        let emp = builtin_customer_service();
        let role = emp.to_role();
        assert_eq!(role.id, emp.id);
        assert_eq!(role.name, emp.name);
        assert_eq!(role.model, emp.model_id);
        assert_eq!(role.skills, emp.capabilities);
        assert_eq!(role.system_prompt.as_deref(), Some(emp.system_prompt.as_str()));
    }

    #[test]
    fn test_load_from_dir_nonexistent() {
        let path = Path::new("/nonexistent/path/that/does/not/exist");
        let result = EmployeeDefinition::load_from_dir(path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_from_dir_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();

        // 保存 2 个员工到目录
        let emp1 = builtin_customer_service();
        let emp2 = builtin_developer();
        emp1.save_to_file(&dir_path.join("cs.yml")).unwrap();
        emp2.save_to_file(&dir_path.join("dev.yaml")).unwrap();

        // 加载
        let loaded = EmployeeDefinition::load_from_dir(dir_path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_examples_deserialization() {
        let yaml = r#"
id: "test"
name: "测试"
role: "test"
model_id: "m1"
system_prompt: "p"
examples:
  - user: "你好"
    assistant: "您好，有什么可以帮您？"
    tag: "greeting"
  - user: "退款"
    assistant: "好的，请提供订单号"
    tag: "refund"
"#;
        let emp = EmployeeDefinition::from_yaml(yaml).unwrap();
        assert_eq!(emp.examples.len(), 2);
        assert_eq!(emp.examples[0].user, "你好");
        assert_eq!(emp.examples[0].tag.as_deref(), Some("greeting"));
    }

    #[test]
    fn test_metadata_deserialization() {
        let yaml = r#"
id: "test"
name: "测试"
role: "test"
model_id: "m1"
system_prompt: "p"
metadata:
  author: "yunji"
  version: "1.0"
"#;
        let emp = EmployeeDefinition::from_yaml(yaml).unwrap();
        assert_eq!(emp.metadata.get("author").unwrap(), "yunji");
        assert_eq!(emp.metadata.get("version").unwrap(), "1.0");
    }
}
