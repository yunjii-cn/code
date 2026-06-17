// TimeFlow 版本控制命令
// W1 阶段返回 mock 数据，W2 接入真实 timeflow-core

use serde::Serialize;
use chrono::{DateTime, Utc};

use crate::AppResult;

/// 快照摘要（列表用）
#[derive(Serialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub snapshot_type: String, // wip | progress | candidate | release
    pub build_status: String,  // green | yellow | red | unknown
    pub author: String,
}

/// 快照详情
#[derive(Serialize)]
pub struct SnapshotDetail {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub ai_summary: Option<String>,
    pub snapshot_type: String,
    pub build_status: String,
    pub author: String,
    pub files_changed: Vec<FileChange>,
}

#[derive(Serialize)]
pub struct FileChange {
    pub path: String,
    pub status: String, // added | modified | deleted
    pub additions: u32,
    pub deletions: u32,
}

/// 列出快照
#[tauri::command]
pub async fn list_snapshots(
    limit: Option<usize>,
    _state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<SnapshotSummary>> {
    // W1: 返回 mock 数据，验证前端链路
    let mock = vec![
        SnapshotSummary {
            id: "snap_a3f8b2c1".to_string(),
            timestamp: Utc::now() - chrono::Duration::minutes(30),
            message: "feat(auth): 添加 JWT 登录验证".to_string(),
            snapshot_type: "candidate".to_string(),
            build_status: "green".to_string(),
            author: "you".to_string(),
        },
        SnapshotSummary {
            id: "snap_b7c2d4e9".to_string(),
            timestamp: Utc::now() - chrono::Duration::hours(2),
            message: "WIP: 登录页面布局".to_string(),
            snapshot_type: "wip".to_string(),
            build_status: "yellow".to_string(),
            author: "you".to_string(),
        },
        SnapshotSummary {
            id: "snap_c4f9e8a3".to_string(),
            timestamp: Utc::now() - chrono::Duration::hours(5),
            message: "fix: 修复文件上传大小限制".to_string(),
            snapshot_type: "progress".to_string(),
            build_status: "green".to_string(),
            author: "you".to_string(),
        },
    ];

    let result = match limit {
        Some(n) => mock.into_iter().take(n).collect(),
        None => mock,
    };

    Ok(result)
}

/// 获取快照详情
#[tauri::command]
pub async fn get_snapshot_detail(
    id: String,
    _state: tauri::State<'_, crate::AppState>,
) -> AppResult<SnapshotDetail> {
    // W1: mock
    Ok(SnapshotDetail {
        id: id.clone(),
        parent_id: Some("snap_parent01".to_string()),
        timestamp: Utc::now() - chrono::Duration::minutes(30),
        message: "feat(auth): 添加 JWT 登录验证".to_string(),
        ai_summary: Some("本次变更在 auth 模块添加了 JWT token 生成与验证逻辑，新增 3 个文件，修改 2 个文件".to_string()),
        snapshot_type: "candidate".to_string(),
        build_status: "green".to_string(),
        author: "you".to_string(),
        files_changed: vec![
            FileChange {
                path: "src/auth/jwt.rs".to_string(),
                status: "added".to_string(),
                additions: 87,
                deletions: 0,
            },
            FileChange {
                path: "src/auth/mod.rs".to_string(),
                status: "modified".to_string(),
                additions: 12,
                deletions: 3,
            },
            FileChange {
                path: "src/main.rs".to_string(),
                status: "modified".to_string(),
                additions: 5,
                deletions: 1,
            },
        ],
    })
}

/// 创建快照
#[tauri::command]
pub async fn create_snapshot(
    message: Option<String>,
    _state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    // W2: 接入 timeflow_core::TimeFlow::snapshot()
    let msg = message.unwrap_or_else(|| "手动快照".to_string());
    tracing::info!("创建快照: {}", msg);
    Ok(format!("snap_{}", chrono::Utc::now().timestamp()))
}

/// 回滚到指定快照
#[tauri::command]
pub async fn rollback_snapshot(
    id: String,
    _state: tauri::State<'_, crate::AppState>,
) -> AppResult<bool> {
    // W2: 接入 timeflow_core::TimeFlow::rollback()
    tracing::info!("回滚到快照: {}", id);
    Ok(true)
}
