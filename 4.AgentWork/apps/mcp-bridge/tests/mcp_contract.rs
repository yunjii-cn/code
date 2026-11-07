// aw-mcp-bridge 契约测试：以 MCP 客户端的视角驱动最小服务器
use aw_mcp_bridge::Server;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant, SystemTime};
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
            "capabilities",
            "task.execute",
            "task.cancel",
            "receipt.get",
            "runs.list",
            "runs.inflight",
            "runs.prune"
        ],
        "执行层至少需要接单、显式取消、回执读取、只读枚举与显式清理五个入口；能力声明与活跃标记查询必须机器可读"
    );
    for tool in tools {
        assert!(
            tool["description"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
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
        "task.cancel",
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
    // v1.26：cmd 不再是唯一执行入口；dispatched 用 cmd，standalone 用 mode+argv
    assert_eq!(execute["required"], json!(["task_id"]));
    assert_eq!(
        execute["anyOf"],
        json!([
            { "required": ["cmd"] },
            { "required": ["mode", "argv"] }
        ]),
        "task.execute 必须声明 cmd | mode+argv 二选一"
    );
    assert_eq!(execute["properties"]["task_id"]["type"], "string");
    assert_eq!(execute["properties"]["cmd"]["type"], "string");
    assert_eq!(
        execute["properties"]["mode"]["enum"],
        json!(["dispatched", "standalone"]),
        "模式必须显式且仅两值"
    );
    assert_eq!(execute["properties"]["argv"]["type"], "array");
    assert_eq!(execute["properties"]["argv"]["minItems"], 1);
    assert_eq!(execute["properties"]["argv"]["items"]["type"], "string");
    assert_eq!(execute["properties"]["acceptance"]["type"], "array");
    assert_eq!(execute["properties"]["acceptance"]["minItems"], 1);
    assert_eq!(
        execute["properties"]["acceptance"]["items"]["properties"]["type"]["enum"],
        json!([
            "exit_code",
            "file_exists",
            "file_content_match",
            "git_diff_non_empty"
        ]),
        "验收标准只允许四类可核对类型"
    );
    assert_eq!(execute["properties"]["max_attempts"]["type"], "integer");
    assert_eq!(execute["properties"]["max_attempts"]["minimum"], 1);
    assert_eq!(execute["properties"]["max_attempts"]["maximum"], 5);
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
    // v1.25：未知参数变为可观测诊断，但仍不得变成拒绝——payload 里不允许出现 error 字段
    assert!(
        payload.get("error").is_none(),
        "默认策略是忽略未知参数，不得新增拒绝行为：{payload}"
    );
    let diagnostics = payload["arguments_diagnostics"]
        .as_object()
        .expect("未知参数必须可观测（v1.25）：{payload}");
    assert!(
        diagnostics["unknown_arguments"].is_array(),
        "未知参数必须落在结构化字段里：{payload}"
    );
}

fn diagnostic_entry<'a>(payload: &'a Value, list: &str, name: &str) -> &'a Value {
    payload["arguments_diagnostics"][list]
        .as_array()
        .unwrap_or_else(|| panic!("{list} 必须是数组：{payload}"))
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("{list} 缺少条目 {name}：{payload}"))
}

#[test]
fn arguments_diagnostics_flags_unknown_names_without_rejecting() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.list",
        json!({ "offest": 0, "_meta": { "source": "unknown-args-test" } }),
    );
    // 诊断只增加可观测信息，不改变成功/失败
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert!(payload["state"].is_string(), "调用必须照常成功：{payload}");
    assert!(
        payload.get("error").is_none(),
        "不得因为诊断而报错：{payload}"
    );

    let diagnostics = &payload["arguments_diagnostics"];
    assert_eq!(diagnostics["tool"], "runs.list");
    let declared: Vec<&str> = diagnostics["declared_arguments"]
        .as_array()
        .expect("declared_arguments 必须是数组")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        declared.contains(&"offset") && declared.contains(&"limit"),
        "必须回显已声明参数名，调用方无需读源码即可对照：{diagnostics}"
    );

    // 完全未知的参数名 → kind=unknown（拼写错误一类），并给出值类型/长度而非值
    let typo = diagnostic_entry(&payload, "unknown_arguments", "offest");
    assert_eq!(typo["kind"], "unknown");
    assert!(
        typo.get("alias_of").is_none(),
        "不是别名就不得写 alias_of：{typo}"
    );
    assert_eq!(typo["value"]["type"], "number");
    assert_eq!(typo["value"]["length"], Value::Null);

    // 客户端多塞的元数据 → kind=client_metadata，与拼写错误区分
    let meta = diagnostic_entry(&payload, "unknown_arguments", "_meta");
    assert_eq!(meta["kind"], "client_metadata");
    assert!(meta.get("alias_of").is_none());
    assert_eq!(meta["value"]["type"], "object");
    assert_eq!(meta["value"]["length"], 1);
}

#[test]
fn arguments_diagnostics_marks_camel_case_alias_not_typo() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365, "dryRun": false, "confirm": true }),
    );
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    // 别名仍不生效（服务端只读已知键）：dry_run 缺省 true，因此本轮仍是 dry-run
    assert_eq!(payload["dry_run"], true, "别名不得被静默应用：{payload}");

    let entry = diagnostic_entry(&payload, "unknown_arguments", "dryRun");
    assert_eq!(
        entry["kind"], "alias",
        "camelCase 别名不得被当成拼写错误：{entry}"
    );
    assert_eq!(entry["alias_of"], "dry_run");
    assert_eq!(entry["value"]["type"], "boolean");
}

#[test]
fn arguments_diagnostics_records_null_optionals_and_stays_successful() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "t-null-optional",
            "cmd": "echo ok",
            "cwd": null,
            "timeout_secs": null
        }),
    );
    // SDK 对可选字段传 null 必须仍然成功，且诊断不得把它误报成错误
    assert_eq!(response["result"]["isError"], false);
    let payload = tool_payload(&response);
    assert_eq!(
        payload["status"], "ok",
        "null 可选字段不得让调用失败：{payload}"
    );
    assert!(
        payload.get("error").is_none(),
        "null 可选字段不是错误：{payload}"
    );

    let nulls = payload["arguments_diagnostics"]["null_optional_arguments"]
        .as_array()
        .expect("null_optional_arguments 必须是数组");
    assert_eq!(nulls.len(), 2, "两个可选字段都显式传了 null：{payload}");
    for (name, declared_type) in [("cwd", "string"), ("timeout_secs", "integer")] {
        let entry = diagnostic_entry(&payload, "null_optional_arguments", name);
        assert_eq!(entry["kind"], "null_optional");
        assert_eq!(entry["declared_type"], declared_type);
        assert_eq!(entry["value"]["type"], "null");
    }
}

#[test]
fn arguments_diagnostics_is_absent_for_clean_calls() {
    let (server, _settings, _runs) = server_with_runs();
    for (name, arguments) in [("runs.list", json!({})), ("health", json!({}))] {
        let response = call_tool(&server, name, arguments);
        assert_eq!(response["result"]["isError"], false, "{name} 必须成功");
        let payload = tool_payload(&response);
        assert!(
            payload.get("arguments_diagnostics").is_none(),
            "{name} 无多余参数时不得产生诊断噪声：{payload}"
        );
    }
}

#[test]
fn arguments_diagnostics_never_leaks_values() {
    let secret = "super-secret-token-value-123456";
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.list",
        json!({ "apiToken": secret, "_meta": { "token": secret } }),
    );
    assert_eq!(response["result"]["isError"], false);
    let text = tool_text(&response);
    assert!(!text.contains(secret), "诊断不得回显参数值：{text}");

    let payload = tool_payload(&response);
    let entry = diagnostic_entry(&payload, "unknown_arguments", "apiToken");
    assert_eq!(entry["value"]["type"], "string");
    assert_eq!(
        entry["value"]["length"],
        secret.chars().count() as u64,
        "只允许值长度这类元信息：{entry}"
    );
    let meta = diagnostic_entry(&payload, "unknown_arguments", "_meta");
    assert_eq!(meta["value"]["type"], "object");
    assert_eq!(meta["value"]["length"], 1);
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

    let artifacts = payload["artifacts"]
        .as_array()
        .expect("artifacts 必须是数组");
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
    assert!(payload["blocked_reason"]
        .as_str()
        .unwrap()
        .starts_with("command_not_found:"));
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
    assert!(payload["blocked_reason"]
        .as_str()
        .unwrap()
        .starts_with("cwd_not_found:"));
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
    let latest = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-8" }),
    ));
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

    let latest = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-9" }),
    ));
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
        audit["receivedAt"]
            .as_str()
            .map(|value| !value.is_empty())
            .unwrap_or(false),
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
    assert_eq!(
        records[0]["clientInfo"]["name"], "../../untrusted",
        "只记录、不解释"
    );
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
    let mut expected: Vec<String> = (0..writers)
        .map(|index| format!("writer-{index}"))
        .collect();
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
        status["session"]["total"],
        writers + 1,
        "status 必须聚合全部 initialize 记录"
    );
    let listed = status["session"]["records"]
        .as_array()
        .expect("status.session.records 必须是数组");
    assert_eq!(listed.len(), writers + 1);
    assert_eq!(
        listed[0]["source"], "initialize.json",
        "聚合必须包含旧版单文件"
    );
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
    let readback = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-12" }),
    ));
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
    assert_eq!(
        first["env_keys"],
        json!([]),
        "缺省 env 的回执应给出空键列表"
    );

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

    let dispatch_path = runs
        .path()
        .join("t-19")
        .join("attempt-1")
        .join("dispatch.json");
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
    assert!(
        text.contains("task_id_conflict"),
        "必须给具名错误码：{text}"
    );
    assert!(!runs.path().join("t-19").join("attempt-2").exists());
    let latest = tool_payload(&call_tool(
        &server,
        "receipt.get",
        json!({ "task_id": "t-19" }),
    ));
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
        assert_eq!(
            response["result"]["isError"], true,
            "非法 env 必须拒绝：{task_id}"
        );
        let text = tool_text(&response);
        assert!(
            text.contains("invalid_env"),
            "必须给具名错误码 invalid_env：{text}"
        );
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
        response["result"]["isError"],
        false,
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
        assert!(
            text.contains("invalid_pagination"),
            "必须给具名错误码：{text}"
        );
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
    assert_eq!(handback["task_id"], "t-30", "新增时间戳不得改变既有字段");

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
        runs.path()
            .join("t-b")
            .join("attempt-7")
            .join("handback.json"),
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
    assert_eq!(whole["tasks"].as_array().map(|tasks| tasks.len()), Some(3));

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
    assert!(
        gap_task["status"].is_null(),
        "回执损坏时 status 必须为 null"
    );
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

// ---- 预留目录不是任务：runs.list 与 runs.prune 共用同一判据（v1.21 真实数据核验）----

/// 机器判据：`runs/` 下只有「不以 `_`/`.` 开头且是合法 task_id」的目录才算任务。
/// `_session`（initialize 审计）与 `.prune-trash`（删除暂存）即使内含完整 attempt 链，
/// 也不得出现在 `runs.list` 的计数/列表里，不得进入 `runs.prune` 的候选或 skipped。
#[test]
fn reserved_directories_are_never_tasks_in_list_or_prune() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(400);

    // 合法任务：两个 attempt，旧的那个在 365 天下应成为候选
    prune_attempt_status(runs.path(), "t-live", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-live", 2, Some("ok"), Some(&old), true);

    // 伪装成任务的内部目录：都放了两个 attempt，且都足够旧、状态都可删
    prune_attempt_status(runs.path(), "_session", 1, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "_session", 2, Some("ok"), Some(&old), true);
    prune_attempt_status(
        runs.path(),
        ".prune-trash",
        1,
        Some("failed"),
        Some(&old),
        true,
    );
    prune_attempt_status(
        runs.path(),
        ".prune-trash",
        2,
        Some("failed"),
        Some(&old),
        true,
    );

    let list = runs_list_payload(&server, json!({}));
    assert_eq!(list["total_tasks"], 1, "只有 t-live 是任务：{list}");
    assert_eq!(list["total"], 2, "total 只计任务目录下的 attempt：{list}");
    let names: Vec<&str> = list["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .map(|task| task["task_id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(names, vec!["t-live"], "预留目录不得出现在任务列表：{list}");

    let dry = prune_dry(&server, json!({ "older_than_days": 365, "limit": 100 }));
    assert_eq!(dry["state"], "ready");
    assert_eq!(
        dry["candidates_total"], 1,
        "只有 t-live/attempt-1 是候选：{dry}"
    );
    let candidates = dry["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["task_id"], "t-live");
    assert_eq!(candidates[0]["attempt"], 1);
    assert_eq!(candidates[0]["status"], "failed");
    let skipped = dry["skipped"].as_array().expect("skipped");
    assert_eq!(
        dry["skipped_total"], 1,
        "只应保留 t-live 的最新 attempt：{dry}"
    );
    assert_eq!(skipped[0]["task_id"], "t-live");
    assert_eq!(skipped[0]["reason"], "latest_attempt_retained");
    assert!(
        !skipped
            .iter()
            .any(|item| item["task_id"] == "_session" || item["task_id"] == ".prune-trash"),
        "预留目录不得产生任何 skipped 记录：{dry}"
    );

    // dry-run 零写入、零删除
    assert!(!runs.path().join("prune.jsonl").exists());
    assert!(runs.path().join("_session").join("attempt-1").is_dir());
    assert!(runs.path().join(".prune-trash").join("attempt-1").is_dir());
}

// ---- path_prepend：显式前插继承 PATH（v1.9）----

#[test]
fn path_prepend_is_placed_before_inherited_path() {
    let (server, _settings, runs) = server_with_runs();
    let extra = TempDir::new().expect("创建前插目录");
    let prepend_dir = fs::canonicalize(extra.path()).expect("规范化前插目录");
    let expected = prepend_dir.to_string_lossy().to_string();
    let echo_path = if cfg!(windows) {
        "echo %PATH%"
    } else {
        "echo $PATH"
    };
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
        !runs.path().join("t-pp-empty").join("attempt-2").exists(),
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
            payload["attempt"],
            1,
            "第 {} 种空/缺省组合必须命中同一 attempt",
            index + 1
        );
        let text = fs::read_to_string(&dispatch_path).expect("读取派单记录");
        if index == 0 {
            baseline = text;
        } else {
            assert_eq!(
                text,
                baseline,
                "第 {} 种组合改写了签名，说明空集合没有归一",
                index + 1
            );
        }
        assert!(
            !runs.path().join("t-pp-matrix").join("attempt-2").exists(),
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
    {
        assert!(
            reason.contains("挂起态") && reason.contains("加入作业"),
            "Windows 必须写明挂起创建、入作业后才恢复执行：{reason}"
        );
        assert!(
            !reason.contains("spawn→Assign"),
            "窗口收窄后不得继续宣称未加约束的 spawn→Assign 窗口：{reason}"
        );
    }
}

// 描述必须与 v1.20 三态实现同口径：不承诺整棵树，写明受控集合与 Windows 挂起-入作业-恢复顺序
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
            assert!(text.contains(token), "{label} 描述缺少 `{token}`：{text}");
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
        "挂起态",
        "加入作业",
    ] {
        assert!(
            tool_desc.contains(token),
            "task.execute 描述缺少 `{token}`：{tool_desc}"
        );
    }
    assert!(
        !tool_desc.contains("spawn→Assign"),
        "窗口收窄后描述不得继续宣称未加约束的 spawn→Assign 窗口：{tool_desc}"
    );
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

// 正向：受控集合内的孙进程必须被清理，并用 OS 观测点复核（不是 stat/写盘代理）。
//
// Windows 夹具曾经用 `powershell -Command "Start-Process ...; Set-Content pids.txt"` 让被测命令
// 自己写出孙进程 PID：24 路负载下 PowerShell 冷启动到写出文件要 10.7–16.2 s，8 s 超时先触发、
// 文件还没生成，用例读到的是夹具启动慢，不是"进程树没清干净"。现在改成：
//   - 孙进程 = 系统 `ping.exe` 的唯一命名副本 `aw-timeout-grandchild.exe`，由 cmd 直接拉起，
//     不经解释器；副本第一次执行（安全软件首次扫描）放在窗口外的预热里，窗口内只测超时终止；
//   - 调用被测命令前先跑一次 cmd 与孙进程做预热，让镜像/控制台走热路径；
//   - PID 不再由被测命令写盘，而是测试进程在超时窗口内用 Toolhelp 快照轮询观测，
//     并记录首次看到它的时刻；"窗口内确实看到过"是硬断言，避免孙进程没起来的空跑通过。
#[test]
fn timeout_cleans_controlled_set_including_grandchild() {
    let (server, _settings, _runs) = server_with_runs();
    let work = tempfile::tempdir().expect("临时工作目录");
    #[cfg(windows)]
    const SENTINEL: &str = "aw-timeout-grandchild.exe";
    #[cfg(windows)]
    let (cmd, timeout_secs) = {
        let source = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
            .join("System32")
            .join("PING.EXE");
        let sentinel = work.path().join(SENTINEL);
        fs::copy(&source, &sentinel).expect("复制 ping.exe 作为唯一命名的孙进程");
        // 预热：按被测命令完全相同的形态（cmd /C 相对路径 + 重定向）跑一次短命版本，
        // 让 cmd、副本镜像、控制台与路径解析都走热路径，窗口内只测超时终止。
        let _ = std::process::Command::new("cmd")
            .args(["/C", &format!("{SENTINEL} -n 1 127.0.0.1 > NUL")])
            .current_dir(work.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        // 握手夹具：子进程写 started → 拉起孙进程 → 写 ready → 保持存活。
        // 父进程（测试）只有看到 ready 才能确认孙进程已被创建，"窗口内观测"才有前提。
        fs::write(
            work.path().join("handshake.cmd"),
            format!(
                "@echo off\r\n\
                 set \"S=%~dp0{SENTINEL}\"\r\n\
                 echo started>\"%~dp0handshake-started.txt\"\r\n\
                 start \"\" /b \"%S%\" -n %1 127.0.0.1 > NUL\r\n\
                 echo ready>\"%~dp0handshake-ready.txt\"\r\n\
                 ping -n %2 127.0.0.1 > NUL\r\n"
            ),
        )
        .expect("写入握手夹具");
        // 窗口外握手测量：同形态跑一次短命孙进程，记录 spawn→ready 的真实耗时；
        // 等探针孙进程完全退出后再进入窗口，避免它被观测成"窗口内的孙进程"。
        let probe_ready = work.path().join("handshake-ready.txt");
        let _ = fs::remove_file(&probe_ready);
        let probe_start = std::time::Instant::now();
        let mut probe = std::process::Command::new("cmd")
            .args(["/C", "handshake.cmd 2 3"])
            .current_dir(work.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("启动握手测量");
        let deadline = probe_start + Duration::from_secs(60);
        while !probe_ready.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let handshake_ms = probe_start.elapsed().as_millis() as u64;
        assert!(
            probe_ready.exists(),
            "握手测量 60s 内没有写出 ready，夹具不可用"
        );
        let _ = probe.wait();
        let purge_deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let empty = process_image_pids(SENTINEL)
                .map(|(pids, _)| pids.is_empty())
                .unwrap_or(false);
            if empty || std::time::Instant::now() >= purge_deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = fs::remove_file(work.path().join("handshake-started.txt"));
        let _ = fs::remove_file(&probe_ready);
        // 超时按实际握手耗时给足：握手 + 6s 余量，至少 10s。
        let timeout_secs = std::cmp::max(10u64, handshake_ms / 1000 + 6);
        println!(
            "[evidence] grandchild_handshake warm_ms={handshake_ms} timeout_ms={}",
            timeout_secs * 1000
        );
        ("handshake.cmd 60 120".to_string(), timeout_secs)
    };
    #[cfg(not(windows))]
    let cmd = "echo $$ > pids.txt; sleep 30 & echo $! >> pids.txt; wait";
    #[cfg(not(windows))]
    let timeout_secs = 1;

    #[cfg(windows)]
    let seen: Arc<std::sync::Mutex<Vec<u32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    #[cfg(windows)]
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // 诊断：轮询次数、快照失败次数、每次快照枚举到的行数、首次看到孙进程的时刻。
    // Toolhelp 快照在进程表剧烈变动时可能拿不到内容（ERROR_BAD_LENGTH），
    // 需要区分"孙进程没起来"/"观测本身失败"/"进程起得太慢超过窗口"。
    #[cfg(windows)]
    let polls = Arc::new(std::sync::atomic::AtomicU64::new(0));
    #[cfg(windows)]
    let snapshot_failures = Arc::new(std::sync::atomic::AtomicU64::new(0));
    #[cfg(windows)]
    let max_rows = Arc::new(std::sync::atomic::AtomicU64::new(0));
    #[cfg(windows)]
    let first_seen_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    // 诊断：观测到的孙进程 PID 集合变化时间线，用来区分"窗口内根本没有孙进程"
    // 与"孙进程被观测到、随后被清理"。
    #[cfg(windows)]
    let timeline: Arc<std::sync::Mutex<Vec<(u64, String)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    #[cfg(windows)]
    let first_poll_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    // 计时基准放在调用线程里：线程被调度延迟时，线程内部的 Instant 会把"起得太慢"记成"很快"
    #[cfg(windows)]
    let call_start = std::time::Instant::now();
    #[cfg(windows)]
    let watcher = {
        let seen = Arc::clone(&seen);
        let stop = Arc::clone(&stop);
        let polls = Arc::clone(&polls);
        let snapshot_failures = Arc::clone(&snapshot_failures);
        let max_rows = Arc::clone(&max_rows);
        let first_seen_ms = Arc::clone(&first_seen_ms);
        let timeline = Arc::clone(&timeline);
        let first_poll_ms = Arc::clone(&first_poll_ms);
        std::thread::spawn(move || {
            let mut last_pids: Vec<u32> = Vec::new();
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                polls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let elapsed = call_start.elapsed().as_millis() as u64;
                if first_poll_ms.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                    // +1 让"第一次轮询发生在 0ms"也能与"还没有轮询"区分开
                    first_poll_ms.store(elapsed + 1, std::sync::atomic::Ordering::Relaxed);
                }
                match process_rows(&[SENTINEL]) {
                    Some((matched, rows)) => {
                        max_rows.fetch_max(rows as u64, std::sync::atomic::Ordering::Relaxed);
                        let pids: Vec<u32> = matched
                            .iter()
                            .filter(|(name, _)| name.eq_ignore_ascii_case(SENTINEL))
                            .map(|(_, pid)| *pid)
                            .collect();
                        if pids != last_pids {
                            timeline
                                .lock()
                                .expect("时间线加锁")
                                .push((elapsed, format!("{pids:?}")));
                            last_pids = pids.clone();
                        }
                        if !pids.is_empty()
                            && first_seen_ms.load(std::sync::atomic::Ordering::Relaxed) == 0
                        {
                            first_seen_ms.store(elapsed + 1, std::sync::atomic::Ordering::Relaxed);
                        }
                        let mut seen = seen.lock().expect("观测结果加锁");
                        for pid in pids {
                            if !seen.contains(&pid) {
                                seen.push(pid);
                            }
                        }
                    }
                    None => {
                        snapshot_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        })
    };

    // 窗口内：执行放独立线程，主线程按 ready 文件做握手确认（不改变被测命令的超时语义）。
    #[cfg(windows)]
    let (payload, ready_seen, ready_delay_ms) = {
        let call_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker = {
            let server = server;
            let done = Arc::clone(&call_done);
            let cwd = work.path().to_path_buf();
            let cmd = cmd.clone();
            std::thread::spawn(move || {
                let result = call_tool(
                    &server,
                    "task.execute",
                    json!({
                        "task_id": "t-to-grandchild",
                        "cmd": cmd,
                        "cwd": cwd,
                        "timeout_secs": timeout_secs
                    }),
                );
                done.store(true, std::sync::atomic::Ordering::SeqCst);
                result
            })
        };
        let ready_path = work.path().join("handshake-ready.txt");
        let mut ready_seen = false;
        let mut ready_delay_ms = 0u64;
        let deadline = call_start + Duration::from_secs(timeout_secs + 20);
        loop {
            if ready_path.exists() {
                ready_seen = true;
                ready_delay_ms = call_start.elapsed().as_millis() as u64;
                break;
            }
            if call_done.load(std::sync::atomic::Ordering::SeqCst)
                || std::time::Instant::now() >= deadline
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let payload = tool_payload(&worker.join().expect("执行线程不得 panic"));
        (payload, ready_seen, ready_delay_ms)
    };
    #[cfg(not(windows))]
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
    #[cfg(windows)]
    {
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        watcher.join().expect("观测线程正常退出");
    }
    assert_eq!(payload["status"], "blocked");
    let reason = payload["blocked_reason"].as_str().unwrap_or("");
    println!("[evidence] grandchild_reason={reason}");
    assert!(
        reason.contains("已清理受控集合"),
        "受控集合内清理应成功：{reason}"
    );

    #[cfg(windows)]
    {
        let observed = seen.lock().expect("观测结果加锁").clone();
        let polls = polls.load(std::sync::atomic::Ordering::Relaxed);
        let failures = snapshot_failures.load(std::sync::atomic::Ordering::Relaxed);
        let rows = max_rows.load(std::sync::atomic::Ordering::Relaxed);
        let first = first_seen_ms.load(std::sync::atomic::Ordering::Relaxed);
        let first_poll = first_poll_ms.load(std::sync::atomic::Ordering::Relaxed);
        let timeline = timeline.lock().expect("时间线加锁").clone();
        let survivors = process_image_pids(SENTINEL);
        println!(
            "[evidence] grandchild_watch pids={observed:?} first_seen_ms={first} \
first_poll_ms={} call_ms={} payload_duration_ms={} polls={polls} \
snapshot_failures={failures} max_rows={rows} file_exists={} after_cleanup={:?} process_exists={:?}",
            first_poll.saturating_sub(1),
            call_start.elapsed().as_millis(),
            payload["duration_ms"],
            work.path().join(SENTINEL).exists(),
            survivors.as_ref().map(|(pids, _)| pids.clone()),
            observed
                .iter()
                .map(|pid| (*pid, process_exists(*pid)))
                .collect::<Vec<_>>()
        );
        println!("[evidence] grandchild_timeline {:?}", timeline);
        // 明确的判定结果：红了第一眼先看这里——是"机器太慢、前提没建立"，还是"产品没清理干净"。
        let survivors_pids = survivors.as_ref().map(|(pids, _)| pids.clone());
        let verdict = match (&survivors_pids, observed.is_empty()) {
            (Some(pids), _) if !pids.is_empty() => "terminate_defect",
            (None, _) => "unobservable",
            (Some(_), true) => "premise_not_established",
            (Some(_), false) => "pass",
        };
        let timeout_ms = timeout_secs * 1000;
        let handshake = if !ready_seen {
            "not_completed"
        } else if ready_delay_ms < timeout_ms {
            "before_timeout"
        } else {
            "after_timeout"
        };
        println!(
            "[verdict] grandchild_case={verdict} ready_seen={ready_seen} \
ready_delay_ms={ready_delay_ms} timeout_ms={timeout_ms} handshake={handshake} \
observed={} survivors={:?} call_ms={} payload_duration_ms={} polls={polls} \
snapshot_failures={failures} premise=窗口内观测到唯一命名的孙进程",
            observed.len(),
            survivors_pids,
            call_start.elapsed().as_millis(),
            payload["duration_ms"],
        );
        let premise_detail = if !ready_seen {
            "握手未完成，本条不构成裁决依据（handshake_not_completed）"
        } else if ready_delay_ms >= timeout_ms {
            "超时早于 ready（timeout_before_ready），本条不构成裁决依据"
        } else {
            "ready 已出现但窗口内未观测到孙进程（ready_seen_but_not_observed）"
        };
        assert!(
            !observed.is_empty(),
            "前提未建立（环境判定，不是终止逻辑判据）：{premise_detail}；\
超时窗口内没有观测到孙进程 {SENTINEL}，见 [verdict] grandchild_case=premise_not_established \
ready_seen={ready_seen} ready_delay_ms={ready_delay_ms} timeout_ms={timeout_ms}；\
该轮既不能证明也不能否证整树清理能力"
        );
        assert_eq!(
            survivors_pids,
            Some(Vec::new()),
            "终止逻辑缺陷（产品判据）：作业内的孙进程没有被清理干净（观测到的 PID {observed:?}）"
        );
    }
    #[cfg(not(windows))]
    {
        let pid_text = fs::read_to_string(work.path().join("pids.txt")).expect("命令必须写出 PID");
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

#[cfg(windows)]
fn process_rows(images: &[&str]) -> Option<(Vec<(String, u32)>, usize)> {
    use std::ffi::CStr;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };
    let mut matched = Vec::new();
    let mut rows = 0usize;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snapshot, &mut entry);
        if ok == 0 {
            // 首行就失败说明快照内容不可用（进程表在枚举期间发生变动时会出现），
            // 不能当成"系统里一个进程都没有"这类有效观测。
            CloseHandle(snapshot);
            return None;
        }
        while ok != 0 {
            rows += 1;
            let name = CStr::from_ptr(entry.szExeFile.as_ptr()).to_string_lossy();
            if images.iter().any(|image| name.eq_ignore_ascii_case(image)) {
                matched.push((name.into_owned(), entry.th32ProcessID));
            }
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
            ok = Process32Next(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    Some((matched, rows))
}

#[cfg(windows)]
fn process_image_pids(image: &str) -> Option<(Vec<u32>, usize)> {
    process_rows(&[image])
        .map(|(matched, rows)| (matched.into_iter().map(|row| row.1).collect(), rows))
}

#[test]
fn legacy_timeout_reason_is_returned_verbatim_and_not_rewritten() {
    let (server, _settings, runs) = server_with_runs();
    let attempt = runs.path().join("t-to-legacy").join("attempt-1");
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

/// 在 `prune_attempt` 基础上改写 `handback.status`：`None` 表示回执缺 status 字段
fn prune_attempt_status(
    runs: &Path,
    task_id: &str,
    attempt: u64,
    status: Option<&str>,
    finished_at: Option<&str>,
    with_dispatch: bool,
) -> PathBuf {
    let dir = prune_attempt(runs, task_id, attempt, true, finished_at, with_dispatch);
    let mut record = json!({
        "task_id": task_id,
        "attempt": attempt,
        "exit_code": 0
    });
    if let Some(value) = status {
        record["status"] = json!(value);
    }
    if let Some(stamp) = finished_at {
        record["finishedAt"] = json!(stamp);
    }
    fs::write(dir.join("handback.json"), record.to_string()).expect("写入 handback.json");
    dir
}

fn prune_schema(server: &Server) -> Value {
    let response = call(
        server,
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/list"}),
    );
    response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组")
        .iter()
        .find(|tool| tool["name"] == "runs.prune")
        .expect("缺少 runs.prune")["inputSchema"]
        .clone()
}

fn prune_tool_description(server: &Server) -> String {
    let response = call(
        server,
        json!({"jsonrpc": "2.0", "id": 6, "method": "tools/list"}),
    );
    response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组")
        .iter()
        .find(|tool| tool["name"] == "runs.prune")
        .expect("缺少 runs.prune")["description"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn prune_error(server: &Server, arguments: Value) -> String {
    let response = call_tool(server, "runs.prune", arguments);
    assert_eq!(
        response["result"]["isError"],
        true,
        "该形态必须拒绝：{}",
        tool_text(&response)
    );
    tool_text(&response)
}

fn prune_dry(server: &Server, arguments: Value) -> Value {
    let response = call_tool(server, "runs.prune", arguments);
    assert_eq!(
        response["result"]["isError"],
        false,
        "dry-run 应成功：{}",
        tool_text(&response)
    );
    tool_payload(&response)
}

// ---- runs.prune：status 多值白名单（v1.21，提案 23 条）----

#[test]
fn prune_status_schema_and_legacy_compatibility() {
    let (server, _settings, runs) = server_with_runs();
    let schema = prune_schema(&server);
    assert_eq!(schema["properties"]["status"]["type"], "array", "case 1");
    assert_eq!(schema["properties"]["status"]["minItems"], 1, "case 1");
    assert_eq!(
        schema["properties"]["status"]["uniqueItems"], true,
        "case 1"
    );
    assert_eq!(
        schema["properties"]["status"]["items"]["type"], "string",
        "case 1"
    );
    assert_eq!(
        schema["properties"]["status"]["items"]["enum"],
        json!(["ok", "failed", "blocked", "canceled"]),
        "case 1"
    );
    assert_eq!(
        schema["required"],
        json!(["older_than_days"]),
        "case 1：年龄阈值仍必填"
    );
    println!("[case 1] schema status=array(minItems=1,uniqueItems,enum=[ok,failed,blocked,canceled]) PASS");

    let description = prune_tool_description(&server);
    assert!(
        description.contains("status") && description.contains("显式"),
        "case 23：描述必须写明 status 显式传入才生效：{description}"
    );
    assert!(
        description.contains("白名单") && description.contains("缺省不过滤"),
        "case 23：描述必须写明白名单与缺省不过滤：{description}"
    );
    println!("[case 23] runs.prune 描述与 status 语义一致 PASS");

    let old = days_ago_stamp(40);
    for (task, status) in [
        ("t-ok", "ok"),
        ("t-failed", "failed"),
        ("t-blocked", "blocked"),
    ] {
        prune_attempt_status(runs.path(), task, 1, Some(status), Some(&old), true);
        prune_attempt_status(runs.path(), task, 2, Some(status), Some(&old), true);
    }

    let legacy = prune_dry(&server, json!({ "older_than_days": 30 }));
    assert_eq!(legacy["status_filter"], Value::Null, "case 2：缺省不过滤");
    assert_eq!(legacy["candidates_total"], 3, "case 2：三种状态都进入候选");
    let candidates = legacy["candidates"].as_array().expect("candidates");
    let statuses: Vec<&str> = candidates
        .iter()
        .map(|item| item["status"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        statuses,
        vec!["blocked", "failed", "ok"],
        "case 21：旧调用方字段不变，候选新增 status 回显"
    );
    assert_eq!(legacy["older_than_days"], 30, "case 21：旧字段不变");
    assert_eq!(legacy["limit"], 100, "case 21：旧字段不变");
    assert_eq!(legacy["dry_run"], true, "case 21：旧字段不变");
    assert_eq!(
        legacy["skipped_total"], 3,
        "case 21：最新 attempt 仍按旧口径跳过"
    );
    println!("[case 2] 缺省 status 与 v1.20 候选集合一致 PASS");
    println!("[case 21] 不传 status 时旧字段/旧行为不变 PASS");
}

#[test]
fn prune_status_whitelist_selection() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    let young = days_ago_stamp(5);

    // t-mix：attempt-1=ok、attempt-2=blocked、attempt-3=ok（最新）
    prune_attempt_status(runs.path(), "t-mix", 1, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-mix", 2, Some("blocked"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-mix", 3, Some("ok"), Some(&old), true);
    // t-failed：attempt-1=failed、attempt-2=failed（最新）
    prune_attempt_status(runs.path(), "t-failed", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-failed", 2, Some("failed"), Some(&old), true);
    // t-young：attempt-1=failed 但只有 5 天，attempt-2=failed 最新
    prune_attempt_status(
        runs.path(),
        "t-young",
        1,
        Some("failed"),
        Some(&young),
        true,
    );
    prune_attempt_status(runs.path(), "t-young", 2, Some("failed"), Some(&old), true);
    // t-nostatus：attempt-1 缺 status，attempt-2=ok 最新
    prune_attempt_status(runs.path(), "t-nostatus", 1, None, Some(&old), true);
    prune_attempt_status(runs.path(), "t-nostatus", 2, Some("ok"), Some(&old), true);
    // t-weird：attempt-1 状态值不认识，attempt-2=ok 最新
    prune_attempt_status(runs.path(), "t-weird", 1, Some("weird"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-weird", 2, Some("ok"), Some(&old), true);

    let failed = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"] }),
    );
    assert_eq!(failed["status_filter"], json!(["failed"]), "case 3");
    let candidates = failed["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 1, "case 3：只保留 failed 的非最新候选");
    assert!(
        candidates
            .iter()
            .all(|item| item["status"] == "failed" && item["task_id"] == "t-failed"),
        "case 3：ok/blocked 不得进入候选：{failed}"
    );
    let skipped = failed["skipped"].as_array().expect("skipped");
    assert!(
        skipped.iter().any(|item| item["task_id"] == "t-mix"
            && item["attempt"] == 2
            && item["reason"] == "status_mismatch"),
        "case 3：blocked 必须标 status_mismatch：{failed}"
    );
    println!("[case 3] status=[failed] 只删 failed，非白名单进 status_mismatch PASS");

    let multi = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["ok", "blocked"] }),
    );
    assert_eq!(multi["status_filter"], json!(["ok", "blocked"]), "case 4");
    let multi_candidates = multi["candidates"].as_array().expect("candidates");
    assert!(
        multi_candidates
            .iter()
            .all(|item| item["status"] == "ok" || item["status"] == "blocked"),
        "case 4：failed 不得进入候选：{multi}"
    );
    assert!(
        multi_candidates
            .iter()
            .any(|item| item["task_id"] == "t-mix"
                && item["attempt"] == 2
                && item["status"] == "blocked"),
        "case 4：blocked 必须在白名单内被选中"
    );
    println!("[case 4] 多值白名单取并集，failed 不进入候选 PASS");

    let young_check = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"] }),
    );
    let young_skipped = young_check["skipped"].as_array().expect("skipped");
    assert!(
        young_skipped.iter().any(|item| item["task_id"] == "t-young"
            && item["attempt"] == 1
            && item["reason"] == "not_old_enough"),
        "case 10：status 与 older_than_days 必须是 AND：{young_check}"
    );
    assert!(
        !young_check["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .any(|item| item["task_id"] == "t-young"),
        "case 10：未满阈值的 failed 不得删"
    );
    println!("[case 10] status + older_than_days 为 AND，未满阈值不删 PASS");

    let latest_check = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"] }),
    );
    assert!(
        latest_check["skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .any(|item| item["task_id"] == "t-failed"
                && item["attempt"] == 2
                && item["reason"] == "latest_attempt_retained"),
        "case 11：最新 attempt 即使状态命中也不删：{latest_check}"
    );
    println!("[case 11] 最新 attempt 永不删，status 白名单不能选中 PASS");

    for (task, expected_reason) in [
        ("t-nostatus", "status_unknown"),
        ("t-weird", "status_unknown"),
    ] {
        let check = prune_dry(&server, json!({ "older_than_days": 30, "status": ["ok"] }));
        assert!(
            !check["candidates"]
                .as_array()
                .expect("candidates")
                .iter()
                .any(|item| item["task_id"] == task),
            "case 12/13：{task} 不得进入候选：{check}"
        );
        assert!(
            check["skipped"]
                .as_array()
                .expect("skipped")
                .iter()
                .any(|item| item["task_id"] == task
                    && item["attempt"] == 1
                    && item["reason"] == expected_reason),
            "case 12/13：{task} 必须标 {expected_reason}：{check}"
        );
    }
    println!("[case 12] 回执缺 status → status_unknown，不删 PASS");
    println!("[case 13] 未知状态值 → status_unknown，不删 PASS");

    for task in ["t-l1", "t-l2", "t-l3"] {
        prune_attempt_status(runs.path(), task, 1, Some("failed"), Some(&old), true);
        prune_attempt_status(runs.path(), task, 2, Some("failed"), Some(&old), true);
    }
    let limited = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"], "limit": 1 }),
    );
    assert!(
        limited["candidates_total"].as_u64().unwrap_or(0) >= 3,
        "case 14：limit 必须在过滤之后生效（总数是过滤后命中数）：{limited}"
    );
    assert_eq!(limited["truncated"], true, "case 14");
    assert_eq!(
        limited["candidates"].as_array().expect("candidates").len(),
        1,
        "case 14：批次只取 limit 个"
    );
    println!("[case 14] limit 在 status 过滤之后生效，candidates_total 为过滤后总数 PASS");

    prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"], "limit": 1 }),
    );
    prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"], "limit": 100 }),
    );
    for bad in [0, 101] {
        let text = prune_error(
            &server,
            json!({ "older_than_days": 30, "status": ["failed"], "limit": bad }),
        );
        assert!(text.contains("invalid_retention"), "case 15：{bad}：{text}");
    }
    println!("[case 15] limit 1/100 通过，0/101 → invalid_retention PASS");
}

#[test]
fn prune_status_validation_is_fail_closed() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt_status(runs.path(), "t-guard", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-guard", 2, Some("failed"), Some(&old), true);

    let invalid_forms = [
        (json!([]), "case 5"),
        (json!(["unknown"]), "case 6"),
        (json!(["Failed"]), "case 6"),
        (json!("failed"), "case 7"),
        (Value::Null, "case 8"),
    ];
    for (status, label) in invalid_forms {
        let text = prune_error(&server, json!({ "older_than_days": 30, "status": status }));
        assert!(text.contains("invalid_retention"), "{label}：{text}");
    }
    println!("[case 5] 空数组 → invalid_retention，零删除零审计 PASS");
    println!("[case 6] 未知值/大小写不符 → invalid_retention PASS");
    println!("[case 7] 非数组 → invalid_retention PASS");
    println!("[case 8] null → invalid_retention，不当作不过滤 PASS");

    let dedup = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed", "failed"] }),
    );
    assert_eq!(
        dedup["status_filter"],
        json!(["failed"]),
        "case 9：重复值去重后回显"
    );
    println!("[case 9] 重复值去重后正常执行，status_filter 回显去重结果 PASS");

    let entries_before = fs::read_dir(runs.path()).expect("读 runs").count();
    let dry = prune_dry(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"] }),
    );
    assert_eq!(dry["dry_run"], true, "case 16");
    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "case 16：dry-run 不得写审计"
    );
    assert!(
        !runs.path().join(".prune-trash").exists(),
        "case 16：dry-run 不得创建 trash"
    );
    assert_eq!(
        fs::read_dir(runs.path()).expect("读 runs").count(),
        entries_before,
        "case 16：dry-run 零写入"
    );
    println!("[case 16] status 过滤的 dry-run 仍零写入 PASS");

    let text = prune_error(
        &server,
        json!({ "older_than_days": 30, "status": ["failed"], "dry_run": false }),
    );
    assert!(text.contains("confirm_required"), "case 17：{text}");
    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "case 17：缺 confirm 不得写审计"
    );
    assert!(
        runs.path().join("t-guard").join("attempt-1").is_dir(),
        "case 17：缺 confirm 不得删除"
    );
    println!("[case 17] status 合法但缺 confirm → confirm_required，零删除零审计 PASS");
}

#[test]
fn prune_status_audit_and_mixed_batch() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt_status(runs.path(), "t-mix", 1, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-mix", 2, Some("blocked"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-mix", 3, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-failed", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-failed", 2, Some("failed"), Some(&old), true);

    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 30,
            "status": ["ok", "failed"],
            "dry_run": false,
            "confirm": true
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status_filter"], json!(["ok", "failed"]), "case 19");
    assert_eq!(payload["deleted_total"], 2, "case 19：ok+failed 两类都删");
    assert_eq!(payload["failed_total"], 0, "case 19");
    assert!(
        runs.path()
            .join("t-mix")
            .join("attempt-2")
            .join("handback.json")
            .is_file(),
        "case 19：blocked 不在白名单，必须保留"
    );
    assert!(
        !runs.path().join("t-mix").join("attempt-1").exists(),
        "case 19：ok 应被删除"
    );
    assert!(
        !runs.path().join("t-failed").join("attempt-1").exists(),
        "case 19：failed 应被删除"
    );
    println!("[case 19] 混合白名单按状态删除，blocked 保留、计数正确 PASS");

    let audit = fs::read_to_string(runs.path().join("prune.jsonl")).expect("必须写审计");
    let lines: Vec<&str> = audit
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2, "case 18：plan + result 两行");
    for line in &lines {
        let record: Value = serde_json::from_str(line).expect("审计行必须是 JSON");
        assert_eq!(record["status_filter"], json!(["ok", "failed"]), "case 18");
        assert_eq!(record["older_than_days"], 30, "case 18");
        assert_eq!(record["limit"], 100, "case 18");
        assert!(record["batch_id"].as_str().is_some(), "case 18");
    }
    println!("[case 18] plan/result 均记录 status_filter、older_than_days、limit、batch_id PASS");
}

#[cfg(windows)]
#[test]
fn prune_status_partial_failure_is_reported_as_partial() {
    use std::os::windows::fs::OpenOptionsExt;

    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt_status(
        runs.path(),
        "t-partial",
        1,
        Some("failed"),
        Some(&old),
        true,
    );
    prune_attempt_status(
        runs.path(),
        "t-partial",
        2,
        Some("failed"),
        Some(&old),
        true,
    );
    prune_attempt_status(
        runs.path(),
        "t-partial",
        3,
        Some("failed"),
        Some(&old),
        true,
    );
    let attempt_one = runs.path().join("t-partial").join("attempt-1");
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(attempt_one.join("stdout.txt"))
        .expect("以独占共享模式打开文件");

    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 30,
            "status": ["failed"],
            "dry_run": false,
            "confirm": true
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "case 20：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "partial", "case 20");
    assert_eq!(payload["deleted_total"], 1, "case 20：其余白名单候选仍删");
    assert_eq!(payload["failed_total"], 1, "case 20");
    assert_eq!(payload["failed"][0]["attempt"], 1, "case 20");
    assert_eq!(payload["status_filter"], json!(["failed"]), "case 20");
    let failed = &payload["failed"][0];
    assert_eq!(
        failed["state"], "retained_at_source",
        "case 20：失败项必须显式标注仍在原处"
    );
    assert_eq!(failed["source_path"], "runs/t-partial/attempt-1", "case 20");
    assert_eq!(
        failed["trash_path"],
        Value::Null,
        "case 20：未移动过的失败项 trash_path 必须为 null"
    );
    assert_eq!(failed["rule"]["older_than_days"], 30, "case 20");
    assert_eq!(
        failed["rule"]["status_filter"],
        json!(["failed"]),
        "case 20"
    );
    assert_eq!(failed["rule"]["batch_id"], payload["batch_id"], "case 20");
    assert!(
        failed["fingerprint"]["handback_sha256"].as_str().is_some(),
        "case 20：失败项也要带删除前指纹"
    );
    assert!(attempt_one.is_dir(), "case 20：失败项不得离开原路径");
    assert_eq!(payload["trashed_total"], 0, "case 20");
    assert!(
        payload["trashed"].as_array().expect("trashed").is_empty(),
        "case 20"
    );
    assert_eq!(
        payload["deleted"][0]["state"], "deleted",
        "case 20：唯一表示已移除的集合"
    );
    assert!(
        payload["deleted"][0]["fingerprint"]["handback_sha256"]
            .as_str()
            .is_some(),
        "case 20：deleted 必须带指纹"
    );
    for item in payload["failed"]
        .as_array()
        .expect("failed")
        .iter()
        .chain(payload["trashed"].as_array().expect("trashed").iter())
    {
        assert_ne!(
            item["state"], "deleted",
            "case 20：failed/trashed 不得自称已删除"
        );
        assert!(
            !item.to_string().to_lowercase().contains("\"deleted\""),
            "case 20：failed/trashed 条目不得出现 deleted 字样：{item}"
        );
    }
    println!("[case 20] status 过滤下 partial 仍 partial + isError=true，其余照删 PASS");
}

#[test]
fn prune_status_capability_gate_and_no_server_side_tier() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt_status(runs.path(), "t-gate", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-gate", 2, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-gate2", 1, Some("ok"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-gate2", 2, Some("ok"), Some(&old), true);

    let schema = prune_schema(&server);
    assert!(
        schema["properties"].get("status").is_some(),
        "case 22：schema 必须声明 status，这是 PM 侧能力门禁的唯一依据"
    );
    println!("[case 22] schema 含 status=能力位；不含时 PM 必须禁止携带并真删 PASS");

    let tier = prune_dry(&server, json!({ "older_than_days": 30, "tier": "failed" }));
    assert_eq!(tier["status_filter"], Value::Null, "case 22");
    assert_eq!(
        tier["candidates_total"], 2,
        "case 22：tier 被忽略、不产生服务端分档：{tier}"
    );
    assert!(
        tier["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .any(|item| item["status"] == "failed")
            && tier["candidates"]
                .as_array()
                .expect("candidates")
                .iter()
                .any(|item| item["status"] == "ok"),
        "case 22：tier 不得被当作状态白名单"
    );
    println!("[case 22] 无服务端 tier；未知参数被忽略，故 PM 必须靠 schema 门禁 PASS");
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
    assert_eq!(payload["limit"], 100, "未传 limit 时默认 100");
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
    assert!(runs.path().join("_session").is_dir(), "不得动 _ 前缀目录");
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

    let audit = fs::read_to_string(runs.path().join("prune.jsonl")).expect("真实删除必须写审计");
    let lines: Vec<&str> = audit
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2, "每批必须 plan + result 两行：{audit}");
    let plan: Value = serde_json::from_str(lines[0]).expect("plan 行必须是 JSON");
    let result: Value = serde_json::from_str(lines[1]).expect("result 行必须是 JSON");
    assert_eq!(plan["phase"], "plan", "第一行必须是删除之前的计划");
    assert_eq!(plan["older_than_days"], 30);
    assert_eq!(plan["candidate_count"], 1);
    assert_eq!(plan["candidates"][0]["finishedAt"], old);
    assert_eq!(
        plan["candidates"][0]["dispatchedAt"],
        "2026-01-01T00:00:00Z"
    );
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

    let payload = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 10 }),
    ));
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
    assert_eq!(
        reason_of("t-unfinished", 1),
        "unfinished_or_missing_handback"
    );
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

/// 删除后重跑：被删 attempt 不得再次成为候选，也不得产生 skipped 噪声。
#[test]
fn prune_rerun_after_delete_has_no_phantom_candidates_or_skips() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(400);
    for attempt in 1..=3 {
        prune_attempt(runs.path(), "t-rerun", attempt, true, Some(&old), true);
    }

    let first = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365, "limit": 100, "dry_run": false, "confirm": true }),
    ));
    assert_eq!(first["status"], "ok");
    assert_eq!(first["deleted_total"], 2, "两个旧 attempt 应被删：{first}");
    assert_eq!(first["failed_total"], 0);
    assert!(!runs.path().join("t-rerun").join("attempt-1").exists());
    assert!(!runs.path().join("t-rerun").join("attempt-2").exists());
    assert!(runs.path().join("t-rerun").join("attempt-3").is_dir());
    let trash = runs.path().join(".prune-trash");
    if trash.is_dir() {
        assert_eq!(
            fs::read_dir(&trash).expect("读暂存目录").count(),
            0,
            "成功删除后 .prune-trash 不得残留条目"
        );
    }

    let again = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365, "limit": 100 }),
    ));
    assert_eq!(
        again["candidates_total"], 0,
        "已删 attempt 不得再次成为候选：{again}"
    );
    assert!(
        again["candidates"]
            .as_array()
            .expect("candidates")
            .is_empty(),
        "重跑候选必须为空：{again}"
    );
    assert_eq!(
        again["skipped_total"], 1,
        "重跑不应产生额外 skipped 噪声：{again}"
    );
    let skipped = again["skipped"].as_array().expect("skipped");
    assert_eq!(skipped[0]["attempt"], 3);
    assert_eq!(skipped[0]["reason"], "latest_attempt_retained");
    assert!(
        !skipped
            .iter()
            .any(|item| item["attempt"] == 1 || item["attempt"] == 2),
        "已删 attempt 不得出现在 skipped：{again}"
    );
}

/// 删除回执自包含：每条 deleted 都带指纹（删除前算好）、trash_path、deleted_at 与整批 rule。
#[test]
fn prune_receipt_deleted_entries_carry_fingerprint_and_rule() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(400);
    prune_attempt_status(runs.path(), "t-fp", 1, Some("failed"), Some(&old), true);
    prune_attempt_status(runs.path(), "t-fp", 2, Some("ok"), Some(&old), true);

    let handback = runs
        .path()
        .join("t-fp")
        .join("attempt-1")
        .join("handback.json");
    let dispatch = runs
        .path()
        .join("t-fp")
        .join("attempt-1")
        .join("dispatch.json");
    let handback_bytes = fs::read(&handback).expect("读 handback");
    let dispatch_bytes = fs::read(&dispatch).expect("读 dispatch");
    let handback_sha = hex::encode(Sha256::digest(&handback_bytes));
    let dispatch_sha = hex::encode(Sha256::digest(&dispatch_bytes));

    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 365,
            "status": ["failed"],
            "limit": 100,
            "dry_run": false,
            "confirm": true
        }),
    );
    let payload = tool_payload(&response);
    let batch = payload["batch_id"].as_str().expect("batch_id").to_string();
    assert_eq!(payload["deleted_total"], 1, "{}", tool_text(&response));
    let item = &payload["deleted"][0];

    // 旧字段全部保留
    assert_eq!(item["task_id"], "t-fp");
    assert_eq!(item["attempt"], 1);
    assert_eq!(item["path"], "runs/t-fp/attempt-1");
    assert_eq!(item["status"], "failed");
    assert_eq!(item["finishedAt"], old);
    assert!(item["dispatchedAt"].as_str().is_some());
    assert!(item["age_days"].as_u64().is_some());

    // 新增字段：状态、去向、时间、整批 rule
    assert_eq!(item["state"], "deleted");
    assert_eq!(item["source_path"], "runs/t-fp/attempt-1");
    assert_eq!(
        item["trash_path"].as_str(),
        Some(format!("runs/.prune-trash/{batch}/t-fp-attempt-1").as_str())
    );
    assert!(
        item["deleted_at"].as_str().is_some(),
        "deleted_at 必须逐条给出"
    );
    assert_eq!(item["rule"]["older_than_days"], 365);
    assert_eq!(item["rule"]["status_filter"], json!(["failed"]));
    assert_eq!(item["rule"]["limit"], 100);
    assert_eq!(item["rule"]["batch_id"], batch);

    // 指纹：删除前算出的真实 sha256 与字节数
    let fingerprint = &item["fingerprint"];
    assert_eq!(
        fingerprint["handback_sha256"].as_str(),
        Some(handback_sha.as_str()),
        "handback sha256 必须与删除前文件一致：{item}"
    );
    assert_eq!(
        fingerprint["dispatch_sha256"].as_str(),
        Some(dispatch_sha.as_str()),
        "dispatch sha256 必须与删除前文件一致：{item}"
    );
    assert_eq!(
        fingerprint["bytes"]["handback"].as_u64(),
        Some(handback_bytes.len() as u64)
    );
    assert_eq!(
        fingerprint["bytes"]["dispatch"].as_u64(),
        Some(dispatch_bytes.len() as u64)
    );
    assert_eq!(
        fingerprint["bytes"]["total"].as_u64(),
        Some((handback_bytes.len() + dispatch_bytes.len()) as u64)
    );
    assert_eq!(fingerprint["status"], "failed");
    assert_eq!(fingerprint["finishedAt"], old);
    assert!(fingerprint["dispatchedAt"].as_str().is_some());

    // 本批无失败、无暂存：两个集合必须存在且为零
    assert_eq!(payload["failed_total"], 0);
    assert_eq!(payload["trashed_total"], 0);
    assert!(payload["trashed"].as_array().expect("trashed").is_empty());

    // 删除后重跑仍无幽灵候选
    let again = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365, "limit": 100 }),
    ));
    assert_eq!(again["candidates_total"], 0, "{}", tool_text(&again));
    assert_eq!(again["skipped_total"], 1);
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
        !payload["failed"][0]["error"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
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

// ---- 描述审计自动门禁：依赖运行时行为的断言（文档 §21）----

#[test]
fn description_gate_behavior_additional_properties_declared_only() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    );
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组");
    for tool in tools {
        let name = tool["name"].as_str().expect("工具缺少 name");
        assert_eq!(
            tool["inputSchema"]["additionalProperties"], false,
            "{name} 必须声明 additionalProperties:false"
        );
    }

    // 只声明不强制：服务端不拒绝未知参数
    let unknown = call_tool(&server, "runs.list", json!({ "bogus": 1 }));
    assert_eq!(
        unknown["result"]["isError"],
        false,
        "未知参数不得被拒绝：{}",
        tool_text(&unknown)
    );
    // 未知参数也不得生效：`tier` 不产生服务端分档
    let dry = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365, "tier": "30" }),
    );
    assert_eq!(dry["result"]["isError"], false, "{}", tool_text(&dry));
    let payload = tool_payload(&dry);
    assert_eq!(payload["dry_run"], true);
    assert!(
        payload.get("tier").is_none(),
        "未知参数 tier 不得生效：{payload}"
    );
    assert_eq!(
        payload["status_filter"],
        Value::Null,
        "未传 status 时不得有默认状态：{payload}"
    );
}

#[test]
fn description_gate_behavior_status_capability_and_filtering() {
    let (server, _settings, runs) = server_with_runs();
    let response = call(
        &server,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let prune = response["result"]["tools"]
        .as_array()
        .expect("tools 必须是数组")
        .iter()
        .find(|tool| tool["name"] == "runs.prune")
        .expect("缺少 runs.prune");
    let status = &prune["inputSchema"]["properties"]["status"];
    assert_eq!(status["type"], "array", "status 能力位必须是数组");
    assert_eq!(status["minItems"], 1);
    assert_eq!(status["uniqueItems"], true);
    let allowed: Vec<&str> = status["items"]["enum"]
        .as_array()
        .expect("status 必须有 enum")
        .iter()
        .map(|value| value.as_str().expect("enum 必须是字符串"))
        .collect();
    assert_eq!(allowed, vec!["ok", "failed", "blocked", "canceled"]);

    // 行为：attempt 1..3 旧 failed、attempt 4 为最新 → 3 个候选
    let old = days_ago_stamp(400);
    for attempt in 1..=3 {
        prune_attempt_status(
            runs.path(),
            "t-gate",
            attempt,
            Some("failed"),
            Some(&old),
            true,
        );
    }
    prune_attempt_status(runs.path(), "t-gate", 4, Some("ok"), Some(&old), true);

    let filtered = prune_dry(
        &server,
        json!({ "older_than_days": 365, "status": ["failed"] }),
    );
    assert_eq!(filtered["candidates_total"], 3, "{}", filtered);
    assert_eq!(filtered["candidates"].as_array().unwrap().len(), 3);

    let truncated = prune_dry(
        &server,
        json!({ "older_than_days": 365, "status": ["failed"], "limit": 1 }),
    );
    assert_eq!(
        truncated["candidates_total"], 3,
        "limit 必须在 status 过滤之后生效：{}",
        truncated
    );
    assert_eq!(truncated["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(truncated["truncated"], true);

    let ok_only = prune_dry(&server, json!({ "older_than_days": 365, "status": ["ok"] }));
    assert_eq!(ok_only["candidates_total"], 0, "{}", ok_only);
    assert!(
        ok_only["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["reason"] == "status_mismatch"),
        "非白名单必须进 status_mismatch：{}",
        ok_only
    );
}

#[test]
fn description_gate_behavior_session_and_limit_defaults() {
    let (server, _settings, runs) = server_with_runs();
    fs::create_dir_all(runs.path().join("_session")).expect("创建 _session");
    fs::write(runs.path().join("_session").join("initialize.json"), b"{}")
        .expect("写入 _session 文件");
    for index in 0..25 {
        fs::create_dir_all(runs.path().join(format!("task-{index:03}"))).expect("创建任务目录");
    }

    let listed = tool_payload(&call_tool(&server, "runs.list", json!({})));
    assert_eq!(listed["total_tasks"], 25, "{}", listed);
    assert_eq!(listed["limit"], 20, "runs.list limit 缺省必须是 20");
    let tasks = listed["tasks"].as_array().expect("tasks 必须是数组");
    assert_eq!(tasks.len(), 20, "缺省只回一页 20 条");
    assert!(
        !tasks.iter().any(|task| task["task_id"] == "_session"),
        "_session 不得作为任务：{listed}"
    );

    let dry = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 365 }),
    ));
    assert_eq!(dry["limit"], 100, "runs.prune limit 缺省必须是 100");
    assert!(
        dry["candidates"]
            .as_array()
            .expect("candidates 必须是数组")
            .iter()
            .all(|item| item["task_id"] != "_session"),
        "_session 不得成为候选：{dry}"
    );
}

#[test]
fn description_gate_behavior_connection_counters_real_stdio() {
    use std::io::{BufRead, BufReader, Write};

    let home = tempfile::tempdir().expect("tempdir");
    let mut bridge_command = std::process::Command::new(env!("CARGO_BIN_EXE_aw-mcp-bridge"));
    bridge_command
        .env("APPDATA", home.path())
        // 本用例与其它真实进程用例共用 target/debug/runs，关闭自动清扫避免互相干扰
        .env("AW_INFLIGHT_SWEEP_SECS", "off")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = spawn_bridge(&mut bridge_command);
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    for id in 1..=2 {
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "initialize",
                "params": { "clientInfo": { "name": format!("probe-{id}") } }
            })
        )
        .expect("写入 initialize");
        let mut line = String::new();
        stdout
            .read_line(&mut line)
            .expect("读取 initialize 响应失败");
        let response: Value =
            serde_json::from_str(line.trim()).expect("initialize 响应必须是 JSON");
        assert_eq!(
            response["result"]["protocolVersion"], "2025-06-18",
            "同一连接必须能重复 initialize：{response}"
        );
    }

    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "status", "arguments": {}}})
    )
    .expect("写入 status");
    let mut line = String::new();
    stdout.read_line(&mut line).expect("读取 status 响应失败");
    let response: Value = serde_json::from_str(line.trim()).expect("status 响应必须是 JSON");
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .expect("status 必须是文本载荷"),
    )
    .expect("status 载荷必须是 JSON");

    let connections = &payload["connections"];
    assert_eq!(connections["unit"], "connection", "计数单位必须是连接级");
    assert_eq!(connections["scope"], "process");
    assert_eq!(connections["active"], 1);
    assert_eq!(connections["total"], 1);
    assert_eq!(connections["peak"], 1);
    assert_eq!(
        connections["initialize_total"], 2,
        "两次 initialize 只累加 initialize_total，不新增连接/session：{payload}"
    );

    let _ = child.kill();
    let _ = child.wait();
}

// ---- v1.26：standalone（AW 自行闭环）与 dispatched（PM 派单）的边界 ----

/// standalone 绕过 shell 直接启动程序，测试用的「写文件」程序按平台给 argv
#[cfg(windows)]
fn write_file_argv(path: &str, content: &str) -> Vec<Value> {
    vec![
        json!("cmd.exe"),
        json!("/C"),
        json!(format!("echo {content}>{path}")),
    ]
}

#[cfg(not(windows))]
fn write_file_argv(path: &str, content: &str) -> Vec<Value> {
    vec![
        json!("sh"),
        json!("-c"),
        json!(format!("printf '%s\\n' '{content}' > {path}")),
    ]
}

/// 慢任务：先等待约 2s 再写文件，用于制造真实的并发窗口
#[cfg(windows)]
fn slow_write_file_argv(path: &str, content: &str) -> Vec<Value> {
    vec![
        json!("cmd.exe"),
        json!("/C"),
        json!(format!("ping -n 3 127.0.0.1 >nul & echo {content}>{path}")),
    ]
}

#[cfg(not(windows))]
fn slow_write_file_argv(path: &str, content: &str) -> Vec<Value> {
    vec![
        json!("sh"),
        json!("-c"),
        json!(format!("sleep 2; printf '%s\\n' '{content}' > {path}")),
    ]
}

/// 长任务：远超 1s 超时，用于验证超时清理边界
#[cfg(windows)]
fn slow_long_argv() -> Vec<Value> {
    vec![
        json!("cmd.exe"),
        json!("/C"),
        json!("ping -n 20 127.0.0.1 >nul"),
    ]
}

#[cfg(not(windows))]
fn slow_long_argv() -> Vec<Value> {
    vec![json!("sh"), json!("-c"), json!("sleep 20")]
}

fn temp_git_dir() -> TempDir {
    let dir = TempDir::new().expect("创建临时 git 目录");
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(dir.path())
        .status()
        .expect("启动 git init");
    assert!(status.success(), "git init 失败");
    dir
}

fn read_json_at(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("读取 {} 失败：{e}", path.display()));
    serde_json::from_str(&text).expect("必须是 JSON")
}

fn attempt_dir(runs: &TempDir, task_id: &str, attempt: u64) -> PathBuf {
    runs.path().join(task_id).join(format!("attempt-{attempt}"))
}

fn blocked_reason(payload: &Value) -> String {
    payload["blocked_reason"]
        .as_str()
        .unwrap_or_else(|| panic!("blocked 回执必须给原因：{payload}"))
        .to_string()
}

#[test]
fn standalone_passes_and_persists_verdict_with_observations() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = temp_git_dir();
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-ok",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [
                { "type": "file_exists", "path": "out.txt" },
                { "type": "file_content_match", "path": "out.txt", "contains": "hello" }
            ],
            "max_attempts": 1
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "standalone 执行本身不应是工具错误：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["mode"], "standalone");
    assert_eq!(payload["verdict_conclusion"], "passed");
    assert_eq!(payload["attempts_total"], 1);
    assert_eq!(payload["changed_files"], json!(["out.txt"]));
    assert_eq!(
        payload["verdict_locator"], "runs/sa-ok/attempt-1/verdict.json",
        "判定必须落在 AW 自己的 attempt 目录内：{payload}"
    );

    let verdict_path = attempt_dir(&runs, "sa-ok", 1).join("verdict.json");
    let verdict = read_json_at(&verdict_path);
    assert_eq!(verdict["version"], "standalone-verdict/1");
    assert_eq!(verdict["mode"], "standalone");
    assert_eq!(verdict["verdict"], "passed");
    assert_eq!(verdict["attempt"], 1);

    let criteria = verdict["criteria"].as_array().expect("criteria 必须是数组");
    assert_eq!(criteria.len(), 2, "每条验收标准都要有判定记录：{verdict}");
    assert_eq!(criteria[0]["index"], 1);
    assert_eq!(criteria[0]["spec"]["type"], "file_exists");
    assert_eq!(criteria[0]["passed"], true);
    assert_eq!(criteria[0]["observed"]["exists"], true);
    assert_eq!(criteria[1]["index"], 2);
    assert_eq!(criteria[1]["spec"]["type"], "file_content_match");
    assert_eq!(
        criteria[1]["observed"]["matched"], true,
        "观测值必须可核对，不能只给结论：{verdict}"
    );
    assert!(
        verdict["missing"]
            .as_array()
            .expect("missing 必须是数组")
            .is_empty(),
        "全部可观测时 missing 必须为空：{verdict}"
    );

    let observations = &verdict["observations"];
    assert_eq!(observations["exit_code"], 0);
    assert_eq!(observations["changed_files"], json!(["out.txt"]));
    let stdout_digest = observations["stdout_digest"]
        .as_str()
        .expect("必须记录输出摘要哈希");
    assert!(stdout_digest.starts_with("sha256:"), "{stdout_digest}");
    assert_eq!(verdict["decision"]["action"], "terminate");
    assert_eq!(verdict["decision"]["reason"], "acceptance_met");

    // 回执里的 verdict 工件哈希必须能对上落盘文件
    let artifact = payload["artifacts"]
        .as_array()
        .expect("artifacts 必须是数组")
        .iter()
        .find(|item| item["kind"] == "verdict")
        .expect("回执必须登记 verdict 工件")
        .clone();
    assert_eq!(artifact["locator"], "runs/sa-ok/attempt-1/verdict.json");
    assert_eq!(artifact["hash"], json!(sha256_prefixed(&verdict_path)));
}

#[test]
fn standalone_failure_retries_without_overwriting_records() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = temp_git_dir();
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-fail",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "WRONG"),
            "acceptance": [
                { "type": "file_content_match", "path": "out.txt", "contains": "EXPECTED" }
            ],
            "max_attempts": 2
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "验收不通过是任务结论不是工具错误：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "failed", "{payload}");
    assert_eq!(payload["verdict_conclusion"], "failed");
    assert_eq!(payload["attempt"], 2);
    assert_eq!(
        payload["attempts_total"], 2,
        "重试上限 2，耗尽即终止：{payload}"
    );

    let first = read_json_at(&attempt_dir(&runs, "sa-fail", 1).join("verdict.json"));
    assert_eq!(first["attempt"], 1);
    assert_eq!(first["verdict"], "failed");
    assert_eq!(first["decision"]["action"], "retry");
    assert_eq!(first["decision"]["reason"], "acceptance_not_met");
    assert_eq!(first["criteria"][0]["passed"], false);
    assert_eq!(
        first["criteria"][0]["observed"]["matched"], false,
        "不通过必须给出实际观测：{first}"
    );

    let second = read_json_at(&attempt_dir(&runs, "sa-fail", 2).join("verdict.json"));
    assert_eq!(second["attempt"], 2);
    assert_eq!(second["verdict"], "failed");
    assert_eq!(second["decision"]["action"], "terminate");
    assert_eq!(second["decision"]["reason"], "retry_exhausted");
    assert_eq!(
        second["decision"]["last_failure"], "acceptance_not_met",
        "终止原因必须写清上一次失败类型：{second}"
    );

    // 重试不覆盖上一次：attempt-1 的目录与回执逐字保留
    let first_handback = read_json_at(&attempt_dir(&runs, "sa-fail", 1).join("handback.json"));
    assert_eq!(first_handback["attempt"], 1);
    assert_eq!(first_handback["status"], "failed");
    assert_eq!(
        first_handback["verdict_locator"], "runs/sa-fail/attempt-1/verdict.json",
        "中间尝试的判定必须留在自己的目录：{first_handback}"
    );
    assert!(
        first_handback.get("attempts_total").is_none(),
        "中间尝试不得冒充最终结论：{first_handback}"
    );
    let second_handback = read_json_at(&attempt_dir(&runs, "sa-fail", 2).join("handback.json"));
    assert_eq!(second_handback["attempts_total"], 2);
}

#[test]
fn standalone_undetermined_is_never_reported_as_pass() {
    let (server, _settings, runs) = server_with_runs();
    // 非 git 仓库：git_diff_non_empty 无法观测，必须落 undetermined 而不是「通过」
    let cwd = TempDir::new().expect("创建非 git 目录");
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-undetermined",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "git_diff_non_empty" }],
            "max_attempts": 1
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(
        payload["status"], "blocked",
        "无法判定不能写成 ok：{payload}"
    );
    assert_eq!(payload["verdict_conclusion"], "undetermined");
    assert!(
        blocked_reason(&payload).contains("observation_unavailable"),
        "必须说明缺什么：{}",
        blocked_reason(&payload)
    );

    let verdict = read_json_at(&attempt_dir(&runs, "sa-undetermined", 1).join("verdict.json"));
    assert_eq!(verdict["verdict"], "undetermined");
    assert_eq!(
        verdict["decision"]["action"], "terminate",
        "无法判定不得重试、不得当通过：{verdict}"
    );
    assert_eq!(verdict["decision"]["reason"], "observation_unavailable");
    assert!(
        !verdict["missing"]
            .as_array()
            .expect("missing 必须是数组")
            .is_empty(),
        "undetermined 必须写明缺失的观测：{verdict}"
    );
    assert!(
        !tool_text(&response).contains("\"passed\""),
        "无法判定不得出现 passed：{}",
        tool_text(&response)
    );
}

#[test]
fn standalone_requires_cwd_argv_and_acceptance_before_claiming() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = TempDir::new().expect("创建临时目录");

    let no_cwd = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-no-cwd",
            "mode": "standalone",
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "file_exists", "path": "out.txt" }]
        }),
    );
    assert_eq!(no_cwd["result"]["isError"], false);
    let payload = tool_payload(&no_cwd);
    assert_eq!(payload["status"], "blocked");
    assert_eq!(payload["mode"], "standalone");
    assert!(
        blocked_reason(&payload).starts_with("standalone_cwd_required"),
        "缺 cwd 必须具名拒绝：{}",
        blocked_reason(&payload)
    );
    assert!(
        !runs.path().join("sa-no-cwd").exists(),
        "占号前拒绝不得留下运行目录"
    );

    let no_argv = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-no-argv",
            "mode": "standalone",
            "cwd": cwd.path(),
            "acceptance": [{ "type": "file_exists", "path": "out.txt" }]
        }),
    );
    let payload = tool_payload(&no_argv);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_argv_required"),
        "缺 argv 必须具名拒绝：{}",
        blocked_reason(&payload)
    );

    let no_acceptance = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-no-acceptance",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello")
        }),
    );
    let payload = tool_payload(&no_acceptance);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_acceptance_required"),
        "未声明验收不得默认通过：{}",
        blocked_reason(&payload)
    );

    let empty_acceptance = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-empty-acceptance",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": []
        }),
    );
    let payload = tool_payload(&empty_acceptance);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_acceptance_required"),
        "空验收等价于没声明：{}",
        blocked_reason(&payload)
    );
}

#[test]
fn standalone_rejects_shell_fields_and_out_of_scope_paths() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = TempDir::new().expect("创建临时目录");
    let acceptance = json!([{ "type": "file_exists", "path": "out.txt" }]);

    let with_cmd = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-cmd", "mode": "standalone", "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"), "cmd": "echo hi",
            "acceptance": acceptance
        }),
    );
    let payload = tool_payload(&with_cmd);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_cmd_rejected"),
        "standalone 不得接受 shell 命令：{}",
        blocked_reason(&payload)
    );

    let with_env = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-env", "mode": "standalone", "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"), "env": { "AW_TEST": "1" },
            "acceptance": acceptance
        }),
    );
    let payload = tool_payload(&with_env);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_env_rejected"),
        "standalone 不得注入 env：{}",
        blocked_reason(&payload)
    );

    let escape = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-escape", "mode": "standalone", "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "file_exists", "path": "../escape.txt" }]
        }),
    );
    let payload = tool_payload(&escape);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_path_escape"),
        "验收路径越界必须具名拒绝：{}",
        blocked_reason(&payload)
    );

    let outside_program = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-program", "mode": "standalone", "cwd": cwd.path(),
            "argv": [std::env::current_exe().expect("取测试可执行文件").to_string_lossy(), "x".to_string()],
            "acceptance": acceptance
        }),
    );
    let payload = tool_payload(&outside_program);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("standalone_program_outside_scope"),
        "显式程序路径越界必须具名拒绝：{}",
        blocked_reason(&payload)
    );

    for task_id in ["sa-cmd", "sa-env", "sa-escape", "sa-program"] {
        assert!(
            !runs.path().join(task_id).exists(),
            "拒绝必须发生在占号前：{task_id} 不应留下运行目录"
        );
    }
}

#[test]
fn standalone_replay_returns_final_result_without_new_attempts() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = temp_git_dir();
    let arguments = json!({
        "task_id": "sa-replay",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": write_file_argv("out.txt", "hello"),
        "acceptance": [{ "type": "file_exists", "path": "out.txt" }],
        "max_attempts": 1
    });
    let first = tool_payload(&call_tool(&server, "task.execute", arguments.clone()));
    assert_eq!(first["attempts_total"], 1);
    assert_eq!(first["verdict_conclusion"], "passed");
    assert_eq!(first["replayed"], false, "首次执行必须标称非回放：{first}");

    let replay = call_tool(&server, "task.execute", arguments);
    assert_eq!(replay["result"]["isError"], false, "{}", tool_text(&replay));
    let payload = tool_payload(&replay);
    assert_eq!(
        payload["attempts_total"], 1,
        "重投只能回放最终结果，不得新增尝试：{payload}"
    );
    assert_eq!(
        payload["replayed"], true,
        "重入必须能凭字段判定为回放：{payload}"
    );
    assert_eq!(
        payload["replay_kind"], "existing",
        "已完成记录的重入应为 existing：{payload}"
    );
    assert_eq!(
        payload["finishedAt"], first["finishedAt"],
        "回放必须原样返回既有最终结果，不得重写时间戳：{payload}"
    );
    assert_eq!(payload["verdict_locator"], first["verdict_locator"]);
    assert!(
        !attempt_dir(&runs, "sa-replay", 2).exists(),
        "重投不得写第二个 attempt 目录"
    );

    let conflicting = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-replay",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "exit_code", "expected": 0 }],
            "max_attempts": 1
        }),
    );
    assert_eq!(conflicting["result"]["isError"], true);
    assert!(
        tool_text(&conflicting).contains("task_id_conflict"),
        "参数不一致不得默默回放旧结论：{}",
        tool_text(&conflicting)
    );
}

/// 两个独立 Server 实例共享同一 runs/：同一 task_id 并发，必须一个执行、另一个等待回放
#[test]
fn standalone_duplicate_execute_is_serialized_across_servers() {
    let runs = TempDir::new().expect("创建共享运行目录");
    let settings_a = TempDir::new().expect("创建设置目录 A");
    let settings_b = TempDir::new().expect("创建设置目录 B");
    let server_a = Arc::new(Server::with_runs_dir(
        settings_a.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    let server_b = Arc::new(Server::with_runs_dir(
        settings_b.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    let cwd = Arc::new(temp_git_dir());
    let barrier = Arc::new(Barrier::new(2));
    let arguments = json!({
        "task_id": "sa-concurrent",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": slow_write_file_argv("out.txt", "once"),
        "acceptance": [{ "type": "file_exists", "path": "out.txt" }],
        "max_attempts": 1
    });

    let mut handles = Vec::new();
    for server in [Arc::clone(&server_a), Arc::clone(&server_b)] {
        let barrier = Arc::clone(&barrier);
        let arguments = arguments.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            tool_payload(&call_tool(&server, "task.execute", arguments))
        }));
    }
    let payloads: Vec<Value> = handles
        .into_iter()
        .map(|handle| handle.join().expect("并发调用不应 panic"))
        .collect();

    let executed = payloads
        .iter()
        .filter(|payload| payload["replayed"] == false)
        .count();
    let replayed = payloads
        .iter()
        .filter(|payload| payload["replayed"] == true)
        .count();
    assert_eq!(
        executed, 1,
        "同一 task_id 只能有一个调用方真正执行：{payloads:?}"
    );
    assert_eq!(replayed, 1, "另一个调用方必须明确标记为回放：{payloads:?}");

    let waiter = payloads
        .iter()
        .find(|payload| payload["replayed"] == true)
        .expect("必须有一方回放");
    assert_eq!(waiter["replay_kind"], "waited", "{waiter}");
    assert_eq!(waiter["status"], "ok", "{waiter}");
    assert_eq!(waiter["attempt"], 1, "{waiter}");
    for payload in &payloads {
        assert_eq!(
            payload["finishedAt"], payloads[0]["finishedAt"],
            "回放必须返回胜者的同一份回执：{payloads:?}"
        );
        assert_eq!(
            payload["stdout_digest"], payloads[0]["stdout_digest"],
            "{payloads:?}"
        );
    }

    assert!(attempt_dir(&runs, "sa-concurrent", 1).exists());
    assert!(
        !attempt_dir(&runs, "sa-concurrent", 2).exists(),
        "并发重投不得产生第二个 attempt 目录"
    );
    let verdicts = fs::read_dir(attempt_dir(&runs, "sa-concurrent", 1))
        .expect("读取 attempt-1")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy() == "verdict.json")
        .count();
    assert_eq!(
        verdicts, 1,
        "verdict.json 只能有一份，不能被两个调用方互相覆盖"
    );
}

/// 4 条不同 task_id 并发，其中 1 条超时：其余任务不受影响，各自证据独立
#[test]
fn standalone_concurrent_tasks_are_isolated_from_one_timeout() {
    let runs = TempDir::new().expect("创建共享运行目录");
    let settings: Vec<TempDir> = (0..4)
        .map(|_| TempDir::new().expect("创建设置目录"))
        .collect();
    let servers: Vec<Arc<Server>> = settings
        .iter()
        .map(|dir| {
            Arc::new(Server::with_runs_dir(
                dir.path().join("settings.json"),
                runs.path().to_path_buf(),
            ))
        })
        .collect();
    let cwd = Arc::new(temp_git_dir());
    let barrier = Arc::new(Barrier::new(4));

    let fast_cases: Vec<(String, String)> = (1..=3)
        .map(|index| (format!("sa-iso-{index}"), format!("out-{index}.txt")))
        .collect();
    let mut handles = Vec::new();
    for (index, (task_id, file)) in fast_cases.clone().into_iter().enumerate() {
        let server = Arc::clone(&servers[index]);
        let cwd = Arc::clone(&cwd);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            let payload = tool_payload(&call_tool(
                &server,
                "task.execute",
                json!({
                    "task_id": task_id,
                    "mode": "standalone",
                    "cwd": cwd.path(),
                    "argv": slow_write_file_argv(&file, "ok"),
                    "acceptance": [{ "type": "file_exists", "path": file }],
                    "max_attempts": 1
                }),
            ));
            (task_id, payload)
        }));
    }
    {
        let server = Arc::clone(&servers[1]);
        let cwd = Arc::clone(&cwd);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            let payload = tool_payload(&call_tool(
                &server,
                "task.execute",
                json!({
                    "task_id": "sa-iso-timeout",
                    "mode": "standalone",
                    "cwd": cwd.path(),
                    "argv": slow_long_argv(),
                    "acceptance": [{ "type": "exit_code", "expected": 0 }],
                    "max_attempts": 1,
                    "timeout_secs": 1
                }),
            ));
            ("sa-iso-timeout".to_string(), payload)
        }));
    }

    let results: Vec<(String, Value)> = handles
        .into_iter()
        .map(|handle| handle.join().expect("并发调用不应 panic"))
        .collect();
    for (task_id, payload) in &results {
        if task_id == "sa-iso-timeout" {
            assert_eq!(
                payload["status"], "blocked",
                "超时任务必须明确阻塞：{payload}"
            );
            assert_eq!(payload["verdict_conclusion"], "undetermined", "{payload}");
            assert_eq!(payload["exit_code"], Value::Null, "{payload}");
            assert_eq!(payload["attempt"], 1, "{payload}");
            let reason = blocked_reason(payload);
            assert!(reason.contains("timeout"), "超时必须给出原因：{reason}");
            assert!(
                reason.contains("已清理受控集合"),
                "清理必须可核对：{reason}"
            );
        } else {
            assert_eq!(
                payload["status"], "ok",
                "{task_id} 不得被其它任务的超时清理误伤：{payload}"
            );
            assert_eq!(payload["replayed"], false, "{payload}");
            assert_eq!(payload["attempts_total"], 1, "{payload}");
            let file = fast_cases
                .iter()
                .find(|(id, _)| id == task_id)
                .map(|(_, file)| file.clone())
                .expect("正例文件");
            assert!(
                cwd.path().join(&file).exists(),
                "{task_id} 的产出文件必须落盘"
            );
        }
    }
    for (task_id, _) in &fast_cases {
        assert!(attempt_dir(&runs, task_id, 1).exists());
        assert!(!attempt_dir(&runs, task_id, 2).exists());
    }
    assert!(attempt_dir(&runs, "sa-iso-timeout", 1).exists());
}

/// 同一 task_id、不同参数跨实例并发：后到方必须拿到具名冲突，而不是跑出第二份证据
#[test]
fn standalone_concurrent_conflicting_signature_is_named_conflict() {
    let runs = TempDir::new().expect("创建共享运行目录");
    let settings_a = TempDir::new().expect("创建设置目录 A");
    let settings_b = TempDir::new().expect("创建设置目录 B");
    let server_a = Arc::new(Server::with_runs_dir(
        settings_a.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    let server_b = Arc::new(Server::with_runs_dir(
        settings_b.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    let cwd = Arc::new(temp_git_dir());
    let barrier = Arc::new(Barrier::new(2));

    let arguments_slow = json!({
        "task_id": "sa-conflict",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": slow_write_file_argv("out.txt", "slow"),
        "acceptance": [{ "type": "file_exists", "path": "out.txt" }],
        "max_attempts": 1
    });
    let arguments_fast = json!({
        "task_id": "sa-conflict",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": write_file_argv("out.txt", "fast"),
        "acceptance": [{ "type": "exit_code", "expected": 0 }],
        "max_attempts": 1
    });

    let mut handles = Vec::new();
    for (server, arguments) in [
        (Arc::clone(&server_a), arguments_slow),
        (Arc::clone(&server_b), arguments_fast),
    ] {
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            call_tool(&server, "task.execute", arguments)
        }));
    }
    let responses: Vec<Value> = handles
        .into_iter()
        .map(|handle| handle.join().expect("并发调用不应 panic"))
        .collect();
    let errors = responses
        .iter()
        .filter(|response| response["result"]["isError"] == true)
        .count();
    assert_eq!(errors, 1, "异参并发必须恰好一方被拒绝：{responses:?}");
    let rejected = responses
        .iter()
        .find(|response| response["result"]["isError"] == true)
        .expect("必须有一方被拒绝");
    assert!(
        tool_text(rejected).contains("task_id_conflict"),
        "异参复用必须具名拒绝：{}",
        tool_text(rejected)
    );
    assert!(
        !attempt_dir(&runs, "sa-conflict", 2).exists(),
        "异参并发不得产生第二份证据"
    );
}

#[test]
fn standalone_and_dispatched_cannot_share_one_task_id() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = temp_git_dir();

    // dispatched 先占号，再以 standalone 复用同一 task_id 必须具名拒绝
    let dispatched = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "sa-mixed", "cmd": "echo hi" }),
    );
    assert_eq!(
        dispatched["result"]["isError"],
        false,
        "{}",
        tool_text(&dispatched)
    );
    let standalone = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-mixed",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "file_exists", "path": "out.txt" }]
        }),
    );
    assert_eq!(
        standalone["result"]["isError"],
        true,
        "同一目录不得既被 PM 当权威又被 AW 自判：{}",
        tool_text(&standalone)
    );
    assert!(
        tool_text(&standalone).contains("mode_conflict"),
        "{}",
        tool_text(&standalone)
    );
    assert!(
        !attempt_dir(&runs, "sa-mixed", 2).exists(),
        "跨模式复用不得写入第二个目录"
    );

    // 反向：standalone 先占号，dispatched 复用也必须拒绝
    let standalone_first = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "sa-mixed-rev",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": write_file_argv("out.txt", "hello"),
            "acceptance": [{ "type": "file_exists", "path": "out.txt" }],
            "max_attempts": 1
        }),
    );
    assert_eq!(tool_payload(&standalone_first)["status"], "ok");
    let reverse = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "sa-mixed-rev", "cmd": "echo hi" }),
    );
    assert_eq!(
        reverse["result"]["isError"],
        true,
        "{}",
        tool_text(&reverse)
    );
    assert!(
        tool_text(&reverse).contains("mode_conflict"),
        "{}",
        tool_text(&reverse)
    );
}

#[test]
fn dispatched_handback_stays_free_of_verdict_fields() {
    let (server, _settings, runs) = server_with_runs();
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "disp-regress", "cmd": "echo ok" }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok");

    for field in [
        "mode",
        "verdict_conclusion",
        "verdict_locator",
        "attempts_total",
        "max_attempts",
        "acceptance",
    ] {
        assert!(
            payload.get(field).is_none(),
            "dispatched 回执不得新增判定字段 {field}：{payload}"
        );
    }
    let dir = attempt_dir(&runs, "disp-regress", 1);
    assert!(dir.join("handback.json").exists());
    assert!(
        !dir.join("verdict.json").exists(),
        "dispatched 不得写 verdict.json"
    );
    assert!(!runs
        .path()
        .join("disp-regress")
        .join("standalone-result.json")
        .exists());
    let dispatch = read_json_at(&dir.join("dispatch.json"));
    assert!(
        dispatch.get("mode").is_none(),
        "dispatched 派单签名不得写 mode：{dispatch}"
    );
    assert!(
        !tool_text(&response).contains("verified"),
        "dispatched 不得自称 verified：{}",
        tool_text(&response)
    );

    // standalone-only 字段出现在 dispatched 调用里，必须显式拒绝而不是被静默忽略
    let rejected = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "disp-argv",
            "argv": ["echo"],
            "acceptance": [{ "type": "file_exists", "path": "out.txt" }]
        }),
    );
    let payload = tool_payload(&rejected);
    assert_eq!(payload["status"], "blocked");
    assert!(
        blocked_reason(&payload).starts_with("dispatched_input_rejected"),
        "dispatched 不得接受 standalone 判据：{}",
        blocked_reason(&payload)
    );
    assert!(!runs.path().join("disp-argv").exists());
}

// ---- runs.prune：按条目精确排除（v1.27）----

fn snapshot_dir_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut items: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in fs::read_dir(dir).expect("读目录").flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        items.push((name, fs::read(entry.path()).expect("读文件")));
    }
    items.sort();
    items
}

#[test]
fn prune_exclude_schema_declares_capability_and_constraints() {
    let (server, _settings, _runs) = server_with_runs();
    let schema = prune_schema(&server);
    let exclude = &schema["properties"]["exclude"];
    assert_eq!(exclude["type"], "array", "exclude 必须是数组：{schema}");
    assert_eq!(exclude["minItems"], 1, "exclude 至少 1 条");
    assert_eq!(exclude["maxItems"], 100, "exclude 至多 100 条");
    assert_eq!(exclude["uniqueItems"], true, "exclude 必须声明去重约束");
    assert_eq!(exclude["items"]["type"], "object");
    assert_eq!(
        exclude["items"]["required"],
        json!(["task_id"]),
        "每条排除项必须带 task_id"
    );
    assert_eq!(exclude["items"]["additionalProperties"], false);
    assert_eq!(exclude["items"]["properties"]["task_id"]["type"], "string");
    assert_eq!(exclude["items"]["properties"]["task_id"]["maxLength"], 64);
    assert_eq!(exclude["items"]["properties"]["attempt"]["type"], "integer");
    assert_eq!(exclude["items"]["properties"]["attempt"]["minimum"], 1);

    let description = exclude["description"].as_str().expect("exclude 必须有描述");
    for phrase in [
        "按条目精确排除",
        "task_id",
        "attempt",
        "不得重复",
        "命名校验",
        "invalid_exclusion",
        "not_in_candidates",
        "枚举入口生效",
    ] {
        assert!(
            description.contains(phrase),
            "exclude 描述缺少「{phrase}」：{description}"
        );
    }
    let tool_description = prune_tool_description(&server);
    for phrase in ["按条目精确排除", "计数不变式", "exclude"] {
        assert!(
            tool_description.contains(phrase),
            "runs.prune 描述缺少「{phrase}」：{tool_description}"
        );
    }
}

#[test]
fn prune_exclude_one_candidate_diverts_it_and_keeps_bytes() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    for task in ["t-a", "t-b", "t-c"] {
        prune_attempt(runs.path(), task, 1, true, Some(&old), true);
        prune_attempt(runs.path(), task, 2, true, Some(&old), true);
    }
    let kept = attempt_dir(&runs, "t-b", 1);
    let before = snapshot_dir_bytes(&kept);
    let exclude = json!([{ "task_id": "t-b", "attempt": 1 }]);

    let dry = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "exclude": exclude }),
    ));
    assert_eq!(dry["candidates_total"], 3, "候选总数含被排除项：{dry}");
    assert_eq!(dry["excluded_total"], 1);
    assert_eq!(dry["not_in_candidates_total"], 0);
    assert_eq!(dry["exclude_requests_total"], 1);
    assert_eq!(dry["batch_candidates_total"], 3);
    assert_eq!(dry["not_processed_total"], 0);
    let candidates = dry["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 2);
    assert!(
        candidates.iter().all(|item| item["task_id"] != "t-b"),
        "被排除项不得进入候选：{dry}"
    );
    assert_eq!(dry["excluded"][0]["task_id"], "t-b");
    assert_eq!(dry["excluded"][0]["attempt"], 1);
    assert_eq!(dry["excluded"][0]["reason"], "excluded_by_request");
    assert_eq!(dry["excluded"][0]["path"], "runs/t-b/attempt-1");

    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 30,
            "exclude": exclude,
            "dry_run": false,
            "confirm": true
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "带 exclude 的真删应成功：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["deleted_total"], 2, "只删 2 条：{payload}");
    assert_eq!(payload["excluded_total"], 1);
    assert_eq!(payload["candidates_total"], 3);
    assert_eq!(payload["batch_candidates_total"], 3);
    assert_eq!(payload["not_processed_total"], 0);
    let deleted_ids: Vec<&str> = payload["deleted"]
        .as_array()
        .expect("deleted")
        .iter()
        .filter_map(|item| item["task_id"].as_str())
        .collect();
    assert_eq!(deleted_ids, vec!["t-a", "t-c"]);
    assert_eq!(
        payload["deleted_total"].as_u64().unwrap_or(0)
            + payload["failed_total"].as_u64().unwrap_or(0)
            + payload["trashed_total"].as_u64().unwrap_or(0)
            + payload["excluded_total"].as_u64().unwrap_or(0),
        payload["batch_candidates_total"]
            .as_u64()
            .unwrap_or(u64::MAX),
        "计数不变式必须成立：删除+失败+暂存+排除 == 本批候选总数：{payload}"
    );
    assert!(kept.is_dir(), "被排除条目不得被删除：{payload}");
    assert_eq!(
        snapshot_dir_bytes(&kept),
        before,
        "被排除条目必须逐字节未变"
    );
    let audit = fs::read_to_string(runs.path().join("prune.jsonl")).expect("读审计");
    let result_line: Value = serde_json::from_str(audit.lines().last().expect("必须有 result 行"))
        .expect("审计行必须是 JSON");
    assert_eq!(
        result_line["excluded_total"], 1,
        "审计 result 行必须记录排除数：{result_line}"
    );
    assert_eq!(
        result_line["excluded"][0]["reason"], "excluded_by_request",
        "审计 result 行必须逐条记录排除原因：{result_line}"
    );
}

#[test]
fn prune_exclude_all_candidates_yields_zero_deletes() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    for task in ["t-a", "t-b", "t-c"] {
        prune_attempt(runs.path(), task, 1, true, Some(&old), true);
        prune_attempt(runs.path(), task, 2, true, Some(&old), true);
    }
    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 30,
            "exclude": [
                { "task_id": "t-a", "attempt": 1 },
                { "task_id": "t-b", "attempt": 1 },
                { "task_id": "t-c", "attempt": 1 }
            ],
            "dry_run": false,
            "confirm": true
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "排除全部候选应成功返回零删除：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["deleted_total"], 0);
    assert_eq!(payload["failed_total"], 0);
    assert_eq!(payload["trashed_total"], 0);
    assert_eq!(payload["excluded_total"], 3);
    assert_eq!(payload["candidates_total"], 3, "排除数必须等于候选数");
    assert_eq!(payload["batch_candidates_total"], 3);
    assert_eq!(payload["not_processed_total"], 0);
    assert_eq!(payload["truncated"], false);
    assert!(
        payload["deleted"].as_array().expect("deleted").is_empty(),
        "零删除：{payload}"
    );
    for task in ["t-a", "t-b", "t-c"] {
        assert!(
            runs.path().join(task).join("attempt-1").is_dir(),
            "{task} 的旧 attempt 不得被删"
        );
    }
    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "没有删除发生就不得写审计"
    );
}

#[test]
fn prune_exclude_reports_not_in_candidates_and_keeps_processing() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-p", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-p", 2, true, Some(&old), true);
    prune_attempt(runs.path(), "t-q", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-q", 2, true, Some(&old), true);

    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "older_than_days": 30,
            "exclude": [
                { "task_id": "t-missing" },
                { "task_id": "t-p", "attempt": 2 },
                { "task_id": "t-p", "attempt": 9 }
            ],
            "dry_run": false,
            "confirm": true
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "not_in_candidates 不是错误：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["candidates_total"], 2);
    assert_eq!(payload["excluded_total"], 0, "没有候选被请求排除");
    assert_eq!(payload["not_in_candidates_total"], 3);
    assert_eq!(payload["exclude_requests_total"], 3);
    assert_eq!(payload["batch_candidates_total"], 2);
    assert_eq!(payload["not_processed_total"], 0);
    let excluded = payload["excluded"].as_array().expect("excluded");
    assert_eq!(excluded.len(), 3, "三条请求都必须逐条回报：{payload}");
    for entry in excluded {
        assert_eq!(
            entry["reason"], "not_in_candidates",
            "不得静默忽略：{entry}"
        );
    }
    assert_eq!(excluded[0]["task_id"], "t-missing");
    assert!(excluded[0]["attempt"].is_null());
    assert!(
        excluded[0]["path"].is_null(),
        "不在候选集的条目没有真实路径，不得编造：{}",
        excluded[0]
    );
    assert_eq!(excluded[1]["task_id"], "t-p");
    assert_eq!(excluded[1]["attempt"], 2, "最新 attempt 本就不在候选集");
    assert_eq!(excluded[2]["attempt"], 9, "不存在的 attempt 也不在候选集");
    assert_eq!(
        payload["deleted_total"], 2,
        "不在候选集的排除请求不得干扰正常删除"
    );
    assert!(!runs.path().join("t-p").join("attempt-1").exists());
    assert!(!runs.path().join("t-q").join("attempt-1").exists());
    assert!(
        runs.path().join("t-p").join("attempt-2").is_dir(),
        "最新 attempt 始终保留"
    );
}

#[test]
fn prune_exclude_rejects_unsafe_or_ambiguous_entries_without_writes() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    prune_attempt(runs.path(), "t-a", 1, true, Some(&old), true);
    prune_attempt(runs.path(), "t-a", 2, true, Some(&old), true);
    let long_id = "a".repeat(65);

    let bad_cases: Vec<(&str, Value)> = vec![
        ("路径分隔符", json!([{ "task_id": "../escape" }])),
        ("上跳", json!([{ "task_id": ".." }])),
        ("Windows 绝对路径", json!([{ "task_id": "C:\\Windows" }])),
        ("POSIX 绝对路径", json!([{ "task_id": "/etc/passwd" }])),
        ("通配符", json!([{ "task_id": "t-*" }])),
        ("预留目录", json!([{ "task_id": "_session" }])),
        ("内部目录", json!([{ "task_id": ".prune-trash" }])),
        ("空格", json!([{ "task_id": "t a" }])),
        ("空字符串", json!([{ "task_id": "" }])),
        ("超长", json!([{ "task_id": long_id }])),
        ("空数组", json!([])),
        ("null", json!(null)),
        ("非对象条目", json!(["t-a"])),
        ("缺 task_id", json!([{ "attempt": 1 }])),
        ("attempt=0", json!([{ "task_id": "t-a", "attempt": 0 }])),
        (
            "attempt 为字符串",
            json!([{ "task_id": "t-a", "attempt": "1" }]),
        ),
        (
            "重复整任务",
            json!([{ "task_id": "t-a" }, { "task_id": "t-a" }]),
        ),
        (
            "重复 attempt",
            json!([
                { "task_id": "t-a", "attempt": 1 },
                { "task_id": "t-a", "attempt": 1 }
            ]),
        ),
        (
            "整任务与单次混用",
            json!([{ "task_id": "t-a" }, { "task_id": "t-a", "attempt": 1 }]),
        ),
    ];
    for (label, exclude) in bad_cases {
        let error = prune_error(
            &server,
            json!({ "older_than_days": 30, "exclude": exclude }),
        );
        assert!(
            error.contains("invalid_exclusion"),
            "{label} 必须按 invalid_exclusion 拒绝：{error}"
        );
    }
    let traversal_error = prune_error(
        &server,
        json!({ "older_than_days": 30, "exclude": [{ "task_id": "../escape" }] }),
    );
    assert!(
        traversal_error.contains("命名校验"),
        "越界拒绝必须写明命名校验规则：{traversal_error}"
    );
    // 真删参数组合下同样在写入前拒绝
    let real_error = prune_error(
        &server,
        json!({
            "older_than_days": 30,
            "exclude": [{ "task_id": "../escape" }],
            "dry_run": false,
            "confirm": true
        }),
    );
    assert!(real_error.contains("invalid_exclusion"), "{real_error}");
    assert!(
        runs.path().join("t-a").join("attempt-1").is_dir(),
        "非法排除参数必须零删除"
    );
    assert!(
        !runs.path().join("prune.jsonl").exists(),
        "非法排除参数必须零审计"
    );
    assert!(
        !runs.path().join(".prune-trash").exists(),
        "非法排除参数不得创建暂存目录"
    );
}

#[test]
fn prune_without_exclude_keeps_legacy_behavior_and_invariant() {
    let (server, _settings, runs) = server_with_runs();
    let old = days_ago_stamp(40);
    for task in ["t-a", "t-b", "t-c"] {
        prune_attempt(runs.path(), task, 1, true, Some(&old), true);
        prune_attempt(runs.path(), task, 2, true, Some(&old), true);
    }
    let response = call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "不带 exclude 的旧调用必须照常成功：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["deleted_total"], 3, "旧行为完全不变：{payload}");
    assert_eq!(payload["excluded_total"], 0);
    assert_eq!(payload["not_in_candidates_total"], 0);
    assert_eq!(payload["exclude_requests_total"], 0);
    assert!(
        payload["excluded"].as_array().expect("excluded").is_empty(),
        "没传 exclude 时 excluded 必须为空数组：{payload}"
    );
    assert_eq!(payload["exclude_filter"], Value::Null);
    assert_eq!(payload["candidates_total"], 3);
    assert_eq!(payload["batch_candidates_total"], 3);
    assert_eq!(payload["not_processed_total"], 0);
    assert_eq!(
        payload["deleted_total"].as_u64().unwrap_or(0)
            + payload["failed_total"].as_u64().unwrap_or(0)
            + payload["trashed_total"].as_u64().unwrap_or(0)
            + payload["excluded_total"].as_u64().unwrap_or(0),
        payload["candidates_total"].as_u64().unwrap_or(u64::MAX),
        "无排除时 删除+失败+暂存+排除 == 候选总数：{payload}"
    );
}

// ---- v1.28：standalone 下 argv 参数携带路径的作用域校验 ----

/// standalone 直接启动程序；测试把待测参数追加在固定前缀之后
#[cfg(windows)]
fn argv_with_extra(extra: &[String]) -> Vec<Value> {
    let mut argv = vec![json!("cmd.exe"), json!("/C"), json!("echo ok")];
    argv.extend(extra.iter().map(|item| json!(item)));
    argv
}

#[cfg(not(windows))]
fn argv_with_extra(extra: &[String]) -> Vec<Value> {
    let mut argv = vec![json!("sh"), json!("-c"), json!("echo ok")];
    argv.extend(extra.iter().map(|item| json!(item)));
    argv
}

fn canonical_cwd(dir: &Path) -> String {
    fs::canonicalize(dir)
        .expect("cwd 必须可规范化")
        .to_string_lossy()
        .to_string()
}

#[test]
fn standalone_argv_scope_accepts_normal_arguments() {
    let (server, _settings, _runs) = server_with_runs();
    let cwd = TempDir::new().expect("创建临时目录");
    let inside_absolute = fs::canonicalize(cwd.path())
        .expect("规范化 cwd")
        .join("out.txt")
        .to_string_lossy()
        .to_string();

    let cases: Vec<(&str, Vec<String>)> = vec![
        ("argv-name", vec!["--name=foo".to_string()]),
        ("argv-count", vec!["--count=3".to_string()]),
        ("argv-spaced", vec!["--out=my dir\\out.txt".to_string()]),
        (
            "argv-separate",
            vec!["--out".to_string(), "out.txt".to_string()],
        ),
        (
            "argv-url",
            vec!["--url=https://example.com/a/b".to_string()],
        ),
        ("argv-regex", vec!["--pattern=^hello$".to_string()]),
        ("argv-abs-inside", vec![inside_absolute]),
        ("argv-colon", vec!["out:out.txt".to_string()]),
    ];

    for (task_id, extra) in cases {
        let response = call_tool(
            &server,
            "task.execute",
            json!({
                "task_id": task_id,
                "mode": "standalone",
                "cwd": cwd.path(),
                "argv": argv_with_extra(&extra),
                "acceptance": [{ "type": "exit_code", "expected": 0 }],
                "max_attempts": 1
            }),
        );
        assert_eq!(
            response["result"]["isError"],
            false,
            "{task_id} 正常参数不应引起工具错误：{}",
            tool_text(&response)
        );
        let payload = tool_payload(&response);
        assert_eq!(
            payload["status"], "ok",
            "{task_id} 正常参数不得被误拒：{payload}"
        );
        assert_eq!(payload["attempts_total"], 1, "{task_id}: {payload}");
        let scan = &payload["argv_path_scan"];
        assert_eq!(
            scan["scope"],
            json!(canonical_cwd(cwd.path())),
            "{task_id} 扫描必须写明作用域：{scan}"
        );
        let checked = scan["checked"]
            .as_array()
            .unwrap_or_else(|| panic!("{task_id} checked 必须是数组：{scan}"));
        let unknown = scan["unknown"]
            .as_array()
            .unwrap_or_else(|| panic!("{task_id} unknown 必须是数组：{scan}"));
        match task_id {
            "argv-spaced" => {
                let entry = checked
                    .iter()
                    .find(|item| item["raw"] == json!("--out=my dir\\out.txt"))
                    .unwrap_or_else(|| panic!("带空格的界内路径必须按路径校验：{scan}"));
                assert_eq!(entry["class"], "relative_path", "{entry}");
                assert_eq!(entry["via"], "flag_equals", "{entry}");
                assert_eq!(entry["decision"], "in_scope", "{entry}");
            }
            "argv-name" | "argv-count" | "argv-colon" => assert!(
                checked.is_empty(),
                "{task_id} 普通参数不得被当成路径：{scan}"
            ),
            "argv-url" | "argv-regex" => {
                assert!(checked.is_empty(), "{task_id} 不是路径：{scan}");
                assert!(unknown.is_empty(), "{task_id} 不应记为未知：{scan}");
            }
            "argv-abs-inside" => {
                let entry = checked
                    .iter()
                    .find(|item| item["class"] == "absolute_path")
                    .unwrap_or_else(|| panic!("界内绝对路径必须按路径校验：{scan}"));
                assert_eq!(entry["decision"], "in_scope", "{entry}");
            }
            _ => {}
        }
    }
}

#[test]
fn standalone_argv_scope_rejects_escapes_before_execution() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = TempDir::new().expect("创建临时目录");
    let outside = TempDir::new().expect("创建界外目录");
    let outside_absolute = fs::canonicalize(outside.path())
        .expect("规范化界外目录")
        .join("escape.txt")
        .to_string_lossy()
        .to_string();

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(outside.path(), cwd.path().join("link"))
        .expect("创建指向界外的符号链接（需符号链接权限）");
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(outside.path(), cwd.path().join("link"))
        .expect("创建指向界外的符号链接");

    let cases: Vec<(&str, Vec<String>)> = vec![
        ("argv-dotdot", vec!["--out=../escape.txt".to_string()]),
        ("argv-abs-out", vec![outside_absolute]),
        ("argv-flag-escape", vec!["--flag=../../x".to_string()]),
        ("argv-symlink", vec!["--out=link/target.txt".to_string()]),
        ("argv-unc", vec!["\\\\server\\share\\evil".to_string()]),
        ("argv-encoded", vec!["..%2f..%2fescape.txt".to_string()]),
        ("argv-quoted", vec!["\"../escape.txt\"".to_string()]),
        (
            "argv-backslash",
            vec![format!(
                "--out=..{sep}..{sep}escape.txt",
                sep = std::path::MAIN_SEPARATOR
            )],
        ),
    ];

    for (task_id, extra) in cases {
        let response = call_tool(
            &server,
            "task.execute",
            json!({
                "task_id": task_id,
                "mode": "standalone",
                "cwd": cwd.path(),
                "argv": argv_with_extra(&extra),
                "acceptance": [{ "type": "exit_code", "expected": 0 }],
                "max_attempts": 1
            }),
        );
        assert_eq!(
            response["result"]["isError"],
            false,
            "{task_id} 越界拒绝是预检结论不是工具错误：{}",
            tool_text(&response)
        );
        let payload = tool_payload(&response);
        assert_eq!(
            payload["status"], "blocked",
            "{task_id} 参数携带的越界路径必须在执行前拒绝：{payload}"
        );
        let reason = blocked_reason(&payload);
        assert!(
            reason.starts_with("standalone_argv_path_escape"),
            "{task_id} 必须给出明确原因码：{reason}"
        );
        assert!(
            reason.contains(extra.last().expect("至少一个参数")),
            "{task_id} 原因必须写明具体参数：{reason}"
        );
        assert!(
            !runs.path().join(task_id).exists(),
            "{task_id} 拒绝必须发生在占号前，不得留下运行目录"
        );
    }
}

#[test]
fn standalone_argv_scan_records_unknown_forms_instead_of_silently_passing() {
    let (server, _settings, _runs) = server_with_runs();
    let cwd = TempDir::new().expect("创建临时目录");
    let extra = vec!["%USERPROFILE%\\outside\\x".to_string()];
    let response = call_tool(
        &server,
        "task.execute",
        json!({
            "task_id": "argv-unknown",
            "mode": "standalone",
            "cwd": cwd.path(),
            "argv": argv_with_extra(&extra),
            "acceptance": [{ "type": "exit_code", "expected": 0 }],
            "max_attempts": 1
        }),
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["status"], "ok", "{payload}");
    let unknown = payload["argv_path_scan"]["unknown"]
        .as_array()
        .unwrap_or_else(|| panic!("unknown 必须是数组：{payload}"));
    assert_eq!(unknown.len(), 1, "无法判定的形态必须逐条记录：{payload}");
    assert_eq!(unknown[0]["decision"], "allowed");
    assert_eq!(
        unknown[0]["reason_code"], "standalone_argv_path_unknown",
        "未知形态必须有原因码：{}",
        unknown[0]
    );
    assert!(
        unknown[0]["raw"]
            .as_str()
            .unwrap_or_default()
            .contains("%USERPROFILE%"),
        "必须记录原样参数：{}",
        unknown[0]
    );
}

// ---- v1.30：prune 与执行并发防护 ----

fn inflight_marker(runs: &Path, task_id: &str) -> PathBuf {
    runs.join(".inflight").join(format!("{task_id}.json"))
}

/// 执行期间必须存在跨进程可见的活跃标记；结束后必须删除；标记目录不得被当成任务
#[test]
fn standalone_inflight_marker_is_visible_then_removed() {
    let (server, _settings, runs) = server_with_runs();
    let server = Arc::new(server);
    let cwd = Arc::new(temp_git_dir());
    let arguments = json!({
        "task_id": "sa-inflight",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": slow_write_file_argv("marker.txt", "ok"),
        "acceptance": [{ "type": "file_exists", "path": "marker.txt" }],
        "max_attempts": 1
    });
    let worker = Arc::clone(&server);
    let handle =
        std::thread::spawn(move || tool_payload(&call_tool(&worker, "task.execute", arguments)));

    let marker = inflight_marker(runs.path(), "sa-inflight");
    let mut seen = false;
    for _ in 0..200 {
        if marker.exists() {
            seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen, "执行期间必须有 runs/.inflight/sa-inflight.json");
    let record = read_json_at(&marker);
    assert_eq!(record["task_id"], "sa-inflight", "{record}");
    assert!(record["pid"].as_u64().is_some(), "标记必须带 pid：{record}");
    assert!(
        record["startedAt"].as_str().is_some() && record["deadlineAt"].as_str().is_some(),
        "标记必须带时间窗：{record}"
    );

    let listed = runs_list_payload(&server, json!({}));
    assert!(
        listed["tasks"]
            .as_array()
            .unwrap_or_else(|| panic!("tasks：{listed}"))
            .iter()
            .all(|task| task["task_id"] != ".inflight"),
        ".inflight 不得被当作任务：{listed}"
    );
    let dry = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30 }),
    ));
    assert!(
        !dry["candidates"]
            .as_array()
            .unwrap_or_else(|| panic!("candidates：{dry}"))
            .iter()
            .any(|candidate| candidate["task_id"] == ".inflight"),
        ".inflight 不得进入 prune 候选：{dry}"
    );

    let payload = handle.join().expect("执行线程不应 panic");
    assert_eq!(payload["status"], "ok", "{payload}");
    assert!(!marker.exists(), "任务结束后必须删除活跃标记：{payload}");
}

/// prune 看到活跃标记必须跳过该任务，即使该 task 有超龄的非最新 attempt
#[test]
fn prune_skips_task_with_live_inflight_marker() {
    let (server, _settings, runs) = server_with_runs();
    prune_attempt(
        runs.path(),
        "sa-prune-inflight",
        1,
        true,
        Some(&days_ago_stamp(30)),
        true,
    );
    prune_attempt(runs.path(), "sa-prune-inflight", 2, false, None, true);
    let marker = inflight_marker(runs.path(), "sa-prune-inflight");
    fs::create_dir_all(marker.parent().expect("标记目录")).expect("创建标记目录");
    fs::write(
        &marker,
        json!({
            "version": "inflight-marker/1",
            "task_id": "sa-prune-inflight",
            "pid": std::process::id(),
            "mode": "standalone",
            "startedAt": days_ago_stamp(0),
            "deadlineAt": days_ahead_stamp(1)
        })
        .to_string(),
    )
    .expect("写入标记");

    let dry = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30 }),
    ));
    assert!(
        dry["skipped"]
            .as_array()
            .unwrap_or_else(|| panic!("dry-run 必须给出 skipped：{dry}"))
            .iter()
            .any(|entry| entry["task_id"] == "sa-prune-inflight"
                && entry["reason"] == "task_inflight"
                && entry["inflight"]["stale"] == false),
        "必须具名记录 task_inflight 跳过：{dry}"
    );
    let payload = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    ));
    assert_eq!(payload["deleted_total"], 0, "活跃任务不得被删：{payload}");
    assert_eq!(
        payload["candidates_total"], 0,
        "活跃任务不得进入候选：{payload}"
    );
    assert!(
        attempt_dir(&runs, "sa-prune-inflight", 1).exists(),
        "active 标记存在时 attempt-1 必须原样保留"
    );
}

/// 活跃 attempt 目录被外部删除后，必须给出 attempt_dir_lost，不得静默 ok
#[test]
fn standalone_active_attempt_deleted_is_reported_not_silent_ok() {
    let (server, _settings, runs) = server_with_runs();
    let server = Arc::new(server);
    let cwd = Arc::new(temp_git_dir());
    let arguments = json!({
        "task_id": "sa-vanish",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": slow_write_file_argv("vanish.txt", "ok"),
        "acceptance": [{ "type": "exit_code", "expected": 0 }],
        "max_attempts": 1
    });
    let handle = std::thread::spawn(move || call_tool(&server, "task.execute", arguments));

    let dir = attempt_dir(&runs, "sa-vanish", 1);
    for _ in 0..200 {
        if dir.join("stdout.txt").exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(dir.exists(), "执行线程必须先创建 attempt-1");
    std::thread::sleep(Duration::from_millis(300));
    let mut removed = false;
    for _ in 0..20 {
        if fs::remove_dir_all(&dir).is_ok() {
            removed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(removed, "测试夹具必须能在运行中删除 attempt 目录");

    let response = handle.join().expect("执行线程不应 panic");
    assert_eq!(
        response["result"]["isError"],
        true,
        "目录消失后不得报告成功：{}",
        tool_text(&response)
    );
    assert!(
        tool_text(&response).contains("attempt_dir_lost"),
        "必须具名报告 attempt_dir_lost：{}",
        tool_text(&response)
    );
}

/// 全部记录被外部清理后重跑同一 task_id：必须能看出这是重跑，而不是既有结果回放
#[test]
fn standalone_rerun_after_records_removed_is_marked() {
    let (server, _settings, runs) = server_with_runs();
    let cwd = temp_git_dir();
    let arguments = json!({
        "task_id": "sa-pruned-rerun",
        "mode": "standalone",
        "cwd": cwd.path(),
        "argv": write_file_argv("rerun.txt", "hello"),
        "acceptance": [{ "type": "file_exists", "path": "rerun.txt" }],
        "max_attempts": 1
    });
    let first = tool_payload(&call_tool(&server, "task.execute", arguments.clone()));
    assert_eq!(
        first["replay_basis"], "fresh",
        "首次执行必须是 fresh：{first}"
    );

    fs::remove_dir_all(runs.path().join("sa-pruned-rerun")).expect("删除全部记录");
    let rerun = tool_payload(&call_tool(&server, "task.execute", arguments.clone()));
    assert_eq!(
        rerun["replay_basis"], "rerun_after_records_missing",
        "记录消失后的新执行必须可被识别：{rerun}"
    );
    assert_eq!(rerun["prior_attempts"], 1, "{rerun}");
    assert_eq!(rerun["attempt"], 1, "重跑从 attempt-1 开始：{rerun}");

    let replay = tool_payload(&call_tool(&server, "task.execute", arguments));
    assert_eq!(replay["replayed"], true, "{replay}");
    assert_eq!(
        replay["replay_basis"], "rerun_after_records_missing",
        "既有结果回放必须保留重跑标注：{replay}"
    );
}

/// v1.31：默认宽松不变；只有显式 `_meta.strict_arguments=true` 才按 schema 拒绝未声明参数
#[test]
fn strict_arguments_meta_rejects_unknown_with_readable_code() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.list",
        json!({
            "offset": 0,
            "offest": 5,
            "_meta": { "strict_arguments": true, "source": "strict-test" }
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "严格模式必须拒绝未声明参数：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    let error = payload["error"]
        .as_str()
        .unwrap_or_else(|| panic!("必须回传具名错误：{payload}"));
    assert!(
        error.starts_with("invalid_arguments"),
        "错误码必须可判定：{error}"
    );
    assert!(error.contains("offest"), "错误必须点名参数：{error}");
    assert!(
        error.contains("offset"),
        "错误必须给出已声明参数，调用方无需读源码：{error}"
    );
    assert_eq!(payload["code"], "invalid_arguments", "{payload}");
    assert!(
        payload["unknown_arguments"]
            .as_array()
            .expect("unknown_arguments 必须是数组")
            .contains(&json!("offest")),
        "必须回显违规参数名：{payload}"
    );
}

#[test]
fn strict_arguments_meta_rejects_alias_with_suggestion() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.prune",
        json!({
            "dryRun": true,
            "older_than_days": 365,
            "_meta": { "strict_arguments": true }
        }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "严格模式下别名也必须拒绝（别名不会自动改名）：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["code"], "invalid_arguments", "{payload}");
    assert!(
        payload["alias_suggestions"]
            .as_array()
            .expect("alias_suggestions 必须是数组")
            .contains(&json!("dryRun -> dry_run")),
        "必须给出可执行的更正建议：{payload}"
    );
}

#[test]
fn strict_arguments_meta_allows_declared_and_omitted_optionals() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(
        &server,
        "runs.list",
        json!({ "_meta": { "strict_arguments": true } }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "合法的可选参数缺省不得被误拒：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert!(payload.get("error").is_none(), "{payload}");
    assert!(payload["total_tasks"].is_number(), "{payload}");
    assert!(
        payload["arguments_diagnostics"]["unknown_arguments"]
            .as_array()
            .expect("unknown_arguments 必须是数组")
            .iter()
            .all(|entry| entry["kind"] == "client_metadata"),
        "严格模式只放行 _meta 这类客户端元数据：{payload}"
    );
}

// ---- v1.32：机器可读的能力/限制声明与 inflight 残留清理 ----

/// 真实不存在的 pid：POSIX `kill(pid,0)` 与 Windows `OpenProcess` 都必须判为已消失
const DEAD_PID: u64 = 999_999_999;

fn write_inflight_marker(
    runs: &Path,
    task_id: &str,
    pid: u64,
    started_at: &str,
    deadline_at: &str,
) {
    let dir = runs.join(".inflight");
    fs::create_dir_all(&dir).expect("创建 .inflight 目录");
    fs::write(
        dir.join(format!("{task_id}.json")),
        json!({
            "version": "inflight-marker/1",
            "task_id": task_id,
            "pid": pid,
            "mode": "dispatched",
            "state": "active",
            "startedAt": started_at,
            "deadlineAt": deadline_at
        })
        .to_string(),
    )
    .expect("写入活跃标记");
}

fn claim<'a>(payload: &'a Value, id: &str) -> &'a Value {
    payload["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("entries 必须是数组：{payload}"))
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("缺少能力声明 {id}：{payload}"))
}

/// 能力/限制必须逐条机器可判定：holds + status + reason_code + 平台覆盖，而不是一段散文
#[test]
fn capabilities_declares_machine_readable_boundaries() {
    let (server, _settings, _runs) = server_with_runs();
    let response = call_tool(&server, "capabilities", json!({}));
    assert_eq!(
        response["result"]["isError"],
        false,
        "能力声明必须可读取：{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    assert_eq!(payload["version"], "capabilities/1", "{payload}");
    let platform_os = payload["platform"]["os"]
        .as_str()
        .expect("platform.os 必须是字符串");
    assert!(
        ["windows", "linux", "macos", "other"].contains(&platform_os),
        "platform.os 必须是稳定枚举之一：{payload}"
    );
    assert!(
        payload["platform"]["arch"].as_str().is_some(),
        "platform.arch 必须回显：{payload}"
    );

    let entries = payload["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("entries 必须是数组：{payload}"));
    assert!(!entries.is_empty(), "能力声明不得为空：{payload}");
    assert_eq!(
        payload["entries_total"].as_u64(),
        Some(entries.len() as u64),
        "entries_total 必须可核对：{payload}"
    );

    let statuses = payload["status_enum"]
        .as_array()
        .unwrap_or_else(|| panic!("status_enum 必须是数组：{payload}"));
    let allowed_status: Vec<&str> =
        ["proven", "not_proven", "partial", "unverified", "supported"].to_vec();
    let declared: Vec<&str> = statuses
        .iter()
        .map(|value| value.as_str().expect("status_enum 元素必须是字符串"))
        .collect();
    assert_eq!(declared, allowed_status, "status 枚举必须稳定：{payload}");

    let mut ids: Vec<String> = Vec::new();
    let mut not_proven = 0u64;
    for entry in entries {
        let id = entry["id"]
            .as_str()
            .unwrap_or_else(|| panic!("每条声明必须有 id：{entry}"));
        assert!(!ids.iter().any(|seen| seen == id), "声明 id 不得重复：{id}");
        ids.push(id.to_string());
        assert!(
            entry["scope"]
                .as_str()
                .map(|scope| !scope.is_empty())
                .unwrap_or(false),
            "{id} 必须有明确的适用范围 scope"
        );
        assert!(
            entry["holds"].is_boolean() || entry["holds"].is_null(),
            "{id} holds 必须是 true/false/null：{entry}"
        );
        assert!(
            entry["automatic"].is_boolean(),
            "{id} automatic 必须是布尔：{entry}"
        );
        assert!(
            entry["platforms_verified"].is_array(),
            "{id} platforms_verified 必须是数组：{entry}"
        );
        assert!(
            entry["platforms_unverified"].is_array(),
            "{id} platforms_unverified 必须是数组：{entry}"
        );
        let status = entry["status"].as_str().expect("status 必须是字符串");
        assert!(
            declared.contains(&status),
            "{id} status={status} 不在枚举内"
        );
        let holds = entry["holds"].as_bool();
        if holds == Some(false) {
            not_proven += 1;
            let reason = entry["reason_code"].as_str().unwrap_or_default();
            assert!(
                !reason.is_empty(),
                "{id} 不成立时必须给出机器可判的原因码，不能只写散文：{entry}"
            );
        }
        if matches!(status, "proven" | "supported") {
            assert_eq!(
                holds,
                Some(true),
                "{id} status={status} 必须 holds=true：{entry}"
            );
        }
        if status == "unverified" {
            assert!(holds.is_none(), "{id} 未验证必须 holds=null：{entry}");
        }
    }
    assert_eq!(
        payload["not_proven_total"].as_u64(),
        Some(not_proven),
        "not_proven_total 必须可核对：{payload}"
    );

    let rule = &payload["decision_rule"]["holds"];
    for key in ["true", "false", "null"] {
        assert!(
            rule[key]
                .as_str()
                .map(|text| !text.is_empty())
                .unwrap_or(false),
            "decision_rule.holds.{key} 必须有语义说明：{payload}"
        );
    }

    // #3 能力位按平台拆分（v1.45）：Windows 的结构性不可证不再拖着 Linux 一起 false。
    // Linux 限定在 cgroup v2 可写 + cgroup.kill 分支上 proven；Windows 单独条目保持
    // not_proven 并点名真正卡点（需要作业外进程可枚举可终止的 OS 原语）。
    let controlled = claim(&payload, "timeout.terminates_controlled_set");
    assert_eq!(controlled["holds"], true, "{controlled}");
    assert_eq!(controlled["status"], "proven", "{controlled}");
    let whole_tree = claim(&payload, "timeout.cleans_entire_process_tree");
    assert_eq!(whole_tree["holds"], true, "{whole_tree}");
    assert_eq!(whole_tree["status"], "proven", "{whole_tree}");
    assert!(whole_tree["reason_code"].is_null(), "{whole_tree}");
    assert_eq!(
        whole_tree["platforms_verified"],
        json!(["linux"]),
        "{whole_tree}"
    );
    assert_eq!(
        whole_tree["platforms_unverified"],
        json!(["windows", "macos"]),
        "{whole_tree}"
    );
    let whole_tree_evidence = whole_tree["evidence"]
        .as_array()
        .expect("evidence 必须是数组");
    for case in [
        "cgroup_kill_terminates_escapee_that_left_cgroup_procs",
        "cgroup_kill_terminates_adopted_escapee",
        "cgroup_without_kill_reports_still_alive",
        "cgroup_degraded_terminate_is_group_only",
    ] {
        assert!(
            whole_tree_evidence.iter().any(|item| item == case),
            "整树能力位必须引用 {case}：{whole_tree}"
        );
    }
    let whole_tree_windows = claim(&payload, "timeout.cleans_entire_process_tree_windows");
    assert_eq!(whole_tree_windows["holds"], false, "{whole_tree_windows}");
    assert_eq!(
        whole_tree_windows["status"], "not_proven",
        "{whole_tree_windows}"
    );
    assert_eq!(
        whole_tree_windows["reason_code"], "requires_out_of_job_enumerable_terminable_os_primitive",
        "{whole_tree_windows}"
    );
    assert_eq!(
        whole_tree_windows["platforms_verified"],
        json!(["windows"]),
        "{whole_tree_windows}"
    );
    let escaped = claim(&payload, "timeout.enumerates_escaped_descendants");
    assert_eq!(escaped["holds"], true, "{escaped}");
    assert_eq!(escaped["status"], "proven", "{escaped}");
    assert!(escaped["reason_code"].is_null(), "{escaped}");
    assert_eq!(escaped["platforms_verified"], json!(["linux"]), "{escaped}");
    let escaped_evidence = escaped["evidence"].as_array().expect("evidence 必须是数组");
    for case in [
        "cgroup_scope_enumerates_adopted_escapee",
        "cgroup_members_unreadable_is_not_empty",
    ] {
        assert!(
            escaped_evidence.iter().any(|item| item == case),
            "逃逸枚举能力位必须引用 {case}：{escaped}"
        );
    }
    let escaped_windows = claim(&payload, "timeout.enumerates_escaped_descendants_windows");
    assert_eq!(escaped_windows["holds"], false, "{escaped_windows}");
    assert_eq!(escaped_windows["status"], "not_proven", "{escaped_windows}");
    assert_eq!(
        escaped_windows["reason_code"], "requires_out_of_job_enumerable_terminable_os_primitive",
        "{escaped_windows}"
    );
    assert_eq!(
        escaped_windows["platforms_verified"],
        json!(["windows"]),
        "{escaped_windows}"
    );
    // cgroup v2 观测点本身必须可证，且明确只在 linux 上验证过
    let cgroup = claim(&payload, "timeout.cgroup_v2_scope_observation");
    assert_eq!(cgroup["holds"], true, "{cgroup}");
    assert_eq!(cgroup["status"], "proven", "{cgroup}");
    assert!(cgroup["reason_code"].is_null(), "{cgroup}");
    assert_eq!(cgroup["platforms_verified"], json!(["linux"]), "{cgroup}");
    assert_eq!(
        cgroup["platforms_unverified"],
        json!(["windows", "macos"]),
        "{cgroup}"
    );
    // cgroup v2 作用域终止必须单独可证：只声明条件性能力，不把整树能力上调
    let scope_kill = claim(&payload, "timeout.cgroup_kill_terminates_scope");
    assert_eq!(scope_kill["holds"], true, "{scope_kill}");
    assert_eq!(scope_kill["status"], "proven", "{scope_kill}");
    assert!(scope_kill["reason_code"].is_null(), "{scope_kill}");
    assert_eq!(
        scope_kill["platforms_verified"],
        json!(["linux"]),
        "{scope_kill}"
    );
    assert_eq!(
        scope_kill["platforms_unverified"],
        json!(["windows", "macos"]),
        "{scope_kill}"
    );
    assert_eq!(
        scope_kill["evidence"],
        json!(["cgroup_kill_terminates_adopted_escapee"]),
        "{scope_kill}"
    );
    assert_eq!(
        whole_tree["evidence"]
            .as_array()
            .expect("whole_tree evidence")
            .iter()
            .filter(|value| *value == &json!("cgroup_kill_terminates_adopted_escapee"))
            .count(),
        1,
        "整树能力的证据必须包含作用域终止用例：{whole_tree}"
    );
    // 新增观测点本身必须可证：枚举到的后代与受控集合成员可交叉核对
    let snapshot = claim(&payload, "timeout.descendant_snapshot_cross_check");
    assert_eq!(snapshot["holds"], true, "{snapshot}");
    assert_eq!(snapshot["status"], "proven", "{snapshot}");
    assert!(snapshot["reason_code"].is_null(), "{snapshot}");

    // POSIX inflight 语义已单独复核：rename 活跃 attempt 目录的语义
    let posix = claim(&payload, "platform.posix_inflight_semantics_verified");
    assert_eq!(posix["holds"], true, "{posix}");
    assert_eq!(posix["status"], "proven", "{posix}");
    assert!(posix["reason_code"].is_null(), "{posix}");
    assert_eq!(
        posix["platforms_verified"],
        json!(["windows", "linux"]),
        "{posix}"
    );
    assert_eq!(posix["platforms_unverified"], json!(["macos"]), "{posix}");
    assert_eq!(
        posix["evidence"],
        json!([
            "standalone_inflight_marker_is_visible_then_removed",
            "prune_skips_task_with_live_inflight_marker",
            "runs_inflight_lists_and_cleans_only_stale_markers",
            "posix_rename_active_attempt_semantics",
        ]),
        "{posix}"
    );

    // TOCTOU：rename 认领 + 认领后重跑 stale_rule 已被并发用例证成
    let toctou = claim(&payload, "concurrency.check_then_delete_atomic");
    assert_eq!(toctou["holds"], true, "{toctou}");
    assert_eq!(toctou["status"], "proven", "{toctou}");
    assert_eq!(toctou["reason_code"], json!(null), "{toctou}");
    let toctou_evidence = toctou["evidence"].as_array().expect("evidence 必须是数组");
    for item in [
        "sweep_concurrent_claimers_delete_once_others_lost_race",
        "sweep_concurrent_refresher_never_deletes_fresh_marker",
    ] {
        assert!(
            toctou_evidence.contains(&json!(item)),
            "{item} 必须列为证据：{toctou}"
        );
    }
    // 窄结论：还原只做 create-if-absent，不覆盖并发写入的新标记
    let restore = claim(&payload, "concurrency.sweep_restore_never_clobbers");
    assert_eq!(restore["holds"], true, "{restore}");
    assert_eq!(restore["status"], "proven", "{restore}");
    assert!(restore["reason_code"].is_null(), "{restore}");
    assert_eq!(
        restore["platforms_verified"],
        json!(["windows", "linux"]),
        "{restore}"
    );
    assert_eq!(
        restore["platforms_unverified"],
        json!(["macos"]),
        "{restore}"
    );
    assert_eq!(
        restore["evidence"],
        json!(["sweep_restore_never_clobbers_concurrent_new_marker"]),
        "{restore}"
    );

    // 残留标记：可观察、可显式清理，且服务启动/周期自动清扫（v1.42）
    let observable = claim(&payload, "inflight.stale_marker_observable");
    assert_eq!(observable["holds"], true, "{observable}");
    assert_eq!(observable["status"], "supported", "{observable}");
    assert_eq!(observable["automatic"], true, "{observable}");
    let cleanable = claim(&payload, "inflight.stale_marker_cleanable");
    assert_eq!(cleanable["holds"], true, "{cleanable}");
    assert_eq!(
        cleanable["automatic"], false,
        "显式 clean 仍需调用方执行 dry_run=false+confirm=true：{cleanable}"
    );
    let auto = claim(&payload, "inflight.stale_markers_auto_swept");
    assert_eq!(auto["holds"], true, "{auto}");
    assert_eq!(auto["status"], "proven", "{auto}");
    assert!(auto["reason_code"].is_null(), "{auto}");
    assert_eq!(auto["automatic"], true, "{auto}");
    let auto_evidence = auto["evidence"].as_array().expect("evidence 必须是数组");
    for case in [
        "inflight_startup_sweep_removes_stale_and_keeps_active",
        "inflight_periodic_sweep_removes_stale_marker",
        "inflight_auto_sweep_audit_failure_deletes_nothing",
    ] {
        assert!(
            auto_evidence.iter().any(|item| item == case),
            "自动清扫能力位必须引用 {case}：{auto}"
        );
    }
}

/// 已在当前平台实跑的能力必须把当前平台写进 platforms_verified，不能只标 Windows
#[test]
fn capabilities_inflight_entries_cover_current_platform() {
    let (server, _settings, _runs) = server_with_runs();
    let payload = tool_payload(&call_tool(&server, "capabilities", json!({})));
    let current = payload["platform"]["os"]
        .as_str()
        .expect("platform.os 必须是字符串");
    for id in [
        "inflight.stale_marker_observable",
        "inflight.stale_marker_cleanable",
        "inflight.stale_markers_auto_swept",
        "inflight.clean_preserves_active_markers",
    ] {
        let entry = claim(&payload, id);
        assert!(
            entry["platforms_verified"]
                .as_array()
                .expect("platforms_verified 必须是数组")
                .contains(&json!(current)),
            "{id} 已在 {current} 实跑，platforms_verified 必须包含 {current}：{entry}"
        );
        assert!(
            !entry["platforms_unverified"]
                .as_array()
                .expect("platforms_unverified 必须是数组")
                .contains(&json!(current)),
            "{id} 不得同时把已实跑的 {current} 列为未验证：{entry}"
        );
    }
}

/// PM 判活只依赖 list 的明确字段；启发式与诊断字段不得当判活依据
#[test]
fn runs_inflight_list_exposes_pm_liveness_contract() {
    let (server, _settings, runs) = server_with_runs();
    write_inflight_marker(
        runs.path(),
        "liveness-stale",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    let listed = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "list" }),
    ));
    let rule = &listed["liveness_rule"];
    assert_eq!(rule["version"], "inflight-liveness/1", "{listed}");
    assert_eq!(
        rule["authoritative_fields"],
        json!(["markers[].task_id", "markers[].stale"]),
        "判活权威字段必须唯一且明确：{rule}"
    );
    assert_eq!(
        rule["cleanup_fields"],
        json!(["markers[].removable"]),
        "清理依据必须与判活分开：{rule}"
    );
    let diagnostic = rule["diagnostic_fields"]
        .as_array()
        .expect("diagnostic_fields 必须是数组");
    for field in [
        "markers[].pid",
        "markers[].pid_alive",
        "markers[].stale_reason",
        "markers[].age_secs",
    ] {
        assert!(
            diagnostic.contains(&json!(field)),
            "{field} 只能作诊断，必须从判活权威字段中分离：{rule}"
        );
    }
    let forbidden = rule["not_liveness_evidence"]
        .as_array()
        .expect("not_liveness_evidence 必须是数组");
    for item in [
        "attempt_missing_handback",
        "attempt_missing_dispatch",
        "raw_marker_file_presence",
    ] {
        assert!(
            forbidden.contains(&json!(item)),
            "{item} 不得作为判活依据：{rule}"
        );
    }
    assert_eq!(
        rule["fallback"]["allowed"], false,
        "AW 提供 runs.inflight 时不得回退 attempt 文件启发式：{rule}"
    );
    for key in [
        "task_live",
        "task_not_live",
        "not_live_does_not_prove",
        "result_authority",
    ] {
        assert!(
            rule["decision"][key]
                .as_str()
                .map(|text| !text.is_empty())
                .unwrap_or(false),
            "decision.{key} 必须给出明确语义：{rule}"
        );
    }
}

/// 残留态可见、可清、且活跃标记永不误删；正常结束后无残留
#[test]
fn runs_inflight_lists_and_cleans_only_stale_markers() {
    let (server, _settings, runs) = server_with_runs();

    // 残留：pid 已消失且已过有效期
    write_inflight_marker(
        runs.path(),
        "stale-task",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    // 活跃：本进程 pid 存活，即使 deadline 已过也不得判残留
    write_inflight_marker(
        runs.path(),
        "active-task",
        std::process::id() as u64,
        &days_ago_stamp(0),
        &days_ago_stamp(1),
    );

    let listed = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "list" }),
    ));
    assert_eq!(listed["version"], "inflight-report/1", "{listed}");
    assert_eq!(listed["total"], 2, "{listed}");
    assert_eq!(listed["stale_total"], 1, "{listed}");
    assert_eq!(listed["active_total"], 1, "{listed}");
    let markers = listed["markers"].as_array().expect("markers 必须是数组");
    let stale = markers
        .iter()
        .find(|marker| marker["task_id"] == "stale-task")
        .unwrap_or_else(|| panic!("必须列出 stale-task：{listed}"));
    assert_eq!(stale["stale"], true, "{stale}");
    assert_eq!(stale["stale_reason"], "pid_missing", "{stale}");
    assert_eq!(stale["pid_alive"], false, "{stale}");
    assert_eq!(stale["removable"], true, "{stale}");
    let active = markers
        .iter()
        .find(|marker| marker["task_id"] == "active-task")
        .unwrap_or_else(|| panic!("必须列出 active-task：{listed}"));
    assert_eq!(active["stale"], false, "{active}");
    assert_eq!(active["stale_reason"], "pid_alive", "{active}");
    assert_eq!(active["pid_alive"], true, "{active}");
    assert_eq!(active["removable"], false, "{active}");

    // 标记目录不得污染 runs.list 的任务枚举
    let task_list = tool_payload(&call_tool(&server, "runs.list", json!({})));
    assert_eq!(task_list["total_tasks"], 0, "标记目录不是任务：{task_list}");

    // 默认 dry_run：只报告 would_remove，不删除任何文件
    let dry = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean" }),
    ));
    assert_eq!(dry["dry_run"], true, "{dry}");
    assert_eq!(dry["would_remove_total"], 1, "{dry}");
    assert_eq!(dry["removed_total"], 0, "{dry}");
    assert!(
        inflight_marker(runs.path(), "stale-task").exists(),
        "dry-run 不得删文件"
    );
    assert!(
        inflight_marker(runs.path(), "active-task").exists(),
        "dry-run 不得删文件"
    );

    // 真清：只删 stale；活跃标记必须保留
    let cleaned = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false, "confirm": true }),
    ));
    assert_eq!(cleaned["removed_total"], 1, "{cleaned}");
    assert_eq!(cleaned["failed_total"], 0, "{cleaned}");
    let removed = cleaned["removed"].as_array().expect("removed 必须是数组");
    assert_eq!(removed[0]["task_id"], "stale-task", "{cleaned}");
    assert_eq!(removed[0]["stale_reason"], "pid_missing", "{cleaned}");
    assert!(
        !inflight_marker(runs.path(), "stale-task").exists(),
        "残留标记必须被清理：{cleaned}"
    );
    assert!(
        inflight_marker(runs.path(), "active-task").exists(),
        "活跃标记不得被误删：{cleaned}"
    );
    let kept = cleaned["kept"].as_array().expect("kept 必须是数组");
    assert!(
        kept.iter()
            .any(|entry| entry["task_id"] == "active-task" && entry["reason"] == "not_stale"),
        "保留项必须给出原因：{cleaned}"
    );

    // 幂等：再次清理不删任何东西
    let again = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false, "confirm": true }),
    ));
    assert_eq!(again["removed_total"], 0, "{again}");
    assert_eq!(again["stale_total"], 0, "{again}");

    // 正常结束的任务不得留下标记
    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "no-residue", "cmd": "echo done", "timeout_secs": 30 }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    assert!(
        !inflight_marker(runs.path(), "no-residue").exists(),
        "正常结束后必须无残留标记"
    );
    let after = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "list" }),
    ));
    assert_eq!(after["total"], 1, "只剩活跃标记：{after}");
    assert_eq!(after["stale_total"], 0, "{after}");
}

#[test]
fn runs_inflight_clean_requires_explicit_confirmation() {
    let (server, _settings, runs) = server_with_runs();
    write_inflight_marker(
        runs.path(),
        "stale-guard",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    let response = call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false }),
    );
    assert_eq!(
        response["result"]["isError"],
        true,
        "{}",
        tool_text(&response)
    );
    let payload = tool_payload(&response);
    let error = payload["error"].as_str().expect("必须回传具名错误");
    assert!(
        error.starts_with("confirm_required"),
        "真删必须显式 confirm=true：{error}"
    );
    assert!(
        inflight_marker(runs.path(), "stale-guard").exists(),
        "未确认时不得删除"
    );

    let invalid = call_tool(&server, "runs.inflight", json!({ "action": "purge" }));
    assert_eq!(
        invalid["result"]["isError"],
        true,
        "{}",
        tool_text(&invalid)
    );
    let payload = tool_payload(&invalid);
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .starts_with("invalid_arguments"),
        "{payload}"
    );
}

/// 读不懂的标记不得当作可删：保留文件并如实报告 unreadable
#[test]
fn runs_inflight_keeps_unreadable_marker() {
    let (server, _settings, runs) = server_with_runs();
    let dir = runs.path().join(".inflight");
    fs::create_dir_all(&dir).expect("创建 .inflight");
    fs::write(dir.join("bad.json"), "{not-json").expect("写入坏标记");

    let listed = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "list" }),
    ));
    assert_eq!(listed["total"], 0, "{listed}");
    assert_eq!(listed["unreadable_total"], 1, "{listed}");
    let unreadable = listed["unreadable"]
        .as_array()
        .expect("unreadable 必须是数组");
    assert_eq!(unreadable[0]["file"], "bad.json", "{listed}");
    assert_eq!(unreadable[0]["stale_reason"], "unreadable", "{listed}");
    assert_eq!(unreadable[0]["removable"], false, "{listed}");

    let cleaned = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false, "confirm": true }),
    ));
    assert_eq!(cleaned["removed_total"], 0, "{cleaned}");
    assert!(
        dir.join("bad.json").exists(),
        "不可读标记不得被删除：{cleaned}"
    );
    assert!(
        cleaned["kept"]
            .as_array()
            .expect("kept 必须是数组")
            .iter()
            .any(|entry| entry["file"] == "bad.json" && entry["reason"] == "unreadable"),
        "必须逐条说明保留原因：{cleaned}"
    );
}

/// 真实 stdio 多进程：正常结束无残留；外部构造的 pid 消失+超时标记可被识别并清掉
#[test]
fn runs_inflight_real_stdio_observes_and_cleans_stale_marker() {
    use std::io::{BufRead, BufReader, Write};

    let exe = env!("CARGO_BIN_EXE_aw-mcp-bridge");
    let runs = Path::new(exe)
        .parent()
        .expect("可执行文件目录")
        .join("runs");
    let task_id = format!("inflight-real-{}", std::process::id());
    let stale_id = format!("{task_id}-stale");
    let stale_marker = runs.join(".inflight").join(format!("{stale_id}.json"));
    let task_dir = runs.join(&task_id);
    let _ = fs::remove_file(&stale_marker);
    let _ = fs::remove_dir_all(&task_dir);

    let home = tempfile::tempdir().expect("tempdir");
    let mut bridge_command = std::process::Command::new(exe);
    bridge_command
        .env("APPDATA", home.path())
        // 该用例自己造残标再显式清理；关闭自动清扫，避免与并发用例的 runs 目录互相影响
        .env("AW_INFLIGHT_SWEEP_SECS", "off")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = spawn_bridge(&mut bridge_command);
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    let mut exchange = |id: u64, method: &str, params: Value| -> Value {
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
        )
        .expect("写入请求");
        let mut line = String::new();
        stdout.read_line(&mut line).expect("读取响应失败");
        serde_json::from_str(line.trim()).expect("响应必须是 JSON")
    };
    let text_payload = |response: &Value| -> Value {
        serde_json::from_str(
            response["result"]["content"][0]["text"]
                .as_str()
                .expect("必须是文本载荷"),
        )
        .expect("载荷必须是 JSON")
    };

    exchange(1, "initialize", json!({}));
    let capabilities = text_payload(&exchange(
        2,
        "tools/call",
        json!({
            "name": "capabilities", "arguments": {}
        }),
    ));
    assert_eq!(capabilities["version"], "capabilities/1", "{capabilities}");

    let executed = text_payload(&exchange(
        3,
        "tools/call",
        json!({
            "name": "task.execute",
            "arguments": { "task_id": task_id, "cmd": "echo done", "timeout_secs": 30 }
        }),
    ));
    assert_eq!(executed["status"], "ok", "{executed}");
    let after_run = text_payload(&exchange(
        4,
        "tools/call",
        json!({
            "name": "runs.inflight", "arguments": { "action": "list" }
        }),
    ));
    assert_eq!(
        after_run["total"], 0,
        "正常结束后不得有残留标记：{after_run}"
    );

    // 外部构造：pid 已消失 + deadline 已过
    fs::create_dir_all(runs.join(".inflight")).expect("创建 .inflight");
    fs::write(
        &stale_marker,
        json!({
            "version": "inflight-marker/1",
            "task_id": stale_id,
            "pid": DEAD_PID,
            "mode": "dispatched",
            "state": "active",
            "startedAt": days_ago_stamp(2),
            "deadlineAt": days_ago_stamp(1)
        })
        .to_string(),
    )
    .expect("写入残留标记");

    let listed = text_payload(&exchange(
        5,
        "tools/call",
        json!({
            "name": "runs.inflight", "arguments": { "action": "list" }
        }),
    ));
    assert_eq!(listed["stale_total"], 1, "{listed}");
    let marker = &listed["markers"][0];
    assert_eq!(marker["task_id"], stale_id, "{listed}");
    assert_eq!(marker["stale_reason"], "pid_missing", "{listed}");

    let cleaned = text_payload(&exchange(
        6,
        "tools/call",
        json!({
            "name": "runs.inflight",
            "arguments": { "action": "clean", "dry_run": false, "confirm": true }
        }),
    ));
    assert_eq!(cleaned["removed_total"], 1, "{cleaned}");
    assert!(!stale_marker.exists(), "残留标记必须被真实清理");

    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(&task_dir);
}

// ---- v1.33：残留 inflight 标记在既有动作路径上顺带清扫（无后台线程） ----

fn read_inflight_sweep_audit(runs: &Path) -> Vec<Value> {
    let path = runs.join("inflight-sweep.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .expect("读取 inflight-sweep.jsonl")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("审计行必须是 JSON"))
        .collect()
}

/// 同一 task_id 再次启动时只清扫自己的残标：别的 task 的残标不动，审计必须留痕
#[test]
fn inflight_sweep_on_task_start_removes_only_target_stale_marker() {
    let (server, _settings, runs) = server_with_runs();
    write_inflight_marker(
        runs.path(),
        "sweep-start",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    write_inflight_marker(
        runs.path(),
        "other-residue",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );

    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "sweep-start", "cmd": "echo done", "timeout_secs": 30 }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    assert!(
        !inflight_marker(runs.path(), "sweep-start").exists(),
        "重启前必须清掉同一 task_id 的残标"
    );
    assert!(
        inflight_marker(runs.path(), "other-residue").exists(),
        "启动清扫只作用于当前 task_id，不得波及其他 task"
    );

    let audit = read_inflight_sweep_audit(runs.path());
    assert!(!audit.is_empty(), "清扫必须写审计：{audit:?}");
    assert_eq!(audit[0]["version"], "inflight-sweep/1", "{audit:?}");
    assert_eq!(audit[0]["phase"], "plan", "先记账后删除：{audit:?}");
    assert_eq!(audit[0]["trigger"], "task_start", "{audit:?}");
    let result = audit
        .iter()
        .find(|line| line["phase"] == "result" && line["trigger"] == "task_start")
        .unwrap_or_else(|| panic!("必须写清扫结果行：{audit:?}"));
    assert_eq!(result["removed_total"], 1, "{result}");
    assert_eq!(result["removed"][0]["task_id"], "sweep-start", "{result}");
    assert_eq!(
        result["removed"][0]["stale_reason"], "pid_missing",
        "{result}"
    );
    assert!(
        result["removed"][0]["path"]
            .as_str()
            .unwrap_or_default()
            .starts_with("runs/.inflight/"),
        "审计必须写明清理对象路径：{result}"
    );
}

/// prune 真删前先清残标，清完后该 task 不再被 task_inflight_stale 挡住
#[test]
fn inflight_sweep_on_prune_removes_stale_marker_and_unblocks_candidate() {
    let (server, _settings, runs) = server_with_runs();
    prune_attempt(
        runs.path(),
        "prune-sweep",
        1,
        true,
        Some(&days_ago_stamp(30)),
        true,
    );
    prune_attempt(runs.path(), "prune-sweep", 2, false, None, true);
    write_inflight_marker(
        runs.path(),
        "prune-sweep",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );

    let pruned = tool_payload(&call_tool(
        &server,
        "runs.prune",
        json!({ "older_than_days": 30, "dry_run": false, "confirm": true }),
    ));
    assert_eq!(pruned["inflight_sweep"]["trigger"], "prune", "{pruned}");
    assert_eq!(pruned["inflight_sweep"]["removed_total"], 1, "{pruned}");
    assert!(
        !inflight_marker(runs.path(), "prune-sweep").exists(),
        "prune 前必须清掉残标：{pruned}"
    );
    assert!(
        pruned["deleted"]
            .as_array()
            .expect("deleted 必须是数组")
            .iter()
            .any(|entry| entry["task_id"] == "prune-sweep" && entry["attempt"] == 1),
        "清掉残标后 prune 不得再跳过该 task：{pruned}"
    );

    let audit = read_inflight_sweep_audit(runs.path());
    let result = audit
        .iter()
        .find(|line| line["phase"] == "result" && line["trigger"] == "prune")
        .unwrap_or_else(|| panic!("必须写 prune 清扫结果行：{audit:?}"));
    assert_eq!(result["removed_total"], 1, "{result}");
    assert_eq!(
        result["removed"][0]["stale_reason"], "pid_missing",
        "{result}"
    );
}

/// 显式 clean 也写审计；坏标记保留并报 unreadable；重复 clean 幂等且不再追加审计
#[test]
fn inflight_clean_writes_audit_and_is_idempotent() {
    let (server, _settings, runs) = server_with_runs();
    write_inflight_marker(
        runs.path(),
        "audit-stale",
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    write_inflight_marker(
        runs.path(),
        "audit-active",
        std::process::id() as u64,
        &days_ago_stamp(0),
        &days_ago_stamp(1),
    );
    let dir = runs.path().join(".inflight");
    fs::write(dir.join("audit-bad.json"), "{not-json").expect("写入坏标记");

    let cleaned = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false, "confirm": true }),
    ));
    assert_eq!(cleaned["removed_total"], 1, "{cleaned}");
    assert!(
        inflight_marker(runs.path(), "audit-active").exists(),
        "活跃标记不得被误删：{cleaned}"
    );
    assert!(
        dir.join("audit-bad.json").exists(),
        "不可读标记不得被删除：{cleaned}"
    );

    let audit = read_inflight_sweep_audit(runs.path());
    assert_eq!(audit.len(), 2, "一次真清必须写 plan+result 两行：{audit:?}");
    let result = audit
        .iter()
        .find(|line| line["phase"] == "result")
        .unwrap_or_else(|| panic!("必须写清扫结果行：{audit:?}"));
    assert_eq!(result["trigger"], "clean", "{result}");
    assert_eq!(result["removed_total"], 1, "{result}");
    assert_eq!(result["removed"][0]["task_id"], "audit-stale", "{result}");
    assert_eq!(
        result["removed"][0]["stale_reason"], "pid_missing",
        "{result}"
    );
    assert_eq!(result["unreadable_total"], 1, "{result}");

    let again = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "clean", "dry_run": false, "confirm": true }),
    ));
    assert_eq!(again["removed_total"], 0, "重复清理必须幂等：{again}");
    assert_eq!(
        read_inflight_sweep_audit(runs.path()).len(),
        2,
        "无删除发生不得追加审计"
    );
}

/// 复制可执行文件与 spawn 之间串行化：并发测试在 copy 尚未关闭写句柄时 fork，
/// 另一个测试 exec 自己的副本会偶发 ETXTBSY（Text file busy），故用同一把锁隔开
static BRIDGE_BINARY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// POSIX 下 `fs::copy` 的写 fd 在 fork 时会被其它测试线程的子进程短暂继承，
/// 即便加了复制锁，本进程 exec 副本仍可能在那个窗口里拿到 ETXTBSY。
/// 这是测试进程内 fork/exec 的固有竞态，不是产品行为：有界重试，重试成功时打印证据行。
fn spawn_bridge(command: &mut Command) -> std::process::Child {
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        match command.spawn() {
            Ok(child) => {
                if attempts > 1 {
                    println!("[evidence] spawn_retry attempts={attempts} reason=ETXTBSY");
                }
                return child;
            }
            Err(error) if attempts < 100 && is_text_file_busy(&error) => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("启动真实 bridge 进程: {error}"),
        }
    }
}

fn is_text_file_busy(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::ETXTBSY)
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

/// 把 bridge 可执行文件复制到独立目录，让 current_exe 旁的 runs/ 与其它用例隔离
fn copy_bridge_binary(dir: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_BIN_EXE_aw-mcp-bridge"));
    let target = dir.join(source.file_name().expect("可执行文件名"));
    fs::copy(source, &target).expect("复制 bridge 可执行文件");
    target
}

/// 自动清扫的启动触发：真实进程重启后清掉真残标、保留活跃标记，审计 trigger=startup
#[test]
fn inflight_startup_sweep_removes_stale_and_keeps_active() {
    use std::io::{BufRead, BufReader, Write};

    let home = tempfile::tempdir().expect("tempdir");
    let guard = BRIDGE_BINARY_LOCK.lock().expect("bridge 复制锁");
    let exe = copy_bridge_binary(home.path());
    let runs = home.path().join("runs");
    let stale_id = format!("startup-stale-{}", std::process::id());
    let active_id = format!("startup-active-{}", std::process::id());
    write_inflight_marker(
        &runs,
        &stale_id,
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    write_inflight_marker(
        &runs,
        &active_id,
        std::process::id() as u64,
        &days_ago_stamp(0),
        &days_ago_stamp(0),
    );

    let mut bridge_command = Command::new(&exe);
    bridge_command
        .env("APPDATA", home.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = spawn_bridge(&mut bridge_command);
    drop(guard);
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
    )
    .expect("写入 initialize");
    let mut line = String::new();
    stdout.read_line(&mut line).expect("读取 initialize 响应");

    let mut removed = false;
    for _ in 0..100 {
        if !inflight_marker(&runs, &stale_id).exists() {
            removed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(removed, "服务启动时必须清掉真残标");
    assert!(
        inflight_marker(&runs, &active_id).exists(),
        "pid 存活的活跃标记绝不能被启动清扫删除"
    );
    let audit = read_inflight_sweep_audit(&runs);
    let result = audit
        .iter()
        .find(|record| {
            record["trigger"] == "startup"
                && record["phase"] == "result"
                && record["removed_total"] == 1
        })
        .unwrap_or_else(|| panic!("启动清扫必须写 trigger=startup 的 result 行：{audit:?}"));
    assert_eq!(result["removed"][0]["task_id"], stale_id, "{result}");
    assert_eq!(
        result["removed"][0]["stale_reason"], "pid_missing",
        "{result}"
    );
    assert_eq!(result["kept_total"], 1, "活跃标记必须出现在 kept：{result}");
    println!(
        "[evidence] inflight_startup trigger={} removed_total={} kept_total={} removed_task={} active_task={active_id} active_marker_kept=true",
        result["trigger"],
        result["removed_total"],
        result["kept_total"],
        result["removed"][0]["task_id"]
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// 自动清扫的周期触发：启动清扫之后才造的残标只能由周期清扫清掉，审计 trigger=periodic
#[test]
fn inflight_periodic_sweep_removes_stale_marker() {
    use std::io::{BufRead, BufReader, Write};

    let home = tempfile::tempdir().expect("tempdir");
    let guard = BRIDGE_BINARY_LOCK.lock().expect("bridge 复制锁");
    let exe = copy_bridge_binary(home.path());
    let runs = home.path().join("runs");
    let mut bridge_command = Command::new(&exe);
    bridge_command
        .env("APPDATA", home.path())
        .env("AW_INFLIGHT_SWEEP_SECS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = spawn_bridge(&mut bridge_command);
    drop(guard);
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
    )
    .expect("写入 initialize");
    let mut line = String::new();
    stdout.read_line(&mut line).expect("读取 initialize 响应");

    // initialize 返回时启动清扫已结束；此时造的残标只可能被 periodic 清掉
    let stale_id = format!("periodic-stale-{}", std::process::id());
    write_inflight_marker(
        &runs,
        &stale_id,
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    let mut removed = false;
    for _ in 0..150 {
        if !inflight_marker(&runs, &stale_id).exists() {
            removed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    assert!(removed, "周期清扫必须清掉启动之后出现的残标");
    let audit = read_inflight_sweep_audit(&runs);
    let result = audit
        .iter()
        .find(|record| {
            record["trigger"] == "periodic"
                && record["phase"] == "result"
                && record["removed_total"] == 1
        })
        .unwrap_or_else(|| panic!("周期清扫必须写 trigger=periodic 的 result 行：{audit:?}"));
    assert_eq!(result["removed"][0]["task_id"], stale_id, "{result}");
    println!(
        "[evidence] inflight_periodic trigger={} removed_total={} removed_task={}",
        result["trigger"], result["removed_total"], result["removed"][0]["task_id"]
    );
    assert!(
        !audit
            .iter()
            .any(|record| record["trigger"] == "startup" && record["removed_total"] == 1),
        "该残标不可能由启动清扫清掉：{audit:?}"
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// 审计写不进去就零删除：自动路径同样受这条约束
#[test]
fn inflight_auto_sweep_audit_failure_deletes_nothing() {
    let (server, _settings, runs) = server_with_runs();
    let stale_id = "auto-audit-fail";
    write_inflight_marker(
        runs.path(),
        stale_id,
        DEAD_PID,
        &days_ago_stamp(2),
        &days_ago_stamp(1),
    );
    // 用目录占住审计文件位置，让 append 必定失败
    let audit_path = runs.path().join("inflight-sweep.jsonl");
    fs::create_dir_all(&audit_path).expect("占住审计文件位置");

    server.run_startup_sweep();

    println!(
        "[evidence] inflight_auto_audit_failure marker_exists={} audit_path_is_dir={}",
        inflight_marker(runs.path(), stale_id).exists(),
        audit_path.is_dir()
    );
    assert!(
        inflight_marker(runs.path(), stale_id).exists(),
        "审计 plan 行写不进去时自动清扫必须零删除"
    );
    assert!(
        audit_path.is_dir(),
        "审计占位目录必须仍在，说明确实没写成文件"
    );
}

/// 启动路径遇到不可读标记：保留原文件、任务照常执行，不得把坏标记当残标删掉
#[test]
fn inflight_sweep_on_task_start_preserves_unreadable_marker() {
    let (server, _settings, runs) = server_with_runs();
    let dir = runs.path().join(".inflight");
    fs::create_dir_all(&dir).expect("创建 .inflight");
    fs::write(dir.join("bad-start.json"), "{not-json").expect("写入坏标记");

    let response = call_tool(
        &server,
        "task.execute",
        json!({ "task_id": "bad-start", "cmd": "echo done", "timeout_secs": 30 }),
    );
    assert_eq!(
        response["result"]["isError"],
        false,
        "{}",
        tool_text(&response)
    );
    assert!(
        dir.join("bad-start.json").exists(),
        "不可读标记必须保留，不能当作残标删除"
    );
    assert!(
        read_inflight_sweep_audit(runs.path()).is_empty(),
        "没有可清扫的残标时不得写审计"
    );
}

/// list 必须机器可读地给出顺带清扫的触发条件、审计位置与仍然存在的边界
#[test]
fn runs_inflight_list_exposes_sweep_contract() {
    let (server, _settings, _runs) = server_with_runs();
    let listed = tool_payload(&call_tool(
        &server,
        "runs.inflight",
        json!({ "action": "list" }),
    ));
    let rule = &listed["sweep_rule"];
    assert_eq!(rule["version"], "inflight-sweep/1", "{listed}");
    assert_eq!(rule["audit"], "runs/inflight-sweep.jsonl", "{rule}");
    assert_eq!(rule["idempotent"], true, "{rule}");
    let triggers = rule["triggers"].as_array().expect("triggers 必须是数组");
    for trigger in [
        "startup",
        "periodic",
        "task_start",
        "runs.prune",
        "runs.inflight.clean",
    ] {
        assert!(
            triggers.contains(&json!(trigger)),
            "{trigger} 必须写明：{rule}"
        );
    }
    assert_eq!(
        rule["automatic"]["periodic"], "默认每 3600 秒一次，默认开启",
        "{rule}"
    );
    assert_eq!(rule["automatic"]["env"], "AW_INFLIGHT_SWEEP_SECS", "{rule}");
    assert!(
        rule["audit_gate"]
            .as_str()
            .unwrap_or_default()
            .contains("零删除"),
        "审计闸门必须机器可读：{rule}"
    );
    let never = rule["never_removed"]
        .as_array()
        .expect("never_removed 必须是数组");
    for item in ["active_pid_alive", "unreadable"] {
        assert!(never.contains(&json!(item)), "{item} 必须写明：{rule}");
    }
    let limits = rule["limits"].as_array().expect("limits 必须是数组");
    for item in [
        "same_volume_rename_required",
        "transient_name_absence_during_claim",
        "audit_append_not_transactional",
        "unreadable_not_self_healing",
    ] {
        assert!(limits.contains(&json!(item)), "{item} 必须写明：{rule}");
    }
    let outcomes = rule["outcome_totals"]
        .as_array()
        .expect("outcome_totals 必须是数组");
    for item in ["deleted_total", "lost_race_total", "not_mine_total"] {
        assert!(outcomes.contains(&json!(item)), "{item} 必须写明：{rule}");
    }
    assert!(
        rule["atomic_delete"]
            .as_str()
            .unwrap_or_default()
            .contains("stale_rule"),
        "atomic_delete 必须写明认领后重跑 stale_rule：{rule}"
    );
}

// ---- v1.47：显式取消 + `_session` 按天保留 ----

#[test]
fn task_cancel_terminates_running_execution_and_reports_canceled() {
    let (server, _settings, runs) = server_with_runs();
    let server = Arc::new(server);
    #[cfg(windows)]
    let slow = "ping -n 30 127.0.0.1 > NUL";
    #[cfg(not(windows))]
    let slow = "sleep 30";
    let worker = {
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            call_tool(
                &server,
                "task.execute",
                json!({ "task_id": "t-cancel", "cmd": slow, "timeout_secs": 60 }),
            )
        })
    };
    // 等 dispatch.json 落盘：标记比 spawn 更早，单等标记可能撞上"启动前取消"分支
    let dispatch = runs
        .path()
        .join("t-cancel")
        .join("attempt-1")
        .join("dispatch.json");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !dispatch.exists() {
        assert!(Instant::now() < deadline, "dispatch.json 未在 15s 内落盘");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));

    let canceled = tool_payload(&call_tool(
        &server,
        "task.cancel",
        json!({ "task_id": "t-cancel", "reason": "验收取消" }),
    ));
    assert_eq!(canceled["status"], "canceled", "{canceled}");
    assert_eq!(canceled["canceled"], true, "{canceled}");
    assert_eq!(canceled["cleanup_claimed"], true, "{canceled}");
    assert_eq!(canceled["handback_status"], "canceled", "{canceled}");

    let executed = tool_payload(&worker.join().expect("执行线程不得 panic"));
    assert_eq!(executed["status"], "canceled", "{executed}");
    let reason = executed["blocked_reason"]
        .as_str()
        .expect("取消回执必须有 blocked_reason");
    assert!(reason.starts_with("canceled："), "{reason}");
    assert!(reason.contains("请求方理由：验收取消"), "{reason}");
    assert!(
        !reason.contains("timeout："),
        "取消不得被记成超时：{reason}"
    );
    assert!(
        !runs.path().join(".inflight").join("t-cancel.json").exists(),
        "取消完成后不得留下活跃标记"
    );
    let handback: Value = serde_json::from_str(
        &fs::read_to_string(
            runs.path()
                .join("t-cancel")
                .join("attempt-1")
                .join("handback.json"),
        )
        .expect("取消回执必须落盘"),
    )
    .expect("回执必须是 JSON");
    assert_eq!(handback["status"], "canceled", "{handback}");
}

#[test]
fn task_cancel_is_visible_across_server_instances() {
    let settings_a = TempDir::new().expect("设置目录 A");
    let settings_b = TempDir::new().expect("设置目录 B");
    let runs = TempDir::new().expect("共享运行目录");
    let server_a = Arc::new(Server::with_runs_dir(
        settings_a.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    let server_b = Arc::new(Server::with_runs_dir(
        settings_b.path().join("settings.json"),
        runs.path().to_path_buf(),
    ));
    #[cfg(windows)]
    let slow = "ping -n 30 127.0.0.1 > NUL";
    #[cfg(not(windows))]
    let slow = "sleep 30";
    let worker = {
        let server = Arc::clone(&server_a);
        std::thread::spawn(move || {
            call_tool(
                &server,
                "task.execute",
                json!({ "task_id": "t-cross-cancel", "cmd": slow, "timeout_secs": 60 }),
            )
        })
    };
    let dispatch = runs
        .path()
        .join("t-cross-cancel")
        .join("attempt-1")
        .join("dispatch.json");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !dispatch.exists() {
        assert!(Instant::now() < deadline, "dispatch.json 未在 15s 内落盘");
        std::thread::sleep(Duration::from_millis(20));
    }
    let marker: Value = serde_json::from_str(
        &fs::read_to_string(runs.path().join(".inflight").join("t-cross-cancel.json"))
            .expect("读取活跃标记"),
    )
    .expect("标记必须是 JSON");
    std::thread::sleep(Duration::from_millis(300));

    // B 进程没有本地取消句柄：必须靠磁盘请求生效，而不是返回 not_running
    let canceled = tool_payload(&call_tool(
        &server_b,
        "task.cancel",
        json!({
            "task_id": "t-cross-cancel",
            "reason": "跨进程验收",
            "requested_by": "test-b"
        }),
    ));
    assert_eq!(canceled["status"], "canceled", "{canceled}");
    assert_eq!(canceled["cross_process"], true, "{canceled}");
    assert_eq!(canceled["cleanup_claimed"], true, "{canceled}");
    assert_eq!(canceled["request_state"], "done", "{canceled}");
    assert_eq!(canceled["handback_status"], "canceled", "{canceled}");
    assert!(canceled["requested_at"].is_string(), "{canceled}");
    assert!(canceled["acknowledged_at"].is_string(), "{canceled}");
    assert!(canceled["finished_at"].is_string(), "{canceled}");

    let request: Value = serde_json::from_str(
        &fs::read_to_string(runs.path().join(".cancel").join("t-cross-cancel.json"))
            .expect("取消请求文件必须保留"),
    )
    .expect("请求必须是 JSON");
    assert_eq!(request["status"], "done", "{request}");
    assert_eq!(request["requested_by"], "test-b", "{request}");
    assert_eq!(request["handback_status"], "canceled", "{request}");
    assert_eq!(request["target"]["pid"], marker["pid"], "{request}");
    assert_eq!(
        request["target"]["startedAt"], marker["startedAt"],
        "取消只能点名本轮执行：{request}"
    );

    let executed = tool_payload(&worker.join().expect("执行线程不得 panic"));
    assert_eq!(executed["status"], "canceled", "{executed}");
    // 执行结束后标记已删：再次取消只能是 not_running，且不得覆盖已落盘的 done 请求
    let again = tool_payload(&call_tool(
        &server_b,
        "task.cancel",
        json!({ "task_id": "t-cross-cancel", "requested_by": "test-b" }),
    ));
    assert_eq!(again["status"], "not_running", "{again}");
    let request_after: Value = serde_json::from_str(
        &fs::read_to_string(runs.path().join(".cancel").join("t-cross-cancel.json"))
            .expect("已完成请求文件必须保留"),
    )
    .expect("请求必须是 JSON");
    assert_eq!(request_after["status"], "done", "{request_after}");
    assert_eq!(request_after["requested_by"], "test-b", "{request_after}");

    let ghost = tool_payload(&call_tool(
        &server_b,
        "task.cancel",
        json!({ "task_id": "t-cross-ghost", "requested_by": "test-b" }),
    ));
    assert_eq!(ghost["status"], "not_running", "{ghost}");
    assert!(
        !runs
            .path()
            .join(".cancel")
            .join("t-cross-ghost.json")
            .exists(),
        "not_running 不得留下取消请求，避免污染后续同 task_id 执行"
    );
}

#[test]
fn task_cancel_without_visible_execution_never_claims_cleanup() {
    let (server, _settings, runs) = server_with_runs();
    let inflight = runs.path().join(".inflight");
    fs::create_dir_all(&inflight).expect("创建 .inflight");
    let marker = inflight.join("t-cancel-ghost.json");
    fs::write(
        &marker,
        json!({
            "version": "inflight/1",
            "task_id": "t-cancel-ghost",
            "pid": 999999,
            "startedAt": "2000-01-01T00:00:00+00:00",
            "deadlineAt": "2000-01-01T00:00:05+00:00"
        })
        .to_string(),
    )
    .expect("写入残标");

    // schema 声明 maxLength=1024：超长理由必须在做任何事之前被拒
    let too_long = call_tool(
        &server,
        "task.cancel",
        json!({ "task_id": "t-cancel-ghost", "reason": "x".repeat(1025) }),
    );
    let error_text = tool_text(&too_long);
    assert!(
        error_text.contains("invalid_reason"),
        "超长 reason 必须拒绝：{error_text}"
    );
    assert!(marker.exists(), "被拒的取消不得触碰残标");

    let payload = tool_payload(&call_tool(
        &server,
        "task.cancel",
        json!({ "task_id": "t-cancel-ghost" }),
    ));
    assert_eq!(payload["status"], "not_running", "{payload}");
    assert_eq!(payload["canceled"], false, "{payload}");
    assert_eq!(
        payload["cleanup_claimed"], false,
        "看不到进程时不得宣称已清理：{payload}"
    );
    assert_eq!(
        payload["inflight_marker"]["stale"], true,
        "残标只作磁盘观测返回：{payload}"
    );
    assert!(
        payload["message"]
            .as_str()
            .unwrap_or_default()
            .contains("未向任何进程发送信号"),
        "{payload}"
    );
    assert!(marker.exists(), "取消不得顺手删除/清理残标");
}

#[test]
fn session_initialize_records_are_retained_by_days_with_audit() {
    let (server, _settings, runs) = server_with_runs();
    let dir = runs.path().join("_session");
    fs::create_dir_all(&dir).expect("创建 _session");
    let old_ts = (chrono::Utc::now() - chrono::Duration::days(45)).timestamp_millis();
    let fresh_ts = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();
    let old = dir.join(format!("initialize-111-{old_ts}-0.json"));
    let fresh = dir.join(format!("initialize-222-{fresh_ts}-0.json"));
    let unparsable = dir.join("initialize-broken-name.json");
    for path in [&old, &fresh, &unparsable] {
        fs::write(path, "{\"clientInfo\":{\"name\":\"gate\"}}").expect("写入 initialize 记录");
    }
    let old_mtime = SystemTime::now() - Duration::from_secs(45 * 24 * 3600);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&old)
        .expect("打开旧记录");
    file.set_times(fs::FileTimes::new().set_modified(old_mtime))
        .expect("回拨 mtime");

    server.run_startup_sweep();

    assert!(!old.exists(), "超过保留天数的 initialize 记录必须被删除");
    assert!(fresh.exists(), "未到保留天数的记录必须保留");
    assert!(
        unparsable.exists(),
        "名字解析不出时间的记录判不准，必须保留"
    );
    let audit = fs::read_to_string(runs.path().join("session-retention.jsonl"))
        .expect("删除必须留下 session-retention 审计");
    assert!(audit.contains("\"phase\":\"plan\""), "{audit}");
    assert!(audit.contains("\"phase\":\"result\""), "{audit}");
    assert!(audit.contains("\"trigger\":\"startup\""), "{audit}");
    assert!(audit.contains("\"outcome\":\"deleted\""), "{audit}");
    assert!(audit.contains("\"reason\":\"unparsable_name\""), "{audit}");
    assert!(audit.contains("\"within_retention_total\":1"), "{audit}");
}
