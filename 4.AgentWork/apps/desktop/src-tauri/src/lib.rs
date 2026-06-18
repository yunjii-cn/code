// AgentWork 桌面端 - Tauri 库
// 所有 Tauri command 和后端逻辑放这里

use serde::Serialize;

mod commands;

/// 应用初始化错误
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("TimeFlow 错误: {0}")]
    Timeflow(#[from] timeflow_core::Error),
    #[error("Git 兼容层错误: {0}")]
    Git(#[from] aw_git::GitError),
    #[error("AI 错误: {0}")]
    Ai(#[from] timeflow_ai::AiError),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
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
    /// TimeFlow 引擎实例
    pub timeflow: std::sync::Mutex<Option<std::sync::Arc<timeflow_core::TimeFlow>>>,
    /// 文件监控器
    pub watcher: std::sync::Mutex<Option<timeflow_core::Watcher>>,
    /// Git 兼容层适配器（Box<dyn> 因为 trait object）
    pub git_adapter: std::sync::Mutex<Option<Box<dyn aw_git::GitAdapter>>>,
    /// 当前仓库路径（用于 GitModeManager）
    pub repo_path: std::sync::Mutex<Option<std::path::PathBuf>>,
    /// AI 引擎（可选，未配置时为 None）
    pub ai_engine: std::sync::Mutex<Option<Box<dyn timeflow_ai::LlmEngine>>>,
    /// AI 配置
    pub ai_config: std::sync::Mutex<timeflow_ai::AiConfig>,
    /// 是否启用 AI 自动生成 commit msg
    pub ai_enabled: std::sync::Mutex<bool>,
    /// 团队协作状态（W7 M3.1）
    pub team: commands::team::TeamState,
}

impl Default for AppState {
    fn default() -> Self {
        // 读取 LLM Gateway 环境变量（M4.0 D7）
        // 若设置 LLM_GATEWAY_URL，则默认 ai_config 走 Gateway 反代
        let gateway_url = std::env::var("LLM_GATEWAY_URL").ok().filter(|s| !s.is_empty());
        let gateway_token = std::env::var("LLM_GATEWAY_TOKEN").ok().filter(|s| !s.is_empty());

        let ai_config = if let Some(url) = gateway_url.as_deref() {
            tracing::info!(
                "LLM Gateway 已配置（LLM_GATEWAY_URL={}），默认 AI 配置走 Gateway",
                url
            );
            timeflow_ai::AiConfig {
                provider: timeflow_ai::LlmProvider::OpenAi,
                base_url: url.trim_end_matches('/').to_string(),
                api_key: gateway_token.clone(),
                model: "glm5.2".to_string(), // 默认走智谱 GLM（Gateway 反代）
                ..Default::default()
            }
        } else {
            timeflow_ai::AiConfig::default()
        };

        Self {
            timeflow: std::sync::Mutex::new(None),
            watcher: std::sync::Mutex::new(None),
            git_adapter: std::sync::Mutex::new(None),
            repo_path: std::sync::Mutex::new(None),
            ai_engine: std::sync::Mutex::new(None),
            ai_config: std::sync::Mutex::new(ai_config),
            ai_enabled: std::sync::Mutex::new(false),
            team: commands::team::TeamState::default(),
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
            // TimeFlow 命令
            commands::timeflow::init_repository,
            commands::timeflow::list_snapshots,
            commands::timeflow::get_snapshot_detail,
            commands::timeflow::create_snapshot,
            commands::timeflow::rollback_snapshot,
            commands::timeflow::diff_snapshots,
            commands::timeflow::list_branches,
            commands::timeflow::current_branch,
            commands::timeflow::create_branch,
            commands::timeflow::switch_branch,
            commands::timeflow::delete_branch,
            // 文件监控命令
            commands::timeflow::start_watcher,
            commands::timeflow::stop_watcher,
            commands::timeflow::watcher_status,
            // Git 兼容层命令（W4 M1.4）
            commands::git::get_git_mode,
            commands::git::set_git_mode,
            commands::git::get_git_status,
            commands::git::git_push,
            commands::git::init_git_repo,
            // AI 命令（W5 M2.1）
            commands::ai::get_ai_config,
            commands::ai::set_ai_config,
            commands::ai::set_ai_enabled,
            commands::ai::generate_commit_msg,
            // AI 命令（W6 M2.2）
            commands::ai::semantic_search,
            commands::ai::detect_candidates,
            commands::ai::classify_snapshot,
            // 团队管理命令（W7 M3.1）
            commands::team::list_team_templates,
            commands::team::load_team_template,
            commands::team::get_team_config,
            commands::team::list_models,
            commands::team::register_model,
            commands::team::get_team_status,
            commands::team::execute_team_task,
            // 工作流管理命令（W9 M3.3 D4）
            commands::team::plan_workflow,
            commands::team::get_current_workflow,
            commands::team::step_workflow,
            commands::team::report_task_completed,
            commands::team::report_task_merged,
            commands::team::report_task_verify_failed,
            commands::team::report_task_failed,
            commands::team::reset_workflow,
            // 流式输出命令（M4.0 D3）
            commands::team::stream_agent_thinking,
            commands::team::stream_task_progress,
        ])
        .setup(|_app| {
            tracing::info!("AgentWork 桌面端启动完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}
