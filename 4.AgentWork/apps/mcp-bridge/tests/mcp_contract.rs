// aw-mcp-bridge 契约测试：以 MCP 客户端的视角驱动最小服务器
use aw_mcp_bridge::Server;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
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
fn tools_list_exposes_read_and_execution_tools() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(
        names,
        vec!["health", "status", "task.execute", "receipt.get"],
        "执行层至少需要接单与回执读取两个入口"
    );
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

// ---- 执行层：接单 → 执行 → 可审计回执 ----

fn server_with_runs() -> (Server, TempDir, TempDir) {
    let settings_dir = TempDir::new().expect("创建设置目录");
    let runs_dir = TempDir::new().expect("创建运行目录");
    let server = Server::with_runs_dir(
        settings_dir.path().join("settings.json"),
        runs_dir.path().to_path_buf(),
    );
    (server, settings_dir, runs_dir)
}

fn call_tool(server: &Server, name: &str, arguments: Value) -> Value {
    call(
        server,
        json!({
            "jsonrpc": "2.0", "id": 99, "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        }),
    )
}

fn sha256_prefixed(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).expect("读取产物");
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

#[test]
fn execute_returns_auditable_handback() {
    let (server, _settings, runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-1", "cmd": "echo done", "timeout_secs": 30 }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["task_id"], "t-1");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["exit_code"], 0);
    assert_eq!(payload["verify_cmd"], "echo done");
    assert!(
        payload["duration_ms"].as_u64().is_some(),
        "回执必须包含耗时"
    );

    let stdout_file = runs.path().join("t-1").join("stdout.txt");
    assert!(stdout_file.exists(), "stdout 必须落盘作为证据");
    assert_eq!(
        payload["stdout_digest"].as_str().unwrap(),
        sha256_prefixed(&stdout_file)
    );

    let artifacts = payload["artifacts"].as_array().expect("artifacts 必须是数组");
    assert_eq!(artifacts.len(), 2, "stdout 与 stderr 都应作为证据");
    assert_eq!(artifacts[0]["locator"], "runs/t-1/stdout.txt");
    assert_eq!(
        artifacts[0]["hash"].as_str().unwrap(),
        payload["stdout_digest"].as_str().unwrap(),
        "证据哈希必须与摘要一致"
    );
    for artifact in artifacts {
        let locator = artifact["locator"].as_str().expect("locator 必须是字符串");
        assert!(
            !locator.starts_with('/') && !locator.contains(':'),
            "locator 必须是相对路径，实际: {locator}"
        );
    }
    assert!(
        runs.path().join("t-1").join("handback.json").exists(),
        "回执必须持久化，供 PM 读取"
    );
}

#[test]
fn execute_nonzero_exit_reports_failed_with_exit_code() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-2", "cmd": "exit 3" }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "failed");
    assert_eq!(payload["exit_code"], 3);
}

#[test]
fn execute_timeout_is_blocked_with_reason() {
    #[cfg(windows)]
    let cmd = "ping -n 4 127.0.0.1 > NUL";
    #[cfg(not(windows))]
    let cmd = "sleep 3";

    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-3", "cmd": cmd, "timeout_secs": 1 }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "blocked");
    assert!(
        payload["blocked_reason"]
            .as_str()
            .map(|reason| reason.contains("timeout"))
            .unwrap_or(false),
        "超时必须给出具名原因"
    );
}

#[test]
fn receipt_get_reads_persisted_handback() {
    let (server, _settings, _runs) = server_with_runs();
    call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-4", "cmd": "echo audit" }),
    );
    let response = call_tool(&server, "receipt.get", json!({ "task_id": "t-4" }));
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["task_id"], "t-4");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["exit_code"], 0);
}

#[test]
fn invalid_task_id_is_rejected_before_execution() {
    let (server, _settings, runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "../evil", "cmd": "echo nope" }),
    );
    assert_eq!(response["result"]["isError"], true);
    assert!(tool_text(&response).contains("task_id"));
    assert!(!runs.path().join("evil").exists(), "非法 task_id 不得落盘");
}

#[test]
fn missing_cmd_is_rejected() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(&server, "task.execute", json!({ "task_id": "t-5" }));
    assert_eq!(response["result"]["isError"], true);
    assert!(tool_text(&response).contains("cmd"));
}
