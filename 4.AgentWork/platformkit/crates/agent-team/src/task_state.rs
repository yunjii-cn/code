// 任务状态机（TaskState）（W8 M3.2 D3）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.3
//
// 任务在 DAG 调度中的生命周期：
//   Pending → Running → Verifying → Merged
//                       ↓
//                     Failed (可重试，最多 3 次)
//                       ↓ (重试耗尽)
//                     Rejected
//
// 状态转移规则：
//   Pending   → Running     (调度器分配任务)
//   Running   → Verifying   (Agent 完成执行，进入验证)
//   Verifying → Merged      (验证通过，合并到 main)
//   Verifying → Failed      (验证失败)
//   Failed    → Running     (重试，retry_count < MAX_RETRIES)
//   Failed    → Rejected    (重试耗尽)
//   Running   → Failed      (Agent 执行异常)

use crate::dag::TaskId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 最大重试次数
pub const MAX_RETRIES: u32 = 3;

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TaskState {
    /// 待执行（依赖未满足或等待调度）
    Pending,
    /// 执行中
    Running {
        /// 执行任务的 Agent 角色 ID
        agent: String,
        /// 任务所在分支（分支隔离）
        branch: String,
        /// 开始时间
        started_at: DateTime<Utc>,
    },
    /// 验证中（Agent 完成执行，等待验证护栏检查）
    Verifying {
        /// 待验证的分支
        branch: String,
    },
    /// 已合并（验证通过，合并到 main）
    Merged {
        /// 合并后的快照 ID
        snapshot_id: String,
        /// 合并时间
        merged_at: DateTime<Utc>,
    },
    /// 失败（可重试）
    Failed {
        /// 错误信息
        error: String,
        /// 已重试次数
        retry_count: u32,
        /// 最近失败时间
        failed_at: DateTime<Utc>,
    },
    /// 拒绝（重试耗尽，无法修复）
    Rejected {
        /// 拒绝原因
        reason: String,
        /// 拒绝时间
        rejected_at: DateTime<Utc>,
    },
}

impl TaskState {
    /// 创建 Pending 状态
    pub fn pending() -> Self {
        Self::Pending
    }

    /// 创建 Running 状态
    pub fn running(agent: impl Into<String>, branch: impl Into<String>) -> Self {
        Self::Running {
            agent: agent.into(),
            branch: branch.into(),
            started_at: Utc::now(),
        }
    }

    /// 创建 Verifying 状态
    pub fn verifying(branch: impl Into<String>) -> Self {
        Self::Verifying {
            branch: branch.into(),
        }
    }

    /// 创建 Merged 状态
    pub fn merged(snapshot_id: impl Into<String>) -> Self {
        Self::Merged {
            snapshot_id: snapshot_id.into(),
            merged_at: Utc::now(),
        }
    }

    /// 创建 Failed 状态
    pub fn failed(error: impl Into<String>, retry_count: u32) -> Self {
        Self::Failed {
            error: error.into(),
            retry_count,
            failed_at: Utc::now(),
        }
    }

    /// 创建 Rejected 状态
    pub fn rejected(reason: impl Into<String>) -> Self {
        Self::Rejected {
            reason: reason.into(),
            rejected_at: Utc::now(),
        }
    }

    /// 是否处于终态（不会再变化）
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Merged { .. } | Self::Rejected { .. })
    }

    /// 是否可重试
    pub fn can_retry(&self) -> bool {
        match self {
            Self::Failed { retry_count, .. } => *retry_count < MAX_RETRIES,
            _ => false,
        }
    }

    /// 获取当前分支（Running / Verifying 状态有分支）
    pub fn branch(&self) -> Option<&str> {
        match self {
            Self::Running { branch, .. } | Self::Verifying { branch } => Some(branch),
            _ => None,
        }
    }

    /// 获取 Agent 角色（仅 Running 状态）
    pub fn agent(&self) -> Option<&str> {
        match self {
            Self::Running { agent, .. } => Some(agent),
            _ => None,
        }
    }

    /// 是否已完成（Merged 视为完成，用于 DAG 推进）
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Merged { .. })
    }

    /// 是否失败（Failed 或 Rejected）
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::Rejected { .. })
    }
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running { agent, branch, .. } => {
                write!(f, "Running(agent={agent}, branch={branch})")
            }
            Self::Verifying { branch } => write!(f, "Verifying(branch={branch})"),
            Self::Merged { snapshot_id, .. } => write!(f, "Merged({snapshot_id})"),
            Self::Failed { retry_count, .. } => write!(f, "Failed(retries={retry_count})"),
            Self::Rejected { .. } => write!(f, "Rejected"),
        }
    }
}

/// 任务状态集合（TaskId → TaskState）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStates {
    /// 任务状态映射
    pub states: HashMap<TaskId, TaskState>,
}

impl TaskStates {
    /// 创建空集合
    pub fn new() -> Self {
        Self::default()
    }

    /// 初始化所有任务为 Pending
    pub fn from_task_ids(task_ids: impl IntoIterator<Item = TaskId>) -> Self {
        let mut states = HashMap::new();
        for id in task_ids {
            states.insert(id, TaskState::Pending);
        }
        Self { states }
    }

    /// 获取任务状态
    pub fn get(&self, task_id: &str) -> Option<&TaskState> {
        self.states.get(task_id)
    }

    /// 设置任务状态
    pub fn set(&mut self, task_id: impl Into<TaskId>, state: TaskState) {
        self.states.insert(task_id.into(), state);
    }

    /// 获取所有已完成的任务 ID
    pub fn completed_tasks(&self) -> Vec<TaskId> {
        self.states
            .iter()
            .filter(|(_, s)| s.is_completed())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 获取所有处于指定状态的任务 ID
    ///
    /// 按状态类型（Pending/Running/Verifying/Merged/Failed/Rejected）匹配。
    pub fn tasks_in_state(&self, target: &TaskState) -> Vec<TaskId> {
        // 按 Display 字符串比较状态类型（忽略具体字段值）
        let target_str = target.to_string();
        let target_kind = target_str.split('(').next().unwrap_or(&target_str);
        self.states
            .iter()
            .filter(|(_, s)| {
                let s_str = s.to_string();
                let s_kind = s_str.split('(').next().unwrap_or(&s_str);
                s_kind == target_kind
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 获取所有 Running 任务的 ID
    pub fn running_tasks(&self) -> Vec<TaskId> {
        self.states
            .iter()
            .filter(|(_, s)| matches!(s, TaskState::Running { .. }))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 获取所有 Failed 任务的 ID
    pub fn failed_tasks(&self) -> Vec<TaskId> {
        self.states
            .iter()
            .filter(|(_, s)| matches!(s, TaskState::Failed { .. }))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// 是否所有任务都处于终态
    pub fn all_terminal(&self) -> bool {
        !self.states.is_empty() && self.states.values().all(|s| s.is_terminal())
    }

    /// 任务总数
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let pending = TaskState::pending();
        assert!(matches!(pending, TaskState::Pending));

        let running = TaskState::running("backend", "task/t1");
        assert!(matches!(running, TaskState::Running { .. }));
        assert_eq!(running.agent(), Some("backend"));
        assert_eq!(running.branch(), Some("task/t1"));

        let verifying = TaskState::verifying("task/t1");
        assert!(matches!(verifying, TaskState::Verifying { .. }));
        assert_eq!(verifying.branch(), Some("task/t1"));

        let merged = TaskState::merged("snap_001");
        assert!(matches!(merged, TaskState::Merged { .. }));

        let failed = TaskState::failed("编译错误", 1);
        assert!(matches!(failed, TaskState::Failed { .. }));

        let rejected = TaskState::rejected("重试耗尽");
        assert!(matches!(rejected, TaskState::Rejected { .. }));
    }

    #[test]
    fn test_is_terminal() {
        assert!(!TaskState::pending().is_terminal());
        assert!(!TaskState::running("a", "b").is_terminal());
        assert!(!TaskState::verifying("b").is_terminal());
        assert!(TaskState::merged("s").is_terminal());
        assert!(!TaskState::failed("e", 0).is_terminal());
        assert!(TaskState::rejected("r").is_terminal());
    }

    #[test]
    fn test_can_retry() {
        // Pending 不能重试
        assert!(!TaskState::pending().can_retry());

        // Failed 且 retry_count < MAX_RETRIES 可重试
        assert!(TaskState::failed("err", 0).can_retry());
        assert!(TaskState::failed("err", 1).can_retry());
        assert!(TaskState::failed("err", 2).can_retry());

        // retry_count == MAX_RETRIES 不能重试
        assert!(!TaskState::failed("err", MAX_RETRIES).can_retry());
        assert!(!TaskState::failed("err", MAX_RETRIES + 1).can_retry());

        // Rejected 不能重试
        assert!(!TaskState::rejected("r").can_retry());
    }

    #[test]
    fn test_is_completed() {
        assert!(!TaskState::pending().is_completed());
        assert!(!TaskState::running("a", "b").is_completed());
        assert!(TaskState::merged("s").is_completed());
        assert!(!TaskState::failed("e", 0).is_completed());
        assert!(!TaskState::rejected("r").is_completed());
    }

    #[test]
    fn test_is_failed() {
        assert!(!TaskState::pending().is_failed());
        assert!(!TaskState::running("a", "b").is_failed());
        assert!(TaskState::failed("e", 0).is_failed());
        assert!(TaskState::rejected("r").is_failed());
    }

    #[test]
    fn test_branch_and_agent() {
        let running = TaskState::running("backend", "task/t1");
        assert_eq!(running.branch(), Some("task/t1"));
        assert_eq!(running.agent(), Some("backend"));

        let verifying = TaskState::verifying("task/t2");
        assert_eq!(verifying.branch(), Some("task/t2"));
        assert_eq!(verifying.agent(), None);

        let pending = TaskState::pending();
        assert_eq!(pending.branch(), None);
        assert_eq!(pending.agent(), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(TaskState::pending().to_string(), "Pending");
        assert_eq!(TaskState::rejected("r").to_string(), "Rejected");

        let running = TaskState::running("backend", "task/t1");
        let display = running.to_string();
        assert!(display.starts_with("Running("));
        assert!(display.contains("backend"));
        assert!(display.contains("task/t1"));
    }

    #[test]
    fn test_task_states_creation() {
        let states = TaskStates::new();
        assert!(states.is_empty());
        assert_eq!(states.len(), 0);
    }

    #[test]
    fn test_from_task_ids() {
        let states = TaskStates::from_task_ids(["t1".to_string(), "t2".to_string()]);
        assert_eq!(states.len(), 2);
        assert!(matches!(states.get("t1"), Some(TaskState::Pending)));
        assert!(matches!(states.get("t2"), Some(TaskState::Pending)));
    }

    #[test]
    fn test_set_and_get() {
        let mut states = TaskStates::new();
        states.set("t1", TaskState::pending());
        assert!(states.get("t1").is_some());
        assert!(states.get("t2").is_none());

        states.set("t1", TaskState::merged("snap_001"));
        assert!(states.get("t1").unwrap().is_completed());
    }

    #[test]
    fn test_completed_tasks() {
        let mut states =
            TaskStates::from_task_ids(["t1".to_string(), "t2".to_string(), "t3".to_string()]);
        states.set("t1", TaskState::merged("snap_001"));
        states.set("t2", TaskState::running("a", "b"));
        // t3 仍为 Pending

        let completed = states.completed_tasks();
        assert_eq!(completed.len(), 1);
        assert!(completed.contains(&"t1".to_string()));
    }

    #[test]
    fn test_running_tasks() {
        let mut states =
            TaskStates::from_task_ids(["t1".to_string(), "t2".to_string(), "t3".to_string()]);
        states.set("t1", TaskState::running("backend", "task/t1"));
        states.set("t2", TaskState::running("frontend", "task/t2"));

        let running = states.running_tasks();
        assert_eq!(running.len(), 2);
        assert!(running.contains(&"t1".to_string()));
        assert!(running.contains(&"t2".to_string()));
    }

    #[test]
    fn test_failed_tasks() {
        let mut states = TaskStates::from_task_ids(["t1".to_string(), "t2".to_string()]);
        states.set("t1", TaskState::failed("err", 1));
        states.set("t2", TaskState::rejected("r"));

        let failed = states.failed_tasks();
        assert_eq!(failed.len(), 1);
        assert!(failed.contains(&"t1".to_string()));
    }

    #[test]
    fn test_all_terminal() {
        let mut states = TaskStates::from_task_ids(["t1".to_string(), "t2".to_string()]);
        assert!(!states.all_terminal());

        states.set("t1", TaskState::merged("s1"));
        assert!(!states.all_terminal());

        states.set("t2", TaskState::rejected("r"));
        assert!(states.all_terminal());
    }

    #[test]
    fn test_default_is_pending() {
        let state = TaskState::default();
        assert!(matches!(state, TaskState::Pending));
    }

    #[test]
    fn test_tasks_in_state() {
        let mut states =
            TaskStates::from_task_ids(["t1".to_string(), "t2".to_string(), "t3".to_string()]);
        states.set("t1", TaskState::merged("s1"));
        states.set("t2", TaskState::running("a", "b"));

        let merged = states.tasks_in_state(&TaskState::merged("dummy"));
        assert_eq!(merged.len(), 1);
        assert!(merged.contains(&"t1".to_string()));

        let running = states.tasks_in_state(&TaskState::running("dummy", "dummy"));
        assert_eq!(running.len(), 1);
        assert!(running.contains(&"t2".to_string()));
    }

    #[test]
    fn test_serde_roundtrip() {
        let state = TaskState::running("backend", "task/t1");
        let json = serde_json::to_string(&state).unwrap();
        let parsed: TaskState = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, TaskState::Running { .. }));
        assert_eq!(parsed.agent(), Some("backend"));
        assert_eq!(parsed.branch(), Some("task/t1"));
    }

    #[test]
    fn test_serde_all_variants() {
        let states = vec![
            TaskState::pending(),
            TaskState::running("a", "b"),
            TaskState::verifying("b"),
            TaskState::merged("s"),
            TaskState::failed("e", 1),
            TaskState::rejected("r"),
        ];

        for s in &states {
            let json = serde_json::to_string(s).unwrap();
            let parsed: TaskState = serde_json::from_str(&json).unwrap();
            let orig_str = s.to_string();
            let parsed_str = parsed.to_string();
            // 比较状态类型（去掉括号内容）
            let orig_kind = orig_str.split('(').next().unwrap();
            let parsed_kind = parsed_str.split('(').next().unwrap();
            assert_eq!(orig_kind, parsed_kind);
        }
    }
}
