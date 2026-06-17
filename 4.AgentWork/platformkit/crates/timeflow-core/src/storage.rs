// TimeFlow 存储引擎
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.3
//
// W1: 骨架实现，定义接口
// W2: 实现内容寻址存储（SHA-256 + 压缩 + 去重）

use crate::error::{Error, Result};
use crate::types::*;
use std::path::{Path, PathBuf};

/// 存储引擎 - 管理所有持久化数据
///
/// 存储布局:
/// ```text
/// .yunji/timeflow/
/// ├── snapshots/    快照元数据
/// ├── blobs/        内容寻址 blob（压缩）
/// ├── trees/        内容寻址树
/// ├── branches/     分支指针
/// └── config.toml   配置
/// ```
pub struct Storage {
    /// 仓库根路径
    repo_path: PathBuf,
    /// TimeFlow 数据目录
    timeflow_dir: PathBuf,
}

impl Storage {
    /// 创建存储引擎实例
    pub fn new(repo_path: &Path) -> Result<Self> {
        let timeflow_dir = repo_path.join(".yunji").join("timeflow");

        // W1: 仅检查路径，W2 创建完整目录结构
        if !timeflow_dir.exists() {
            tracing::debug!("TimeFlow 目录不存在，W2 将自动创建: {}", timeflow_dir.display());
        }

        Ok(Self {
            repo_path: repo_path.to_path_buf(),
            timeflow_dir,
        })
    }

    /// 获取 TimeFlow 数据目录
    pub fn timeflow_dir(&self) -> &Path {
        &self.timeflow_dir
    }

    /// 获取仓库根路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    // ===== Blob 操作（W2 实现）=====

    /// 存储 Blob（内容寻址）
    ///
    /// W2 实现:
    /// 1. 计算 SHA-256
    /// 2. 压缩内容
    /// 3. 写入 blobs/{blob_id}
    /// 4. 如果已存在则跳过（去重）
    pub fn store_blob(&self, _content: &[u8]) -> Result<BlobId> {
        // W1: 占位
        Err(Error::Other("W2 待实现: store_blob".to_string()))
    }

    /// 读取 Blob
    pub fn read_blob(&self, _id: &BlobId) -> Result<Blob> {
        Err(Error::Other("W2 待实现: read_blob".to_string()))
    }

    // ===== Tree 操作（W2 实现）=====

    /// 存储 Tree
    pub fn store_tree(&self, _tree: &Tree) -> Result<TreeId> {
        Err(Error::Other("W2 待实现: store_tree".to_string()))
    }

    /// 读取 Tree
    pub fn read_tree(&self, _id: &TreeId) -> Result<Tree> {
        Err(Error::Other("W2 待实现: read_tree".to_string()))
    }

    // ===== Snapshot 操作（W2 实现）=====

    /// 存储 Snapshot 元数据
    pub fn store_snapshot(&self, _snapshot: &Snapshot) -> Result<()> {
        Err(Error::Other("W2 待实现: store_snapshot".to_string()))
    }

    /// 读取 Snapshot
    pub fn read_snapshot(&self, _id: &SnapshotId) -> Result<Snapshot> {
        Err(Error::Other("W2 待实现: read_snapshot".to_string()))
    }

    /// 列出所有快照
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        Err(Error::Other("W2 待实现: list_snapshots".to_string()))
    }
}
