// TimeFlow 本地版本控制引擎
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3
//
// W1 阶段: 定义核心数据结构和 trait，提供编译通过的骨架
// W2 阶段: 实现内容寻址存储 + 快照链 + 回滚 + 分支

#![warn(missing_docs)]

mod error;
mod types;
mod storage;
mod snapshot;
mod branch;

pub use error::Error;
pub use types::*;
pub use storage::Storage;
pub use snapshot::SnapshotOps;
pub use branch::BranchOps;

/// TimeFlow 引擎主入口
///
/// 所有版本控制操作通过此结构体发起。
/// W2 将实现完整的初始化逻辑（打开/创建仓库）。
pub struct TimeFlow {
    /// 存储引擎
    storage: Storage,
    /// 仓库路径
    repo_path: std::path::PathBuf,
}

impl TimeFlow {
    /// 初始化 TimeFlow 引擎
    ///
    /// W2 实现: 打开或创建 `.yunji/timeflow/` 目录结构
    pub fn new(repo_path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        // W1: 占位实现，W2 填充真实逻辑
        let repo_path = repo_path.as_ref().to_path_buf();
        let storage = Storage::new(&repo_path)?;

        tracing::info!("TimeFlow 引擎初始化 (W1 骨架): {}", repo_path.display());

        Ok(Self { storage, repo_path })
    }

    /// 获取仓库路径
    pub fn repo_path(&self) -> &std::path::Path {
        &self.repo_path
    }
}
