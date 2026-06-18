// aw-git 错误类型

use thiserror::Error;

/// aw-git 错误
#[derive(Debug, Error)]
pub enum GitError {
    /// libgit2 错误
    #[error("Git 错误: {0}")]
    Git(#[from] git2::Error),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// TimeFlow 错误
    #[error("TimeFlow 错误: {0}")]
    Timeflow(#[from] timeflow_core::Error),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    /// 仓库未初始化
    #[error("Git 仓库未初始化: {0}")]
    NotInitialized(String),

    /// 模式不匹配
    #[error("模式不支持此操作: 当前={current}, 期望={expected}")]
    ModeMismatch {
        /// 当前模式
        current: String,
        /// 期望模式
        expected: String,
    },

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

/// 结果类型
pub type Result<T> = std::result::Result<T, GitError>;
