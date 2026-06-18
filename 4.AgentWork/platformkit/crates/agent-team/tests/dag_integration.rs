// DAG 调度引擎集成测试（W8 M3.2 D5）
//
// 测试 DagScheduler + SharedContext + TaskState 的端到端流程。
// 模拟真实调度循环：step → 分配任务 → 模拟 Agent 执行 → handle_event → 直到完成。

use agent_team::{
    Artifact, ContextCategory, ContextEntry, DagScheduler, ScheduleAction, ScheduleEvent,
    SharedContext, Task, TaskDag, TaskState,
};

/// 辅助函数：完成任务（Completed → Merged）
fn complete_task(scheduler: &mut DagScheduler, task_id: &str, snapshot_id: &str) {
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: task_id.to_string(),
            branch: format!("task/{task_id}"),
        })
        .unwrap();
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: task_id.to_string(),
            snapshot_id: snapshot_id.to_string(),
        })
        .unwrap();
}

/// 辅助函数：完成任务并写入上下文产出
fn complete_task_with_artifact(
    scheduler: &mut DagScheduler,
    context: &mut SharedContext,
    task_id: &str,
    snapshot_id: &str,
    artifact: Artifact,
) {
    complete_task(scheduler, task_id, snapshot_id);
    context.write_artifact(task_id, artifact);
}

// ============================================================================
// 场景 1：线性 DAG 完整流程
// t1 → t2 → t3
// ============================================================================
#[test]
fn test_integration_linear_workflow() {
    let mut dag = TaskDag::new("wf_linear", "线性工作流");
    dag.add_task(Task::new("t1", "需求分析", "pm")).unwrap();
    dag.add_task(
        Task::new("t2", "后端开发", "backend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t3", "测试验收", "tester").with_dependencies(vec!["t2".to_string()]),
    )
    .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // 初始状态
    assert!(!scheduler.is_finished());
    assert_eq!(scheduler.progress(), (0, 3));

    // step 1: 分配 t1
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);
    assert!(matches!(scheduler.task_state("t1"), Some(TaskState::Running { .. })));

    // t1 完成
    complete_task(&mut scheduler, "t1", "snap_001");
    assert_eq!(scheduler.progress(), (1, 3));

    // step 2: 分配 t2
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    // t2 完成
    complete_task(&mut scheduler, "t2", "snap_002");
    assert_eq!(scheduler.progress(), (2, 3));

    // step 3: 分配 t3
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    // t3 完成
    complete_task(&mut scheduler, "t3", "snap_003");

    // 所有任务完成
    assert!(scheduler.is_finished());
    assert_eq!(scheduler.progress(), (3, 3));
    assert_eq!(scheduler.progress_percent(), 100);
}

// ============================================================================
// 场景 2：并行 DAG 完整流程
// t1, t2 并行 → t3 依赖 t1 和 t2
// ============================================================================
#[test]
fn test_integration_parallel_workflow() {
    let mut dag = TaskDag::new("wf_parallel", "并行工作流");
    dag.add_task(Task::new("t1", "后端", "backend")).unwrap();
    dag.add_task(Task::new("t2", "前端", "frontend")).unwrap();
    dag.add_task(
        Task::new("t3", "集成测试", "tester")
            .with_dependencies(vec!["t1".to_string(), "t2".to_string()]),
    )
    .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // step 1: 分配 t1 和 t2（并行）
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 2);

    // t1 完成
    complete_task(&mut scheduler, "t1", "snap_001");

    // step 2: t2 还在运行，t3 依赖未满足
    let actions = scheduler.step().unwrap();
    assert!(actions.is_empty());

    // t2 完成
    complete_task(&mut scheduler, "t2", "snap_002");

    // step 3: 分配 t3
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    // t3 完成
    complete_task(&mut scheduler, "t3", "snap_003");

    assert!(scheduler.is_finished());
}

// ============================================================================
// 场景 3：菱形 DAG 完整流程
// t1 → t2, t1 → t3, t2 → t4, t3 → t4
// ============================================================================
#[test]
fn test_integration_diamond_workflow() {
    let mut dag = TaskDag::new("wf_diamond", "菱形工作流");
    dag.add_task(Task::new("t1", "基础架构", "backend")).unwrap();
    dag.add_task(
        Task::new("t2", "后端 API", "backend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t3", "前端页面", "frontend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t4", "集成测试", "tester")
            .with_dependencies(vec!["t2".to_string(), "t3".to_string()]),
    )
    .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // step 1: 分配 t1
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);
    complete_task(&mut scheduler, "t1", "snap_001");

    // step 2: 分配 t2 和 t3（并行）
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 2);
    complete_task(&mut scheduler, "t2", "snap_002");
    complete_task(&mut scheduler, "t3", "snap_003");

    // step 3: 分配 t4
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);
    complete_task(&mut scheduler, "t4", "snap_004");

    assert!(scheduler.is_finished());
    assert_eq!(scheduler.progress(), (4, 4));
}

// ============================================================================
// 场景 4：任务失败 + 重试 + 最终成功
// ============================================================================
#[test]
fn test_integration_task_failure_and_retry() {
    let mut dag = TaskDag::new("wf_retry", "重试工作流");
    dag.add_task(Task::new("t1", "任务1", "backend")).unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // 分配 t1
    let _ = scheduler.step().unwrap();

    // t1 执行失败
    let actions = scheduler
        .handle_event(ScheduleEvent::TaskFailed {
            task_id: "t1".to_string(),
            error: "编译错误".to_string(),
        })
        .unwrap();
    assert!(actions.is_empty()); // 首次失败不触发动作

    // t1 应该是 Failed 状态
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Failed { .. })
    ));

    // 重试 t1
    let action = scheduler.retry_task("t1", "backend", "task/t1").unwrap();
    assert!(matches!(action, ScheduleAction::AssignTask { .. }));

    // t1 重新执行成功
    complete_task(&mut scheduler, "t1", "snap_001");

    assert!(scheduler.is_finished());
}

// ============================================================================
// 场景 5：验证失败 + 重试 + 最终成功
// ============================================================================
#[test]
fn test_integration_verify_failure_and_retry() {
    let mut dag = TaskDag::new("wf_verify_retry", "验证重试工作流");
    dag.add_task(Task::new("t1", "任务1", "backend")).unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // 分配 t1
    let _ = scheduler.step().unwrap();

    // t1 执行完成
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "t1".to_string(),
            branch: "task/t1".to_string(),
        })
        .unwrap();

    // 验证失败
    let actions = scheduler
        .handle_event(ScheduleEvent::TaskVerifyFailed {
            task_id: "t1".to_string(),
            errors: vec!["测试失败".to_string()],
        })
        .unwrap();

    // 应该返回 RetryTask 动作
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], ScheduleAction::RetryTask { .. }));

    // 重试 t1
    let _ = scheduler.retry_task("t1", "backend", "task/t1").unwrap();

    // t1 重新执行并验证通过
    complete_task(&mut scheduler, "t1", "snap_001");

    assert!(scheduler.is_finished());
}

// ============================================================================
// 场景 6：上下文传递（上游任务产出对下游可见）
// ============================================================================
#[test]
fn test_integration_context_propagation() {
    let mut dag = TaskDag::new("wf_context", "上下文传递工作流");
    dag.add_task(Task::new("t1", "后端 API", "backend")).unwrap();
    dag.add_task(
        Task::new("t2", "前端集成", "frontend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();

    let mut context = SharedContext::new();

    // 添加项目规范
    context
        .add_entry(ContextEntry::new(
            "standard/coding",
            "使用 TypeScript + React",
            ContextCategory::Standard,
        ))
        .unwrap();

    // 添加后端文档（只对 backend 可见）
    context.add_entry(
        ContextEntry::new("docs/api.md", "# API 设计", ContextCategory::Documentation)
            .visible_to(vec!["backend".to_string()]),
    ).unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // step 1: 分配 t1（backend）
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    // 读取 backend 角色的上下文
    let t1 = scheduler.dag().get_task("t1").unwrap();
    let backend_ctx = context.read_for_role("backend", t1);
    assert!(!backend_ctx.standards.is_empty());
    assert!(backend_ctx.docs.iter().any(|d| d.key == "docs/api.md"));
    assert!(backend_ctx.upstream_artifacts.is_empty()); // t1 无依赖

    // t1 完成，写入产出
    complete_task_with_artifact(
        &mut scheduler,
        &mut context,
        "t1",
        "snap_001",
        Artifact::summary("后端 API 已完成，端点：/api/users"),
    );

    // step 2: 分配 t2（frontend）
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    // 读取 frontend 角色的上下文
    let t2 = scheduler.dag().get_task("t2").unwrap();
    let frontend_ctx = context.read_for_role("frontend", t2);

    // frontend 应该能看到：
    // - 项目规范（所有角色可见）
    // - 不能看到后端文档（只对 backend 可见）
    // - 上游任务 t1 的产出
    assert!(!frontend_ctx.standards.is_empty());
    assert!(!frontend_ctx.docs.iter().any(|d| d.key == "docs/api.md"));
    assert_eq!(frontend_ctx.upstream_artifacts.len(), 1);
    assert!(frontend_ctx.upstream_artifacts[0]
        .content
        .contains("后端 API 已完成"));

    // t2 完成
    complete_task(&mut scheduler, "t2", "snap_002");

    assert!(scheduler.is_finished());
}

// ============================================================================
// 场景 7：完整调度循环（模拟真实使用）
// ============================================================================
#[test]
fn test_integration_full_schedule_loop() {
    // 模拟一个全栈项目：需求 → 后端 + 前端（并行）→ 集成 → 测试
    let mut dag = TaskDag::new("wf_fullstack", "全栈项目");
    dag.add_task(Task::new("t1", "需求分析", "pm")).unwrap();
    dag.add_task(
        Task::new("t2", "后端开发", "backend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t3", "前端开发", "frontend").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t4", "集成联调", "backend")
            .with_dependencies(vec!["t2".to_string(), "t3".to_string()]),
    )
    .unwrap();
    dag.add_task(
        Task::new("t5", "测试验收", "tester").with_dependencies(vec!["t4".to_string()]),
    )
    .unwrap();

    let mut context = SharedContext::new();
    context
        .add_entry(ContextEntry::new(
            "standard/arch",
            "前后端分离架构",
            ContextCategory::Standard,
        ))
        .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();
    let mut snapshot_counter = 0;

    // 模拟调度循环
    let max_iterations = 100;
    for _ in 0..max_iterations {
        if scheduler.is_finished() {
            break;
        }

        // step: 找出 ready 任务并分配
        let actions = scheduler.step().unwrap();

        // 模拟所有已分配任务立即完成（简化测试）
        for action in &actions {
            if let ScheduleAction::AssignTask { task, .. } = action {
                snapshot_counter += 1;
                let snap = format!("snap_{snapshot_counter:03}");
                complete_task_with_artifact(
                    &mut scheduler,
                    &mut context,
                    &task.id,
                    &snap,
                    Artifact::summary(format!("任务 {} 完成", task.title)),
                );
            }
        }

        // 如果没有动作，说明在等待外部事件，但本测试中所有任务都立即完成
        if actions.is_empty() && !scheduler.is_finished() {
            // 可能是所有 ready 任务都已分配但还在运行中
            // 在本测试中不会发生（我们立即完成）
            break;
        }
    }

    assert!(scheduler.is_finished());
    assert_eq!(scheduler.progress(), (5, 5));
    assert_eq!(scheduler.progress_percent(), 100);

    // 验证上下文池中有 5 个任务产出
    assert_eq!(context.artifact_count(), 5);
}

// ============================================================================
// 场景 8：DAG JSON 序列化往返
// ============================================================================
#[test]
fn test_integration_dag_json_roundtrip() {
    let mut dag = TaskDag::new("wf_json", "JSON 序列化测试");
    dag.add_task(
        Task::new("t1", "任务1", "backend").with_description("详细描述").with_acceptance("验收标准"),
    )
    .unwrap();
    dag.add_task(
        Task::new("t2", "任务2", "frontend")
            .with_dependencies(vec!["t1".to_string()])
            .with_description("前端任务")
            .with_acceptance("前端验收"),
    )
    .unwrap();

    let json = dag.to_json().unwrap();
    let parsed = TaskDag::from_json(&json).unwrap();

    assert_eq!(parsed.workflow_id, "wf_json");
    assert_eq!(parsed.task_count(), 2);

    let scheduler1 = DagScheduler::new(dag).unwrap();
    let scheduler2 = DagScheduler::new(parsed).unwrap();

    assert_eq!(scheduler1.dag().task_count(), scheduler2.dag().task_count());
    assert_eq!(scheduler1.progress(), scheduler2.progress());
}

// ============================================================================
// 场景 9：SharedContext JSON 序列化往返
// ============================================================================
#[test]
fn test_integration_context_json_roundtrip() {
    let mut context = SharedContext::new();
    context
        .add_entry(ContextEntry::new("k1", "v1", ContextCategory::Code))
        .unwrap();
    context.add_entry(
        ContextEntry::new("k2", "v2", ContextCategory::Documentation)
            .visible_to(vec!["backend".to_string()]),
    ).unwrap();
    context.add_artifact("t1", Artifact::summary("完成"));

    let json = context.to_json().unwrap();
    let parsed = SharedContext::from_json(&json).unwrap();

    assert_eq!(parsed.entry_count(), 2);
    assert_eq!(parsed.artifact_count(), 1);

    // 验证可见角色保留
    let entry = parsed.get_entry("k2").unwrap();
    assert_eq!(entry.visible_roles, vec!["backend"]);
}

// ============================================================================
// 场景 10：多角色上下文隔离
// ============================================================================
#[test]
fn test_integration_role_isolation() {
    let mut context = SharedContext::new();

    // 数据库 Schema（只对 backend 和 db 可见）
    context.add_entry(
        ContextEntry::new("db/schema.sql", "CREATE TABLE users...", ContextCategory::Code)
            .visible_to(vec!["backend".to_string(), "db".to_string()]),
    ).unwrap();

    // UI 设计稿（只对 frontend 可见）
    context.add_entry(
        ContextEntry::new("design/ui.md", "# UI 设计", ContextCategory::Documentation)
            .visible_to(vec!["frontend".to_string()]),
    ).unwrap();

    // 通用规范（所有角色可见）
    context
        .add_entry(ContextEntry::new("standard/general", "通用规范", ContextCategory::Standard))
        .unwrap();

    let task = Task::new("t1", "任务", "backend");

    // backend 角色应该看到：数据库 Schema + 通用规范，看不到 UI 设计稿
    let backend_ctx = context.read_for_role("backend", &task);
    assert!(backend_ctx.code_snippets.iter().any(|c| c.key == "db/schema.sql"));
    assert!(!backend_ctx.docs.iter().any(|d| d.key == "design/ui.md"));
    assert!(!backend_ctx.standards.is_empty());

    // frontend 角色应该看到：UI 设计稿 + 通用规范，看不到数据库 Schema
    let frontend_ctx = context.read_for_role("frontend", &task);
    assert!(frontend_ctx.docs.iter().any(|d| d.key == "design/ui.md"));
    assert!(!frontend_ctx.code_snippets.iter().any(|c| c.key == "db/schema.sql"));
    assert!(!frontend_ctx.standards.is_empty());

    // tester 角色应该只看到：通用规范
    let tester_ctx = context.read_for_role("tester", &task);
    assert!(tester_ctx.code_snippets.is_empty());
    assert!(tester_ctx.docs.is_empty());
    assert!(!tester_ctx.standards.is_empty());
}

// ============================================================================
// 场景 11：Agent 上下文 prompt 生成
// ============================================================================
#[test]
fn test_integration_agent_context_prompt() {
    let mut context = SharedContext::new();
    context
        .add_entry(ContextEntry::new(
            "standard/coding",
            "使用 4 空格缩进，禁止使用 tab",
            ContextCategory::Standard,
        ))
        .unwrap();
    context
        .add_entry(ContextEntry::new(
            "docs/api.md",
            "# API 文档\n## /users\n获取用户列表",
            ContextCategory::Documentation,
        ))
        .unwrap();
    context
        .add_entry(ContextEntry::new(
            "src/main.rs",
            "fn main() { println!(\"hello\"); }",
            ContextCategory::Code,
        ))
        .unwrap();
    context.add_artifact("t1", Artifact::summary("后端 API 已完成"));

    let task = Task::new("t2", "前端集成", "frontend")
        .with_dependencies(vec!["t1".to_string()]);

    let agent_ctx = context.read_for_role("frontend", &task);
    let prompt = agent_ctx.to_prompt_context();

    // 验证 prompt 包含所有上下文
    assert!(prompt.contains("项目规范"));
    assert!(prompt.contains("相关文档"));
    assert!(prompt.contains("相关代码"));
    assert!(prompt.contains("上游任务产出"));
    assert!(prompt.contains("4 空格缩进"));
    assert!(prompt.contains("API 文档"));
    assert!(prompt.contains("fn main()"));
    assert!(prompt.contains("后端 API 已完成"));
}
