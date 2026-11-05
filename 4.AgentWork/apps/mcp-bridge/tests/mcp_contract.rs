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

    let audit_path = runs.path().join("_session").join("initialize.json");
    assert!(audit_path.exists(), "initialize 必须落审计记录");
    let audit: Value =
        serde_json::from_str(&fs::read_to_string(&audit_path).expect("读取审计记录")).unwrap();
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

    let audit_path = runs.path().join("_session").join("initialize.json");
    assert!(audit_path.exists(), "缺 clientInfo 也要落审计记录");
    let audit: Value =
        serde_json::from_str(&fs::read_to_string(&audit_path).expect("读取审计记录")).unwrap();
    assert!(
        audit["clientInfo"].is_null(),
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

    let audit_path = runs.path().join("_session").join("initialize.json");
    let audit: Value =
        serde_json::from_str(&fs::read_to_string(&audit_path).expect("读取审计记录")).unwrap();
    assert_eq!(audit["clientInfo"]["name"], "../../untrusted", "只记录、不解释");
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
