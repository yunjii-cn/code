// agent-team 错误类型

use thiserror::Error;

/// agent-team 错误
#[derive(Debug, Error)]
pub enum TeamError {
    /// YAML 解析失败
    #[error("YAML 解析失败: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON 解析失败
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// TimeFlow AI 错误
    #[error("AI 引擎错误: {0}")]
    Ai(#[from] timeflow_ai::AiError),

    /// HTTP 请求失败
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    /// 角色未找到
    #[error("角色未找到: {0}")]
    RoleNotFound(String),

    /// 模型未配置
    #[error("模型未配置: 角色 {role} 没有可用模型")]
    NoModel { role: String },

    /// 所有模型都失败
    #[error("角色 {role} 的所有模型都调用失败（尝试了 {attempts} 个）")]
    AllModelsFailed { role: String, attempts: usize },

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

/// 结果类型
pub type Result<T> = std::result::Result<T, TeamError>;
