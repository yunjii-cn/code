// Git 兼容层命令
// W4 M1.4: 三模式切换 + 状态查询 + 手动推送

use serde::Serialize;
use std::path::PathBuf;

use crate::AppResult;
use aw_git::{GitAdapter, GitConfig, GitMode, GitModeManager};

/// Git 状态信息（前端展示用）
#[derive(Serialize)]
pub struct GitStatusInfo {
    /// 当前模式（stealth / sync / release）
    pub mode: String,
    /// Git 仓库是否已初始化
    pub initialized: bool,
    /// 当前 git 分支名
    pub current_branch: Option<String>,
    /// 远程仓库 URL
    pub remote_url: Option<String>,
    /// 最近一次 commit hash
    pub last_commit: Option<String>,
    /// 待推送的 commit 数
    pub unpushed_commits: usize,
}

impl From<aw_git::GitStatus> for GitStatusInfo {
    fn from(s: aw_git::GitStatus) -> Self {
        Self {
            mode: s.mode.to_string(),
            initialized: s.initialized,
            current_branch: s.current_branch,
            remote_url: s.remote_url,
            last_commit: s.last_commit,
            unpushed_commits: s.unpushed_commits,
        }
    }
}

/// 获取当前 Git 模式
#[tauri::command]
pub async fn get_git_mode(state: tauri::State<'_, crate::AppState>) -> AppResult<String> {
    let repo_path_lock = state.repo_path.lock().unwrap();
    match repo_path_lock.as_ref() {
        Some(path) => {
            let mgr = GitModeManager::new(path);
            let mode = mgr.load_mode()?;
            Ok(mode.to_string())
        }
        None => Ok(GitMode::default().to_string()),
    }
}

/// 切换 Git 模式
///
/// 切换时会：
/// 1. 持久化新模式到 .yunji/timeflow/config.toml
/// 2. 重建 GitAdapter 实例
/// 3. 如果新模式是 sync/release，自动 ensure_init
#[tauri::command]
pub async fn set_git_mode(
    mode: String,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let new_mode: GitMode = mode
        .parse()
        .map_err(|e: aw_git::GitError| crate::AppError::Other(e.to_string()))?;

    // 获取仓库路径
    let repo_path = {
        let lock = state.repo_path.lock().unwrap();
        match lock.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Err(crate::AppError::Other(
                    "仓库未初始化，请先调用 init_repository".to_string(),
                ))
            }
        }
    };

    // 持久化模式
    let mgr = GitModeManager::new(&repo_path);
    mgr.save_mode(new_mode)?;

    // 构建新的 adapter
    let config = GitConfig {
        repo_path: repo_path.clone(),
        ..Default::default()
    };
    let adapter = aw_git::create_adapter(new_mode, config);

    // 如果是 sync/release，自动初始化 git 仓库
    if matches!(new_mode, GitMode::Sync | GitMode::Release) {
        adapter.ensure_init(&repo_path)?;
    }

    // 替换 adapter
    {
        let mut lock = state.git_adapter.lock().unwrap();
        *lock = Some(adapter);
    }

    tracing::info!("Git 模式切换: {} -> {}", mode, new_mode);
    Ok(format!("已切换到 {new_mode} 模式"))
}

/// 获取 Git 状态
#[tauri::command]
pub async fn get_git_status(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<GitStatusInfo> {
    // 优先使用已创建的 adapter
    let adapter_lock = state.git_adapter.lock().unwrap();
    if let Some(ref adapter) = *adapter_lock {
        let repo_path = state.repo_path.lock().unwrap();
        let path = repo_path
            .as_ref()
            .ok_or_else(|| crate::AppError::Other("仓库未初始化".to_string()))?;
        let status = adapter.status(path)?;
        return Ok(status.into());
    }
    drop(adapter_lock);

    // 没有 adapter，尝试从配置文件读取模式并创建临时 adapter
    let repo_path = {
        let lock = state.repo_path.lock().unwrap();
        match lock.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Ok(GitStatusInfo {
                    mode: GitMode::Stealth.to_string(),
                    initialized: false,
                    current_branch: None,
                    remote_url: None,
                    last_commit: None,
                    unpushed_commits: 0,
                })
            }
        }
    };

    let mgr = GitModeManager::new(&repo_path);
    let mode = mgr.load_mode().unwrap_or(GitMode::Stealth);
    let config = GitConfig {
        repo_path: repo_path.clone(),
        ..Default::default()
    };
    let adapter = aw_git::create_adapter(mode, config);
    let status = adapter.status(&repo_path)?;
    Ok(status.into())
}

/// 手动推送到远程
#[tauri::command]
pub async fn git_push(state: tauri::State<'_, crate::AppState>) -> AppResult<String> {
    let repo_path = {
        let lock = state.repo_path.lock().unwrap();
        match lock.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Err(crate::AppError::Other(
                    "仓库未初始化".to_string(),
                ))
            }
        }
    };

    // 获取或创建 adapter
    let mut adapter_lock = state.git_adapter.lock().unwrap();
    if adapter_lock.is_none() {
        let mgr = GitModeManager::new(&repo_path);
        let mode = mgr.load_mode().unwrap_or(GitMode::Stealth);
        let config = GitConfig {
            repo_path: repo_path.clone(),
            ..Default::default()
        };
        *adapter_lock = Some(aw_git::create_adapter(mode, config));
    }

    let adapter = adapter_lock
        .as_ref()
        .ok_or_else(|| crate::AppError::Other("Git 适配器未初始化".to_string()))?;

    adapter.push(&repo_path)?;
    Ok("推送成功".to_string())
}

/// 初始化 Git 仓库（仅 sync/release 模式有效）
#[tauri::command]
pub async fn init_git_repo(
    remote_url: Option<String>,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    let repo_path = {
        let lock = state.repo_path.lock().unwrap();
        match lock.as_ref() {
            Some(p) => p.clone(),
            None => {
                return Err(crate::AppError::Other(
                    "仓库未初始化，请先调用 init_repository".to_string(),
                ))
            }
        }
    };

    let config = GitConfig {
        repo_path: repo_path.clone(),
        remote_url,
        ..Default::default()
    };

    // 使用 SyncMode 来初始化（它有完整的 init 逻辑）
    let sync = aw_git::SyncMode::new(config);
    sync.ensure_init(&repo_path)?;

    tracing::info!("Git 仓库已初始化: {}", repo_path.display());
    Ok("Git 仓库已初始化".to_string())
}

/// 内部使用：在 TimeFlow 快照后调用，让 Git 适配器同步
///
/// 注意：此函数不是 tauri::command，而是供 timeflow.rs 的 create_snapshot 调用
pub fn notify_snapshot_created(
    state: &crate::AppState,
    snapshot: &timeflow_core::Snapshot,
    timeflow: &std::sync::Arc<timeflow_core::TimeFlow>,
) -> AppResult<()> {
    let repo_path = {
        let lock = state.repo_path.lock().unwrap();
        match lock.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(()), // 仓库未初始化，跳过
        }
    };

    let mut adapter_lock = state.git_adapter.lock().unwrap();
    if adapter_lock.is_none() {
        // 懒加载 adapter
        let mgr = GitModeManager::new(&repo_path);
        let mode = mgr.load_mode().unwrap_or(GitMode::Stealth);
        let config = GitConfig {
            repo_path: repo_path.clone(),
            ..Default::default()
        };
        *adapter_lock = Some(aw_git::create_adapter(mode, config));
    }

    if let Some(ref adapter) = *adapter_lock {
        adapter.on_snapshot(snapshot, timeflow, &repo_path)?;
    }

    Ok(())
}

/// 让 PathBuf 可序列化（用于内部状态）
#[allow(dead_code)]
fn _pathbuf_serialize_check(p: &PathBuf) -> String {
    p.to_string_lossy().to_string()
}
