// 同步模式
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7.2
//
// TimeFlow 自动镜像到 git 仓库
//   - 每个快照 → git commit（使用快照的 ai_commit_msg 作为 commit message）
//   - 可选自动 push 到 GitHub/Gitee
//
// 实现策略：
//   - on_snapshot 被调用时，工作区已经处于快照状态（snapshot() 扫描工作区）
//   - 所以只需 git add -A + git commit
//   - .yunji 目录通过 .gitignore 排除

use std::path::Path;
use timeflow_core::Snapshot;

use crate::adapter::{GitAdapter, GitStatus};
use crate::config::{GitConfig, GitMode};
use crate::error::{GitError, Result};

/// 同步模式适配器
pub struct SyncMode {
    config: GitConfig,
}

impl SyncMode {
    /// 创建同步模式实例
    pub fn new(config: GitConfig) -> Self {
        Self { config }
    }

    /// 打开或初始化 git 仓库
    pub(crate) fn open_or_init_repo(repo_path: &Path, config: &GitConfig) -> Result<git2::Repository> {
        let repo = match git2::Repository::open(repo_path) {
            Ok(r) => r,
            Err(_) => {
                // 仓库不存在，初始化
                let repo = git2::Repository::init(repo_path)?;
                tracing::info!("Git 仓库已初始化: {}", repo_path.display());

                // 配置 author
                let mut cfg = repo.config()?;
                cfg.set_str("user.name", &config.author_name)?;
                cfg.set_str("user.email", &config.author_email)?;

                // 添加远程（如果配置了）
                if let Some(remote_url) = &config.remote_url {
                    // 如果远程已存在则跳过
                    if repo.find_remote(&config.remote_name).is_err() {
                        repo.remote(&config.remote_name, remote_url)?;
                        tracing::info!("Git 远程已添加: {} -> {}", config.remote_name, remote_url);
                    }
                }

                repo
            }
        };

        // 确保 .gitignore 包含 .yunji
        Self::ensure_gitignore(repo_path)?;

        Ok(repo)
    }

    /// 确保 .gitignore 包含必要的排除项
    fn ensure_gitignore(repo_path: &Path) -> Result<()> {
        let gitignore_path = repo_path.join(".gitignore");
        let mut content = if gitignore_path.exists() {
            std::fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };

        // 必须排除的路径
        let required = [".yunji/", "node_modules/", "target/", "dist/", "build/", ".next/", ".cache/"];
        for line in required {
            if !content.lines().any(|l| l.trim() == line) {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(line);
                content.push('\n');
            }
        }

        std::fs::write(&gitignore_path, content)?;
        Ok(())
    }

    /// 执行 git add -A
    fn add_all(repo: &git2::Repository) -> Result<()> {
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// 创建 git commit
    fn commit(repo: &git2::Repository, message: &str, config: &GitConfig) -> Result<Option<git2::Oid>> {
        let sig = git2::Signature::now(&config.author_name, &config.author_email)?;
        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        // 获取当前 HEAD（如果有）
        let parents: Vec<git2::Commit> = match repo.head() {
            Ok(head) => vec![head.peel_to_commit()?],
            Err(_) => vec![],
        };

        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        // 检查是否有变更（避免空 commit）
        if let Some(last) = parents.first() {
            let last_tree = last.tree()?;
            let diff = repo.diff_tree_to_tree(Some(&last_tree), Some(&tree), None)?;
            if diff.deltas().count() == 0 {
                tracing::debug!("Git: 无变更，跳过 commit");
                return Ok(None);
            }
        }

        let oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parent_refs,
        )?;

        tracing::info!("Git commit: {} ({})", &oid.to_string()[..8], message);
        Ok(Some(oid))
    }

    /// 推送到远程
    fn push_to_remote(repo: &git2::Repository, config: &GitConfig) -> Result<()> {
        let remote_name = &config.remote_name;
        let mut remote = repo.find_remote(remote_name)?;

        // 获取当前分支名
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| GitError::Other("无法获取当前分支名".to_string()))?;

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        // libgit2 push 需要回调设置凭据
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
                // 使用默认凭据（SSH agent 或 cached credentials）
                git2::Cred::default()
            });
        callbacks.push_update_reference(|_refname, status| {
            if let Some(s) = status {
                return Err(git2::Error::from_str(s));
            }
            Ok(())
        });

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);
        remote.push(&[&refspec], Some(&mut push_options))?;

        tracing::info!("Git push: {} -> {}", branch_name, remote_name);
        Ok(())
    }
}

impl GitAdapter for SyncMode {
    fn mode(&self) -> GitMode {
        GitMode::Sync
    }

    fn ensure_init(&self, repo_path: &Path) -> Result<()> {
        Self::open_or_init_repo(repo_path, &self.config)?;
        Ok(())
    }

    fn on_snapshot(
        &self,
        snapshot: &Snapshot,
        _timeflow: &timeflow_core::TimeFlow,
        repo_path: &Path,
    ) -> Result<()> {
        let repo = Self::open_or_init_repo(repo_path, &self.config)?;

        // 1. git add -A
        Self::add_all(&repo)?;

        // 2. git commit（使用快照的 commit message）
        let message = snapshot
            .metadata
            .ai_commit_msg
            .clone()
            .unwrap_or_else(|| format!("snapshot: {}", &snapshot.id[..8.min(snapshot.id.len())]));
        let full_message = format!("{}\n\nTimeFlow-Snapshot: {}", message, snapshot.id);

        let commit_oid = Self::commit(&repo, &full_message, &self.config)?;

        // 3. 自动 push（如果配置）
        if commit_oid.is_some() && self.config.auto_push && self.config.remote_url.is_some() {
            Self::push_to_remote(&repo, &self.config)?;
        }

        Ok(())
    }

    fn push(&self, repo_path: &Path) -> Result<()> {
        let repo = Self::open_or_init_repo(repo_path, &self.config)?;
        if self.config.remote_url.is_none() {
            return Err(GitError::Other("未配置远程仓库 URL".to_string()));
        }
        Self::push_to_remote(&repo, &self.config)
    }

    fn status(&self, repo_path: &Path) -> Result<GitStatus> {
        let repo_result = git2::Repository::open(repo_path);

        match repo_result {
            Ok(repo) => {
                let current_branch = repo
                    .head()
                    .ok()
                    .and_then(|h| h.shorthand().map(|s| s.to_string()));

                let last_commit = repo
                    .head()
                    .ok()
                    .and_then(|h| h.peel_to_commit().ok())
                    .map(|c| c.id().to_string());

                // 计算未推送的 commit 数（粗略：本地 HEAD 与 upstream 的差）
                let unpushed_commits = if let Some(ref branch) = current_branch {
                    let local_head = repo
                        .head()
                        .ok()
                        .and_then(|h| h.peel_to_commit().ok())
                        .map(|c| c.id());
                    let upstream = repo
                        .find_branch(
                            &format!("{}/{}", self.config.remote_name, branch),
                            git2::BranchType::Remote,
                        )
                        .ok()
                        .and_then(|b| b.get().peel_to_commit().ok())
                        .map(|c| c.id());

                    match (local_head, upstream) {
                        (Some(local), Some(up)) => {
                            // revwalk 从 local 到 up 之前的 commit 数
                            repo.revwalk()
                                .and_then(|mut walk| {
                                    walk.push(local)?;
                                    walk.hide(up)?;
                                    Ok(walk.count())
                                })
                                .unwrap_or(0)
                        }
                        _ => 0,
                    }
                } else {
                    0
                };

                let remote_url = repo
                    .find_remote(&self.config.remote_name)
                    .ok()
                    .and_then(|r| r.url().map(|s| s.to_string()));

                Ok(GitStatus {
                    mode: GitMode::Sync,
                    initialized: true,
                    current_branch,
                    remote_url,
                    last_commit,
                    unpushed_commits,
                })
            }
            Err(_) => Ok(GitStatus {
                mode: GitMode::Sync,
                initialized: false,
                current_branch: None,
                remote_url: self.config.remote_url.clone(),
                last_commit: None,
                unpushed_commits: 0,
            }),
        }
    }

    fn should_handle(&self, _snapshot_type: &timeflow_core::SnapshotType) -> bool {
        // 同步模式：处理所有快照
        true
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
    fn test_sync_mode_ensure_init_creates_git() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        mode.ensure_init(tmp.path()).unwrap();
        assert!(tmp.path().join(".git").exists());
        assert!(tmp.path().join(".gitignore").exists());
    }

    #[test]
    fn test_sync_mode_gitignore_contains_yunji() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        mode.ensure_init(tmp.path()).unwrap();
        let gitignore = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains(".yunji/"));
        assert!(gitignore.contains("node_modules/"));
    }

    #[test]
    fn test_sync_mode_idempotent_init() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        // 多次调用不应报错
        mode.ensure_init(tmp.path()).unwrap();
        mode.ensure_init(tmp.path()).unwrap();
        mode.ensure_init(tmp.path()).unwrap();
    }

    #[test]
    fn test_sync_mode_status_uninitialized() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        let status = mode.status(tmp.path()).unwrap();
        assert_eq!(status.mode, GitMode::Sync);
        assert!(!status.initialized);
    }

    #[test]
    fn test_sync_mode_status_initialized() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        mode.ensure_init(tmp.path()).unwrap();
        let status = mode.status(tmp.path()).unwrap();
        assert_eq!(status.mode, GitMode::Sync);
        assert!(status.initialized);
    }

    #[test]
    fn test_sync_mode_push_without_remote_fails() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        mode.ensure_init(tmp.path()).unwrap();
        let result = mode.push(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_mode_should_handle_all_types() {
        use timeflow_core::SnapshotType;
        let tmp = TempDir::new().unwrap();
        let config = make_config(tmp.path());
        let mode = SyncMode::new(config);

        assert!(mode.should_handle(&SnapshotType::Wip));
        assert!(mode.should_handle(&SnapshotType::Progress));
        assert!(mode.should_handle(&SnapshotType::Candidate));
        assert!(mode.should_handle(&SnapshotType::Release));
    }
}
