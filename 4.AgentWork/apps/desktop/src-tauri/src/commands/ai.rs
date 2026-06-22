// AI 命令（W5 M2.1 + W6 M2.2）
// - 配置 LLM 引擎（provider / api_key / model）
// - 启用/禁用 AI 自动生成
// - 手动生成 commit msg
// - 集成到快照流程
// - 语义搜索快照（W6 M2.2 D5）
// - 候选版本识别（W6 M2.2 D5）

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppResult;
use timeflow_ai::{
    CandidateDetector, ClassifyInput, CommitMsgGenerator, LlmEngine, LlmProvider, MockEngine,
    MockEmbeddingEngine, SearchIndex, SnapshotClassifier,
};
use timeflow_core::{ChangeStatus, FileChange, SnapshotOps, SnapshotType};

/// AI 配置（前端可读写）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfigInfo {
    /// LLM 提供方
    pub provider: String,
    /// API base URL
    pub base_url: String,
    /// 模型名
    pub model: String,
    /// API key（只读出是否已配置，不返回明文）
    pub has_api_key: bool,
    /// 最大 token 数
    pub max_tokens: u32,
    /// 温度
    pub temperature: f32,
    /// 是否启用 AI 自动生成
    pub enabled: bool,
}

/// 获取当前 AI 配置
#[tauri::command]
pub async fn get_ai_config(state: tauri::State<'_, crate::AppState>) -> AppResult<AiConfigInfo> {
    let cfg = state.ai_config.lock().unwrap();
    let enabled = *state.ai_enabled.lock().unwrap();
    Ok(AiConfigInfo {
        provider: cfg.provider.to_string(),
        base_url: cfg.base_url.clone(),
        model: cfg.model.clone(),
        has_api_key: cfg.api_key.is_some(),
        max_tokens: cfg.max_tokens,
        temperature: cfg.temperature,
        enabled,
    })
}

/// 更新 AI 配置
///
/// 更新后会重建 LLM 引擎。如果 provider 是 mock，不要求 api_key。
#[tauri::command]
pub async fn set_ai_config(
    provider: String,
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    enabled: Option<bool>,
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
) -> AppResult<String> {
    let parsed_provider: LlmProvider = provider
        .parse()
        .map_err(|e: timeflow_ai::AiError| crate::AppError::Other(e.to_string()))?;

    // 更新配置
    {
        let mut cfg = state.ai_config.lock().unwrap();
        cfg.provider = parsed_provider;

        if let Some(url) = base_url {
            cfg.base_url = url;
        } else {
            cfg.base_url = match parsed_provider {
                LlmProvider::OpenAi => "https://api.openai.com/v1".to_string(),
                LlmProvider::Ollama => "http://localhost:11434".to_string(),
                LlmProvider::Mock => "mock://localhost".to_string(),
                LlmProvider::Mg => "https://mg.yunjii.cn/v1".to_string(),
            };
        }

        if let Some(key) = api_key {
            cfg.api_key = if key.is_empty() { None } else { Some(key) };
        }

        if let Some(m) = model {
            cfg.model = m;
        } else {
            cfg.model = match parsed_provider {
                LlmProvider::OpenAi => "gpt-4o-mini".to_string(),
                LlmProvider::Ollama => "qwen2.5-coder:7b".to_string(),
                LlmProvider::Mock => "mock-model".to_string(),
                LlmProvider::Mg => "deepseek-coder".to_string(),
            };
        }

        if let Some(mt) = max_tokens {
            cfg.max_tokens = mt;
        }
        if let Some(t) = temperature {
            cfg.temperature = t;
        }
    }

    // 更新启用状态
    if let Some(e) = enabled {
        *state.ai_enabled.lock().unwrap() = e;
    }

    // 重建 LLM 引擎
    let new_cfg = state.ai_config.lock().unwrap().clone();
    let engine: Box<dyn LlmEngine> = match new_cfg.provider {
        LlmProvider::Mock => Box::new(MockEngine::conventional_commit()),
        LlmProvider::Mg => match timeflow_ai::create_engine(new_cfg) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("MG 引擎创建失败: {}", e);
                let mut ae = state.ai_engine.lock().unwrap();
                *ae = None;
                return Err(crate::AppError::Ai(e));
            }
        },
        _ => match timeflow_ai::create_engine(new_cfg) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("LLM 引擎创建失败: {}", e);
                let mut ae = state.ai_engine.lock().unwrap();
                *ae = None;
                return Err(crate::AppError::Ai(e));
            }
        },
    };

    let mut ae = state.ai_engine.lock().unwrap();
    *ae = Some(engine);

    // 持久化保存 AI 配置
    let cfg_snapshot = state.ai_config.lock().unwrap().clone();
    crate::config_store::save(&app, crate::config_store::keys::AI_CONFIG, &cfg_snapshot);
    tracing::info!("AI 配置已持久化: provider={}", provider);

    Ok(format!("AI 配置已更新（provider={provider}）"))
}

/// 启用/禁用 AI 自动生成
#[tauri::command]
pub async fn set_ai_enabled(
    enabled: bool,
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    *state.ai_enabled.lock().unwrap() = enabled;
    crate::config_store::save(&app, crate::config_store::keys::AI_ENABLED, &enabled);
    tracing::info!("AI 自动生成: {}（已持久化）", if enabled { "已启用" } else { "已禁用" });
    Ok(())
}

/// 手动为指定快照生成 commit msg
///
/// 参数：
/// - `snapshot_id`: 快照 ID
///
/// 返回生成的 Conventional Commits 格式 msg
#[tauri::command]
pub async fn generate_commit_msg(
    snapshot_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    // 获取 LLM 引擎引用（通过 Arc 共享）
    let engine_arc = {
        let lock = state.ai_engine.lock().unwrap();
        match lock.as_ref() {
            Some(_) => {
                // 引擎存在，但我们无法直接获取 &dyn LlmEngine
                // 因为 lock 守卫的生命周期问题
                // 所以这里重新构造一个临时引擎
                let cfg = state.ai_config.lock().unwrap().clone();
                timeflow_ai::create_engine(cfg)?
            }
            None => {
                return Err(crate::AppError::Other(
                    "AI 引擎未初始化，请先配置 LLM".to_string(),
                ))
            }
        }
    };

    // 获取 TimeFlow 引擎
    let tf = {
        let lock = state.timeflow.lock().unwrap();
        match lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    let msg = generate_msg_internal(engine_arc.as_ref(), &snapshot_id, &tf).await?;
    tracing::info!("AI 生成 commit msg: {} -> {}", &snapshot_id[..8.min(snapshot_id.len())], msg);
    Ok(msg)
}

/// 内部使用：在快照后自动生成 commit msg（如果启用）
///
/// 此函数不是 tauri::command，而是供 timeflow.rs 的 create_snapshot 调用。
/// 失败时只记录日志，不影响快照本身。
pub fn try_auto_generate_msg(
    state: &crate::AppState,
    snapshot_id: &str,
    timeflow: &Arc<timeflow_core::TimeFlow>,
) {
    let enabled = *state.ai_enabled.lock().unwrap();
    if !enabled {
        return;
    }

    // 检查引擎是否已配置
    let has_engine = state.ai_engine.lock().unwrap().is_some();
    if !has_engine {
        return;
    }

    // 重建引擎（因为无法持有对 Box<dyn> 的引用）
    let engine = {
        let cfg = state.ai_config.lock().unwrap().clone();
        match timeflow_ai::create_engine(cfg) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("AI 自动生成：引擎创建失败: {}", e);
                return;
            }
        }
    };

    let snapshot_id = snapshot_id.to_string();
    let tf = timeflow.clone();

    // 在后台 tokio task 中执行（避免阻塞快照流程）
    tauri::async_runtime::spawn(async move {
        match generate_msg_internal(engine.as_ref(), &snapshot_id, &tf).await {
            Ok(msg) => {
                tracing::info!("AI 自动生成 commit msg: {} -> {}", &snapshot_id[..8.min(snapshot_id.len())], msg);
            }
            Err(e) => {
                tracing::warn!("AI 自动生成 commit msg 失败: {}", e);
            }
        }
    });
}

async fn generate_msg_internal(
    engine: &dyn LlmEngine,
    snapshot_id: &str,
    tf: &Arc<timeflow_core::TimeFlow>,
) -> crate::AppResult<String> {
    let snapshot_id_owned = snapshot_id.to_string();
    let snapshot = tf.get_snapshot(&snapshot_id_owned)?;

    let diff_text = if let Some(parent_id) = &snapshot.parent {
        let diff = tf.diff(parent_id, &snapshot_id_owned)?;
        timeflow_ai::format_diff(&diff)
    } else {
        let tree = tf.storage().read_tree(&snapshot.tree)?;
        let mut text = format!("初始快照，包含 {} 个文件\n\n", tree.entries.len());
        for path in tree.entries.keys() {
            text.push_str(&format!("A {}\n", path.display()));
        }
        text
    };

    let history: Vec<String> = tf
        .list_snapshots(None)?
        .iter()
        .rev()
        .take(5)
        .filter_map(|s| s.metadata.ai_commit_msg.clone())
        .collect();

    let generator = CommitMsgGenerator::new(engine);
    let msg = generator.generate(&diff_text, &history).await?;
    Ok(msg)
}

// ===== W6 M2.2 D5: 语义搜索 + 候选版本识别 =====

/// 语义搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultInfo {
    /// 快照 ID
    pub snapshot_id: String,
    /// 相似度分数（0.0-1.0）
    pub score: f32,
    /// 快照的 commit msg
    pub message: String,
    /// 快照时间戳
    pub timestamp: String,
    /// 快照类型
    pub snapshot_type: String,
}

/// 候选版本评分信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScoreInfo {
    /// 快照 ID
    pub snapshot_id: String,
    /// 总分（0-100）
    pub score: u32,
    /// 构建状态得分
    pub build_score: u32,
    /// 改动规模得分
    pub scale_score: u32,
    /// 文件多样性得分
    pub diversity_score: u32,
    /// commit msg 质量得分
    pub message_score: u32,
    /// 推荐分类
    pub recommended_type: String,
    /// 评分理由
    pub reason: String,
    /// commit msg
    pub message: String,
    /// 快照时间戳
    pub timestamp: String,
}

/// 语义搜索快照
///
/// 用自然语言查询快照，例如 "回到加登录的版本"
#[tauri::command]
pub async fn semantic_search(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<SearchResultInfo>> {
    let tf = {
        let lock = state.timeflow.lock().unwrap();
        match lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    // 获取所有快照
    let snapshots = tf.list_snapshots(None)?;
    if snapshots.is_empty() {
        return Ok(vec![]);
    }

    // 构建 embedding 索引（每次搜索重建，简化实现）
    // 生产环境应缓存索引，快照变更时增量更新
    let engine: Box<dyn timeflow_ai::EmbeddingEngine> = {
        let cfg = state.ai_config.lock().unwrap().clone();
        match timeflow_ai::create_embedding_engine(&cfg) {
            Ok(e) => e,
            Err(_) => Box::new(MockEmbeddingEngine::new()),
        }
    };

    let mut index = SearchIndex::new(engine);
    for snap in &snapshots {
        let text = snap
            .metadata
            .ai_commit_msg
            .clone()
            .or_else(|| snap.metadata.ai_summary.clone())
            .unwrap_or_else(|| format!("快照 {}", &snap.id[..8.min(snap.id.len())]));
        if let Err(e) = index.add(&snap.id, &text).await {
            tracing::warn!("索引快照 {} 失败: {}", &snap.id[..8], e);
        }
    }

    let k = limit.unwrap_or(10);
    let results = index.search(&query, k).await?;

    // 转换为前端格式
    let mut out = Vec::with_capacity(results.len());
    for r in results {
        if let Ok(snap) = tf.get_snapshot(&r.snapshot_id) {
            out.push(SearchResultInfo {
                snapshot_id: r.snapshot_id,
                score: r.score,
                message: r.message,
                timestamp: snap.timestamp.to_rfc3339(),
                snapshot_type: format!("{:?}", snap.metadata.snapshot_type).to_lowercase(),
            });
        }
    }
    Ok(out)
}

/// 识别候选版本
///
/// 扫描所有快照，返回评分 >= 70 的候选版本列表
#[tauri::command]
pub async fn detect_candidates(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<CandidateScoreInfo>> {
    let tf = {
        let lock = state.timeflow.lock().unwrap();
        match lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    let snapshots = tf.list_snapshots(None)?;
    let detector = CandidateDetector::new();

    let mut out = Vec::new();
    for snap in &snapshots {
        // 构建 ClassifyInput
        let files = if let Some(parent_id) = &snap.parent {
            match tf.diff(parent_id, &snap.id) {
                Ok(d) => d.files,
                Err(_) => vec![],
            }
        } else {
            // 初始快照：从 tree 构造文件列表
            match tf.storage().read_tree(&snap.tree) {
                Ok(tree) => tree
                    .entries
                    .keys()
                    .map(|p| FileChange {
                        path: p.clone(),
                        status: ChangeStatus::Added,
                        additions: 0,
                        deletions: 0,
                    })
                    .collect(),
                Err(_) => vec![],
            }
        };

        let message = snap
            .metadata
            .ai_commit_msg
            .clone()
            .unwrap_or_else(|| format!("快照 {}", &snap.id[..8.min(snap.id.len())]));

        let input = ClassifyInput {
            snapshot_id: snap.id.clone(),
            message,
            build_status: snap.metadata.build_status.clone(),
            files,
            is_manual: snap.metadata.trigger == timeflow_core::TriggerType::Manual,
        };

        let score = detector.score(&input);
        // 只返回候选版本（score >= 70）
        if score.recommended_type == SnapshotType::Candidate {
            out.push(CandidateScoreInfo {
                snapshot_id: score.snapshot_id,
                score: score.score,
                build_score: score.dimensions.build,
                scale_score: score.dimensions.scale,
                diversity_score: score.dimensions.diversity,
                message_score: score.dimensions.message,
                recommended_type: format!("{:?}", score.recommended_type).to_lowercase(),
                reason: score.reason,
                message: input.message,
                timestamp: snap.timestamp.to_rfc3339(),
            });
        }
    }

    // 按分数降序
    out.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(out)
}

/// 分类单个快照（手动触发）
#[tauri::command]
pub async fn classify_snapshot(
    snapshot_id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let tf = {
        let lock = state.timeflow.lock().unwrap();
        match lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    let snap = tf.get_snapshot(&snapshot_id)?;
    let files = if let Some(parent_id) = &snap.parent {
        tf.diff(parent_id, &snapshot_id)?.files
    } else {
        vec![]
    };

    let message = snap
        .metadata
        .ai_commit_msg
        .clone()
        .unwrap_or_else(|| "(无 commit msg)".to_string());

    let input = ClassifyInput {
        snapshot_id: snapshot_id.clone(),
        message,
        build_status: snap.metadata.build_status.clone(),
        files,
        is_manual: snap.metadata.trigger == timeflow_core::TriggerType::Manual,
    };

    // 优先规则分类，无 LLM 时也能用
    let classifier = SnapshotClassifier::new_rules_only();
    let result = classifier.classify(&input).await?;
    Ok(format!(
        "{:?} (confidence={:.2}): {}",
        result.snapshot_type, result.confidence, result.reason
    ))
}
