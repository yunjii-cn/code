// Git 模式配置
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{GitError, Result};

/// Git 兼容模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMode {
    /// 隐身模式（默认）：纯本地 TimeFlow，不碰 git
    Stealth,
    /// 同步模式：每个 TimeFlow 快照自动镜像为 git commit
    Sync,
    /// 发布模式：只把 Release 类型的快照推送到 git
    Release,
}

impl Default for GitMode {
    fn default() -> Self {
        Self::Stealth
    }
}

impl std::fmt::Display for GitMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stealth => write!(f, "stealth"),
            Self::Sync => write!(f, "sync"),
            Self::Release => write!(f, "release"),
        }
    }
}

impl std::str::FromStr for GitMode {
    type Err = GitError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "stealth" | "hidden" | "local" => Ok(Self::Stealth),
            "sync" | "mirror" => Ok(Self::Sync),
            "release" | "publish" => Ok(Self::Release),
            _ => Err(GitError::Other(format!("未知 Git 模式: {}", s))),
        }
    }
}

/// Git 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// 仓库路径（TimeFlow 工作区根）
    pub repo_path: PathBuf,
    /// Git 作者名
    pub author_name: String,
    /// Git 作者邮箱
    pub author_email: String,
    /// 远程仓库 URL（同步/发布模式可选）
    pub remote_url: Option<String>,
    /// 远程名称（默认 origin）
    pub remote_name: String,
    /// 是否自动 push（同步/发布模式）
    pub auto_push: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            author_name: "AgentWork".to_string(),
            author_email: "agentwork@yunji.ai".to_string(),
            remote_url: None,
            remote_name: "origin".to_string(),
            auto_push: false,
        }
    }
}

/// Git 模式管理器：负责持久化当前模式到 .yunji/timeflow/config.toml
pub struct GitModeManager {
    config_path: PathBuf,
}

impl GitModeManager {
    /// 创建模式管理器
    pub fn new(repo_path: &std::path::Path) -> Self {
        let config_path = repo_path.join(".yunji").join("timeflow").join("config.toml");
        Self { config_path }
    }

    /// 读取当前模式
    pub fn load_mode(&self) -> Result<GitMode> {
        if !self.config_path.exists() {
            return Ok(GitMode::default());
        }
        let content = std::fs::read_to_string(&self.config_path)?;
        let value: toml::Value = toml::from_str(&content)
            .map_err(|e| GitError::Other(format!("解析 config.toml 失败: {}", e)))?;

        // 优先读 [git] mode，否则默认 stealth
        let mode_str = value
            .get("git")
            .and_then(|g| g.get("mode"))
            .and_then(|m| m.as_str())
            .unwrap_or("stealth");
        Ok(mode_str.parse()?)
    }

    /// 保存当前模式
    pub fn save_mode(&self, mode: GitMode) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 读取现有配置（如果存在），合并后写回
        let mut root: toml::Value = if self.config_path.exists() {
            let content = std::fs::read_to_string(&self.config_path)?;
            toml::from_str(&content).unwrap_or_else(|_| toml::Value::Table(Default::default()))
        } else {
            toml::Value::Table(Default::default())
        };

        let table = root.as_table_mut().ok_or_else(|| {
            GitError::Other("config.toml 顶层不是 table".to_string())
        })?;

        // 确保 [git] 子表存在
        if !table.contains_key("git") {
            table.insert("git".to_string(), toml::Value::Table(Default::default()));
        }
        let git_table = table
            .get_mut("git")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| GitError::Other("[git] 不是 table".to_string()))?;

        git_table.insert("mode".to_string(), toml::Value::String(mode.to_string()));

        let serialized = toml::to_string_pretty(&root)
            .map_err(|e| GitError::Other(format!("序列化 config.toml 失败: {}", e)))?;
        std::fs::write(&self.config_path, serialized)?;

        tracing::info!("Git 模式已保存: {} -> {}", self.config_path.display(), mode);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_mode_parse() {
        assert_eq!("stealth".parse::<GitMode>().unwrap(), GitMode::Stealth);
        assert_eq!("sync".parse::<GitMode>().unwrap(), GitMode::Sync);
        assert_eq!("release".parse::<GitMode>().unwrap(), GitMode::Release);
        assert!("invalid".parse::<GitMode>().is_err());
    }

    #[test]
    fn test_git_mode_display() {
        assert_eq!(GitMode::Stealth.to_string(), "stealth");
        assert_eq!(GitMode::Sync.to_string(), "sync");
        assert_eq!(GitMode::Release.to_string(), "release");
    }

    #[test]
    fn test_git_mode_default() {
        assert_eq!(GitMode::default(), GitMode::Stealth);
    }

    #[test]
    fn test_mode_manager_load_default_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let mgr = GitModeManager::new(tmp.path());
        assert_eq!(mgr.load_mode().unwrap(), GitMode::Stealth);
    }

    #[test]
    fn test_mode_manager_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let mgr = GitModeManager::new(tmp.path());

        mgr.save_mode(GitMode::Sync).unwrap();
        assert_eq!(mgr.load_mode().unwrap(), GitMode::Sync);

        mgr.save_mode(GitMode::Release).unwrap();
        assert_eq!(mgr.load_mode().unwrap(), GitMode::Release);

        // 切回 stealth
        mgr.save_mode(GitMode::Stealth).unwrap();
        assert_eq!(mgr.load_mode().unwrap(), GitMode::Stealth);
    }

    #[test]
    fn test_mode_manager_preserves_other_keys() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".yunji").join("timeflow").join("config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &config_path,
            "[timeflow]\nversion = \"1.0\"\n[git]\nmode = \"sync\"\n",
        )
        .unwrap();

        let mgr = GitModeManager::new(tmp.path());
        assert_eq!(mgr.load_mode().unwrap(), GitMode::Sync);

        // 切换模式不应破坏其他键
        mgr.save_mode(GitMode::Release).unwrap();
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("release"));
        assert!(content.contains("version"));
    }

    #[test]
    fn test_git_mode_serde() {
        let json = serde_json::to_string(&GitMode::Sync).unwrap();
        assert_eq!(json, "\"sync\"");

        let mode: GitMode = serde_json::from_str("\"release\"").unwrap();
        assert_eq!(mode, GitMode::Release);
    }
}
