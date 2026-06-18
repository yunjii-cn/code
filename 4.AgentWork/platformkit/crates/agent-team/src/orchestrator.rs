// Orchestrator Agent（W9 M3.3 D1）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.2, § 9
//
// Orchestrator 是团队的项目负责人，职责：
//   1. 需求分析：理解用户需求
//   2. 任务拆分：将需求拆分为可执行任务
//   3. DAG 生成：输出结构化 JSON DAG（非自然语言）
//   4. 进度监督：监控任务执行进度
//   5. 质量验收：任务完成后验收
//
// 模型：Qwen3.7（强推理）/ Claude Opus（备选）
//
// 输出格式：严格 JSON，匹配 TaskDag 结构
//   {
//     "workflow_id": "wf_xxx",
//     "requirement": "用户需求",
//     "tasks": [
//       { "id": "task-1", "title": "...", "assignee": "...", "dependencies": [...], ... }
//     ]
//   }

use crate::dag::TaskDag;
use crate::error::{Result, TeamError};
use crate::team_template::Role;
use serde::{Deserialize, Serialize};
use timeflow_ai::LlmEngine;

/// Orchestrator 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// 工作流 ID 前缀（默认 "wf_"）
    pub workflow_id_prefix: String,
    /// 最大任务数（防止 LLM 拆分过多）
    pub max_tasks: usize,
    /// 最小任务数（防止 LLM 拆分不足）
    pub min_tasks: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            workflow_id_prefix: "wf_".to_string(),
            max_tasks: 20,
            min_tasks: 1,
        }
    }
}

/// Orchestrator Agent
///
/// 项目负责人角色，负责需求分析 + 任务拆分 + DAG 生成 + 验收。
///
/// 持有 `Box<dyn LlmEngine>`，由外部注入（生产用 ModelRouter 创建，测试用 MockEngine）。
pub struct Orchestrator {
    /// 角色定义
    pub role: Role,
    /// LLM 引擎
    engine: Box<dyn LlmEngine>,
    /// 当前使用的模型 ID
    model_id: String,
    /// 配置
    config: OrchestratorConfig,
}

impl Orchestrator {
    /// 创建 Orchestrator（直接注入引擎，用于测试或自定义场景）
    pub fn new(role: Role, engine: Box<dyn LlmEngine>, model_id: impl Into<String>) -> Self {
        Self {
            role,
            engine,
            model_id: model_id.into(),
            config: OrchestratorConfig::default(),
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// 拆分需求为 DAG
    ///
    /// 输入：自然语言需求
    /// 输出：结构化 TaskDag（已校验）
    pub async fn plan(&self, requirement: &str) -> Result<TaskDag> {
        // 1. 构建 prompt
        let system = self.build_system_prompt();
        let user = self.build_plan_prompt(requirement);

        // 2. 调用 LLM
        let response = self.engine.complete(&system, &user).await?;

        // 3. 解析 JSON 响应
        let dag = self.parse_dag_response(&response, requirement)?;

        // 4. 校验 DAG
        dag.validate()?;

        // 5. 检查任务数限制
        if dag.tasks.len() < self.config.min_tasks {
            return Err(TeamError::Config(format!(
                "任务数 {} 少于最小要求 {}",
                dag.tasks.len(),
                self.config.min_tasks
            )));
        }
        if dag.tasks.len() > self.config.max_tasks {
            return Err(TeamError::Config(format!(
                "任务数 {} 超过最大限制 {}",
                dag.tasks.len(),
                self.config.max_tasks
            )));
        }

        Ok(dag)
    }

    /// 验收任务
    ///
    /// 检查任务产出是否满足验收标准。
    /// 返回 (通过, 反馈)。
    pub async fn review(
        &self,
        task_title: &str,
        acceptance_criteria: &str,
        artifact: &str,
    ) -> Result<(bool, String)> {
        let system = self.build_system_prompt();
        let user = format!(
            "## 任务验收\n\n\
            任务标题：{task_title}\n\n\
            验收标准：{acceptance_criteria}\n\n\
            任务产出：\n```\n{artifact}\n```\n\n\
            请判断产出是否满足验收标准。第一行输出 `PASS` 或 `FAIL`，第二行起输出具体反馈。"
        );

        let response = self.engine.complete(&system, &user).await?;
        let content = response.trim();

        let passed = content
            .lines()
            .next()
            .map(|line| line.trim().eq_ignore_ascii_case("PASS"))
            .unwrap_or(false);

        let feedback = content
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        Ok((passed, feedback))
    }

    /// 解析 LLM 响应为 TaskDag
    ///
    /// 支持两种格式：
    ///   1. 纯 JSON
    ///   2. ```json ... ``` 包裹的 JSON
    fn parse_dag_response(&self, content: &str, requirement: &str) -> Result<TaskDag> {
        let json_str = extract_json(content)?;

        let mut dag: TaskDag = serde_json::from_str(json_str).map_err(|e| {
            TeamError::Config(format!("DAG JSON 解析失败: {e}（原始内容: {content}）"))
        })?;

        // 如果 LLM 没填 workflow_id 或 requirement，补上
        if dag.workflow_id.is_empty() {
            dag.workflow_id = format!("{}auto", self.config.workflow_id_prefix);
        }
        if dag.requirement.is_empty() {
            dag.requirement = requirement.to_string();
        }

        Ok(dag)
    }

    /// 构建系统提示词
    fn build_system_prompt(&self) -> String {
        if let Some(custom) = &self.role.system_prompt {
            return custom.clone();
        }

        format!(
            "你是{}（{}），AI 研发团队的项目负责人。\n\n\
            ## 职责\n\
            - 需求分析：理解用户需求，识别技术挑战\n\
            - 任务拆分：将需求拆分为可执行的小任务（每个任务 1-4 小时）\n\
            - DAG 生成：输出结构化 JSON DAG，调度引擎直接解析\n\
            - 进度监督：监控任务执行进度，处理异常\n\
            - 质量验收：任务完成后验收，确保满足验收标准\n\n\
            ## 输出规范\n\
            - 严格输出 JSON 格式，不要包含任何解释性文字\n\
            - JSON 结构必须匹配 TaskDag：workflow_id / requirement / tasks\n\
            - 每个任务必须有 id / title / assignee / dependencies\n\
            - assignee 必须是有效角色 ID（orchestrator/backend/frontend/tester）\n\
            - dependencies 必须引用已存在的 task id\n\
            - 任务数控制在 {}-{} 个\n\n\
            ## 团队角色\n\
            - orchestrator: 项目负责人（你），负责设计/验收\n\
            - backend: 后端工程师，负责 API/业务逻辑/数据库\n\
            - frontend: 前端工程师，负责 UI/交互\n\
            - tester: 测试工程师，负责单元测试/E2E 测试\n\n\
            请严格遵循输出规范。",
            self.role.name,
            self.role.id,
            self.config.min_tasks,
            self.config.max_tasks
        )
    }

    /// 构建任务规划 prompt
    fn build_plan_prompt(&self, requirement: &str) -> String {
        format!(
            "## 用户需求\n\
            {requirement}\n\n\
            ## 任务\n\
            请分析需求，拆分为可执行的任务，输出 JSON DAG。\n\n\
            ## 输出格式（严格 JSON，不要 markdown 代码块）\n\
            {{\n\
              \"workflow_id\": \"wf_<简短标识>\",\n\
              \"requirement\": \"<原始需求>\",\n\
              \"tasks\": [\n\
                {{\n\
                  \"id\": \"task-1\",\n\
                  \"title\": \"<任务标题>\",\n\
                  \"assignee\": \"<角色ID>\",\n\
                  \"dependencies\": [],\n\
                  \"estimated_effort\": \"<预估工作量>\",\n\
                  \"acceptance_criteria\": \"<验收标准>\",\n\
                  \"description\": \"<详细描述>\"\n\
                }}\n\
              ]\n\
            }}"
        )
    }

    /// 获取当前使用的模型 ID
    pub fn current_model(&self) -> &str {
        &self.model_id
    }
}

/// 从 LLM 响应中提取 JSON
///
/// 支持三种格式：
///   1. 纯 JSON（直接以 `{` 开头）
///   2. ```json ... ``` 包裹
///   3. ``` ... ``` 包裹
pub fn extract_json(content: &str) -> Result<&str> {
    let trimmed = content.trim();

    // 格式 1：纯 JSON
    if trimmed.starts_with('{') {
        // 找到最后一个 `}`
        if let Some(end) = trimmed.rfind('}') {
            return Ok(&trimmed[..=end]);
        }
        return Err(TeamError::Config("JSON 响应不完整：缺少 `}`".to_string()));
    }

    // 格式 2：```json ... ```
    if let Some(start) = trimmed.find("```json") {
        let after_json = &trimmed[start + 7..];
        if let Some(end) = after_json.find("```") {
            return Ok(after_json[..end].trim());
        }
    }

    // 格式 3：``` ... ```
    if let Some(start) = trimmed.find("```") {
        let after_code = &trimmed[start + 3..];
        // 跳过可能的语言标识行
        let after_lang = if after_code.starts_with("json") {
            &after_code[4..]
        } else {
            after_code
        };
        if let Some(end) = after_lang.find("```") {
            return Ok(after_lang[..end].trim());
        }
    }

    // 兜底：尝试找第一个 `{` 到最后一个 `}`
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            return Ok(&trimmed[start..=end]);
        }
    }

    Err(TeamError::Config(format!(
        "无法从 LLM 响应中提取 JSON：{content}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use timeflow_ai::MockEngine;

    fn make_test_role() -> Role {
        Role {
            id: "orchestrator".to_string(),
            name: "项目负责人".to_string(),
            description: "需求分析、任务拆分、DAG 生成".to_string(),
            model: "mock".to_string(),
            model_fallback: vec![],
            skills: vec!["planning".to_string()],
            knowledge: vec![],
            permissions: vec!["review".to_string()],
            system_prompt: None,
        }
    }

    fn make_orchestrator(response: &str) -> Orchestrator {
        Orchestrator::new(
            make_test_role(),
            Box::new(MockEngine::new(response)),
            "mock",
        )
    }

    #[test]
    fn test_extract_json_pure() {
        let content = r#"{"key": "value"}"#;
        let json = extract_json(content).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_with_prefix() {
        let content = "好的，这是 DAG：\n{\"key\": \"value\"}\n完成";
        let json = extract_json(content).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_markdown_block() {
        let content = "```json\n{\"key\": \"value\"}\n```";
        let json = extract_json(content).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_code_block() {
        let content = "```\n{\"key\": \"value\"}\n```";
        let json = extract_json(content).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_invalid() {
        let content = "这不是 JSON";
        assert!(extract_json(content).is_err());
    }

    #[test]
    fn test_extract_json_nested() {
        let content = r#"{"outer": {"inner": "value"}, "arr": [1, 2, 3]}"#;
        let json = extract_json(content).unwrap();
        assert!(json.contains(r#""outer""#));
        assert!(json.contains(r#""arr""#));
    }

    #[tokio::test]
    async fn test_orchestrator_plan_success() {
        let dag_json = r#"{
            "workflow_id": "wf_health",
            "requirement": "添加健康检查接口",
            "tasks": [
                {
                    "id": "task-1",
                    "title": "实现 /api/health 接口",
                    "assignee": "backend",
                    "dependencies": [],
                    "estimated_effort": "30min",
                    "acceptance_criteria": "GET /api/health 返回 {\"status\":\"ok\"}",
                    "description": "实现健康检查接口"
                },
                {
                    "id": "task-2",
                    "title": "编写测试",
                    "assignee": "tester",
                    "dependencies": ["task-1"],
                    "estimated_effort": "20min",
                    "acceptance_criteria": "测试覆盖率 >= 80%",
                    "description": "编写单元测试"
                }
            ]
        }"#;

        let orchestrator = make_orchestrator(dag_json);

        let dag = orchestrator.plan("添加健康检查接口").await.unwrap();
        assert_eq!(dag.workflow_id, "wf_health");
        assert_eq!(dag.tasks.len(), 2);
        assert_eq!(dag.tasks[0].id, "task-1");
        assert_eq!(dag.tasks[0].assignee, "backend");
        assert_eq!(dag.tasks[1].id, "task-2");
        assert_eq!(dag.tasks[1].dependencies, vec!["task-1"]);
    }

    #[tokio::test]
    async fn test_orchestrator_plan_with_markdown() {
        let response = format!(
            "好的，这是 DAG：\n```json\n{}\n```",
            r#"{"workflow_id": "wf_test", "requirement": "测试", "tasks": [{"id": "t1", "title": "任务1", "assignee": "backend", "dependencies": []}]}"#
        );

        let orchestrator = make_orchestrator(&response);

        let dag = orchestrator.plan("测试").await.unwrap();
        assert_eq!(dag.workflow_id, "wf_test");
        assert_eq!(dag.tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_plan_invalid_json() {
        let orchestrator = make_orchestrator("这不是 JSON");
        let result = orchestrator.plan("测试").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_orchestrator_plan_invalid_dag() {
        // 缺少 tasks 字段
        let invalid_json = r#"{"workflow_id": "wf_test", "requirement": "测试"}"#;
        let orchestrator = make_orchestrator(invalid_json);
        let result = orchestrator.plan("测试").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_orchestrator_plan_too_many_tasks() {
        let mut tasks = Vec::new();
        for i in 1..=25 {
            tasks.push(format!(
                r#"{{"id": "task-{i}", "title": "任务{i}", "assignee": "backend", "dependencies": []}}"#
            ));
        }
        let dag_json = format!(
            r#"{{"workflow_id": "wf_test", "requirement": "测试", "tasks": [{}]}}"#,
            tasks.join(", ")
        );

        let orchestrator =
            make_orchestrator(&dag_json).with_config(OrchestratorConfig {
                max_tasks: 20,
                ..Default::default()
            });

        let result = orchestrator.plan("测试").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_orchestrator_plan_empty_tasks() {
        let dag_json =
            r#"{"workflow_id": "wf_test", "requirement": "测试", "tasks": []}"#;
        let orchestrator = make_orchestrator(dag_json);
        let result = orchestrator.plan("测试").await;
        assert!(result.is_err()); // validate() 会拒绝空 DAG
    }

    #[tokio::test]
    async fn test_orchestrator_review_pass() {
        let response = "PASS\n代码质量良好，符合验收标准。";
        let orchestrator = make_orchestrator(response);

        let (passed, feedback) = orchestrator
            .review("实现 /api/health", "返回 {\"status\":\"ok\"}", "fn health() { ... }")
            .await
            .unwrap();

        assert!(passed);
        assert!(feedback.contains("代码质量良好"));
    }

    #[tokio::test]
    async fn test_orchestrator_review_fail() {
        let response = "FAIL\n缺少错误处理，需要添加 try-catch。";
        let orchestrator = make_orchestrator(response);

        let (passed, feedback) = orchestrator
            .review("实现 /api/health", "返回 {\"status\":\"ok\"}", "fn health() { ... }")
            .await
            .unwrap();

        assert!(!passed);
        assert!(feedback.contains("缺少错误处理"));
    }

    #[tokio::test]
    async fn test_orchestrator_review_case_insensitive() {
        let response = "pass\n通过";
        let orchestrator = make_orchestrator(response);

        let (passed, _) = orchestrator
            .review("任务", "标准", "产出")
            .await
            .unwrap();
        assert!(passed);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.workflow_id_prefix, "wf_");
        assert_eq!(config.max_tasks, 20);
        assert_eq!(config.min_tasks, 1);
    }

    #[tokio::test]
    async fn test_orchestrator_fills_missing_fields() {
        let dag_json = r#"{
            "workflow_id": "",
            "requirement": "",
            "tasks": [{"id": "t1", "title": "任务", "assignee": "backend", "dependencies": []}]
        }"#;
        let orchestrator = make_orchestrator(dag_json);

        let dag = orchestrator.plan("用户需求").await.unwrap();
        assert!(!dag.workflow_id.is_empty());
        assert_eq!(dag.requirement, "用户需求");
    }

    #[test]
    fn test_current_model() {
        let orchestrator = make_orchestrator("test");
        assert_eq!(orchestrator.current_model(), "mock");
    }
}
