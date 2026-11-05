// aw-mcp-bridge 契约测试：以 MCP 客户端的视角驱动最小服务器
use aw_mcp_bridge::Server;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn server_with_settings(content: Option<&str>) -> (Server, TempDir) {
    let dir = TempDir::new().expect("创建临时目录");
    let path: PathBuf = dir.path().join("settings.json");
    if let Some(content) = content {
        fs::write(&path, content).expect("写入临时 settings.json");
    }
    (Server::new(path), dir)
}

fn call(server: &Server, message: Value) -> Value {
    server
        .handle_message(&message)
        .unwrap_or_else(|| panic!("请求 {} 应当产生响应", message))
}

fn tool_text(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tools/call 应返回文本内容：{response}"))
        .to_string()
}

fn tool_payload(response: &Value) -> Value {
    serde_json::from_str(&tool_text(response)).expect("工具文本内容应为 JSON")
}

#[test]
fn initialize_reports_protocol_and_server_info() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    let version = response["result"]["protocolVersion"]
        .as_str()
        .expect("必须返回 protocolVersion");
    assert!(!version.is_empty(), "protocolVersion 不能为空");
    assert_eq!(response["result"]["serverInfo"]["name"], "aw-mcp-bridge");
    assert_eq!(
        response["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        response["result"]["capabilities"]["tools"].is_object(),
        "必须声明 tools 能力"
    );
}

#[test]
fn tools_list_exposes_health_and_status_only() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names, vec!["health", "status"]);
    for tool in tools {
        assert!(
            tool["description"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个工具都要有描述"
        );
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "inputSchema 必须是 object"
        );
    }
}

#[test]
fn health_reports_version_and_build_time() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "health", "arguments": {}}}),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["server"]["name"], "aw-mcp-bridge");
    assert_eq!(payload["server"]["version"], env!("CARGO_PKG_VERSION"));
    let build_time = payload["server"]["buildTime"]
        .as_str()
        .expect("必须返回 buildTime");
    assert!(!build_time.is_empty(), "buildTime 不能为空");
}

#[test]
fn status_summarizes_desktop_state_without_leaking_values() {
    let settings = json!({
        "ai_enabled": true,
        "enabled_models": ["model-a", "model-b"],
        "repo_path": "C:/secret/private-repo",
        "ai_config": {"api_key": "sk-live-should-not-leak", "base_url": "https://secret.example"}
    })
    .to_string();
    let (server, _dir) = server_with_settings(Some(&settings));
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "status", "arguments": {}}}),
    );
    assert_eq!(response["result"]["isError"], false);
    let text = tool_text(&response);
    let payload = tool_payload(&response);
    assert_eq!(payload["desktop"]["settingsPresent"], true);
    assert_eq!(payload["desktop"]["aiEnabled"], true);
    assert_eq!(payload["desktop"]["enabledModelsCount"], 2);
    assert_eq!(payload["desktop"]["repoPathConfigured"], true);
    assert_eq!(payload["desktop"]["aiConfigPresent"], true);
    for forbidden in [
        "sk-live-should-not-leak",
        "https://secret.example",
        "C:/secret/private-repo",
    ] {
        assert!(
            !text.contains(forbidden),
            "只读汇总不得回显敏感值：{forbidden}"
        );
    }
}

#[test]
fn status_reports_missing_settings_instead_of_failing() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": {"name": "status", "arguments": {}}}),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["desktop"]["settingsPresent"], false);
    assert_eq!(payload["desktop"]["aiEnabled"], Value::Null);
}

#[test]
fn unknown_method_returns_method_not_found() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 6, "method": "aw/unknown"}),
    );
    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn unknown_tool_call_returns_tool_error_content() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": "not-a-tool", "arguments": {}}}),
    );
    assert_eq!(response["result"]["isError"], true);
    let text = tool_text(&response);
    assert!(text.contains("not-a-tool"), "错误需指明未知工具：{text}");
}

#[test]
fn malformed_line_yields_parse_error_response() {
    let (server, _dir) = server_with_settings(None);
    let response = server
        .handle_line("{not-json")
        .expect("非法 JSON 也应产生错误响应");
    assert_eq!(response["error"]["code"], -32700);
    assert_eq!(response["id"], Value::Null);
}

#[test]
fn notifications_do_not_produce_responses() {
    let (server, _dir) = server_with_settings(None);
    let response = server.handle_message(&json!({
        "jsonrpc": "2.0", "method": "notifications/initialized"
    }));
    assert!(response.is_none(), "通知不应产生响应");
}
