// Git 适配器 trait
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7
//
// 三种模式各自实现此 trait：
//   - StealthMode: 所有方法 no-op
//   - SyncMode: 每次 snapshot 后镜像为 git commit
//   - ReleaseMode: 只有 Release 类型快照才推送到 git

use std::path::Path;
use timeflow_core::{Snapshot, SnapshotType};

use crate::config::GitMode;
use crate::error::Result;

/// Git 适配器状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitStatus {
    /// 当前模式
    pub mode: GitMode,
    /// Git 仓库是否已初始化
    pub initialized: bool,
    /// 当前分支名（git 仓库的分支，可能不同于 TimeFlow 分支）
    pub current_branch: Option<String>,
    /// 远程仓库 URL
    pub remote_url: Option<String>,
    /// 最近一次 commit hash
    pub last_commit: Option<String>,
    /// 待推送的 commit 数
    pub unpushed_commits: usize,
}

/// Git 适配器 trait
///
/// 所有 TimeFlow → Git 的同步操作都通过此 trait 抽象。
/// 不同模式有不同实现策略。
pub trait GitAdapter: Send + Sync {
    /// 返回当前模式
    fn mode(&self) -> GitMode;

    /// 初始化 Git 仓库（如果尚未初始化）
    ///
    /// - Stealth 模式：no-op
    /// - Sync/Release 模式：如果 .git 不存在，git init + 配置 author + 添加 remote
    fn ensure_init(&self, repo_path: &Path) -> Result<()>;

    /// 处理 TimeFlow 快照
    ///
    /// - Stealth 模式：no-op
    /// - Sync 模式：将快照内容写入 git 工作区并 commit
    /// - Release 模式：只有 SnapshotType::Release 才 commit + tag + push
    ///
    /// 参数：
    /// - `snapshot`: TimeFlow 快照
    /// - `timeflow`: TimeFlow 引擎（用于读取 tree/blob 内容）
    /// - `repo_path`: 工作区路径
    fn on_snapshot(
        &self,
        snapshot: &Snapshot,
        timeflow: &timeflow_core::TimeFlow,
        repo_path: &Path,
    ) -> Result<()>;

    /// 推送到远程（手动触发）
    ///
    /// - Stealth 模式：返回错误
    /// - Sync/Release 模式：git push
    fn push(&self, repo_path: &Path) -> Result<()>;

    /// 查询状态
    fn status(&self, repo_path: &Path) -> Result<GitStatus>;

    /// 是否应该处理此快照类型
    ///
    /// 默认实现：所有快照都处理（Sync 模式）
    /// Release 模式覆盖此方法，只处理 Release 类型
    fn should_handle(&self, snapshot_type: &SnapshotType) -> bool {
        let _ = snapshot_type;
        true
    }
}
