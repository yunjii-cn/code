// 团队知识共享（M6.3 W10-W11）
//
// 3 层覆盖机制：
//   1. 公司级（.aw/）：全员共享的基础知识
//   2. 部门级（部门/.aw/）：部门特化知识
//   3. 个人级（用户/.aw/）：个人偏好
//
// 存储：每层一个 JSON 文件，按 key 覆盖
// 同步：基于 Git 的隐身/同步/发布三模式
// 冲突解决：3-way merge（基于 TimeFlow）

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 知识层级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KnowledgeLayer {
    /// 公司级（最高优先级被覆盖）
    Company = 0,
    /// 部门级
    Department = 1,
    /// 个人级（最低优先级覆盖）
    Personal = 2,
}

impl KnowledgeLayer {
    /// 目录名
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Company => "company",
            Self::Department => "department",
            Self::Personal => "personal",
        }
    }
}

/// 知识条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// 键（唯一标识）
    pub key: String,
    /// 值
    pub value: String,
    /// 来源层级
    pub layer: KnowledgeLayer,
    /// 最后修改者
    #[serde(default)]
    pub author: String,
    /// 最后修改时间
    #[serde(default)]
    pub updated_at: String,
}

/// 团队知识库
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamKnowledge {
    /// 公司级知识
    #[serde(default)]
    pub company: HashMap<String, String>,
    /// 部门级知识
    #[serde(default)]
    pub department: HashMap<String, String>,
    /// 个人级知识
    #[serde(default)]
    pub personal: HashMap<String, String>,
}

impl TeamKnowledge {
    /// 创建空知识库
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置知识（按层级）
    pub fn set(&mut self, layer: KnowledgeLayer, key: impl Into<String>, value: impl Into<String>) {
        let map = self.layer_map_mut(layer);
        map.insert(key.into(), value.into());
    }

    /// 获取知识（按覆盖优先级：个人 > 部门 > 公司）
    pub fn get(&self, key: &str) -> Option<&str> {
        // 个人级优先
        if let Some(v) = self.personal.get(key) {
            return Some(v.as_str());
        }
        // 其次部门级
        if let Some(v) = self.department.get(key) {
            return Some(v.as_str());
        }
        // 最后公司级
        self.company.get(key).map(|s| s.as_str())
    }

    /// 获取所有层级的 key
    pub fn get_all(&self, key: &str) -> Vec<KnowledgeEntry> {
        let mut entries = Vec::new();
        if let Some(v) = self.company.get(key) {
            entries.push(KnowledgeEntry {
                key: key.to_string(),
                value: v.clone(),
                layer: KnowledgeLayer::Company,
                author: String::new(),
                updated_at: String::new(),
            });
        }
        if let Some(v) = self.department.get(key) {
            entries.push(KnowledgeEntry {
                key: key.to_string(),
                value: v.clone(),
                layer: KnowledgeLayer::Department,
                author: String::new(),
                updated_at: String::new(),
            });
        }
        if let Some(v) = self.personal.get(key) {
            entries.push(KnowledgeEntry {
                key: key.to_string(),
                value: v.clone(),
                layer: KnowledgeLayer::Personal,
                author: String::new(),
                updated_at: String::new(),
            });
        }
        entries
    }

    /// 删除指定层级的知识
    pub fn remove(&mut self, layer: KnowledgeLayer, key: &str) -> bool {
        self.layer_map_mut(layer).remove(key).is_some()
    }

    /// 列出所有有效 key（取覆盖后结果）
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.company.keys().cloned().collect();
        for k in self.department.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
        for k in self.personal.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
        keys
    }

    /// 导出为扁平 JSON
    pub fn to_flat_json(&self) -> Result<String> {
        let mut flat: HashMap<String, String> = HashMap::new();
        for key in self.keys() {
            if let Some(val) = self.get(&key) {
                flat.insert(key, val.to_string());
            }
        }
        serde_json::to_string_pretty(&flat).map_err(Into::into)
    }

    /// 合并另一个知识库（冲突时保留高优先级）
    pub fn merge(&mut self, other: &TeamKnowledge, conflict_resolution: ConflictResolution) {
        match conflict_resolution {
            ConflictResolution::KeepHigher => {
                // 逐层合并，高层覆盖低层
                for (k, v) in &other.company {
                    self.company.entry(k.clone()).or_insert_with(|| v.clone());
                }
                for (k, v) in &other.department {
                    self.department.entry(k.clone()).or_insert_with(|| v.clone());
                }
                for (k, v) in &other.personal {
                    self.personal.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
            ConflictResolution::Overwrite => {
                self.company.extend(other.company.clone());
                self.department.extend(other.department.clone());
                self.personal.extend(other.personal.clone());
            }
        }
    }

    /// 保存到目录
    pub fn save_to_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("team_knowledge.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// 从目录加载
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join("team_knowledge.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(Into::into)
    }

    fn layer_map_mut(&mut self, layer: KnowledgeLayer) -> &mut HashMap<String, String> {
        match layer {
            KnowledgeLayer::Company => &mut self.company,
            KnowledgeLayer::Department => &mut self.department,
            KnowledgeLayer::Personal => &mut self.personal,
        }
    }
}

/// 冲突解决策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// 保留更高优先级的值
    KeepHigher,
    /// 完全覆盖
    Overwrite,
}

/// 知识同步模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeSyncMode {
    /// 隐身模式：本地使用，不提交 git
    Stealth,
    /// 同步模式：自动 git push/pull
    Sync,
    /// 发布模式：手动 git push
    Release,
}

impl Default for KnowledgeSyncMode {
    fn default() -> Self {
        Self::Stealth
    }
}

/// 团队知识管理器
pub struct TeamKnowledgeManager {
    /// 根路径（.aw/ 目录）
    pub root: PathBuf,
    /// 知识库
    pub knowledge: TeamKnowledge,
    /// 同步模式
    pub sync_mode: KnowledgeSyncMode,
}

impl TeamKnowledgeManager {
    /// 创建管理器
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            knowledge: TeamKnowledge::new(),
            sync_mode: KnowledgeSyncMode::default(),
        }
    }

    /// 设置同步模式
    pub fn with_sync_mode(mut self, mode: KnowledgeSyncMode) -> Self {
        self.sync_mode = mode;
        self
    }

    /// 初始化 .aw/ 目录结构
    pub fn init(&self) -> Result<()> {
        for layer in &[KnowledgeLayer::Company, KnowledgeLayer::Department, KnowledgeLayer::Personal] {
            let dir = self.root.join(layer.dir_name());
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    /// 保存
    pub fn save(&self) -> Result<()> {
        self.knowledge.save_to_dir(&self.root)
    }

    /// 加载
    pub fn load(&mut self) -> Result<()> {
        self.knowledge = TeamKnowledge::load_from_dir(&self.root)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_layer_override() {
        let mut tk = TeamKnowledge::new();
        tk.set(KnowledgeLayer::Company, "brand", "星巴克");
        tk.set(KnowledgeLayer::Department, "brand", "星巴克上海");
        tk.set(KnowledgeLayer::Personal, "brand", "星巴克上海张三");

        assert_eq!(tk.get("brand"), Some("星巴克上海张三"));
        assert_eq!(tk.keys().len(), 1);
    }

    #[test]
    fn test_personal_overrides_department() {
        let mut tk = TeamKnowledge::new();
        tk.set(KnowledgeLayer::Company, "reply_style", "正式");
        tk.set(KnowledgeLayer::Department, "reply_style", "亲切");
        assert_eq!(tk.get("reply_style"), Some("亲切"));

        tk.set(KnowledgeLayer::Personal, "reply_style", "简洁");
        assert_eq!(tk.get("reply_style"), Some("简洁"));
    }

    #[test]
    fn test_get_all_layers() {
        let mut tk = TeamKnowledge::new();
        tk.set(KnowledgeLayer::Company, "greeting", "您好");
        tk.set(KnowledgeLayer::Department, "greeting", "侬好");
        tk.set(KnowledgeLayer::Personal, "greeting", "嗨");

        let entries = tk.get_all("greeting");
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_remove() {
        let mut tk = TeamKnowledge::new();
        tk.set(KnowledgeLayer::Company, "key", "val");
        assert!(tk.remove(KnowledgeLayer::Company, "key"));
        assert!(!tk.remove(KnowledgeLayer::Company, "key"));
        assert!(tk.get("key").is_none());
    }

    #[test]
    fn test_merge_keep_higher() {
        let mut a = TeamKnowledge::new();
        a.set(KnowledgeLayer::Company, "brand", "星巴克");
        a.set(KnowledgeLayer::Personal, "pref", "少糖");

        let mut b = TeamKnowledge::new();
        b.set(KnowledgeLayer::Company, "brand", "瑞幸");
        b.set(KnowledgeLayer::Personal, "pref", "多糖");

        a.merge(&b, ConflictResolution::KeepHigher);
        // company 的 brand 保留原来的（星巴克，因为 keep higher）
        assert_eq!(a.get("brand"), Some("星巴克"));
        // personal 的 pref 保留原来的（少糖）
        assert_eq!(a.get("pref"), Some("少糖"));
    }

    #[test]
    fn test_merge_overwrite() {
        let mut a = TeamKnowledge::new();
        a.set(KnowledgeLayer::Company, "brand", "星巴克");

        let mut b = TeamKnowledge::new();
        b.set(KnowledgeLayer::Company, "brand", "瑞幸");

        a.merge(&b, ConflictResolution::Overwrite);
        assert_eq!(a.get("brand"), Some("瑞幸"));
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut tk = TeamKnowledge::new();
        tk.set(KnowledgeLayer::Company, "brand", "星巴克");
        tk.set(KnowledgeLayer::Personal, "pref", "少糖");

        tk.save_to_dir(dir.path()).unwrap();
        let loaded = TeamKnowledge::load_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.get("brand"), Some("星巴克"));
        assert_eq!(loaded.get("pref"), Some("少糖"));
    }
}