// 系统信息命令

use serde::Serialize;

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

fn num_cpus() -> usize {
    // 简单实现，不引入额外依赖
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn get_total_memory() -> u64 {
    // TODO: 跨平台获取内存，先用 0 占位
    0
}
