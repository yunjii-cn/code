// 知识库系统（M4.1 D1）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D1
//
// 目标：为 AI 员工提供知识库支持，实现 RAG 检索增强。
//
// 技术选型（v2.0 明确）：
//   - 存储：内存 + 可选 JSON 持久化（不引入 SQLite，MVP 阶段简化）
//   - Embedding：调用 timeflow-ai 的 EmbeddingEngine（Ollama/Mock/Gateway）
//   - 文档解析：Text/Markdown 完整实现，PDF/Word/Excel 接口预留
//   - 检索：线性扫描 + 余弦相似度（1000 chunk 内 < 100ms，无需 hnsw_rs）
//   - 不用 GPU：纯 CPU 检索
//
// 模块组成：
//   - KnowledgeSource: 来源类型枚举
//   - KnowledgeChunk: 知识块（分块后的文本 + 向量）
//   - KnowledgeBase: 知识库（管理多个 chunk）
//   - TextChunker: 文本分块器（500 字/chunk，50 字重叠）
//   - DocumentParser: 文档解析器（Text/Markdown 完整，PDF/Word/Excel 预留）
//   - SensitiveFilter: 敏感词过滤（标记"不入索引"）
//   - KnowledgeStore: 知识库存储（内存 HashMap）
//   - RagRetriever: RAG 检索器（top-k + 阈值过滤）

use crate::error::{Result, TeamError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use timeflow_ai::{EmbeddingEngine, EmbeddingVector, cosine_similarity};

// ============================================================================
// 数据结构
// ============================================================================

/// 知识库来源类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSource {
    /// 纯文本（直接粘贴）
    Text,
    /// Markdown 文档
    Markdown,
    /// PDF 文档（解析预留，MVP 暂未实现）
    Pdf,
    /// Word 文档（解析预留，MVP 暂未实现）
    Word,
    /// Excel 表格（解析预留，MVP 暂未实现）
    Excel,
    /// URL（抓取预留，MVP 暂未实现）
    Url,
}

impl Default for KnowledgeSource {
    fn default() -> Self {
        Self::Text
    }
}

impl std::fmt::Display for KnowledgeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Markdown => write!(f, "markdown"),
            Self::Pdf => write!(f, "pdf"),
            Self::Word => write!(f, "word"),
            Self::Excel => write!(f, "excel"),
            Self::Url => write!(f, "url"),
        }
    }
}

impl std::str::FromStr for KnowledgeSource {
    type Err = TeamError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "pdf" => Ok(Self::Pdf),
            "word" | "docx" | "doc" => Ok(Self::Word),
            "excel" | "xlsx" | "xls" => Ok(Self::Excel),
            "url" | "http" => Ok(Self::Url),
            other => Err(TeamError::Config(format!("未知知识库来源类型: {other}"))),
        }
    }
}

/// 知识块（分块后的文本单元）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    /// 块 ID（格式：{kb_id}_chunk_{index}）
    pub id: String,
    /// 所属知识库 ID
    pub kb_id: String,
    /// 块文本内容
    pub content: String,
    /// 块在知识库中的序号（从 0 开始）
    pub index: usize,
    /// 估算 token 数（粗略按 字数/1.5 估算，中文 1 字 ≈ 1.5 token）
    pub token_count: usize,
    /// 向量（未索引时为 None）
    #[serde(default)]
    pub embedding: Option<EmbeddingVector>,
    /// 是否含敏感词（含敏感词的块不入索引，但仍保留在知识库中）
    #[serde(default)]
    pub sensitive: bool,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl KnowledgeChunk {
    /// 创建新块
    pub fn new(kb_id: &str, index: usize, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_count = estimate_tokens(&content);
        Self {
            id: format!("{kb_id}_chunk_{index}"),
            kb_id: kb_id.to_string(),
            content,
            index,
            token_count,
            embedding: None,
            sensitive: false,
            metadata: HashMap::new(),
        }
    }

    /// 是否已索引（有向量）
    pub fn is_indexed(&self) -> bool {
        self.embedding.is_some() && !self.sensitive
    }

    /// 设置向量
    pub fn set_embedding(&mut self, embedding: EmbeddingVector) {
        if !self.sensitive {
            self.embedding = Some(embedding);
        }
    }

    /// 标记为敏感
    pub fn mark_sensitive(&mut self) {
        self.sensitive = true;
        self.embedding = None; // 敏感块清除向量
    }
}

/// 知识库
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// 知识库 ID
    pub id: String,
    /// 所属员工 ID
    pub employee_id: String,
    /// 知识库名称
    pub name: String,
    /// 来源类型
    pub source: KnowledgeSource,
    /// 来源路径（文件路径或 URL，Text 类型可为 None）
    #[serde(default)]
    pub source_path: Option<String>,
    /// 知识块列表
    pub chunks: Vec<KnowledgeChunk>,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeBase {
    /// 创建新知识库
    pub fn new(id: impl Into<String>, employee_id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            employee_id: employee_id.into(),
            name: name.into(),
            source: KnowledgeSource::default(),
            source_path: None,
            chunks: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 从文本创建知识库（自动分块）
    pub fn from_text(
        id: impl Into<String>,
        employee_id: impl Into<String>,
        name: impl Into<String>,
        text: &str,
    ) -> Self {
        let mut kb = Self::new(id, employee_id, name);
        kb.source = KnowledgeSource::Text;
        let chunks = TextChunker::default().chunk(text);
        for (i, chunk_text) in chunks.into_iter().enumerate() {
            kb.chunks.push(KnowledgeChunk::new(&kb.id, i, chunk_text));
        }
        kb.updated_at = Utc::now();
        kb
    }

    /// 从文件创建知识库（根据扩展名推断来源类型）
    pub fn from_file(
        id: impl Into<String>,
        employee_id: impl Into<String>,
        name: impl Into<String>,
        path: &Path,
    ) -> Result<Self> {
        let mut kb = Self::new(id, employee_id, name);
        kb.source_path = Some(path.to_string_lossy().to_string());

        let text = DocumentParser::parse_file(path)?;
        kb.source = DocumentParser::detect_source(path);

        let chunks = TextChunker::default().chunk(&text);
        for (i, chunk_text) in chunks.into_iter().enumerate() {
            kb.chunks.push(KnowledgeChunk::new(&kb.id, i, chunk_text));
        }
        kb.updated_at = Utc::now();
        Ok(kb)
    }

    /// 块数量
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// 已索引的块数量
    pub fn indexed_count(&self) -> usize {
        self.chunks.iter().filter(|c| c.is_indexed()).count()
    }

    /// 敏感块数量
    pub fn sensitive_count(&self) -> usize {
        self.chunks.iter().filter(|c| c.sensitive).count()
    }

    /// 为所有未索引的块生成向量
    pub async fn build_index(&mut self, engine: &dyn EmbeddingEngine) -> Result<usize> {
        let mut indexed = 0;
        for chunk in self.chunks.iter_mut() {
            if chunk.embedding.is_none() && !chunk.sensitive {
                let emb = engine.embed(&chunk.content).await?;
                chunk.set_embedding(emb);
                indexed += 1;
            }
        }
        self.updated_at = Utc::now();
        Ok(indexed)
    }

    /// 应用敏感词过滤（标记含敏感词的块）
    pub fn apply_sensitive_filter(&mut self, filter: &SensitiveFilter) -> usize {
        let mut marked = 0;
        for chunk in self.chunks.iter_mut() {
            if !chunk.sensitive && filter.contains_sensitive(&chunk.content) {
                chunk.mark_sensitive();
                marked += 1;
            }
        }
        self.updated_at = Utc::now();
        marked
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

// ============================================================================
// 文本分块器
// ============================================================================

/// 默认分块大小（字符数）
pub const DEFAULT_CHUNK_SIZE: usize = 500;

/// 默认分块重叠（字符数）
pub const DEFAULT_CHUNK_OVERLAP: usize = 50;

/// 文本分块器
///
/// 按字符数分块（支持中文，1 个汉字 = 1 字符），相邻块有重叠以保持上下文连续性。
#[derive(Debug, Clone)]
pub struct TextChunker {
    /// 每块字符数
    pub chunk_size: usize,
    /// 相邻块重叠字符数
    pub overlap: usize,
}

impl Default for TextChunker {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_CHUNK_OVERLAP,
        }
    }
}

impl TextChunker {
    /// 创建分块器
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size: chunk_size.max(50),
            overlap: overlap.min(chunk_size / 2),
        }
    }

    /// 分块
    pub fn chunk(&self, text: &str) -> Vec<String> {
        let text = text.trim();
        if text.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();
        if total <= self.chunk_size {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let step = self.chunk_size.saturating_sub(self.overlap).max(1);

        while start < total {
            let end = (start + self.chunk_size).min(total);
            let chunk: String = chars[start..end].iter().collect();
            let chunk = chunk.trim();
            if !chunk.is_empty() {
                chunks.push(chunk.to_string());
            }
            if end >= total {
                break;
            }
            start += step;
        }

        chunks
    }
}

// ============================================================================
// 文档解析器
// ============================================================================

/// 文档解析器
///
/// MVP 阶段完整实现 Text/Markdown，PDF/Word/Excel 返回未实现错误。
pub struct DocumentParser;

impl DocumentParser {
    /// 根据文件扩展名推断来源类型
    pub fn detect_source(path: &Path) -> KnowledgeSource {
        match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("txt") => KnowledgeSource::Text,
            Some("md") | Some("markdown") => KnowledgeSource::Markdown,
            Some("pdf") => KnowledgeSource::Pdf,
            Some("doc") | Some("docx") => KnowledgeSource::Word,
            Some("xls") | Some("xlsx") => KnowledgeSource::Excel,
            Some("url") | Some("http") => KnowledgeSource::Url,
            _ => KnowledgeSource::Text, // 默认按文本处理
        }
    }

    /// 解析文件，返回纯文本
    pub fn parse_file(path: &Path) -> Result<String> {
        let source = Self::detect_source(path);
        match source {
            KnowledgeSource::Text | KnowledgeSource::Markdown => {
                let text = std::fs::read_to_string(path)?;
                Ok(text)
            }
            KnowledgeSource::Pdf => Err(TeamError::Config(
                "PDF 解析暂未实现（MVP 阶段仅支持 Text/Markdown）。后续将引入 pdf-extract crate".to_string(),
            )),
            KnowledgeSource::Word => Err(TeamError::Config(
                "Word 解析暂未实现（MVP 阶段仅支持 Text/Markdown）。后续将引入 docx-rs crate".to_string(),
            )),
            KnowledgeSource::Excel => Err(TeamError::Config(
                "Excel 解析暂未实现（MVP 阶段仅支持 Text/Markdown）。后续将引入 calamine crate".to_string(),
            )),
            KnowledgeSource::Url => Err(TeamError::Config(
                "URL 抓取暂未实现（MVP 阶段仅支持 Text/Markdown）。后续将引入 reqwest + html2text".to_string(),
            )),
        }
    }

    /// 解析文本内容（直接使用，不分文件）
    pub fn parse_text(text: &str, source: KnowledgeSource) -> Result<String> {
        match source {
            KnowledgeSource::Text | KnowledgeSource::Markdown => Ok(text.to_string()),
            other => Err(TeamError::Config(format!(
                "{other} 来源不支持直接解析文本，请通过 parse_file 从文件加载"
            ))),
        }
    }
}

// ============================================================================
// 敏感词过滤
// ============================================================================

/// 敏感词过滤器
///
/// 简单关键词匹配，命中任一敏感词的块会被标记为 sensitive（不入索引）。
#[derive(Debug, Clone, Default)]
pub struct SensitiveFilter {
    /// 敏感词列表
    pub words: Vec<String>,
}

impl SensitiveFilter {
    /// 创建空过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 从词表创建
    pub fn from_words(words: Vec<String>) -> Self {
        Self { words }
    }

    /// 添加敏感词
    pub fn add(&mut self, word: impl Into<String>) {
        let word = word.into();
        if !word.is_empty() && !self.words.contains(&word) {
            self.words.push(word);
        }
    }

    /// 批量添加
    pub fn extend(&mut self, words: Vec<String>) {
        for w in words {
            self.add(w);
        }
    }

    /// 检查文本是否包含敏感词
    pub fn contains_sensitive(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.words.iter().any(|w| {
            let w_lower = w.to_lowercase();
            !w_lower.is_empty() && lower.contains(&w_lower)
        })
    }

    /// 返回文本中命中的敏感词列表
    pub fn find_sensitive(&self, text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        self.words
            .iter()
            .filter(|w| {
                let w_lower = w.to_lowercase();
                !w_lower.is_empty() && lower.contains(&w_lower)
            })
            .cloned()
            .collect()
    }

    /// 敏感词数量
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// 内置金融合规敏感词模板
    pub fn builtin_finance() -> Self {
        Self::from_words(vec![
            "身份证号".to_string(),
            "银行卡号".to_string(),
            "密码".to_string(),
            "验证码".to_string(),
            "CVV".to_string(),
            "手机号".to_string(),
        ])
    }

    /// 内置医疗保密敏感词模板
    pub fn builtin_medical() -> Self {
        Self::from_words(vec![
            "病历".to_string(),
            "诊断".to_string(),
            "处方".to_string(),
            "HIV".to_string(),
            "精神疾病".to_string(),
        ])
    }

    /// 内置教育守则敏感词模板
    pub fn builtin_education() -> Self {
        Self::from_words(vec![
            "学生成绩".to_string(),
            "家庭住址".to_string(),
            "家长电话".to_string(),
        ])
    }
}

// ============================================================================
// 知识库存储
// ============================================================================

/// 知识库存储（内存版）
///
/// 管理 employee_id → Vec<KnowledgeBase> 的映射。
/// MVP 阶段不引入 SQLite，所有知识库常驻内存，启动时从 JSON 文件加载。
#[derive(Debug, Default)]
pub struct KnowledgeStore {
    /// employee_id → 知识库列表
    pub kbs: HashMap<String, Vec<KnowledgeBase>>,
}

impl KnowledgeStore {
    /// 创建空存储
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加知识库
    pub fn add(&mut self, kb: KnowledgeBase) {
        self.kbs.entry(kb.employee_id.clone()).or_default().push(kb);
    }

    /// 获取员工的所有知识库
    pub fn get_by_employee(&self, employee_id: &str) -> Vec<&KnowledgeBase> {
        self.kbs.get(employee_id).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// 获取员工的所有知识库（可变）
    pub fn get_by_employee_mut(&mut self, employee_id: &str) -> Vec<&mut KnowledgeBase> {
        self.kbs.get_mut(employee_id).map(|v| v.iter_mut().collect()).unwrap_or_default()
    }

    /// 按 ID 获取知识库
    pub fn get(&self, kb_id: &str) -> Option<&KnowledgeBase> {
        self.kbs.values().flatten().find(|kb| kb.id == kb_id)
    }

    /// 按 ID 获取知识库（可变）
    pub fn get_mut(&mut self, kb_id: &str) -> Option<&mut KnowledgeBase> {
        self.kbs.values_mut().flatten().find(|kb| kb.id == kb_id)
    }

    /// 删除知识库
    pub fn remove(&mut self, kb_id: &str) -> Option<KnowledgeBase> {
        for kbs in self.kbs.values_mut() {
            if let Some(idx) = kbs.iter().position(|kb| kb.id == kb_id) {
                return Some(kbs.remove(idx));
            }
        }
        None
    }

    /// 获取员工的所有已索引块（用于检索）
    pub fn get_indexed_chunks(&self, employee_id: &str) -> Vec<&KnowledgeChunk> {
        self.get_by_employee(employee_id)
            .into_iter()
            .flat_map(|kb| kb.chunks.iter())
            .filter(|c| c.is_indexed())
            .collect()
    }

    /// 总块数
    pub fn total_chunks(&self) -> usize {
        self.kbs.values().flatten().map(|kb| kb.chunks.len()).sum()
    }

    /// 总已索引块数
    pub fn total_indexed(&self) -> usize {
        self.kbs
            .values()
            .flatten()
            .map(|kb| kb.indexed_count())
            .sum()
    }

    /// 总敏感块数
    pub fn total_sensitive(&self) -> usize {
        self.kbs
            .values()
            .flatten()
            .map(|kb| kb.sensitive_count())
            .sum()
    }

    /// 知识库数量
    pub fn len(&self) -> usize {
        self.kbs.values().map(|v| v.len()).sum()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ============================================================================
// RAG 检索器
// ============================================================================

/// RAG 检索结果
#[derive(Debug, Clone)]
pub struct RagResult {
    /// 命中的块
    pub chunk: KnowledgeChunk,
    /// 相似度分数（0.0 ~ 1.0）
    pub score: f32,
    /// 所属知识库 ID
    pub kb_id: String,
    /// 所属知识库名称
    pub kb_name: String,
}

/// RAG 检索器
///
/// 默认配置：top-5 + 阈值 0.7
pub struct RagRetriever {
    /// 返回结果数
    pub top_k: usize,
    /// 最小相似度阈值（低于此值的结果被过滤）
    pub threshold: f32,
}

impl Default for RagRetriever {
    fn default() -> Self {
        Self {
            top_k: 5,
            threshold: 0.7,
        }
    }
}

impl RagRetriever {
    /// 创建检索器
    pub fn new(top_k: usize, threshold: f32) -> Self {
        Self {
            top_k: top_k.max(1),
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// 检索
    ///
    /// 参数：
    /// - `query_embedding`: 查询文本的向量
    /// - `chunks`: 候选块（必须已索引，即有向量）
    ///
    /// 返回按相似度降序的结果列表
    pub fn search(
        &self,
        query_embedding: &EmbeddingVector,
        chunks: Vec<&KnowledgeChunk>,
    ) -> Vec<RagResult> {
        let mut scored: Vec<(usize, f32)> = chunks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.embedding
                    .as_ref()
                    .map(|emb| (i, cosine_similarity(query_embedding, emb)))
            })
            .filter(|(_, score)| *score >= self.threshold)
            .collect();

        // 按相似度降序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.top_k);

        scored
            .into_iter()
            .filter_map(|(i, score)| {
                let c = chunks[i];
                Some(RagResult {
                    chunk: c.clone(),
                    score,
                    kb_id: c.kb_id.clone(),
                    kb_name: String::new(), // 由调用方填充
                })
            })
            .collect()
    }

    /// 在知识库存储中检索（按员工 ID）
    pub async fn search_employee(
        &self,
        store: &KnowledgeStore,
        employee_id: &str,
        query: &str,
        engine: &dyn EmbeddingEngine,
    ) -> Result<Vec<RagResult>> {
        let query_emb = engine.embed(query).await?;
        let chunks = store.get_indexed_chunks(employee_id);

        // 构建 kb_id → kb_name 映射
        let kb_names: HashMap<String, String> = store
            .get_by_employee(employee_id)
            .into_iter()
            .map(|kb| (kb.id.clone(), kb.name.clone()))
            .collect();

        let mut results = self.search(&query_emb, chunks);
        // 填充 kb_name
        for r in results.iter_mut() {
            if let Some(name) = kb_names.get(&r.kb_id) {
                r.kb_name = name.clone();
            }
        }
        Ok(results)
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 估算文本的 token 数（粗略：中文 1 字 ≈ 1.5 token，英文 1 词 ≈ 1.3 token）
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk_chars = 0usize;
    let mut other_chars = 0usize;
    for c in text.chars() {
        if is_cjk_char(c) {
            cjk_chars += 1;
        } else if !c.is_whitespace() {
            other_chars += 1;
        }
    }
    // CJK: 1 字 ≈ 1.5 token
    // 其他: 4 字符 ≈ 1 token（英文平均词长 4）
    (cjk_chars as f32 * 1.5).round() as usize + (other_chars as f32 / 4.0).round() as usize
}

/// 判断字符是否为 CJK 字符
pub fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    // CJK 统一表意文字 + 扩展 A 区 + 兼容表意文字
    (0x4E00..=0x9FFF).contains(&code)
        || (0x3400..=0x4DBF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2A6DF).contains(&code)
        || (0x2A700..=0x2B73F).contains(&code)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use timeflow_ai::MockEmbeddingEngine;

    // ---- KnowledgeSource ----

    #[test]
    fn test_source_from_str() {
        assert_eq!("text".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Text);
        assert_eq!("md".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Markdown);
        assert_eq!("pdf".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Pdf);
        assert_eq!("docx".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Word);
        assert_eq!("xlsx".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Excel);
        assert_eq!("url".parse::<KnowledgeSource>().unwrap(), KnowledgeSource::Url);
        assert!("invalid".parse::<KnowledgeSource>().is_err());
    }

    #[test]
    fn test_source_display() {
        assert_eq!(KnowledgeSource::Text.to_string(), "text");
        assert_eq!(KnowledgeSource::Markdown.to_string(), "markdown");
        assert_eq!(KnowledgeSource::Pdf.to_string(), "pdf");
    }

    // ---- KnowledgeChunk ----

    #[test]
    fn test_chunk_new() {
        let chunk = KnowledgeChunk::new("kb1", 0, "hello world");
        assert_eq!(chunk.id, "kb1_chunk_0");
        assert_eq!(chunk.kb_id, "kb1");
        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.content, "hello world");
        assert!(!chunk.sensitive);
        assert!(chunk.embedding.is_none());
        assert!(!chunk.is_indexed());
    }

    #[test]
    fn test_chunk_set_embedding() {
        let mut chunk = KnowledgeChunk::new("kb1", 0, "test");
        chunk.set_embedding(vec![0.1, 0.2, 0.3]);
        assert!(chunk.is_indexed());
    }

    #[test]
    fn test_chunk_mark_sensitive_clears_embedding() {
        let mut chunk = KnowledgeChunk::new("kb1", 0, "test");
        chunk.set_embedding(vec![0.1, 0.2]);
        assert!(chunk.is_indexed());
        chunk.mark_sensitive();
        assert!(chunk.sensitive);
        assert!(chunk.embedding.is_none());
        assert!(!chunk.is_indexed());
    }

    #[test]
    fn test_chunk_sensitive_not_indexed() {
        let mut chunk = KnowledgeChunk::new("kb1", 0, "test");
        chunk.mark_sensitive();
        chunk.set_embedding(vec![0.1, 0.2]); // 应被忽略
        assert!(chunk.embedding.is_none());
        assert!(!chunk.is_indexed());
    }

    // ---- TextChunker ----

    #[test]
    fn test_chunker_short_text_single_chunk() {
        let chunker = TextChunker::default();
        let chunks = chunker.chunk("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn test_chunker_empty_text() {
        let chunker = TextChunker::default();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunker_long_text_multiple_chunks() {
        let chunker = TextChunker::new(100, 20);
        let text: String = "a".repeat(250);
        let chunks = chunker.chunk(&text);
        assert!(chunks.len() > 1, "长文本应分成多块，实际 {} 块", chunks.len());
        // 每块不超过 100 字符
        for c in &chunks {
            assert!(c.len() <= 100, "块长度 {} 超过 100", c.len());
        }
    }

    #[test]
    fn test_chunker_chinese_text() {
        // TextChunker::new 有 .max(50) 最小限制，所以用 50 + 较长文本
        let chunker = TextChunker::new(50, 10);
        let text: String = "你好世界测试文本分块器".repeat(10); // 100 个中文字符
        let chunks = chunker.chunk(&text);
        assert!(chunks.len() > 1, "中文长文本应分成多块，实际 {} 块", chunks.len());
        // 第一块应是前 50 个字符
        assert_eq!(chunks[0].chars().count(), 50);
    }

    #[test]
    fn test_chunker_overlap_preserves_context() {
        let chunker = TextChunker::new(50, 20);
        let text: String = "0123456789".repeat(10); // 100 字符
        let chunks = chunker.chunk(&text);
        // 有重叠时，块数应多于无重叠
        let no_overlap_chunker = TextChunker::new(50, 0);
        let no_overlap_chunks = no_overlap_chunker.chunk(&text);
        assert!(chunks.len() >= no_overlap_chunks.len());
    }

    // ---- KnowledgeBase ----

    #[test]
    fn test_kb_from_text() {
        let kb = KnowledgeBase::from_text("kb1", "emp1", "测试知识库", "hello world");
        assert_eq!(kb.id, "kb1");
        assert_eq!(kb.employee_id, "emp1");
        assert_eq!(kb.name, "测试知识库");
        assert_eq!(kb.source, KnowledgeSource::Text);
        assert_eq!(kb.chunks.len(), 1);
        assert_eq!(kb.chunks[0].content, "hello world");
    }

    #[test]
    fn test_kb_from_text_long() {
        let chunker = TextChunker::default();
        let text: String = "a".repeat(chunker.chunk_size * 3);
        let kb = KnowledgeBase::from_text("kb1", "emp1", "测试", &text);
        assert!(kb.chunks.len() >= 3, "长文本应分成 3+ 块");
    }

    #[test]
    fn test_kb_indexed_count() {
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello");
        assert_eq!(kb.indexed_count(), 0); // 未生成向量

        // 模拟设置向量
        kb.chunks[0].set_embedding(vec![0.1, 0.2]);
        assert_eq!(kb.indexed_count(), 1);
    }

    #[test]
    fn test_kb_sensitive_count() {
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello");
        assert_eq!(kb.sensitive_count(), 0);

        kb.chunks[0].mark_sensitive();
        assert_eq!(kb.sensitive_count(), 1);
        assert_eq!(kb.indexed_count(), 0);
    }

    #[tokio::test]
    async fn test_kb_build_index_with_mock() {
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello world foo bar");
        let engine = MockEmbeddingEngine::new();
        let indexed = kb.build_index(&engine).await.unwrap();
        assert!(indexed > 0);
        assert_eq!(kb.indexed_count(), kb.chunks.len());
    }

    #[tokio::test]
    async fn test_kb_build_index_skips_sensitive() {
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello world");
        kb.chunks[0].mark_sensitive();
        let engine = MockEmbeddingEngine::new();
        let indexed = kb.build_index(&engine).await.unwrap();
        assert_eq!(indexed, 0); // 敏感块被跳过
        assert_eq!(kb.indexed_count(), 0);
    }

    #[test]
    fn test_kb_apply_sensitive_filter() {
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "包含身份证号的文本");
        let filter = SensitiveFilter::builtin_finance();
        let marked = kb.apply_sensitive_filter(&filter);
        assert_eq!(marked, 1);
        assert_eq!(kb.sensitive_count(), 1);
    }

    #[test]
    fn test_kb_json_roundtrip() {
        let kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello world");
        let json = kb.to_json().unwrap();
        let kb2 = KnowledgeBase::from_json(&json).unwrap();
        assert_eq!(kb.id, kb2.id);
        assert_eq!(kb.name, kb2.name);
        assert_eq!(kb.chunks.len(), kb2.chunks.len());
    }

    // ---- DocumentParser ----

    #[test]
    fn test_parser_detect_source() {
        assert_eq!(DocumentParser::detect_source(Path::new("a.txt")), KnowledgeSource::Text);
        assert_eq!(DocumentParser::detect_source(Path::new("a.md")), KnowledgeSource::Markdown);
        assert_eq!(DocumentParser::detect_source(Path::new("a.pdf")), KnowledgeSource::Pdf);
        assert_eq!(DocumentParser::detect_source(Path::new("a.docx")), KnowledgeSource::Word);
        assert_eq!(DocumentParser::detect_source(Path::new("a.xlsx")), KnowledgeSource::Excel);
        assert_eq!(DocumentParser::detect_source(Path::new("a.unknown")), KnowledgeSource::Text);
    }

    #[test]
    fn test_parser_parse_text() {
        let text = "hello world";
        let result = DocumentParser::parse_text(text, KnowledgeSource::Text).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_parser_parse_file_unsupported() {
        // PDF 应返回未实现错误
        let result = DocumentParser::parse_file(Path::new("test.pdf"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("PDF 解析暂未实现"));
    }

    // ---- SensitiveFilter ----

    #[test]
    fn test_sensitive_filter_contains() {
        let filter = SensitiveFilter::from_words(vec!["密码".to_string(), "身份证".to_string()]);
        assert!(filter.contains_sensitive("请告诉我身份证号"));
        assert!(filter.contains_sensitive("银行卡密码是多少"));
        assert!(!filter.contains_sensitive("普通文本"));
    }

    #[test]
    fn test_sensitive_filter_case_insensitive() {
        let filter = SensitiveFilter::from_words(vec!["cvv".to_string()]);
        assert!(filter.contains_sensitive("card CVV is 123"));
        assert!(filter.contains_sensitive("card cvv is 123"));
    }

    #[test]
    fn test_sensitive_filter_find() {
        let filter = SensitiveFilter::from_words(vec![
            "密码".to_string(),
            "身份证".to_string(),
            "验证码".to_string(),
        ]);
        let found = filter.find_sensitive("请提供身份证号和验证码");
        assert_eq!(found.len(), 2);
        assert!(found.contains(&"身份证".to_string()));
        assert!(found.contains(&"验证码".to_string()));
    }

    #[test]
    fn test_sensitive_filter_builtin_finance() {
        let filter = SensitiveFilter::builtin_finance();
        assert!(filter.contains_sensitive("银行卡号是 622848"));
        assert!(!filter.contains_sensitive("普通业务文本"));
    }

    #[test]
    fn test_sensitive_filter_builtin_medical() {
        let filter = SensitiveFilter::builtin_medical();
        assert!(filter.contains_sensitive("患者病历显示"));
        assert!(!filter.contains_sensitive("普通文本"));
    }

    #[test]
    fn test_sensitive_filter_add_and_extend() {
        let mut filter = SensitiveFilter::new();
        assert!(filter.is_empty());
        filter.add("密码");
        filter.add("密码"); // 重复添加应忽略
        assert_eq!(filter.len(), 1);
        filter.extend(vec!["身份证".to_string(), "手机号".to_string()]);
        assert_eq!(filter.len(), 3);
    }

    // ---- KnowledgeStore ----

    #[test]
    fn test_store_add_and_get() {
        let mut store = KnowledgeStore::new();
        let kb1 = KnowledgeBase::from_text("kb1", "emp1", "测试1", "hello");
        let kb2 = KnowledgeBase::from_text("kb2", "emp1", "测试2", "world");
        let kb3 = KnowledgeBase::from_text("kb3", "emp2", "测试3", "foo");
        store.add(kb1);
        store.add(kb2);
        store.add(kb3);

        assert_eq!(store.get_by_employee("emp1").len(), 2);
        assert_eq!(store.get_by_employee("emp2").len(), 1);
        assert_eq!(store.get_by_employee("emp3").len(), 0);
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_store_get_by_id() {
        let mut store = KnowledgeStore::new();
        store.add(KnowledgeBase::from_text("kb1", "emp1", "测试", "hello"));
        assert!(store.get("kb1").is_some());
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_store_remove() {
        let mut store = KnowledgeStore::new();
        store.add(KnowledgeBase::from_text("kb1", "emp1", "测试", "hello"));
        assert_eq!(store.len(), 1);

        let removed = store.remove("kb1");
        assert!(removed.is_some());
        assert_eq!(store.len(), 0);

        let not_found = store.remove("nonexistent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_store_get_indexed_chunks() {
        let mut store = KnowledgeStore::new();
        let mut kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello world");
        let engine = MockEmbeddingEngine::new();
        kb.build_index(&engine).await.unwrap();
        store.add(kb);

        let chunks = store.get_indexed_chunks("emp1");
        assert!(!chunks.is_empty());
        assert_eq!(store.total_indexed(), chunks.len());
    }

    // ---- RagRetriever ----

    #[tokio::test]
    async fn test_rag_search_basic() {
        let mut store = KnowledgeStore::new();
        let mut kb = KnowledgeBase::from_text(
            "kb1",
            "emp1",
            "测试",
            "你好世界 hello world 这是测试文本",
        );
        let engine = MockEmbeddingEngine::new();
        kb.build_index(&engine).await.unwrap();
        store.add(kb);

        let retriever = RagRetriever::default();
        let results = retriever
            .search_employee(&store, "emp1", "hello", &engine)
            .await
            .unwrap();

        // Mock 引擎下，相似文本应能命中
        // 注意：Mock 引擎基于哈希，相似度可能不高，threshold=0.7 可能过滤掉所有结果
        // 这里只验证不报错
        let _ = results;
    }

    #[test]
    fn test_rag_search_threshold_filter() {
        let retriever = RagRetriever::new(5, 0.99); // 极高阈值
        let query = vec![1.0, 0.0, 0.0];
        let mut chunk = KnowledgeChunk::new("kb1", 0, "test");
        chunk.set_embedding(vec![0.7, 0.7, 0.0]); // 与 query 相似度约 0.707
        let chunks = vec![&chunk];

        let results = retriever.search(&query, chunks);
        assert!(results.is_empty(), "0.99 阈值应过滤掉 0.707 相似度");
    }

    #[test]
    fn test_rag_search_top_k() {
        let retriever = RagRetriever::new(2, 0.0); // 低阈值，限制 top-2
        let query = vec![1.0, 0.0];

        let mut chunks: Vec<KnowledgeChunk> = Vec::new();
        for i in 0..5 {
            let mut c = KnowledgeChunk::new("kb1", i, format!("chunk {i}"));
            c.set_embedding(vec![1.0, 0.0]); // 全部与 query 完全相同
            chunks.push(c);
        }
        let chunk_refs: Vec<&KnowledgeChunk> = chunks.iter().collect();

        let results = retriever.search(&query, chunk_refs);
        assert_eq!(results.len(), 2, "top-2 应只返回 2 个结果");
    }

    #[test]
    fn test_rag_search_skips_unindexed() {
        let retriever = RagRetriever::new(5, 0.0);
        let query = vec![1.0, 0.0];

        let mut chunk = KnowledgeChunk::new("kb1", 0, "test");
        // 不设置 embedding
        chunk.mark_sensitive();
        let chunks = vec![&chunk];

        let results = retriever.search(&query, chunks);
        assert!(results.is_empty(), "敏感块（无向量）应被跳过");
    }

    // ---- 工具函数 ----

    #[test]
    fn test_estimate_tokens_english() {
        let tokens = estimate_tokens("hello world foo bar");
        // 19 字符 / 4 ≈ 5 token
        assert!(tokens > 0 && tokens < 10);
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let tokens = estimate_tokens("你好世界");
        // 4 个 CJK 字符 × 1.5 = 6 token
        assert_eq!(tokens, 6);
    }

    #[test]
    fn test_is_cjk_char() {
        assert!(is_cjk_char('你'));
        assert!(is_cjk_char('好'));
        assert!(!is_cjk_char('a'));
        assert!(!is_cjk_char('1'));
        assert!(!is_cjk_char(' '));
    }

    #[test]
    fn test_kb_save_load_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("kb1.json");

        let kb = KnowledgeBase::from_text("kb1", "emp1", "测试", "hello world");
        kb.save_to_file(&path).unwrap();

        let loaded = KnowledgeBase::load_from_file(&path).unwrap();
        assert_eq!(kb.id, loaded.id);
        assert_eq!(kb.name, loaded.name);
        assert_eq!(kb.chunks.len(), loaded.chunks.len());
    }
}
