// 共享上下文池（SharedContext）（W8 M3.2 D4）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.4
//
// 关键设计：以 Git 仓库作为上下文总线，而非另建数据库。
// 所有 Agent 共享项目知识，存储在 .yunji/context/ 目录（Git 管理）。
//
// 本模块提供内存版的 SharedContext，外部系统（TimeFlow / AST 引擎）
// 通过 write 方法注入上下文，Agent 通过 read_for_role 读取。
//
// 上下文分类：
//   - Code: 代码片段（AST 智能提取）
//   - Documentation: 项目文档
//   - Standard: 项目规范（编码规范、架构规范）
//   - Note: 任务笔记（Agent 完成任务后的总结）
//
// 角色过滤：
//   每个上下文条目可指定可见角色，read_for_role 按角色过滤。
//   例如：数据库 Schema 只对 backend / db 角色可见，不对 frontend 可见。

use crate::dag::TaskId;
use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 上下文分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextCategory {
    /// 代码片段
    Code,
    /// 项目文档
    Documentation,
    /// 项目规范
    Standard,
    /// 任务笔记
    Note,
}

impl std::fmt::Display for ContextCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code => write!(f, "code"),
            Self::Documentation => write!(f, "documentation"),
            Self::Standard => write!(f, "standard"),
            Self::Note => write!(f, "note"),
        }
    }
}

/// 上下文条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// 条目键（唯一标识，例如 "src/main.rs" / "docs/api.md"）
    pub key: String,
    /// 内容
    pub content: String,
    /// 分类
    pub category: ContextCategory,
    /// 可见角色列表（空表示对所有角色可见）
    #[serde(default)]
    pub visible_roles: Vec<String>,
    /// 来源任务 ID（如果是任务产出）
    #[serde(default)]
    pub source_task: Option<TaskId>,
}

impl ContextEntry {
    /// 创建新条目
    pub fn new(key: impl Into<String>, content: impl Into<String>, category: ContextCategory) -> Self {
        Self {
            key: key.into(),
            content: content.into(),
            category,
            visible_roles: vec![],
            source_task: None,
        }
    }

    /// 设置可见角色
    pub fn visible_to(mut self, roles: Vec<String>) -> Self {
        self.visible_roles = roles;
        self
    }

    /// 设置来源任务
    pub fn from_task(mut self, task_id: impl Into<TaskId>) -> Self {
        self.source_task = Some(task_id.into());
        self
    }

    /// 是否对指定角色可见
    pub fn is_visible_to(&self, role: &str) -> bool {
        // 空列表表示对所有角色可见
        self.visible_roles.is_empty() || self.visible_roles.iter().any(|r| r == role)
    }
}

/// 任务产出（Artifact）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// 产出内容
    pub content: String,
    /// 产出类型
    pub artifact_type: ArtifactType,
    /// 产出时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 产出类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// 代码改动
    Code,
    /// Diff
    Diff,
    /// 任务总结
    Summary,
    /// 错误日志
    ErrorLog,
}

impl Artifact {
    /// 创建代码产出
    pub fn code(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            artifact_type: ArtifactType::Code,
            created_at: chrono::Utc::now(),
        }
    }

    /// 创建总结产出
    pub fn summary(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            artifact_type: ArtifactType::Summary,
            created_at: chrono::Utc::now(),
        }
    }

    /// 创建 Diff 产出
    pub fn diff(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            artifact_type: ArtifactType::Diff,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Agent 上下文（按角色过滤后的上下文集合）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// 代码片段
    pub code_snippets: Vec<ContextEntry>,
    /// 文档
    pub docs: Vec<ContextEntry>,
    /// 上游任务产出
    pub upstream_artifacts: Vec<Artifact>,
    /// 项目规范
    pub standards: Vec<ContextEntry>,
    /// 任务笔记
    pub notes: Vec<ContextEntry>,
}

impl AgentContext {
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.code_snippets.is_empty()
            && self.docs.is_empty()
            && self.upstream_artifacts.is_empty()
            && self.standards.is_empty()
            && self.notes.is_empty()
    }

    /// 条目总数
    pub fn entry_count(&self) -> usize {
        self.code_snippets.len()
            + self.docs.len()
            + self.upstream_artifacts.len()
            + self.standards.len()
            + self.notes.len()
    }

    /// 拼接所有内容为一个字符串（供 LLM prompt 使用）
    pub fn to_prompt_context(&self) -> String {
        let mut parts = Vec::new();

        if !self.standards.is_empty() {
            parts.push("## 项目规范".to_string());
            for s in &self.standards {
                parts.push(format!("### {}\n{}", s.key, s.content));
            }
        }

        if !self.docs.is_empty() {
            parts.push("## 相关文档".to_string());
            for d in &self.docs {
                parts.push(format!("### {}\n{}", d.key, d.content));
            }
        }

        if !self.code_snippets.is_empty() {
            parts.push("## 相关代码".to_string());
            for c in &self.code_snippets {
                parts.push(format!("### {}\n```\n{}\n```", c.key, c.content));
            }
        }

        if !self.upstream_artifacts.is_empty() {
            parts.push("## 上游任务产出".to_string());
            for a in &self.upstream_artifacts {
                parts.push(format!("[{}] {}", a.artifact_type_label(), a.content));
            }
        }

        if !self.notes.is_empty() {
            parts.push("## 任务笔记".to_string());
            for n in &self.notes {
                parts.push(format!("### {}\n{}", n.key, n.content));
            }
        }

        parts.join("\n\n")
    }
}

impl Artifact {
    /// 产出类型标签
    pub fn artifact_type_label(&self) -> &'static str {
        match self.artifact_type {
            ArtifactType::Code => "代码",
            ArtifactType::Diff => "Diff",
            ArtifactType::Summary => "总结",
            ArtifactType::ErrorLog => "错误日志",
        }
    }
}

/// 共享上下文池
///
/// 所有 Agent 共享项目知识。外部系统（TimeFlow / AST 引擎）通过 write 注入，
/// Agent 通过 read_for_role 读取。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharedContext {
    /// 上下文条目（key → ContextEntry）
    pub entries: HashMap<String, ContextEntry>,
    /// 任务产出（task_id → artifacts）
    pub task_artifacts: HashMap<TaskId, Vec<Artifact>>,
}

impl SharedContext {
    /// 创建空上下文池
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加上下文条目
    pub fn add_entry(&mut self, entry: ContextEntry) -> Result<()> {
        if self.entries.contains_key(&entry.key) {
            return Err(TeamError::Config(format!(
                "上下文条目 '{}' 已存在",
                entry.key
            )));
        }
        self.entries.insert(entry.key.clone(), entry);
        Ok(())
    }

    /// 获取上下文条目
    pub fn get_entry(&self, key: &str) -> Option<&ContextEntry> {
        self.entries.get(key)
    }

    /// 更新上下文条目（如果存在则覆盖，不存在则添加）
    pub fn upsert_entry(&mut self, entry: ContextEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// 移除上下文条目
    pub fn remove_entry(&mut self, key: &str) -> Option<ContextEntry> {
        self.entries.remove(key)
    }

    /// 添加任务产出
    pub fn add_artifact(&mut self, task_id: impl Into<TaskId>, artifact: Artifact) {
        self.task_artifacts
            .entry(task_id.into())
            .or_default()
            .push(artifact);
    }

    /// 获取任务产出
    pub fn get_artifacts(&self, task_id: &str) -> &[Artifact] {
        self.task_artifacts
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取多个任务的产出（用于上游任务产出聚合）
    pub fn get_artifacts_for_tasks(&self, task_ids: &[TaskId]) -> Vec<Artifact> {
        let mut result = Vec::new();
        for id in task_ids {
            if let Some(arts) = self.task_artifacts.get(id) {
                result.extend(arts.iter().cloned());
            }
        }
        result
    }

    /// 按角色读取上下文
    ///
    /// 根据角色和任务依赖关系，聚合相关上下文：
    ///   1. 项目规范（所有角色可见的）
    ///   2. 角色相关文档
    ///   3. 任务相关代码
    ///   4. 上游任务产出（依赖任务的产出）
    ///   5. 任务笔记
    pub fn read_for_role(
        &self,
        role: &str,
        task: &crate::dag::Task,
    ) -> AgentContext {
        let mut context = AgentContext::default();

        for entry in self.entries.values() {
            if !entry.is_visible_to(role) {
                continue;
            }

            match entry.category {
                ContextCategory::Standard => context.standards.push(entry.clone()),
                ContextCategory::Documentation => context.docs.push(entry.clone()),
                ContextCategory::Code => context.code_snippets.push(entry.clone()),
                ContextCategory::Note => context.notes.push(entry.clone()),
            }
        }

        // 上游任务产出
        context.upstream_artifacts = self.get_artifacts_for_tasks(&task.dependencies);

        context
    }

    /// Agent 写入上下文（完成任务后）
    ///
    /// 将任务产出添加到上下文池，供下游任务使用。
    pub fn write_artifact(
        &mut self,
        task_id: impl Into<TaskId>,
        artifact: Artifact,
    ) {
        self.add_artifact(task_id, artifact);
    }

    /// 写入上下文条目
    pub fn write_entry(&mut self, entry: ContextEntry) {
        self.upsert_entry(entry);
    }

    /// 条目总数
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 任务产出总数
    pub fn artifact_count(&self) -> usize {
        self.task_artifacts.values().map(|v| v.len()).sum()
    }

    /// 按分类获取条目
    pub fn entries_by_category(&self, category: &ContextCategory) -> Vec<&ContextEntry> {
        self.entries
            .values()
            .filter(|e| e.category == *category)
            .collect()
    }

    /// 清空所有上下文
    pub fn clear(&mut self) {
        self.entries.clear();
        self.task_artifacts.clear();
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 从 JSON 解析
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::Task;

    #[test]
    fn test_context_category_display() {
        assert_eq!(ContextCategory::Code.to_string(), "code");
        assert_eq!(ContextCategory::Documentation.to_string(), "documentation");
        assert_eq!(ContextCategory::Standard.to_string(), "standard");
        assert_eq!(ContextCategory::Note.to_string(), "note");
    }

    #[test]
    fn test_context_entry_creation() {
        let entry = ContextEntry::new("src/main.rs", "fn main() {}", ContextCategory::Code);
        assert_eq!(entry.key, "src/main.rs");
        assert_eq!(entry.content, "fn main() {}");
        assert_eq!(entry.category, ContextCategory::Code);
        assert!(entry.visible_roles.is_empty());
        assert!(entry.source_task.is_none());
    }

    #[test]
    fn test_context_entry_builder() {
        let entry = ContextEntry::new("docs/api.md", "# API", ContextCategory::Documentation)
            .visible_to(vec!["backend".to_string(), "frontend".to_string()])
            .from_task("t1");

        assert_eq!(entry.visible_roles, vec!["backend", "frontend"]);
        assert_eq!(entry.source_task, Some("t1".to_string()));
    }

    #[test]
    fn test_entry_visibility() {
        // 空角色列表 → 对所有角色可见
        let entry = ContextEntry::new("k", "v", ContextCategory::Note);
        assert!(entry.is_visible_to("backend"));
        assert!(entry.is_visible_to("frontend"));

        // 指定角色列表 → 只对列表内角色可见
        let entry = ContextEntry::new("k", "v", ContextCategory::Note)
            .visible_to(vec!["backend".to_string()]);
        assert!(entry.is_visible_to("backend"));
        assert!(!entry.is_visible_to("frontend"));
    }

    #[test]
    fn test_artifact_creation() {
        let code = Artifact::code("fn foo() {}");
        assert_eq!(code.artifact_type, ArtifactType::Code);

        let summary = Artifact::summary("完成了用户登录");
        assert_eq!(summary.artifact_type, ArtifactType::Summary);

        let diff = Artifact::diff("+ added");
        assert_eq!(diff.artifact_type, ArtifactType::Diff);
    }

    #[test]
    fn test_shared_context_creation() {
        let ctx = SharedContext::new();
        assert_eq!(ctx.entry_count(), 0);
        assert_eq!(ctx.artifact_count(), 0);
    }

    #[test]
    fn test_add_and_get_entry() {
        let mut ctx = SharedContext::new();
        let entry = ContextEntry::new("key1", "content1", ContextCategory::Code);
        ctx.add_entry(entry).unwrap();

        assert!(ctx.get_entry("key1").is_some());
        assert!(ctx.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_add_duplicate_entry() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("key1", "v1", ContextCategory::Code))
            .unwrap();
        let result = ctx.add_entry(ContextEntry::new("key1", "v2", ContextCategory::Code));
        assert!(result.is_err());
    }

    #[test]
    fn test_upsert_entry() {
        let mut ctx = SharedContext::new();
        ctx.upsert_entry(ContextEntry::new("key1", "v1", ContextCategory::Code));
        ctx.upsert_entry(ContextEntry::new("key1", "v2", ContextCategory::Code));

        let entry = ctx.get_entry("key1").unwrap();
        assert_eq!(entry.content, "v2");
    }

    #[test]
    fn test_remove_entry() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("key1", "v1", ContextCategory::Code))
            .unwrap();

        let removed = ctx.remove_entry("key1");
        assert!(removed.is_some());
        assert!(ctx.get_entry("key1").is_none());
    }

    #[test]
    fn test_add_and_get_artifact() {
        let mut ctx = SharedContext::new();
        ctx.add_artifact("t1", Artifact::code("code1"));
        ctx.add_artifact("t1", Artifact::summary("summary1"));
        ctx.add_artifact("t2", Artifact::diff("diff1"));

        let t1_artifacts = ctx.get_artifacts("t1");
        assert_eq!(t1_artifacts.len(), 2);

        let t2_artifacts = ctx.get_artifacts("t2");
        assert_eq!(t2_artifacts.len(), 1);

        let t3_artifacts = ctx.get_artifacts("t3");
        assert!(t3_artifacts.is_empty());
    }

    #[test]
    fn test_get_artifacts_for_tasks() {
        let mut ctx = SharedContext::new();
        ctx.add_artifact("t1", Artifact::code("c1"));
        ctx.add_artifact("t2", Artifact::code("c2"));
        ctx.add_artifact("t3", Artifact::code("c3"));

        let artifacts = ctx.get_artifacts_for_tasks(&["t1".to_string(), "t3".to_string()]);
        assert_eq!(artifacts.len(), 2);
    }

    #[test]
    fn test_read_for_role() {
        let mut ctx = SharedContext::new();

        // 项目规范（所有角色可见）
        ctx.add_entry(ContextEntry::new(
            "standard/coding",
            "使用 4 空格缩进",
            ContextCategory::Standard,
        ))
        .unwrap();

        // 后端文档（只对 backend 可见）
        ctx.add_entry(
            ContextEntry::new("docs/api.md", "# API 文档", ContextCategory::Documentation)
                .visible_to(vec!["backend".to_string()]),
        )
        .unwrap();

        // 前端文档（只对 frontend 可见）
        ctx.add_entry(
            ContextEntry::new("docs/ui.md", "# UI 文档", ContextCategory::Documentation)
                .visible_to(vec!["frontend".to_string()]),
        )
        .unwrap();

        // 代码片段（所有角色可见）
        ctx.add_entry(ContextEntry::new(
            "src/main.rs",
            "fn main() {}",
            ContextCategory::Code,
        ))
        .unwrap();

        // 上游任务产出
        ctx.add_artifact("t1", Artifact::summary("后端 API 已完成"));

        // 创建任务 t2，依赖 t1
        let task = Task::new("t2", "前端集成", "frontend")
            .with_dependencies(vec!["t1".to_string()]);

        // 读取 frontend 角色的上下文
        let context = ctx.read_for_role("frontend", &task);

        // 应该包含：规范 + 前端文档 + 代码 + 上游产出
        assert!(!context.standards.is_empty());
        assert!(context.docs.iter().any(|d| d.key == "docs/ui.md"));
        assert!(!context.docs.iter().any(|d| d.key == "docs/api.md")); // 后端文档不可见
        assert!(!context.code_snippets.is_empty());
        assert_eq!(context.upstream_artifacts.len(), 1);

        // 读取 backend 角色的上下文
        let context = ctx.read_for_role("backend", &task);
        assert!(context.docs.iter().any(|d| d.key == "docs/api.md"));
        assert!(!context.docs.iter().any(|d| d.key == "docs/ui.md"));
    }

    #[test]
    fn test_read_for_role_no_upstream() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("s1", "规范", ContextCategory::Standard))
            .unwrap();

        // 无依赖的任务
        let task = Task::new("t1", "任务1", "backend");

        let context = ctx.read_for_role("backend", &task);
        assert!(context.upstream_artifacts.is_empty());
        assert!(!context.standards.is_empty());
    }

    #[test]
    fn test_write_artifact() {
        let mut ctx = SharedContext::new();
        ctx.write_artifact("t1", Artifact::summary("完成"));
        assert_eq!(ctx.get_artifacts("t1").len(), 1);
    }

    #[test]
    fn test_write_entry() {
        let mut ctx = SharedContext::new();
        ctx.write_entry(ContextEntry::new("k1", "v1", ContextCategory::Code));
        ctx.write_entry(ContextEntry::new("k1", "v2", ContextCategory::Code));
        assert_eq!(ctx.get_entry("k1").unwrap().content, "v2");
    }

    #[test]
    fn test_entries_by_category() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("c1", "code1", ContextCategory::Code))
            .unwrap();
        ctx.add_entry(ContextEntry::new("c2", "code2", ContextCategory::Code))
            .unwrap();
        ctx.add_entry(ContextEntry::new("d1", "doc1", ContextCategory::Documentation))
            .unwrap();

        let code_entries = ctx.entries_by_category(&ContextCategory::Code);
        assert_eq!(code_entries.len(), 2);

        let doc_entries = ctx.entries_by_category(&ContextCategory::Documentation);
        assert_eq!(doc_entries.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("k1", "v1", ContextCategory::Code))
            .unwrap();
        ctx.add_artifact("t1", Artifact::code("c"));

        ctx.clear();
        assert_eq!(ctx.entry_count(), 0);
        assert_eq!(ctx.artifact_count(), 0);
    }

    #[test]
    fn test_json_roundtrip() {
        let mut ctx = SharedContext::new();
        ctx.add_entry(ContextEntry::new("k1", "v1", ContextCategory::Code))
            .unwrap();
        ctx.add_artifact("t1", Artifact::code("c1"));

        let json = ctx.to_json().unwrap();
        let parsed = SharedContext::from_json(&json).unwrap();

        assert_eq!(parsed.entry_count(), 1);
        assert_eq!(parsed.artifact_count(), 1);
        assert!(parsed.get_entry("k1").is_some());
    }

    #[test]
    fn test_agent_context_empty() {
        let context = AgentContext::default();
        assert!(context.is_empty());
        assert_eq!(context.entry_count(), 0);
    }

    #[test]
    fn test_agent_context_to_prompt() {
        let mut context = AgentContext::default();
        context.standards.push(ContextEntry::new(
            "standard/coding",
            "使用 4 空格缩进",
            ContextCategory::Standard,
        ));
        context.code_snippets.push(ContextEntry::new(
            "src/main.rs",
            "fn main() {}",
            ContextCategory::Code,
        ));
        context.upstream_artifacts.push(Artifact::summary("上游完成"));

        let prompt = context.to_prompt_context();
        assert!(prompt.contains("项目规范"));
        assert!(prompt.contains("相关代码"));
        assert!(prompt.contains("上游任务产出"));
        assert!(prompt.contains("使用 4 空格缩进"));
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("上游完成"));
    }

    #[test]
    fn test_agent_context_to_prompt_empty() {
        let context = AgentContext::default();
        let prompt = context.to_prompt_context();
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_artifact_type_label() {
        assert_eq!(Artifact::code("").artifact_type_label(), "代码");
        assert_eq!(Artifact::summary("").artifact_type_label(), "总结");
        assert_eq!(Artifact::diff("").artifact_type_label(), "Diff");
    }

    #[test]
    fn test_read_for_role_filters_by_role() {
        let mut ctx = SharedContext::new();

        // 三个文档，分别对不同角色可见
        ctx.add_entry(
            ContextEntry::new("d1", "backend doc", ContextCategory::Documentation)
                .visible_to(vec!["backend".to_string()]),
        )
        .unwrap();
        ctx.add_entry(
            ContextEntry::new("d2", "frontend doc", ContextCategory::Documentation)
                .visible_to(vec!["frontend".to_string()]),
        )
        .unwrap();
        ctx.add_entry(
            ContextEntry::new("d3", "shared doc", ContextCategory::Documentation)
                .visible_to(vec!["backend".to_string(), "frontend".to_string()]),
        )
        .unwrap();

        let task = Task::new("t1", "任务", "backend");

        let backend_ctx = ctx.read_for_role("backend", &task);
        assert_eq!(backend_ctx.docs.len(), 2); // d1 + d3
        assert!(backend_ctx.docs.iter().any(|d| d.key == "d1"));
        assert!(backend_ctx.docs.iter().any(|d| d.key == "d3"));

        let frontend_ctx = ctx.read_for_role("frontend", &task);
        assert_eq!(frontend_ctx.docs.len(), 2); // d2 + d3
        assert!(frontend_ctx.docs.iter().any(|d| d.key == "d2"));
        assert!(frontend_ctx.docs.iter().any(|d| d.key == "d3"));
    }
}
