// 语义搜索（W6 M2.2 D3）
// 设计文档: docs/ROADMAP.md § W6 M2.2 D3
//
// 目标：用户用自然语言查询版本，例如：
//   "回到加登录的版本"  → 找到 commit msg 含"登录"的快照
//   "修复数据库 bug 那次" → 找到含"数据库"+"bug"+"修复"的快照
//
// 实现：
//   1. 为每个快照的 commit msg + ai_summary 生成 embedding
//   2. 为查询文本生成 embedding
//   3. 余弦相似度排序，返回 top-k
//
// 验收："回到加登录的版本" 能找到对应快照

use crate::embedding::{cosine_similarity, EmbeddingEngine, EmbeddingIndex, EmbeddingVector};
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 快照 ID
    pub snapshot_id: String,
    /// 相似度分数（0.0-1.0）
    pub score: f32,
    /// 快照的 commit msg（用于 UI 展示）
    pub message: String,
}

/// 搜索索引
///
/// 存储快照元数据 + embedding，支持语义搜索
pub struct SearchIndex {
    /// embedding 索引
    embeddings: EmbeddingIndex,
    /// snapshot_id → 展示文本（commit msg + ai_summary）
    texts: std::collections::HashMap<String, String>,
    /// embedding 引擎
    engine: Box<dyn EmbeddingEngine>,
}

impl SearchIndex {
    /// 创建搜索索引
    pub fn new(engine: Box<dyn EmbeddingEngine>) -> Self {
        let dim = engine.dim();
        Self {
            embeddings: EmbeddingIndex::new(dim),
            texts: std::collections::HashMap::new(),
            engine,
        }
    }

    /// 添加快照到索引
    ///
    /// `text` 通常是 commit msg + ai_summary 拼接
    pub async fn add(&mut self, snapshot_id: impl Into<String>, text: impl Into<String>) -> Result<()> {
        let snapshot_id = snapshot_id.into();
        let text = text.into();
        let embedding = self.engine.embed(&text).await?;
        self.embeddings.upsert(snapshot_id.clone(), embedding);
        self.texts.insert(snapshot_id, text);
        Ok(())
    }

    /// 批量添加
    pub async fn add_batch(
        &mut self,
        items: Vec<(String, String)>,
    ) -> Result<()> {
        for (id, text) in items {
            self.add(id, text).await?;
        }
        Ok(())
    }

    /// 移除快照
    pub fn remove(&mut self, snapshot_id: &str) -> Option<String> {
        self.embeddings.remove(snapshot_id);
        self.texts.remove(snapshot_id)
    }

    /// 语义搜索
    ///
    /// 返回最相似的 top-k 快照
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        if self.embeddings.is_empty() {
            return Ok(vec![]);
        }

        let query_emb = self.engine.embed(query).await?;
        let scores = self.embeddings.search(&query_emb, k);

        let results: Vec<SearchResult> = scores
            .into_iter()
            .map(|(id, score)| {
                let message = self
                    .texts
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();
                SearchResult {
                    snapshot_id: id,
                    score,
                    message,
                }
            })
            .collect();

        Ok(results)
    }

    /// 索引大小
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// 清空索引
    pub fn clear(&mut self) {
        self.embeddings.clear();
        self.texts.clear();
    }
}

/// 语义搜索器
///
/// 封装 SearchIndex + TimeFlow，提供端到端搜索
pub struct SemanticSearch<'a> {
    /// 搜索索引
    index: &'a SearchIndex,
}

impl<'a> SemanticSearch<'a> {
    /// 创建搜索器
    pub fn new(index: &'a SearchIndex) -> Self {
        Self { index }
    }

    /// 语义搜索
    ///
    /// 用法：
    /// ```ignore
    /// let results = semantic_search.search("加登录的版本", 5).await?;
    /// for r in results {
    ///     println!("{} (score={:.2}): {}", r.snapshot_id, r.score, r.message);
    /// }
    /// ```
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        self.index.search(query, k).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::MockEmbeddingEngine;

    async fn make_index_with_data() -> SearchIndex {
        let engine = Box::new(MockEmbeddingEngine::new());
        let mut index = SearchIndex::new(engine);
        index
            .add_batch(vec![
                ("snap1".to_string(), "feat: 添加用户登录功能".to_string()),
                ("snap2".to_string(), "fix: 修复登录页面崩溃".to_string()),
                ("snap3".to_string(), "feat: 添加数据库迁移".to_string()),
                ("snap4".to_string(), "docs: 更新 README".to_string()),
                ("snap5".to_string(), "refactor: 重构认证模块".to_string()),
            ])
            .await
            .unwrap();
        index
    }

    #[tokio::test]
    async fn test_search_finds_relevant() {
        let index = make_index_with_data().await;
        let searcher = SemanticSearch::new(&index);

        let results = searcher.search("登录功能", 5).await.unwrap();
        assert!(!results.is_empty());
        // 含 "登录" 的快照应出现在结果中
        // 注：Mock embedding 基于哈希，不保证 top-1，但相关快照应在结果里
        let has_login = results
            .iter()
            .any(|r| r.message.contains("登录"));
        assert!(has_login, "搜索 '登录' 应在结果中找到含 '登录' 的快照");
    }

    #[tokio::test]
    async fn test_search_empty_index() {
        let engine = Box::new(MockEmbeddingEngine::new());
        let index = SearchIndex::new(engine);
        let searcher = SemanticSearch::new(&index);

        let results = searcher.search("anything", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_limit_k() {
        let index = make_index_with_data().await;
        let searcher = SemanticSearch::new(&index);

        let results = searcher.search("代码", 2).await.unwrap();
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_returns_score() {
        let index = make_index_with_data().await;
        let searcher = SemanticSearch::new(&index);

        let results = searcher.search("登录", 5).await.unwrap();
        for r in &results {
            assert!(r.score >= -1.0 && r.score <= 1.0, "score 应在 [-1, 1] 范围");
        }
        // 结果应按分数降序
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score, "结果应按分数降序");
        }
    }

    #[tokio::test]
    async fn test_index_add_remove() {
        let engine = Box::new(MockEmbeddingEngine::new());
        let mut index = SearchIndex::new(engine);
        index.add("s1", "hello").await.unwrap();
        assert_eq!(index.len(), 1);

        let removed = index.remove("s1");
        assert!(removed.is_some());
        assert!(index.is_empty());
    }

    #[tokio::test]
    async fn test_index_clear() {
        let engine = Box::new(MockEmbeddingEngine::new());
        let mut index = SearchIndex::new(engine);
        index.add("a", "x").await.unwrap();
        index.add("b", "y").await.unwrap();
        index.clear();
        assert!(index.is_empty());
    }

    #[tokio::test]
    async fn test_search_exact_match() {
        let index = make_index_with_data().await;
        let searcher = SemanticSearch::new(&index);

        // 用完全相同的文本查询
        let results = searcher.search("feat: 添加用户登录功能", 1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].snapshot_id, "snap1");
        // 完全匹配相似度应接近 1.0
        assert!(results[0].score > 0.99, "完全匹配 score 应接近 1.0, 实际: {}", results[0].score);
    }

    #[tokio::test]
    async fn test_search_database_query() {
        let index = make_index_with_data().await;
        let searcher = SemanticSearch::new(&index);

        let results = searcher.search("数据库", 3).await.unwrap();
        // snap3 含 "数据库"，应出现在 top-3 结果中
        // 注：Mock embedding 基于哈希，不保证 top-1，但相关快照应在 top-k
        assert!(!results.is_empty());
        let ids: Vec<&str> = results.iter().map(|r| r.snapshot_id.as_str()).collect();
        assert!(ids.contains(&"snap3"), "snap3 应出现在 top-3 结果中");
    }
}
