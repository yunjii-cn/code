// 双 Agent 协作 E2E 测试（W9 M3.3 D3）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 9
//
// 验证场景：用户输入需求 → Orchestrator 拆分 → Coder 执行 → 任务完成
//
// MVP 场景："添加一个健康检查接口 /api/health"
//   1. Orchestrator (Qwen3.7) 拆分任务：[后端实现 /api/health, 测试用例]
//   2. Coder (GLM5.2) 执行后端任务：实现 GET /api/health 返回 {"status":"ok"}
//   3. Coder (GLM5.2) 执行测试任务：编写 test_health_check_returns_ok
//   4. Orchestrator 验收通过

use agent_team::{
    AgentContext, Coder, ContextCategory, ContextEntry, DagScheduler,
    FileAction, Orchestrator, Role, ScheduleAction, ScheduleEvent, SharedContext, Task,
    TaskDag, TaskState,
};
use timeflow_ai::MockEngine;

// ===== 辅助函数 =====

fn make_orchestrator_role() -> Role {
    Role {
        id: "orchestrator".to_string(),
        name: "项目负责人".to_string(),
        description: "需求分析、任务拆分、DAG 生成".to_string(),
        model: "qwen3.7".to_string(),
        model_fallback: vec![],
        skills: vec!["planning".to_string(), "review".to_string()],
        knowledge: vec![],
        permissions: vec!["review".to_string()],
        system_prompt: None,
    }
}

fn make_backend_role() -> Role {
    Role {
        id: "backend".to_string(),
        name: "后端工程师".to_string(),
        description: "API 设计、业务逻辑".to_string(),
        model: "glm5.2".to_string(),
        model_fallback: vec![],
        skills: vec!["api-design".to_string(), "rust".to_string()],
        knowledge: vec![],
        permissions: vec!["write_code".to_string()],
        system_prompt: None,
    }
}

fn make_tester_role() -> Role {
    Role {
        id: "tester".to_string(),
        name: "测试工程师".to_string(),
        description: "单元测试、E2E 测试".to_string(),
        model: "glm5.2".to_string(),
        model_fallback: vec![],
        skills: vec!["testing".to_string()],
        knowledge: vec![],
        permissions: vec!["write_test".to_string()],
        system_prompt: None,
    }
}

// ===== 测试场景 =====

// ============================================================================
// 场景 1：Orchestrator 拆分需求为 DAG
// ============================================================================
#[tokio::test]
async fn test_e2e_orchestrator_plans_dag() {
    // 模拟 Qwen3.7 返回的 DAG JSON
    let dag_json = r#"{
        "workflow_id": "wf_health_check",
        "requirement": "添加一个健康检查接口 /api/health",
        "tasks": [
            {
                "id": "task-1",
                "title": "实现 GET /api/health 接口",
                "assignee": "backend",
                "dependencies": [],
                "estimated_effort": "30min",
                "acceptance_criteria": "GET /api/health 返回 {\"status\":\"ok\"}",
                "description": "实现健康检查接口，返回 JSON 格式的状态信息"
            },
            {
                "id": "task-2",
                "title": "编写健康检查测试",
                "assignee": "tester",
                "dependencies": ["task-1"],
                "estimated_effort": "20min",
                "acceptance_criteria": "测试 test_health_check_returns_ok 通过",
                "description": "编写单元测试验证健康检查接口"
            }
        ]
    }"#;

    let orchestrator = Orchestrator::new(
        make_orchestrator_role(),
        Box::new(MockEngine::new(dag_json)),
        "qwen3.7",
    );

    let dag = orchestrator
        .plan("添加一个健康检查接口 /api/health")
        .await
        .unwrap();

    assert_eq!(dag.workflow_id, "wf_health_check");
    assert_eq!(dag.tasks.len(), 2);
    assert_eq!(dag.tasks[0].id, "task-1");
    assert_eq!(dag.tasks[0].assignee, "backend");
    assert_eq!(dag.tasks[1].id, "task-2");
    assert_eq!(dag.tasks[1].assignee, "tester");
    assert_eq!(dag.tasks[1].dependencies, vec!["task-1"]);
}

// ============================================================================
// 场景 2：Coder 执行后端任务
// ============================================================================
#[tokio::test]
async fn test_e2e_coder_implements_api() {
    let code_response = r#"{
        "files": [
            {
                "path": "src/api/health.rs",
                "content": "use serde_json::json;\n\npub fn health() -> String {\n    json!({\"status\": \"ok\"}).to_string()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    \n    #[test]\n    fn test_health_check_returns_ok() {\n        let response = health();\n        assert!(response.contains(\"ok\"));\n    }\n}",
                "action": "create"
            },
            {
                "path": "src/api/mod.rs",
                "content": "pub mod health;",
                "action": "modify"
            }
        ],
        "summary": "实现了 GET /api/health 接口，返回 {\"status\":\"ok\"}",
        "commit_message": "feat(api): 添加健康检查接口"
    }"#;

    let coder = Coder::new(
        make_backend_role(),
        Box::new(MockEngine::new(code_response)),
        "glm5.2",
    );

    let task = Task::new("task-1", "实现 GET /api/health 接口", "backend")
        .with_description("实现健康检查接口")
        .with_acceptance("GET /api/health 返回 {\"status\":\"ok\"}");

    let context = AgentContext::default();
    let artifact = coder.execute(&task, &context).await.unwrap();

    assert_eq!(artifact.file_count(), 2);
    assert_eq!(artifact.files[0].path, "src/api/health.rs");
    assert_eq!(artifact.files[0].action, FileAction::Create);
    assert!(artifact.files[0].content.contains("health"));
    assert!(artifact.files[0].content.contains("ok"));
    assert_eq!(artifact.files[1].path, "src/api/mod.rs");
    assert_eq!(artifact.files[1].action, FileAction::Modify);
    assert_eq!(artifact.commit_message, "feat(api): 添加健康检查接口");
}

// ============================================================================
// 场景 3：Coder 执行测试任务（带上游产出上下文）
// ============================================================================
#[tokio::test]
async fn test_e2e_coder_writes_tests_with_upstream() {
    let test_response = r#"{
        "files": [
            {
                "path": "tests/api_health_test.rs",
                "content": "use api::health;\n\n#[test]\nfn test_health_check_returns_ok() {\n    let response = health();\n    assert!(response.contains(\"ok\"));\n}",
                "action": "create"
            }
        ],
        "summary": "编写了健康检查接口的单元测试",
        "commit_message": "test(api): 添加健康检查测试"
    }"#;

    let coder = Coder::new(
        make_tester_role(),
        Box::new(MockEngine::new(test_response)),
        "glm5.2",
    );

    let task = Task::new("task-2", "编写健康检查测试", "tester")
        .with_dependencies(vec!["task-1".to_string()])
        .with_acceptance("测试 test_health_check_returns_ok 通过");

    // 构建上下文：包含上游任务 task-1 的产出
    let mut context = AgentContext::default();
    context.upstream_artifacts.push(agent_team::Artifact::summary(
        "后端已实现 health() 函数，返回 {\"status\":\"ok\"}",
    ));

    let artifact = coder.execute(&task, &context).await.unwrap();

    assert_eq!(artifact.file_count(), 1);
    assert_eq!(artifact.files[0].path, "tests/api_health_test.rs");
    assert!(artifact.files[0].content.contains("test_health_check_returns_ok"));
    assert_eq!(artifact.commit_message, "test(api): 添加健康检查测试");
}

// ============================================================================
// 场景 4：Orchestrator 验收任务
// ============================================================================
#[tokio::test]
async fn test_e2e_orchestrator_reviews_task() {
    let review_response = "PASS\n代码实现正确，返回格式符合要求，测试覆盖完整。";
    let orchestrator = Orchestrator::new(
        make_orchestrator_role(),
        Box::new(MockEngine::new(review_response)),
        "qwen3.7",
    );

    let artifact = "pub fn health() -> String { ... }";
    let (passed, feedback) = orchestrator
        .review(
            "实现 GET /api/health 接口",
            "GET /api/health 返回 {\"status\":\"ok\"}",
            artifact,
        )
        .await
        .unwrap();

    assert!(passed);
    assert!(feedback.contains("代码实现正确"));
}

// ============================================================================
// 场景 5：完整双 Agent 协作流程（端到端）
// ============================================================================
#[tokio::test]
async fn test_e2e_full_dual_agent_workflow() {
    // ===== Step 1: Orchestrator 拆分需求 =====
    let dag_json = r#"{
        "workflow_id": "wf_health",
        "requirement": "添加一个健康检查接口 /api/health",
        "tasks": [
            {
                "id": "task-1",
                "title": "实现 GET /api/health 接口",
                "assignee": "backend",
                "dependencies": [],
                "estimated_effort": "30min",
                "acceptance_criteria": "GET /api/health 返回 {\"status\":\"ok\"}",
                "description": "实现健康检查接口"
            },
            {
                "id": "task-2",
                "title": "编写健康检查测试",
                "assignee": "tester",
                "dependencies": ["task-1"],
                "estimated_effort": "20min",
                "acceptance_criteria": "测试通过",
                "description": "编写单元测试"
            }
        ]
    }"#;

    let orchestrator = Orchestrator::new(
        make_orchestrator_role(),
        Box::new(MockEngine::new(dag_json)),
        "qwen3.7",
    );

    let dag = orchestrator
        .plan("添加一个健康检查接口 /api/health")
        .await
        .unwrap();

    // ===== Step 2: 创建调度器 + 上下文池 =====
    let mut scheduler = DagScheduler::new(dag).unwrap();
    let mut context = SharedContext::new();

    // 添加项目规范
    context
        .add_entry(ContextEntry::new(
            "standard/coding",
            "使用 Rust 编写，遵循 rustfmt 规范",
            ContextCategory::Standard,
        ))
        .unwrap();

    assert_eq!(scheduler.progress(), (0, 2));

    // ===== Step 3: 调度 task-1（后端） =====
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    let task1 = match &actions[0] {
        ScheduleAction::AssignTask { task, .. } => task.clone(),
        _ => panic!("期望 AssignTask"),
    };
    assert_eq!(task1.id, "task-1");

    // Coder 执行 task-1
    let code_response = r#"{
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
    let coder = Coder::new(
        make_backend_role(),
        Box::new(MockEngine::new(code_response)),
        "glm5.2",
    );

    let agent_ctx = context.read_for_role("backend", &task1);
    let artifact = coder.execute(&task1, &agent_ctx).await.unwrap();
    assert_eq!(artifact.file_count(), 1);
    assert!(artifact.files[0].content.contains("health"));

    // 写入上下文产出
    context.write_artifact("task-1", agent_team::Artifact::summary(&artifact.summary));

    // 任务完成（Completed → Merged）
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "task-1".to_string(),
            branch: "task/task-1".to_string(),
        })
        .unwrap();
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: "task-1".to_string(),
            snapshot_id: "snap_001".to_string(),
        })
        .unwrap();

    assert_eq!(scheduler.progress(), (1, 2));

    // ===== Step 4: 调度 task-2（测试） =====
    let actions = scheduler.step().unwrap();
    assert_eq!(actions.len(), 1);

    let task2 = match &actions[0] {
        ScheduleAction::AssignTask { task, .. } => task.clone(),
        _ => panic!("期望 AssignTask"),
    };
    assert_eq!(task2.id, "task-2");

    // Coder 执行 task-2（测试工程师角色）
    let test_response = r#"{
        "files": [
            {
                "path": "tests/health_test.rs",
                "content": "use api::health;\n\n#[test]\nfn test_health_check_returns_ok() {\n    assert!(health().contains(\"ok\"));\n}",
                "action": "create"
            }
        ],
        "summary": "编写了健康检查测试",
        "commit_message": "test(api): 添加健康检查测试"
    }"#;
    let tester = Coder::new(
        make_tester_role(),
        Box::new(MockEngine::new(test_response)),
        "glm5.2",
    );

    // 测试工程师应该能看到上游 task-1 的产出
    let agent_ctx = context.read_for_role("tester", &task2);
    assert_eq!(agent_ctx.upstream_artifacts.len(), 1);
    assert!(agent_ctx.upstream_artifacts[0]
        .content
        .contains("健康检查接口"));

    let test_artifact = tester.execute(&task2, &agent_ctx).await.unwrap();
    assert_eq!(test_artifact.file_count(), 1);
    assert!(test_artifact.files[0]
        .content
        .contains("test_health_check_returns_ok"));

    // 任务完成
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "task-2".to_string(),
            branch: "task/task-2".to_string(),
        })
        .unwrap();
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: "task-2".to_string(),
            snapshot_id: "snap_002".to_string(),
        })
        .unwrap();

    // ===== Step 5: 所有任务完成 =====
    assert!(scheduler.is_finished());
    assert_eq!(scheduler.progress(), (2, 2));
    assert_eq!(scheduler.progress_percent(), 100);

    // ===== Step 6: Orchestrator 验收（模拟） =====
    // 注意：这里用新的 Orchestrator 实例（MockEngine 返回 PASS）
    let review_orchestrator = Orchestrator::new(
        make_orchestrator_role(),
        Box::new(MockEngine::new("PASS\n所有任务完成，代码质量良好。")),
        "qwen3.7",
    );

    let (passed, _) = review_orchestrator
        .review(
            "添加健康检查接口",
            "GET /api/health 返回 {\"status\":\"ok\"}",
            &artifact.summary,
        )
        .await
        .unwrap();

    assert!(passed);
}

// ============================================================================
// 场景 6：验证失败 + Coder 重试
// ============================================================================
#[tokio::test]
async fn test_e2e_verify_failure_and_retry() {
    // 创建单任务 DAG
    let mut dag = TaskDag::new("wf_retry", "测试重试");
    dag.add_task(Task::new("task-1", "实现接口", "backend"))
        .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // 分配 task-1
    let _ = scheduler.step().unwrap();

    // task-1 完成
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "task-1".to_string(),
            branch: "task/task-1".to_string(),
        })
        .unwrap();

    // 验证失败
    let actions = scheduler
        .handle_event(ScheduleEvent::TaskVerifyFailed {
            task_id: "task-1".to_string(),
            errors: vec!["缺少错误处理".to_string()],
        })
        .unwrap();

    // 应该返回 RetryTask 动作
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        ScheduleAction::RetryTask { task_id, errors, .. } => {
            assert_eq!(task_id, "task-1");
            assert_eq!(errors, &vec!["缺少错误处理".to_string()]);
        }
        _ => panic!("期望 RetryTask"),
    }

    // Coder 重试（带错误反馈）
    let retry_response = r#"{
        "files": [
            {
                "path": "src/api.rs",
                "content": "pub fn api() -> Result<String, Error> { Ok(...) }",
                "action": "create"
            }
        ],
        "summary": "修复了错误处理",
        "commit_message": "fix(api): 添加错误处理"
    }"#;
    let coder = Coder::new(
        make_backend_role(),
        Box::new(MockEngine::new(retry_response)),
        "glm5.2",
    );

    let task = scheduler.dag().get_task("task-1").unwrap().clone();
    let context = AgentContext::default();
    let errors = vec!["缺少错误处理".to_string()];

    let artifact = coder
        .retry_with_errors(&task, &context, &errors)
        .await
        .unwrap();

    assert_eq!(artifact.commit_message, "fix(api): 添加错误处理");
    assert!(artifact.files[0].content.contains("Result"));

    // 重试任务转回 Running
    let _ = scheduler
        .retry_task("task-1", "backend", "task/task-1")
        .unwrap();

    // 重试后完成
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "task-1".to_string(),
            branch: "task/task-1".to_string(),
        })
        .unwrap();
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: "task-1".to_string(),
            snapshot_id: "snap_001".to_string(),
        })
        .unwrap();

    assert!(scheduler.is_finished());
}

// ============================================================================
// 场景 7：Orchestrator 验收失败
// ============================================================================
#[tokio::test]
async fn test_e2e_review_failure() {
    let review_response = "FAIL\n接口返回格式不正确，期望 {\"status\":\"ok\"}，实际返回了纯文本。";
    let orchestrator = Orchestrator::new(
        make_orchestrator_role(),
        Box::new(MockEngine::new(review_response)),
        "qwen3.7",
    );

    let (passed, feedback) = orchestrator
        .review(
            "实现 GET /api/health 接口",
            "GET /api/health 返回 {\"status\":\"ok\"}",
            "pub fn health() -> String { \"ok\" }",
        )
        .await
        .unwrap();

    assert!(!passed);
    assert!(feedback.contains("返回格式不正确"));
}

// ============================================================================
// 场景 8：上下文传递（Orchestrator → Coder）
// ============================================================================
#[tokio::test]
async fn test_e2e_context_propagation() {
    let mut context = SharedContext::new();

    // 项目规范
    context
        .add_entry(ContextEntry::new(
            "standard/api",
            "所有 API 必须返回 JSON 格式",
            ContextCategory::Standard,
        ))
        .unwrap();

    // 后端文档（只对 backend 可见）
    context.add_entry(
        ContextEntry::new("docs/api.md", "# API 设计规范", ContextCategory::Documentation)
            .visible_to(vec!["backend".to_string()]),
    ).unwrap();

    let task = Task::new("task-1", "实现接口", "backend");

    // backend 角色应该看到规范 + 后端文档
    let backend_ctx = context.read_for_role("backend", &task);
    assert!(!backend_ctx.standards.is_empty());
    assert!(backend_ctx.docs.iter().any(|d| d.key == "docs/api.md"));

    // tester 角色应该只看到规范
    let tester_ctx = context.read_for_role("tester", &task);
    assert!(!tester_ctx.standards.is_empty());
    assert!(tester_ctx.docs.is_empty());
}

// ============================================================================
// 场景 9：多文件代码生成
// ============================================================================
#[tokio::test]
async fn test_e2e_multi_file_generation() {
    let response = r#"{
        "files": [
            {"path": "src/api/health.rs", "content": "pub fn health() -> String { ... }", "action": "create"},
            {"path": "src/api/mod.rs", "content": "pub mod health;", "action": "modify"},
            {"path": "src/main.rs", "content": "mod api; fn main() { println!(\"{}\", api::health::health()); }", "action": "modify"},
            {"path": "src/old_api.rs", "content": "", "action": "delete"}
        ],
        "summary": "添加健康检查接口并清理旧代码",
        "commit_message": "feat(api): 添加健康检查接口 + 删除旧代码"
    }"#;

    let coder = Coder::new(
        make_backend_role(),
        Box::new(MockEngine::new(response)),
        "glm5.2",
    );

    let task = Task::new("task-1", "实现接口", "backend");
    let context = AgentContext::default();

    let artifact = coder.execute(&task, &context).await.unwrap();

    assert_eq!(artifact.file_count(), 4);
    assert_eq!(artifact.files[0].action, FileAction::Create);
    assert_eq!(artifact.files[1].action, FileAction::Modify);
    assert_eq!(artifact.files[2].action, FileAction::Modify);
    assert_eq!(artifact.files[3].action, FileAction::Delete);
}

// ============================================================================
// 场景 10：DAG 调度 + 双 Agent 协作完整流程（含状态验证）
// ============================================================================
#[tokio::test]
async fn test_e2e_dag_state_transitions() {
    let mut dag = TaskDag::new("wf_state", "状态转移测试");
    dag.add_task(Task::new("t1", "任务1", "backend")).unwrap();
    dag.add_task(
        Task::new("t2", "任务2", "tester").with_dependencies(vec!["t1".to_string()]),
    )
    .unwrap();

    let mut scheduler = DagScheduler::new(dag).unwrap();

    // 初始状态：所有任务 Pending
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Pending)
    ));
    assert!(matches!(
        scheduler.task_state("t2"),
        Some(TaskState::Pending)
    ));

    // step 1: 分配 t1
    let _ = scheduler.step().unwrap();
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Running { .. })
    ));
    assert!(matches!(
        scheduler.task_state("t2"),
        Some(TaskState::Pending)
    ));

    // t1 完成 → 验证中
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "t1".to_string(),
            branch: "task/t1".to_string(),
        })
        .unwrap();
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Verifying { .. })
    ));

    // t1 验证通过 → 合并
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: "t1".to_string(),
            snapshot_id: "snap1".to_string(),
        })
        .unwrap();
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Merged { .. })
    ));

    // step 2: 分配 t2
    let _ = scheduler.step().unwrap();
    assert!(matches!(
        scheduler.task_state("t2"),
        Some(TaskState::Running { .. })
    ));

    // t2 完成 → 合并
    scheduler
        .handle_event(ScheduleEvent::TaskCompleted {
            task_id: "t2".to_string(),
            branch: "task/t2".to_string(),
        })
        .unwrap();
    scheduler
        .handle_event(ScheduleEvent::TaskMerged {
            task_id: "t2".to_string(),
            snapshot_id: "snap2".to_string(),
        })
        .unwrap();

    // 所有任务完成
    assert!(scheduler.is_finished());
    assert!(matches!(
        scheduler.task_state("t1"),
        Some(TaskState::Merged { .. })
    ));
    assert!(matches!(
        scheduler.task_state("t2"),
        Some(TaskState::Merged { .. })
    ));
}
