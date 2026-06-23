// 系统信息命令

use crate::AppResult;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

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

/// 导出诊断日志包
///
/// 收集应用信息、系统信息、AI 配置（脱敏）、仓库路径、settings.json，
/// 写入一个带时间戳的 JSON 文件，返回文件路径。
#[tauri::command]
pub async fn export_logs(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<String> {
    // 应用信息
    let app_info = get_app_info().await;

    // 系统信息
    let sys_info = get_system_info().await;

    // AI 配置（脱敏：不导出 api_key 明文）
    let (ai_provider, ai_base_url, ai_model, ai_has_key, ai_enabled) = {
        let cfg = state.ai_config.lock().unwrap();
        let enabled = *state.ai_enabled.lock().unwrap();
        (
            cfg.provider.to_string(),
            cfg.base_url.clone(),
            cfg.model.clone(),
            cfg.api_key.is_some(),
            enabled,
        )
    };

    // 仓库路径
    let repo_path = state
        .repo_path
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    // 读取 settings.json 原文（脱敏处理：移除 api_key 字段）
    let settings_json: serde_json::Value = {
        let store = app.store(crate::config_store::STORE_FILE).ok();
        match store {
            Some(store) => {
                let mut obj = serde_json::Map::new();
                for (key, value) in store.entries() {
                    // 跳过可能含密钥的字段
                    if key == crate::config_store::keys::AI_CONFIG {
                        if let serde_json::Value::Object(mut ai_obj) = value {
                            ai_obj.remove("api_key");
                            obj.insert(key, serde_json::Value::Object(ai_obj));
                        } else {
                            obj.insert(key, value);
                        }
                    } else {
                        obj.insert(key, value);
                    }
                }
                serde_json::Value::Object(obj)
            }
            None => serde_json::Value::Null,
        }
    };

    // 环境变量（仅 LLM 相关，脱敏 token）
    let mut env_info = BTreeMap::new();
    if let Ok(v) = std::env::var("LLM_GATEWAY_URL") {
        env_info.insert("LLM_GATEWAY_URL".to_string(), v);
    }
    if std::env::var("LLM_GATEWAY_TOKEN").is_ok() {
        env_info.insert("LLM_GATEWAY_TOKEN".to_string(), "(已设置，已脱敏)".to_string());
    }

    // 组装诊断包
    let bundle = json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "app": {
            "name": app_info.name,
            "version": app_info.version,
            "description": app_info.description,
        },
        "system": {
            "os": sys_info.os,
            "arch": sys_info.arch,
            "cpu_count": sys_info.cpu_count,
            "total_memory": sys_info.total_memory,
        },
        "ai_config": {
            "provider": ai_provider,
            "base_url": ai_base_url,
            "model": ai_model,
            "has_api_key": ai_has_key,
            "enabled": ai_enabled,
        },
        "repository": {
            "path": repo_path,
        },
        "environment": env_info,
        "settings": settings_json,
    });

    // 写入文件
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| crate::AppError::Other(format!("获取应用目录失败: {}", e)))?;
    let logs_dir = data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| crate::AppError::Other(format!("创建日志目录失败: {}", e)))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = format!("diagnostics_{}.json", timestamp);
    let file_path = logs_dir.join(&file_name);

    let content = serde_json::to_string_pretty(&bundle)?;
    std::fs::write(&file_path, content)
        .map_err(|e| crate::AppError::Other(format!("写入日志文件失败: {}", e)))?;

    tracing::info!("诊断日志包已导出: {}", file_path.display());
    Ok(file_path.to_string_lossy().to_string())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn get_total_memory() -> u64 {
    0
}
