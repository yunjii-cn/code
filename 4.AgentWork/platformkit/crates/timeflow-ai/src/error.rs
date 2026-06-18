// timeflow-ai 错误类型

use thiserror::Error;

/// AI 层错误
#[derive(Debug, Error)]
pub enum AiError {
    /// HTTP 请求失败
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON 解析失败
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),

    /// TimeFlow 错误
    #[error("TimeFlow 错误: {0}")]
    Timeflow(#[from] timeflow_core::Error),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// LLM 返回为空
    #[error("LLM 返回为空")]
    EmptyResponse,

    /// LLM 响应格式错误
    #[error("LLM 响应格式错误: {0}")]
    InvalidResponse(String),

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// API key 未配置
    #[error("API key 未配置")]
    MissingApiKey,

    /// LLM 调用超时
    #[error("LLM 调用超时（{timeout_ms}ms）")]
    Timeout {
        /// 超时时间（毫秒）
        timeout_ms: u64,
    },

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

/// 结果类型
pub type Result<T> = std::result::Result<T, AiError>;
