// TimeFlow 版本控制命令
// W2: 接入真实 timeflow-core 引擎
// W3: 添加文件监控 + 自动快照

use serde::Serialize;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;

use crate::AppResult;
use timeflow_core::{BranchOps, SnapshotOps, SnapshotFilter, Watcher, WatcherConfig};

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

/// 分支信息
#[derive(Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub head: String,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// 监控器状态
#[derive(Serialize)]
pub struct WatcherStatus {
    pub running: bool,
    pub repo_path: String,
}

/// 初始化 TimeFlow 仓库
#[tauri::command]
pub async fn init_repository(
    path: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let repo_path = PathBuf::from(&path);
    if !repo_path.exists() {
        return Err(crate::AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("路径不存在: {path}"),
        )));
    }

    let timeflow = Arc::new(timeflow_core::TimeFlow::new(&repo_path)?);
    let branch = timeflow.current_branch()?;

    // 保存 repo_path 供 Git 模块使用
    {
        let mut rp_lock = state.repo_path.lock().unwrap();
        *rp_lock = Some(repo_path.clone());
    }

    // 懒加载 Git adapter（读取已保存的模式）
    {
        let mgr = aw_git::GitModeManager::new(&repo_path);
        let mode = mgr.load_mode().unwrap_or(aw_git::GitMode::Stealth);
        let config = aw_git::GitConfig {
            repo_path: repo_path.clone(),
            ..Default::default()
        };
        let adapter = aw_git::create_adapter(mode, config);
        let mut ga_lock = state.git_adapter.lock().unwrap();
        *ga_lock = Some(adapter);
    }

    let mut tf_lock = state.timeflow.lock().unwrap();
    *tf_lock = Some(timeflow);

    tracing::info!("TimeFlow 仓库已初始化: {} (分支: {})", path, branch);
    Ok(format!("仓库已初始化，当前分支: {branch}"))
}

/// 列出快照
#[tauri::command]
pub async fn list_snapshots(
    limit: Option<usize>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<SnapshotSummary>> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化，请先调用 init_repository".to_string())),
    };
    drop(tf_lock);

    let filter = SnapshotFilter {
        limit,
        ..Default::default()
    };
    let snapshots = tf.list_snapshots(Some(filter))?;

    let result: Vec<SnapshotSummary> = snapshots
        .into_iter()
        .map(|s| SnapshotSummary {
            id: s.id,
            timestamp: s.timestamp,
            message: s.metadata.ai_commit_msg.unwrap_or_else(|| "(无消息)".to_string()),
            snapshot_type: format!("{:?}", s.metadata.snapshot_type).to_lowercase(),
            build_status: format!("{:?}", s.metadata.build_status).to_lowercase(),
            author: s.author,
        })
        .collect();

    Ok(result)
}

/// 获取快照详情
#[tauri::command]
pub async fn get_snapshot_detail(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<SnapshotDetail> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    let snapshot = tf.get_snapshot(&id)?;

    // 计算与父快照的差异
    let files_changed = if let Some(parent_id) = &snapshot.parent {
        let diff = tf.diff(parent_id, &id)?;
        diff.files
            .into_iter()
            .map(|f| FileChange {
                path: f.path.to_string_lossy().to_string(),
                status: format!("{:?}", f.status).to_lowercase(),
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect()
    } else {
        // 初始快照：所有文件都是新增
        let tree = tf.storage().read_tree(&snapshot.tree)?;
        tree.entries
            .keys()
            .map(|p| FileChange {
                path: p.to_string_lossy().to_string(),
                status: "added".to_string(),
                additions: 0,
                deletions: 0,
            })
            .collect()
    };

    Ok(SnapshotDetail {
        id: snapshot.id,
        parent_id: snapshot.parent,
        timestamp: snapshot.timestamp,
        message: snapshot
            .metadata
            .ai_commit_msg
            .unwrap_or_else(|| "(无消息)".to_string()),
        ai_summary: snapshot.metadata.ai_summary,
        snapshot_type: format!("{:?}", snapshot.metadata.snapshot_type).to_lowercase(),
        build_status: format!("{:?}", snapshot.metadata.build_status).to_lowercase(),
        author: snapshot.author,
        files_changed,
    })
}

/// 创建快照
#[tauri::command]
pub async fn create_snapshot(
    message: Option<String>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let tf = {
        let tf_lock = state.timeflow.lock().unwrap();
        match tf_lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    let msg = message.as_deref();
    let snapshot_id = tf.snapshot(msg)?;
    tracing::info!("创建快照: {}", snapshot_id);

    // 通知 Git 适配器（如果是 sync/release 模式，会自动镜像到 git）
    let snapshot = tf.get_snapshot(&snapshot_id)?;
    if let Err(e) = crate::commands::git::notify_snapshot_created(&state, &snapshot, &tf) {
        tracing::warn!("Git 适配器同步失败（不影响 TimeFlow 快照）: {}", e);
    }

    // AI 自动生成 commit msg（如果启用）
    crate::commands::ai::try_auto_generate_msg(&state, &snapshot_id, &tf);

    Ok(snapshot_id)
}

/// 回滚到指定快照
#[tauri::command]
pub async fn rollback_snapshot(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    let new_snapshot_id = tf.rollback(&id)?;
    tracing::info!("回滚到快照: {} (新快照: {})", id, new_snapshot_id);
    Ok(new_snapshot_id)
}

/// 计算两快照差异
#[tauri::command]
pub async fn diff_snapshots(
    from: String,
    to: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<FileChange>> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    let diff = tf.diff(&from, &to)?;
    let result: Vec<FileChange> = diff
        .files
        .into_iter()
        .map(|f| FileChange {
            path: f.path.to_string_lossy().to_string(),
            status: format!("{:?}", f.status).to_lowercase(),
            additions: f.additions,
            deletions: f.deletions,
        })
        .collect();

    Ok(result)
}

/// 列出所有分支
#[tauri::command]
pub async fn list_branches(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<Vec<BranchInfo>> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    let current = tf.current_branch()?;
    let branches = tf.list_branches()?;

    let result: Vec<BranchInfo> = branches
        .into_iter()
        .map(|b| BranchInfo {
            is_current: b.name == current,
            name: b.name,
            head: b.head,
            created_at: b.created_at,
        })
        .collect();

    Ok(result)
}

/// 获取当前分支
#[tauri::command]
pub async fn current_branch(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    Ok(tf.current_branch()?)
}

/// 创建分支
#[tauri::command]
pub async fn create_branch(
    name: String,
    from: Option<String>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    // 如果未指定 from，使用当前 HEAD
    let from_snapshot = match from {
        Some(id) => id,
        None => {
            let branch_name = tf.current_branch()?;
            let branch = tf.list_branches()?
                .into_iter()
                .find(|b| b.name == branch_name)
                .ok_or_else(|| crate::AppError::Other("当前分支不存在".to_string()))?;
            branch.head
        }
    };

    let branch = tf.create_branch(&name, &from_snapshot)?;
    tracing::info!("创建分支: {}", name);
    Ok(branch.name)
}

/// 切换分支
#[tauri::command]
pub async fn switch_branch(
    name: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<()> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    tf.switch_branch(&name)?;
    tracing::info!("切换到分支: {}", name);
    Ok(())
}

/// 删除分支
#[tauri::command]
pub async fn delete_branch(
    name: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<()> {
    let tf_lock = state.timeflow.lock().unwrap();
    let tf = match tf_lock.as_ref() {
        Some(tf) => tf.clone(),
        None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
    };
    drop(tf_lock);

    tf.delete_branch(&name)?;
    tracing::info!("删除分支: {}", name);
    Ok(())
}

// ===== 文件监控命令 =====

/// 启动文件监控（自动快照）
#[tauri::command]
pub async fn start_watcher(
    debounce_secs: Option<u64>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    // 检查是否已在运行
    {
        let watcher_lock = state.watcher.lock().unwrap();
        if let Some(ref w) = *watcher_lock {
            if w.is_running() {
                return Err(crate::AppError::Other("监控器已在运行".to_string()));
            }
        }
    }

    // 获取 TimeFlow 引擎
    let tf = {
        let tf_lock = state.timeflow.lock().unwrap();
        match tf_lock.as_ref() {
            Some(tf) => tf.clone(),
            None => return Err(crate::AppError::Other("仓库未初始化".to_string())),
        }
    };

    let config = WatcherConfig {
        debounce_secs: debounce_secs.unwrap_or(timeflow_core::DEFAULT_DEBOUNCE_SECS),
        ..Default::default()
    };

    let watcher = Watcher::start(tf, config)?;
    let repo_path = watcher.repo_path().to_string_lossy().to_string();
    let msg = format!("监控器已启动: {} (防抖={}s)", repo_path, debounce_secs.unwrap_or(timeflow_core::DEFAULT_DEBOUNCE_SECS));

    let mut watcher_lock = state.watcher.lock().unwrap();
    *watcher_lock = Some(watcher);

    Ok(msg)
}

/// 停止文件监控
#[tauri::command]
pub async fn stop_watcher(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<()> {
    let mut watcher_lock = state.watcher.lock().unwrap();
    if let Some(ref w) = *watcher_lock {
        w.stop();
    }
    *watcher_lock = None;
    tracing::info!("监控器已停止");
    Ok(())
}

/// 获取监控器状态
#[tauri::command]
pub async fn watcher_status(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<WatcherStatus> {
    let watcher_lock = state.watcher.lock().unwrap();
    match watcher_lock.as_ref() {
        Some(w) if w.is_running() => Ok(WatcherStatus {
            running: true,
            repo_path: w.repo_path().to_string_lossy().to_string(),
        }),
        _ => Ok(WatcherStatus {
            running: false,
            repo_path: String::new(),
        }),
    }
}
