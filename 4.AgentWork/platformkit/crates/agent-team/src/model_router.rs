// 模型路由层（W7 M3.1 D3）
// 设计文档: docs/AGENT-TEAM-DESIGN.md § 3.1
//
// 目标：为每个角色绑定主模型 + 备用模型，主模型失败时自动故障转移。
//
// 路由策略：
//   1. 优先用主模型
//   2. 主模型失败（网络/超时/限流）→ 按顺序尝试备用模型
//   3. 所有模型都失败 → 返回 AllModelsFailed 错误
//
// 模型 ID 映射到 LlmEngine：
//   - "glm5.2" / "qwen3.7" / "claude-opus" → OpenAI 兼容 API
//   - "local-llama-3" / "qwen-coder" → Ollama 本地
//   - "mock-*" → Mock 引擎（测试用）

use crate::error::{Result, TeamError};
use crate::team_template::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use timeflow_ai::{
    AiConfig, LlmEngine, LlmMessage, LlmProvider, LlmResponse, MockEngine, OllamaEngine, OpenAiEngine,
    RoutingHint,
};

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型 ID（如 "glm5.2"）
    pub id: String,
    /// 提供方
    pub provider: LlmProvider,
    /// API base URL
    pub base_url: String,
    /// API key（OpenAI 必填，Ollama 可空，Mg 用 UM access_token）
    pub api_key: Option<String>,
    /// 模型名（实际传给 API 的，如 "gpt-4o-mini"）
    pub model_name: String,
    /// 超时（秒）
    pub timeout_secs: u64,
    /// 最大 token 数
    pub max_tokens: u32,
    /// 温度
    pub temperature: f32,
    /// 模型路由提示（仅 Mg provider 生效）
    #[serde(default)]
    pub routing_hint: RoutingHint,
}

impl ModelConfig {
    /// 创建 OpenAI 兼容模型配置
    pub fn openai(id: impl Into<String>, base_url: impl Into<String>, api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            provider: LlmProvider::OpenAi,
            base_url: base_url.into(),
            api_key: Some(api_key.into()),
            model_name: model_name.into(),
            timeout_secs: 60,
            max_tokens: 4096,
            temperature: 0.3,
            routing_hint: RoutingHint::Chat,
        }
    }

    /// 创建 MG（云集模型网关）模型配置
    ///
    /// - `api_key`: UM access_token（走 UM 钱包统一计费）
    /// - `model_name`: MG 侧模型名（如 "deepseek-coder" / "claude-3-opus"）
    /// - `hint`: 路由提示，MG BFF 据此选择最优模型
    pub fn mg(id: impl Into<String>, api_key: impl Into<String>, model_name: impl Into<String>, hint: RoutingHint) -> Self {
        Self {
            id: id.into(),
            provider: LlmProvider::Mg,
            base_url: "https://mg.yunjii.cn/v1".to_string(),
            api_key: Some(api_key.into()),
            model_name: model_name.into(),
            timeout_secs: 90,
            max_tokens: 8192,
            temperature: 0.3,
            routing_hint: hint,
        }
    }

    /// 创建 Ollama 本地模型配置
    pub fn ollama(id: impl Into<String>, base_url: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            provider: LlmProvider::Ollama,
            base_url: base_url.into(),
            api_key: None,
            model_name: model_name.into(),
            timeout_secs: 120,
            max_tokens: 4096,
            temperature: 0.3,
            routing_hint: RoutingHint::Chat,
        }
    }

    /// 创建 Mock 模型配置（测试用）
    pub fn mock(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            provider: LlmProvider::Mock,
            base_url: "mock://localhost".to_string(),
            api_key: None,
            model_name: "mock-model".to_string(),
            timeout_secs: 5,
            max_tokens: 256,
            temperature: 0.0,
            routing_hint: RoutingHint::Chat,
        }
    }

    /// 转为 AiConfig
    pub fn to_ai_config(&self) -> AiConfig {
        AiConfig {
            provider: self.provider,
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model_name.clone(),
            timeout: Duration::from_secs(self.timeout_secs),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            routing_hint: self.routing_hint,
        }
    }
}

/// 模型路由器
///
/// 管理模型 ID → ModelConfig 映射，为角色提供故障转移路由
#[derive(Debug, Clone)]
pub struct ModelRouter {
    /// 模型配置表
    models: HashMap<String, ModelConfig>,
}

impl ModelRouter {
    /// 创建空路由器
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// 注册模型
    pub fn register(&mut self, config: ModelConfig) {
        self.models.insert(config.id.clone(), config);
    }

    /// 批量注册
    pub fn register_all(&mut self, configs: Vec<ModelConfig>) {
        for c in configs {
            self.register(c);
        }
    }

    /// 获取模型配置
    pub fn get(&self, model_id: &str) -> Option<&ModelConfig> {
        self.models.get(model_id)
    }

    /// 列出所有已注册模型 ID
    pub fn list_models(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// 为角色创建引擎（按优先级尝试，第一个成功的为准）
    ///
    /// 返回 (引擎, 使用的模型 ID)
    pub fn create_engine_for_role(&self, role: &Role) -> Result<(Box<dyn LlmEngine>, String)> {
        let models = role.all_models();
        let mut attempts = 0;

        for model_id in &models {
            attempts += 1;
            match self.try_create_engine(model_id) {
                Ok(engine) => {
                    if attempts > 1 {
                        tracing::info!(
                            "角色 {} 故障转移到模型 {}（第 {} 次尝试）",
                            role.id, model_id, attempts
                        );
                    }
                    return Ok((engine, model_id.clone()));
                }
                Err(e) => {
                    tracing::warn!(
                        "角色 {} 的模型 {} 创建失败: {}",
                        role.id, model_id, e
                    );
                }
            }
        }

        Err(TeamError::AllModelsFailed {
            role: role.id.clone(),
            attempts,
        })
    }

    /// 尝试创建单个模型的引擎
    fn try_create_engine(&self, model_id: &str) -> Result<Box<dyn LlmEngine>> {
        let config = self.models.get(model_id).ok_or_else(|| {
            TeamError::Config(format!("模型 '{model_id}' 未注册"))
        })?;

        let ai_config = config.to_ai_config();
        let engine: Box<dyn LlmEngine> = match config.provider {
            LlmProvider::OpenAi => Box::new(OpenAiEngine::new(ai_config)?),
            LlmProvider::Mg => Box::new(OpenAiEngine::with_routing_hint(
                ai_config,
                Some(config.routing_hint),
            )?),
            LlmProvider::Ollama => Box::new(OllamaEngine::new(ai_config)?),
            LlmProvider::Mock => Box::new(MockEngine::conventional_commit()),
        };
        Ok(engine)
    }

    /// 为角色创建带故障转移的调用器
    pub fn create_caller(&self, role: &Role) -> Result<ModelCaller> {
        let (engine, model_id) = self.create_engine_for_role(role)?;
        Ok(ModelCaller {
            engine,
            model_id,
            fallback_models: role.model_fallback.clone(),
            router: self,
        })
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// 模型调用器（封装故障转移逻辑）
pub struct ModelCaller<'a> {
    /// 当前引擎
    engine: Box<dyn LlmEngine>,
    /// 当前使用的模型 ID
    model_id: String,
    /// 备用模型列表
    fallback_models: Vec<String>,
    /// 路由器引用
    router: &'a ModelRouter,
}

impl<'a> ModelCaller<'a> {
    /// 获取当前模型 ID
    pub fn current_model(&self) -> &str {
        &self.model_id
    }

    /// 调用 LLM（带故障转移）
    ///
    /// 主模型失败时自动切换到备用模型重试
    pub async fn chat(&mut self, messages: Vec<LlmMessage>) -> Result<LlmResponse> {
        // 尝试当前引擎
        match self.engine.chat(messages.clone()).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                tracing::warn!(
                    "模型 {} 调用失败: {}，尝试故障转移",
                    self.model_id, e
                );
            }
        }

        // 故障转移：尝试备用模型
        for fallback_id in &self.fallback_models {
            if fallback_id == &self.model_id {
                continue;
            }

            tracing::info!("故障转移到模型: {}", fallback_id);
            match self.router.try_create_engine(fallback_id) {
                Ok(new_engine) => {
                    self.engine = new_engine;
                    self.model_id = fallback_id.clone();
                }
                Err(e) => {
                    tracing::warn!("备用模型 {} 创建失败: {}", fallback_id, e);
                    continue;
                }
            }

            match self.engine.chat(messages.clone()).await {
                Ok(resp) => {
                    tracing::info!("故障转移成功，使用模型: {}", self.model_id);
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!("备用模型 {} 也失败: {}", self.model_id, e);
                }
            }
        }

        Err(TeamError::AllModelsFailed {
            role: self.model_id.clone(),
            attempts: 1 + self.fallback_models.len(),
        })
    }

    /// 便捷单轮对话
    pub async fn complete(&mut self, system: &str, user: &str) -> Result<String> {
        let messages = vec![
            LlmMessage::system(system),
            LlmMessage::user(user),
        ];
        let resp = self.chat(messages).await?;
        Ok(resp.content)
    }
}

/// 创建内置模型路由器（预注册常用模型）
///
/// 环境变量覆盖（M4.0 D7）：
/// - `LLM_GATEWAY_URL`：若设置，所有云端模型（OpenAI 兼容）的 base_url 改为该值
/// - `LLM_GATEWAY_TOKEN`：若设置，所有云端模型的 api_key 改为该值
///
/// 这样部署时只需设置两个环境变量，即可将所有云端模型走 LLM Gateway 反代，
/// 而无需逐个模型配置。Ollama 本地模型和 Mock 不受影响。
pub fn builtin_router() -> ModelRouter {
    let mut router = ModelRouter::new();

    // 读取 Gateway 环境变量（M4.0 D7）
    let gateway_url = std::env::var("LLM_GATEWAY_URL").ok().filter(|s| !s.is_empty());
    let gateway_token = std::env::var("LLM_GATEWAY_TOKEN").ok().filter(|s| !s.is_empty());

    // OpenAI 兼容模型（需用户填 API key，这里用占位符）
    router.register(ModelConfig::openai(
        "gpt-4o",
        "https://api.openai.com/v1",
        "",
        "gpt-4o",
    ));
    router.register(ModelConfig::openai(
        "gpt-4o-mini",
        "https://api.openai.com/v1",
        "",
        "gpt-4o-mini",
    ));

    // 智谱 GLM
    router.register(ModelConfig::openai(
        "glm5.2",
        "https://open.bigmodel.cn/api/paas/v4",
        "",
        "glm-4-plus",
    ));

    // 通义千问
    router.register(ModelConfig::openai(
        "qwen3.7",
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "",
        "qwen-max",
    ));
    router.register(ModelConfig::openai(
        "qwen-coder",
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "",
        "qwen-coder-plus",
    ));

    // MiniMax
    router.register(ModelConfig::openai(
        "minimax3",
        "https://api.minimax.chat/v1",
        "",
        "abab6.5-chat",
    ));

    // Claude（通过 Anthropic API 或代理）
    router.register(ModelConfig::openai(
        "claude-opus",
        "https://api.anthropic.com/v1",
        "",
        "claude-3-opus",
    ));

    // Ollama 本地模型
    router.register(ModelConfig::ollama(
        "local-llama-3",
        "http://localhost:11434",
        "llama3.2",
    ));
    router.register(ModelConfig::ollama(
        "local-qwen-coder",
        "http://localhost:11434",
        "qwen2.5-coder:7b",
    ));

    // 云集模型网关（MG）—— 走 UM 钱包统一计费 + 智能路由
    // api_key 留空，运行时由 UM SSO token 自动注入
    // MG BFF 根据 routing_hint 自动选择最优模型
    router.register(ModelConfig::mg(
        "mg-deepseek-coder",
        "",
        "deepseek-coder",
        RoutingHint::Code,
    ));
    router.register(ModelConfig::mg(
        "mg-claude",
        "",
        "claude-3-opus",
        RoutingHint::Reasoning,
    ));
    router.register(ModelConfig::mg(
        "mg-gpt4o",
        "",
        "gpt-4o",
        RoutingHint::Vision,
    ));
    router.register(ModelConfig::mg(
        "mg-qwen-chat",
        "",
        "qwen-max",
        RoutingHint::Chat,
    ));

    // Mock（测试用）
    router.register(ModelConfig::mock("mock-test"));

    // 应用 Gateway 环境变量覆盖（M4.0 D7）
    if gateway_url.is_some() || gateway_token.is_some() {
        apply_gateway_override(&mut router, gateway_url.as_deref(), gateway_token.as_deref());
    }

    router
}

/// 将路由器中所有云端模型（OpenAI 兼容）的 base_url / api_key 替换为 Gateway 配置。
/// Ollama 本地模型和 Mock 不受影响。
fn apply_gateway_override(
    router: &mut ModelRouter,
    gateway_url: Option<&str>,
    gateway_token: Option<&str>,
) {
    tracing::info!(
        "LLM Gateway 已启用：覆盖云端模型 base_url（{} 个模型）",
        router.models.len()
    );

    for (_, config) in router.models.iter_mut() {
        // 只覆盖 OpenAI 兼容模型和 MG 模型（云端），保留 Ollama 和 Mock
        if config.provider == LlmProvider::OpenAi || config.provider == LlmProvider::Mg {
            if let Some(url) = gateway_url {
                // 拼接模型 ID 作为路径后缀，例如 https://gateway.com/api/llm/zhipu
                // 文档约定：Gateway 路径 /api/llm/{provider}/chat/completions
                // 这里 base_url 设为 https://gateway.com/api/llm/{provider}
                // OpenAiEngine 会自动拼接 /chat/completions
                let trimmed = url.trim_end_matches('/');
                config.base_url = format!("{}/{}", trimmed, config.id);
            }
            if let Some(token) = gateway_token {
                config.api_key = Some(token.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_model() {
        let mut router = ModelRouter::new();
        let config = ModelConfig::mock("test-model");
        router.register(config);

        assert!(router.get("test-model").is_some());
        assert!(router.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_models() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::mock("m1"));
        router.register(ModelConfig::mock("m2"));
        router.register(ModelConfig::mock("m3"));

        let mut models = router.list_models();
        models.sort();
        assert_eq!(models, vec!["m1", "m2", "m3"]);
    }

    #[test]
    fn test_create_engine_mock() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::mock("mock-1"));

        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "mock-1".to_string(),
            model_fallback: vec![],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: None,
        };

        let (engine, model_id) = router.create_engine_for_role(&role).unwrap();
        assert_eq!(model_id, "mock-1");
        assert_eq!(engine.name(), "mock");
    }

    #[test]
    fn test_create_engine_fallback() {
        let mut router = ModelRouter::new();
        // 主模型未注册（会失败），备用模型是 mock
        router.register(ModelConfig::mock("backup-mock"));

        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "unregistered-model".to_string(),
            model_fallback: vec!["backup-mock".to_string()],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: None,
        };

        let (engine, model_id) = router.create_engine_for_role(&role).unwrap();
        assert_eq!(model_id, "backup-mock");
        assert_eq!(engine.name(), "mock");
    }

    #[test]
    fn test_create_engine_all_fail() {
        let router = ModelRouter::new();

        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "unregistered-1".to_string(),
            model_fallback: vec!["unregistered-2".to_string()],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: None,
        };

        let result = router.create_engine_for_role(&role);
        assert!(result.is_err());
        match result {
            Err(TeamError::AllModelsFailed { role, attempts }) => {
                assert_eq!(role, "test");
                assert_eq!(attempts, 2);
            }
            _ => panic!("期望 AllModelsFailed 错误"),
        }
    }

    #[tokio::test]
    async fn test_model_caller_success() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::mock("mock-1"));

        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "mock-1".to_string(),
            model_fallback: vec![],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: None,
        };

        let mut caller = router.create_caller(&role).unwrap();
        assert_eq!(caller.current_model(), "mock-1");

        let resp = caller.complete("system", "user").await.unwrap();
        assert!(!resp.is_empty());
    }

    #[tokio::test]
    async fn test_model_caller_fallback() {
        let mut router = ModelRouter::new();
        // 主模型是 OpenAI（无 API key 会失败），备用是 mock
        router.register(ModelConfig::openai(
            "openai-no-key",
            "https://api.openai.com/v1",
            "",
            "gpt-4o",
        ));
        router.register(ModelConfig::mock("mock-backup"));

        let role = Role {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            model: "mock-backup".to_string(), // 直接用 mock 主模型
            model_fallback: vec!["openai-no-key".to_string()],
            skills: vec![],
            knowledge: vec![],
            permissions: vec![],
            system_prompt: None,
        };

        let mut caller = router.create_caller(&role).unwrap();
        // 主模型是 mock，应直接成功
        let resp = caller.complete("s", "u").await.unwrap();
        assert!(!resp.is_empty());
        assert_eq!(caller.current_model(), "mock-backup");
    }

    #[test]
    fn test_builtin_router_has_models() {
        let router = builtin_router();
        let models = router.list_models();
        assert!(models.contains(&"gpt-4o".to_string()));
        assert!(models.contains(&"glm5.2".to_string()));
        assert!(models.contains(&"qwen3.7".to_string()));
        assert!(models.contains(&"local-llama-3".to_string()));
        assert!(models.contains(&"mg-deepseek-coder".to_string()));
        assert!(models.contains(&"mg-claude".to_string()));
        assert!(models.contains(&"mg-gpt4o".to_string()));
        assert!(models.contains(&"mg-qwen-chat".to_string()));
        assert!(models.contains(&"mock-test".to_string()));
    }

    #[test]
    fn test_model_config_mg() {
        let config = ModelConfig::mg("mg-test", "um-token", "deepseek-coder", RoutingHint::Code);
        assert_eq!(config.provider, LlmProvider::Mg);
        assert_eq!(config.base_url, "https://mg.yunjii.cn/v1");
        assert_eq!(config.api_key, Some("um-token".to_string()));
        assert_eq!(config.model_name, "deepseek-coder");
        assert_eq!(config.routing_hint, RoutingHint::Code);
    }

    #[test]
    fn test_model_config_to_ai_config() {
        let config = ModelConfig::openai("test", "https://example.com", "key", "model");
        let ai = config.to_ai_config();
        assert_eq!(ai.provider, LlmProvider::OpenAi);
        assert_eq!(ai.base_url, "https://example.com");
        assert_eq!(ai.api_key, Some("key".to_string()));
        assert_eq!(ai.model, "model");
    }

    #[test]
    fn test_model_config_ollama() {
        let config = ModelConfig::ollama("local", "http://localhost:11434", "llama3");
        assert_eq!(config.provider, LlmProvider::Ollama);
        assert_eq!(config.api_key, None);
        assert_eq!(config.model_name, "llama3");
    }

    #[test]
    fn test_model_config_mock() {
        let config = ModelConfig::mock("test");
        assert_eq!(config.provider, LlmProvider::Mock);
        assert_eq!(config.api_key, None);
    }

    #[test]
    fn test_apply_gateway_override_overwrites_openai_only() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::openai(
            "gpt-4o",
            "https://api.openai.com/v1",
            "",
            "gpt-4o",
        ));
        router.register(ModelConfig::mg(
            "mg-claude",
            "",
            "claude-3-opus",
            RoutingHint::Reasoning,
        ));
        router.register(ModelConfig::ollama(
            "local-llama",
            "http://localhost:11434",
            "llama3",
        ));
        router.register(ModelConfig::mock("mock-1"));

        apply_gateway_override(
            &mut router,
            Some("https://gateway.example.com/api/llm"),
            Some("gateway-token-xyz"),
        );

        // OpenAI 模型应被覆盖
        let gpt = router.get("gpt-4o").unwrap();
        assert_eq!(gpt.base_url, "https://gateway.example.com/api/llm/gpt-4o");
        assert_eq!(gpt.api_key, Some("gateway-token-xyz".to_string()));

        // MG 模型也应被覆盖
        let mg = router.get("mg-claude").unwrap();
        assert_eq!(mg.base_url, "https://gateway.example.com/api/llm/mg-claude");
        assert_eq!(mg.api_key, Some("gateway-token-xyz".to_string()));

        // Ollama 模型不应被覆盖
        let local = router.get("local-llama").unwrap();
        assert_eq!(local.base_url, "http://localhost:11434");
        assert_eq!(local.api_key, None);

        // Mock 不应被覆盖
        let mock = router.get("mock-1").unwrap();
        assert_eq!(mock.base_url, "mock://localhost");
    }

    #[test]
    fn test_apply_gateway_override_only_token() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::openai("gpt-4o", "https://api.openai.com/v1", "", "gpt-4o"));

        // 只传 token，不传 url → base_url 不变，api_key 被覆盖
        apply_gateway_override(&mut router, None, Some("token-only"));

        let gpt = router.get("gpt-4o").unwrap();
        assert_eq!(gpt.base_url, "https://api.openai.com/v1");
        assert_eq!(gpt.api_key, Some("token-only".to_string()));
    }

    #[test]
    fn test_apply_gateway_override_trims_trailing_slash() {
        let mut router = ModelRouter::new();
        router.register(ModelConfig::openai("glm5.2", "https://original.com", "", "glm-4-plus"));

        apply_gateway_override(
            &mut router,
            Some("https://gateway.example.com/api/llm/"),
            None,
        );

        let glm = router.get("glm5.2").unwrap();
        assert_eq!(glm.base_url, "https://gateway.example.com/api/llm/glm5.2");
    }
}
