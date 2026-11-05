// aw-mcp-bridge 方法分发：MCP 工具 health / status（只读）
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "aw-mcp-bridge";

// 由 build.rs 注入，缺失时为 "unknown"（不使用当前时间兜底，保证可复现构建）
pub const BUILD_TIME: &str = env!("AW_BUILD_TIME");
pub const DESKTOP_BUILD_TIME: &str = env!("AW_DESKTOP_BUILD_TIME");

pub struct Server {
    settings_path: PathBuf,
}

impl Server {
    pub fn new(settings_path: PathBuf) -> Self {
        Self { settings_path }
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    /// 逐行入口：解析失败也必须返回 -32700，而不是静默丢弃
    pub fn handle_line(&self, line: &str) -> Option<Value> {
        match serde_json::from_str::<Value>(line) {
            Ok(message) => self.handle_message(&message),
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                &format!("JSON 解析失败：{error}"),
            )),
        }
    }

    pub fn handle_message(&self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = match message.get("method").and_then(|value| value.as_str()) {
            Some(method) => method,
            None => {
                return id.map(|id| error_response(id, -32600, "缺少 method 字段"));
            }
        };

        // 通知（无 id）不产生响应
        let id = id?;

        let result = match method {
            "initialize" => Some(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                    "title": "AgentWork MCP Bridge"
                }
            })),
            "ping" => Some(json!({})),
            "tools/list" => Some(json!({ "tools": tools() })),
            "tools/call" => Some(self.call_tool(message.get("params"))),
            _ => None,
        };

        Some(match result {
            Some(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            None => error_response(id, -32601, &format!("未知方法：{method}")),
        })
    }

    fn call_tool(&self, params: Option<&Value>) -> Value {
        let name = params
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let payload = match name {
            "health" => Some(self.health_payload()),
            "status" => Some(self.status_payload()),
            _ => None,
        };
        match payload {
            Some(payload) => tool_result(&payload, false),
            None => tool_result(
                &json!({ "error": format!("未知工具：{name}") }),
                true,
            ),
        }
    }

    fn health_payload(&self) -> Value {
        json!({
            "status": "ok",
            "server": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "buildTime": BUILD_TIME
            }
        })
    }

    fn status_payload(&self) -> Value {
        json!({
            "desktop": summarize_settings(&self.settings_path),
            "server": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "buildTime": BUILD_TIME,
                "desktopBuildTime": DESKTOP_BUILD_TIME
            }
        })
    }
}

fn tools() -> Value {
    json!([
        {
            "name": "health",
            "description": "返回 MCP 桥存活状态、版本与构建时间（只读）",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "status",
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }
    ])
}

fn tool_result(payload: &Value, is_error: bool) -> Value {
    let text = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

/// 只读汇总桌面端 settings.json；仅暴露布尔/计数，任何配置值都不回显
fn summarize_settings(path: &Path) -> Value {
    let parsed = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .filter(|value| value.is_object());

    match parsed {
        Some(settings) => json!({
            "settingsPresent": true,
            "aiEnabled": settings.get("ai_enabled").and_then(|value| value.as_bool()),
            "enabledModelsCount": settings
                .get("enabled_models")
                .and_then(|value| value.as_array())
                .map(|models| models.len()),
            "repoPathConfigured": settings
                .get("repo_path")
                .and_then(|value| value.as_str())
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
            "aiConfigPresent": settings
                .get("ai_config")
                .map(|value| !value.is_null())
                .unwrap_or(false)
        }),
        None => json!({
            "settingsPresent": false,
            "aiEnabled": null,
            "enabledModelsCount": null,
            "repoPathConfigured": false,
            "aiConfigPresent": false
        }),
    }
}
