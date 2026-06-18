// AI 配置
// 设计文档: docs/TIMEFLOW-DESIGN.md § 4.4

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LLM 提供方
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    /// OpenAI（GPT 系列）
    OpenAi,
    /// Ollama（本地模型，隐身模式首选）
    Ollama,
    /// Mock（测试用）
    Mock,
}

impl Default for LlmProvider {
    fn default() -> Self {
        Self::Ollama
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAi => write!(f, "openai"),
            Self::Ollama => write!(f, "ollama"),
            Self::Mock => write!(f, "mock"),
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = crate::error::AiError;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "gpt" => Ok(Self::OpenAi),
            "ollama" | "local" => Ok(Self::Ollama),
            "mock" | "test" => Ok(Self::Mock),
            _ => Err(crate::error::AiError::Config(format!("未知 LLM 提供方: {}", s))),
        }
    }
}

/// AI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// LLM 提供方
    pub provider: LlmProvider,
    /// API base URL
    ///
    /// - OpenAI: 默认 https://api.openai.com/v1
    /// - Ollama: 默认 http://localhost:11434
    pub base_url: String,
    /// API key（OpenAI 必填，Ollama 可空）
    pub api_key: Option<String>,
    /// 模型名
    ///
    /// - OpenAI: gpt-4o-mini / gpt-4o
    /// - Ollama: qwen2.5-coder:7b / llama3.2
    pub model: String,
    /// 请求超时
    #[serde(with = "duration_secs")]
    pub timeout: Duration,
    /// 最大 token 数
    pub max_tokens: u32,
    /// 温度（0.0-1.0，越低越确定性）
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            model: "qwen2.5-coder:7b".to_string(),
            timeout: Duration::from_secs(60),
            max_tokens: 256,
            temperature: 0.3,
        }
    }
}

impl AiConfig {
    /// 创建 OpenAI 默认配置
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: Some(api_key.into()),
            model: "gpt-4o-mini".to_string(),
            ..Default::default()
        }
    }

    /// 创建 Ollama 默认配置
    pub fn ollama() -> Self {
        Self::default()
    }

    /// 创建 Mock 配置（测试用）
    pub fn mock() -> Self {
        Self {
            provider: LlmProvider::Mock,
            ..Default::default()
        }
    }
}

// Duration 用秒序列化
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_secs().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_secs(u64::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_parse() {
        assert_eq!("openai".parse::<LlmProvider>().unwrap(), LlmProvider::OpenAi);
        assert_eq!("ollama".parse::<LlmProvider>().unwrap(), LlmProvider::Ollama);
        assert_eq!("mock".parse::<LlmProvider>().unwrap(), LlmProvider::Mock);
        assert!("invalid".parse::<LlmProvider>().is_err());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(LlmProvider::OpenAi.to_string(), "openai");
        assert_eq!(LlmProvider::Ollama.to_string(), "ollama");
        assert_eq!(LlmProvider::Mock.to_string(), "mock");
    }

    #[test]
    fn test_config_default_is_ollama() {
        let cfg = AiConfig::default();
        assert_eq!(cfg.provider, LlmProvider::Ollama);
        assert_eq!(cfg.model, "qwen2.5-coder:7b");
    }

    #[test]
    fn test_config_openai() {
        let cfg = AiConfig::openai("sk-test");
        assert_eq!(cfg.provider, LlmProvider::OpenAi);
        assert_eq!(cfg.api_key, Some("sk-test".to_string()));
        assert_eq!(cfg.model, "gpt-4o-mini");
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = AiConfig {
            provider: LlmProvider::OpenAi,
            base_url: "https://example.com".to_string(),
            api_key: Some("key".to_string()),
            model: "gpt-4".to_string(),
            timeout: Duration::from_secs(30),
            max_tokens: 512,
            temperature: 0.5,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, LlmProvider::OpenAi);
        assert_eq!(parsed.timeout, Duration::from_secs(30));
        assert_eq!(parsed.max_tokens, 512);
    }
}
