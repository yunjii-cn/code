// Embedding 引擎抽象 + 索引（W6 M2.2 D2）
// 设计文档: docs/ROADMAP.md § W6 M2.2 D2
//
// 目标：为快照的 commit msg / ai_summary 生成向量索引，支持语义搜索。
//
// 实现策略：
//   - OllamaEmbeddingEngine: 调用本地 Ollama /api/embeddings（bge-small / nomic-embed-text）
//   - MockEmbeddingEngine:   基于哈希的确定性伪向量（测试用，不调 API）
//   - OpenAIEmbeddingEngine: 可选（暂未实现，留接口）
//
// 向量存储：内存 HashMap<snapshot_id, EmbeddingVector>
// 相似度：余弦相似度（cosine similarity）

use crate::config::{AiConfig, LlmProvider};
use crate::error::{AiError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Embedding 向量
pub type EmbeddingVector = Vec<f32>;

/// Embedding 引擎 trait
#[async_trait]
pub trait EmbeddingEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;

    /// 向量维度
    fn dim(&self) -> usize;

    /// 为文本生成 embedding
    async fn embed(&self, text: &str) -> Result<EmbeddingVector>;

    /// 批量生成
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingVector>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t).await?);
        }
        Ok(out)
    }
}

// ===== Ollama Embedding 引擎 =====

/// Ollama 本地 embedding 引擎
///
/// 调用 Ollama `/api/embeddings` 端点
/// 推荐模型：`bge-small`（轻量）/ `nomic-embed-text`（多语言）
pub struct OllamaEmbeddingEngine {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaEmbeddingEngine {
    /// 创建 Ollama embedding 引擎
    pub fn new(config: &AiConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()?;
        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            // embedding 模型与 chat 模型不同，使用专门的 embedding 模型
            model: "bge-small".to_string(),
        })
    }

    /// 指定模型创建
    pub fn with_model(config: &AiConfig, model: impl Into<String>) -> Result<Self> {
        let mut engine = Self::new(config)?;
        engine.model = model.into();
        Ok(engine)
    }
}

#[async_trait]
impl EmbeddingEngine for OllamaEmbeddingEngine {
    fn name(&self) -> &str {
        "ollama-embedding"
    }

    fn dim(&self) -> usize {
        // bge-small: 384 维
        // nomic-embed-text: 768 维
        384
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingVector> {
        let url = format!("{}/api/embeddings", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": text,
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text_resp = resp.text().await?;

        if !status.is_success() {
            return Err(AiError::InvalidResponse(format!(
                "Ollama embedding API 返回 {}: {}",
                status, text_resp
            )));
        }

        let value: serde_json::Value = serde_json::from_str(&text_resp)?;
        let embedding = value
            .get("embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| {
                AiError::InvalidResponse(format!("无法解析 embedding 响应: {}", text_resp))
            })?;

        let vector: EmbeddingVector = embedding
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if vector.is_empty() {
            return Err(AiError::EmptyResponse);
        }

        Ok(vector)
    }
}

// ===== Mock Embedding 引擎（测试用）=====

/// Mock embedding 引擎：基于 FNV-1a 哈希生成确定性伪向量
///
/// 不调用真实 API，向量由文本哈希派生，保证：
///   1. 相同文本 → 相同向量
///   2. 相似文本 → 相似向量（共享词的哈希位会重叠）
///   3. 不同文本 → 不同向量
///
/// 维度固定 64（测试够用，避免内存浪费）
pub struct MockEmbeddingEngine {
    dim: usize,
}

impl MockEmbeddingEngine {
    /// 创建 Mock 引擎（默认 64 维）
    pub fn new() -> Self {
        Self { dim: 64 }
    }

    /// 指定维度创建
    pub fn with_dim(dim: usize) -> Self {
        Self { dim }
    }

    /// FNV-1a 哈希
    fn hash(text: &str, seed: u32) -> u32 {
        let mut h = seed;
        for b in text.as_bytes() {
            h = h.wrapping_mul(0x01000193);
            h ^= *b as u32;
        }
        h
    }
}

impl Default for MockEmbeddingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingEngine for MockEmbeddingEngine {
    fn name(&self) -> &str {
        "mock-embedding"
    }

    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, text: &str) -> Result<EmbeddingVector> {
        // 对文本的每个 token 哈希，累加到对应维度（类似 hashing trick）
        let mut vector = vec![0.0f32; self.dim];
        let tokens: Vec<&str> = text.split_whitespace().collect();

        for token in &tokens {
            let h = Self::hash(token, 0x811c9dc5) as usize;
            let idx = h % self.dim;
            vector[idx] += 1.0;
        }

        // L2 归一化
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vector.iter_mut() {
                *v /= norm;
            }
        }

        Ok(vector)
    }
}

// ===== 工厂函数 =====

/// 根据配置创建 embedding 引擎
pub fn create_embedding_engine(config: &AiConfig) -> Result<Box<dyn EmbeddingEngine>> {
    match config.provider {
        LlmProvider::Ollama => Ok(Box::new(OllamaEmbeddingEngine::new(config)?)),
        LlmProvider::OpenAi => {
            // OpenAI embedding 暂未实现，回退到 Mock
            tracing::warn!("OpenAI embedding 暂未实现，使用 Mock 引擎");
            Ok(Box::new(MockEmbeddingEngine::new()))
        }
        LlmProvider::Mock => Ok(Box::new(MockEmbeddingEngine::new())),
    }
}

// ===== 相似度计算 =====

/// 计算余弦相似度
///
/// 返回值范围 [-1.0, 1.0]，越接近 1 越相似
pub fn cosine_similarity(a: &EmbeddingVector, b: &EmbeddingVector) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|v| v * v).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Embedding 索引（内存版）
///
/// 存储 snapshot_id → embedding 的映射，支持相似度查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingIndex {
    /// snapshot_id → embedding
    pub entries: HashMap<String, EmbeddingVector>,
    /// 维度
    pub dim: usize,
}

impl EmbeddingIndex {
    /// 创建空索引
    pub fn new(dim: usize) -> Self {
        Self {
            entries: HashMap::new(),
            dim,
        }
    }

    /// 添加/更新 embedding
    pub fn upsert(&mut self, snapshot_id: impl Into<String>, embedding: EmbeddingVector) {
        self.entries.insert(snapshot_id.into(), embedding);
    }

    /// 移除
    pub fn remove(&mut self, snapshot_id: &str) -> Option<EmbeddingVector> {
        self.entries.remove(snapshot_id)
    }

    /// 查询最相似的 top-k
    pub fn search(&self, query: &EmbeddingVector, k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|(id, emb)| (id.clone(), cosine_similarity(query, emb)))
            .collect();

        // 按相似度降序
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }

    /// 索引大小
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embed_deterministic() {
        let engine = MockEmbeddingEngine::new();
        let v1 = engine.embed("hello world").await.unwrap();
        let v2 = engine.embed("hello world").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn test_mock_embed_dim() {
        let engine = MockEmbeddingEngine::with_dim(128);
        let v = engine.embed("test").await.unwrap();
        assert_eq!(v.len(), 128);
    }

    #[tokio::test]
    async fn test_mock_embed_normalized() {
        let engine = MockEmbeddingEngine::new();
        let v = engine.embed("some text here").await.unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "向量应已归一化，norm={}", norm);
    }

    #[tokio::test]
    async fn test_mock_embed_similar_text() {
        let engine = MockEmbeddingEngine::new();
        let v1 = engine.embed("add login function").await.unwrap();
        let v2 = engine.embed("add login function test").await.unwrap();
        let v3 = engine.embed("delete database table").await.unwrap();

        let sim_same = cosine_similarity(&v1, &v2);
        let sim_diff = cosine_similarity(&v1, &v3);

        // 相似文本相似度应高于不同文本
        assert!(sim_same > sim_diff, "相似文本相似度 {} 应高于不同文本 {}", sim_same, sim_diff);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_different_len() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_embedding_index_upsert_search() {
        let mut index = EmbeddingIndex::new(64);
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let v3 = vec![0.9, 0.1, 0.0]; // 与 v1 相似

        index.upsert("snap1", v1.clone());
        index.upsert("snap2", v2.clone());
        index.upsert("snap3", v3.clone());

        let results = index.search(&v1, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "snap1");
        assert!((results[0].1 - 1.0).abs() < 0.001);
        // snap3 应排第二（与 v1 相似度高）
        assert_eq!(results[1].0, "snap3");
    }

    #[test]
    fn test_embedding_index_remove() {
        let mut index = EmbeddingIndex::new(64);
        index.upsert("snap1", vec![1.0, 0.0]);
        assert_eq!(index.len(), 1);

        let removed = index.remove("snap1");
        assert!(removed.is_some());
        assert_eq!(index.len(), 0);

        let not_found = index.remove("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_embedding_index_clear() {
        let mut index = EmbeddingIndex::new(64);
        index.upsert("a", vec![1.0]);
        index.upsert("b", vec![1.0]);
        index.clear();
        assert!(index.is_empty());
    }

    #[test]
    fn test_create_embedding_engine_mock() {
        let config = AiConfig::mock();
        let engine = create_embedding_engine(&config).unwrap();
        assert_eq!(engine.name(), "mock-embedding");
    }

    #[test]
    fn test_create_embedding_engine_ollama() {
        let config = AiConfig::ollama();
        let engine = create_embedding_engine(&config).unwrap();
        assert_eq!(engine.name(), "ollama-embedding");
    }
}
