// TimeFlow AI 语义层
// 设计文档: docs/TIMEFLOW-DESIGN.md § 4
//
// W5 M2.1: AI commit msg 自动生成
//   - LLM 引擎抽象（OpenAI / Ollama 双模式）
//   - diff 提取 + Conventional Commits prompt 模板
//   - generate_commit_msg() API
//   - 集成到快照流程
//
// W6 M2.2: AI 版本整理 + 语义搜索
//   - D1 快照分类（WIP/进度/候选/正式）
//   - D2 Embedding 索引
//   - D3 语义搜索 semantic_search()
//   - D4 候选版本识别

#![warn(missing_docs)]

mod error;
mod llm;
mod prompts;
mod commit_msg;
mod config;
mod classify;
mod embedding;
mod search;
mod candidate;

pub use error::{AiError, Result};
pub use llm::{LlmEngine, LlmMessage, LlmResponse, OpenAiEngine, OllamaEngine, MockEngine, create_engine};
pub use prompts::{build_commit_msg_prompt, build_summary_prompt, build_classify_prompt, ConventionalCommit, CommitType};
pub use commit_msg::{CommitMsgGenerator, format_diff, format_file_changes};
pub use config::{AiConfig, LlmProvider};
pub use classify::{SnapshotClassifier, ClassifyInput, ClassifyResult, snapshot_to_input};
pub use embedding::{EmbeddingEngine, EmbeddingVector, MockEmbeddingEngine, OllamaEmbeddingEngine, create_embedding_engine, cosine_similarity};
pub use search::{SemanticSearch, SearchResult, SearchIndex};
pub use candidate::{CandidateDetector, CandidateScore, detect_candidates};

/// 便捷函数：使用给定 LLM 引擎为 diff 生成 commit msg
///
/// 返回 Conventional Commits 格式的提交信息，例如：
/// `feat(auth): 添加 JWT 登录接口`
pub async fn generate_commit_msg(
    engine: &dyn LlmEngine,
    diff: &str,
    history: &[String],
) -> Result<String> {
    let generator = CommitMsgGenerator::new(engine);
    generator.generate(diff, history).await
}
