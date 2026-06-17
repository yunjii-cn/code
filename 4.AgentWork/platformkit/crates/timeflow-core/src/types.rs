// TimeFlow 核心数据类型
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.2

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 快照 ID（内容哈希）
pub type SnapshotId = String;

/// Blob ID（内容哈希）
pub type BlobId = String;

/// Tree ID（内容哈希）
pub type TreeId = String;

/// 分支名
pub type BranchName = String;

/// 快照（Snapshot）= Git 的 commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// 快照 ID（内容哈希）
    pub id: SnapshotId,
    /// 父快照 ID（形成链）
    pub parent: Option<SnapshotId>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 内容寻址树根
    pub tree: TreeId,
    /// 作者
    pub author: String,
    /// 元数据
    pub metadata: SnapshotMetadata,
}

/// 快照元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// 触发类型
    pub trigger: TriggerType,
    /// AI 生成的变更摘要
    pub ai_summary: Option<String>,
    /// AI 生成的 commit message
    pub ai_commit_msg: Option<String>,
    /// 构建状态
    pub build_status: BuildStatus,
    /// 快照类型
    pub snapshot_type: SnapshotType,
    /// 标签
    pub tags: Vec<String>,
}

/// 触发类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// 文件变化触发
    FileChange,
    /// 手动触发
    Manual,
    /// 定时触发
    Schedule,
    /// 工作流触发
    Workflow,
}

/// 构建状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    /// 可编译可测试通过
    Green,
    /// 可编译但测试失败
    Yellow,
    /// 编译失败
    Red,
    /// 未知（未构建）
    Unknown,
}

/// 快照类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    /// WIP 快照（改动小/未完成，7天后自动清理）
    Wip,
    /// 进度快照（有意义但非正式）
    Progress,
    /// 候选版本（编译通过+测试通过+改动充分）
    Candidate,
    /// 正式版本（用户确认/触发词）
    Release,
}

/// 内容寻址树
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// 树 ID
    pub id: TreeId,
    /// 文件路径 → Blob ID 映射
    pub entries: HashMap<PathBuf, BlobId>,
}

/// Blob（内容寻址的数据块）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    /// Blob ID（SHA-256）
    pub id: BlobId,
    /// 实际内容
    pub content: Vec<u8>,
}

/// 分支指针
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// 分支名
    pub name: BranchName,
    /// 指向的最新快照 ID
    pub head: SnapshotId,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 文件变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// 文件路径
    pub path: PathBuf,
    /// 变更状态
    pub status: ChangeStatus,
    /// 新增行数
    pub additions: u32,
    /// 删除行数
    pub deletions: u32,
}

/// 变更状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    /// 新增
    Added,
    /// 修改
    Modified,
    /// 删除
    Deleted,
}

/// 快照差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// 源快照 ID
    pub from: SnapshotId,
    /// 目标快照 ID
    pub to: SnapshotId,
    /// 文件变更列表
    pub files: Vec<FileChange>,
}
