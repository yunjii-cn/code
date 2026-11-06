// aw-mcp-bridge 方法分发：只读 health/status + 执行层 task.execute/receipt.get
use serde_json::{json, Value};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
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
            "runs.list" => self.runs_list(&arguments),
            "runs.prune" => {
                // 部分失败必须让调用方看到 isError=true：不许把"删了一半"包装成成功
                return match self.runs_prune(&arguments) {
                    Ok(payload) => {
                        let is_error = payload.get("status").and_then(Value::as_str)
                            == Some("partial");
                        tool_result(&payload, is_error)
                    }
                    Err(error) => tool_result(&json!({ "error": error }), true),
                };
            }
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
        let received = chrono::Utc::now();
        let record = json!({
            "clientInfo": client_info,
            "receivedAt": received.to_rfc3339()
        });
        let session_dir = self.runs_dir.join(SESSION_DIR);
        if fs::create_dir_all(&session_dir).is_err() {
            return;
        }
        let text = match serde_json::to_string_pretty(&record) {
            Ok(text) => text,
            Err(_) => return,
        };
        // 每次 initialize 一份独立文件：单文件覆盖写会让多 bridge 共享 runs/ 时静默丢掉先前记录
        static INITIALIZE_SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = INITIALIZE_SEQ.fetch_add(1, Ordering::Relaxed);
        let file_name = format!(
            "initialize-{}-{}-{}.json",
            std::process::id(),
            received.timestamp_millis(),
            seq
        );
        // tmp + rename：读取方只会看到完整 JSON，不会看到写了一半的记录
        let tmp_path = session_dir.join(format!("{file_name}.tmp"));
        if fs::write(&tmp_path, text).is_err() {
            return;
        }
        if fs::rename(&tmp_path, session_dir.join(&file_name)).is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
    }

    /// 只读聚合 initialize 审计：每次 initialize 一份，按 receivedAt 升序；含旧版单文件
    fn session_audit(&self) -> Value {
        let session_dir = self.runs_dir.join(SESSION_DIR);
        let mut records: Vec<Value> = Vec::new();
        if let Ok(entries) = fs::read_dir(&session_dir) {
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
                let Ok(mut record) = serde_json::from_str::<Value>(&raw) else {
                    continue;
                };
                if let Some(object) = record.as_object_mut() {
                    // source 只是文件名，便于对账；不构成身份
                    object.insert("source".to_string(), json!(name));
                }
                records.push(record);
            }
        }
        records.sort_by(|left, right| {
            left.get("receivedAt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(right.get("receivedAt").and_then(Value::as_str).unwrap_or(""))
        });
        let total = records.len() as u64;
        let latest = records.last().cloned().unwrap_or(Value::Null);
        json!({
            "clientInfo": latest.get("clientInfo").cloned().unwrap_or(Value::Null),
            "receivedAt": latest.get("receivedAt").cloned().unwrap_or(Value::Null),
            "total": total,
            "records": records
        })
    }

    /// 只读枚举 runs 目录：不写盘、不创建目录，排除 `_session` 等预留目录
    ///
    /// 定位：目录枚举只是磁盘现状的镜像，**不是执行状态的权威来源**；
    /// 判定某次派单的结果必须用 `receipt.get`。
    fn runs_list(&self, args: &Value) -> Result<Value, String> {
        let (offset, limit) = pagination_args(args)?;
        let exists = self.runs_dir.is_dir();
        let mut task_ids: Vec<String> = Vec::new();
        if exists {
            let entries = fs::read_dir(&self.runs_dir)
                .map_err(|e| format!("读取 runs 目录失败：{e}"))?;
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                // 预留目录（_session 等以 _ 开头）与非法 task_id 一律不当作任务
                if name.starts_with('_') || !is_valid_task_id(&name) {
                    continue;
                }
                task_ids.push(name);
            }
        }
        task_ids.sort();
        let total_tasks = task_ids.len() as u64;
        // total 是 attempt 目录总数、不是任务数：必须等于逐条 attempts 之和，且不受分页影响
        let total: u64 = task_ids
            .iter()
            .map(|task_id| attempt_numbers(&self.runs_dir.join(task_id)).len() as u64)
            .sum();
        let tasks: Vec<Value> = task_ids
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|task_id| self.task_summary(task_id))
            .collect();

        Ok(json!({
            "state": if !exists { "missing" } else if total_tasks == 0 { "empty" } else { "ready" },
            "total": total,
            "total_tasks": total_tasks,
            "offset": offset,
            "limit": limit,
            "tasks": tasks
        }))
    }

    /// 证据清理：默认 dry-run、零写入；只有 dry_run=false + confirm=true 才真删。
    /// 规则（契约 §8）：只删非最新 attempt；年龄只认 finishedAt；缺字段一律不删。
    fn runs_prune(&self, args: &Value) -> Result<Value, String> {
        let older_than_days = match args.get("older_than_days") {
            Some(value) => value.as_u64().ok_or_else(|| {
                "invalid_retention: older_than_days 必须是 1..=3650 的整数".to_string()
            })?,
            None => {
                return Err(
                    "invalid_retention: 缺少 older_than_days（不提供默认阈值：没写阈值不能变成按默认值删）"
                        .to_string(),
                )
            }
        };
        if !(1..=3650).contains(&older_than_days) {
            return Err("invalid_retention: older_than_days 必须在 1..=3650 之间".to_string());
        }
        let dry_run = match args.get("dry_run") {
            None => true,
            Some(value) => value
                .as_bool()
                .ok_or_else(|| "invalid_retention: dry_run 必须是布尔值".to_string())?,
        };
        let confirm = match args.get("confirm") {
            None => false,
            Some(value) => value.as_bool().ok_or_else(|| {
                "confirm_required: confirm 必须是布尔值；真删需要显式 confirm=true".to_string()
            })?,
        };
        if !dry_run && !confirm {
            return Err(
                "confirm_required: 真删必须同时显式 dry_run=false 与 confirm=true；本轮未删除任何目录，也未写审计"
                    .to_string(),
            );
        }
        let limit = match args.get("limit") {
            None => 100,
            Some(value) => value.as_u64().ok_or_else(|| {
                "invalid_retention: limit 必须是 1..=100 的整数".to_string()
            })?,
        };
        if !(1..=100).contains(&limit) {
            return Err("invalid_retention: limit 必须在 1..=100 之间".to_string());
        }
        let status_filter = match args.get("status") {
            None => None,
            Some(value) => {
                let array = value.as_array().ok_or_else(|| {
                    "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked"
                        .to_string()
                })?;
                if array.is_empty() {
                    return Err(
                        "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked"
                            .to_string(),
                    );
                }
                let mut parsed: Vec<String> = Vec::new();
                for item in array {
                    let Some(text) = item.as_str() else {
                        return Err(
                            "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked"
                                .to_string(),
                        );
                    };
                    if !matches!(text, "ok" | "failed" | "blocked") {
                        return Err(
                            "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked"
                                .to_string(),
                        );
                    }
                    if !parsed.iter().any(|existing| existing == text) {
                        parsed.push(text.to_string());
                    }
                }
                Some(parsed)
            }
        };
        let status_filter_json = match &status_filter {
            Some(values) => json!(values),
            None => Value::Null,
        };

        let evaluated_at = now_stamp();
        let now = chrono::Utc::now();
        let (candidates, skipped, task_count) =
            self.prune_candidates(older_than_days, now, status_filter.as_deref());
        let total_candidates = candidates.len() as u64;
        let truncated = total_candidates > limit;
        let batch: Vec<Value> = candidates.into_iter().take(limit as usize).collect();

        if dry_run {
            let state = if !self.runs_dir.is_dir() {
                "missing"
            } else if task_count == 0 {
                "empty"
            } else {
                "ready"
            };
            return Ok(json!({
                "dry_run": true,
                "state": state,
                "older_than_days": older_than_days,
                "status_filter": status_filter_json,
                "evaluated_at": evaluated_at,
                "limit": limit,
                "candidates_total": total_candidates,
                "truncated": truncated,
                "candidates": batch,
                "skipped_total": skipped.len() as u64,
                "skipped": skipped
            }));
        }

        if batch.is_empty() {
            // 没有删除发生：不写审计，也不创建任何目录（含 .prune-trash）
            return Ok(json!({
                "dry_run": false,
                "status": "ok",
                "older_than_days": older_than_days,
                "status_filter": status_filter_json,
                "evaluated_at": evaluated_at,
                "limit": limit,
                "candidates_total": total_candidates,
                "truncated": truncated,
                "deleted_total": 0,
                "failed_total": 0,
                "deleted": [],
                "failed": []
            }));
        }

        let batch_id = format!(
            "prune-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            std::process::id()
        );
        let plan = json!({
            "at": now_stamp(),
            "phase": "plan",
            "batch_id": batch_id,
            "older_than_days": older_than_days,
            "status_filter": status_filter_json,
            "limit": limit,
            "candidate_count": batch.len() as u64,
            "candidates": batch.clone()
        });
        // 先记账后删除：plan 行写不进去就一个目录都不删
        self.append_prune_record(&plan)
            .map_err(|error| format!("{error}；本轮未删除任何目录"))?;

        let mut deleted: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        for candidate in &batch {
            let task_id = candidate["task_id"].as_str().unwrap_or_default().to_string();
            let attempt = candidate["attempt"].as_u64().unwrap_or_default();
            match self.delete_attempt_dir(&task_id, attempt, &batch_id) {
                Ok(()) => deleted.push(candidate.clone()),
                Err(error) => failed.push(json!({
                    "task_id": task_id,
                    "attempt": attempt,
                    "path": candidate["path"],
                    "error": error
                })),
            }
        }
        let deleted_total = deleted.len() as u64;
        let failed_total = failed.len() as u64;
        let status = if failed_total == 0 { "ok" } else { "partial" };
        let result = json!({
            "at": now_stamp(),
            "phase": "result",
            "batch_id": batch_id,
            "older_than_days": older_than_days,
            "status_filter": status_filter_json,
            "limit": limit,
            "deleted": deleted.clone(),
            "failed": failed.clone(),
            "deleted_total": deleted_total,
            "failed_total": failed_total
        });
        if let Err(error) = self.append_prune_record(&result) {
            return Err(format!(
                "prune_audit_failed: 已删除 {deleted_total} 个、失败 {failed_total} 个；结果行写入 prune.jsonl 失败：{error}"
            ));
        }

        Ok(json!({
            "dry_run": false,
            "status": status,
            "batch_id": batch_id,
            "older_than_days": older_than_days,
            "status_filter": status_filter_json,
            "evaluated_at": evaluated_at,
            "limit": limit,
            "candidates_total": total_candidates,
            "truncated": truncated,
            "deleted_total": deleted_total,
            "failed_total": failed_total,
            "deleted": deleted,
            "failed": failed
        }))
    }

    /// 计算清理候选：只认 finishedAt、只碰非最新 attempt、不跟随符号链接
    fn prune_candidates(
        &self,
        older_than_days: u64,
        now: chrono::DateTime<chrono::Utc>,
        status_filter: Option<&[String]>,
    ) -> (Vec<Value>, Vec<Value>, usize) {
        let mut candidates: Vec<Value> = Vec::new();
        let mut skipped: Vec<Value> = Vec::new();
        if !self.runs_dir.is_dir() {
            return (candidates, skipped, 0);
        }
        let mut task_ids: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.runs_dir) {
            for entry in entries.flatten() {
                // file_type 不跟随符号链接：链接不会被当作任务，避免删到 runs 之外的路径
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if !file_type.is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('_') || name.starts_with('.') || !is_valid_task_id(&name) {
                    continue;
                }
                task_ids.push(name);
            }
        }
        task_ids.sort();
        let task_count = task_ids.len();

        for task_id in task_ids {
            let root = self.runs_dir.join(&task_id);
            let numbers = attempt_numbers(&root);
            let Some(latest) = numbers.last().copied() else {
                continue;
            };
            // 最新 attempt 永不进入候选：这是"每个 task 至少保留一次 attempt"的落地方式
            skipped.push(json!({
                "task_id": task_id.clone(),
                "attempt": latest,
                "reason": "latest_attempt_retained"
            }));
            for number in numbers.into_iter().filter(|number| *number < latest) {
                let dir = root.join(format!("attempt-{number}"));
                let safe = fs::symlink_metadata(&dir)
                    .map(|meta| meta.file_type().is_dir())
                    .unwrap_or(false);
                if !safe {
                    skipped.push(json!({
                        "task_id": task_id.clone(),
                        "attempt": number,
                        "reason": "unsafe_attempt_path"
                    }));
                    continue;
                }
                let dispatch = match read_json(&dir.join("dispatch.json")) {
                    Ok(record) => record,
                    Err(_) => {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "missing_dispatch"
                        }));
                        continue;
                    }
                };
                let handback = match read_json(&dir.join("handback.json")) {
                    Ok(record) => record,
                    Err(_) => {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "unfinished_or_missing_handback"
                        }));
                        continue;
                    }
                };
                let Some(finished_at) = handback.get("finishedAt").and_then(Value::as_str) else {
                    skipped.push(json!({
                        "task_id": task_id.clone(),
                        "attempt": number,
                        "reason": "age_unknown"
                    }));
                    continue;
                };
                let parsed = match chrono::DateTime::parse_from_rfc3339(finished_at) {
                    Ok(stamp) => stamp.with_timezone(&chrono::Utc),
                    Err(_) => {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "invalid_finished_at"
                        }));
                        continue;
                    }
                };
                if parsed > now {
                    skipped.push(json!({
                        "task_id": task_id.clone(),
                        "attempt": number,
                        "reason": "future_timestamp"
                    }));
                    continue;
                }
                let age_days = now.signed_duration_since(parsed).num_days();
                if age_days < older_than_days as i64 {
                    skipped.push(json!({
                        "task_id": task_id.clone(),
                        "attempt": number,
                        "reason": "not_old_enough"
                    }));
                    continue;
                }
                let status = handback.get("status").and_then(Value::as_str);
                if let Some(filter) = status_filter {
                    let Some(value) = status else {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "status_unknown"
                        }));
                        continue;
                    };
                    if !matches!(value, "ok" | "failed" | "blocked") {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "status_unknown"
                        }));
                        continue;
                    }
                    if !filter.iter().any(|allowed| allowed == value) {
                        skipped.push(json!({
                            "task_id": task_id.clone(),
                            "attempt": number,
                            "reason": "status_mismatch"
                        }));
                        continue;
                    }
                }
                candidates.push(json!({
                    "task_id": task_id.clone(),
                    "attempt": number,
                    "path": format!("runs/{task_id}/attempt-{number}"),
                    "status": status,
                    "dispatchedAt": dispatch.get("dispatchedAt").cloned().unwrap_or(Value::Null),
                    "finishedAt": finished_at,
                    "age_days": age_days
                }));
            }
        }
        (candidates, skipped, task_count)
    }

    /// 整目录搬进 .prune-trash 后再删：重命名失败则原样不动，避免"只删了一半文件"
    fn delete_attempt_dir(
        &self,
        task_id: &str,
        attempt: u64,
        batch_id: &str,
    ) -> Result<(), String> {
        let source = self.runs_dir.join(task_id).join(format!("attempt-{attempt}"));
        let stage_root = self.runs_dir.join(".prune-trash").join(batch_id);
        fs::create_dir_all(&stage_root)
            .map_err(|error| format!("创建暂存目录失败：{error}"))?;
        let staged = stage_root.join(format!("{task_id}-attempt-{attempt}"));
        fs::rename(&source, &staged).map_err(|error| {
            format!("整体移动失败（attempt 原样未动）：{error}")
        })?;
        fs::remove_dir_all(&staged).map_err(|error| {
            format!("已整链移至 runs/.prune-trash/{batch_id}，但删除暂存失败（数据未丢）：{error}")
        })?;
        // 本批暂存目录空了就顺手移除；失败不影响结果
        let _ = fs::remove_dir(&stage_root);
        Ok(())
    }

    /// 审计落盘：append-only JSONL，写一行并 fsync；失败必须让调用方看到
    fn append_prune_record(&self, record: &Value) -> Result<(), String> {
        use std::io::Write;
        let path = self.runs_dir.join("prune.jsonl");
        let line = serde_json::to_string(record)
            .map_err(|error| format!("prune_audit_failed: 序列化审计记录失败：{error}"))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("prune_audit_failed: 打开 runs/prune.jsonl 失败：{error}"))?;
        writeln!(file, "{line}")
            .map_err(|error| format!("prune_audit_failed: 写入 runs/prune.jsonl 失败：{error}"))?;
        file.sync_all()
            .map_err(|error| format!("prune_audit_failed: 同步 runs/prune.jsonl 失败：{error}"))?;
        Ok(())
    }

    /// 单个任务的只读摘要：只给状态与 env 键名，不回显 env 值
    fn task_summary(&self, task_id: &str) -> Value {
        let root = self.runs_dir.join(task_id);
        let numbers = attempt_numbers(&root);
        let latest = numbers.last().copied();

        let mut handback = false;
        let mut status = Value::Null;
        let mut exit_code = Value::Null;
        let mut duration_ms = Value::Null;
        let mut blocked_reason = Value::Null;
        let mut env_keys: Vec<String> = Vec::new();
        if let Some(number) = latest {
            let run_dir = root.join(format!("attempt-{number}"));
            if let Ok(record) = read_json(&run_dir.join("handback.json")) {
                handback = true;
                status = record.get("status").cloned().unwrap_or(Value::Null);
                exit_code = record.get("exit_code").cloned().unwrap_or(Value::Null);
                duration_ms = record.get("duration_ms").cloned().unwrap_or(Value::Null);
                blocked_reason = record.get("blocked_reason").cloned().unwrap_or(Value::Null);
            }
            if let Ok(dispatch) = read_json(&run_dir.join("dispatch.json")) {
                if let Some(map) = dispatch.get("env").and_then(Value::as_object) {
                    env_keys = map.keys().cloned().collect();
                }
            }
        }

        json!({
            "task_id": task_id,
            "attempts": numbers.len(),
            "latest_attempt": latest,
            "handback": handback,
            "status": status,
            "exit_code": exit_code,
            "duration_ms": duration_ms,
            "blocked_reason": blocked_reason,
            "env_keys": env_keys
        })
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

        // env 透传：只在子进程环境里追加/覆盖同名变量，不做任何推断；非法键值在占号之前拒绝
        let env_pairs = parse_env(args)?;
        // path_prepend：显式前插目录，继承 PATH 永远保留；非法形态同样在占号之前拒绝
        let path_prepend = parse_path_prepend(args)?;

        // 幂等签名：同 task_id 重投必须 cmd / cwd / timeout_secs / env / path_prepend 完全一致；
        // env 只存值摘要，不把可能的密钥明文写进 dispatch.json
        let canonical_cwd = fs::canonicalize(&cwd).unwrap_or_else(|_| cwd.clone());
        let signature = json!({
            "task_id": task_id,
            "cmd": cmd,
            "cwd": canonical_cwd.to_string_lossy(),
            "timeout_secs": timeout_secs,
            "env": env_signature_value(&env_pairs),
            "path_prepend": path_prepend.clone()
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
        // 显式 env 覆盖同名继承变量；未列出的变量照常继承父进程环境
        if !env_pairs.is_empty() {
            command.envs(
                env_pairs
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            );
        }
        // path_prepend 只做前插：数组按字面顺序在前，继承 PATH 原样接在后；分隔符按平台
        if !path_prepend.is_empty() {
            let separator = if cfg!(windows) { ';' } else { ':' };
            let mut joined = OsString::new();
            for dir in &path_prepend {
                if !joined.is_empty() {
                    joined.push(separator.to_string());
                }
                joined.push(dir);
            }
            if let Some(inherited) = std::env::var_os("PATH").filter(|value| !value.is_empty()) {
                if !joined.is_empty() {
                    joined.push(separator.to_string());
                }
                joined.push(inherited);
            }
            command.env("PATH", joined);
        }

        let started = Instant::now();
        let deadline = Duration::from_secs(timeout_secs);
        let mut timed_out = false;
        let mut cleanup_outcome = None;
        let mut spawn_error: Option<String> = None;
        let mut supervisor = process_scope::Supervisor::new();
        supervisor.configure(&mut command);
        let exit_status = match command.spawn() {
            Ok(mut child) => {
                supervisor.attach(&mut child);
                if let Some(error) = supervisor.attach_error().map(str::to_string) {
                    // 恢复执行失败：子进程已被终止，不能把"从未执行"记成命令结果
                    let _ = child.wait();
                    spawn_error = Some(error);
                    None
                } else {
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => break Some(status),
                            Ok(None) => {
                                if started.elapsed() >= deadline {
                                    timed_out = true;
                                    cleanup_outcome = Some(supervisor.terminate(&mut child));
                                    break None;
                                }
                                std::thread::sleep(Duration::from_millis(20));
                            }
                            Err(error) => return Err(format!("等待进程失败：{error}")),
                        }
                    }
                }
            }
            Err(error) => {
                spawn_error = Some(format!("command_not_found: 启动命令失败：{error}"));
                None
            }
        };
        let duration_ms = started.elapsed().as_millis() as u64;

        let stdout_digest = digest_file(&stdout_path)?;
        let stderr_digest = digest_file(&stderr_path)?;
        let blocked_reason = spawn_error
            .or_else(|| timed_out.then(|| timeout_reason(timeout_secs, cleanup_outcome)));
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
            // 该 attempt 结束并落盘回执的时刻；ok / failed / blocked 一律写实际结束时间
            "finishedAt": now_stamp(),
            "status": status,
            "changed_files": changed_files,
            "changed_files_truncated": changed_files_truncated,
            "cwd": canonical_cwd.to_string_lossy(),
            "timeout_secs": timeout_secs,
            "env_keys": env_pairs.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(),
            // 只回目录名列表，不回任何 env 值
            "path_prepend": path_prepend,
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
        // 升级前的签名没有 env 字段：其语义等价于 env 为空，补齐后再比对（非空 env 仍会判冲突）
        let mut stored = stored;
        if let Some(object) = stored.as_object_mut() {
            // dispatchedAt 是时间事实、不是派单参数：必须剔除，否则任何一次重投都会因时间不同误判冲突
            object.remove("dispatchedAt");
            if !object.contains_key("env") {
                object.insert("env".to_string(), json!({}));
            }
            // 升级前的签名没有 path_prepend：其语义等价于空数组，补齐后再比对
            if !object.contains_key("path_prepend") {
                object.insert("path_prepend".to_string(), json!([]));
            }
        }
        if &stored != signature {
            return Err(format!(
                "task_id_conflict: task_id `{task_id}` 已存在且派单参数不一致（cmd / cwd / timeout_secs / env / path_prepend 必须完全一致）；重跑请使用新 task_id"
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

/// 写派单记录：签名 + 落盘时间戳 `dispatchedAt`（时间事实不参与同参比对，见 replay_existing）
fn write_dispatch(run_dir: &Path, signature: &Value) -> Result<(), String> {
    let mut record = signature.clone();
    if let Some(object) = record.as_object_mut() {
        object.insert("dispatchedAt".to_string(), json!(now_stamp()));
    }
    let path = run_dir.join("dispatch.json");
    let text = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("序列化派单签名失败：{e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入派单签名失败：{e}"))
}

/// 证据时间戳统一口径：RFC3339 / UTC / 秒级（`Z` 结尾），`dispatchedAt` 与 `finishedAt` 共用
fn now_stamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
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

/// 超时清理只承诺"受控集合"（POSIX 进程组 / Windows Job Object）内无存活进程。
/// 逃逸后代（POSIX `setsid`、Windows `BREAKAWAY` / 加入作业失败）不在判据内，`#3` 仍为 `NOT_PROVEN`。
mod process_scope {
    /// 受控集合内的清理结果。`Unavailable` 表示机制未建立/无法观测，绝不等于"已清理"。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum CleanupOutcome {
        Cleaned,
        StillAlive,
        Unavailable,
    }

    #[cfg(unix)]
    mod imp {
        use super::CleanupOutcome;
        use std::os::unix::process::CommandExt;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};

        const TERM_GRACE: Duration = Duration::from_millis(500);
        const CLEANUP_DEADLINE: Duration = Duration::from_secs(3);

        pub(crate) struct Supervisor {
            pgid: Option<i32>,
        }

        impl Supervisor {
            pub(crate) fn new() -> Self {
                Self { pgid: None }
            }

            /// 在 exec 之前建组：`process_group(0)` 让子进程成为新进程组的组长，
            /// 因此不存在 Windows 那种"spawn 之后再加入"的竞态窗口。
            pub(crate) fn configure(&mut self, command: &mut Command) {
                command.process_group(0);
            }

            pub(crate) fn attach(&mut self, child: &mut Child) {
                self.pgid = i32::try_from(child.id()).ok().filter(|pgid| *pgid > 0);
            }

            pub(crate) fn attach_error(&self) -> Option<&str> {
                None
            }

            pub(crate) fn terminate(&mut self, child: &mut Child) -> CleanupOutcome {
                let Some(pgid) = self.pgid else {
                    let _ = child.kill();
                    return CleanupOutcome::Unavailable;
                };
                // 先礼后兵：SIGTERM 给宽限期，再 SIGKILL
                unsafe {
                    libc::kill(-pgid, libc::SIGTERM);
                }
                let grace_deadline = Instant::now() + TERM_GRACE;
                while Instant::now() < grace_deadline && group_exists(pgid) {
                    std::thread::sleep(Duration::from_millis(20));
                }
                if group_exists(pgid) {
                    unsafe {
                        libc::kill(-pgid, libc::SIGKILL);
                    }
                }
                let _ = child.wait();

                // 判据：只查"进程组内是否还有进程"，不查文件是否写盘
                let deadline = Instant::now() + CLEANUP_DEADLINE;
                loop {
                    if !group_exists(pgid) {
                        return CleanupOutcome::Cleaned;
                    }
                    if Instant::now() >= deadline {
                        return CleanupOutcome::StillAlive;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }

        fn group_exists(pgid: i32) -> bool {
            let rc = unsafe { libc::kill(-pgid, 0) };
            if rc == 0 {
                return true;
            }
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ESRCH) => false,
                // EPERM 等：存在但不可信号，按"存在"处理，绝不能当成功
                _ => true,
            }
        }

        #[cfg(test)]
        fn pid_exists(pid: i32) -> bool {
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

        #[cfg(test)]
        mod tests {
            use super::*;
            use std::os::unix::process::CommandExt;
            use std::process::{Command, Stdio};

            fn wait_group_empty(pgid: i32) -> bool {
                for _ in 0..150 {
                    if !group_exists(pgid) {
                        return true;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                false
            }

            // 反假通过：活着但被暂停、不写盘的进程，必须仍被判为"存在"
            #[test]
            fn stopped_process_still_counts_as_alive() {
                let mut command = Command::new("sleep");
                command
                    .arg("30")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0);
                let mut child = command.spawn().expect("spawn sleep");
                let pgid = child.id() as i32;
                unsafe {
                    libc::kill(pgid, libc::SIGSTOP);
                }
                std::thread::sleep(Duration::from_millis(50));
                println!(
                    "[evidence] stopped pgid={pgid} kill(-pgid,0)_exists={}",
                    group_exists(pgid)
                );
                assert!(
                    group_exists(pgid),
                    "SIGSTOP 的进程仍然存在，不能被文件 stat 式判据误判为已退出"
                );
                unsafe {
                    libc::kill(-pgid, libc::SIGKILL);
                }
                let _ = child.wait();
                let emptied = wait_group_empty(pgid);
                println!("[evidence] stopped after_SIGKILL group_empty={emptied}");
                assert!(emptied, "SIGKILL 后进程组应最终清空");
            }

            // 反向：进程已退出，组判据应为空
            #[test]
            fn exited_process_group_has_no_members() {
                let mut command = Command::new("sh");
                command.arg("-c").arg("exit 0").process_group(0);
                let mut child = command.spawn().expect("spawn sh");
                let pgid = child.id() as i32;
                let _ = child.wait();
                let emptied = wait_group_empty(pgid);
                println!("[evidence] exited pgid={pgid} group_empty={emptied}");
                assert!(emptied, "进程退出后进程组应为空");
            }

            // 作用域负例：setsid 逃逸进程不在原进程组内 → 组判据看不到它（仅证明作用域有限）
            #[cfg(target_os = "linux")]
            #[test]
            fn setsid_escape_is_outside_the_observed_group() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-setsid-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let pid_file = dir.join("pid.txt");
                let script = format!(
                    "if command -v setsid >/dev/null 2>&1; then setsid sleep 30 & echo $! > '{}'; else echo skip > '{}'; fi",
                    pid_file.display(),
                    pid_file.display()
                );
                let mut command = Command::new("sh");
                command.arg("-c").arg(script).process_group(0);
                let mut child = command.spawn().expect("spawn setsid fixture");
                let pgid = child.id() as i32;
                let mut content = None;
                for _ in 0..100 {
                    if let Ok(text) = std::fs::read_to_string(&pid_file) {
                        content = Some(text.trim().to_string());
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let _ = child.wait();
                let Some(content) = content else {
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };
                if content == "skip" {
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                }
                let escaped: i32 = content.parse().expect("setsid 子进程 PID");
                println!(
                    "[evidence] setsid escaped_pid={escaped} escaped_alive={} original_group_empty={}",
                    pid_exists(escaped),
                    !group_exists(pgid)
                );
                assert!(pid_exists(escaped), "setsid 逃逸进程应仍存活");
                assert!(
                    !group_exists(pgid),
                    "逃逸进程不在原进程组内：组判据为空，证明作用域仅为受控集合"
                );
                unsafe {
                    libc::kill(escaped, libc::SIGKILL);
                }
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }

    #[cfg(windows)]
    mod imp {
        use super::CleanupOutcome;
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};
        use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
            THREADENTRY32,
        };
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
            TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        use windows_sys::Win32::System::Threading::{
            OpenThread, ResumeThread, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
        };

        const CLEANUP_DEADLINE: Duration = Duration::from_secs(3);

        /// 作业句柄由 bridge 持有；`KILL_ON_JOB_CLOSE` 在最后一个句柄关闭时兜底清理。
        ///
        /// 窗口收窄（v1.20）：子进程以 `CREATE_SUSPENDED` 创建，先 `AssignProcessToJobObject`
        /// 再恢复主线程。恢复之前子进程不执行任何代码，因此不可能在"spawn → Assign"之间
        /// 派生作业外后代；恢复后派生的后代默认继承作业。残余逃逸为 `BREAKAWAY`、
        /// 加入作业失败或嵌套作业策略，仍不可观测，文档与回执必须写明。
        pub(crate) struct Supervisor {
            job: Option<HANDLE>,
            attached: bool,
            suspended: bool,
            attach_error: Option<String>,
        }

        impl Supervisor {
            pub(crate) fn new() -> Self {
                unsafe {
                    let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                    if job.is_null() {
                        return Self {
                            job: None,
                            attached: false,
                            suspended: false,
                            attach_error: None,
                        };
                    }
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    let ok = SetInformationJobObject(
                        job,
                        JobObjectExtendedLimitInformation,
                        &info as *const _ as *const core::ffi::c_void,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                    if ok == 0 {
                        CloseHandle(job);
                        return Self {
                            job: None,
                            attached: false,
                            suspended: false,
                            attach_error: None,
                        };
                    }
                    Self {
                        job: Some(job),
                        attached: false,
                        suspended: false,
                        attach_error: None,
                    }
                }
            }

            pub(crate) fn configure(&mut self, command: &mut Command) {
                command.creation_flags(CREATE_SUSPENDED);
                self.suspended = true;
            }

            pub(crate) fn attach(&mut self, child: &mut Child) {
                if let Some(job) = self.job {
                    let ok =
                        unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) };
                    self.attached = ok != 0;
                }
                if self.suspended {
                    if let Err(error) = resume_primary_thread(child.id()) {
                        let _ = child.kill();
                        self.attach_error = Some(format!(
                            "process_resume_failed: 子进程已挂起创建但无法恢复执行：{error}"
                        ));
                    }
                }
            }

            pub(crate) fn attach_error(&self) -> Option<&str> {
                self.attach_error.as_deref()
            }

            pub(crate) fn terminate(&mut self, child: &mut Child) -> CleanupOutcome {
                let Some(job) = self.job else {
                    let _ = child.kill();
                    return CleanupOutcome::Unavailable;
                };
                if !self.attached {
                    // 加入作业失败：不得假定成员已受控，也不能宣称已清理
                    let _ = child.kill();
                    return CleanupOutcome::Unavailable;
                }
                unsafe {
                    TerminateJobObject(job, 1);
                }
                let _ = child.wait();

                let deadline = Instant::now() + CLEANUP_DEADLINE;
                loop {
                    match active_processes(job) {
                        Some(0) => return CleanupOutcome::Cleaned,
                        Some(_) if Instant::now() < deadline => {
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        Some(_) => return CleanupOutcome::StillAlive,
                        // 查询失败不等于 0：按不可观测处理
                        None => return CleanupOutcome::Unavailable,
                    }
                }
            }

            #[cfg(test)]
            fn active_processes(&self) -> Option<u32> {
                self.job.and_then(active_processes)
            }
        }

        impl Drop for Supervisor {
            fn drop(&mut self) {
                if let Some(job) = self.job.take() {
                    unsafe {
                        CloseHandle(job);
                    }
                }
            }
        }

        fn active_processes(job: HANDLE) -> Option<u32> {
            unsafe {
                let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
                let ok = QueryInformationJobObject(
                    job,
                    JobObjectBasicAccountingInformation,
                    &mut info as *mut _ as *mut core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                    std::ptr::null_mut(),
                );
                (ok != 0).then_some(info.ActiveProcesses)
            }
        }

        /// 恢复以 `CREATE_SUSPENDED` 创建的子进程主线程。
        ///
        /// `std::process::Command` 不返回主线程句柄，因此按进程 ID 用 Toolhelp 快照
        /// 找到唯一的主线程，再 `OpenThread + ResumeThread`。任何一步失败都返回错误，
        /// 由调用方终止该挂起进程并记 `process_resume_failed`，不得让"从未执行"被当成命令结果。
        fn resume_primary_thread(pid: u32) -> Result<(), String> {
            unsafe {
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
                if snapshot == INVALID_HANDLE_VALUE {
                    return Err(format!(
                        "线程快照失败：{}",
                        std::io::Error::last_os_error()
                    ));
                }
                let mut entry: THREADENTRY32 = std::mem::zeroed();
                entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
                let mut thread_id = None;
                let mut ok = Thread32First(snapshot, &mut entry);
                while ok != 0 {
                    if entry.th32OwnerProcessID == pid {
                        thread_id = Some(entry.th32ThreadID);
                        break;
                    }
                    entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
                    ok = Thread32Next(snapshot, &mut entry);
                }
                CloseHandle(snapshot);
                let Some(thread_id) = thread_id else {
                    return Err("未找到子进程主线程".to_string());
                };
                let thread = OpenThread(THREAD_SUSPEND_RESUME, FALSE, thread_id);
                if thread.is_null() {
                    return Err(format!(
                        "打开主线程失败：{}",
                        std::io::Error::last_os_error()
                    ));
                }
                let previous = ResumeThread(thread);
                CloseHandle(thread);
                if previous == u32::MAX {
                    return Err(format!(
                        "恢复主线程失败：{}",
                        std::io::Error::last_os_error()
                    ));
                }
                Ok(())
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use std::process::{Command, Stdio};

            // 反假通过：活着但不写盘的进程，ActiveProcesses 必须 > 0
            #[test]
            fn alive_but_idle_process_is_counted_in_job() {
                let mut supervisor = Supervisor::new();
                assert!(supervisor.job.is_some(), "作业应创建成功");
                let mut command = Command::new("ping");
                command
                    .args(["-n", "30", "127.0.0.1"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let mut child = command.spawn().expect("spawn ping");
                supervisor.attach(&mut child);
                assert!(supervisor.attached, "进程应成功加入作业");
                let before = supervisor.active_processes();
                println!("[evidence] alive_but_idle attached={} ActiveProcesses_before={before:?}", supervisor.attached);
                assert!(
                    before.unwrap_or(0) > 0,
                    "活着但不写盘的进程必须计入 ActiveProcesses"
                );
                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.active_processes();
                println!("[evidence] alive_but_idle terminate={outcome:?} ActiveProcesses_after={after:?}");
                assert_eq!(outcome, CleanupOutcome::Cleaned, "作业内成员应被清空");
                assert_eq!(after, Some(0));
            }

            // 正向：作业内的孙进程也必须被清理，判据打印 ActiveProcesses
            #[test]
            fn job_counts_grandchild_and_empties_after_terminate() {
                let mut supervisor = Supervisor::new();
                assert!(supervisor.job.is_some(), "作业应创建成功");
                let mut command = Command::new("cmd");
                command
                    .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let mut child = command.spawn().expect("spawn cmd");
                supervisor.attach(&mut child);
                assert!(supervisor.attached, "cmd 应成功加入作业");
                let before = supervisor.active_processes();
                println!(
                    "[evidence] grandchild attached={} ActiveProcesses_before={before:?}",
                    supervisor.attached
                );
                assert!(
                    before.unwrap_or(0) > 0,
                    "cmd 派生的孙进程必须计入 ActiveProcesses"
                );
                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.active_processes();
                println!(
                    "[evidence] grandchild terminate={outcome:?} ActiveProcesses_after={after:?}"
                );
                assert_eq!(outcome, CleanupOutcome::Cleaned, "作业内成员应被清空");
                assert_eq!(after, Some(0));
            }

            // 窗口收窄：子进程以挂起态创建，加入作业前不得执行任何代码；
            // 因此"spawn→Assign 之间派生的孙进程不在作业内"这一窗口对后代不再存在。
            #[test]
            fn suspended_start_defers_execution_until_job_assign() {
                let mut supervisor = Supervisor::new();
                assert!(supervisor.job.is_some(), "作业应创建成功");
                let work = tempfile::tempdir().expect("临时目录");
                let marker = work.path().join("started.txt");
                let mut command = Command::new("cmd");
                command
                    .current_dir(work.path())
                    .args([
                        "/C",
                        "echo started> started.txt & ping -n 30 127.0.0.1 > NUL",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn cmd");
                // 挂起期间等待 300ms：进程若已执行，标记文件必然出现
                std::thread::sleep(Duration::from_millis(300));
                assert!(
                    !marker.exists(),
                    "加入作业前子进程不得执行（否则可派生作业外后代）"
                );
                assert_eq!(
                    supervisor.active_processes(),
                    Some(0),
                    "未加入作业前作业内计数必须为 0"
                );
                supervisor.attach(&mut child);
                assert!(
                    supervisor.attach_error().is_none(),
                    "加入作业并恢复执行不得失败"
                );
                assert!(supervisor.attached, "进程应成功加入作业");
                let mut appeared = false;
                for _ in 0..150 {
                    if marker.exists() {
                        appeared = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = supervisor.active_processes();
                println!(
                    "[evidence] suspended_start marker_before_assign=false marker_after_resume={appeared} attached={} ActiveProcesses_before_terminate={before:?}",
                    supervisor.attached
                );
                assert!(appeared, "加入作业并恢复后命令必须开始执行");
                assert!(before.unwrap_or(0) > 0, "恢复后作业内必须有活动进程");
                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.active_processes();
                println!(
                    "[evidence] suspended_start terminate={outcome:?} ActiveProcesses_after={after:?}"
                );
                assert_eq!(outcome, CleanupOutcome::Cleaned, "作业内成员应被清空");
                assert_eq!(after, Some(0));
            }

            // 反向：进程已退出，作业内活动进程数应为 0
            #[test]
            fn exited_process_leaves_job_empty() {
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("cmd");
                command
                    .args(["/C", "ping -n 2 127.0.0.1 > NUL"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let mut child = command.spawn().expect("spawn cmd");
                supervisor.attach(&mut child);
                let _ = child.wait();
                let mut observed = None;
                for _ in 0..100 {
                    if let Some(count) = supervisor.active_processes() {
                        observed = Some(count);
                        if count == 0 {
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                println!("[evidence] exited_process ActiveProcesses_final={observed:?}");
                assert_eq!(observed, Some(0), "进程退出后作业内活动进程数应为 0");
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    mod imp {
        use super::CleanupOutcome;
        use std::process::{Child, Command};

        pub(crate) struct Supervisor;

        impl Supervisor {
            pub(crate) fn new() -> Self {
                Self
            }
            pub(crate) fn configure(&mut self, _command: &mut Command) {}
            pub(crate) fn attach(&mut self, _child: &mut Child) {}

            pub(crate) fn attach_error(&self) -> Option<&str> {
                None
            }
            pub(crate) fn terminate(&mut self, child: &mut Child) -> CleanupOutcome {
                let _ = child.kill();
                CleanupOutcome::Unavailable
            }
        }
    }

    pub(crate) use imp::Supervisor;
}

/// 超时回执措辞按清理结果分流：只有 `Cleaned` 才能说"已清理受控集合"。
/// 状态门已满足（v1.17）：POSIX 四类用例实跑通过 + Windows 三类通过 + 契约版本递增。
/// Windows 逃逸负例仍为 `UNPROVEN`，回执明写 `spawn → Assign` 窗口。
/// 任何分支都不得出现"已终止进程树 / 无残留"。
fn timeout_reason(timeout_secs: u64, outcome: Option<process_scope::CleanupOutcome>) -> String {
    use process_scope::CleanupOutcome;
    match outcome {
        Some(CleanupOutcome::Cleaned) => {
            #[cfg(windows)]
            {
                format!(
                    "timeout：超过 {timeout_secs}s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测"
                )
            }
            #[cfg(not(windows))]
            {
                format!("timeout：超过 {timeout_secs}s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证")
            }
        }
        Some(CleanupOutcome::StillAlive) => format!(
            "timeout：超过 {timeout_secs}s 未结束，已向受控集合发送终止信号，但受控集合内仍有存活进程；逃逸进程是否仍在运行未验证"
        ),
        _ => format!(
            "timeout：超过 {timeout_secs}s 未结束，受控集合未能建立或无法查询，仅向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"
        ),
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn digest_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取证据失败：{e}"))?;
    Ok(digest_bytes(&bytes))
}

/// env 明细：只透传，不做任何推断；返回原样 (键, 值)
fn parse_env(args: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(raw) = args.get("env") else {
        return Ok(Vec::new());
    };
    let Some(map) = raw.as_object() else {
        return Err("invalid_env: env 必须是字符串到字符串的对象".to_string());
    };
    let mut pairs = Vec::with_capacity(map.len());
    for (key, value) in map {
        if key.is_empty() {
            return Err("invalid_env: env 的键不能为空".to_string());
        }
        // fail-closed：PATH 不许整体覆盖。看见就拒绝，不静默忽略，否则调用方会误以为自己的 PATH 生效。
        // 大小写变体一并拒绝：Windows 下 PATH / Path / path 指向同一变量。
        if key.eq_ignore_ascii_case("PATH") {
            return Err(format!(
                "invalid_env: 不接受 env.{key}，PATH 不可整体覆盖（继承的 PATH 始终保留）；如需追加目录请等显式 path_prepend 字段，env 不承担该职责"
            ));
        }
        if key.contains('=') {
            return Err(format!("invalid_env: env 的键不能含 '='：{key}"));
        }
        if key.contains('\0') {
            return Err("invalid_env: env 的键不能含空字节".to_string());
        }
        let Some(text) = value.as_str() else {
            return Err(format!("invalid_env: env 键 {key} 的值必须是字符串"));
        };
        if text.contains('\0') {
            return Err(format!("invalid_env: env 键 {key} 的值不能含空字节"));
        }
        pairs.push((key.clone(), text.to_string()));
    }
    Ok(pairs)
}

/// 该 task 下真实存在的 attempt 目录编号：按目录个数计，不假设编号连续，也不看 handback.json
fn attempt_numbers(task_root: &Path) -> Vec<u64> {
    let mut numbers: Vec<u64> = Vec::new();
    if let Ok(entries) = fs::read_dir(task_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(number) = name
                .strip_prefix("attempt-")
                .and_then(|rest| rest.parse::<u64>().ok())
            else {
                continue;
            };
            if entry.path().is_dir() {
                numbers.push(number);
            }
        }
    }
    numbers.sort_unstable();
    numbers
}

/// path_prepend：字符串数组，按字面顺序前插；空数组与缺省等价，空元素/NUL/超量/超长一律在占号之前拒绝
fn parse_path_prepend(args: &Value) -> Result<Vec<String>, String> {
    const MAX_ENTRIES: usize = 16;
    const MAX_ENTRY_LEN: usize = 4096;
    let Some(value) = args.get("path_prepend") else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err(
            "invalid_path_prepend: path_prepend 必须是字符串数组（不接受带分隔符的单字符串）"
                .to_string(),
        );
    };
    if items.is_empty() {
        // 与缺省等价（与 env: {} 口径一致）：空集合表示"不追加"，不是错误
        return Ok(Vec::new());
    }
    if items.len() > MAX_ENTRIES {
        return Err(format!(
            "invalid_path_prepend: 最多 {MAX_ENTRIES} 个目录，收到 {} 个",
            items.len()
        ));
    }
    let mut dirs = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(dir) = item.as_str() else {
            return Err(format!(
                "invalid_path_prepend: 第 {} 个元素不是字符串",
                index + 1
            ));
        };
        if dir.is_empty() {
            return Err(format!(
                "invalid_path_prepend: 第 {} 个元素是空字符串",
                index + 1
            ));
        }
        if dir.contains('\0') {
            return Err(format!(
                "invalid_path_prepend: 第 {} 个元素含 NUL 字符",
                index + 1
            ));
        }
        if dir.len() > MAX_ENTRY_LEN {
            return Err(format!(
                "invalid_path_prepend: 第 {} 个元素超过 {MAX_ENTRY_LEN} 字符",
                index + 1
            ));
        }
        dirs.push(dir.to_string());
    }
    Ok(dirs)
}

/// 分页参数：缺省 offset=0 / limit=20；类型或范围不合法时给具名错误
fn pagination_args(args: &Value) -> Result<(u64, u64), String> {
    let offset = match args.get("offset") {
        None => 0,
        Some(value) => value
            .as_u64()
            .ok_or_else(|| "invalid_pagination: offset 必须是 0 或正整数".to_string())?,
    };
    let limit = match args.get("limit") {
        None => 20,
        Some(value) => value
            .as_u64()
            .ok_or_else(|| "invalid_pagination: limit 必须是 1..=100 的整数".to_string())?,
    };
    if !(1..=100).contains(&limit) {
        return Err("invalid_pagination: limit 必须在 1..=100 之间".to_string());
    }
    Ok((offset, limit))
}

/// env 的幂等摘要：键 -> "sha256:<hex>"，只用于比对，不把可能的密钥明文落盘
fn env_signature_value(pairs: &[(String, String)]) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in pairs {
        map.insert(key.clone(), Value::String(digest_bytes(value.as_bytes())));
    }
    Value::Object(map)
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
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置，以及 initialize 审计（记录可能多份，按 receivedAt 聚合，顶层回显最近一条 clientInfo；仅审计弱识别，不构成身份）；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "task.execute",
            "description": "接单执行：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）；AW 不定义判据、不做验收。超时只向受控集合（POSIX 进程组 / Windows Job Object）发送终止信号，仅当观测到集合内无存活进程才写「已清理受控集合」，并注明逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建、先加入作业再恢复执行（恢复前不会派生子进程），但 BREAKAWAY 或加入作业失败的作业外进程仍不可观测。不承诺终止整棵进程树",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "派单标识（字母或数字开头，仅含字母数字 . _ -，≤64）" },
                    "cmd": { "type": "string", "description": "派单方指定的执行/验证命令" },
                    "cwd": { "type": "string", "description": "工作目录，默认进程当前目录；回执中的 locator 始终为相对路径" },
                    "timeout_secs": { "type": "integer", "minimum": 1, "maximum": 600, "description": "超时秒数，默认 120；超时记 blocked 并给出原因。原因按受控集合清理结果三态给出：已清理受控集合 / 受控集合内仍有存活进程 / 受控集合未能建立或无法查询（此时仅向直接子进程发送终止信号）；逃逸进程是否仍在运行未验证；不承诺终止整棵进程树" },
                    "env": {
                        "type": "object",
                        "description": "可选：追加/覆盖子进程环境变量（字符串 map，仅非 PATH 变量）。未列出的变量继承 bridge 进程环境；显式同名变量覆盖继承值。PATH 任意大小写变体一律拒绝（invalid_env）：继承的 PATH 永远保留、不会整体覆盖、也不会被静默忽略；需要追加目录请用 path_prepend。值为空串表示置空，不是 unset；不做任何自动推断",
                        "additionalProperties": { "type": "string" }
                    },
                    "path_prepend": {
                        "type": "array",
                        "maxItems": 16,
                        "items": { "type": "string", "maxLength": 4096 },
                        "description": "可选：前插到子进程 PATH 的目录列表（字符串数组）。按数组字面顺序排在最前，其后才是继承的 PATH；平台分隔符自动使用（Windows 为分号，其他平台为冒号）。空数组与缺省完全等价（签名相同）。参与幂等签名：同 task_id 重投必须完全一致。上限 16 项、单项 ≤4096 字符；不做目录存在性校验。非法形态（非字符串数组/空元素/NUL/超量/超长）在占号前以 invalid_path_prepend 拒绝"
                    }
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
        },
        {
            "name": "runs.list",
            "description": "只读枚举 runs 目录下的任务：按 task_id 升序，offset/limit 确定分页；排除 _session 等预留目录；只回 env 键名、不回 env 值；目录枚举不是执行状态的权威来源，判定请用 receipt.get",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "offset": { "type": "integer", "minimum": 0, "description": "跳过多少条，默认 0" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "description": "本页条数，默认 20" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "runs.prune",
            "description": "显式证据清理：默认 dry_run=true 只列出候选、零写入；真删必须同时 dry_run=false 与 confirm=true。只删非最新 attempt 的整个目录，最新 attempt 永不进入候选；年龄只认 handback 的 finishedAt（缺字段一律不删）；`status` 是可选的状态多值白名单，显式传入才生效，缺省不过滤、无默认状态、无服务端分档，与 older_than_days 为 AND；删除前先写 runs/prune.jsonl 记账；部分失败如实报 partial 且 isError=true。系统内没有定时/启动自动清理",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "older_than_days": { "type": "integer", "minimum": 1, "maximum": 3650, "description": "必填：只清理 finishedAt 距今不少于该天数的 attempt。PM 口径统一推荐 365 天；仍须显式传入，不提供隐式默认阈值，缺省按 invalid_retention 拒绝" },
                    "status": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "type": "string", "enum": ["ok", "failed", "blocked"] }, "description": "可选：状态多值白名单，显式传入才生效；缺省不过滤（与旧行为一致）。仅 handback.status 精确匹配才进入候选，与 older_than_days 为 AND；空数组/null/未知值按 invalid_retention 拒绝；无默认状态、无服务端分档" },
                    "dry_run": { "type": "boolean", "description": "默认 true：只列候选、不删不写；false 表示要求真删（还需 confirm=true）" },
                    "confirm": { "type": "boolean", "description": "默认 false：真删必须显式 true；单独 true 不隐含真删" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "description": "本批最多处理的候选数，默认 100；在 status 过滤之后生效" }
                },
                "required": ["older_than_days"],
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
