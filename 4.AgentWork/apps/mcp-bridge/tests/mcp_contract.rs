// aw-mcp-bridge 契约测试：以 MCP 客户端的视角驱动最小服务器
use aw_mcp_bridge::Server;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
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
        vec![
            "health",
            "status",
            "task.execute",
            "receipt.get",
            "runs.list",
            "runs.prune"
        ],
        "执行层至少需要接单、回执读取、只读枚举与显式清理四个入口"
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
fn env_description_is_fail_closed_like_implementation() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let execute = tools
        .iter()
        .find(|tool| tool["name"] == "task.execute")
        .expect("task.execute 必须存在");
    let env = execute["inputSchema"]["properties"]["env"]["description"]
        .as_str()
        .expect("env 必须有描述");

    // 只查"肯定式"说法：negation（"不会整体覆盖"）是 fail-closed 的正确措辞
    for claim in [
        "PATH 会整体覆盖",
        "PATH 可整体覆盖",
        "会整体覆盖继承值",
        "覆盖继承值；PATH",
    ] {
        assert!(
            !env.contains(claim),
            "描述不得声称 PATH 可整体覆盖（实现是 fail-closed）：命中 `{claim}`；原文：{env}"
        );
    }
    assert!(
        env.contains("不会整体覆盖"),
        "描述必须显式否定整体覆盖，避免调用方误读：{env}"
    );
    assert!(env.contains("PATH"), "描述必须点名 PATH：{env}");
    assert!(
        env.contains("拒绝") || env.contains("不接受"),
        "描述必须写明 PATH 任意大小写变体被拒绝：{env}"
    );
    assert!(
        env.contains("继承") && env.contains("保留"),
        "描述必须写明继承的 PATH 始终保留：{env}"
    );
    assert!(
        env.contains("path_prepend"),
        "描述必须指明追加目录走 path_prepend：{env}"
    );
}

#[test]
fn schema_declares_path_prepend_with_contract_limits() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let execute = tools
        .iter()
        .find(|tool| tool["name"] == "task.execute")
        .expect("task.execute 必须存在");
    let properties = execute["inputSchema"]["properties"]
        .as_object()
        .expect("properties 必须是对象");
    let path_prepend = properties
        .get("path_prepend")
        .expect("schema 必须声明 path_prepend（实现已支持且参与幂等签名）");

    assert_eq!(path_prepend["type"], "array");
    assert_eq!(path_prepend["items"]["type"], "string");
    let description = path_prepend["description"]
        .as_str()
        .expect("path_prepend 必须有描述");
    for keyword in ["字面顺序", "分隔符", "幂等", "16", "4096", "存在性"] {
        assert!(
            description.contains(keyword),
            "path_prepend 描述缺少 `{keyword}`：{description}"
        );
    }
}

#[test]
fn schema_structure_declares_required_fields_and_bounds() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let find = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("缺少工具 {name}"))
    };

    for name in [
        "health",
        "status",
        "task.execute",
        "receipt.get",
        "runs.list",
        "runs.prune",
    ] {
        let schema = &find(name)["inputSchema"];
        assert_eq!(
            schema["type"], "object",
            "{name} 的 inputSchema 必须是 object"
        );
        // 声明性约束：服务端只读已知键、不据此拒绝未知参数，故这里只断言声明本身
        assert_eq!(
            schema["additionalProperties"], false,
            "{name} 必须声明 additionalProperties=false"
        );
        assert!(
            schema["properties"].is_object(),
            "{name} 的 properties 必须是对象"
        );
    }

    let execute = &find("task.execute")["inputSchema"];
    assert_eq!(execute["required"], json!(["task_id", "cmd"]));
    assert_eq!(execute["properties"]["task_id"]["type"], "string");
    assert_eq!(execute["properties"]["cmd"]["type"], "string");
    assert_eq!(execute["properties"]["cwd"]["type"], "string");
    assert_eq!(execute["properties"]["timeout_secs"]["type"], "integer");
    assert_eq!(execute["properties"]["timeout_secs"]["minimum"], 1);
    assert_eq!(execute["properties"]["timeout_secs"]["maximum"], 600);
    assert_eq!(execute["properties"]["env"]["type"], "object");
    assert_eq!(
        execute["properties"]["env"]["additionalProperties"]["type"],
        "string"
    );
    assert_eq!(execute["properties"]["path_prepend"]["type"], "array");
    assert_eq!(execute["properties"]["path_prepend"]["maxItems"], 16);
    assert_eq!(
        execute["properties"]["path_prepend"]["items"]["type"],
        "string"
    );
    assert_eq!(
        execute["properties"]["path_prepend"]["items"]["maxLength"],
        4096
    );

    let receipt = &find("receipt.get")["inputSchema"];
    assert_eq!(receipt["required"], json!(["task_id"]));
    assert_eq!(receipt["properties"]["include_history"]["type"], "boolean");

    let list = &find("runs.list")["inputSchema"];
    assert_eq!(list["properties"]["offset"]["minimum"], 0);
    assert_eq!(list["properties"]["limit"]["minimum"], 1);
    assert_eq!(list["properties"]["limit"]["maximum"], 100);

    let prune = &find("runs.prune")["inputSchema"];
    assert_eq!(prune["required"], json!(["older_than_days"]));
    assert_eq!(prune["properties"]["older_than_days"]["minimum"], 1);
    assert_eq!(prune["properties"]["older_than_days"]["maximum"], 3650);
    assert_eq!(prune["properties"]["dry_run"]["type"], "boolean");
    assert_eq!(prune["properties"]["confirm"]["type"], "boolean");
    assert_eq!(prune["properties"]["limit"]["minimum"], 1);
    assert_eq!(prune["properties"]["limit"]["maximum"], 100);

    for name in ["health", "status"] {
        assert!(
            find(name)["inputSchema"]["properties"]
                .as_object()
                .map(|map| map.is_empty())
                .unwrap_or(false),
            "{name} 不该声明输入参数"
        );
    }
}

#[test]
fn unknown_arguments_are_ignored_by_default() {
    let (server, _settings, _runs) = server_with_runs();
    // v1.18 默认策略：schema 声明 additionalProperties=false 仅用于提示，服务端保持忽略以兼容现有调用方
    let response = call_tool(
        &server,
        "runs.list",
        json!({
            "offset": 0,
            "unexpected": "ignored",
            "dryRun": true,
            "_meta": { "source": "compat-test" }
        }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert!(payload["state"].is_string(), "正常返回状态：{payload}");
    assert!(
        payload["total_tasks"].is_number(),
        "正常返回计数：{payload}"
    );
    assert!(
        !tool_text(&response).contains("unknown_argument"),
        "默认策略是忽略未知参数，不得新增拒绝行为"
    );
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

    let stdout_file = runs.path().join("t-1").join("attempt-1").join("stdout.txt");
    assert!(stdout_file.exists(), "stdout 必须落盘作为证据");
    assert_eq!(
        payload["stdout_digest"].as_str().unwrap(),
        sha256_prefixed(&stdout_file)
    );

    let artifacts = payload["artifacts"].as_array().expect("artifacts 必须是数组");
    assert_eq!(artifacts.len(), 2, "stdout 与 stderr 都应作为证据");
    assert_eq!(artifacts[0]["locator"], "runs/t-1/attempt-1/stdout.txt");
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
        runs.path()
            .join("t-1")
            .join("attempt-1")
            .join("handback.json")
            .exists(),
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
fn missing_cmd_returns_blocked_command_not_found_without_consuming_attempt() {
    let (server, _settings, runs) = server_with_runs();
    let response = call_tool(&server, "task.execute", json!({ "task_id": "t-5" }));
    assert_eq!(
        response["result"]["isError"], false,
        "缺命令应返回 blocked 回执，而不是工具错误"
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "blocked");
    assert!(
        payload["blocked_reason"]
            .as_str()
            .unwrap()
            .starts_with("command_not_found:")
    );
    assert!(payload["attempt"].is_null(), "拒绝发生在占号之前");
    assert!(!runs.path().join("t-5").exists(), "不得创建 attempt 目录");
}

#[test]
fn missing_cwd_returns_blocked_cwd_not_found_without_consuming_attempt() {
    let (server, _settings, runs) = server_with_runs();
    let missing = runs.path().join("no-such-cwd");
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-5b", "cmd": "echo hi", "cwd": missing.to_string_lossy() }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "blocked");
    assert!(
        payload["blocked_reason"]
            .as_str()
            .unwrap()
            .starts_with("cwd_not_found:")
    );
    assert!(payload["attempt"].is_null(), "拒绝发生在占号之前");
    assert!(!runs.path().join("t-5b").exists(), "不得创建 attempt 目录");
}

#[test]
fn duplicate_dispatch_reuses_attempt_without_executing_again() {
    let (server, _settings, runs) = server_with_runs();
    let args = json!({ "task_id": "t-6", "cmd": "exit 3" });

    let first = tool_payload(&call_tool(&server, "task.execute", args.clone()));
    assert_eq!(first["attempt"], 1);
    assert_eq!(first["status"], "failed");

    let stdout_path = runs.path().join("t-6").join("attempt-1").join("stdout.txt");
    let log_before = fs::read(&stdout_path).expect("首次日志应存在");

    let replay = tool_payload(&call_tool(&server, "task.execute", args));
    assert_eq!(replay["attempt"], 1, "重投必须复用既有 attempt");
    assert_eq!(replay["exit_code"], 3);
    assert_eq!(
        replay["stdout_digest"].as_str(),
        first["stdout_digest"].as_str()
    );
    assert_eq!(
        fs::read(&stdout_path).expect("日志仍应存在"),
        log_before,
        "重投不得重新执行或覆盖证据"
    );
    assert!(
        !runs.path().join("t-6").join("attempt-2").exists(),
        "重投不得创建新 attempt 目录"
    );

    let history = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-6", "include_history": true }),
    ));
    assert_eq!(history["attempts"], 1, "重投不得增加执行次数");
}

#[test]
fn same_task_id_with_different_params_is_rejected() {
    let (server, _settings, runs) = server_with_runs();
    call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-8", "cmd": "echo one" }),
    );

    let different_cmd = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-8", "cmd": "echo two" }),
    );
    assert_eq!(different_cmd["result"]["isError"], true);
    assert!(
        tool_text(&different_cmd).contains("task_id_conflict"),
        "异参必须给出具名错误码，实际: {}",
        tool_text(&different_cmd)
    );

    let different_timeout = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-8", "cmd": "echo one", "timeout_secs": 30 }),
    );
    assert_eq!(different_timeout["result"]["isError"], true);
    assert!(tool_text(&different_timeout).contains("task_id_conflict"));

    assert!(
        !runs.path().join("t-8").join("attempt-2").exists(),
        "异参不得静默追加 attempt"
    );
    let latest = tool_payload(&call_tool(&server, "receipt.get", json!({ "task_id": "t-8" })));
    assert_eq!(latest["attempt"], 1);
    assert_eq!(latest["verify_cmd"], "echo one", "旧回执必须保持原参数");
}

#[test]
fn receipt_history_is_still_readable() {
    let (server, _settings, runs) = server_with_runs();
    let task_root = runs.path().join("t-9");
    fs::create_dir_all(&task_root).unwrap();
    for number in 1..=2u64 {
        let dir = task_root.join(format!("attempt-{number}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("handback.json"),
            format!(
                "{{\"task_id\":\"t-9\",\"attempt\":{number},\"status\":\"ok\",\"exit_code\":0,\"duration_ms\":10}}"
            ),
        )
        .unwrap();
    }

    let latest = tool_payload(&call_tool(&server, "receipt.get", json!({ "task_id": "t-9" })));
    assert_eq!(latest["attempt"], 2);
    let history = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-9", "include_history": true }),
    ));
    assert_eq!(history["attempts"], 2);
    let records = history["history"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["attempt"], 1);
    assert_eq!(records[1]["attempt"], 2);
}

#[test]
fn concurrent_duplicate_dispatch_executes_once() {
    let (server, _settings, runs) = server_with_runs();
    let server = Arc::new(server);
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let server = Arc::clone(&server);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            tool_payload(&call(
                &server,
                json!({
                    "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                    "params": {
                        "name": "task.execute",
                        "arguments": { "task_id": "t-7", "cmd": "echo once" }
                    }
                }),
            ))
        }));
    }
    let results: Vec<Value> = handles
        .into_iter()
        .map(|handle| handle.join().expect("并发线程不应 panic"))
        .collect();

    for payload in &results {
        assert_eq!(payload["attempt"], 1, "并发重投只能有一个 attempt");
        assert_eq!(payload["status"], "ok");
        assert_eq!(
            payload["stdout_digest"].as_str(),
            results[0]["stdout_digest"].as_str(),
            "两个调用方应拿到同一份回执证据"
        );
    }

    let history = tool_payload(&call(
        &server,
        json!({
            "jsonrpc": "2.0", "id": 8, "method": "tools/call",
            "params": {
                "name": "receipt.get",
                "arguments": { "task_id": "t-7", "include_history": true }
            }
        }),
    ));
    assert_eq!(history["attempts"], 1, "并发重投不得执行两次");
    assert!(
        !runs.path().join("t-7").join("attempt-2").exists(),
        "并发重投不得创建第二个 attempt"
    );
    let stdout_file = runs.path().join("t-7").join("attempt-1").join("stdout.txt");
    assert!(stdout_file.exists());
    assert_eq!(
        results[0]["stdout_digest"].as_str().unwrap(),
        sha256_prefixed(&stdout_file)
    );
}

// ---- 会话审计：clientInfo 只记录、只展示，不构成身份、不参与准入 ----

/// 读取全部 initialize 审计记录（含旧版单文件 `initialize.json` 与逐份 `initialize-*.json`）
fn initialize_records(runs: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    let Ok(entries) = fs::read_dir(runs.join("_session")) else {
        return records;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_initialize = name == "initialize.json"
            || (name.starts_with("initialize-") && name.ends_with(".json"));
        if !is_initialize {
            continue;
        }
        let Ok(raw) = fs::read_to_string(entry.path()) else {
            continue;
        };
        if let Ok(record) = serde_json::from_str::<Value>(&raw) {
            records.push(record);
        }
    }
    records.sort_by(|left, right| {
        left["receivedAt"]
            .as_str()
            .unwrap_or("")
            .cmp(right["receivedAt"].as_str().unwrap_or(""))
    });
    records
}

#[test]
fn initialize_records_client_info_as_audit_only() {
    let (server, _settings, runs) = server_with_runs();
    let response = call(
        &server,
        json!({
            "jsonrpc": "2.0", "id": 10, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {
                    "name": "pm-dispatcher",
                    "version": "1.2.3",
                    "title": "PM 派单器",
                    "extraField": "kept-as-is"
                }
            }
        }),
    );
    assert_eq!(response["result"]["serverInfo"]["name"], "aw-mcp-bridge");

    let records = initialize_records(runs.path());
    assert_eq!(records.len(), 1, "initialize 必须落审计记录：{records:?}");
    let audit = &records[0];
    assert_eq!(audit["clientInfo"]["name"], "pm-dispatcher");
    assert_eq!(audit["clientInfo"]["version"], "1.2.3");
    assert_eq!(
        audit["clientInfo"]["title"], "PM 派单器",
        "clientInfo 其余字段应原样保留"
    );
    assert_eq!(audit["clientInfo"]["extraField"], "kept-as-is");
    assert!(
        audit["receivedAt"].as_str().map(|value| !value.is_empty()).unwrap_or(false),
        "审计记录必须带接收时间"
    );

    let status = tool_payload(&call_tool(&server, "status", json!({})));
    assert_eq!(
        status["session"]["clientInfo"]["name"], "pm-dispatcher",
        "status 必须只读回显最近一次 initialize 的 clientInfo"
    );
    assert_eq!(status["session"]["clientInfo"]["version"], "1.2.3");
    assert_eq!(status["session"]["clientInfo"]["extraField"], "kept-as-is");
}

#[test]
fn initialize_without_client_info_does_not_crash_and_records_null() {
    let (server, _settings, runs) = server_with_runs();
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 11, "method": "initialize", "params": {}}),
    );
    assert_eq!(
        response["result"]["serverInfo"]["name"], "aw-mcp-bridge",
        "缺 clientInfo 不得影响 initialize 的既有权能"
    );

    let records = initialize_records(runs.path());
    assert_eq!(records.len(), 1, "缺 clientInfo 也要落审计记录");
    assert!(
        records[0]["clientInfo"].is_null(),
        "缺失时记为 null，不得伪造内容"
    );

    let status = tool_payload(&call_tool(&server, "status", json!({})));
    assert!(status["session"]["clientInfo"].is_null());
}

#[test]
fn client_info_is_never_used_for_admission() {
    let (server, _settings, runs) = server_with_runs();
    // 故意给一个"不可信/可疑"的声明：它只应出现在审计记录里，不得改变放行结果
    call(
        &server,
        json!({
            "jsonrpc": "2.0", "id": 12, "method": "initialize",
            "params": { "clientInfo": { "name": "../../untrusted", "version": "" } }
        }),
    );
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-11", "cmd": "echo pass", "cwd": runs.path().to_string_lossy() }),
    );
    assert_eq!(
        response["result"]["isError"], false,
        "clientInfo 不得参与准入判断：工具调用照常放行"
    );
    assert_eq!(tool_payload(&response)["status"], "ok");

    let records = initialize_records(runs.path());
    assert_eq!(records.len(), 1, "initialize 必须落审计记录");
    assert_eq!(records[0]["clientInfo"]["name"], "../../untrusted", "只记录、不解释");
}

#[test]
fn concurrent_initialize_writers_keep_every_record_and_status_aggregates() {
    let settings = TempDir::new().expect("创建设置目录");
    let runs = TempDir::new().expect("创建 runs 目录");
    // 旧版单文件记录：必须被聚合读取，且不得被回填或改写
    let legacy = runs.path().join("_session").join("initialize.json");
    fs::create_dir_all(runs.path().join("_session")).expect("创建审计目录");
    let legacy_text = json!({
        "clientInfo": { "name": "legacy-bridge" },
        "receivedAt": "2026-01-01T00:00:00+00:00"
    })
    .to_string();
    fs::write(&legacy, &legacy_text).expect("写入旧版记录");

    let writers = 6;
    let barrier = Arc::new(Barrier::new(writers));
    let mut handles = Vec::new();
    for index in 0..writers {
        let settings_path = settings.path().join("settings.json");
        let runs_path = runs.path().to_path_buf();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            // 每个 writer 一个独立 Server 实例，模拟多个 bridge 共享同一 runs/
            let server = Server::with_runs_dir(settings_path, runs_path);
            barrier.wait();
            let response = server.handle_message(&json!({
                "jsonrpc": "2.0", "id": index, "method": "initialize",
                "params": { "clientInfo": { "name": format!("writer-{index}") } }
            }));
            assert!(response.is_some(), "并发 initialize 必须应答");
        }));
    }
    for handle in handles {
        handle.join().expect("并发 initialize 不得 panic");
    }

    let records = initialize_records(runs.path());
    let mut names: Vec<String> = records
        .iter()
        .filter_map(|record| record["clientInfo"]["name"].as_str().map(str::to_string))
        .collect();
    names.sort();
    let mut expected: Vec<String> = (0..writers).map(|index| format!("writer-{index}")).collect();
    expected.push("legacy-bridge".to_string());
    expected.sort();
    assert_eq!(
        names, expected,
        "并发写必须保留每一条 initialize 记录，不许静默覆盖：{records:?}"
    );
    assert_eq!(
        fs::read_to_string(&legacy).expect("旧记录仍可读"),
        legacy_text,
        "旧版 initialize.json 不得被回填或改写"
    );

    let server = Server::with_runs_dir(
        settings.path().join("settings.json"),
        runs.path().to_path_buf(),
    );
    let status = tool_payload(&call_tool(&server, "status", json!({})));
    assert_eq!(
        status["session"]["total"], writers + 1,
        "status 必须聚合全部 initialize 记录"
    );
    let listed = status["session"]["records"]
        .as_array()
        .expect("status.session.records 必须是数组");
    assert_eq!(listed.len(), writers + 1);
    assert_eq!(listed[0]["source"], "initialize.json", "聚合必须包含旧版单文件");
    assert!(
        status["session"]["clientInfo"]["name"]
            .as_str()
            .unwrap_or("")
            .starts_with("writer-"),
        "status.session 顶层仍回显最近一条：{status}"
    );
}

// ---- 回执自证：执行上下文必须可复算、截断必须显式 ----

#[test]
fn handback_records_execution_context_for_recompute() {
    let (server, _settings, _runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-12",
            "cmd": "echo ctx",
            "cwd": workdir.path().to_string_lossy(),
            "timeout_secs": 45
        }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);

    let expected_cwd = fs::canonicalize(workdir.path()).expect("规范化工作目录");
    assert_eq!(
        payload["cwd"].as_str(),
        Some(expected_cwd.to_string_lossy().as_ref()),
        "回执必须记录实际执行目录（规范化），PM 才能复算 changed_files / commit"
    );
    assert_eq!(
        payload["timeout_secs"], 45,
        "回执必须记录本次实际生效的超时，供 PM 解释 timeout 原因"
    );
    assert_eq!(
        payload["changed_files_truncated"], false,
        "未截断时必须显式给出 false，而不是缺字段"
    );

    // receipt.get 只读回原记录，不得回填或改写旧回执
    let readback = tool_payload(&call_tool(&server, "receipt.get", json!({ "task_id": "t-12" })));
    assert_eq!(readback["cwd"], payload["cwd"]);
    assert_eq!(readback["timeout_secs"], 45);
    assert_eq!(readback["changed_files_truncated"], false);
}

#[test]
fn changed_files_truncation_is_explicit_not_silent() {
    let (server, _settings, _runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(workdir.path())
            .args(args)
            .output()
            .expect("执行 git");
        assert!(
            output.status.success(),
            "git {:?} 失败：{}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);

    #[cfg(windows)]
    let cmd = "for %i in (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25) do @echo x > f-%i.txt";
    #[cfg(not(windows))]
    let cmd = "for i in $(seq 1 25); do echo x > f-$i.txt; done";

    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-13", "cmd": cmd, "cwd": workdir.path().to_string_lossy() }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    let files = payload["changed_files"]
        .as_array()
        .expect("changed_files 必须是数组");
    assert_eq!(files.len(), 20, "changed_files 仍按既有上限截断为 20");
    assert_eq!(
        payload["changed_files_truncated"], true,
        "超过上限被截断时必须显式告知，不能静默丢弃改动"
    );
}

// ---- env 透传：只追加/覆盖，不推断，不落明文 ----

#[test]
fn env_values_are_passed_to_child() {
    let (server, _settings, runs) = server_with_runs();
    #[cfg(windows)]
    let cmd = "echo %AW_TEST_ENV_VALUE%";
    #[cfg(not(windows))]
    let cmd = "echo $AW_TEST_ENV_VALUE";
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-15",
            "cmd": cmd,
            "env": { "AW_TEST_ENV_VALUE": "visible-value" }
        }),
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok");
    let stdout = fs::read_to_string(
        runs.path()
            .join("t-15")
            .join("attempt-1")
            .join("stdout.txt"),
    )
    .expect("读取 stdout");
    assert_eq!(stdout.trim(), "visible-value", "env 必须对子进程真实可见");
    assert_eq!(
        payload["env_keys"],
        json!(["AW_TEST_ENV_VALUE"]),
        "回执必须列出实际覆盖的 env 键（不含值）"
    );
}

#[test]
fn explicit_env_overrides_inherited_value() {
    let (server, _settings, runs) = server_with_runs();
    std::env::set_var("AW_TEST_INHERITED_VALUE", "from-parent");
    #[cfg(windows)]
    let cmd = "echo %AW_TEST_INHERITED_VALUE%";
    #[cfg(not(windows))]
    let cmd = "echo $AW_TEST_INHERITED_VALUE";

    let inherited = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-16", "cmd": cmd }),
    ));
    assert_eq!(inherited["status"], "ok");
    let inherited_stdout = fs::read_to_string(
        runs.path()
            .join("t-16")
            .join("attempt-1")
            .join("stdout.txt"),
    )
    .expect("读取继承场景 stdout");
    assert_eq!(
        inherited_stdout.trim(),
        "from-parent",
        "缺省 env 时子进程必须继承父进程环境"
    );

    let overridden = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-17",
            "cmd": cmd,
            "env": { "AW_TEST_INHERITED_VALUE": "from-explicit-env" }
        }),
    ));
    assert_eq!(overridden["status"], "ok");
    let overridden_stdout = fs::read_to_string(
        runs.path()
            .join("t-17")
            .join("attempt-1")
            .join("stdout.txt"),
    )
    .expect("读取覆盖场景 stdout");
    assert_eq!(
        overridden_stdout.trim(),
        "from-explicit-env",
        "显式 env 必须覆盖同名继承变量"
    );
    std::env::remove_var("AW_TEST_INHERITED_VALUE");
}

#[test]
fn omitted_env_and_empty_env_map_are_equivalent() {
    let (server, _settings, runs) = server_with_runs();
    let first = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-18", "cmd": "echo same" }),
    ));
    assert_eq!(first["env_keys"], json!([]), "缺省 env 的回执应给出空键列表");

    // 同 task_id 换成显式空 map：行为一致，且签名相同，必须复用同一 attempt
    let second = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-18", "cmd": "echo same", "env": {} }),
    ));
    assert_eq!(second["attempt"], 1);
    assert_eq!(second["env_keys"], json!([]));
    assert!(!runs.path().join("t-18").join("attempt-2").exists());
}

#[test]
fn different_env_is_rejected_as_conflict_without_storing_values() {
    let (server, _settings, runs) = server_with_runs();
    let secret = "s3cr3t-value-9f2a";
    let first = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-19",
            "cmd": "echo env",
            "env": { "AW_TEST_SECRET": secret }
        }),
    );
    assert_eq!(first["result"]["isError"], false);

    let dispatch_path = runs.path().join("t-19").join("attempt-1").join("dispatch.json");
    let dispatch_raw = fs::read_to_string(&dispatch_path).expect("读取派单签名");
    let dispatch: Value = serde_json::from_str(&dispatch_raw).unwrap();
    assert!(
        dispatch["env"]["AW_TEST_SECRET"]
            .as_str()
            .map(|value| value.starts_with("sha256:"))
            .unwrap_or(false),
        "dispatch.json 必须记录 env 摘要，供幂等比对"
    );
    assert!(
        !dispatch_raw.contains(secret),
        "派单签名不得落盘 env 明文（env 常承载密钥）"
    );

    let second = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-19",
            "cmd": "echo env",
            "env": { "AW_TEST_SECRET": "another-value" }
        }),
    );
    assert_eq!(
        second["result"]["isError"], true,
        "同 id 不同 env 必须拒绝，不能静默返回旧回执"
    );
    let text = tool_text(&second);
    assert!(text.contains("task_id_conflict"), "必须给具名错误码：{text}");
    assert!(!runs.path().join("t-19").join("attempt-2").exists());
    let latest = tool_payload(&call_tool(&server, "receipt.get", json!({ "task_id": "t-19" })));
    assert_eq!(latest["attempt"], 1, "拒绝异参不得覆盖既有回执");
}

#[test]
fn invalid_env_is_rejected_before_claiming_attempt() {
    let (server, _settings, runs) = server_with_runs();
    let cases = vec![
        ("t-20a", json!({ "": "empty-key" })),
        ("t-20b", json!({ "A=B": "equals-in-key" })),
        ("t-20c", json!({ "A": 1 })),
        ("t-20d", json!({ "A": "bad\u{0}value" })),
    ];
    for (task_id, env) in cases {
        let response = call_tool(
            &server,
            "task.execute",
            json!({ "task_id": task_id, "cmd": "echo x", "env": env }),
        );
        assert_eq!(response["result"]["isError"], true, "非法 env 必须拒绝：{task_id}");
        let text = tool_text(&response);
        assert!(text.contains("invalid_env"), "必须给具名错误码 invalid_env：{text}");
        assert!(
            !runs.path().join(task_id).exists(),
            "非法 env 必须在占号之前拒绝，不得创建目录"
        );
    }
}

#[test]
fn legacy_dispatch_without_env_is_still_replayed() {
    let (server, _settings, runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let canonical = fs::canonicalize(workdir.path()).expect("规范化工作目录");
    let attempt = runs.path().join("t-21").join("attempt-1");
    fs::create_dir_all(&attempt).expect("创建旧格式 attempt");
    fs::write(
        attempt.join("dispatch.json"),
        json!({
            "task_id": "t-21",
            "cmd": "echo legacy",
            "cwd": canonical.to_string_lossy(),
            "timeout_secs": 120
        })
        .to_string(),
    )
    .expect("写入旧格式派单签名");
    fs::write(
        attempt.join("handback.json"),
        json!({ "task_id": "t-21", "attempt": 1, "status": "ok", "stdout_digest": "sha256:legacy" })
            .to_string(),
    )
    .expect("写入旧格式回执");

    let payload = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-21",
            "cmd": "echo legacy",
            "cwd": workdir.path().to_string_lossy()
        }),
    ));
    assert_eq!(payload["attempt"], 1);
    assert_eq!(
        payload["stdout_digest"], "sha256:legacy",
        "升级前的派单（dispatch.json 无 env）应视为 env 为空并回放"
    );
    assert!(!runs.path().join("t-21").join("attempt-2").exists());
}

// ---- runs.list：只读枚举 runs 目录，分页确定，不泄漏 env 值 ----

/// 先断言调用成功再解析正文，让红阶段停在断言上而不是解析 panic
fn runs_list_payload(server: &Server, arguments: Value) -> Value {
    let response = call_tool(server, "runs.list", arguments);
    assert_eq!(
        response["result"]["isError"], false,
        "runs.list 应可用：{}",
        tool_text(&response)
    );
    serde_json::from_str(&tool_text(&response)).expect("runs.list 必须返回 JSON 对象")
}

#[test]
fn runs_list_distinguishes_missing_from_empty_without_writing() {
    let (server, _settings, _runs) = server_with_runs();
    let empty = runs_list_payload(&server, json!({}));
    assert_eq!(empty["state"], "empty", "目录存在但无任务应明确为 empty");
    assert_eq!(empty["total"], 0);
    assert_eq!(empty["tasks"], json!([]));
    assert_eq!(empty["offset"], 0);
    assert_eq!(empty["limit"], 20);

    let settings_dir = TempDir::new().expect("创建设置目录");
    let base = TempDir::new().expect("创建临时根目录");
    let absent = base.path().join("absent-runs");
    let missing_server =
        Server::with_runs_dir(settings_dir.path().join("settings.json"), absent.clone());
    let missing = runs_list_payload(&missing_server, json!({}));
    assert_eq!(missing["state"], "missing", "目录缺失必须与空目录区分");
    assert_eq!(missing["total"], 0);
    assert!(!absent.exists(), "只读枚举不得创建 runs 目录");
}

#[test]
fn runs_list_sorts_paginates_and_excludes_session() {
    let (server, _settings, runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    // 故意乱序派单：排序由枚举侧决定，而非目录创建顺序
    for task_id in ["t-3", "t-1", "t-2"] {
        let response = call_tool(
            &server,
            "task.execute",
            json!({ "task_id": task_id, "cmd": "echo list", "cwd": workdir.path().to_string_lossy() }),
        );
        assert_eq!(response["result"]["isError"], false);
    }
    call(
        &server,
        json!({
            "jsonrpc": "2.0", "id": 20, "method": "initialize",
            "params": { "clientInfo": { "name": "pm" } }
        }),
    );
    assert!(runs.path().join("_session").exists());

    let all = runs_list_payload(&server, json!({}));
    assert_eq!(all["state"], "ready");
    assert_eq!(all["total"], 3);
    let ids: Vec<&str> = all["tasks"]
        .as_array()
        .expect("tasks 必须是数组")
        .iter()
        .filter_map(|task| task["task_id"].as_str())
        .collect();
    assert_eq!(ids, vec!["t-1", "t-2", "t-3"], "必须按 task_id 升序");
    assert!(
        ids.iter().all(|task_id| !task_id.starts_with('_')),
        "_session 等预留目录不得被当成任务列出"
    );

    let page = runs_list_payload(&server, json!({ "offset": 1, "limit": 1 }));
    assert_eq!(page["total"], 3);
    assert_eq!(page["offset"], 1);
    assert_eq!(page["limit"], 1);
    let page_ids: Vec<&str> = page["tasks"]
        .as_array()
        .expect("tasks 必须是数组")
        .iter()
        .filter_map(|task| task["task_id"].as_str())
        .collect();
    assert_eq!(page_ids, vec!["t-2"]);

    let again = runs_list_payload(&server, json!({ "offset": 1, "limit": 1 }));
    assert_eq!(again, page, "同一状态多次枚举必须完全一致");
}

#[test]
fn runs_list_never_exposes_env_values() {
    let (server, _settings, _runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let secret = "s3cr3t-list-8c1f";
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-30",
            "cmd": "echo list",
            "cwd": workdir.path().to_string_lossy(),
            "env": { "AW_LIST_SECRET": secret }
        }),
    );
    assert_eq!(response["result"]["isError"], false);

    let listed = runs_list_payload(&server, json!({}));
    let entry = listed["tasks"]
        .as_array()
        .expect("tasks 必须是数组")
        .iter()
        .find(|task| task["task_id"] == "t-30")
        .expect("必须列出已执行任务");
    assert_eq!(entry["attempts"], 1);
    assert_eq!(entry["latest_attempt"], 1);
    assert_eq!(entry["handback"], true);
    assert_eq!(entry["status"], "ok");
    assert_eq!(
        entry["env_keys"],
        json!(["AW_LIST_SECRET"]),
        "只给 env 键名，不给值"
    );
    assert!(
        !listed.to_string().contains(secret),
        "枚举结果不得泄漏 env 值"
    );
}

#[test]
fn runs_list_rejects_invalid_pagination() {
    let (server, _settings, _runs) = server_with_runs();
    let cases = vec![
        json!({ "limit": 0 }),
        json!({ "limit": 101 }),
        json!({ "offset": -1 }),
        json!({ "offset": "x" }),
        json!({ "limit": 1.5 }),
    ];
    for arguments in cases {
        let response = call_tool(&server, "runs.list", arguments);
        assert_eq!(response["result"]["isError"], true, "非法分页必须拒绝");
        let text = tool_text(&response);
        assert!(text.contains("invalid_pagination"), "必须给具名错误码：{text}");
    }
}

// ---- 证据时间戳：dispatchedAt / finishedAt（v1.6）----

/// 断言某字段是 RFC3339/UTC/秒级时间戳并返回原文；先给空值断言，红阶段停在断言而不是解析 panic
fn require_utc_stamp(value: &Value, field: &str, context: &str) -> String {
    let text = value[field].as_str().unwrap_or("");
    assert!(!text.is_empty(), "{context} 必须记录 {field}：{value}");
    assert!(
        text.ends_with('Z'),
        "{context} 的 {field} 必须是 UTC（Z 结尾）：{text}"
    );
    let parsed = match chrono::DateTime::parse_from_rfc3339(text) {
        Ok(parsed) => parsed,
        Err(error) => panic!("{context} 的 {field} 不是 RFC3339：{text}（{error}）"),
    };
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "{context} 的 {field} 必须是 UTC 偏移 0：{text}"
    );
    assert_eq!(
        text.len(),
        20,
        "{context} 的 {field} 应为秒级精度（YYYY-MM-DDTHH:MM:SSZ）：{text}"
    );
    text.to_string()
}

#[test]
fn dispatch_and_handback_record_utc_timestamps() {
    let (server, _settings, runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-30", "cmd": "echo ts" }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "派单应成功：{}",
        tool_text(&response)
    );
    let attempt = runs.path().join("t-30").join("attempt-1");
    let dispatch: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("读取 dispatch.json"),
    )
    .expect("dispatch.json 必须是 JSON");
    let handback: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("handback.json")).expect("读取 handback.json"),
    )
    .expect("handback.json 必须是 JSON");

    let dispatched = require_utc_stamp(&dispatch, "dispatchedAt", "派单记录");
    let finished = require_utc_stamp(&handback, "finishedAt", "回执");
    // 同一固定宽度 UTC 格式下，字典序即时间序
    assert!(
        dispatched <= finished,
        "结束时间不得早于派单时间：{dispatched} vs {finished}"
    );
    assert_eq!(
        handback["task_id"], "t-30",
        "新增时间戳不得改变既有字段"
    );

    let receipt = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-30" }),
    ));
    assert_eq!(
        require_utc_stamp(&receipt, "finishedAt", "receipt.get"),
        finished,
        "receipt.get 必须原样返回落盘时间戳"
    );
}

#[test]
fn identical_retry_replays_without_rewriting_dispatched_at() {
    let (server, _settings, runs) = server_with_runs();
    let first = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-31", "cmd": "echo once" }),
    ));
    let attempt = runs.path().join("t-31").join("attempt-1");
    let original: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("读取原始派单记录"),
    )
    .expect("派单记录必须是 JSON");
    let dispatched = require_utc_stamp(&original, "dispatchedAt", "派单记录");

    let second = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-31", "cmd": "echo once" }),
    ));
    assert_eq!(
        second["attempt"], 1,
        "同参重投必须回放，不得追加 attempt，也不得因 dispatchedAt 变化判冲突"
    );
    assert_eq!(
        first["stdout_digest"], second["stdout_digest"],
        "重投必须回放同一份回执"
    );
    let after: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("重读派单记录"),
    )
    .expect("派单记录必须是 JSON");
    assert_eq!(
        require_utc_stamp(&after, "dispatchedAt", "派单记录"),
        dispatched,
        "重投不得改写派单时间戳"
    );
    assert!(
        !runs.path().join("t-31").join("attempt-2").exists(),
        "重投不得产生新 attempt"
    );
}

#[test]
fn legacy_records_without_timestamps_are_replayed_and_not_backfilled() {
    let (server, _settings, runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let canonical = fs::canonicalize(workdir.path()).expect("规范化工作目录");
    let attempt = runs.path().join("t-32").join("attempt-1");
    fs::create_dir_all(&attempt).expect("创建旧格式 attempt");
    fs::write(
        attempt.join("dispatch.json"),
        json!({
            "task_id": "t-32",
            "cmd": "echo legacy-ts",
            "cwd": canonical.to_string_lossy(),
            "timeout_secs": 120,
            "env": {}
        })
        .to_string(),
    )
    .expect("写入旧格式派单记录");
    fs::write(
        attempt.join("handback.json"),
        json!({
            "task_id": "t-32",
            "attempt": 1,
            "status": "ok",
            "exit_code": 0,
            "stdout_digest": "sha256:legacy-ts"
        })
        .to_string(),
    )
    .expect("写入旧格式回执");

    let replay = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-32",
            "cmd": "echo legacy-ts",
            "cwd": workdir.path().to_string_lossy()
        }),
    ));
    assert_eq!(replay["attempt"], 1);
    assert_eq!(
        replay["stdout_digest"], "sha256:legacy-ts",
        "缺时间戳的旧派单仍应按参数回放"
    );
    assert!(
        replay.get("finishedAt").is_none(),
        "旧回执缺失时间戳时必须原样返回，不回填：{replay}"
    );

    let dispatch_after: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("重读旧派单记录"),
    )
    .expect("派单记录必须是 JSON");
    assert!(
        dispatch_after.get("dispatchedAt").is_none(),
        "旧派单缺失时间戳时不得补写：{dispatch_after}"
    );
    let handback_after: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("handback.json")).expect("重读旧回执"),
    )
    .expect("回执必须是 JSON");
    assert!(
        handback_after.get("finishedAt").is_none(),
        "旧回执不得被回填：{handback_after}"
    );
}

#[test]
fn failed_and_timeout_attempts_record_finished_at_but_preflight_does_not() {
    let (server, _settings, runs) = server_with_runs();
    let failed = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-33", "cmd": "exit 3" }),
    ));
    assert_eq!(failed["status"], "failed");
    require_utc_stamp(&failed, "finishedAt", "失败回执");

    #[cfg(windows)]
    let slow = "ping -n 4 127.0.0.1 > NUL";
    #[cfg(not(windows))]
    let slow = "sleep 3";
    let timed_out = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-34", "cmd": slow, "timeout_secs": 1 }),
    ));
    assert_eq!(timed_out["status"], "blocked");
    require_utc_stamp(&timed_out, "finishedAt", "超时回执");

    let preflight = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-35" }),
    ));
    assert_eq!(preflight["status"], "blocked");
    assert!(preflight["attempt"].is_null(), "预检 blocked 不得占号");
    assert!(
        preflight.get("finishedAt").is_none(),
        "预检 blocked 不落盘，不应带 finishedAt：{preflight}"
    );
    assert!(!runs.path().join("t-35").exists(), "预检 blocked 不得落盘");
}

// ---- env.PATH fail-closed：不许整体覆盖继承 PATH（v1.7）----

#[test]
fn env_path_is_rejected_before_claiming_attempt() {
    let (server, _settings, runs) = server_with_runs();
    for (index, key) in ["PATH", "Path", "path", "PaTh"].iter().enumerate() {
        let task_id = format!("t-path-{index}");
        let mut env = serde_json::Map::new();
        env.insert((*key).to_string(), json!("/tmp/only-this-dir"));
        let response = call_tool(
            &server,
            "task.execute",
            json!({
                "task_id": task_id,
                "cmd": "echo should-not-run",
                "env": Value::Object(env)
            }),
        );
        assert_eq!(
            response["result"]["isError"],
            true,
            "env.{key} 必须在占号之前被拒，不得整体覆盖继承 PATH：{}",
            tool_text(&response)
        );
        let text = tool_text(&response);
        assert!(
            text.contains("invalid_env"),
            "必须给具名错误码 invalid_env：{text}"
        );
        assert!(
            text.contains("PATH") && text.contains("覆盖"),
            "错误必须说明 PATH 不可整体覆盖：{text}"
        );
        assert!(
            !runs.path().join(&task_id).exists(),
            "被拒的派单不得建目录：{task_id}"
        );
    }
}

#[test]
fn legacy_dispatch_with_env_path_stays_readable_but_input_is_rejected() {
    let (server, _settings, runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let canonical = fs::canonicalize(workdir.path()).expect("规范化工作目录");
    let attempt = runs.path().join("t-path-legacy").join("attempt-1");
    fs::create_dir_all(&attempt).expect("创建旧格式 attempt");
    fs::write(
        attempt.join("dispatch.json"),
        json!({
            "task_id": "t-path-legacy",
            "cmd": "echo legacy-path",
            "cwd": canonical.to_string_lossy(),
            "timeout_secs": 120,
            "env": { "PATH": "sha256:legacy-path-override" },
            "dispatchedAt": "2026-11-06T00:00:00Z"
        })
        .to_string(),
    )
    .expect("写入旧格式派单记录");
    fs::write(
        attempt.join("handback.json"),
        json!({
            "task_id": "t-path-legacy",
            "attempt": 1,
            "status": "ok",
            "exit_code": 0,
            "stdout_digest": "sha256:legacy-path",
            "env_keys": ["PATH"],
            "finishedAt": "2026-11-06T00:00:01Z"
        })
        .to_string(),
    )
    .expect("写入旧格式回执");

    // 只读路径不受影响：旧记录原样返回，不因含 env.PATH 而被改写或抹掉
    let receipt = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-path-legacy" }),
    ));
    assert_eq!(receipt["stdout_digest"], "sha256:legacy-path");
    assert_eq!(
        receipt["env_keys"][0], "PATH",
        "旧记录的 env_keys 必须原样返回"
    );

    // 但入参里的 env.PATH 一律拒绝：旧覆盖行为属已知缺陷，不追认
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-path-legacy",
            "cmd": "echo legacy-path",
            "cwd": workdir.path().to_string_lossy(),
            "env": { "PATH": "/tmp/legacy-only" }
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "旧记录不得成为 env.PATH 的放行通道：{}",
        tool_text(&response)
    );
    assert!(
        tool_text(&response).contains("invalid_env"),
        "必须给 invalid_env 而不是参数冲突：{}",
        tool_text(&response)
    );

    let dispatch_after: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("重读旧派单记录"),
    )
    .expect("派单记录必须是 JSON");
    assert_eq!(
        dispatch_after["env"]["PATH"], "sha256:legacy-path-override",
        "旧记录不得被改写"
    );
    assert_eq!(
        dispatch_after["dispatchedAt"], "2026-11-06T00:00:00Z",
        "旧记录时间戳不得被回填或改写"
    );
}

// ---- runs.list：total == Σ attempts（v1.8）----

/// 构造：t-a 真实派单（1 个 attempt）；t-b 手工制造 1/3/5/7 且 7 的回执损坏；
/// t-c 只有 task 目录、0 个 attempt；t-reject 是占号前被拒的派单（不得留下任何目录）
fn runs_list_totals_fixture() -> (Server, TempDir, TempDir) {
    let (server, settings, runs) = server_with_runs();
    let ok = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-a", "cmd": "echo a" }),
    );
    assert_eq!(
        ok["result"]["isError"],
        false,
        "t-a 派单应成功：{}",
        tool_text(&ok)
    );

    let b1 = runs.path().join("t-b").join("attempt-1");
    fs::create_dir_all(&b1).expect("创建 t-b/attempt-1");
    fs::write(
        b1.join("handback.json"),
        json!({ "task_id": "t-b", "attempt": 1, "status": "ok", "exit_code": 0 }).to_string(),
    )
    .expect("写入 t-b/attempt-1 回执");
    for number in [3u64, 5, 7] {
        let dir = runs.path().join("t-b").join(format!("attempt-{number}"));
        fs::create_dir_all(&dir).expect("创建 t-b 的跳号 attempt");
    }
    fs::write(
        runs.path().join("t-b").join("attempt-7").join("handback.json"),
        "{ this is not json",
    )
    .expect("写入损坏回执");

    fs::create_dir_all(runs.path().join("t-c")).expect("创建只有目录的 task");

    let rejected = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-reject", "cmd": "echo nope", "env": { "PATH": "/tmp/x" } }),
    );
    assert_eq!(
        rejected["result"]["isError"],
        true,
        "占号前必须拒绝：{}",
        tool_text(&rejected)
    );
    assert!(
        !runs.path().join("t-reject").exists(),
        "占号前拒绝不得留下任何目录"
    );

    (server, settings, runs)
}

#[test]
fn runs_list_total_equals_sum_of_attempts_and_ignores_pagination() {
    let (server, _settings, _runs) = runs_list_totals_fixture();
    let mut totals = Vec::new();
    let mut summed = 0u64;
    for offset in 0..4 {
        let page = runs_list_payload(&server, json!({ "offset": offset, "limit": 1 }));
        totals.push(page["total"].as_u64().unwrap_or(u64::MAX));
        if let Some(tasks) = page["tasks"].as_array() {
            for task in tasks {
                summed += task["attempts"].as_u64().unwrap_or(u64::MAX);
            }
        }
    }
    assert_eq!(
        totals,
        vec![5u64, 5, 5, 5],
        "分页不得改变 total：{totals:?}"
    );
    assert_eq!(summed, 5, "Σ 条目.attempts 必须等于 total");

    let whole = runs_list_payload(&server, json!({}));
    assert_eq!(
        whole["total"], 5,
        "total 是 attempt 目录总数，不是任务数（任务只有 3 个）"
    );
    assert_eq!(whole["total_tasks"], 3, "total_tasks 才是任务数");
    assert_eq!(
        whole["tasks"].as_array().map(|tasks| tasks.len()),
        Some(3)
    );

    let mut attempts = Vec::new();
    for task in whole["tasks"].as_array().expect("tasks 必须是数组") {
        attempts.push((
            task["task_id"].as_str().unwrap_or("").to_string(),
            task["attempts"].as_u64().unwrap_or(u64::MAX),
        ));
    }
    assert_eq!(
        attempts,
        vec![
            ("t-a".to_string(), 1u64),
            ("t-b".to_string(), 4u64),
            ("t-c".to_string(), 0u64),
        ],
        "逐条 attempts 必须是真实 attempt 目录数（跳号按目录个数计）"
    );
}

#[test]
fn runs_list_counts_attempt_dirs_regardless_of_handback_and_gaps() {
    let (server, _settings, _runs) = runs_list_totals_fixture();
    let payload = runs_list_payload(&server, json!({}));
    let tasks = payload["tasks"].as_array().expect("tasks 必须是数组");
    let by_id = |id: &str| -> Value {
        tasks
            .iter()
            .find(|task| task["task_id"] == id)
            .unwrap_or_else(|| panic!("缺少任务 {id}：{payload}"))
            .clone()
    };

    let ok_task = by_id("t-a");
    assert_eq!(ok_task["attempts"], 1);
    assert_eq!(ok_task["latest_attempt"], 1);
    assert_eq!(ok_task["handback"], true, "真实回执必须可解析");

    let gap_task = by_id("t-b");
    assert_eq!(
        gap_task["attempts"], 4,
        "attempts 按目录个数计：1/3/5/7 存在、2/4/6 不存在 → 4"
    );
    assert_eq!(
        gap_task["latest_attempt"], 7,
        "latest_attempt 是编号最大值，不等于 attempts"
    );
    assert_eq!(
        gap_task["handback"], false,
        "最新 attempt 的回执损坏，必须判为不可解析"
    );
    assert!(gap_task["status"].is_null(), "回执损坏时 status 必须为 null");
    assert!(gap_task["exit_code"].is_null());

    let empty_task = by_id("t-c");
    assert_eq!(empty_task["attempts"], 0, "无 attempt 目录即为 0");
    assert!(empty_task["latest_attempt"].is_null());
    assert_eq!(empty_task["handback"], false);

    assert!(
        !tasks.iter().any(|task| task["task_id"] == "t-reject"),
        "占号前拒绝的 task 不得出现在列表里"
    );
}

// ---- path_prepend：显式前插继承 PATH（v1.9）----

#[test]
fn path_prepend_is_placed_before_inherited_path() {
    let (server, _settings, runs) = server_with_runs();
    let extra = TempDir::new().expect("创建前插目录");
    let prepend_dir = fs::canonicalize(extra.path()).expect("规范化前插目录");
    let expected = prepend_dir.to_string_lossy().to_string();
    let echo_path = if cfg!(windows) { "echo %PATH%" } else { "echo $PATH" };
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-pp",
            "cmd": echo_path,
            "path_prepend": vec![prepend_dir.to_string_lossy().to_string()]
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "path_prepend 应被接受：{}",
        tool_text(&response)
    );

    let stdout = fs::read_to_string(
        runs.path()
            .join("t-pp")
            .join("attempt-1")
            .join("stdout.txt"),
    )
    .expect("读取 stdout");
    let separator = if cfg!(windows) { ';' } else { ':' };
    let parts: Vec<&str> = stdout.trim().split(separator).collect();
    assert!(
        parts.len() >= 2,
        "PATH 必须同时含前插目录与继承部分：{stdout}"
    );
    assert_eq!(
        parts[0],
        expected.as_str(),
        "prepend 必须排在最前：{stdout}"
    );
    assert!(
        parts[1..].iter().any(|part| !part.is_empty()),
        "继承的 PATH 不得被丢掉：{stdout}"
    );

    let handback = tool_payload(&response);
    assert_eq!(
        handback["path_prepend"].as_array().map(|items| items.len()),
        Some(1),
        "回执必须回目录名列表：{handback}"
    );
    assert_eq!(
        handback["path_prepend"][0].as_str().unwrap_or(""),
        expected.as_str()
    );
}

#[test]
fn path_prepend_participates_in_idempotency_signature() {
    let (server, _settings, runs) = server_with_runs();
    let first = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-idem", "cmd": "echo idem", "path_prepend": ["/opt/one"] }),
    ));
    assert_eq!(first["attempt"], 1);
    let same = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-idem", "cmd": "echo idem", "path_prepend": ["/opt/one"] }),
    ));
    assert_eq!(same["attempt"], 1, "同参重投必须回放，不得判冲突");

    let changed = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-idem", "cmd": "echo idem", "path_prepend": ["/opt/two"] }),
    );
    assert_eq!(
        changed["result"]["isError"],
        true,
        "改 path_prepend 必须判冲突：{}",
        tool_text(&changed)
    );
    assert!(
        tool_text(&changed).contains("task_id_conflict"),
        "必须是具名的 task_id_conflict：{}",
        tool_text(&changed)
    );

    let reordered = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-idem", "cmd": "echo idem", "path_prepend": ["/opt/one", "/opt/two"] }),
    );
    assert_eq!(
        reordered["result"]["isError"],
        true,
        "元素个数变化也是异参：{}",
        tool_text(&reordered)
    );
    assert!(
        !runs.path().join("t-pp-idem").join("attempt-2").exists(),
        "冲突不得产生新 attempt"
    );
}

#[test]
fn invalid_path_prepend_is_rejected_before_claiming_attempt() {
    let (server, _settings, runs) = server_with_runs();
    let long_element = "a".repeat(4097);
    let mut many: Vec<String> = Vec::new();
    for index in 0..17 {
        many.push(format!("/opt/{index}"));
    }
    let cases = vec![
        ("t-pp-bad-1", json!([""])),
        ("t-pp-bad-2", json!(["/opt/one\0two"])),
        ("t-pp-bad-3", json!("a;b")),
        ("t-pp-bad-4", json!([1])),
        ("t-pp-bad-5", json!(many)),
        ("t-pp-bad-6", json!([long_element])),
    ];
    for (task_id, path_prepend) in cases {
        let response = call_tool(
            &server,
            "task.execute",
            json!({ "task_id": task_id, "cmd": "echo nope", "path_prepend": path_prepend }),
        );
        assert_eq!(
            response["result"]["isError"],
            true,
            "非法 path_prepend 必须在占号前被拒（{task_id}）：{}",
            tool_text(&response)
        );
        assert!(
            tool_text(&response).contains("invalid_path_prepend"),
            "必须给具名错误码 invalid_path_prepend：{}",
            tool_text(&response)
        );
        assert!(
            !runs.path().join(task_id).exists(),
            "被拒的派单不得建目录：{task_id}"
        );
    }
}

#[test]
fn legacy_dispatch_without_path_prepend_still_replays() {
    let (server, _settings, runs) = server_with_runs();
    let workdir = TempDir::new().expect("创建工作目录");
    let canonical = fs::canonicalize(workdir.path()).expect("规范化工作目录");
    let attempt = runs.path().join("t-pp-legacy").join("attempt-1");
    fs::create_dir_all(&attempt).expect("创建旧格式 attempt");
    fs::write(
        attempt.join("dispatch.json"),
        json!({
            "task_id": "t-pp-legacy",
            "cmd": "echo legacy-pp",
            "cwd": canonical.to_string_lossy(),
            "timeout_secs": 120,
            "env": {}
        })
        .to_string(),
    )
    .expect("写入旧格式派单记录");
    fs::write(
        attempt.join("handback.json"),
        json!({
            "task_id": "t-pp-legacy",
            "attempt": 1,
            "status": "ok",
            "exit_code": 0,
            "stdout_digest": "sha256:legacy-pp"
        })
        .to_string(),
    )
    .expect("写入旧格式回执");

    let replay = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-pp-legacy",
            "cmd": "echo legacy-pp",
            "cwd": workdir.path().to_string_lossy()
        }),
    ));
    assert_eq!(replay["attempt"], 1, "缺 path_prepend 的旧派单仍应回放");
    assert_eq!(replay["stdout_digest"], "sha256:legacy-pp");
    assert!(
        replay.get("path_prepend").is_none(),
        "旧回执缺失该字段时必须原样返回，不回填：{replay}"
    );
    let after: Value = serde_json::from_str(
        &fs::read_to_string(attempt.join("dispatch.json")).expect("重读旧派单记录"),
    )
    .expect("派单记录必须是 JSON");
    assert!(
        after.get("path_prepend").is_none(),
        "旧派单不得被补写 path_prepend：{after}"
    );
}

// ---- 空集合口径一致：path_prepend: [] ≡ 缺省（v1.10，选 A）----

#[test]
fn empty_path_prepend_is_equivalent_to_omitted() {
    let (server, _settings, runs) = server_with_runs();
    let first = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-empty", "cmd": "echo empty", "path_prepend": [] }),
    ));
    assert_eq!(first["attempt"], 1, "空数组必须被接受，不得判非法");

    let replay = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-pp-empty", "cmd": "echo empty" }),
    ));
    assert_eq!(
        replay["attempt"], 1,
        "[] 与缺省同签名：同 id 重投必须命中同一 attempt，不得判冲突"
    );
    assert_eq!(first["stdout_digest"], replay["stdout_digest"]);
    assert!(
        !runs.path()
            .join("t-pp-empty")
            .join("attempt-2")
            .exists(),
        "等价空集合不得产生新 attempt"
    );

    let dispatch: Value = serde_json::from_str(
        &fs::read_to_string(
            runs.path()
                .join("t-pp-empty")
                .join("attempt-1")
                .join("dispatch.json"),
        )
        .expect("读取派单记录"),
    )
    .expect("派单记录必须是 JSON");
    assert_eq!(
        dispatch["path_prepend"],
        json!([]),
        "签名里空集合必须归一为空数组"
    );
    assert_eq!(
        first["path_prepend"],
        json!([]),
        "回执必须给空列表，而不是省略字段"
    );
}

#[test]
fn empty_env_and_path_prepend_combinations_share_one_signature() {
    let (server, _settings, runs) = server_with_runs();
    let dispatch_path = runs
        .path()
        .join("t-pp-matrix")
        .join("attempt-1")
        .join("dispatch.json");
    let combos = [
        json!({ "task_id": "t-pp-matrix", "cmd": "echo matrix" }),
        json!({ "task_id": "t-pp-matrix", "cmd": "echo matrix", "env": {}, "path_prepend": [] }),
        json!({ "task_id": "t-pp-matrix", "cmd": "echo matrix", "env": {} }),
        json!({ "task_id": "t-pp-matrix", "cmd": "echo matrix", "path_prepend": [] }),
    ];
    let mut baseline = String::new();
    for (index, arguments) in combos.into_iter().enumerate() {
        let payload = tool_payload(&call_tool(&server, "task.execute", arguments));
        assert_eq!(
            payload["attempt"], 1,
            "第 {} 种空/缺省组合必须命中同一 attempt",
            index + 1
        );
        let text = fs::read_to_string(&dispatch_path).expect("读取派单记录");
        if index == 0 {
            baseline = text;
        } else {
            assert_eq!(
                text, baseline,
                "第 {} 种组合改写了签名，说明空集合没有归一",
                index + 1
            );
        }
        assert!(
            !runs
                .path()
                .join("t-pp-matrix")
                .join("attempt-2")
                .exists(),
            "第 {} 种组合产生了新 attempt",
            index + 1
        );
    }
    let dispatch: Value = serde_json::from_str(&baseline).expect("派单记录必须是 JSON");
    assert_eq!(dispatch["env"], json!({}), "缺省 env 必须归一到空 map");
    assert_eq!(
        dispatch["path_prepend"],
        json!([]),
        "缺省 path_prepend 必须归一到空数组"
    );
}

// ---- 超时文案诚实性：只描述已执行动作，不宣称整棵树已终止（v1.11；v1.16 窄定义）----

#[test]
fn timeout_reason_states_action_not_tree_death() {
    let (server, _settings, _runs) = server_with_runs();
    #[cfg(windows)]
    let slow = "ping -n 4 127.0.0.1 > NUL";
    #[cfg(not(windows))]
    let slow = "sleep 3";
    let payload = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "t-to-honest", "cmd": slow, "timeout_secs": 1 }),
    ));
    assert_eq!(payload["status"], "blocked");
    let reason = payload["blocked_reason"].as_str().unwrap_or("");
    println!("[evidence] timeout_reason={reason}");
    assert!(
        reason.starts_with("timeout"),
        "必须保留 timeout 前缀码：{reason}"
    );
    assert!(
        reason.contains("已清理受控集合"),
        "状态门已满足（v1.17）后应只宣称受控集合：{reason}"
    );
    assert!(
        reason.contains("逃逸进程是否仍在运行未验证"),
        "必须明写逃逸进程未验证：{reason}"
    );
    assert!(
        !reason.contains("已终止进程树") && !reason.contains("无残留"),
        "永远不得宣称整棵树已终止：{reason}"
    );
    #[cfg(windows)]
    assert!(
        reason.contains("spawn→Assign") && reason.contains("窗口"),
        "Windows 必须写明 spawn→Assign 竞态窗口：{reason}"
    );
}

// 描述必须与 v1.17 三态实现同口径：不承诺整棵树，写明受控集合与 Windows 窗口
#[test]
fn timeout_description_matches_controlled_set_semantics() {
    let (server, _dir) = server_with_settings(None);
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    let execute = tools
        .iter()
        .find(|tool| tool["name"] == "task.execute")
        .expect("task.execute 必须存在");
    let tool_desc = execute["description"].as_str().unwrap_or("");
    let field_desc = execute["inputSchema"]["properties"]["timeout_secs"]["description"]
        .as_str()
        .unwrap_or("");
    println!("[evidence] task.execute.description={tool_desc}");
    println!("[evidence] timeout_secs.description={field_desc}");

    for (label, text) in [("task.execute", tool_desc), ("timeout_secs", field_desc)] {
        for token in ["受控集合", "未验证", "不承诺终止整棵进程树"] {
            assert!(
                text.contains(token),
                "{label} 描述缺少 `{token}`：{text}"
            );
        }
        assert!(
            !text.contains("已终止进程树"),
            "{label} 描述不得宣称整棵树：{text}"
        );
    }
    for token in [
        "POSIX 进程组",
        "Windows Job Object",
        "已清理受控集合",
        "spawn→Assign",
        "窗口",
    ] {
        assert!(
            tool_desc.contains(token),
            "task.execute 描述缺少 `{token}`：{tool_desc}"
        );
    }
    for token in [
        "三态",
        "已清理受控集合",
        "仍有存活进程",
        "无法查询",
        "仅向直接子进程发送终止信号",
    ] {
        assert!(
            field_desc.contains(token),
            "timeout_secs 描述缺少 `{token}`：{field_desc}"
        );
    }
}

// 正向：受控集合内的孙进程必须被清理，并用 OS 观测点复核（不是 stat/写盘代理）
#[test]
fn timeout_cleans_controlled_set_including_grandchild() {
    let (server, _settings, _runs) = server_with_runs();
    let work = tempfile::tempdir().expect("临时工作目录");
    #[cfg(windows)]
    let cmd = "powershell -NoProfile -Command $p=Start-Process ping -ArgumentList '-n','30','127.0.0.1' -PassThru -WindowStyle Hidden; Set-Content -Path pids.txt -Value $p.Id; Wait-Process -Id $p.Id";
    #[cfg(not(windows))]
    let cmd = "echo $$ > pids.txt; sleep 30 & echo $! >> pids.txt; wait";
    #[cfg(windows)]
    let timeout_secs = 8;
    #[cfg(not(windows))]
    let timeout_secs = 1;

    let payload = tool_payload(&call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-to-grandchild",
            "cmd": cmd,
            "cwd": work.path(),
            "timeout_secs": timeout_secs
        }),
    ));
    assert_eq!(payload["status"], "blocked");
    let reason = payload["blocked_reason"].as_str().unwrap_or("");
    println!("[evidence] grandchild_reason={reason}");
    assert!(
        reason.contains("已清理受控集合"),
        "受控集合内清理应成功：{reason}"
    );

    let pid_text = fs::read_to_string(work.path().join("pids.txt")).expect("命令必须写出 PID");
    #[cfg(windows)]
    {
        let pid: u32 = pid_text
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .parse()
            .expect("PowerShell 子进程 PID");
        println!(
            "[evidence] grandchild pid={pid} process_exists_after={}",
            process_exists(pid)
        );
        assert!(
            !process_exists(pid),
            "作业内的孙进程必须已被清理，PID {pid} 仍存活"
        );
    }
    #[cfg(not(windows))]
    {
        let pids: Vec<i32> = pid_text
            .lines()
            .filter_map(|line| line.trim().parse::<i32>().ok())
            .collect();
        assert!(pids.len() >= 2, "必须同时记录 shell 与 sleep 的 PID");
        let pgid = pids[0];
        assert!(!process_exists(pids[0]), "直接子进程应已被清理");
        assert!(!process_exists(pids[1]), "孙进程应已被清理");
        assert!(group_is_empty(pgid), "受控集合（进程组）判据必须为 ESRCH");
    }
}

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    let rc = unsafe { libc::kill(pid, 0) };
    rc == 0
        || !matches!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        )
}

#[cfg(unix)]
fn group_is_empty(pgid: i32) -> bool {
    let rc = unsafe { libc::kill(-pgid, 0) };
    rc != 0
        && matches!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        )
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut code = 0u32;
    let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
    unsafe {
        CloseHandle(handle);
    }
    // STILL_ACTIVE = 259
    ok != 0 && code == 259
}

#[test]
fn legacy_timeout_reason_is_returned_verbatim_and_not_rewritten() {
    let (server, _settings, runs) = server_with_runs();
    let attempt = runs
        .path()
        .join("t-to-legacy")
        .join("attempt-1");
    fs::create_dir_all(&attempt).expect("创建旧格式 attempt");
    let legacy_reason = "timeout：超过 120s 未结束，已终止进程";
    let handback = json!({
        "task_id": "t-to-legacy",
        "attempt": 1,
        "status": "blocked",
        "exit_code": null,
        "blocked_reason": legacy_reason,
        "stdout_digest": "sha256:legacy-timeout"
    })
    .to_string();
    fs::write(attempt.join("handback.json"), &handback).expect("写入旧格式回执");

    let receipt = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-to-legacy" }),
    ));
    assert_eq!(
        receipt["blocked_reason"].as_str().unwrap_or(""),
        legacy_reason,
        "旧文案必须原样返回，不推断、不改写"
    );
    let after = fs::read_to_string(attempt.join("handback.json")).expect("重读旧回执");
    assert_eq!(after, handback, "旧回执文件不得被改写");
}

// ---- runs.prune：默认不删、两段式、只删非最新 attempt（v1.12）----

fn days_ago_stamp(days: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::days(days))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn days_ahead_stamp(days: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::days(days))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 构造一个 attempt 目录：`write_handback=false` 表示未完成（只有 dispatch 与日志）；
/// `finished_at=None` 且 `write_handback=true` 表示回执存在但缺字段（v1.6 之前的旧记录）
fn prune_attempt(
    runs: &Path,
    task_id: &str,
    attempt: u64,
    write_handback: bool,
    finished_at: Option<&str>,
    with_dispatch: bool,
) -> PathBuf {
    let dir = runs.join(task_id).join(format!("attempt-{attempt}"));
    fs::create_dir_all(&dir).expect("创建 attempt 目录");
    if with_dispatch {
        fs::write(
            dir.join("dispatch.json"),
            json!({
                "task_id": task_id,
                "cmd": "echo prune-fixture",
                "cwd": ".",
                "timeout_secs": 120,
                "env": { "SECRET_TOKEN": "sha256:prune-fixture" },
                "path_prepend": [],
                "dispatchedAt": "2026-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("写入 dispatch.json");
    }
    if write_handback {
        let mut record = json!({
            "task_id": task_id,
            "attempt": attempt,
            "status": "ok",
            "exit_code": 0
        });
        if let Some(stamp) = finished_at {
            record["finishedAt"] = json!(stamp);
        }
        fs::write(dir.join("handback.json"), record.to_string()).expect("写入 handback.json");
    }
    fs::write(dir.join("stdout.txt"), "stdout-fixture").expect("写入 stdout");
    fs::write(dir.join("stderr.txt"), "stderr-fixture").expect("写入 stderr");
    dir
}

#[test]
fn prune_defaults_to_dry_run_and_lists_candidates_without_writing() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-old", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-old", 2, true, Some(&old), true);
    fs::create_dir_all(runs.path().join("_session")).expect("创建审计目录");
    let entries_before = fs::read_dir(runs.path()).expect("读 runs").count();

    let response = call_tool(&server, "runs.prune", json!({ "older_than_days": 30 }));
    assert_eq!(
        response["result"]["isError"],
        false,
        "dry-run 应成功：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["dry_run"], true, "未传 dry_run 时必须默认 dry-run");
    assert_eq!(payload["state"], "ready");
    assert_eq!(payload["older_than_days"], 30);
    assert_eq!(payload["candidates_total"], 1);
    assert_eq!(payload["truncated"], false);

    let candidates = payload["candidates"]
        .as_array()
        .expect("candidates 必须是数组");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["task_id"], "t-old");
    assert_eq!(candidates[0]["attempt"], 1, "最新 attempt 永不进入候选");
    assert_eq!(candidates[0]["path"], "runs/t-old/attempt-1");
    assert_eq!(candidates[0]["finishedAt"], old);
    assert_eq!(candidates[0]["dispatchedAt"], "2026-01-01T00:00:00Z");
    assert!(
        candidates[0]["age_days"].as_i64().unwrap_or(0) >= 30,
        "年龄必须按 finishedAt 计算：{payload}"
    );

    let skipped = payload["skipped"].as_array().expect("skipped 必须是数组");
    assert!(
        skipped.iter().any(|item| item["task_id"] == "t-old"
            && item["attempt"] == 2
            && item["reason"] == "latest_attempt_retained"),
        "必须写明最新 attempt 被保留：{payload}"
    );

    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "dry-run 不得写审计"
    );
    assert!(
        runs.path().join("t-old").join("attempt-1").is_dir(),
        "dry-run 不得删目录"
    );
    assert!(runs.path().join("t-old").join("attempt-2").is_dir());
    assert!(
        runs.path().join("_session").is_dir(),
        "不得动 _ 前缀目录"
    );
    assert_eq!(
        fs::read_dir(runs.path()).expect("读 runs").count(),
        entries_before,
        "dry-run 必须零写入"
    );
}

#[test]
fn prune_keeps_latest_attempt_and_single_attempt_tasks_and_records_audit() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-old", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-old", 2, true, Some(&old), true);
    prune_attempt(runs.path(), "t-one", 1, true, Some(&old), true);
    fs::create_dir_all(runs.path().join("_session")).expect("创建审计目录");

    let response = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "全部成功时不得报错：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["deleted_total"], 1);
    assert_eq!(payload["failed_total"], 0);
    assert_eq!(payload["deleted"][0]["task_id"], "t-old");
    assert_eq!(payload["deleted"][0]["attempt"], 1);

    assert!(
        !runs.path().join("t-old").join("attempt-1").exists(),
        "旧的非最新 attempt 应被删除"
    );
    assert!(
        runs.path().join("t-old").join("attempt-2").is_dir(),
        "最新 attempt 必须保留"
    );
    assert!(
        runs.path()
            .join("t-old")
            .join("attempt-2")
            .join("handback.json")
            .is_file(),
        "整链必须一致（回执仍在）"
    );
    assert!(
        runs.path().join("t-one").join("attempt-1").is_dir(),
        "只有一个 attempt 的 task 永远删不掉"
    );
    assert!(runs.path().join("_session").is_dir());

    let audit = fs::read_to_string(runs.path().join("prune.jsonl"))
        .expect("真实删除必须写审计");
    let lines: Vec<&str> = audit.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "每批必须 plan + result 两行：{audit}");
    let plan: Value = serde_json::from_str(lines[0]).expect("plan 行必须是 JSON");
    let result: Value = serde_json::from_str(lines[1]).expect("result 行必须是 JSON");
    assert_eq!(plan["phase"], "plan", "第一行必须是删除之前的计划");
    assert_eq!(plan["older_than_days"], 30);
    assert_eq!(plan["candidate_count"], 1);
    assert_eq!(plan["candidates"][0]["finishedAt"], old);
    assert_eq!(plan["candidates"][0]["dispatchedAt"], "2026-01-01T00:00:00Z");
    assert_eq!(result["phase"], "result");
    assert_eq!(result["batch_id"], plan["batch_id"], "两行必须同批次");
    assert_eq!(result["deleted_total"], 1);
    assert_eq!(result["failed_total"], 0);
    assert!(
        !audit.contains("SECRET_TOKEN"),
        "审计不得记录 env 键值：{audit}"
    );

    let list = runs_list_payload(&server, json!({}));
    let task = list["tasks"]
        .as_array()
        .expect("tasks 必须是数组")
        .iter()
        .find(|item| item["task_id"] == "t-old")
        .cloned()
        .expect("t-old 仍应在列");
    assert_eq!(task["attempts"], 1, "runs.list 必须反映真实目录数");
    assert_eq!(task["latest_attempt"], 2);
}

#[test]
fn prune_refuses_real_delete_without_confirm_or_threshold() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-guard", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-guard", 2, true, Some(&old), true);

    let no_confirm = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false }),
    );
    assert_eq!(no_confirm["result"]["isError"], true);
    assert!(
        tool_text(&no_confirm).contains("confirm_required"),
        "必须有 confirm_required：{}",
        tool_text(&no_confirm)
    );

    let no_threshold = call_tool(
        &server,
        "runs.prune",
        json!({ "dry_run": false, "confirm": true }),
    );
    assert_eq!(no_threshold["result"]["isError"], true);
    assert!(
        tool_text(&no_threshold).contains("invalid_retention"),
        "必须要求显式阈值：{}",
        tool_text(&no_threshold)
    );

    let bad_threshold = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 0, "dry_run": false, "confirm": true }),
    );
    assert_eq!(bad_threshold["result"]["isError"], true);
    assert!(tool_text(&bad_threshold).contains("invalid_retention"));

    // confirm 单独出现不隐含真删：仍是 dry-run
    let confirm_only = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "confirm": true }),
    );
    assert_eq!(confirm_only["result"]["isError"], false);
    assert_eq!(tool_payload(&confirm_only)["dry_run"], true);

    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "任何拒绝路径都不得写审计"
    );
    assert!(
        runs.path().join("t-guard").join("attempt-1").is_dir(),
        "拒绝路径不得删除任何目录"
    );
}

#[test]
fn prune_skips_age_unknown_unfinished_future_and_missing_dispatch() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    let young = days_ago_stamp(5);
    let future = days_ahead_stamp(5);

    // 回执存在但缺 finishedAt（v1.6 之前的记录）
    prune_attempt(runs.path(), "t-legacy", 1, true, None, true);
    prune_attempt(runs.path(), "t-legacy", 2, true, Some(&old), true);
    // 未完成：只有 dispatch，没有回执
    prune_attempt(runs.path(), "t-unfinished", 1, false, None, true);
    prune_attempt(runs.path(), "t-unfinished", 2, true, Some(&old), true);
    // 时间戳在未来（时钟异常）
    prune_attempt(runs.path(), "t-future", 1, true, Some(&future), true);
    prune_attempt(runs.path(), "t-future", 2, true, Some(&old), true);
    // 缺 dispatch：年龄可判但一致性无法证明
    prune_attempt(runs.path(), "t-nodispatch", 1, true, Some(&old), false);
    prune_attempt(runs.path(), "t-nodispatch", 2, true, Some(&old), true);
    // 未达阈值
    prune_attempt(runs.path(), "t-young", 1, true, Some(&young), true);
    prune_attempt(runs.path(), "t-young", 2, true, Some(&old), true);

    let payload = tool_payload(&call_tool(&server, "runs.prune", json!({ "older_than_days": 10 })));
    assert_eq!(payload["dry_run"], true);
    assert_eq!(
        payload["candidates_total"], 0,
        "未知年龄 / 未完成 / 异常时间一律不删：{payload}"
    );
    assert_eq!(payload["candidates"].as_array().map(Vec::len), Some(0));

    let skipped = payload["skipped"].as_array().expect("skipped 必须是数组");
    let reason_of = |task: &str, attempt: u64| -> String {
        skipped
            .iter()
            .find(|item| item["task_id"] == task && item["attempt"] == attempt)
            .and_then(|item| item["reason"].as_str())
            .unwrap_or_else(|| panic!("缺少跳过记录 {task}/attempt-{attempt}：{payload}"))
            .to_string()
    };
    assert_eq!(reason_of("t-legacy", 1), "age_unknown");
    assert_eq!(reason_of("t-unfinished", 1), "unfinished_or_missing_handback");
    assert_eq!(reason_of("t-future", 1), "future_timestamp");
    assert_eq!(reason_of("t-nodispatch", 1), "missing_dispatch");
    assert_eq!(reason_of("t-young", 1), "not_old_enough");
}

#[test]
fn prune_limit_is_deterministic_and_resumable() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    for task in ["t-a", "t-b", "t-c"] {
        prune_attempt(runs.path(), task, 1, true, Some(&old), true);
        prune_attempt(runs.path(), task, 2, true, Some(&old), true);
    }

    let dry = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "limit": 1 }),
    ));
    assert_eq!(dry["candidates_total"], 3);
    assert_eq!(dry["truncated"], true);
    assert_eq!(
        dry["candidates"][0]["task_id"], "t-a",
        "必须按 (task_id, attempt) 升序"
    );

    let first = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "limit": 1, "dry_run": false, "confirm": true }),
    ));
    assert_eq!(first["deleted_total"], 1);
    assert!(!runs.path().join("t-a").join("attempt-1").exists());
    assert!(
        runs.path().join("t-b").join("attempt-1").is_dir(),
        "单批上限外不得删"
    );

    let second = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "limit": 1, "dry_run": false, "confirm": true }),
    ));
    assert_eq!(second["deleted_total"], 1);
    assert!(!runs.path().join("t-b").join("attempt-1").exists());
    assert!(
        runs.path().join("t-c").join("attempt-1").is_dir(),
        "下一批留给下一次调用"
    );
    assert!(
        runs.path().join("t-c").join("attempt-2").is_dir(),
        "最新 attempt 始终保留"
    );
}

#[test]
fn prune_does_not_create_runs_dir_when_missing() {
    let settings = TempDir::new().expect("创建设置目录");
    let parent = TempDir::new().expect("创建父目录");
    let missing = parent.path().join("runs");
    let server = Server::with_runs_dir(settings.path().join("settings.json"), missing.clone());

    let dry = call_tool(&server, "runs.prune", json!({ "older_than_days": 30 }));
    assert_eq!(dry["result"]["isError"], false);
    let payload = tool_payload(&dry);
    assert_eq!(payload["state"], "missing");
    assert_eq!(payload["candidates_total"], 0);
    assert!(!missing.exists(), "dry-run 不得创建 runs 目录");

    let real = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    );
    assert_eq!(real["result"]["isError"], false);
    assert_eq!(tool_payload(&real)["deleted_total"], 0);
    assert!(
        !missing.exists(),
        "无候选时不得创建任何目录（含 .prune-trash）"
    );
}

#[cfg(windows)]
#[test]
fn prune_partial_failure_is_reported_as_partial_without_retry() {
    use std::os::windows::fs::OpenOptionsExt;

    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-partial", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-partial", 2, true, Some(&old), true);
    prune_attempt(runs.path(), "t-partial", 3, true, Some(&old), true);

    // 以"不带 FILE_SHARE_DELETE"的方式占住 attempt-1 里的文件：
    // 目录重命名可能成功，但对文件的删除会被拒绝 → 制造真实的"删一半"失败
    let attempt_one = runs.path().join("t-partial").join("attempt-1");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(attempt_one.join("stdout.txt"))
        .expect("以独占共享模式打开文件");

    let response = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "删一半不得报成功：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "partial");
    assert_eq!(payload["deleted_total"], 1);
    assert_eq!(payload["failed_total"], 1);
    assert_eq!(payload["failed"][0]["attempt"], 1);
    assert!(
        !payload["failed"][0]["error"].as_str().unwrap_or("").is_empty(),
        "失败必须带原因：{payload}"
    );
    assert!(
        !runs.path().join("t-partial").join("attempt-2").exists(),
        "成功项应已删除"
    );

    // 失败项不得丢数据、也不得只留下半条链：要么原样在原位，要么整链在 .prune-trash
    let dispatch_at_source = attempt_one.join("dispatch.json").is_file();
    let handback_at_source = attempt_one.join("handback.json").is_file();
    assert_eq!(
        dispatch_at_source, handback_at_source,
        "失败项不得留下孤立的半个 attempt"
    );
    let staged_ok = fs::read_dir(runs.path().join(".prune-trash"))
        .map(|entries| {
            entries.flatten().any(|batch| {
                fs::read_dir(batch.path()).map_or(false, |staged| {
                    staged.flatten().any(|item| {
                        item.path().join("dispatch.json").is_file()
                            && item.path().join("handback.json").is_file()
                    })
                })
            })
        })
        .unwrap_or(false);
    assert!(
        (dispatch_at_source && handback_at_source) || staged_ok,
        "失败项必须整链保留（原位或 .prune-trash）"
    );

    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(
        attempt_one.join("dispatch.json").is_file(),
        dispatch_at_source,
        "不得自动重试"
    );

    drop(locked);
}
