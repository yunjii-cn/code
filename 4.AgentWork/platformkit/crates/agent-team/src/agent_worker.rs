// Agent 工作池（W7 M3.1 D4）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.1
//
// 目标：为每个角色创建一个 Worker，封装：
//   - 角色定义（Role）
//   - 模型调用器（ModelCaller，含故障转移）
//   - 系统提示词（根据角色生成）
//   - 工作状态（Idle / Running / Error）
//
// Worker 池管理所有角色 Worker，支持：
//   - 按 ID 获取 Worker
//   - 并行调度多个 Worker
//   - 状态查询

use crate::error::Result;
use crate::model_router::{ModelCaller, ModelRouter};
use crate::team_template::{Role, TeamTemplate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use timeflow_ai::{LlmMessage, LlmResponse};

/// Worker 状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerState {
    /// 空闲
    Idle,
    /// 运行中
    Running,
    /// 错误
    Error,
    /// 已停止
    Stopped,
}

/// Agent Worker
///
/// 每个角色对应一个 Worker，封装模型调用 + 状态管理
pub struct AgentWorker<'a> {
    /// 角色定义
    pub role: Role,
    /// 模型调用器
    caller: ModelCaller<'a>,
    /// 当前状态
    state: WorkerState,
    /// 最后一次错误信息
    last_error: Option<String>,
    /// 处理的任务数
    tasks_completed: u32,
    /// 处理失败的任务数
    tasks_failed: u32,
}

impl<'a> AgentWorker<'a> {
    /// 创建 Worker
    pub fn new(role: Role, router: &'a ModelRouter) -> Result<Self> {
        let caller = router.create_caller(&role)?;
        Ok(Self {
            role,
            caller,
            state: WorkerState::Idle,
            last_error: None,
            tasks_completed: 0,
            tasks_failed: 0,
        })
    }

    /// 获取当前状态
    pub fn state(&self) -> &WorkerState {
        &self.state
    }

    /// 获取当前使用的模型 ID
    pub fn current_model(&self) -> &str {
        self.caller.current_model()
    }

    /// 获取统计信息
    pub fn stats(&self) -> WorkerStats {
        WorkerStats {
            role_id: self.role.id.clone(),
            role_name: self.role.name.clone(),
            state: self.state.clone(),
            current_model: self.caller.current_model().to_string(),
            tasks_completed: self.tasks_completed,
            tasks_failed: self.tasks_failed,
            last_error: self.last_error.clone(),
        }
    }

    /// 执行任务
    ///
    /// 参数：
    /// - `task_prompt`: 任务描述（user 消息）
    ///
    /// 返回 LLM 响应
    pub async fn execute(&mut self, task_prompt: &str) -> Result<String> {
        self.state = WorkerState::Running;
        self.last_error = None;

        let system = self.build_system_prompt();
        tracing::info!("Worker [{}] 开始执行任务（模型: {}）", self.role.id, self.current_model());

        match self.caller.complete(&system, task_prompt).await {
            Ok(resp) => {
                self.state = WorkerState::Idle;
                self.tasks_completed += 1;
                tracing::info!(
                    "Worker [{}] 任务完成（模型: {}，已完成: {}）",
                    self.role.id, self.current_model(), self.tasks_completed
                );
                Ok(resp)
            }
            Err(e) => {
                self.state = WorkerState::Error;
                self.last_error = Some(e.to_string());
                self.tasks_failed += 1;
                tracing::warn!(
                    "Worker [{}] 任务失败: {}（已失败: {}）",
                    self.role.id, e, self.tasks_failed
                );
                Err(e)
            }
        }
    }

    /// 执行带消息历史的对话
    pub async fn chat(&mut self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        self.state = WorkerState::Running;
        self.last_error = None;

        match self.caller.chat(messages).await {
            Ok(resp) => {
                self.state = WorkerState::Idle;
                self.tasks_completed += 1;
                Ok(resp)
            }
            Err(e) => {
                self.state = WorkerState::Error;
                self.last_error = Some(e.to_string());
                self.tasks_failed += 1;
                Err(e)
            }
        }
    }

    /// 重置状态（从 Error 恢复到 Idle）
    pub fn reset(&mut self) {
        self.state = WorkerState::Idle;
        self.last_error = None;
    }

    /// 停止 Worker
    pub fn stop(&mut self) {
        self.state = WorkerState::Stopped;
    }

    /// 构建系统提示词
    fn build_system_prompt(&self) -> String {
        if let Some(custom) = &self.role.system_prompt {
            return custom.clone();
        }

        let mut prompt = format!(
            "你是{}（{}）。\n\n职责：{}\n\n",
            self.role.name, self.role.id, self.role.description
        );

        if !self.role.skills.is_empty() {
            prompt.push_str("技能：\n");
            for skill in &self.role.skills {
                prompt.push_str(&format!("- {skill}\n"));
            }
            prompt.push('\n');
        }

        if !self.role.permissions.is_empty() {
            prompt.push_str("权限：\n");
            for perm in &self.role.permissions {
                prompt.push_str(&format!("- {perm}\n"));
            }
            prompt.push('\n');
        }

        prompt.push_str("请严格遵循角色职责，输出结构化、可执行的结果。");
        prompt
    }
}

/// Worker 统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    /// 角色 ID
    pub role_id: String,
    /// 角色名称
    pub role_name: String,
    /// 当前状态
    pub state: WorkerState,
    /// 当前使用的模型
    pub current_model: String,
    /// 已完成任务数
    pub tasks_completed: u32,
    /// 失败任务数
    pub tasks_failed: u32,
    /// 最后一次错误
    pub last_error: Option<String>,
}

/// Worker 池
///
/// 管理团队中所有角色的 Worker
pub struct WorkerPool<'a> {
    /// 路由器引用
    router: &'a ModelRouter,
    /// Worker 列表
    workers: HashMap<String, AgentWorker<'a>>,
}

impl<'a> WorkerPool<'a> {
    /// 创建空 Worker 池
    pub fn new(router: &'a ModelRouter) -> Self {
        Self {
            router,
            workers: HashMap::new(),
        }
    }

    /// 从团队模板创建 Worker 池
    pub fn from_template(router: &'a ModelRouter, template: &TeamTemplate) -> Result<Self> {
        let mut pool = Self::new(router);
        for role in &template.team.roles {
            pool.add_worker(role.clone())?;
        }
        Ok(pool)
    }

    /// 添加 Worker
    pub fn add_worker(&mut self, role: Role) -> Result<()> {
        let worker = AgentWorker::new(role.clone(), self.router)?;
        self.workers.insert(role.id, worker);
        Ok(())
    }

    /// 移除 Worker
    pub fn remove_worker(&mut self, role_id: &str) -> Option<AgentWorker<'a>> {
        self.workers.remove(role_id)
    }

    /// 获取 Worker（mutable）
    pub fn get_worker_mut(&mut self, role_id: &str) -> Option<&mut AgentWorker<'a>> {
        self.workers.get_mut(role_id)
    }

    /// 获取 Worker（只读）
    pub fn get_worker(&self, role_id: &str) -> Option<&AgentWorker<'a>> {
        self.workers.get(role_id)
    }

    /// 列出所有角色 ID
    pub fn role_ids(&self) -> Vec<String> {
        self.workers.keys().cloned().collect()
    }

    /// Worker 数量
    pub fn len(&self) -> usize {
        self.workers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    /// 获取所有 Worker 统计信息
    pub fn stats(&self) -> Vec<WorkerStats> {
        self.workers.values().map(|w| w.stats()).collect()
    }

    /// 并行执行多个角色的任务
    ///
    /// 参数：`tasks` 为 (role_id, task_prompt) 列表
    /// 返回：每个角色的执行结果
    pub async fn parallel_execute(
        &mut self,
        tasks: Vec<(String, String)>,
    ) -> Vec<WorkerTaskResult> {
        let mut results = Vec::with_capacity(tasks.len());

        for (role_id, task) in tasks {
            let result = match self.get_worker_mut(&role_id) {
                Some(worker) => {
                    let model = worker.current_model().to_string();
                    match worker.execute(&task).await {
                        Ok(output) => WorkerTaskResult {
                            role_id: role_id.clone(),
                            success: true,
                            output: Some(output),
                            error: None,
                            model_used: model,
                        },
                        Err(e) => WorkerTaskResult {
                            role_id: role_id.clone(),
                            success: false,
                            output: None,
                            error: Some(e.to_string()),
                            model_used: model,
                        },
                    }
                }
                None => WorkerTaskResult {
                    role_id: role_id.clone(),
                    success: false,
                    output: None,
                    error: Some(format!("角色 {role_id} 不存在")),
                    model_used: String::new(),
                },
            };
            results.push(result);
        }

        results
    }
}

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTaskResult {
    /// 角色 ID
    pub role_id: String,
    /// 是否成功
    pub success: bool,
    /// 输出内容（成功时）
    pub output: Option<String>,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 使用的模型
    pub model_used: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_router::ModelConfig;
    use crate::team_template::builtin_fullstack;

    fn make_test_router() -> ModelRouter {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::mock("mock-test"));
        router
    }

    fn make_test_role(id: &str, model: &str) -> Role {
        Role {
            id: id.to_string(),
            name: format!("角色-{id}"),
            description: "测试角色".to_string(),
            model: model.to_string(),
            model_fallback: vec![],
            skills: vec!["test-skill".to_string()],
            knowledge: vec![],
            permissions: vec!["can_test".to_string()],
            system_prompt: None,
        }
    }

    #[tokio::test]
    async fn test_worker_execute_success() {
        let router = make_test_router();
        let role = make_test_role("test", "mock-test");
        let mut worker = AgentWorker::new(role, &router).unwrap();

        assert_eq!(worker.state(), &WorkerState::Idle);

        let result = worker.execute("执行测试任务").await.unwrap();
        assert!(!result.is_empty());
        assert_eq!(worker.state(), &WorkerState::Idle);
        assert_eq!(worker.tasks_completed, 1);
        assert_eq!(worker.tasks_failed, 0);
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let router = make_test_router();
        let role = make_test_role("test", "mock-test");
        let mut worker = AgentWorker::new(role, &router).unwrap();

        worker.execute("task1").await.unwrap();
        worker.execute("task2").await.unwrap();

        let stats = worker.stats();
        assert_eq!(stats.role_id, "test");
        assert_eq!(stats.tasks_completed, 2);
        assert_eq!(stats.tasks_failed, 0);
        assert_eq!(stats.state, WorkerState::Idle);
        assert_eq!(stats.current_model, "mock-test");
    }

    #[test]
    fn test_worker_reset() {
        let router = make_test_router();
        let role = make_test_role("test", "mock-test");
        let mut worker = AgentWorker::new(role, &router).unwrap();

        // 模拟错误状态（直接设置）
        worker.state = WorkerState::Error;
        worker.last_error = Some("test error".to_string());

        worker.reset();
        assert_eq!(worker.state(), &WorkerState::Idle);
        assert!(worker.last_error.is_none());
    }

    #[test]
    fn test_worker_stop() {
        let router = make_test_router();
        let role = make_test_role("test", "mock-test");
        let mut worker = AgentWorker::new(role, &router).unwrap();

        worker.stop();
        assert_eq!(worker.state(), &WorkerState::Stopped);
    }

    #[test]
    fn test_worker_system_prompt() {
        let router = make_test_router();
        let role = Role {
            id: "backend".to_string(),
            name: "后端工程师".to_string(),
            description: "API 设计".to_string(),
            model: "mock-test".to_string(),
            model_fallback: vec![],
            skills: vec!["api-design".to_string()],
            knowledge: vec![],
            permissions: vec!["can_edit_backend".to_string()],
            system_prompt: None,
        };

        let worker = AgentWorker::new(role, &router).unwrap();
        let prompt = worker.build_system_prompt();
        assert!(prompt.contains("后端工程师"));
        assert!(prompt.contains("backend"));
        assert!(prompt.contains("api-design"));
        assert!(prompt.contains("can_edit_backend"));
    }

    #[test]
    fn test_worker_custom_system_prompt() {
        let router = make_test_router();
        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "mock-test".to_string(),
            model_fallback: vec![],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: Some("自定义提示词".to_string()),
        };

        let worker = AgentWorker::new(role, &router).unwrap();
        let prompt = worker.build_system_prompt();
        assert_eq!(prompt, "自定义提示词");
    }

    #[test]
    fn test_worker_pool_from_template() {
        let router = make_test_router();
        // 修改内置模板的模型为 mock
        let mut template = builtin_fullstack();
        for role in &mut template.team.roles {
            role.model = "mock-test".to_string();
            role.model_fallback = vec![];
        }

        let pool = WorkerPool::from_template(&router, &template).unwrap();
        assert_eq!(pool.len(), 4);
        assert!(pool.get_worker("orchestrator").is_some());
        assert!(pool.get_worker("backend").is_some());
        assert!(pool.get_worker("frontend").is_some());
        assert!(pool.get_worker("tester").is_some());
    }

    #[tokio::test]
    async fn test_worker_pool_parallel_execute() {
        let router = make_test_router();
        let mut template = builtin_fullstack();
        for role in &mut template.team.roles {
            role.model = "mock-test".to_string();
            role.model_fallback = vec![];
        }

        let mut pool = WorkerPool::from_template(&router, &template).unwrap();

        let tasks = vec![
            ("orchestrator".to_string(), "分析需求".to_string()),
            ("backend".to_string(), "设计 API".to_string()),
            ("tester".to_string(), "写测试".to_string()),
        ];

        let results = pool.parallel_execute(tasks).await;
        assert_eq!(results.len(), 3);

        for r in &results {
            assert!(r.success, "角色 {} 应成功: {:?}", r.role_id, r.error);
            assert!(r.output.is_some());
            assert_eq!(r.model_used, "mock-test");
        }
    }

    #[tokio::test]
    async fn test_worker_pool_parallel_with_missing_role() {
        let router = make_test_router();
        let mut pool = WorkerPool::new(&router);

        let tasks = vec![("nonexistent".to_string(), "task".to_string())];
        let results = pool.parallel_execute(tasks).await;

        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].error.as_ref().unwrap().contains("不存在"));
    }

    #[test]
    fn test_worker_pool_add_remove() {
        let router = make_test_router();
        let mut pool = WorkerPool::new(&router);

        let role = make_test_role("test", "mock-test");
        pool.add_worker(role).unwrap();
        assert_eq!(pool.len(), 1);

        let removed = pool.remove_worker("test");
        assert!(removed.is_some());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_worker_pool_stats() {
        let router = make_test_router();
        let mut template = builtin_fullstack();
        for role in &mut template.team.roles {
            role.model = "mock-test".to_string();
            role.model_fallback = vec![];
        }

        let pool = WorkerPool::from_template(&router, &template).unwrap();
        let stats = pool.stats();
        assert_eq!(stats.len(), 4);
        assert!(stats.iter().any(|s| s.role_id == "orchestrator"));
    }

    #[test]
    fn test_worker_creation_fails_for_unregistered_model() {
        let router = ModelRouter::new(); // 空路由器
        let role = make_test_role("test", "unregistered");
        let result = AgentWorker::new(role, &router);
        assert!(result.is_err());
    }
}
