// Profile 管理器（M6.3 W14）
//
// 借鉴 Hermes Profile 设计，升级为：
//   - 可视化创建（不写 YAML）
//   - 一键克隆（基于现有员工派生）
//   - 团队共享（git 同步）
//   - 权限隔离（独立密钥、独立工具集）
//
// 存储：<repo>/.aw/profiles/{profile_id}.json

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Profile 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 关联的员工 ID
    #[serde(default)]
    pub employee_id: String,
    /// 模型配置
    pub model_id: String,
    /// 备用模型
    #[serde(default)]
    pub model_fallback: Vec<String>,
    /// 自定义系统提示词
    #[serde(default)]
    pub system_prompt_override: Option<String>,
    /// 启用的工具列表
    #[serde(default)]
    pub tools: Vec<String>,
    /// 禁用的工具列表
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// 密钥（API keys），加密存储
    #[serde(default)]
    pub secrets: HashMap<String, String>,
    /// 环境变量
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// 权限白名单
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 是否是只读 Profile
    #[serde(default)]
    pub read_only: bool,
    /// 是否是团队共享
    #[serde(default)]
    pub shared: bool,
    /// 创建时间
    #[serde(default)]
    pub created_at: String,
    /// 更新时间
    #[serde(default)]
    pub updated_at: String,
}

impl Profile {
    /// 创建 Profile
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            employee_id: String::new(),
            model_id: "qwen3.7".to_string(),
            model_fallback: Vec::new(),
            system_prompt_override: None,
            tools: Vec::new(),
            disabled_tools: Vec::new(),
            secrets: HashMap::new(),
            env_vars: HashMap::new(),
            permissions: Vec::new(),
            read_only: false,
            shared: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// 克隆 Profile
    pub fn clone_profile(&self, new_id: impl Into<String>, new_name: impl Into<String>) -> Self {
        let mut cloned = self.clone();
        cloned.id = new_id.into();
        cloned.name = new_name.into();
        cloned.created_at = String::new();
        cloned.updated_at = String::new();
        // 克隆时清除密钥（安全）
        cloned.secrets.clear();
        cloned
    }

    /// 检查工具是否启用
    pub fn is_tool_enabled(&self, tool: &str) -> bool {
        if self.disabled_tools.contains(&tool.to_string()) {
            return false;
        }
        if self.tools.is_empty() {
            return true; // 空列表 = 全部启用
        }
        self.tools.contains(&tool.to_string())
    }

    /// 检查是否有某权限
    pub fn has_permission(&self, perm: &str) -> bool {
        if self.permissions.is_empty() {
            return true; // 空列表 = 全部权限
        }
        self.permissions.contains(&perm.to_string())
    }
}

/// Profile 管理器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileManager {
    /// Profile 列表
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// 当前激活的 Profile ID
    #[serde(default)]
    pub active_id: Option<String>,
}

impl ProfileManager {
    /// 创建管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 Profile
    pub fn add(&mut self, profile: Profile) -> Result<()> {
        if self.profiles.iter().any(|p| p.id == profile.id) {
            return Err(crate::error::TeamError::Config(format!(
                "Profile '{}' 已存在",
                profile.id
            )));
        }
        self.profiles.push(profile);
        Ok(())
    }

    /// 删除 Profile
    pub fn remove(&mut self, id: &str) -> bool {
        let len_before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
        self.profiles.len() != len_before
    }

    /// 获取 Profile
    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// 获取可变 Profile
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// 克隆 Profile
    pub fn clone(&self, source_id: &str, new_id: &str, new_name: &str) -> Result<Profile> {
        let source = self
            .get(source_id)
            .ok_or_else(|| crate::error::TeamError::Config(format!("Profile '{}' 不存在", source_id)))?;
        Ok(source.clone_profile(new_id, new_name))
    }

    /// 设置激活的 Profile
    pub fn set_active(&mut self, id: &str) -> Result<()> {
        if !self.profiles.iter().any(|p| p.id == id) {
            return Err(crate::error::TeamError::Config(format!("Profile '{}' 不存在", id)));
        }
        self.active_id = Some(id.to_string());
        Ok(())
    }

    /// 获取激活的 Profile
    pub fn active(&self) -> Option<&Profile> {
        self.active_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|p| &p.id == id))
    }

    /// 保存到目录
    pub fn save_to_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("profiles.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// 从目录加载
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join("profiles.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_add_and_get() {
        let mut pm = ProfileManager::new();
        let profile = Profile::new("dev", "开发 Profile");
        pm.add(profile).unwrap();
        assert!(pm.get("dev").is_some());
    }

    #[test]
    fn test_profile_duplicate_fails() {
        let mut pm = ProfileManager::new();
        pm.add(Profile::new("dev", "Dev")).unwrap();
        assert!(pm.add(Profile::new("dev", "Dev2")).is_err());
    }

    #[test]
    fn test_profile_remove() {
        let mut pm = ProfileManager::new();
        pm.add(Profile::new("dev", "Dev")).unwrap();
        pm.set_active("dev").unwrap();
        assert!(pm.remove("dev"));
        assert!(pm.get("dev").is_none());
        assert!(pm.active().is_none());
    }

    #[test]
    fn test_profile_clone() {
        let mut pm = ProfileManager::new();
        let mut profile = Profile::new("dev", "Dev");
        profile.secrets.insert("API_KEY".to_string(), "secret123".to_string());
        pm.add(profile).unwrap();

        let cloned = pm.clone("dev", "dev2", "Dev2").unwrap();
        assert_eq!(cloned.id, "dev2");
        assert_eq!(cloned.name, "Dev2");
        // 克隆时清除密钥
        assert!(cloned.secrets.is_empty());
    }

    #[test]
    fn test_tool_enabled() {
        let mut profile = Profile::new("dev", "Dev");
        // 空列表 = 全部启用
        assert!(profile.is_tool_enabled("any_tool"));

        profile.tools = vec!["file_read".to_string(), "file_write".to_string()];
        assert!(profile.is_tool_enabled("file_read"));
        assert!(!profile.is_tool_enabled("shell_exec"));

        profile.disabled_tools = vec!["file_read".to_string()];
        assert!(!profile.is_tool_enabled("file_read"));
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut pm = ProfileManager::new();
        pm.add(Profile::new("dev", "Dev")).unwrap();
        pm.set_active("dev").unwrap();

        pm.save_to_dir(dir.path()).unwrap();
        let loaded = ProfileManager::load_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.active_id.as_deref(), Some("dev"));
    }
}