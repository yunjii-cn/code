// Prompt 构建器（M6.1 W4）
//
// 目标：按层级组装 prompt，并预留 token 预算裁剪能力。
// 设计：每个 PromptSection 有 layer + priority；构建时先按 layer 固定顺序，
//       再按 priority 降序；超出预算时从低优先级开始丢弃。

use crate::{
    employee::EmployeeDefinition,
    memory::{MemoryEntry, MemoryStore},
    sessions::SessionStore,
    skills_v2::{SkillRegistry, SkillV2},
    work_mode::WorkMode,
};
use serde::{Deserialize, Serialize};

/// Prompt 分层（数字越大越靠前/越重要）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PromptLayer {
    /// 系统层（角色人格、语气）
    System = 100,
    /// 团队层（团队规范）
    Team = 80,
    /// 员工层（员工技能/职责）
    Employee = 60,
    /// 任务层（当前任务上下文）
    Task = 40,
    /// 运行时上下文层（RAG/会话历史）
    Runtime = 20,
}

impl PromptLayer {
    /// 用于排序的权重（越大越靠前）
    pub fn weight(&self) -> u8 {
        *self as u8
    }

    /// 从字符串解析
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "team" => Self::Team,
            "employee" => Self::Employee,
            "task" => Self::Task,
            "runtime" => Self::Runtime,
            _ => Self::System,
        }
    }
}

/// Prompt 片段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptSection {
    /// 层级
    pub layer: Option<PromptLayer>,
    /// 标题
    #[serde(default)]
    pub title: String,
    /// 内容
    #[serde(default)]
    pub content: String,
    /// 优先级，越大越重要（0-255）
    #[serde(default)]
    pub priority: u8,
}

impl PromptSection {
    /// 创建片段
    pub fn new(layer: PromptLayer, content: impl Into<String>) -> Self {
        Self {
            layer: Some(layer),
            title: String::new(),
            content: content.into(),
            priority: 0,
        }
    }

    /// 设置标题
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// 综合排序键（layer.weight << 8 | priority），越大越靠前
    fn sort_key(&self) -> u16 {
        let layer = self.layer.map(|l| l.weight()).unwrap_or(0);
        ((layer as u16) << 8) | (self.priority as u16)
    }

    /// 估算 token（粗略：4 字符 ≈ 1 token，CJK 1.5 倍）
    fn estimate_tokens(&self) -> usize {
        estimate_tokens_str(&self.content) + estimate_tokens_str(&self.title)
    }
}

/// 粗略 token 估算
fn estimate_tokens_str(s: &str) -> usize {
    let cjk = s.chars().filter(|c| {
        (*c >= '\u{4E00}' && *c <= '\u{9FFF}') || (*c >= '\u{3000}' && *c <= '\u{30FF}')
    }).count();
    let ascii = s.chars().count() - cjk;
    // CJK 约 1 token/字，ASCII 约 0.25 token/字
    (cjk + (ascii + 3) / 4).max(1)
}

/// Token 预算
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptBudget {
    /// 最大 token 数
    #[serde(default)]
    pub max_tokens: usize,
    /// 预留输出 token
    #[serde(default)]
    pub reserved_output_tokens: usize,
}

impl PromptBudget {
    /// 创建预算
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            reserved_output_tokens: 0,
        }
    }

    /// 预留输出 token
    pub fn with_reserved_output(mut self, tokens: usize) -> Self {
        self.reserved_output_tokens = tokens;
        self
    }

    /// 可用输入 token（max - reserved，下限 0）
    pub fn available_input(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_output_tokens)
    }
}

/// Prompt 构建器
#[derive(Debug, Clone, Default)]
pub struct PromptBuilder {
    sections: Vec<PromptSection>,
}

impl PromptBuilder {
    /// 创建空 builder
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加片段
    pub fn push_section(&mut self, section: PromptSection) {
        self.sections.push(section);
    }

    /// 链式添加片段
    pub fn with_section(mut self, section: PromptSection) -> Self {
        self.sections.push(section);
        self
    }

    /// 当前片段数
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// 排序后的片段副本（按 layer 降序 + priority 降序）
    pub fn sorted_sections(&self) -> Vec<PromptSection> {
        let mut copy = self.sections.clone();
        copy.sort_by(|a, b| b.sort_key().cmp(&a.sort_key()));
        copy
    }

    /// 构建文本（按 layer+priority 排序后拼接）
    pub fn build(&self) -> String {
        let sorted = self.sorted_sections();
        let mut parts = Vec::new();
        for section in &sorted {
            if !section.title.is_empty() {
                parts.push(format!("## {}\n{}", section.title, section.content));
            } else if !section.content.is_empty() {
                parts.push(section.content.clone());
            }
        }
        parts.join("\n\n")
    }

    /// 在 token 预算内构建（超出部分从低优先级开始丢弃）
    /// 返回 (文本, 是否完整包含所有片段)
    pub fn build_with_budget(&self, budget: &PromptBudget) -> (String, bool) {
        let limit = budget.available_input();
        let mut sorted = self.sorted_sections();
        // 反向（低优先级在尾部）逐个估算
        let mut dropped = 0usize;
        // 从尾部往前丢弃，直到满足预算
        while !sorted.is_empty() {
            let total: usize = sorted.iter().map(|s| s.estimate_tokens() + 4).sum();
            if total <= limit {
                break;
            }
            sorted.pop();
            dropped += 1;
        }
        let used: usize = sorted.iter().map(|s| s.estimate_tokens() + 4).sum();
        let text = Self::join_sections(&sorted);
        (text, dropped == 0 && used <= limit)
    }

    fn join_sections(sections: &[PromptSection]) -> String {
        let mut parts = Vec::new();
        for section in sections {
            if !section.title.is_empty() {
                parts.push(format!("## {}\n{}", section.title, section.content));
            } else if !section.content.is_empty() {
                parts.push(section.content.clone());
            }
        }
        parts.join("\n\n")
    }

    /// 估算当前所有片段总 token
    pub fn estimate_total_tokens(&self) -> usize {
        self.sections.iter().map(|s| s.estimate_tokens() + 4).sum()
    }
}

/// 为员工 + 任务 + 上下文构建完整 prompt（端到端集成 M6.1 能力）
pub struct EmployeePromptBuilder<'a> {
    employee: &'a EmployeeDefinition,
    mode: WorkMode,
    task: String,
    memory_store: Option<&'a MemoryStore>,
    session_store: Option<&'a SessionStore>,
    skill_registry: Option<&'a SkillRegistry>,
    context_snippets: Vec<String>,
}

impl<'a> EmployeePromptBuilder<'a> {
    /// 创建 builder
    pub fn new(
        employee: &'a EmployeeDefinition,
        mode: WorkMode,
        task: impl Into<String>,
    ) -> Self {
        Self {
            employee,
            mode,
            task: task.into(),
            memory_store: None,
            session_store: None,
            skill_registry: None,
            context_snippets: Vec::new(),
        }
    }

    /// 注入记忆库
    pub fn with_memory_store(mut self, store: &'a MemoryStore) -> Self {
        self.memory_store = Some(store);
        self
    }

    /// 注入会话历史
    pub fn with_session_store(mut self, store: &'a SessionStore) -> Self {
        self.session_store = Some(store);
        self
    }

    /// 注入技能注册表
    pub fn with_skill_registry(mut self, registry: &'a SkillRegistry) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    /// 追加 RAG / 当前上下文片段
    pub fn with_context(mut self, snippet: impl Into<String>) -> Self {
        self.context_snippets.push(snippet.into());
        self
    }

    /// 构建 PromptBuilder（未裁剪）
    pub fn build_prompt_builder(&self) -> crate::Result<PromptBuilder> {
        let mut pb = PromptBuilder::new();

        // System 层：员工基础 system_prompt
        pb.push_section(
            PromptSection::new(PromptLayer::System, &self.employee.system_prompt)
                .with_title("员工设定")
                .with_priority(200),
        );

        // System 层：工作模式提示
        pb.push_section(
            PromptSection::new(PromptLayer::System, self.mode.system_prompt_hint())
                .with_title("工作模式")
                .with_priority(190),
        );

        // Employee 层：技能清单（仅描述，渐进发现）
        if let Some(registry) = self.skill_registry {
            let mut relevant: Vec<&SkillV2> = Vec::new();
            for cap in &self.employee.capabilities {
                relevant.extend(registry.by_tag(cap));
            }
            // 去重
            let mut seen = std::collections::HashSet::new();
            let skill_index: Vec<String> = relevant
                .into_iter()
                .filter(|s| seen.insert(s.manifest.id.clone()))
                .map(|s| format!("- {}: {}", s.manifest.name, s.manifest.description))
                .collect();
            if !skill_index.is_empty() {
                pb.push_section(
                    PromptSection::new(
                        PromptLayer::Employee,
                        skill_index.join("\n"),
                    )
                    .with_title("可用技能")
                    .with_priority(150),
                );
            }
        }

        // Team 层：规则/话术（简化用 capabilities + rules）
        if !self.employee.rules.is_empty() {
            pb.push_section(
                PromptSection::new(PromptLayer::Team, self.employee.rules.join("\n"))
                    .with_title("必须遵守的规则")
                    .with_priority(120),
            );
        }

        // Memory 层：员工相关记忆
        if let Some(mem_store) = self.memory_store {
            let mut facts: Vec<&MemoryEntry> = mem_store.by_scope(&self.employee.id);
            facts.sort_by(|a, b| b.priority.cmp(&a.priority));
            let top = facts.into_iter().take(10);
            let memory_text = top
                .map(|e| format!("- [{}] {}", e.kind_string(), e.content))
                .collect::<Vec<_>>()
                .join("\n");
            if !memory_text.is_empty() {
                pb.push_section(
                    PromptSection::new(PromptLayer::Employee, memory_text)
                        .with_title("已记住的事实")
                        .with_priority(100),
                );
            }
        }

        // Task 层：当前任务
        pb.push_section(
            PromptSection::new(PromptLayer::Task, &self.task)
                .with_title("当前任务")
                .with_priority(80),
        );

        // Runtime 层：上下文片段
        if !self.context_snippets.is_empty() {
            pb.push_section(
                PromptSection::new(PromptLayer::Runtime, self.context_snippets.join("\n\n"))
                    .with_title("参考资料")
                    .with_priority(60),
            );
        }

        // Runtime 层：相关历史会话（最近 3 条摘要）
        if let Some(sess_store) = self.session_store {
            if let Ok(sessions) = sess_store.by_employee(&self.employee.id, 3) {
                let summaries: Vec<String> = sessions
                    .iter()
                    .map(|s| {
                        format!(
                            "- [{}] {} ({} turns, {} tools)",
                            s.session_id,
                            s.title,
                            s.turn_count(),
                            s.total_tool_calls()
                        )
                    })
                    .collect();
                if !summaries.is_empty() {
                    pb.push_section(
                        PromptSection::new(PromptLayer::Runtime, summaries.join("\n"))
                            .with_title("近期会话")
                            .with_priority(40),
                    );
                }
            }
        }

        Ok(pb)
    }

    /// 直接构建文本（带预算裁剪）
    pub fn build_with_budget(&self, budget: &PromptBudget) -> crate::Result<(String, bool)> {
        let pb = self.build_prompt_builder()?;
        let (text, complete) = pb.build_with_budget(budget);
        Ok((text, complete))
    }
}

impl MemoryEntry {
    fn kind_string(&self) -> &'static str {
        match self.kind {
            crate::memory::MemoryKind::Fact => "事实",
            crate::memory::MemoryKind::Preference => "偏好",
            crate::memory::MemoryKind::Procedure => "流程",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_sort_key_orders_system_first() {
        let sys = PromptSection::new(PromptLayer::System, "sys").with_priority(0);
        let runtime = PromptSection::new(PromptLayer::Runtime, "ctx").with_priority(255);
        assert!(sys.sort_key() > runtime.sort_key());
    }

    #[test]
    fn build_orders_by_layer_then_priority() {
        let builder = PromptBuilder::new()
            .with_section(PromptSection::new(PromptLayer::Runtime, "runtime-content"))
            .with_section(
                PromptSection::new(PromptLayer::Employee, "employee-content")
                    .with_title("员工")
                    .with_priority(10),
            )
            .with_section(
                PromptSection::new(PromptLayer::System, "system-content")
                    .with_title("系统"),
            );
        let built = builder.build();
        let sys_pos = built.find("system-content").unwrap();
        let emp_pos = built.find("employee-content").unwrap();
        let runtime_pos = built.find("runtime-content").unwrap();
        assert!(sys_pos < emp_pos);
        assert!(emp_pos < runtime_pos);
    }

    #[test]
    fn priority_within_same_layer_orders_desc() {
        let builder = PromptBuilder::new()
            .with_section(
                PromptSection::new(PromptLayer::Task, "low").with_priority(1),
            )
            .with_section(
                PromptSection::new(PromptLayer::Task, "high").with_priority(99),
            );
        let built = builder.build();
        let high_pos = built.find("high").unwrap();
        let low_pos = built.find("low").unwrap();
        assert!(high_pos < low_pos);
    }

    #[test]
    fn build_with_budget_keeps_high_priority() {
        let big = "x".repeat(200); // 约 50 token
        let builder = PromptBuilder::new()
            .with_section(
                PromptSection::new(PromptLayer::System, "重要系统提示").with_priority(200),
            )
            .with_section(PromptSection::new(PromptLayer::Runtime, &big).with_priority(1));
        let budget = PromptBudget::new(30); // 很小，会丢弃 runtime
        let (text, complete) = builder.build_with_budget(&budget);
        assert!(text.contains("重要系统提示"));
        assert!(!complete);
    }

    #[test]
    fn build_with_budget_complete_when_enough() {
        let builder = PromptBuilder::new()
            .with_section(PromptSection::new(PromptLayer::System, "短").with_priority(100))
            .with_section(PromptSection::new(PromptLayer::Task, "也短").with_priority(10));
        let budget = PromptBudget::new(10000);
        let (text, complete) = builder.build_with_budget(&budget);
        assert!(complete);
        assert!(text.contains("短"));
    }

    #[test]
    fn reserved_output_reduces_available() {
        let budget = PromptBudget::new(100).with_reserved_output(40);
        assert_eq!(budget.available_input(), 60);
    }

    #[test]
    fn reserved_output_clamps_to_zero() {
        let budget = PromptBudget::new(10).with_reserved_output(100);
        assert_eq!(budget.available_input(), 0);
    }

    #[test]
    fn token_estimate_cjk_and_ascii() {
        assert!(estimate_tokens_str("你好") >= 2);
        assert!(estimate_tokens_str("hello world") >= 2);
        assert_eq!(estimate_tokens_str(""), 1);
    }

    #[test]
    fn layer_from_str_roundtrip() {
        assert_eq!(PromptLayer::from_str_lossy("system"), PromptLayer::System);
        assert_eq!(PromptLayer::from_str_lossy("TEAM"), PromptLayer::Team);
        assert_eq!(PromptLayer::from_str_lossy("unknown"), PromptLayer::System);
    }

    #[test]
    fn employee_prompt_builder_integrates_all_layers() {
        use crate::employee::builtin_customer_service;
        use crate::memory::{MemoryEntry, MemoryKind, MemoryStore};
        use crate::sessions::{SessionRecord, TurnRecord};
        use crate::skills_v2::{SkillRegistry, SkillV2};
        use crate::work_mode::WorkMode;

        let emp = builtin_customer_service();

        // 准备记忆
        let mut mem_store = MemoryStore::new();
        mem_store.add(
            MemoryEntry::new(
                "m1",
                emp.id.clone(),
                crate::memory::MemoryLayer::Employee,
                MemoryKind::Fact,
                "用户喜欢简洁回复",
            )
            .with_priority(80),
        );

        // 准备技能
        let mut registry = SkillRegistry::new();
        registry.register(
            SkillV2::new("refund", "退款流程")
                .with_tag("after_sales_handling")
                .with_step("确认订单", "要求用户提供订单号"),
        );

        // 准备会话历史
        let sess_store = SessionStore::open_in_memory().unwrap();
        let mut s = SessionRecord::new("s1")
            .with_employee(&emp.id)
            .with_title("历史会话");
        s.push_turn(TurnRecord::new("user", "退款"));
        sess_store.save(&s).unwrap();

        // 构建 prompt
        let (text, complete) = EmployeePromptBuilder::new(&emp, WorkMode::Collaborate, "处理退款申请")
            .with_memory_store(&mem_store)
            .with_skill_registry(&registry)
            .with_session_store(&sess_store)
            .with_context("用户订单号 DD20260622001")
            .build_with_budget(&PromptBudget::new(4000))
            .unwrap();

        assert!(!text.is_empty());
        assert!(text.contains("客服")); // system_prompt 内容
        assert!(text.contains("协作模式")); // 工作模式
        assert!(text.contains("退款流程")); // 技能
        assert!(text.contains("用户喜欢简洁回复")); // 记忆
        assert!(text.contains("DD20260622001")); // 上下文
        assert!(text.contains("历史会话")); // 会话摘要
        assert!(complete);
    }
}
