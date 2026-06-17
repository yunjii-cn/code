// TimeFlow 分支操作
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.4
//
// W1: trait 定义
// W2: 实现分支创建/切换/合并

use crate::error::Result;
use crate::types::*;

/// 分支操作 trait
pub trait BranchOps {
    /// 创建分支（O(1)，仅创建指针）
    fn create_branch(&self, name: &BranchName, from: &SnapshotId) -> Result<Branch>;

    /// 删除分支
    fn delete_branch(&self, name: &BranchName) -> Result<()>;

    /// 切换分支
    fn switch_branch(&self, name: &BranchName) -> Result<()>;

    /// 列出所有分支
    fn list_branches(&self) -> Result<Vec<Branch>>;

    /// 获取当前分支
    fn current_branch(&self) -> Result<BranchName>;
}
