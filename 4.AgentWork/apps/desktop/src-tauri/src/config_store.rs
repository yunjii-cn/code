// 配置持久化
// 使用 tauri-plugin-store 将应用配置保存到本地文件
// 配置文件路径: %APPDATA%/ai.yunji.agentwork/settings.json

use serde::{de::DeserializeOwned, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// 配置存储 key
pub const STORE_FILE: &str = "settings.json";

/// 持久化字段 key
pub mod keys {
    pub const AI_CONFIG: &str = "ai_config";
    pub const AI_ENABLED: &str = "ai_enabled";
    pub const ENABLED_MODELS: &str = "enabled_models";
    pub const REPO_PATH: &str = "repo_path";
}

/// 读取配置项
pub fn load<T: DeserializeOwned>(app: &AppHandle, key: &str) -> Option<T> {
    let store = app.store(STORE_FILE).ok()?;
    let value = store.get(key)?;
    serde_json::from_value(value).ok()
}

/// 保存配置项
pub fn save<T: Serialize>(app: &AppHandle, key: &str, value: &T) {
    match app.store(STORE_FILE) {
        Ok(store) => {
            match serde_json::to_value(value) {
                Ok(json_val) => {
                    store.set(key, json_val);
                    // store 自动延迟写入，但我们可以主动 save
                    let _ = store.save();
                }
                Err(e) => {
                    tracing::warn!("配置序列化失败 [{}]: {}", key, e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("配置存储不可用 [{}]: {}", key, e);
        }
    }
}
