// 长期记忆层（M6.1 W1）
//
// 目标：为用户 / 团队 / 员工提供结构化、可 git 跟踪的长期记忆。
// 存储：<repo>/.aw/memory/memory.json（用户）/ <repo>/.aw/memory/team-{id}.json（团队）/ ...employee-{id}.json
//
// 三层记忆：
//   - User     用户全局偏好
//   - Team     团队共享事实
//   - Employee 员工专属技能/坑点
//
// 记忆类型：
//   - Fact        长期事实（"项目用 Rust 1.88"）
//   - Preference  偏好（"喜欢简洁回复"）
//   - Procedure   程序性线索（"PDF 解析慢，先转文本"）

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 记忆层级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryLayer {
    /// 用户级记忆
    User,
    /// 团队级记忆
    Team,
    /// 员工级记忆
    Employee,
}

impl MemoryLayer {
    /// 文件名前缀
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Team => "team",
            Self::Employee => "employee",
        }
    }

    /// 从字符串解析
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "team" => Self::Team,
            "employee" => Self::Employee,
            _ => Self::User,
        }
    }
}

/// 记忆条目类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryKind {
    /// 长期事实
    Fact,
    /// 用户偏好
    Preference,
    /// 程序性线索
    Procedure,
}

impl MemoryKind {
    /// 从字符串解析
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "preference" => Self::Preference,
            "procedure" => Self::Procedure,
            _ => Self::Fact,
        }
    }
}

/// 单条记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 记忆 ID
    pub id: String,
    /// 作用域标识（用户 ID / 团队 ID / 员工 ID）
    pub scope_id: String,
    /// 记忆层级
    pub layer: MemoryLayer,
    /// 记忆类型
    pub kind: MemoryKind,
    /// 主题标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 记忆内容
    pub content: String,
    /// 重要性，范围 0-100
    #[serde(default = "default_priority")]
    pub priority: u8,
}

fn default_priority() -> u8 {
    50
}

impl MemoryEntry {
    /// 创建新记忆条目
    pub fn new(
        id: impl Into<String>,
        scope_id: impl Into<String>,
        layer: MemoryLayer,
        kind: MemoryKind,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            scope_id: scope_id.into(),
            layer,
            kind,
            tags: Vec::new(),
            content: content.into(),
            priority: 50,
        }
    }

    /// 追加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 设置优先级（自动 clamp 到 0-100）
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(100);
        self
    }
}

/// 记忆存储
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStore {
    /// 全部记忆条目
    #[serde(default)]
    pub entries: Vec<MemoryEntry>,
}

impl MemoryStore {
    /// 创建空记忆库
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一条记忆（若同 id 存在则覆盖）
    pub fn add(&mut self, entry: MemoryEntry) {
        if let Some(slot) = self.entries.iter_mut().find(|e| e.id == entry.id) {
            *slot = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// 删除指定 id 的记忆，返回是否删除成功
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != before
    }

    /// 按作用域获取记忆（不可变引用）
    pub fn by_scope(&self, scope_id: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.scope_id == scope_id)
            .collect()
    }

    /// 按层级过滤
    pub fn by_layer(&self, layer: MemoryLayer) -> Vec<&MemoryEntry> {
        self.entries.iter().filter(|e| e.layer == layer).collect()
    }

    /// 按标签过滤
    pub fn by_tag(&self, tag: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// 简单关键词搜索（大小写不敏感，匹配 content 或 tag）
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let q = query.to_ascii_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|e| {
                e.content.to_ascii_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q))
            })
            .collect()
    }

    /// 按优先级降序排序后返回前 N 条
    pub fn top_by_priority(&self, limit: usize) -> Vec<&MemoryEntry> {
        let mut refs: Vec<&MemoryEntry> = self.entries.iter().collect();
        refs.sort_by(|a, b| b.priority.cmp(&a.priority));
        refs.into_iter().take(limit).collect()
    }

    /// 按标签分组统计
    pub fn tag_summary(&self) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        for entry in &self.entries {
            for tag in &entry.tags {
                *map.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        map
    }

    /// 合并另一个记忆库（简单按 id 去重，新值覆盖旧值）
    pub fn merge(&mut self, other: &Self) {
        for entry in &other.entries {
            self.add(entry.clone());
        }
    }

    /// 当前记忆条目总数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 从 JSON 字符串加载
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 序列化为 JSON 字符串（pretty）
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 从 JSON 文件加载
    pub fn load_json(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// 保存到 JSON 文件
    pub fn save_json(&self, path: &Path) -> Result<()> {
        let content = self.to_json()?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, scope: &str, layer: MemoryLayer, content: &str) -> MemoryEntry {
        MemoryEntry::new(id, scope, layer, MemoryKind::Fact, content)
    }

    #[test]
    fn add_and_get() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "喜欢简洁回复"));
        store.add(entry("m2", "u1", MemoryLayer::User, "用 Rust 1.88"));
        assert_eq!(store.len(), 2);
        assert_eq!(store.by_scope("u1").len(), 2);
    }

    #[test]
    fn add_with_same_id_overwrites() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "old"));
        store.add(entry("m1", "u1", MemoryLayer::User, "new"));
        assert_eq!(store.len(), 1);
        assert_eq!(store.by_scope("u1")[0].content, "new");
    }

    #[test]
    fn remove_works() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "a"));
        assert!(store.remove("m1"));
        assert!(store.is_empty());
        assert!(!store.remove("not-exist"));
    }

    #[test]
    fn search_by_content_and_tag() {
        let mut store = MemoryStore::new();
        store.add(
            entry("m1", "u1", MemoryLayer::User, "PDF 解析慢")
                .with_tag("tool-pitfall"),
        );
        store.add(entry("m2", "u1", MemoryLayer::User, "用 Rust 1.88"));
        assert_eq!(store.search("pdf").len(), 1);
        assert_eq!(store.search("tool-pitfall").len(), 1);
        assert_eq!(store.search("rust").len(), 1);
        assert!(store.search("").is_empty());
    }

    #[test]
    fn filter_by_layer_and_tag() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "a").with_tag("style"));
        store.add(entry("m2", "t1", MemoryLayer::Team, "b").with_tag("rule"));
        assert_eq!(store.by_layer(MemoryLayer::Team).len(), 1);
        assert_eq!(store.by_tag("style").len(), 1);
    }

    #[test]
    fn top_by_priority_orders_desc() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "low").with_priority(10));
        store.add(entry("m2", "u1", MemoryLayer::User, "high").with_priority(90));
        store.add(entry("m3", "u1", MemoryLayer::User, "mid").with_priority(50));
        let top = store.top_by_priority(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].content, "high");
        assert_eq!(top[1].content, "mid");
    }

    #[test]
    fn merge_dedups_by_id() {
        let mut a = MemoryStore::new();
        a.add(entry("m1", "u1", MemoryLayer::User, "old"));
        let mut b = MemoryStore::new();
        b.add(entry("m1", "u1", MemoryLayer::User, "new"));
        b.add(entry("m2", "u1", MemoryLayer::User, "extra"));
        a.merge(&b);
        assert_eq!(a.len(), 2);
        assert_eq!(a.by_scope("u1").iter().find(|e| e.id == "m1").unwrap().content, "new");
    }

    #[test]
    fn json_roundtrip() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "测试").with_priority(80));
        let json = store.to_json().unwrap();
        let restored = MemoryStore::from_json(&json).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored.entries[0].priority, 80);
    }

    #[test]
    fn tag_summary_counts() {
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "a").with_tag("x").with_tag("y"));
        store.add(entry("m2", "u1", MemoryLayer::User, "b").with_tag("x"));
        let summary = store.tag_summary();
        assert_eq!(summary.get("x"), Some(&2));
        assert_eq!(summary.get("y"), Some(&1));
    }

    #[test]
    fn layer_prefix_and_parse() {
        assert_eq!(MemoryLayer::User.prefix(), "user");
        assert_eq!(MemoryLayer::Team.prefix(), "team");
        assert_eq!(MemoryLayer::Employee.prefix(), "employee");
        assert_eq!(MemoryLayer::from_str_lossy("TEAM"), MemoryLayer::Team);
        assert_eq!(MemoryLayer::from_str_lossy("unknown"), MemoryLayer::User);
    }

    #[test]
    fn file_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("aw_memory_test.json");
        let mut store = MemoryStore::new();
        store.add(entry("m1", "u1", MemoryLayer::User, "file").with_tag("t"));
        store.save_json(&path).unwrap();
        let loaded = MemoryStore::load_json(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        let _ = std::fs::remove_file(&path);
    }
}
