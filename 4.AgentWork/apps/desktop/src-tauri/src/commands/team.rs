// 团队管理命令（W7 M3.1 D5 + W9 M3.3 D4）
// - 列出内置团队模板
// - 加载/保存团队配置
// - 列出已注册模型
// - 获取 Worker 池状态
// - 工作流管理（W9 M3.3 D4）：plan / step / 报告事件

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Emitter;

use crate::AppResult;
use agent_team::{
    builtin_router, builtin_templates, DagScheduler, ModelConfig, ModelRouter, ScheduleAction,
    ScheduleEvent, Task, TaskDag, TaskState, TeamTemplate,
};

/// 团队模板信息（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTemplateInfo {
    /// 模板名（如 "fullstack"）
    pub name: String,
    /// 团队名称
    pub team_name: String,
    /// 团队描述
    pub description: String,
    /// 角色数量
    pub role_count: usize,
    /// 角色 ID 列表
    pub role_ids: Vec<String>,
}

/// 角色信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub model: String,
    pub model_fallback: Vec<String>,
    pub skills: Vec<String>,
    pub permissions: Vec<String>,
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub base_url: String,
    pub model_name: String,
    pub has_api_key: bool,
}

/// Worker 状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub role_id: String,
    pub role_name: String,
    pub state: String,
    pub current_model: String,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
    pub last_error: Option<String>,
}

/// 全局团队状态（存于 AppState）
pub struct TeamState {
    /// 当前团队模板
    pub template: Mutex<Option<TeamTemplate>>,
    /// 模型路由器
    pub router: Mutex<ModelRouter>,
    // 注：WorkerPool 持有对 router 的引用，无法直接存 Mutex<WorkerPool>
    // 所以这里只存配置，执行时临时创建 WorkerPool
    /// 当前工作流（W9 M3.3 D4）
    pub workflow: Mutex<WorkflowState>,
}

impl Default for TeamState {
    fn default() -> Self {
        Self {
            template: Mutex::new(None),
            router: Mutex::new(builtin_router()),
            workflow: Mutex::new(WorkflowState::default()),
        }
    }
}

/// 列出内置团队模板
#[tauri::command]
pub async fn list_team_templates() -> AppResult<Vec<TeamTemplateInfo>> {
    let templates = builtin_templates();
    let mut out = Vec::with_capacity(templates.len());
    for (name, template) in templates {
        out.push(TeamTemplateInfo {
            name,
            team_name: template.team.name.clone(),
            description: template.team.description.clone(),
            role_count: template.team.roles.len(),
            role_ids: template.team.role_ids(),
        });
    }
    Ok(out)
}

/// 加载团队模板（按名称）
#[tauri::command]
pub async fn load_team_template(
    name: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let templates = builtin_templates();
    let template = templates
        .get(&name)
        .ok_or_else(|| crate::AppError::Other(format!("未知团队模板: {name}")))?
        .clone();

    let info = format!(
        "已加载团队 '{}'（{} 个角色）",
        template.team.name,
        template.team.roles.len()
    );

    *state.team.template.lock().unwrap() = Some(template);
    tracing::info!("团队模板已加载: {}", name);
    Ok(info)
}

/// 获取当前团队配置
#[tauri::command]
pub async fn get_team_config(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<RoleInfo>> {
    let template = state.team.template.lock().unwrap();
    match template.as_ref() {
        Some(t) => {
            let roles: Vec<RoleInfo> = t
                .team
                .roles
                .iter()
                .map(|r| RoleInfo {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    description: r.description.clone(),
                    model: r.model.clone(),
                    model_fallback: r.model_fallback.clone(),
                    skills: r.skills.clone(),
                    permissions: r.permissions.clone(),
                })
                .collect();
            Ok(roles)
        }
        None => Ok(vec![]),
    }
}

/// 列出已注册模型
#[tauri::command]
pub async fn list_models(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<ModelInfo>> {
    let router = state.team.router.lock().unwrap();
    let mut out = Vec::new();
    for id in router.list_models() {
        if let Some(config) = router.get(&id) {
            out.push(ModelInfo {
                id: config.id.clone(),
                provider: format!("{:?}", config.provider).to_lowercase(),
                base_url: config.base_url.clone(),
                model_name: config.model_name.clone(),
                has_api_key: config.api_key.is_some() && !config.api_key.as_ref().unwrap().is_empty(),
            });
        }
    }
    Ok(out)
}

/// 注册自定义模型
#[tauri::command]
pub async fn register_model(
    id: String,
    provider: String,
    base_url: String,
    api_key: Option<String>,
    model_name: String,
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
) -> AppResult<String> {
    let provider_parsed: timeflow_ai::LlmProvider = provider
        .parse()
        .map_err(|e: timeflow_ai::AiError| crate::AppError::Other(e.to_string()))?;

    let config = ModelConfig {
        id: id.clone(),
        provider: provider_parsed,
        base_url,
        api_key: api_key.filter(|k| !k.is_empty()),
        model_name,
        timeout_secs: 60,
        max_tokens: 4096,
        temperature: 0.3,
        routing_hint: timeflow_ai::RoutingHint::Chat,
    };

    let mut router = state.team.router.lock().unwrap();
    router.register(config);
    tracing::info!("模型已注册: {}", id);

    // 持久化：保存当前所有模型列表
    let models: Vec<ModelConfig> = router.list_models().into_iter().map(|mid| {
        router.get(&mid).cloned().unwrap()
    }).collect();
    crate::config_store::save(&app, "custom_models", &models);

    Ok(format!("模型 {id} 已注册"))
}

/// 获取 Worker 池状态
///
/// 注：由于 WorkerPool 生命周期问题，这里只返回模板中定义的角色信息
/// 实际运行状态需在任务执行后查询
#[tauri::command]
pub async fn get_team_status(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<WorkerInfo>> {
    let template = state.team.template.lock().unwrap();
    match template.as_ref() {
        Some(t) => {
            let workers: Vec<WorkerInfo> = t
                .team
                .roles
                .iter()
                .map(|r| WorkerInfo {
                    role_id: r.id.clone(),
                    role_name: r.name.clone(),
                    state: "idle".to_string(),
                    current_model: r.model.clone(),
                    tasks_completed: 0,
                    tasks_failed: 0,
                    last_error: None,
                })
                .collect();
            Ok(workers)
        }
        None => Ok(vec![]),
    }
}

/// 执行团队任务（单角色，测试用）
///
/// W8 M3.2 将实现完整的 DAG 调度
#[tauri::command]
pub async fn execute_team_task(
    role_id: String,
    task: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let template = state
        .team
        .template
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| crate::AppError::Other("未加载团队模板".to_string()))?;

    let role = template
        .team
        .get_role(&role_id)
        .ok_or_else(|| crate::AppError::Other(format!("角色 {role_id} 不存在")))?
        .clone();

    // 先 clone router 释放 guard，避免 MutexGuard 跨 await（Send 问题）
    let router = state.team.router.lock().unwrap().clone();
    let mut worker = agent_team::AgentWorker::new(role, &router)
        .map_err(|e| crate::AppError::Other(e.to_string()))?;

    let result = worker
        .execute(&task)
        .await
        .map_err(|e| crate::AppError::Other(e.to_string()))?;

    Ok(result)
}

// ===== 工作流管理（W9 M3.3 D4）=====

/// 工作流状态（内存中维护，MVP 不持久化）
#[derive(Default)]
pub struct WorkflowState {
    /// 当前 DAG
    pub dag: Option<TaskDag>,
    /// 调度器
    pub scheduler: Option<DagScheduler>,
    /// 工作流 ID
    pub workflow_id: Option<String>,
    /// 原始需求
    pub requirement: Option<String>,
    /// 调度动作日志（最近 N 条）
    pub action_log: Vec<ActionLogEntry>,
}

/// 调度动作日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    /// 时间戳（ISO 8601）
    pub timestamp: String,
    /// 动作类型
    pub action_type: String,
    /// 任务 ID
    pub task_id: String,
    /// 描述
    pub description: String,
}

/// 任务状态信息（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateInfo {
    /// 任务 ID
    pub task_id: String,
    /// 状态类型（Pending / Running / Verifying / Merged / Failed / Rejected）
    pub state_type: String,
    /// 状态详情（人类可读）
    pub state_detail: String,
    /// 分支（Running / Verifying 状态有）
    pub branch: Option<String>,
    /// Agent 角色（Running 状态有）
    pub agent: Option<String>,
    /// 错误信息（Failed 状态有）
    pub error: Option<String>,
    /// 重试次数（Failed 状态有）
    pub retry_count: u32,
    /// 快照 ID（Merged 状态有）
    pub snapshot_id: Option<String>,
}

/// 任务信息（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// 任务 ID
    pub id: String,
    /// 任务标题
    pub title: String,
    /// 分配给哪个角色
    pub assignee: String,
    /// 依赖的任务 ID 列表
    pub dependencies: Vec<String>,
    /// 预估工作量
    pub estimated_effort: String,
    /// 验收标准
    pub acceptance_criteria: String,
    /// 任务描述
    pub description: String,
    /// 当前状态
    pub state: TaskStateInfo,
}

/// 工作流概览（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOverview {
    /// 工作流 ID
    pub workflow_id: Option<String>,
    /// 原始需求
    pub requirement: Option<String>,
    /// 任务列表（含状态）
    pub tasks: Vec<TaskInfo>,
    /// 已完成任务数
    pub completed_count: usize,
    /// 总任务数
    pub total_count: usize,
    /// 进度百分比（0-100）
    pub progress_percent: u32,
    /// 是否已完成（所有任务终态）
    pub is_finished: bool,
    /// 调度动作日志
    pub action_log: Vec<ActionLogEntry>,
}

/// 调度动作信息（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleActionInfo {
    /// 动作类型（AssignTask / RetryTask / RejectTask）
    pub action_type: String,
    /// 任务 ID
    pub task_id: String,
    /// 分支
    pub branch: Option<String>,
    /// 分配给的角色
    pub assignee: Option<String>,
    /// 错误信息（RetryTask 有）
    pub errors: Vec<String>,
    /// 拒绝原因（RejectTask 有）
    pub reason: Option<String>,
    /// 任务标题
    pub task_title: String,
}

/// 将 ScheduleAction 转换为前端可读结构
fn action_to_info(action: &ScheduleAction) -> ScheduleActionInfo {
    match action {
        ScheduleAction::AssignTask {
            task,
            branch,
            assignee,
        } => ScheduleActionInfo {
            action_type: "AssignTask".to_string(),
            task_id: task.id.clone(),
            branch: Some(branch.clone()),
            assignee: Some(assignee.clone()),
            errors: vec![],
            reason: None,
            task_title: task.title.clone(),
        },
        ScheduleAction::RetryTask {
            task_id,
            branch,
            errors,
        } => ScheduleActionInfo {
            action_type: "RetryTask".to_string(),
            task_id: task_id.clone(),
            branch: Some(branch.clone()),
            assignee: None,
            errors: errors.clone(),
            reason: None,
            task_title: String::new(),
        },
        ScheduleAction::RejectTask { task_id, reason } => ScheduleActionInfo {
            action_type: "RejectTask".to_string(),
            task_id: task_id.clone(),
            branch: None,
            assignee: None,
            errors: vec![],
            reason: Some(reason.clone()),
            task_title: String::new(),
        },
    }
}

/// 将 TaskState 转换为前端可读结构
fn state_to_info(task_id: &str, state: &TaskState) -> TaskStateInfo {
    match state {
        TaskState::Pending => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Pending".to_string(),
            state_detail: "待执行".to_string(),
            branch: None,
            agent: None,
            error: None,
            retry_count: 0,
            snapshot_id: None,
        },
        TaskState::Running {
            agent,
            branch,
            started_at,
        } => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Running".to_string(),
            state_detail: format!("执行中（{agent} @ {branch}，开始于 {started_at}）"),
            branch: Some(branch.clone()),
            agent: Some(agent.clone()),
            error: None,
            retry_count: 0,
            snapshot_id: None,
        },
        TaskState::Verifying { branch } => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Verifying".to_string(),
            state_detail: format!("验证中（分支 {branch}）"),
            branch: Some(branch.clone()),
            agent: None,
            error: None,
            retry_count: 0,
            snapshot_id: None,
        },
        TaskState::Merged {
            snapshot_id,
            merged_at,
        } => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Merged".to_string(),
            state_detail: format!("已合并（快照 {snapshot_id} @ {merged_at}）"),
            branch: None,
            agent: None,
            error: None,
            retry_count: 0,
            snapshot_id: Some(snapshot_id.clone()),
        },
        TaskState::Failed {
            error,
            retry_count,
            failed_at,
        } => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Failed".to_string(),
            state_detail: format!(
                "失败（重试 {retry_count}/3 @ {failed_at}）：{error}"
            ),
            branch: None,
            agent: None,
            error: Some(error.clone()),
            retry_count: *retry_count,
            snapshot_id: None,
        },
        TaskState::Rejected {
            reason,
            rejected_at,
        } => TaskStateInfo {
            task_id: task_id.to_string(),
            state_type: "Rejected".to_string(),
            state_detail: format!("已拒绝（{reason} @ {rejected_at}）"),
            branch: None,
            agent: None,
            error: Some(reason.clone()),
            retry_count: 3,
            snapshot_id: None,
        },
    }
}

/// 构建工作流概览
fn build_overview(workflow: &WorkflowState) -> WorkflowOverview {
    let (tasks, completed_count, total_count, progress_percent, is_finished) =
        match (&workflow.dag, &workflow.scheduler) {
            (Some(dag), Some(scheduler)) => {
                let tasks: Vec<TaskInfo> = dag
                    .tasks
                    .iter()
                    .map(|t| {
                        let state = scheduler
                            .task_state(&t.id)
                            .cloned()
                            .unwrap_or(TaskState::Pending);
                        TaskInfo {
                            id: t.id.clone(),
                            title: t.title.clone(),
                            assignee: t.assignee.clone(),
                            dependencies: t.dependencies.clone(),
                            estimated_effort: t.estimated_effort.clone(),
                            acceptance_criteria: t.acceptance_criteria.clone(),
                            description: t.description.clone(),
                            state: state_to_info(&t.id, &state),
                        }
                    })
                    .collect();
                let (completed, total) = scheduler.progress();
                (
                    tasks,
                    completed,
                    total,
                    scheduler.progress_percent(),
                    scheduler.is_finished(),
                )
            }
            _ => (vec![], 0, 0, 0, false),
        };

    WorkflowOverview {
        workflow_id: workflow.workflow_id.clone(),
        requirement: workflow.requirement.clone(),
        tasks,
        completed_count,
        total_count,
        progress_percent,
        is_finished,
        action_log: workflow.action_log.clone(),
    }
}

/// 规划工作流（使用内置示例 DAG，MVP 不实际调用 LLM）
///
/// 注：W9 M3.3 D4 桌面端 MVP 阶段，使用内置示例 DAG 演示调度流程。
/// 后续 W10+ 接入真实 LLM（Orchestrator::plan）。
#[tauri::command]
pub async fn plan_workflow(
    requirement: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WorkflowOverview> {
    let dag = build_demo_dag(&requirement);

    let scheduler = DagScheduler::new(dag.clone())
        .map_err(|e| crate::AppError::Other(format!("DAG 校验失败: {e}")))?;

    let mut workflow = state.team.workflow.lock().unwrap();
    workflow.dag = Some(dag.clone());
    workflow.scheduler = Some(scheduler);
    workflow.workflow_id = Some(dag.workflow_id.clone());
    workflow.requirement = Some(requirement.clone());
    workflow.action_log.clear();
    workflow.action_log.push(ActionLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action_type: "PlanWorkflow".to_string(),
        task_id: "-".to_string(),
        description: format!(
            "工作流 {} 已规划（{} 个任务）：{}",
            dag.workflow_id,
            dag.tasks.len(),
            requirement
        ),
    });

    tracing::info!(
        "工作流已规划: {} ({} 个任务)",
        dag.workflow_id,
        dag.tasks.len()
    );
    Ok(build_overview(&workflow))
}

/// 构建示例 DAG（用于 MVP 演示）
///
/// 场景："添加健康检查接口"
///   task-1 (backend) → task-2 (tester)
fn build_demo_dag(requirement: &str) -> TaskDag {
    let mut dag = TaskDag::new("wf_demo_001", requirement);
    dag.add_task(
        Task::new("task-1", "实现健康检查接口", "backend")
            .with_description("实现 GET /api/health 接口，返回 {\"status\":\"ok\"}")
            .with_acceptance("接口可访问，返回 200 + {\"status\":\"ok\"}"),
    )
    .ok();
    dag.add_task(
        Task::new(
            "task-2",
            "编写健康检查测试",
            "tester",
        )
        .with_dependencies(vec!["task-1".to_string()])
        .with_description("编写测试用例 test_health_check_returns_ok")
        .with_acceptance("测试通过，覆盖 200 和 503 两种情况"),
    )
    .ok();
    dag
}

/// 获取当前工作流概览
#[tauri::command]
pub async fn get_current_workflow(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WorkflowOverview> {
    let workflow = state.team.workflow.lock().unwrap();
    Ok(build_overview(&workflow))
}

/// 推进一步调度（找出 ready 任务并分配）
///
/// 返回本次调度产生的动作列表（AssignTask / RetryTask / RejectTask）。
#[tauri::command]
pub async fn step_workflow(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<(WorkflowOverview, Vec<ScheduleActionInfo>)> {
    let mut workflow = state.team.workflow.lock().unwrap();

    let scheduler = workflow
        .scheduler
        .as_mut()
        .ok_or_else(|| crate::AppError::Other("未规划工作流，请先调用 plan_workflow".to_string()))?;

    let actions = scheduler
        .step()
        .map_err(|e| crate::AppError::Other(format!("调度失败: {e}")))?;

    let now = chrono::Utc::now().to_rfc3339();
    for action in &actions {
        let info = action_to_info(action);
        let desc = match action {
            ScheduleAction::AssignTask {
                task, branch, ..
            } => format!(
                "分配任务 '{}' 给 {} @ 分支 {}",
                task.title, info.assignee.as_deref().unwrap_or("?"), branch
            ),
            ScheduleAction::RetryTask {
                task_id, errors, ..
            } => format!(
                "重试任务 {}（错误: {}）",
                task_id,
                errors.join("; ")
            ),
            ScheduleAction::RejectTask {
                task_id, reason, ..
            } => format!("拒绝任务 {task_id}：{reason}"),
        };
        workflow.action_log.push(ActionLogEntry {
            timestamp: now.clone(),
            action_type: info.action_type.clone(),
            task_id: info.task_id.clone(),
            description: desc,
        });
    }

    // 限制日志条数
    if workflow.action_log.len() > 50 {
        let drain = workflow.action_log.len() - 50;
        workflow.action_log.drain(0..drain);
    }

    let overview = build_overview(&workflow);
    let action_infos = actions.iter().map(action_to_info).collect();
    Ok((overview, action_infos))
}

/// 报告任务执行完成（Running → Verifying）
#[tauri::command]
pub async fn report_task_completed(
    task_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WorkflowOverview> {
    let mut workflow = state.team.workflow.lock().unwrap();
    let scheduler = workflow
        .scheduler
        .as_mut()
        .ok_or_else(|| crate::AppError::Other("未规划工作流".to_string()))?;

    let branch = scheduler
        .task_state(&task_id)
        .and_then(|s| s.branch().map(|b| b.to_string()))
        .unwrap_or_else(|| format!("task/{task_id}"));

    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: task_id.clone(),
            branch: branch.clone(),
        })
        .map_err(|e| crate::AppError::Other(format!("处理事件失败: {e}")))?;

    workflow.action_log.push(ActionLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action_type: "TaskCompleted".to_string(),
        task_id: task_id.clone(),
        description: format!("任务 {task_id} 执行完成，进入验证（分支 {branch}）"),
    });

    Ok(build_overview(&workflow))
}

/// 报告任务验证通过（Verifying → Merged）
#[tauri::command]
pub async fn report_task_merged(
    task_id: String,
    snapshot_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WorkflowOverview> {
    let mut workflow = state.team.workflow.lock().unwrap();
    let scheduler = workflow
        .scheduler
        .as_mut()
        .ok_or_else(|| crate::AppError::Other("未规划工作流".to_string()))?;

    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: task_id.clone(),
            snapshot_id: snapshot_id.clone(),
        })
        .map_err(|e| crate::AppError::Other(format!("处理事件失败: {e}")))?;

    workflow.action_log.push(ActionLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action_type: "TaskMerged".to_string(),
        task_id: task_id.clone(),
        description: format!(
            "任务 {task_id} 验证通过，已合并（快照 {snapshot_id}）"
        ),
    });

    Ok(build_overview(&workflow))
}

/// 报告任务验证失败（Verifying → Failed，可能触发重试）
#[tauri::command]
pub async fn report_task_verify_failed(
    task_id: String,
    errors: Vec<String>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<(WorkflowOverview, Vec<ScheduleActionInfo>)> {
    let mut workflow = state.team.workflow.lock().unwrap();
    let scheduler = workflow
        .scheduler
        .as_mut()
        .ok_or_else(|| crate::AppError::Other("未规划工作流".to_string()))?;

    let actions = scheduler
        .handle_event(ScheduleEvent::TaskVerifyFailed {
            task_id: task_id.clone(),
            errors: errors.clone(),
        })
        .map_err(|e| crate::AppError::Other(format!("处理事件失败: {e}")))?;

    workflow.action_log.push(ActionLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action_type: "TaskVerifyFailed".to_string(),
        task_id: task_id.clone(),
        description: format!(
            "任务 {} 验证失败（错误: {}）→ 触发 {} 个动作",
            task_id,
            errors.join("; "),
            actions.len()
        ),
    });

    let overview = build_overview(&workflow);
    let action_infos = actions.iter().map(action_to_info).collect();
    Ok((overview, action_infos))
}

/// 报告任务执行失败（Running → Failed）
#[tauri::command]
pub async fn report_task_failed(
    task_id: String,
    error: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WorkflowOverview> {
    let mut workflow = state.team.workflow.lock().unwrap();
    let scheduler = workflow
        .scheduler
        .as_mut()
        .ok_or_else(|| crate::AppError::Other("未规划工作流".to_string()))?;

    scheduler
        .handle_event(ScheduleEvent::TaskFailed {
            task_id: task_id.clone(),
            error: error.clone(),
        })
        .map_err(|e| crate::AppError::Other(format!("处理事件失败: {e}")))?;

    workflow.action_log.push(ActionLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action_type: "TaskFailed".to_string(),
        task_id: task_id.clone(),
        description: format!("任务 {task_id} 执行失败：{error}"),
    });

    Ok(build_overview(&workflow))
}

/// 重置工作流
#[tauri::command]
pub async fn reset_workflow(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let mut workflow = state.team.workflow.lock().unwrap();
    *workflow = WorkflowState::default();
    Ok("工作流已重置".to_string())
}

// ===== 流式输出（M4.0 D3）=====

/// 流式推送 Agent 思考过程
///
/// MVP 实现：模拟 Orchestrator 拆解需求的过程，分段推送 thinking 事件。
/// 真实 LLM 集成在 M4.0 D4 ChatPanel 中实现。
///
/// 前端监听 `agent_stream` 事件：
/// ```ts
/// import { listen } from "@tauri-apps/api/event";
/// listen<StreamEvent>("agent_stream", (e) => {
///   console.log(e.payload.type, e.payload);
/// });
/// ```
#[tauri::command]
pub async fn stream_agent_thinking(
    requirement: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    use agent_team::StreamEvent;
    use std::time::Instant;

    let workflow_id = format!("wf_stream_{}", chrono::Utc::now().timestamp_millis());
    let start = Instant::now();

    // 获取当前工作流 ID（如果已规划）
    let wf_id = {
        let wf = state.team.workflow.lock().unwrap();
        wf.workflow_id.clone().unwrap_or_else(|| workflow_id.clone())
    };

    // 模拟 Orchestrator 思考过程（分段推送）
    let thinking_steps = vec![
        "正在分析需求...",
        "识别关键任务...",
        "生成任务依赖图...",
        "分配角色...",
        "工作流规划完成。",
    ];

    let mut accumulated = 0usize;
    for step in &thinking_steps {
        accumulated += step.chars().count();
        let event = StreamEvent::thinking(
            &wf_id,
            None,
            "orchestrator",
            *step,
            accumulated,
        );
        app_handle.emit("agent_stream", &event).map_err(|e| {
            crate::AppError::Other(format!("推送事件失败: {e}"))
        })?;
        // 模拟思考延迟（200ms）
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // 推送完成事件
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let done_event = StreamEvent::done(
        &wf_id,
        elapsed_ms,
        0,
        0,
        format!("需求「{requirement}」的思考过程已推送完毕"),
    );
    app_handle.emit("agent_stream", &done_event).map_err(|e| {
        crate::AppError::Other(format!("推送完成事件失败: {e}"))
    })?;

    Ok(wf_id)
}

/// 流式推送任务执行进度
///
/// 模拟单个任务从 Pending → Running → Verifying → Merged 的过程。
#[tauri::command]
pub async fn stream_task_progress(
    task_id: String,
    task_title: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    use agent_team::StreamEvent;

    let wf_id = {
        let wf = state.team.workflow.lock().unwrap();
        wf.workflow_id.clone().unwrap_or_else(|| "wf_demo".to_string())
    };

    // 模拟任务状态流转
    let states = vec![
        ("Pending", 0u8),
        ("Running", 30),
        ("Running", 60),
        ("Verifying", 80),
        ("Merged", 100),
    ];

    for (new_state, percent) in &states {
        let event = StreamEvent::progress(
            &wf_id,
            &task_id,
            &task_title,
            *new_state,
            *percent,
        );
        app_handle.emit("agent_stream", &event).map_err(|e| {
            crate::AppError::Other(format!("推送进度失败: {e}"))
        })?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    Ok(format!("任务 {task_id} 进度推送完毕"))
}
