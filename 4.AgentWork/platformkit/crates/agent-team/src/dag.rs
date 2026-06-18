// 任务依赖图（TaskDag）（W8 M3.2 D1）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.2
//
// Orchestrator 输出结构化 DAG，调度引擎直接解析。
// 每个任务有：
//   - id: 唯一标识
//   - title: 任务标题
//   - assignee: 分配给哪个角色
//   - dependencies: 依赖的任务 ID 列表（必须全部完成才能执行）
//   - estimated_effort: 预估工作量
//   - acceptance_criteria: 验收标准
//
// DAG 解析：
//   - 检测循环依赖
//   - 拓扑排序
//   - 找出 ready 任务（依赖全部完成）

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 任务 ID
pub type TaskId = String;

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务 ID
    pub id: TaskId,
    /// 任务标题
    pub title: String,
    /// 分配给哪个角色
    pub assignee: String,
    /// 依赖的任务 ID 列表
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    /// 预估工作量
    #[serde(default)]
    pub estimated_effort: String,
    /// 验收标准
    #[serde(default)]
    pub acceptance_criteria: String,
    /// 任务描述（详细 prompt）
    #[serde(default)]
    pub description: String,
}

impl Task {
    /// 创建新任务
    pub fn new(id: impl Into<TaskId>, title: impl Into<String>, assignee: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            assignee: assignee.into(),
            dependencies: vec![],
            estimated_effort: String::new(),
            acceptance_criteria: String::new(),
            description: String::new(),
        }
    }

    /// 设置依赖
    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.dependencies = deps;
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置验收标准
    pub fn with_acceptance(mut self, criteria: impl Into<String>) -> Self {
        self.acceptance_criteria = criteria.into();
        self
    }
}

/// 任务 DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDag {
    /// 工作流 ID
    pub workflow_id: String,
    /// 原始需求
    pub requirement: String,
    /// 任务列表
    pub tasks: Vec<Task>,
}

impl TaskDag {
    /// 创建空 DAG
    pub fn new(workflow_id: impl Into<String>, requirement: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            requirement: requirement.into(),
            tasks: vec![],
        }
    }

    /// 从 JSON 解析
    pub fn from_json(json: &str) -> Result<Self> {
        let dag: Self = serde_json::from_str(json)?;
        dag.validate()?;
        Ok(dag)
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 添加任务
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        // 检查 ID 唯一
        if self.tasks.iter().any(|t| t.id == task.id) {
            return Err(TeamError::Config(format!("任务 ID '{}' 重复", task.id)));
        }
        // 检查依赖是否存在
        for dep in &task.dependencies {
            if !self.tasks.iter().any(|t| &t.id == dep) && dep != &task.id {
                // 允许依赖尚未添加的任务（后续会添加）
                // 但 validate 时会检查
            }
        }
        self.tasks.push(task);
        Ok(())
    }

    /// 获取任务
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// 获取任务（mutable）
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// 校验 DAG 合法性
    ///
    /// 1. 任务 ID 唯一
    /// 2. 依赖存在
    /// 3. 无循环依赖
    pub fn validate(&self) -> Result<()> {
        if self.tasks.is_empty() {
            return Err(TeamError::Config("DAG 至少需要一个任务".to_string()));
        }

        // 检查 ID 唯一
        let mut seen = HashSet::new();
        for task in &self.tasks {
            if !seen.insert(&task.id) {
                return Err(TeamError::Config(format!("任务 ID '{}' 重复", task.id)));
            }
        }

        // 检查依赖存在
        for task in &self.tasks {
            for dep in &task.dependencies {
                if !seen.contains(dep) {
                    return Err(TeamError::Config(format!(
                        "任务 '{}' 依赖不存在的任务 '{}'",
                        task.id, dep
                    )));
                }
                // 自依赖
                if dep == &task.id {
                    return Err(TeamError::Config(format!(
                        "任务 '{}' 不能依赖自己",
                        task.id
                    )));
                }
            }
        }

        // 检测循环依赖（DFS）
        self.detect_cycle()?;

        Ok(())
    }

    /// 检测循环依赖
    fn detect_cycle(&self) -> Result<()> {
        let task_map: HashMap<&str, &Task> = self
            .tasks
            .iter()
            .map(|t| (t.id.as_str(), t))
            .collect();

        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for task in &self.tasks {
            if self.has_cycle_dfs(&task.id, &task_map, &mut visited, &mut recursion_stack)? {
                return Err(TeamError::Config(format!(
                    "检测到循环依赖，涉及任务 '{}'",
                    task.id
                )));
            }
        }
        Ok(())
    }

    fn has_cycle_dfs(
        &self,
        task_id: &str,
        task_map: &HashMap<&str, &Task>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> Result<bool> {
        if recursion_stack.contains(task_id) {
            return Ok(true);
        }
        if visited.contains(task_id) {
            return Ok(false);
        }

        visited.insert(task_id.to_string());
        recursion_stack.insert(task_id.to_string());

        if let Some(task) = task_map.get(task_id) {
            for dep in &task.dependencies {
                if self.has_cycle_dfs(dep, task_map, visited, recursion_stack)? {
                    return Ok(true);
                }
            }
        }

        recursion_stack.remove(task_id);
        Ok(false)
    }

    /// 获取所有 ready 任务（依赖全部完成的任务）
    ///
    /// `completed_tasks`: 已完成的任务 ID 集合
    pub fn get_ready_tasks(&self, completed_tasks: &HashSet<TaskId>) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| {
                // 未完成
                !completed_tasks.contains(&t.id)
                // 所有依赖都已完成
                && t.dependencies.iter().all(|dep| completed_tasks.contains(dep))
            })
            .collect()
    }

    /// 获取所有 ready 任务 ID
    pub fn get_ready_task_ids(&self, completed_tasks: &HashSet<TaskId>) -> Vec<TaskId> {
        self.get_ready_tasks(completed_tasks)
            .iter()
            .map(|t| t.id.clone())
            .collect()
    }

    /// 拓扑排序
    ///
    /// 返回任务的执行顺序（Kahn 算法）
    pub fn topological_sort(&self) -> Result<Vec<TaskId>> {
        self.validate()?;

        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        let mut adj: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        for task in &self.tasks {
            in_degree.entry(task.id.clone()).or_insert(0);
            adj.entry(task.id.clone()).or_default();
            for dep in &task.dependencies {
                adj.entry(dep.clone()).or_default().push(task.id.clone());
                *in_degree.entry(task.id.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<TaskId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut result = Vec::new();
        while let Some(id) = queue.pop() {
            result.push(id.clone());
            if let Some(neighbors) = adj.get(&id) {
                for n in neighbors {
                    if let Some(deg) = in_degree.get_mut(n) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(n.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.tasks.len() {
            return Err(TeamError::Config("拓扑排序失败：存在循环依赖".to_string()));
        }

        Ok(result)
    }

    /// 任务总数
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// 获取所有角色 ID（去重）
    pub fn assignees(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for task in &self.tasks {
            set.insert(task.assignee.clone());
        }
        set.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_dag() -> TaskDag {
        let mut dag = TaskDag::new("wf_test", "测试需求");
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

    #[test]
    fn test_dag_creation() {
        let dag = make_simple_dag();
        assert_eq!(dag.workflow_id, "wf_test");
        assert_eq!(dag.task_count(), 3);
    }

    #[test]
    fn test_get_task() {
        let dag = make_simple_dag();
        assert!(dag.get_task("t1").is_some());
        assert!(dag.get_task("nonexistent").is_none());
    }

    #[test]
    fn test_validate_empty_dag() {
        let dag = TaskDag::new("empty", "");
        assert!(dag.validate().is_err());
    }

    #[test]
    fn test_validate_duplicate_id() {
        let mut dag = TaskDag::new("dup", "");
        dag.add_task(Task::new("t1", "A", "backend")).unwrap();
        let result = dag.add_task(Task::new("t1", "B", "frontend"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let mut dag = TaskDag::new("missing", "");
        dag.add_task(
            Task::new("t1", "A", "backend").with_dependencies(vec!["nonexistent".to_string()]),
        )
        .unwrap();
        assert!(dag.validate().is_err());
    }

    #[test]
    fn test_validate_self_dependency() {
        let mut dag = TaskDag::new("self", "");
        dag.add_task(
            Task::new("t1", "A", "backend").with_dependencies(vec!["t1".to_string()]),
        )
        .unwrap();
        assert!(dag.validate().is_err());
    }

    #[test]
    fn test_validate_cycle() {
        let mut dag = TaskDag::new("cycle", "");
        dag.add_task(
            Task::new("t1", "A", "backend").with_dependencies(vec!["t3".to_string()]),
        )
        .unwrap();
        dag.add_task(
            Task::new("t2", "B", "frontend").with_dependencies(vec!["t1".to_string()]),
        )
        .unwrap();
        dag.add_task(
            Task::new("t3", "C", "tester").with_dependencies(vec!["t2".to_string()]),
        )
        .unwrap();
        assert!(dag.validate().is_err());
    }

    #[test]
    fn test_get_ready_tasks_empty_completed() {
        let dag = make_simple_dag();
        let completed = HashSet::new();
        let ready = dag.get_ready_tasks(&completed);
        // 只有 t1 无依赖
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t1");
    }

    #[test]
    fn test_get_ready_tasks_after_t1() {
        let dag = make_simple_dag();
        let mut completed = HashSet::new();
        completed.insert("t1".to_string());
        let ready = dag.get_ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t2");
    }

    #[test]
    fn test_get_ready_tasks_all_completed() {
        let dag = make_simple_dag();
        let mut completed = HashSet::new();
        completed.insert("t1".to_string());
        completed.insert("t2".to_string());
        completed.insert("t3".to_string());
        let ready = dag.get_ready_tasks(&completed);
        assert!(ready.is_empty());
    }

    #[test]
    fn test_get_ready_tasks_parallel() {
        // t1, t2 无依赖，可并行；t3 依赖 t1 和 t2
        let mut dag = TaskDag::new("parallel", "");
        dag.add_task(Task::new("t1", "A", "backend")).unwrap();
        dag.add_task(Task::new("t2", "B", "frontend")).unwrap();
        dag.add_task(
            Task::new("t3", "C", "tester")
                .with_dependencies(vec!["t1".to_string(), "t2".to_string()]),
        )
        .unwrap();

        let completed = HashSet::new();
        let ready = dag.get_ready_tasks(&completed);
        assert_eq!(ready.len(), 2);
        let ids: Vec<&str> = ready.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"t1"));
        assert!(ids.contains(&"t2"));
    }

    #[test]
    fn test_topological_sort() {
        let dag = make_simple_dag();
        let order = dag.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        // t1 应在 t2 前，t2 应在 t3 前
        let pos_t1 = order.iter().position(|x| x == "t1").unwrap();
        let pos_t2 = order.iter().position(|x| x == "t2").unwrap();
        let pos_t3 = order.iter().position(|x| x == "t3").unwrap();
        assert!(pos_t1 < pos_t2);
        assert!(pos_t2 < pos_t3);
    }

    #[test]
    fn test_topological_sort_parallel() {
        let mut dag = TaskDag::new("parallel", "");
        dag.add_task(Task::new("t1", "A", "backend")).unwrap();
        dag.add_task(Task::new("t2", "B", "frontend")).unwrap();
        dag.add_task(
            Task::new("t3", "C", "tester")
                .with_dependencies(vec!["t1".to_string(), "t2".to_string()]),
        )
        .unwrap();

        let order = dag.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        // t3 应在最后
        assert_eq!(order[2], "t3");
    }

    #[test]
    fn test_json_roundtrip() {
        let dag = make_simple_dag();
        let json = dag.to_json().unwrap();
        let parsed = TaskDag::from_json(&json).unwrap();
        assert_eq!(parsed.workflow_id, dag.workflow_id);
        assert_eq!(parsed.task_count(), dag.task_count());
    }

    #[test]
    fn test_assignees() {
        let dag = make_simple_dag();
        let mut assignees = dag.assignees();
        assignees.sort();
        assert_eq!(assignees, vec!["backend", "frontend", "tester"]);
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("t1", "标题", "backend")
            .with_dependencies(vec!["t0".to_string()])
            .with_description("详细描述")
            .with_acceptance("验收标准");
        assert_eq!(task.id, "t1");
        assert_eq!(task.title, "标题");
        assert_eq!(task.assignee, "backend");
        assert_eq!(task.dependencies, vec!["t0"]);
        assert_eq!(task.description, "详细描述");
        assert_eq!(task.acceptance_criteria, "验收标准");
    }

    #[test]
    fn test_get_task_mut() {
        let mut dag = make_simple_dag();
        let task = dag.get_task_mut("t1").unwrap();
        task.title = "修改后的标题".to_string();
        assert_eq!(dag.get_task("t1").unwrap().title, "修改后的标题");
    }

    #[test]
    fn test_get_ready_task_ids() {
        let dag = make_simple_dag();
        let completed = HashSet::new();
        let ids = dag.get_ready_task_ids(&completed);
        assert_eq!(ids, vec!["t1"]);
    }
}
