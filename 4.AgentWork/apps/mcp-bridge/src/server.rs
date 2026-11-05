// aw-mcp-bridge 方法分发：只读 health/status + 执行层 task.execute/receipt.get
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "aw-mcp-bridge";

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
            "server": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "buildTime": BUILD_TIME,
                "desktopBuildTime": DESKTOP_BUILD_TIME
            }
        })
    }

    /// 接单执行：命令与判据由派单方给出，AW 只运行并产出可审计回执
    fn execute_task(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_str(args, "task_id")?;
        if !is_valid_task_id(task_id) {
            return Err(format!(
                "task_id 非法（需以字母或数字开头，仅含字母数字 . _ -，长度 ≤64）：{task_id}"
            ));
        }
        let cmd = required_str(args, "cmd")?;
        let cwd = match args.get("cwd").and_then(Value::as_str).map(str::trim) {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => std::env::current_dir().map_err(|e| format!("获取当前目录失败：{e}"))?,
        };
        if !cwd.is_dir() {
            return Err(format!("cwd 不是目录：{}", cwd.display()));
        }
        let timeout_secs = args
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .unwrap_or(120);
        if !(1..=600).contains(&timeout_secs) {
            return Err("timeout_secs 必须在 1..=600 之间".to_string());
        }

        let task_root = self.runs_dir.join(&task_id);
        fs::create_dir_all(&task_root).map_err(|e| format!("创建运行目录失败：{e}"))?;
        // 同一 task_id 重跑追加为新的一次尝试，历史回执与日志一律保留
        let attempt = next_attempt(&task_root);
        let run_dir = task_root.join(format!("attempt-{attempt}"));
        fs::create_dir_all(&run_dir).map_err(|e| format!("创建尝试目录失败：{e}"))?;
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
        let mut child = command.spawn().map_err(|e| format!("启动命令失败：{e}"))?;
        let deadline = Duration::from_secs(timeout_secs);
        let mut timed_out = false;
        let exit_status = loop {
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
        };
        let duration_ms = started.elapsed().as_millis() as u64;

        let stdout_digest = digest_file(&stdout_path)?;
        let stderr_digest = digest_file(&stderr_path)?;
        let exit_code = exit_status.and_then(|status| status.code());
        let status = if timed_out {
            "blocked"
        } else if exit_code == Some(0) {
            "ok"
        } else {
            "failed"
        };
        let blocked_reason = timed_out.then(|| {
            format!("timeout：超过 {timeout_secs}s 未结束，已终止进程")
        });

        let changed_files = {
            let after = git_paths(&cwd);
            let mut changed: Vec<String> = after.difference(&before).cloned().collect();
            changed.sort();
            changed.truncate(20);
            changed
        };

        let handback = json!({
            "task_id": task_id,
            "attempt": attempt,
            "status": status,
            "changed_files": changed_files,
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

/// 同一 task_id 的下一次尝试序号；历史目录即事实来源，不做覆盖
fn next_attempt(root: &Path) -> u64 {
    collect_attempts(root)
        .last()
        .map(|(number, _)| *number)
        .unwrap_or(0)
        + 1
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
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "task.execute",
            "description": "接单执行：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执；AW 不定义判据、不做验收",
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
