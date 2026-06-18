// Coder Agent（W9 M3.3 D2）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 9
//
// Coder 是团队的代码工程师，职责：
//   1. 接收任务（Task）+ 上下文（AgentContext）
//   2. 生成代码（文件路径 + 内容）
//   3. 返回结构化产出（CodeArtifact）
//
// 模型：GLM5.2（代码能力强）/ Qwen-Coder（备选）
//
// 输出格式：结构化 JSON，包含文件改动列表
//   {
//     "files": [
//       { "path": "src/api/health.rs", "content": "...", "action": "create" | "modify" | "delete" }
//     ],
//     "summary": "实现了健康检查接口",
//     "commit_message": "feat(api): 添加健康检查接口"
//   }

use crate::context::AgentContext;
use crate::dag::Task;
use crate::error::{Result, TeamError};
use crate::team_template::Role;
use serde::{Deserialize, Serialize};
use timeflow_ai::LlmEngine;

/// Coder 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoderConfig {
    /// 最大输出 token 数
    pub max_tokens: u32,
    /// 温度（代码生成建议低温度）
    pub temperature: f32,
    /// 是否包含上下文（关闭则只传任务描述）
    pub include_context: bool,
}

impl Default for CoderConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            temperature: 0.2,
            include_context: true,
        }
    }
}

/// 文件改动动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    /// 创建新文件
    Create,
    /// 修改已有文件
    Modify,
    /// 删除文件
    Delete,
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Modify => write!(f, "modify"),
            Self::Delete => write!(f, "delete"),
        }
    }
}

/// 文件改动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// 文件路径
    pub path: String,
    /// 文件内容（Delete 时为空）
    #[serde(default)]
    pub content: String,
    /// 改动动作
    pub action: FileAction,
}

/// 代码产出（Coder 的输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeArtifact {
    /// 文件改动列表
    pub files: Vec<FileChange>,
    /// 任务总结
    #[serde(default)]
    pub summary: String,
    /// 建议 commit message
    #[serde(default)]
    pub commit_message: String,
}

impl CodeArtifact {
    /// 创建空产出
    pub fn empty() -> Self {
        Self {
            files: vec![],
            summary: String::new(),
            commit_message: String::new(),
        }
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// 文件数
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// 添加文件改动
    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>, action: FileAction) {
        self.files.push(FileChange {
            path: path.into(),
            content: content.into(),
            action,
        });
    }
}

/// Coder Agent
///
/// 代码工程师角色，接收任务 + 上下文，生成代码。
///
/// 持有 `Box<dyn LlmEngine>`，由外部注入。
pub struct Coder {
    /// 角色定义
    pub role: Role,
    /// LLM 引擎
    engine: Box<dyn LlmEngine>,
    /// 当前使用的模型 ID
    model_id: String,
    /// 配置
    config: CoderConfig,
}

impl Coder {
    /// 创建 Coder（直接注入引擎）
    pub fn new(role: Role, engine: Box<dyn LlmEngine>, model_id: impl Into<String>) -> Self {
        Self {
            role,
            engine,
            model_id: model_id.into(),
            config: CoderConfig::default(),
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: CoderConfig) -> Self {
        self.config = config;
        self
    }

    /// 执行任务，生成代码
    ///
    /// 参数：
    /// - `task`: 任务定义
    /// - `context`: Agent 上下文（项目规范、相关代码、上游产出等）
    ///
    /// 返回结构化代码产出
    pub async fn execute(&self, task: &Task, context: &AgentContext) -> Result<CodeArtifact> {
        // 1. 构建 prompt
        let system = self.build_system_prompt();
        let user = self.build_task_prompt(task, context);

        // 2. 调用 LLM
        let response = self.engine.complete(&system, &user).await?;

        // 3. 解析 JSON 响应
        let artifact = self.parse_code_response(&response)?;

        Ok(artifact)
    }

    /// 带错误反馈重试
    ///
    /// 当验证护栏失败时，将错误信息喂回 Coder 重新生成。
    pub async fn retry_with_errors(
        &self,
        task: &Task,
        context: &AgentContext,
        errors: &[String],
    ) -> Result<CodeArtifact> {
        let system = self.build_system_prompt();
        let user = self.build_retry_prompt(task, context, errors);

        let response = self.engine.complete(&system, &user).await?;
        self.parse_code_response(&response)
    }

    /// 解析 LLM 响应为 CodeArtifact
    fn parse_code_response(&self, content: &str) -> Result<CodeArtifact> {
        let json_str = crate::orchestrator::extract_json(content)?;

        let artifact: CodeArtifact = serde_json::from_str(json_str).map_err(|e| {
            TeamError::Config(format!("CodeArtifact JSON 解析失败: {e}（原始内容: {content}）"))
        })?;

        Ok(artifact)
    }

    /// 构建系统提示词
    fn build_system_prompt(&self) -> String {
        if let Some(custom) = &self.role.system_prompt {
            return custom.clone();
        }

        format!(
            "你是{}（{}），AI 研发团队的代码工程师。\n\n\
            ## 职责\n\
            - 接收任务定义和上下文\n\
            - 生成高质量代码（符合项目规范）\n\
            - 输出结构化 JSON，包含文件改动列表\n\n\
            ## 输出规范\n\
            严格输出 JSON 格式，不要包含任何解释性文字。结构如下：\n\
            {{\n\
              \"files\": [\n\
                {{\n\
                  \"path\": \"<文件路径>\",\n\
                  \"content\": \"<文件内容>\",\n\
                  \"action\": \"create\" | \"modify\" | \"delete\"\n\
                }}\n\
              ],\n\
              \"summary\": \"<任务总结>\",\n\
              \"commit_message\": \"<Conventional Commits 格式>\"\n\
            }}\n\n\
            ## 代码规范\n\
            - 遵循项目现有代码风格\n\
            - 添加必要的注释（中文）\n\
            - 确保代码可编译/可运行\n\
            - 处理边界情况和错误\n\
            - commit_message 使用 Conventional Commits 格式（feat/fix/docs/refactor 等）\n\n\
            请严格遵循输出规范。",
            self.role.name, self.role.id
        )
    }

    /// 构建任务执行 prompt
    fn build_task_prompt(&self, task: &Task, context: &AgentContext) -> String {
        let mut prompt = format!(
            "## 任务\n\
            标题：{}\n\
            描述：{}\n\
            验收标准：{}\n\
            预估工作量：{}\n\n",
            task.title,
            if task.description.is_empty() { "（无）" } else { &task.description },
            if task.acceptance_criteria.is_empty() { "（无）" } else { &task.acceptance_criteria },
            if task.estimated_effort.is_empty() { "（未指定）" } else { &task.estimated_effort },
        );

        if self.config.include_context && !context.is_empty() {
            prompt.push_str("## 项目上下文\n\n");
            prompt.push_str(&context.to_prompt_context());
            prompt.push_str("\n\n");
        }

        prompt.push_str(
            "## 任务\n\
            请根据任务和上下文，生成代码改动。输出 JSON 格式。",
        );

        prompt
    }

    /// 构建重试 prompt（带错误反馈）
    fn build_retry_prompt(
        &self,
        task: &Task,
        context: &AgentContext,
        errors: &[String],
    ) -> String {
        let mut prompt = self.build_task_prompt(task, context);

        prompt.push_str("\n\n## 上次提交的验证错误\n");
        for (i, err) in errors.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, err));
        }

        prompt.push_str(
            "\n请修复以上错误，重新生成代码改动。输出 JSON 格式。",
        );

        prompt
    }

    /// 获取当前使用的模型 ID
    pub fn current_model(&self) -> &str {
        &self.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use timeflow_ai::MockEngine;

    fn make_test_role(id: &str, name: &str) -> Role {
        Role {
            id: id.to_string(),
            name: name.to_string(),
            description: "代码工程师".to_string(),
            model: "mock".to_string(),
            model_fallback: vec![],
            skills: vec!["coding".to_string()],
            knowledge: vec![],
            permissions: vec!["write_code".to_string()],
            system_prompt: None,
        }
    }

    fn make_coder(response: &str) -> Coder {
        Coder::new(
            make_test_role("backend", "后端工程师"),
            Box::new(MockEngine::new(response)),
            "mock",
        )
    }

    fn make_task() -> Task {
        Task::new("task-1", "实现 /api/health 接口", "backend")
            .with_description("实现健康检查接口，返回 {\"status\":\"ok\"}")
            .with_acceptance("GET /api/health 返回 {\"status\":\"ok\"}")
    }

    #[test]
    fn test_file_action_display() {
        assert_eq!(FileAction::Create.to_string(), "create");
        assert_eq!(FileAction::Modify.to_string(), "modify");
        assert_eq!(FileAction::Delete.to_string(), "delete");
    }

    #[test]
    fn test_code_artifact_empty() {
        let artifact = CodeArtifact::empty();
        assert!(artifact.is_empty());
        assert_eq!(artifact.file_count(), 0);
    }

    #[test]
    fn test_code_artifact_add_file() {
        let mut artifact = CodeArtifact::empty();
        artifact.add_file("src/main.rs", "fn main() {}", FileAction::Create);
        assert_eq!(artifact.file_count(), 1);
        assert_eq!(artifact.files[0].path, "src/main.rs");
        assert_eq!(artifact.files[0].action, FileAction::Create);
    }

    #[test]
    fn test_code_artifact_serde() {
        let artifact = CodeArtifact {
            files: vec![FileChange {
                path: "src/api.rs".to_string(),
                content: "fn api() {}".to_string(),
                action: FileAction::Create,
            }],
            summary: "实现了 API".to_string(),
            commit_message: "feat(api): 添加 API".to_string(),
        };

        let json = serde_json::to_string(&artifact).unwrap();
        let parsed: CodeArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.file_count(), 1);
        assert_eq!(parsed.files[0].path, "src/api.rs");
        assert_eq!(parsed.summary, "实现了 API");
        assert_eq!(parsed.commit_message, "feat(api): 添加 API");
    }

    #[tokio::test]
    async fn test_coder_execute_success() {
        let response = r#"{
            "files": [
                {
                    "path": "src/api/health.rs",
                    "content": "pub fn health() -> String { \"{\\\"status\\\":\\\"ok\\\"}\".to_string() }",
                    "action": "create"
                }
            ],
            "summary": "实现了健康检查接口",
            "commit_message": "feat(api): 添加健康检查接口"
        }"#;

        let coder = make_coder(response);
        let task = make_task();
        let context = AgentContext::default();

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 1);
        assert_eq!(artifact.files[0].path, "src/api/health.rs");
        assert_eq!(artifact.files[0].action, FileAction::Create);
        assert_eq!(artifact.summary, "实现了健康检查接口");
        assert_eq!(artifact.commit_message, "feat(api): 添加健康检查接口");
    }

    #[tokio::test]
    async fn test_coder_execute_with_markdown() {
        let response = format!(
            "```json\n{}\n```",
            r#"{"files": [{"path": "main.rs", "content": "fn main() {}", "action": "create"}], "summary": "创建主文件", "commit_message": "feat: 初始化"}"#
        );

        let coder = make_coder(&response);
        let task = make_task();
        let context = AgentContext::default();

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 1);
        assert_eq!(artifact.files[0].path, "main.rs");
    }

    #[tokio::test]
    async fn test_coder_execute_invalid_json() {
        let coder = make_coder("这不是 JSON");
        let task = make_task();
        let context = AgentContext::default();

        let result = coder.execute(&task, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_coder_execute_multiple_files() {
        let response = r#"{
            "files": [
                {"path": "src/api/health.rs", "content": "pub fn health() -> String { ... }", "action": "create"},
                {"path": "src/main.rs", "content": "mod api; fn main() { println!(\"{}\", api::health::health()); }", "action": "modify"},
                {"path": "src/old.rs", "content": "", "action": "delete"}
            ],
            "summary": "添加健康检查并清理旧代码",
            "commit_message": "feat(api): 添加健康检查接口 + 删除旧代码"
        }"#;

        let coder = make_coder(response);
        let task = make_task();
        let context = AgentContext::default();

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 3);
        assert_eq!(artifact.files[0].action, FileAction::Create);
        assert_eq!(artifact.files[1].action, FileAction::Modify);
        assert_eq!(artifact.files[2].action, FileAction::Delete);
    }

    #[tokio::test]
    async fn test_coder_retry_with_errors() {
        let response = r#"{
            "files": [
                {"path": "src/api/health.rs", "content": "pub fn health() -> Result<String, Error> { Ok(...) }", "action": "create"}
            ],
            "summary": "修复错误处理",
            "commit_message": "fix(api): 添加错误处理"
        }"#;

        let coder = make_coder(response);
        let task = make_task();
        let context = AgentContext::default();
        let errors = vec!["缺少错误处理".to_string()];

        let artifact = coder
            .retry_with_errors(&task, &context, &errors)
            .await
            .unwrap();
        assert_eq!(artifact.file_count(), 1);
        assert_eq!(artifact.commit_message, "fix(api): 添加错误处理");
    }

    #[tokio::test]
    async fn test_coder_execute_with_context() {
        let response = r#"{
            "files": [{"path": "src/api.rs", "content": "// 遵循 4 空格缩进", "action": "create"}],
            "summary": "遵循规范",
            "commit_message": "feat: 添加 API"
        }"#;

        let coder = make_coder(response);
        let task = make_task();

        let mut context = AgentContext::default();
        context.standards.push(crate::context::ContextEntry::new(
            "standard/coding",
            "使用 4 空格缩进",
            crate::context::ContextCategory::Standard,
        ));

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 1);
    }

    #[tokio::test]
    async fn test_coder_execute_without_context() {
        let response = r#"{
            "files": [{"path": "src/api.rs", "content": "fn api() {}", "action": "create"}],
            "summary": "无上下文",
            "commit_message": "feat: 添加 API"
        }"#;

        let coder = make_coder(response).with_config(CoderConfig {
            include_context: false,
            ..Default::default()
        });
        let task = make_task();

        let mut context = AgentContext::default();
        context.standards.push(crate::context::ContextEntry::new(
            "standard/coding",
            "使用 4 空格缩进",
            crate::context::ContextCategory::Standard,
        ));

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 1);
    }

    #[test]
    fn test_coder_config_default() {
        let config = CoderConfig::default();
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.2);
        assert!(config.include_context);
    }

    #[test]
    fn test_current_model() {
        let coder = make_coder("test");
        assert_eq!(coder.current_model(), "mock");
    }

    #[tokio::test]
    async fn test_coder_empty_files_response() {
        // LLM 返回空文件列表（合法但无产出）
        let response = r#"{
            "files": [],
            "summary": "无需改动",
            "commit_message": ""
        }"#;

        let coder = make_coder(response);
        let task = make_task();
        let context = AgentContext::default();

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert!(artifact.is_empty());
    }

    #[tokio::test]
    async fn test_coder_missing_optional_fields() {
        // 缺少 summary 和 commit_message（serde default 应填充空字符串）
        let response = r#"{
            "files": [{"path": "src/api.rs", "content": "fn api() {}", "action": "create"}]
        }"#;

        let coder = make_coder(response);
        let task = make_task();
        let context = AgentContext::default();

        let artifact = coder.execute(&task, &context).await.unwrap();
        assert_eq!(artifact.file_count(), 1);
        assert_eq!(artifact.summary, "");
        assert_eq!(artifact.commit_message, "");
    }
}
