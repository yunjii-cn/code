// aw-mcp-bridge 方法分发：只读 health/status + 执行层 task.execute/receipt.get
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "aw-mcp-bridge";
/// 会话审计目录名：以 _ 开头，与合法 task_id（必须字母或数字开头）不可能撞名
const SESSION_DIR: &str = "_session";

// 由 build.rs 注入，缺失时为 "unknown"（不使用当前时间兜底，保证可复现构建）
pub const BUILD_TIME: &str = env!("AW_BUILD_TIME");
pub const DESKTOP_BUILD_TIME: &str = env!("AW_DESKTOP_BUILD_TIME");

pub struct Server {
    settings_path: PathBuf,
    runs_dir: PathBuf,
}

impl Server {
    pub fn new(settings_path: PathBuf) -> Self {
        Self::with_runs_dir(settings_path, default_runs_dir())
    }

    pub fn with_runs_dir(settings_path: PathBuf, runs_dir: PathBuf) -> Self {
        Self {
            settings_path,
            runs_dir,
        }
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
            "initialize" => {
                // clientInfo 只落审计记录：它是调用方自称，不是身份，任何准入逻辑都不得读它
                self.record_client_info(message.get("params"));
                Some(json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": env!("CARGO_PKG_VERSION"),
                        "title": "AgentWork MCP Bridge"
                    }
                }))
            }
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
        let arguments = params
            .and_then(|value| value.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        let payload = match name {
            "health" => Ok(self.health_payload()),
            "status" => Ok(self.status_payload()),
            "task.execute" => self.execute_task(&arguments),
            "receipt.get" => self.receipt_payload(&arguments),
            _ => {
                return tool_result(
                    &json!({ "error": format!("未知工具：{name}") }),
                    true,
                )
            }
        };

        match payload {
            Ok(payload) => tool_result(&payload, false),
            Err(error) => tool_result(&json!({ "error": error }), true),
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
            "session": self.session_audit(),
            "server": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "buildTime": BUILD_TIME,
                "desktopBuildTime": DESKTOP_BUILD_TIME
            }
        })
    }

    /// 记录最近一次 initialize 的 clientInfo（只审计，不鉴权）
    ///
    /// 边界写死：clientInfo 是调用方**自称**，可任意伪造，因此
    /// 1. 不构成身份，不产生任何凭据；
    /// 2. 任何拒绝/放行逻辑都不得读取本记录；
    /// 3. 写审计失败不影响 initialize 应答（审计不得成为可用性前提）。
    fn record_client_info(&self, params: Option<&Value>) {
        let client_info = params
            .and_then(|value| value.get("clientInfo"))
            .cloned()
            .unwrap_or(Value::Null);
        let record = json!({
            "clientInfo": client_info,
            "receivedAt": chrono::Utc::now().to_rfc3339()
        });
        let session_dir = self.runs_dir.join(SESSION_DIR);
        if fs::create_dir_all(&session_dir).is_err() {
            return;
        }
        let text = match serde_json::to_string_pretty(&record) {
            Ok(text) => text,
            Err(_) => return,
        };
        let _ = fs::write(session_dir.join("initialize.json"), text);
    }

    /// 只读回审计记录：供 status 展示"最近一次 initialize 的 clientInfo"
    fn session_audit(&self) -> Value {
        let path = self.runs_dir.join(SESSION_DIR).join("initialize.json");
        let record = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
        match record {
            Some(record) => json!({
                "clientInfo": record.get("clientInfo").cloned().unwrap_or(Value::Null),
                "receivedAt": record.get("receivedAt").cloned().unwrap_or(Value::Null)
            }),
            None => json!({ "clientInfo": null, "receivedAt": null }),
        }
    }

    /// 接单执行：命令与判据由派单方给出，AW 只运行并产出可审计回执
    ///
    /// 幂等口径：task_id 即派单标识。同参重投复用既有 attempt（零执行、零新目录）；
    /// 异参重投返回具名错误 task_id_conflict；有意重跑必须换新 task_id。
    fn execute_task(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_str(args, "task_id")?;
        if !is_valid_task_id(task_id) {
            return Err(format!(
                "task_id 非法（需以字母或数字开头，仅含字母数字 . _ -，长度 ≤64）：{task_id}"
            ));
        }

        // 前置判定一律在占号之前完成：只返回 blocked 回执，不创建任何 attempt 目录
        let cmd = match required_str(args, "cmd") {
            Ok(cmd) => cmd,
            Err(_) => {
                return Ok(blocked_receipt(
                    task_id,
                    "",
                    "command_not_found: 缺少 cmd（必填字符串）".to_string(),
                ))
            }
        };
        let cwd = match args.get("cwd").and_then(Value::as_str).map(str::trim) {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => std::env::current_dir().map_err(|e| format!("获取当前目录失败：{e}"))?,
        };
        if !cwd.is_dir() {
            return Ok(blocked_receipt(
                task_id,
                cmd,
                format!("cwd_not_found: cwd 不是目录：{}", cwd.display()),
            ));
        }
        let timeout_secs = args
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .unwrap_or(120);
        if !(1..=600).contains(&timeout_secs) {
            return Err("timeout_secs 必须在 1..=600 之间".to_string());
        }

        // 幂等签名：同 task_id 重投必须 cmd / cwd / timeout_secs 完全一致
        let canonical_cwd = fs::canonicalize(&cwd).unwrap_or_else(|_| cwd.clone());
        let signature = json!({
            "task_id": task_id,
            "cmd": cmd,
            "cwd": canonical_cwd.to_string_lossy(),
            "timeout_secs": timeout_secs
        });

        let task_root = self.runs_dir.join(&task_id);
        if let Some((_, existing)) = latest_attempt_dir(&task_root) {
            return self.replay_existing(task_id, &existing, &signature, timeout_secs);
        }

        fs::create_dir_all(&task_root).map_err(|e| format!("创建运行目录失败：{e}"))?;
        let attempt = 1u64;
        let run_dir = task_root.join(format!("attempt-{attempt}"));
        // 独占创建仍是并发闸门：没抢到的调用方按重投处理，等待并复用胜者回执
        match fs::create_dir(&run_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return self.replay_existing(task_id, &run_dir, &signature, timeout_secs);
            }
            Err(error) => return Err(format!("创建尝试目录失败：{error}")),
        }
        write_dispatch(&run_dir, &signature)?;
        let stdout_path = run_dir.join("stdout.txt");
        let stderr_path = run_dir.join("stderr.txt");
        let stdout_file =
            fs::File::create(&stdout_path).map_err(|e| format!("创建 stdout 失败：{e}"))?;
        let stderr_file =
            fs::File::create(&stderr_path).map_err(|e| format!("创建 stderr 失败：{e}"))?;

        let before = git_paths(&cwd);
        let commit = git_output(&cwd, &["rev-parse", "--short", "HEAD"])
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let mut command = shell_command(cmd);
        command
            .current_dir(&cwd)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file));

        let started = Instant::now();
        let deadline = Duration::from_secs(timeout_secs);
        let mut timed_out = false;
        let mut spawn_error: Option<String> = None;
        let exit_status = match command.spawn() {
            Ok(mut child) => loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Some(status),
                    Ok(None) => {
                        if started.elapsed() >= deadline {
                            timed_out = true;
                            let _ = child.kill();
                            let _ = child.wait();
                            break None;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => return Err(format!("等待进程失败：{error}")),
                }
            },
            Err(error) => {
                spawn_error = Some(format!("command_not_found: 启动命令失败：{error}"));
                None
            }
        };
        let duration_ms = started.elapsed().as_millis() as u64;

        let stdout_digest = digest_file(&stdout_path)?;
        let stderr_digest = digest_file(&stderr_path)?;
        let blocked_reason = spawn_error.or_else(|| {
            timed_out.then(|| format!("timeout：超过 {timeout_secs}s 未结束，已终止进程"))
        });
        let exit_code = exit_status.and_then(|status| status.code());
        let status = if blocked_reason.is_some() {
            "blocked"
        } else if exit_code == Some(0) {
            "ok"
        } else {
            "failed"
        };

        let (changed_files, changed_files_truncated) = {
            let after = git_paths(&cwd);
            let mut changed: Vec<String> = after.difference(&before).cloned().collect();
            changed.sort();
            let truncated = changed.len() > 20;
            changed.truncate(20);
            (changed, truncated)
        };

        let handback = json!({
            "task_id": task_id,
            "attempt": attempt,
            "status": status,
            "changed_files": changed_files,
            "changed_files_truncated": changed_files_truncated,
            "cwd": canonical_cwd.to_string_lossy(),
            "timeout_secs": timeout_secs,
            "verify_cmd": cmd,
            "exit_code": exit_code,
            "stdout_digest": stdout_digest,
            "stderr_digest": stderr_digest,
            "commit": commit,
            "duration_ms": duration_ms,
            "blocked_reason": blocked_reason,
            "artifacts": [
                {
                    "kind": "stdout",
                    "locator": format!("runs/{task_id}/attempt-{attempt}/stdout.txt"),
                    "hash": stdout_digest.clone()
                },
                {
                    "kind": "stderr",
                    "locator": format!("runs/{task_id}/attempt-{attempt}/stderr.txt"),
                    "hash": stderr_digest.clone()
                }
            ]
        });
        write_handback(&run_dir, &handback)?;
        Ok(handback)
    }

    /// 同 task_id 重投：参数一致则回放既有回执，不一致则具名拒绝
    fn replay_existing(
        &self,
        task_id: &str,
        attempt_dir: &Path,
        signature: &Value,
        timeout_secs: u64,
    ) -> Result<Value, String> {
        let dispatch_path = attempt_dir.join("dispatch.json");
        let stored = match read_json(&dispatch_path) {
            Ok(stored) => stored,
            Err(_) => wait_for_json(&dispatch_path, Duration::from_secs(5)).ok_or_else(|| {
                format!(
                    "task_id_conflict: task_id `{task_id}` 已有运行记录但缺少派单签名（dispatch.json），无法证明参数一致；重跑请使用新 task_id"
                )
            })?,
        };
        if &stored != signature {
            return Err(format!(
                "task_id_conflict: task_id `{task_id}` 已存在且派单参数不一致（cmd / cwd / timeout_secs 必须完全一致）；重跑请使用新 task_id"
            ));
        }

        let handback_path = attempt_dir.join("handback.json");
        if let Ok(handback) = read_json(&handback_path) {
            return Ok(handback);
        }
        wait_for_json(&handback_path, Duration::from_secs(timeout_secs + 5)).ok_or_else(|| {
            format!("task_in_progress: task_id `{task_id}` 的既有尝试仍在执行中，请稍后重试")
        })
    }

    /// 按 task_id 读回回执：默认最新一次，include_history 时给出全部尝试
    fn receipt_payload(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_str(args, "task_id")?;
        if !is_valid_task_id(task_id) {
            return Err(format!("task_id 非法：{task_id}"));
        }
        let include_history = args
            .get("include_history")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let records = collect_attempts(&self.runs_dir.join(task_id));
        let (_, latest) = records
            .last()
            .ok_or_else(|| format!("回执不存在：{task_id}"))?;

        if include_history {
            let history: Vec<&Value> = records.iter().map(|(_, record)| record).collect();
            Ok(json!({
                "task_id": task_id,
                "attempts": records.len(),
                "latest": latest,
                "history": history
            }))
        } else {
            Ok(latest.clone())
        }
    }
}

/// 写派单签名：作为“同 task_id 是否同参”的唯一判据，先于执行落盘
fn write_dispatch(run_dir: &Path, signature: &Value) -> Result<(), String> {
    let path = run_dir.join("dispatch.json");
    let text = serde_json::to_string_pretty(signature)
        .map_err(|e| format!("序列化派单签名失败：{e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入派单签名失败：{e}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("解析 {} 失败：{e}", path.display()))
}

/// 等待文件出现且可解析：并发重投落败的一方据此等待胜者先落盘签名/回执
fn wait_for_json(path: &Path, timeout: Duration) -> Option<Value> {
    let started = Instant::now();
    loop {
        if let Ok(value) = read_json(path) {
            return Some(value);
        }
        if started.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 取该 task_id 下序号最大的 attempt 目录，用于重投回放
fn latest_attempt_dir(root: &Path) -> Option<(u64, PathBuf)> {
    let mut latest: Option<(u64, PathBuf)> = None;
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(number) = name
                .strip_prefix("attempt-")
                .and_then(|rest| rest.parse::<u64>().ok())
            else {
                continue;
            };
            if latest.as_ref().map(|(seen, _)| number > *seen).unwrap_or(true) {
                latest = Some((number, entry.path()));
            }
        }
    }
    latest
}

/// 未占号即拒绝时的回执：attempt 为 null，不落盘、不产生证据
fn blocked_receipt(task_id: &str, cmd: &str, reason: String) -> Value {
    json!({
        "task_id": task_id,
        "attempt": null,
        "status": "blocked",
        "changed_files": [],
        "verify_cmd": cmd,
        "exit_code": null,
        "stdout_digest": null,
        "stderr_digest": null,
        "commit": null,
        "duration_ms": 0,
        "blocked_reason": reason,
        "artifacts": []
    })
}

/// 读取某 task_id 下全部 attempt-N/handback.json，按尝试序号升序
fn collect_attempts(root: &Path) -> Vec<(u64, Value)> {
    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(number) = name
                .strip_prefix("attempt-")
                .and_then(|rest| rest.parse::<u64>().ok())
            else {
                continue;
            };
            let path = entry.path().join("handback.json");
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<Value>(&raw) {
                    records.push((number, record));
                }
            }
        }
    }
    records.sort_by_key(|(number, _)| *number);
    records
}

fn required_str<'a>(args: &'a Value, field: &str) -> Result<&'a str, String> {
    args.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("缺少 {field}（必填字符串）"))
}

fn is_valid_task_id(value: &str) -> bool {
    if value.len() > 64 {
        return false;
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn shell_command(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
}

fn digest_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).map_err(|e| format!("读取证据失败：{e}"))?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn write_handback(run_dir: &Path, handback: &Value) -> Result<(), String> {
    let path = run_dir.join("handback.json");
    let text =
        serde_json::to_string_pretty(handback).map_err(|e| format!("序列化回执失败：{e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入回执失败：{e}"))
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 取工作区变更路径快照，用于对比执行前后新增的改动
fn git_paths(cwd: &Path) -> HashSet<String> {
    git_output(cwd, &["status", "--porcelain"])
        .map(|raw| {
            raw.lines()
                .filter_map(|line| line.get(3..))
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// 运行目录默认放在可执行文件旁，保证回执里的 locator 始终是相对路径
fn default_runs_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("runs")
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
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置，以及最近一次 initialize 的 clientInfo（仅审计弱识别，不构成身份）；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "task.execute",
            "description": "接单执行：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、changed_files 是否截断）；AW 不定义判据、不做验收",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "派单标识（字母或数字开头，仅含字母数字 . _ -，≤64）" },
                    "cmd": { "type": "string", "description": "派单方指定的执行/验证命令" },
                    "cwd": { "type": "string", "description": "工作目录，默认进程当前目录；回执中的 locator 始终为相对路径" },
                    "timeout_secs": { "type": "integer", "minimum": 1, "maximum": 600, "description": "超时秒数，默认 120；超时记 blocked 并给出原因" }
                },
                "required": ["task_id", "cmd"],
                "additionalProperties": false
            }
        },
        {
            "name": "receipt.get",
            "description": "按 task_id 读回执行回执：默认返回最新一次；include_history=true 时返回 attempts 次数与全部历史（每次 exit_code 与耗时）",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "task.execute 使用的派单标识" },
                    "include_history": { "type": "boolean", "description": "是否返回全部尝试记录，默认 false" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }
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
