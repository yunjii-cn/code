// TimeFlow 错误类型

use thiserror::Error;

/// TimeFlow 错误
#[derive(Debug, Error)]
pub enum Error {
    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    /// 快照不存在
    #[error("快照不存在: {0}")]
    SnapshotNotFound(String),

    /// 分支不存在
    #[error("分支不存在: {0}")]
    BranchNotFound(String),

    /// 仓库未初始化
    #[error("仓库未初始化: {0}")]
    NotInitialized(String),

    /// 校验和不匹配
    #[error("校验和不匹配: 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

/// TimeFlow 结果类型
pub type Result<T> = std::result::Result<T, Error>;
