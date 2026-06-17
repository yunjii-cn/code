// AgentWork 桌面端 - Tauri 库
// 所有 Tauri command 和后端逻辑放这里

use serde::Serialize;
use tracing_subscriber;

mod commands;

/// 应用初始化错误
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("TimeFlow 错误: {0}")]
    Timeflow(#[from] timeflow_core::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
}

// 让 AppError 可序列化传给前端
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// 应用全局状态
pub struct AppState {
    /// TimeFlow 引擎实例（W2 接入真实实现）
    pub timeflow: std::sync::Mutex<Option<timeflow_core::TimeFlow>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            timeflow: std::sync::Mutex::new(None),
        }
    }
}

/// 启动 Tauri 应用
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,aw_desktop=debug")),
        )
        .with_target(true)
        .init();

    tracing::info!("AgentWork 桌面端启动中...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // 系统命令
            commands::system::get_app_info,
            commands::system::get_system_info,
            // TimeFlow 命令（W2 实现真实逻辑）
            commands::timeflow::list_snapshots,
            commands::timeflow::get_snapshot_detail,
            commands::timeflow::create_snapshot,
            commands::timeflow::rollback_snapshot,
        ])
        .setup(|_app| {
            tracing::info!("AgentWork 桌面端启动完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}
