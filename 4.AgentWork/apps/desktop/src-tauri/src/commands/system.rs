// 系统信息命令

use crate::AppResult;
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_count: usize,
    pub total_memory: u64,
}

/// 获取应用信息
#[tauri::command]
pub async fn get_app_info() -> AppInfo {
    AppInfo {
        name: "云集智能体工作台".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "AI-Native 全链路开发发布工作台".to_string(),
    }
}

/// 获取系统信息
#[tauri::command]
pub async fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_count: num_cpus(),
        total_memory: get_total_memory(),
    }
}

/// 获取默认仓库路径（<exe_dir>/data/workspace）
#[tauri::command]
pub async fn get_default_repo_path(app: tauri::AppHandle) -> AppResult<String> {
    let exe_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::AppError::Other(format!("获取应用目录失败: {}", e)))?;
    let repo_dir = exe_dir.join("data").join("workspace");
    Ok(repo_dir.to_string_lossy().to_string())
}

/// 在系统资源管理器中打开文件夹
#[tauri::command]
pub async fn open_folder(path: String) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    Ok(())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn get_total_memory() -> u64 {
    0
}
