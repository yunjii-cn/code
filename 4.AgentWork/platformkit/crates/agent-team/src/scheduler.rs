// DAG 调度引擎（DagScheduler）（W8 M3.2 D2）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.3
//
// DagScheduler 是 DAG 调度的核心，负责：
//   1. 找出所有依赖已满足的待执行任务（ready 任务）
//   2. 维护任务状态机（TaskStates）
//   3. 推进 DAG 执行进度
//
// 调度循环（run / step）：
//   loop {
//     ready = scheduler.find_ready_tasks();
//     for task in ready { scheduler.assign(task); }
//     // 等待外部事件（任务完成/失败）
//     event = event_rx.recv();
//     scheduler.handle_event(event);
//     if scheduler.is_finished() { break; }
//   }
//
// 设计原则：
//   - Scheduler 是纯逻辑（不直接调用 Agent / TimeFlow）
//   - 外部系统通过调用 scheduler 的方法更新状态
//   - 便于单元测试（无需 mock 整个 TimeFlow）

use crate::dag::{Task, TaskDag, TaskId};
use crate::error::{Result, TeamError};
use crate::task_state::{TaskState, TaskStates, MAX_RETRIES};
use std::collections::HashSet;

/// 调度事件（外部系统通知 scheduler 任务状态变化）
#[derive(Debug, Clone)]
pub enum ScheduleEvent {
    /// 任务执行完成（Agent 已完成执行，等待验证）
    TaskCompleted {
        /// 任务 ID
        task_id: TaskId,
        /// 任务所在分支
        branch: String,
    },
    /// 任务执行失败
    TaskFailed {
        /// 任务 ID
        task_id: TaskId,
        /// 错误信息
        error: String,
    },
    /// 任务验证通过（已合并到 main）
    TaskMerged {
        /// 任务 ID
        task_id: TaskId,
        /// 合并后的快照 ID
        snapshot_id: String,
    },
    /// 任务验证失败
    TaskVerifyFailed {
        /// 任务 ID
        task_id: TaskId,
        /// 验证错误信息
        errors: Vec<String>,
    },
}

/// 调度决策（scheduler 给外部系统的指令）
#[derive(Debug, Clone)]
pub enum ScheduleAction {
    /// 分配任务给 Agent（在指定分支执行）
    AssignTask {
        /// 任务
        task: Task,
        /// 分支名（task/<task_id>）
        branch: String,
        /// 执行任务的 Agent 角色
        assignee: String,
    },
    /// 重试任务（喂回 Agent，附带错误信息）
    RetryTask {
        /// 任务 ID
        task_id: TaskId,
        /// 分支名
        branch: String,
        /// 错误信息（供 Agent 参考）
        errors: Vec<String>,
    },
    /// 拒绝任务（重试耗尽）
    RejectTask {
        /// 任务 ID
        task_id: TaskId,
        /// 拒绝原因
        reason: String,
    },
}

/// DAG 调度引擎
#[derive(Debug, Clone)]
pub struct DagScheduler {
    /// 任务图
    pub dag: TaskDag,
    /// 任务状态集合
    pub task_states: TaskStates,
}

impl DagScheduler {
    /// 创建调度器（自动初始化所有任务为 Pending）
    pub fn new(dag: TaskDag) -> Result<Self> {
        // 创建时校验 DAG
        dag.validate()?;
        let task_ids: Vec<TaskId> = dag.tasks.iter().map(|t| t.id.clone()).collect();
        let task_states = TaskStates::from_task_ids(task_ids);
        Ok(Self { dag, task_states })
    }

    /// 从 JSON 创建调度器
    pub fn from_json(json: &str) -> Result<Self> {
        let dag = TaskDag::from_json(json)?;
        Self::new(dag)
    }

    /// 找出所有 ready 任务（Pending 且依赖全部 Merged）
    ///
    /// ready 任务是 Pending 状态且所有依赖都已 Merged 的任务。
    /// 返回的任务列表按 DAG 中的顺序排列。
    pub fn find_ready_tasks(&self) -> Vec<&Task> {
        let completed: HashSet<TaskId> = self.task_states.completed_tasks().into_iter().collect();
        self.dag
            .tasks
            .iter()
            .filter(|t| {
                // 必须是 Pending 状态
                matches!(
                    self.task_states.get(&t.id),
                    Some(TaskState::Pending) | None
                )
                // 所有依赖必须 Merged
                && t.dependencies.iter().all(|dep| completed.contains(dep))
            })
            .collect()
    }

    /// 找出所有 ready 任务 ID
    pub fn find_ready_task_ids(&self) -> Vec<TaskId> {
        self.find_ready_tasks().iter().map(|t| t.id.clone()).collect()
    }

    /// 分配任务（将任务从 Pending → Running）
    ///
    /// 返回 ScheduleAction::AssignTask，外部系统据此调用 Agent 执行。
    pub fn assign_task(&mut self, task_id: &str, agent: &str) -> Result<ScheduleAction> {
        let task = self
            .dag
            .get_task(task_id)
            .ok_or_else(|| TeamError::Config(format!("任务 '{task_id}' 不存在")))?
            .clone();

        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Pending) | None => {
                let branch = format!("task/{task_id}");
                self.task_states
                    .set(task_id, TaskState::running(agent, &branch));
                Ok(ScheduleAction::AssignTask {
                    task,
                    branch,
                    assignee: agent.to_string(),
                })
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法分配（必须是 Pending）",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 处理调度事件
    ///
    /// 返回需要执行的动作列表（可能有多个，例如重试或拒绝）。
    pub fn handle_event(&mut self, event: ScheduleEvent) -> Result<Vec<ScheduleAction>> {
        match event {
            ScheduleEvent::TaskCompleted { task_id, branch } => {
                self.handle_task_completed(&task_id, &branch)
            }
            ScheduleEvent::TaskFailed { task_id, error } => {
                self.handle_task_failed(&task_id, &error)
            }
            ScheduleEvent::TaskMerged {
                task_id,
                snapshot_id,
            } => self.handle_task_merged(&task_id, &snapshot_id),
            ScheduleEvent::TaskVerifyFailed { task_id, errors } => {
                self.handle_verify_failed(&task_id, &errors)
            }
        }
    }

    /// 处理任务执行完成（Running → Verifying）
    fn handle_task_completed(&mut self, task_id: &str, branch: &str) -> Result<Vec<ScheduleAction>> {
        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Running { .. }) => {
                self.task_states
                    .set(task_id, TaskState::verifying(branch));
                Ok(vec![])
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法转为 Verifying（必须是 Running）",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 处理任务执行失败（Running → Failed 或 Rejected）
    fn handle_task_failed(&mut self, task_id: &str, error: &str) -> Result<Vec<ScheduleAction>> {
        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Running { .. }) => {
                // Running 状态没有 retry_count，使用 0 作为初始重试次数
                // 实际重试次数由 Failed 状态追踪
                let new_retry_count = 0;
                self.task_states
                    .set(task_id, TaskState::failed(error, new_retry_count));
                Ok(vec![])
            }
            Some(TaskState::Failed { retry_count, .. }) => {
                // 已经是 Failed，增加 retry_count
                let new_retry_count = retry_count + 1;
                if new_retry_count >= MAX_RETRIES {
                    self.task_states
                        .set(task_id, TaskState::rejected("重试耗尽"));
                    Ok(vec![ScheduleAction::RejectTask {
                        task_id: task_id.to_string(),
                        reason: format!("重试耗尽（{MAX_RETRIES}次）"),
                    }])
                } else {
                    self.task_states
                        .set(task_id, TaskState::failed(error, new_retry_count));
                    Ok(vec![])
                }
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法处理失败事件",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 处理任务验证通过（Verifying → Merged）
    fn handle_task_merged(
        &mut self,
        task_id: &str,
        snapshot_id: &str,
    ) -> Result<Vec<ScheduleAction>> {
        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Verifying { .. }) => {
                self.task_states
                    .set(task_id, TaskState::merged(snapshot_id));
                Ok(vec![])
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法合并（必须是 Verifying）",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 处理验证失败（Verifying → Failed，可能触发重试）
    fn handle_verify_failed(
        &mut self,
        task_id: &str,
        errors: &[String],
    ) -> Result<Vec<ScheduleAction>> {
        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Verifying { branch }) => {
                let branch = branch.clone();
                // Verifying → Failed（retry_count = 0）
                self.task_states
                    .set(task_id, TaskState::failed(errors.join("; "), 0));

                // 立即检查是否可重试
                let current_state = self.task_states.get(task_id).unwrap();
                if current_state.can_retry() {
                    // 增加重试计数并转回 Running
                    self.task_states.set(
                        task_id,
                        TaskState::failed(errors.join("; "), 1),
                    );
                    // 实际重试需要外部系统重新分配
                    Ok(vec![ScheduleAction::RetryTask {
                        task_id: task_id.to_string(),
                        branch,
                        errors: errors.to_vec(),
                    }])
                } else {
                    // 重试耗尽，拒绝
                    self.task_states
                        .set(task_id, TaskState::rejected("验证失败超过 3 次"));
                    Ok(vec![ScheduleAction::RejectTask {
                        task_id: task_id.to_string(),
                        reason: "验证失败超过 3 次".to_string(),
                    }])
                }
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法处理验证失败（必须是 Verifying）",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 重试任务（Failed → Running）
    ///
    /// 外部系统收到 RetryTask 动作后，调用此方法将任务转回 Running。
    pub fn retry_task(
        &mut self,
        task_id: &str,
        agent: &str,
        branch: &str,
    ) -> Result<ScheduleAction> {
        let state = self.task_states.get(task_id);
        match state {
            Some(TaskState::Failed { retry_count, .. }) if *retry_count < MAX_RETRIES => {
                self.task_states
                    .set(task_id, TaskState::running(agent, branch));
                Ok(ScheduleAction::AssignTask {
                    task: self
                        .dag
                        .get_task(task_id)
                        .ok_or_else(|| TeamError::Config(format!("任务 '{task_id}' 不存在")))?
                        .clone(),
                    branch: branch.to_string(),
                    assignee: agent.to_string(),
                })
            }
            _ => Err(TeamError::Config(format!(
                "任务 '{}' 状态为 {:?}，无法重试",
                task_id,
                state.map(|s| s.to_string())
            ))),
        }
    }

    /// 单步调度（找出 ready 任务并返回分配动作）
    ///
    /// 这是调度循环的核心：找出所有 ready 任务，生成 AssignTask 动作。
    /// 外部系统执行这些动作（调用 Agent），然后通过 handle_event 通知结果。
    pub fn step(&mut self) -> Result<Vec<ScheduleAction>> {
        let ready_ids = self.find_ready_task_ids();
        let mut actions = Vec::new();
        for task_id in ready_ids {
            let assignee = self
                .dag
                .get_task(&task_id)
                .map(|t| t.assignee.clone())
                .unwrap_or_default();
            if let Ok(action) = self.assign_task(&task_id, &assignee) {
                actions.push(action);
            }
        }
        Ok(actions)
    }

    /// 是否所有任务都处于终态（Merged 或 Rejected）
    pub fn is_finished(&self) -> bool {
        self.task_states.all_terminal()
    }

    /// 调度进度（已完成 / 总数）
    pub fn progress(&self) -> (usize, usize) {
        let total = self.dag.task_count();
        let completed = self.task_states.completed_tasks().len();
        (completed, total)
    }

    /// 完成百分比（0-100）
    pub fn progress_percent(&self) -> u32 {
        let (completed, total) = self.progress();
        if total == 0 {
            return 0;
        }
        ((completed as f64 / total as f64) * 100.0) as u32
    }

    /// 获取任务状态
    pub fn task_state(&self, task_id: &str) -> Option<&TaskState> {
        self.task_states.get(task_id)
    }

    /// 获取所有失败任务 ID
    pub fn failed_tasks(&self) -> Vec<TaskId> {
        self.task_states.failed_tasks()
    }

    /// 获取所有运行中任务 ID
    pub fn running_tasks(&self) -> Vec<TaskId> {
        self.task_states.running_tasks()
    }

    /// 获取 DAG
    pub fn dag(&self) -> &TaskDag {
        &self.dag
    }

    /// 获取任务状态集合
    pub fn task_states(&self) -> &TaskStates {
        &self.task_states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_linear_dag() -> TaskDag {
        let mut dag = TaskDag::new("wf_linear", "线性依赖");
        dag.add_task(Task::new("t1", "任务1", "backend")).unwrap();
        dag.add_task(
            Task::new("t2", "任务2", "frontend").with_dependencies(vec!["t1".to_string()]),
        )
        .unwrap();
        dag.add_task(
            Task::new("t3", "任务3", "tester").with_dependencies(vec!["t2".to_string()]),
        )
        .unwrap();
        dag
    }

    fn make_parallel_dag() -> TaskDag {
        let mut dag = TaskDag::new("wf_parallel", "并行任务");
        dag.add_task(Task::new("t1", "后端", "backend")).unwrap();
        dag.add_task(Task::new("t2", "前端", "frontend")).unwrap();
        dag.add_task(
            Task::new("t3", "测试", "tester")
                .with_dependencies(vec!["t1".to_string(), "t2".to_string()]),
        )
        .unwrap();
        dag
    }

    #[test]
    fn test_scheduler_creation() {
        let dag = make_linear_dag();
        let scheduler = DagScheduler::new(dag).unwrap();
        assert_eq!(scheduler.dag.task_count(), 3);
        assert!(!scheduler.is_finished());
        assert_eq!(scheduler.progress(), (0, 3));
        assert_eq!(scheduler.progress_percent(), 0);
    }

    #[test]
    fn test_scheduler_invalid_dag() {
        // 空 DAG 应该报错
        let dag = TaskDag::new("empty", "");
        assert!(DagScheduler::new(dag).is_err());
    }

    #[test]
    fn test_find_ready_tasks_initial() {
        let dag = make_linear_dag();
        let scheduler = DagScheduler::new(dag).unwrap();
        let ready = scheduler.find_ready_task_ids();
        // 只有 t1 无依赖
        assert_eq!(ready, vec!["t1"]);
    }

    #[test]
    fn test_find_ready_tasks_parallel() {
        let dag = make_parallel_dag();
        let scheduler = DagScheduler::new(dag).unwrap();
        let ready = scheduler.find_ready_task_ids();
        // t1 和 t2 都无依赖
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"t1".to_string()));
        assert!(ready.contains(&"t2".to_string()));
    }

    #[test]
    fn test_step_assigns_ready_tasks() {
        let dag = make_parallel_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();
        let actions = scheduler.step().unwrap();

        // 应该分配 t1 和 t2
        assert_eq!(actions.len(), 2);
        for action in &actions {
            match action {
                ScheduleAction::AssignTask { task, branch, assignee } => {
                    assert!(branch.starts_with("task/"));
                    assert_eq!(branch.as_str(), format!("task/{}", task.id));
                    assert_eq!(task.assignee, *assignee);
                }
                _ => panic!("期望 AssignTask 动作"),
            }
        }

        // t1 和 t2 应该是 Running 状态
        assert!(matches!(
            scheduler.task_state("t1"),
            Some(TaskState::Running { .. })
        ));
        assert!(matches!(
            scheduler.task_state("t2"),
            Some(TaskState::Running { .. })
        ));
        // t3 应该还是 Pending
        assert!(matches!(
            scheduler.task_state("t3"),
            Some(TaskState::Pending)
        ));
    }

    #[test]
    fn test_step_after_completion() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 第一次 step：分配 t1
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            match &actions[0] {
                ScheduleAction::AssignTask { task, .. } => task.id.clone(),
                _ => panic!(),
            },
            "t1"
        );

        // t1 完成 → 验证 → 合并
        scheduler
            .handle_event(ScheduleEvent::TaskCompleted {
                task_id: "t1".to_string(),
                branch: "task/t1".to_string(),
            })
            .unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskMerged {
                task_id: "t1".to_string(),
                snapshot_id: "snap_001".to_string(),
            })
            .unwrap();

        // 第二次 step：分配 t2
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            match &actions[0] {
                ScheduleAction::AssignTask { task, .. } => task.id.clone(),
                _ => panic!(),
            },
            "t2"
        );
    }

    #[test]
    fn test_full_workflow_linear() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 执行 t1
        let _ = scheduler.step().unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskCompleted {
                task_id: "t1".to_string(),
                branch: "task/t1".to_string(),
            })
            .unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskMerged {
                task_id: "t1".to_string(),
                snapshot_id: "snap1".to_string(),
            })
            .unwrap();

        // 执行 t2
        let _ = scheduler.step().unwrap();
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

        // 执行 t3
        let _ = scheduler.step().unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskCompleted {
                task_id: "t3".to_string(),
                branch: "task/t3".to_string(),
            })
            .unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskMerged {
                task_id: "t3".to_string(),
                snapshot_id: "snap3".to_string(),
            })
            .unwrap();

        // 所有任务完成
        assert!(scheduler.is_finished());
        assert_eq!(scheduler.progress(), (3, 3));
        assert_eq!(scheduler.progress_percent(), 100);
    }

    #[test]
    fn test_task_failure_and_retry() {
        let dag = make_linear_dag();
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

        // 应该没有动作（首次失败，retry_count=0）
        assert!(actions.is_empty());

        // t1 应该是 Failed 状态
        assert!(matches!(
            scheduler.task_state("t1"),
            Some(TaskState::Failed { .. })
        ));

        // 可以重试
        let state = scheduler.task_state("t1").unwrap();
        assert!(state.can_retry());
    }

    #[test]
    fn test_verify_failed_triggers_retry() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 分配 t1
        let _ = scheduler.step().unwrap();

        // t1 完成
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
        match &actions[0] {
            ScheduleAction::RetryTask {
                task_id,
                branch,
                errors,
            } => {
                assert_eq!(task_id, "t1");
                assert_eq!(branch, "task/t1");
                assert_eq!(errors, &vec!["测试失败".to_string()]);
            }
            _ => panic!("期望 RetryTask 动作"),
        }
    }

    #[test]
    fn test_assign_task_invalid_state() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 分配 t1
        let _ = scheduler.step().unwrap();

        // 再次分配 t1 应该失败（已经是 Running）
        let result = scheduler.assign_task("t1", "backend");
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_nonexistent_task() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        let result = scheduler.assign_task("nonexistent", "backend");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_event_invalid_state() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // t1 还是 Pending，直接发 TaskCompleted 应该失败
        let result = scheduler.handle_event(ScheduleEvent::TaskCompleted {
            task_id: "t1".to_string(),
            branch: "task/t1".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_progress_tracking() {
        let dag = make_parallel_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        assert_eq!(scheduler.progress(), (0, 3));

        // 分配 t1, t2
        let _ = scheduler.step().unwrap();
        assert_eq!(scheduler.progress(), (0, 3));

        // t1 完成
        scheduler
            .handle_event(ScheduleEvent::TaskCompleted {
                task_id: "t1".to_string(),
                branch: "task/t1".to_string(),
            })
            .unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskMerged {
                task_id: "t1".to_string(),
                snapshot_id: "s1".to_string(),
            })
            .unwrap();
        assert_eq!(scheduler.progress(), (1, 3));

        // t2 完成
        scheduler
            .handle_event(ScheduleEvent::TaskCompleted {
                task_id: "t2".to_string(),
                branch: "task/t2".to_string(),
            })
            .unwrap();
        scheduler
            .handle_event(ScheduleEvent::TaskMerged {
                task_id: "t2".to_string(),
                snapshot_id: "s2".to_string(),
            })
            .unwrap();
        assert_eq!(scheduler.progress(), (2, 3));
        assert_eq!(scheduler.progress_percent(), 66);
    }

    #[test]
    fn test_running_and_failed_tasks() {
        let dag = make_parallel_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 分配 t1, t2
        let _ = scheduler.step().unwrap();

        let running = scheduler.running_tasks();
        assert_eq!(running.len(), 2);

        // t1 失败
        scheduler
            .handle_event(ScheduleEvent::TaskFailed {
                task_id: "t1".to_string(),
                error: "err".to_string(),
            })
            .unwrap();

        let failed = scheduler.failed_tasks();
        assert_eq!(failed.len(), 1);
        assert!(failed.contains(&"t1".to_string()));

        let running = scheduler.running_tasks();
        assert_eq!(running.len(), 1);
        assert!(running.contains(&"t2".to_string()));
    }

    #[test]
    fn test_from_json() {
        let dag = make_linear_dag();
        let json = dag.to_json().unwrap();
        let scheduler = DagScheduler::from_json(&json).unwrap();
        assert_eq!(scheduler.dag.task_count(), 3);
    }

    #[test]
    fn test_retry_task() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 分配 t1
        let _ = scheduler.step().unwrap();

        // t1 失败
        scheduler
            .handle_event(ScheduleEvent::TaskFailed {
                task_id: "t1".to_string(),
                error: "err".to_string(),
            })
            .unwrap();

        // 重试 t1
        let action = scheduler.retry_task("t1", "backend", "task/t1").unwrap();
        match action {
            ScheduleAction::AssignTask { task, branch, .. } => {
                assert_eq!(task.id, "t1");
                assert_eq!(branch, "task/t1");
            }
            _ => panic!("期望 AssignTask 动作"),
        }

        // t1 应该回到 Running
        assert!(matches!(
            scheduler.task_state("t1"),
            Some(TaskState::Running { .. })
        ));
    }

    #[test]
    fn test_step_idempotent_when_no_ready() {
        let dag = make_linear_dag();
        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 分配 t1
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 1);

        // 再次 step，t2 依赖 t1 未完成，应该没有 ready 任务
        let actions = scheduler.step().unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_scheduler_with_diamond_dag() {
        // 菱形 DAG: t1 → t2, t1 → t3, t2 → t4, t3 → t4
        let mut dag = TaskDag::new("diamond", "菱形依赖");
        dag.add_task(Task::new("t1", "基础", "backend")).unwrap();
        dag.add_task(
            Task::new("t2", "分支A", "frontend").with_dependencies(vec!["t1".to_string()]),
        )
        .unwrap();
        dag.add_task(
            Task::new("t3", "分支B", "db").with_dependencies(vec!["t1".to_string()]),
        )
        .unwrap();
        dag.add_task(
            Task::new("t4", "集成", "tester")
                .with_dependencies(vec!["t2".to_string(), "t3".to_string()]),
        )
        .unwrap();

        let mut scheduler = DagScheduler::new(dag).unwrap();

        // 辅助闭包：完成任务（Completed → Merged）
        let complete = |s: &mut DagScheduler, task_id: &str, snap: &str| {
            s.handle_event(ScheduleEvent::TaskCompleted {
                task_id: task_id.to_string(),
                branch: format!("task/{task_id}"),
            })
            .unwrap();
            s.handle_event(ScheduleEvent::TaskMerged {
                task_id: task_id.to_string(),
                snapshot_id: snap.to_string(),
            })
            .unwrap();
        };

        // step 1: 分配 t1
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 1);

        // t1 完成
        complete(&mut scheduler, "t1", "s1");

        // step 2: 分配 t2 和 t3（并行）
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 2);

        // t2, t3 完成
        complete(&mut scheduler, "t2", "s2");
        complete(&mut scheduler, "t3", "s3");

        // step 3: 分配 t4
        let actions = scheduler.step().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            match &actions[0] {
                ScheduleAction::AssignTask { task, .. } => task.id.clone(),
                _ => panic!(),
            },
            "t4"
        );

        // t4 完成
        complete(&mut scheduler, "t4", "s4");

        assert!(scheduler.is_finished());
    }
}
