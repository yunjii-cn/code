// 团队模板定义（W7 M3.1 D2）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.1
//
// 团队模板 = 一组角色的定义，每个角色绑定：
//   - 主模型 + 备用模型（故障转移）
//   - 技能列表（skills）
//   - 知识库引用（knowledge）
//   - 权限（permissions）
//
// 模板格式：YAML（.yunji/team.yml）
// 内置模板：全栈团队 / 后端团队 / 前端团队 / 测试团队

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 角色 ID 类型
pub type RoleId = String;

/// 团队模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTemplate {
    /// 团队信息
    pub team: TeamInfo,
}

/// 团队信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    /// 团队名称
    pub name: String,
    /// 团队描述
    #[serde(default)]
    pub description: String,
    /// 角色列表
    pub roles: Vec<Role>,
}

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// 角色 ID（唯一标识，如 "orchestrator" / "backend"）
    pub id: RoleId,
    /// 角色名称（展示用，如 "项目负责人"）
    pub name: String,
    /// 角色描述
    #[serde(default)]
    pub description: String,
    /// 主模型（模型 ID）
    pub model: String,
    /// 备用模型列表（故障转移顺序）
    #[serde(default)]
    pub model_fallback: Vec<String>,
    /// 技能列表
    #[serde(default)]
    pub skills: Vec<String>,
    /// 知识库引用
    #[serde(default)]
    pub knowledge: Vec<String>,
    /// 权限列表
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 系统提示词（可选，覆盖默认）
    #[serde(default)]
    pub system_prompt: Option<String>,
}

impl Role {
    /// 获取所有模型（主 + 备用），按优先级排序
    pub fn all_models(&self) -> Vec<String> {
        let mut models = vec![self.model.clone()];
        models.extend(self.model_fallback.iter().cloned());
        models
    }

    /// 检查是否拥有某权限
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }

    /// 检查是否拥有某技能
    pub fn has_skill(&self, skill: &str) -> bool {
        self.skills.iter().any(|s| s == skill)
    }
}

impl TeamInfo {
    /// 根据 ID 查找角色
    pub fn get_role(&self, id: &str) -> Option<&Role> {
        self.roles.iter().find(|r| r.id == id)
    }

    /// 根据 ID 查找角色（mutable）
    pub fn get_role_mut(&mut self, id: &str) -> Option<&mut Role> {
        self.roles.iter_mut().find(|r| r.id == id)
    }

    /// 获取所有角色 ID
    pub fn role_ids(&self) -> Vec<String> {
        self.roles.iter().map(|r| r.id.clone()).collect()
    }

    /// 角色数量
    pub fn role_count(&self) -> usize {
        self.roles.len()
    }
}

impl TeamTemplate {
    /// 从 YAML 字符串解析
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let template: Self = serde_yaml::from_str(yaml)?;
        template.validate()?;
        Ok(template)
    }

    /// 从文件加载
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// 序列化为 YAML
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// 校验模板合法性
    pub fn validate(&self) -> Result<()> {
        if self.team.name.trim().is_empty() {
            return Err(crate::error::TeamError::Config("团队名称不能为空".to_string()));
        }

        if self.team.roles.is_empty() {
            return Err(crate::error::TeamError::Config("团队至少需要一个角色".to_string()));
        }

        // 检查角色 ID 唯一
        let mut seen = std::collections::HashSet::new();
        for role in &self.team.roles {
            if !seen.insert(&role.id) {
                return Err(crate::error::TeamError::Config(format!(
                    "角色 ID '{}' 重复",
                    role.id
                )));
            }
            if role.model.trim().is_empty() {
                return Err(crate::error::TeamError::Config(format!(
                    "角色 '{}' 的主模型不能为空",
                    role.id
                )));
            }
        }

        Ok(())
    }

    /// 获取团队信息
    pub fn info(&self) -> &TeamInfo {
        &self.team
    }
}

// ===== 内置团队模板 =====

/// 全栈开发团队（PM + 后端 + 前端 + 测试）
pub fn builtin_fullstack() -> TeamTemplate {
    let yaml = r#"
team:
  name: "全栈开发团队"
  description: "包含 PM、后端、前端、测试的完整开发团队"
  roles:
    - id: orchestrator
      name: "项目负责人"
      description: "需求分析、任务拆分、DAG 生成、进度监督、质量验收"
      model: "qwen3.7"
      model_fallback: ["claude-opus", "glm5.2"]
      skills:
        - requirement-analysis
        - task-decomposition
        - dag-generation
        - progress-monitoring
        - quality-review
      knowledge:
        - project-standards
        - architecture-guidelines
      permissions:
        - can_create_subtasks
        - can_merge_pr
        - can_reject_work

    - id: backend
      name: "后端工程师"
      description: "API 设计、业务逻辑、数据库交互"
      model: "glm5.2"
      model_fallback: ["qwen-coder"]
      skills:
        - api-design
        - business-logic
        - database-query
      knowledge:
        - backend-best-practices
        - security-guidelines
      permissions:
        - can_edit_backend_files
        - can_run_tests

    - id: frontend
      name: "前端工程师"
      description: "UI 设计、交互实现、样式开发"
      model: "minimax3"
      model_fallback: ["gpt-4o"]
      skills:
        - ui-design
        - component-development
        - css-styling
      knowledge:
        - design-system
        - accessibility-guidelines
      permissions:
        - can_edit_frontend_files
        - can_run_tests

    - id: tester
      name: "测试工程师"
      description: "测试用例编写、自动化测试、BUG 修复"
      model: "glm5.2"
      model_fallback: ["local-llama-3"]
      skills:
        - test-case-design
        - automated-testing
        - bug-fixing
      knowledge:
        - testing-standards
      permissions:
        - can_edit_test_files
        - can_run_tests
        - can_reject_build
"#;
    TeamTemplate::from_yaml(yaml).expect("内置全栈团队模板应合法")
}

/// 后端团队（PM + 后端 + 测试）
pub fn builtin_backend() -> TeamTemplate {
    let yaml = r#"
team:
  name: "后端开发团队"
  description: "专注后端 API 开发的精简团队"
  roles:
    - id: orchestrator
      name: "项目负责人"
      description: "需求分析、任务拆分、质量验收"
      model: "qwen3.7"
      model_fallback: ["glm5.2"]
      skills:
        - requirement-analysis
        - task-decomposition
        - quality-review
      permissions:
        - can_create_subtasks
        - can_merge_pr

    - id: backend
      name: "后端工程师"
      description: "API 设计、业务逻辑、数据库交互"
      model: "glm5.2"
      model_fallback: ["qwen-coder"]
      skills:
        - api-design
        - business-logic
        - database-query
      permissions:
        - can_edit_backend_files
        - can_run_tests

    - id: tester
      name: "测试工程师"
      description: "测试用例编写、自动化测试"
      model: "glm5.2"
      model_fallback: ["local-llama-3"]
      skills:
        - test-case-design
        - automated-testing
      permissions:
        - can_edit_test_files
        - can_run_tests
"#;
    TeamTemplate::from_yaml(yaml).expect("内置后端团队模板应合法")
}

/// 单人团队（仅一个全栈 Agent）
pub fn builtin_solo() -> TeamTemplate {
    let yaml = r#"
team:
  name: "单人模式"
  description: "单个全栈 Agent 处理所有任务"
  roles:
    - id: fullstack
      name: "全栈工程师"
      description: "独立完成需求分析、开发、测试"
      model: "glm5.2"
      model_fallback: ["qwen3.7"]
      skills:
        - requirement-analysis
        - api-design
        - ui-design
        - test-case-design
        - bug-fixing
      permissions:
        - can_edit_all_files
        - can_run_tests
        - can_merge_pr
"#;
    TeamTemplate::from_yaml(yaml).expect("内置单人团队模板应合法")
}

/// 获取所有内置模板
pub fn builtin_templates() -> HashMap<String, TeamTemplate> {
    let mut map = HashMap::new();
    map.insert("fullstack".to_string(), builtin_fullstack());
    map.insert("backend".to_string(), builtin_backend());
    map.insert("solo".to_string(), builtin_solo());
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fullstack_template() {
        let template = builtin_fullstack();
        assert_eq!(template.team.name, "全栈开发团队");
        assert_eq!(template.team.roles.len(), 4);
        assert_eq!(template.team.roles[0].id, "orchestrator");
        assert_eq!(template.team.roles[1].id, "backend");
        assert_eq!(template.team.roles[2].id, "frontend");
        assert_eq!(template.team.roles[3].id, "tester");
    }

    #[test]
    fn test_role_all_models() {
        let template = builtin_fullstack();
        let orchestrator = template.team.get_role("orchestrator").unwrap();
        let models = orchestrator.all_models();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0], "qwen3.7");
        assert!(models.contains(&"claude-opus".to_string()));
        assert!(models.contains(&"glm5.2".to_string()));
    }

    #[test]
    fn test_role_permissions() {
        let template = builtin_fullstack();
        let backend = template.team.get_role("backend").unwrap();
        assert!(backend.has_permission("can_edit_backend_files"));
        assert!(backend.has_permission("can_run_tests"));
        assert!(!backend.has_permission("can_merge_pr"));
    }

    #[test]
    fn test_role_skills() {
        let template = builtin_fullstack();
        let tester = template.team.get_role("tester").unwrap();
        assert!(tester.has_skill("test-case-design"));
        assert!(tester.has_skill("automated-testing"));
        assert!(!tester.has_skill("api-design"));
    }

    #[test]
    fn test_get_role_not_found() {
        let template = builtin_fullstack();
        assert!(template.team.get_role("nonexistent").is_none());
    }

    #[test]
    fn test_role_ids() {
        let template = builtin_fullstack();
        let ids = template.team.role_ids();
        assert_eq!(ids.len(), 4);
        assert!(ids.contains(&"orchestrator".to_string()));
        assert!(ids.contains(&"backend".to_string()));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let template = builtin_fullstack();
        let yaml = template.to_yaml().unwrap();
        let parsed = TeamTemplate::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.team.name, template.team.name);
        assert_eq!(parsed.team.roles.len(), template.team.roles.len());
    }

    #[test]
    fn test_save_and_load_file() {
        let template = builtin_solo();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path();
        template.save_to_file(path).unwrap();
        let loaded = TeamTemplate::from_file(path).unwrap();
        assert_eq!(loaded.team.name, "单人模式");
        assert_eq!(loaded.team.roles.len(), 1);
    }

    #[test]
    fn test_validate_empty_name() {
        let yaml = r#"
team:
  name: ""
  roles:
    - id: a
      name: "A"
      model: "m1"
"#;
        let result = TeamTemplate::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_roles() {
        let yaml = r#"
team:
  name: "空团队"
  roles: []
"#;
        let result = TeamTemplate::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_duplicate_role_id() {
        let yaml = r#"
team:
  name: "重复"
  roles:
    - id: a
      name: "A"
      model: "m1"
    - id: a
      name: "B"
      model: "m2"
"#;
        let result = TeamTemplate::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_model() {
        let yaml = r#"
team:
  name: "无模型"
  roles:
    - id: a
      name: "A"
      model: ""
"#;
        let result = TeamTemplate::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_templates_map() {
        let map = builtin_templates();
        assert_eq!(map.len(), 3);
        assert!(map.contains_key("fullstack"));
        assert!(map.contains_key("backend"));
        assert!(map.contains_key("solo"));
    }

    #[test]
    fn test_builtin_backend_template() {
        let template = builtin_backend();
        assert_eq!(template.team.roles.len(), 3);
        assert_eq!(template.team.roles[0].id, "orchestrator");
        assert_eq!(template.team.roles[1].id, "backend");
        assert_eq!(template.team.roles[2].id, "tester");
    }

    #[test]
    fn test_builtin_solo_template() {
        let template = builtin_solo();
        assert_eq!(template.team.roles.len(), 1);
        assert_eq!(template.team.roles[0].id, "fullstack");
    }

    #[test]
    fn test_get_role_mut() {
        let mut template = builtin_solo();
        let role = template.team.get_role_mut("fullstack").unwrap();
        role.name = "修改后的全栈".to_string();
        assert_eq!(template.team.get_role("fullstack").unwrap().name, "修改后的全栈");
    }
}
