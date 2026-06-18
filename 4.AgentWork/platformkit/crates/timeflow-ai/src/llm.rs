// LLM 引擎抽象 + 三种实现（OpenAI / Ollama / Mock）
// 设计文档: docs/TIMEFLOW-DESIGN.md § 4.4
//
// 隐身模式 → Ollama 本地模型（代码不上云）
// 同步/发布模式 → OpenAI 云端模型（更快更准）
// 测试 → Mock 引擎（不调用真实 API）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::{AiConfig, LlmProvider};
use crate::error::{AiError, Result};

/// LLM 消息（chat completion 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// 角色：system / user / assistant
    pub role: String,
    /// 内容
    pub content: String,
}

impl LlmMessage {
    /// 创建 system 消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// 创建 user 消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 生成的文本
    pub content: String,
    /// 使用的 token 数（可选）
    pub total_tokens: Option<u32>,
}

/// LLM 引擎 trait
///
/// 所有 LLM 调用通过此 trait 抽象，便于切换提供方。
#[async_trait]
pub trait LlmEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;

    /// 聊天补全
    ///
    /// 参数：
    /// - `messages`: 消息列表（system + user）
    ///
    /// 返回生成的文本
    async fn chat(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse>;

    /// 简单单轮对话（便捷方法）
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let messages = vec![
            LlmMessage::system(system),
            LlmMessage::user(user),
        ];
        let resp = self.chat(messages).await?;
        Ok(resp.content)
    }
}

// ===== OpenAI 引擎 =====

/// OpenAI Chat Completions 引擎
#[derive(Debug)]
pub struct OpenAiEngine {
    client: reqwest::Client,
    config: AiConfig,
}

impl OpenAiEngine {
    /// 创建 OpenAI 引擎
    pub fn new(config: AiConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or(AiError::MissingApiKey)?;

        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                        .map_err(|e| AiError::Config(format!("无效 API key: {}", e)))?,
                );
                headers
            })
            .build()?;

        Ok(Self { client, config })
    }
}

#[async_trait]
impl LlmEngine for OpenAiEngine {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(AiError::InvalidResponse(format!(
                "OpenAI API 返回 {}: {}",
                status, text
            )));
        }

        let value: serde_json::Value = serde_json::from_str(&text)?;
        let content = value
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AiError::InvalidResponse(format!("无法解析响应: {}", text)))?
            .to_string();

        let total_tokens = value
            .get("usage")
            .and_then(|u| u.get("total_tokens"))
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);

        Ok(LlmResponse {
            content,
            total_tokens,
        })
    }
}

// ===== Ollama 引擎 =====

/// Ollama 本地模型引擎
#[derive(Debug)]
pub struct OllamaEngine {
    client: reqwest::Client,
    config: AiConfig,
}

impl OllamaEngine {
    /// 创建 Ollama 引擎
    pub fn new(config: AiConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()?;
        Ok(Self { client, config })
    }
}

#[async_trait]
impl LlmEngine for OllamaEngine {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn chat(&self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens,
            }
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(AiError::InvalidResponse(format!(
                "Ollama API 返回 {}: {}",
                status, text
            )));
        }

        let value: serde_json::Value = serde_json::from_str(&text)?;
        let content = value
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AiError::InvalidResponse(format!("无法解析响应: {}", text)))?
            .to_string();

        let total_tokens = value
            .get("eval_count")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);

        Ok(LlmResponse {
            content,
            total_tokens,
        })
    }
}

// ===== Mock 引擎（测试用）=====

/// Mock 引擎：返回预设的固定响应
pub struct MockEngine {
    /// 预设响应
    pub response: String,
}

impl MockEngine {
    /// 创建 Mock 引擎，返回指定内容
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }

    /// 创建返回 Conventional Commits 格式的 Mock
    pub fn conventional_commit() -> Self {
        Self::new("feat: 添加用户登录功能".to_string())
    }
}

impl Default for MockEngine {
    fn default() -> Self {
        Self::new("mock response")
    }
}

#[async_trait]
impl LlmEngine for MockEngine {
    fn name(&self) -> &str {
        "mock"
    }

    async fn chat(&self, _messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: self.response.clone(),
            total_tokens: Some(42),
        })
    }
}

// ===== 工厂函数 =====

/// 根据配置创建对应的 LLM 引擎
pub fn create_engine(config: AiConfig) -> Result<Box<dyn LlmEngine>> {
    match config.provider {
        LlmProvider::OpenAi => Ok(Box::new(OpenAiEngine::new(config)?)),
        LlmProvider::Ollama => Ok(Box::new(OllamaEngine::new(config)?)),
        LlmProvider::Mock => Ok(Box::new(MockEngine::conventional_commit())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_engine_chat() {
        let engine = MockEngine::new("hello world");
        let messages = vec![
            LlmMessage::system("you are a helper"),
            LlmMessage::user("hi"),
        ];
        let resp = engine.chat(messages).await.unwrap();
        assert_eq!(resp.content, "hello world");
        assert_eq!(resp.total_tokens, Some(42));
    }

    #[tokio::test]
    async fn test_mock_engine_complete() {
        let engine = MockEngine::new("test response");
        let result = engine.complete("system", "user").await.unwrap();
        assert_eq!(result, "test response");
    }

    #[tokio::test]
    async fn test_mock_engine_conventional_commit() {
        let engine = MockEngine::conventional_commit();
        let result = engine.complete("s", "u").await.unwrap();
        assert!(result.starts_with("feat:"));
    }

    #[test]
    fn test_create_engine_mock() {
        let config = AiConfig::mock();
        let engine = create_engine(config).unwrap();
        assert_eq!(engine.name(), "mock");
    }

    #[test]
    fn test_create_engine_ollama() {
        let config = AiConfig::ollama();
        let engine = create_engine(config).unwrap();
        assert_eq!(engine.name(), "ollama");
    }

    #[test]
    fn test_openai_engine_requires_api_key() {
        let mut config = AiConfig::openai("sk-test");
        config.api_key = None;
        let result = OpenAiEngine::new(config);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiError::MissingApiKey => {}
            _ => panic!("期望 MissingApiKey 错误"),
        }
    }

    #[test]
    fn test_llm_message_helpers() {
        let sys = LlmMessage::system("you are a bot");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "you are a bot");

        let user = LlmMessage::user("hello");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, "hello");
    }
}
