// 隐身模式（默认）
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7.1
//
// 纯本地 TimeFlow，不碰 git，代码永不上云
// 所有 GitAdapter 方法都是 no-op

use std::path::Path;
use timeflow_core::Snapshot;

use crate::adapter::{GitAdapter, GitStatus};
use crate::config::{GitConfig, GitMode};
use crate::error::{GitError, Result};

/// 隐身模式适配器
pub struct StealthMode {
    #[allow(dead_code)]
    config: GitConfig,
}

impl StealthMode {
    /// 创建隐身模式实例
    pub fn new() -> Self {
        Self {
            config: GitConfig::default(),
        }
    }
}

impl Default for StealthMode {
    fn default() -> Self {
        Self::new()
    }
}

impl GitAdapter for StealthMode {
    fn mode(&self) -> GitMode {
        GitMode::Stealth
    }

    fn ensure_init(&self, _repo_path: &Path) -> Result<()> {
        // 隐身模式：不初始化 git
        tracing::debug!("隐身模式：跳过 git 初始化");
        Ok(())
    }

    fn on_snapshot(
        &self,
        _snapshot: &Snapshot,
        _timeflow: &timeflow_core::TimeFlow,
        _repo_path: &Path,
    ) -> Result<()> {
        // 隐身模式：不镜像到 git
        tracing::debug!("隐身模式：跳过 git commit");
        Ok(())
    }

    fn push(&self, _repo_path: &Path) -> Result<()> {
        // 隐身模式：不允许 push
        Err(GitError::ModeMismatch {
            current: "stealth".to_string(),
            expected: "sync 或 release".to_string(),
        })
    }

    fn status(&self, _repo_path: &Path) -> Result<GitStatus> {
        Ok(GitStatus {
            mode: GitMode::Stealth,
            initialized: false,
            current_branch: None,
            remote_url: None,
            last_commit: None,
            unpushed_commits: 0,
        })
    }

    fn should_handle(&self, _snapshot_type: &timeflow_core::SnapshotType) -> bool {
        // 隐身模式：不处理任何快照
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_stealth_mode_no_op() {
        let mode = StealthMode::new();
        let tmp = TempDir::new().unwrap();

        // ensure_init 是 no-op
        mode.ensure_init(tmp.path()).unwrap();

        // .git 目录不应存在
        assert!(!tmp.path().join(".git").exists());
    }

    #[test]
    fn test_stealth_mode_push_fails() {
        let mode = StealthMode::new();
        let tmp = TempDir::new().unwrap();
        let result = mode.push(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::ModeMismatch { current, .. } => assert_eq!(current, "stealth"),
            _ => panic!("期望 ModeMismatch 错误"),
        }
    }

    #[test]
    fn test_stealth_mode_status() {
        let mode = StealthMode::new();
        let tmp = TempDir::new().unwrap();
        let status = mode.status(tmp.path()).unwrap();
        assert_eq!(status.mode, GitMode::Stealth);
        assert!(!status.initialized);
        assert_eq!(status.unpushed_commits, 0);
    }

    #[test]
    fn test_stealth_should_handle_returns_false() {
        let mode = StealthMode::new();
        use timeflow_core::SnapshotType;
        assert!(!mode.should_handle(&SnapshotType::Wip));
        assert!(!mode.should_handle(&SnapshotType::Progress));
        assert!(!mode.should_handle(&SnapshotType::Candidate));
        assert!(!mode.should_handle(&SnapshotType::Release));
    }
}
