// TimeFlow 快照操作
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.4
//
// W1: trait 定义
// W2: 实现快照/回滚/diff

use crate::error::Result;
use crate::types::*;

/// 快照操作 trait
///
/// 所有快照相关操作通过此 trait 定义。
/// W2 由 TimeFlow 实现。
pub trait SnapshotOps {
    /// 创建快照
    ///
    /// 扫描工作区变更文件，生成新快照。
    /// - `message`: 可选手动消息（不传则 AI 生成）
    /// - 返回新快照 ID
    fn snapshot(&self, message: Option<&str>) -> Result<SnapshotId>;

    /// 回滚到指定快照
    ///
    /// 恢复工作区到指定快照状态。
    /// **重要**: 保留回滚前的历史（不覆盖），而是创建新快照。
    fn rollback(&self, target: &SnapshotId) -> Result<SnapshotId>;

    /// 计算两快照差异
    fn diff(&self, from: &SnapshotId, to: &SnapshotId) -> Result<SnapshotDiff>;

    /// 获取快照详情
    fn get_snapshot(&self, id: &SnapshotId) -> Result<Snapshot>;

    /// 列出快照（支持过滤）
    fn list_snapshots(&self, filter: Option<SnapshotFilter>) -> Result<Vec<Snapshot>>;
}

/// 快照过滤条件
#[derive(Debug, Clone, Default)]
pub struct SnapshotFilter {
    /// 按类型过滤
    pub snapshot_type: Option<SnapshotType>,
    /// 按构建状态过滤
    pub build_status: Option<BuildStatus>,
    /// 限制数量
    pub limit: Option<usize>,
}
