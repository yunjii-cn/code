// 发布模式
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7.3
//
// 只把"正式版本"推到 git，WIP 留本地
//   - 候选版本 → 提示用户（暂不实现，留给 M2.2）
//   - 正式版本（SnapshotType::Release）→ 自动 tag + push
//
// 实现策略：
//   - 复用 SyncMode 的 git 操作逻辑（init/add/commit/push）
//   - 但 should_handle 只对 Release 类型返回 true
//   - Release 快照额外创建 git tag

use std::path::Path;
use timeflow_core::{Snapshot, SnapshotType};

use crate::adapter::{GitAdapter, GitStatus};
use crate::config::{GitConfig, GitMode};
use crate::error::{GitError, Result};
use crate::modes::sync::SyncMode;

/// 发布模式适配器
///
/// 内部委托 SyncMode 处理 git 操作，但只对 Release 类型快照生效
pub struct ReleaseMode {
    config: GitConfig,
    inner: SyncMode,
}

impl ReleaseMode {
    /// 创建发布模式实例
    pub fn new(config: GitConfig) -> Self {
        let inner = SyncMode::new(config.clone());
        Self { config, inner }
    }

    /// 为 Release 快照创建 git tag
    fn create_tag(repo: &git2::Repository, snapshot: &Snapshot, config: &GitConfig) -> Result<()> {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let sig = git2::Signature::now(&config.author_name, &config.author_email)?;

        // tag 名格式：release-{snapshot_id 前 8 位}
        let short_id = &snapshot.id[..8.min(snapshot.id.len())];
        let tag_name = format!("release-{}", short_id);

        // 如果 tag 已存在则跳过
        // 使用 refname_to_id 检查 tag 是否存在
        let tag_ref = format!("refs/tags/{}", tag_name);
        if repo.refname_to_id(&tag_ref).is_ok() {
            tracing::debug!("Git tag 已存在: {}", tag_name);
            return Ok(());
        }

        let message = format!(
            "Release {}\n\nTimeFlow-Snapshot: {}",
            snapshot
                .metadata
                .ai_commit_msg
                .as_deref()
                .unwrap_or("正式版本"),
            snapshot.id
        );

        repo.tag(&tag_name, &commit.as_object(), &sig, &message, false)?;
        tracing::info!("Git tag 创建: {} (commit={})", tag_name, &commit.id().to_string()[..8]);

        Ok(())
    }
}

impl GitAdapter for ReleaseMode {
    fn mode(&self) -> GitMode {
        GitMode::Release
    }

    fn ensure_init(&self, repo_path: &Path) -> Result<()> {
        self.inner.ensure_init(repo_path)
    }

    fn on_snapshot(
        &self,
        snapshot: &Snapshot,
        timeflow: &timeflow_core::TimeFlow,
        repo_path: &Path,
    ) -> Result<()> {
        // 只处理 Release 类型快照
        if !self.should_handle(&snapshot.metadata.snapshot_type) {
            tracing::debug!(
                "发布模式：跳过非 Release 快照 (type={:?})",
                snapshot.metadata.snapshot_type
            );
            return Ok(());
        }

        // 委托 SyncMode 执行 commit
        self.inner
            .on_snapshot(snapshot, timeflow, repo_path)?;

        // 额外创建 git tag
        let repo = SyncMode::open_or_init_repo(repo_path, &self.config)
            .map_err(|e| GitError::Other(format!("打开 git 仓库失败: {}", e)))?;
        Self::create_tag(&repo, snapshot, &self.config)?;

        // 自动 push（包括 tag）
        if self.config.auto_push && self.config.remote_url.is_some() {
            // 先 push commits
            self.inner.push(repo_path)?;
            // 再 push tags
            let mut remote = repo.find_remote(&self.config.remote_name)?;
            let mut push_options = git2::PushOptions::new();
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_url, _username, _allowed| {
                git2::Cred::default()
            });
            push_options.remote_callbacks(callbacks);
            let short_id = &snapshot.id[..8.min(snapshot.id.len())];
            let refspec = format!("refs/tags/release-{}:refs/tags/release-{}", short_id, short_id);
            remote.push(&[&refspec], Some(&mut push_options))?;
            tracing::info!("Git tag pushed: release-{}", short_id);
        }

        Ok(())
    }

    fn push(&self, repo_path: &Path) -> Result<()> {
        self.inner.push(repo_path)
    }

    fn status(&self, repo_path: &Path) -> Result<GitStatus> {
        let mut status = self.inner.status(repo_path)?;
        status.mode = GitMode::Release;
        Ok(status)
    }

    fn should_handle(&self, snapshot_type: &SnapshotType) -> bool {
        // 发布模式：只处理 Release 类型
        matches!(snapshot_type, SnapshotType::Release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_config(repo_path: &Path) -> GitConfig {
        GitConfig {
            repo_path: repo_path.to_path_buf(),
            author_name: "Test User".to_string(),
            author_email: "test@example.com".to_string(),
            remote_url: None,
            remote_name: "origin".to_string(),
            auto_push: false,
        }
    }

    #[test]
    fn test_release_mode_should_handle_only_release() {
        use timeflow_core::SnapshotType;
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = ReleaseMode::new(config);

        assert!(!mode.should_handle(&SnapshotType::Wip));
        assert!(!mode.should_handle(&SnapshotType::Progress));
        assert!(!mode.should_handle(&SnapshotType::Candidate));
        assert!(mode.should_handle(&SnapshotType::Release));
    }

    #[test]
    fn test_release_mode_ensure_init() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = ReleaseMode::new(config);

        mode.ensure_init(tmp.path()).unwrap();
        assert!(tmp.path().join(".git").exists());
    }

    #[test]
    fn test_release_mode_status_mode() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = ReleaseMode::new(config);

        let status = mode.status(tmp.path()).unwrap();
        assert_eq!(status.mode, GitMode::Release);
    }

    #[test]
    fn test_release_mode_skips_non_release_snapshots() {
        use chrono::Utc;
        use timeflow_core::{SnapshotMetadata, TriggerType, BuildStatus};

        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = ReleaseMode::new(config);

        // 初始化 git 仓库
        mode.ensure_init(tmp.path()).unwrap();

        // 创建一个非 Release 快照
        let snapshot = Snapshot {
            id: "abc123".to_string(),
            parent: None,
            timestamp: Utc::now(),
            tree: "tree123".to_string(),
            author: "test".to_string(),
            metadata: SnapshotMetadata {
                trigger: TriggerType::Manual,
                ai_summary: None,
                ai_commit_msg: Some("WIP".to_string()),
                build_status: BuildStatus::Unknown,
                snapshot_type: SnapshotType::Wip,
                tags: vec![],
            },
        };

        // 应该跳过（不报错，但不创建 commit）
        // 注意：这里会失败因为没有 TimeFlow 引擎，但 should_handle 检查在前
        // 所以应该在调用 inner.on_snapshot 之前就返回 Ok(())
        let tf = timeflow_core::TimeFlow::new(tmp.path()).unwrap();
        let result = mode.on_snapshot(&snapshot, &tf, tmp.path());
        assert!(result.is_ok());

        // 验证 git 仓库中没有 commit
        let repo = git2::Repository::open(tmp.path()).unwrap();
        assert!(repo.head().is_err()); // 没有任何 commit
    }
}
