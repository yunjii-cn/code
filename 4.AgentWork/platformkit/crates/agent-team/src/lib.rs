// agent-team: AI 团队协作引擎
// 设计文档: docs/AGENT-TEAM-DESIGN.md
//
// W7 M3.1: 团队模板 + 模型路由 + Agent 工作池
//   - D1 crate 初始化
//   - D2 团队模板定义（YAML 解析 + 角色结构）
//   - D3 模型路由层（主模型 + 备用 + 故障转移）
//   - D4 Agent 工作池（每角色一个 Worker）
//   - D5 桌面端团队配置 UI
//
// W8 M3.2: DAG 调度引擎
//   - D1 TaskDag 数据结构 + 依赖解析
//   - D2 DagScheduler 调度循环
//   - D3 任务状态机
//   - D4 共享上下文池
//   - D5 调度引擎单测 + 集成测试
//
// M4.0 D2-D7: AI 员工定义 + 流式输出 + ChatPanel + 交付物预览 + LLM Gateway
// M4.1 D1: 知识库系统（RAG 检索增强）
// M4.1 D2: 规则引擎（关键词 + 逻辑运算 + 5 种动作）
// M4.1 D3: 话术训练系统（Few-shot + LLM-as-Judge）
// M4.1 D4: 培训效果评估（EvalSuite + 批量跑测试集 + 基线对比）
// M4.1 D6: 云端备份（加密上传 + 多设备同步）
// M4.2 D1: 跨岗位协作（邮件式异步消息 + 工作流模板）
// M4.2 D2: 行业模板基础设施（加载 + 安装 + 版本 + 依赖）

#![warn(missing_docs)]

mod error;
mod team_template;
mod model_router;
mod agent_worker;
mod dag;
mod task_state;
mod scheduler;
mod context;
mod orchestrator;
mod coder;
mod retry;
mod employee;
mod stream;
mod knowledge;
mod rule;
mod speech;
mod evaluation;
mod sync;
mod collaboration;
mod template_loader;
mod template_market;

pub use error::{TeamError, Result};
pub use team_template::{
    TeamTemplate, TeamInfo, Role, RoleId,
    builtin_fullstack, builtin_backend, builtin_solo, builtin_templates,
};
pub use model_router::{
    ModelConfig, ModelRouter, ModelCaller, builtin_router,
};
pub use agent_worker::{
    AgentWorker, WorkerPool, WorkerState, WorkerStats, WorkerTaskResult,
};
pub use dag::{Task, TaskDag, TaskId};
pub use task_state::{TaskState, TaskStates, MAX_RETRIES};
pub use scheduler::{DagScheduler, ScheduleEvent, ScheduleAction};
pub use context::{
    SharedContext, ContextEntry, ContextCategory, AgentContext, Artifact, ArtifactType,
};
pub use orchestrator::{Orchestrator, OrchestratorConfig, extract_json};
pub use coder::{Coder, CoderConfig, CodeArtifact, FileChange, FileAction};
pub use retry::{RetryPolicy, RetryExecutor, RetryResult, ErrorClassifier, ErrorKind};
pub use employee::{
    EmployeeDefinition, Example,
    builtin_customer_service, builtin_developer, builtin_assistant,
    builtin_education_teacher, builtin_sales_followup, builtin_finance_auditor,
    builtin_employees, builtin_by_id, builtin_general_employees, builtin_industry_employees,
};
pub use stream::{StreamEvent, ToolCallStatus, StreamCallback};
pub use knowledge::{
    KnowledgeSource, KnowledgeChunk, KnowledgeBase,
    TextChunker, DocumentParser, SensitiveFilter,
    KnowledgeStore, RagRetriever, RagResult,
    DEFAULT_CHUNK_SIZE, DEFAULT_CHUNK_OVERLAP,
    estimate_tokens, is_cjk_char,
};
pub use rule::{
    LogicOp, RuleCondition, RuleAction, Rule, RuleSet, RuleMatchResult,
    RuleEngine, EngineResult,
    builtin_finance_rules, builtin_education_rules, builtin_medical_rules,
    builtin_all_rules, builtin_by_industry,
};
pub use speech::{
    SpeechStyle, SpeechExample, SpeechConstraints, ConstraintCheckResult, ConstraintViolation,
    SpeechTraining, FewShotBuilder, ConstraintChecker, JudgeResult, SpeechJudge,
    builtin_customer_service_speech, builtin_finance_advisor_speech, builtin_medical_consult_speech,
    builtin_all_speech,
};
pub use evaluation::{
    EvalCase, EvalSuite, EvalResult, EvalReport, CategoryStat,
    BaselineComparison, CaseComparison, EvaluationRunner,
    builtin_customer_service_eval, builtin_finance_advisor_eval, builtin_medical_consult_eval,
    builtin_all_evals, builtin_eval_by_employee,
};
pub use sync::{
    SyncConfig, FileEntry, SyncManifest, SyncPackage, CryptoUtil,
    SyncClient, SyncResult, ManifestDiff,
};
pub use collaboration::{
    MessageId, AgentId, MessagePriority, MessageStatus, MessagePayload,
    AgentMessage, MessageBus,
    StepTrigger, StepAction, WorkflowStep, WorkflowTemplate,
    WorkflowStatus, WorkflowInstance, WorkflowRunner,
    builtin_marketing_workflow, builtin_complaint_workflow, builtin_sales_followup_workflow,
    builtin_workflows, builtin_workflow_by_id,
};
pub use template_loader::{
    TemplateManifest, TemplateFileEntry, TemplateFileGroup, TemplateDependency,
    TemplateLoader, TemplateInstallResult, TemplateInstallReport,
    TemplateVersion, TemplateVersionCompat,
    builtin_template_registry, builtin_template_by_id,
};
pub use template_market::{
    MarketId, AuthorId, PricingType, ReviewStatus,
    TemplateRating, RatingSummary,
    TemplateMarketEntry, TemplateSubmission,
    AutoReviewDimension, AutoReviewResult, ManualReviewResult,
    TemplateMarketplace,
};

/// 便捷函数：从内置模板 + 内置路由器创建 Worker 池
pub fn create_pool_from_builtin(template_name: &str) -> Result<WorkerPool<'static>> {
    // 注：此函数返回 'static 生命周期需要 Box::leak，实际使用时
    // 推荐显式管理 router 生命周期
    Err(TeamError::Other(
        format!("请显式创建 ModelRouter 并调用 WorkerPool::from_template。template_name={template_name}")
    ))
}
