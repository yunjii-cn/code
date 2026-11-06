// aw-mcp-bridge 方法分发：只读 health/status + 执行层 task.execute/receipt.get
use serde_json::{json, Value};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "aw-mcp-bridge";
/// 会话审计目录名：以 _ 开头，与合法 task_id（必须字母或数字开头）不可能撞名
const SESSION_DIR: &str = "_session";
/// 活跃标记目录：以 . 开头，`is_task_dir_name` 会排除它，runs.list / runs.prune 不会把它当任务
const INFLIGHT_DIR: &str = ".inflight";
/// 任务历史台账目录：即使 task 目录被整体清理，这里仍保留"曾经存在过"的事实
const TASK_HISTORY_DIR: &str = ".task-history";
const INFLIGHT_VERSION: &str = "inflight-marker/1";
/// 标记过期宽限：超过 deadlineAt 且 pid 已不可见才判定为残留
const INFLIGHT_STALE_GRACE_SECS: i64 = 120;

// 由 build.rs 注入，缺失时为 "unknown"（不使用当前时间兜底，保证可复现构建）
pub const BUILD_TIME: &str = env!("AW_BUILD_TIME");
pub const DESKTOP_BUILD_TIME: &str = env!("AW_DESKTOP_BUILD_TIME");

pub struct Server {
    settings_path: PathBuf,
    runs_dir: PathBuf,
    /// 连接级计数：进程内没有"会话"概念，只有一条 stdio 连接。
    connection_started: AtomicBool,
    connections_peak: AtomicU64,
    initialize_total: AtomicU64,
}

impl Server {
    pub fn new(settings_path: PathBuf) -> Self {
        Self::with_runs_dir(settings_path, default_runs_dir())
    }

    pub fn with_runs_dir(settings_path: PathBuf, runs_dir: PathBuf) -> Self {
        Self {
            settings_path,
            runs_dir,
            connection_started: AtomicBool::new(false),
            connections_peak: AtomicU64::new(0),
            initialize_total: AtomicU64::new(0),
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
        // 首条消息即视为 stdio 连接建立；进程存活期间只有这一条连接
        if !self.connection_started.swap(true, Ordering::SeqCst) {
            self.connections_peak.fetch_max(1, Ordering::SeqCst);
        }
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
                self.initialize_total.fetch_add(1, Ordering::SeqCst);
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

        // 参数诊断只观测、不拦截：先算好，调用结果完全不受它影响
        let diagnostics = argument_diagnostics(name, &arguments);

        // v1.31：仅当调用方显式要求（_meta.strict_arguments=true）才按 schema 拒绝；
        // 默认仍是 v1.18/v1.25 的“只诊断、不拒绝”，不打断携带 _meta/别名/自定义元数据的既有调用。
        if let Some(diagnostics) = diagnostics.as_ref() {
            if strict_arguments_requested(&arguments) {
                if let Some(error) = strict_arguments_error(diagnostics) {
                    let payload = attach_argument_diagnostics(error, diagnostics.clone());
                    return tool_result(&payload, true);
                }
            }
        }

        let (payload, is_error) = match name {
            "health" => (self.health_payload(), false),
            "status" => (self.status_payload(), false),
            "task.execute" => match self.execute_task(&arguments) {
                Ok(payload) => (payload, false),
                Err(error) => (json!({ "error": error }), true),
            },
            "receipt.get" => match self.receipt_payload(&arguments) {
                Ok(payload) => (payload, false),
                Err(error) => (json!({ "error": error }), true),
            },
            "runs.list" => match self.runs_list(&arguments) {
                Ok(payload) => (payload, false),
                Err(error) => (json!({ "error": error }), true),
            },
            "runs.prune" => match self.runs_prune(&arguments) {
                Ok(payload) => {
                    // 部分失败必须让调用方看到 isError=true：不许把"删了一半"包装成成功
                    let is_error = payload.get("status").and_then(Value::as_str) == Some("partial");
                    (payload, is_error)
                }
                Err(error) => (json!({ "error": error }), true),
            },
            _ => (json!({ "error": format!("未知工具：{name}") }), true),
        };

        let payload = match diagnostics {
            Some(diagnostics) => {
                log_argument_diagnostics(&diagnostics);
                attach_argument_diagnostics(payload, diagnostics)
            }
            None => payload,
        };
        tool_result(&payload, is_error)
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
            "connections": self.connections_payload(),
            "server": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION"),
                "buildTime": BUILD_TIME,
                "desktopBuildTime": DESKTOP_BUILD_TIME
            }
        })
    }

    /// 连接级计数：如实说明进程内没有会话概念
    fn connections_payload(&self) -> Value {
        let started = self.connection_started.load(Ordering::SeqCst);
        json!({
            "unit": "connection",
            "scope": "process",
            "active": if started { 1 } else { 0 },
            "total": if started { 1 } else { 0 },
            "peak": self.connections_peak.load(Ordering::SeqCst),
            "initialize_total": self.initialize_total.load(Ordering::SeqCst),
            "note": "进程内无会话概念：stdio 单连接，active/total/peak 恒为 1；同一连接重复 initialize 只累加 initialize_total，不新增连接；仅观测，不做准入或上限"
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
                if !is_task_dir_name(&name) {
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
        // 排除参数在任何写入之前解析：非法/越界一律 invalid_exclusion，零删除零审计
        let exclusions = parse_exclusions(args)?;
        let exclude_filter = if exclusions.is_empty() {
            Value::Null
        } else {
            json!(
                exclusions
                    .iter()
                    .map(|exclusion| json!({
                        "task_id": exclusion.task_id,
                        "attempt": exclusion.attempt
                    }))
                    .collect::<Vec<Value>>()
            )
        };

        let evaluated_at = now_stamp();
        let now = chrono::Utc::now();
        let PruneScan {
            candidates,
            skipped,
            excluded,
            eligible_total,
            task_count,
        } = self.prune_candidates(older_than_days, now, status_filter.as_deref(), &exclusions);
        // candidates_total 沿用旧口径：枚举入口的合格总数（含被请求排除的条目）
        let total_candidates = eligible_total;
        let excluded_total = excluded
            .iter()
            .filter(|item| item["reason"].as_str() == Some("excluded_by_request"))
            .count() as u64;
        let not_in_candidates_total = excluded.len() as u64 - excluded_total;
        let exclude_requests_total = exclusions.len() as u64;
        let processing_total = candidates.len() as u64;
        let batch: Vec<Value> = candidates.into_iter().take(limit as usize).collect();
        // 两种可直接核对的等式：
        // 1) deleted + failed + trashed + excluded_total == batch_candidates_total
        // 2) candidates_total == batch_candidates_total + not_processed_total（limit 截断部分）
        let batch_candidates_total = batch.len() as u64 + excluded_total;
        let not_processed_total = processing_total - batch.len() as u64;
        let truncated = not_processed_total > 0;

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
                "exclude_filter": exclude_filter,
                "evaluated_at": evaluated_at,
                "limit": limit,
                "candidates_total": total_candidates,
                "batch_candidates_total": batch_candidates_total,
                "not_processed_total": not_processed_total,
                "truncated": truncated,
                "candidates": batch,
                "skipped_total": skipped.len() as u64,
                "skipped": skipped,
                "exclude_requests_total": exclude_requests_total,
                "excluded_total": excluded_total,
                "not_in_candidates_total": not_in_candidates_total,
                "excluded": excluded
            }));
        }

        if batch.is_empty() {
            // 没有删除发生：不写审计，也不创建任何目录（含 .prune-trash）
            return Ok(json!({
                "dry_run": false,
                "status": "ok",
                "older_than_days": older_than_days,
                "status_filter": status_filter_json,
                "exclude_filter": exclude_filter,
                "evaluated_at": evaluated_at,
                "limit": limit,
                "candidates_total": total_candidates,
                "batch_candidates_total": batch_candidates_total,
                "not_processed_total": not_processed_total,
                "truncated": truncated,
                "deleted_total": 0,
                "failed_total": 0,
                "trashed_total": 0,
                "exclude_requests_total": exclude_requests_total,
                "excluded_total": excluded_total,
                "not_in_candidates_total": not_in_candidates_total,
                "deleted": [],
                "failed": [],
                "trashed": [],
                "excluded": excluded
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
            "exclude_filter": exclude_filter,
            "limit": limit,
            "candidate_count": batch.len() as u64,
            "candidates": batch.clone(),
            "exclude_requests_total": exclude_requests_total,
            "excluded_total": excluded_total,
            "not_in_candidates_total": not_in_candidates_total,
            "excluded": excluded
        });
        // 先记账后删除：plan 行写不进去就一个目录都不删
        self.append_prune_record(&plan)
            .map_err(|error| format!("{error}；本轮未删除任何目录"))?;

        let mut deleted: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        let mut trashed: Vec<Value> = Vec::new();
        // 每条条目自包含整批 rule，跨批次核对不必回连顶层
        let rule = json!({
            "older_than_days": older_than_days,
            "status_filter": status_filter_json.clone(),
            "limit": limit,
            "batch_id": batch_id.clone()
        });
        for candidate in &batch {
            let task_id = candidate["task_id"].as_str().unwrap_or_default().to_string();
            let attempt = candidate["attempt"].as_u64().unwrap_or_default();
            let source_path = candidate["path"].as_str().unwrap_or_default().to_string();
            let attempt_dir = self
                .runs_dir
                .join(&task_id)
                .join(format!("attempt-{attempt}"));
            // 指纹在删除前算出；文件不存在时写 null，不编造
            let (handback_sha, handback_bytes) =
                file_sha256_and_len(&attempt_dir.join("handback.json"));
            let (dispatch_sha, dispatch_bytes) =
                file_sha256_and_len(&attempt_dir.join("dispatch.json"));
            let total_bytes = match (handback_bytes, dispatch_bytes) {
                (Some(handback), Some(dispatch)) => json!(handback + dispatch),
                _ => Value::Null,
            };
            let fingerprint = json!({
                "status": candidate["status"],
                "finishedAt": candidate["finishedAt"],
                "dispatchedAt": candidate["dispatchedAt"],
                "handback_sha256": handback_sha,
                "dispatch_sha256": dispatch_sha,
                "bytes": {
                    "handback": handback_bytes,
                    "dispatch": dispatch_bytes,
                    "total": total_bytes
                }
            });
            match self.delete_attempt_dir(&task_id, attempt, &batch_id) {
                DeleteOutcome::Deleted { trash_path } => {
                    let mut item = candidate.clone();
                    if let Some(object) = item.as_object_mut() {
                        object.insert("state".into(), json!("deleted"));
                        object.insert("source_path".into(), json!(source_path));
                        object.insert("trash_path".into(), json!(trash_path));
                        object.insert("deleted_at".into(), json!(now_stamp()));
                        object.insert("rule".into(), rule.clone());
                        object.insert("fingerprint".into(), fingerprint);
                    }
                    deleted.push(item);
                }
                DeleteOutcome::RetainedAtSource { error } => failed.push(retained_entry(
                    RetainedState::AtSource,
                    &task_id,
                    attempt,
                    &source_path,
                    None,
                    &error,
                    &rule,
                    &fingerprint,
                )),
                DeleteOutcome::RetainedInTrash { trash_path, error } => trashed.push(retained_entry(
                    RetainedState::InTrash,
                    &task_id,
                    attempt,
                    &source_path,
                    Some(&trash_path),
                    &error,
                    &rule,
                    &fingerprint,
                )),
            }
        }
        let deleted_total = deleted.len() as u64;
        let failed_total = failed.len() as u64;
        let trashed_total = trashed.len() as u64;
        let status = if failed_total + trashed_total == 0 {
            "ok"
        } else {
            "partial"
        };
        let result = json!({
            "at": now_stamp(),
            "phase": "result",
            "batch_id": batch_id,
            "older_than_days": older_than_days,
            "status_filter": status_filter_json,
            "exclude_filter": exclude_filter,
            "limit": limit,
            "deleted": deleted.clone(),
            "failed": failed.clone(),
            "trashed": trashed.clone(),
            "deleted_total": deleted_total,
            "failed_total": failed_total,
            "trashed_total": trashed_total,
            "exclude_requests_total": exclude_requests_total,
            "excluded_total": excluded_total,
            "not_in_candidates_total": not_in_candidates_total,
            "batch_candidates_total": batch_candidates_total,
            "excluded": excluded
        });
        if let Err(error) = self.append_prune_record(&result) {
            return Err(format!(
                "prune_audit_failed: 已删除 {deleted_total} 个、失败 {failed_total} 个、暂存 {trashed_total} 个；结果行写入 prune.jsonl 失败：{error}"
            ));
        }

        Ok(json!({
            "dry_run": false,
            "status": status,
            "batch_id": batch_id,
            "older_than_days": older_than_days,
            "status_filter": status_filter_json,
            "exclude_filter": exclude_filter,
            "evaluated_at": evaluated_at,
            "limit": limit,
            "candidates_total": total_candidates,
            "batch_candidates_total": batch_candidates_total,
            "not_processed_total": not_processed_total,
            "truncated": truncated,
            "deleted_total": deleted_total,
            "failed_total": failed_total,
            "trashed_total": trashed_total,
            "exclude_requests_total": exclude_requests_total,
            "excluded_total": excluded_total,
            "not_in_candidates_total": not_in_candidates_total,
            "deleted": deleted,
            "failed": failed,
            "trashed": trashed,
            "excluded": excluded
        }))
    }

    /// 计算清理候选：只认 finishedAt、只碰非最新 attempt、不跟随符号链接。
    ///
    /// 按条目排除在**枚举入口**一次完成：命中排除的合格条目只进 `excluded[]`，
    /// 既不会进入 `candidates[]`，也不会被后续 `limit` 截断或删除；`eligible_total`
    /// 仍把它们算在候选总数内，供回执核对计数不变式。
    fn prune_candidates(
        &self,
        older_than_days: u64,
        now: chrono::DateTime<chrono::Utc>,
        status_filter: Option<&[String]>,
        exclusions: &[Exclusion],
    ) -> PruneScan {
        let mut candidates: Vec<Value> = Vec::new();
        let mut skipped: Vec<Value> = Vec::new();
        let mut excluded: Vec<Value> = Vec::new();
        let mut eligible_total: u64 = 0;
        let mut matched: Vec<bool> = vec![false; exclusions.len()];
        let mut task_ids: Vec<String> = Vec::new();
        if self.runs_dir.is_dir() {
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
                    if !is_task_dir_name(&name) {
                        continue;
                    }
                    task_ids.push(name);
                }
            }
        }
        task_ids.sort();
        let task_count = task_ids.len();

        for task_id in task_ids {
            let root = self.runs_dir.join(&task_id);
            let numbers = attempt_numbers(&root);
            let latest = numbers.last().copied();
            // 活跃标记优先于一切候选条件：即使有超龄非最新 attempt，也不在本次 prune 中删除
            if let Some(snapshot) = self.inflight_snapshot(&task_id) {
                let stale = snapshot
                    .get("stale")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                skipped.push(json!({
                    "task_id": task_id.clone(),
                    "attempt": latest,
                    "reason": if stale { "task_inflight_stale" } else { "task_inflight" },
                    "inflight": snapshot
                }));
                continue;
            }
            let Some(latest) = latest else {
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
                eligible_total += 1;
                if let Some(index) = exclusions
                    .iter()
                    .position(|exclusion| exclusion.matches(&task_id, number))
                {
                    matched[index] = true;
                    excluded.push(json!({
                        "task_id": task_id.clone(),
                        "attempt": number,
                        "path": format!("runs/{task_id}/attempt-{number}"),
                        "reason": "excluded_by_request"
                    }));
                    continue;
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
        // 没命中任何合格条目的排除请求不报错、也不静默忽略：逐条记 not_in_candidates
        for (index, exclusion) in exclusions.iter().enumerate() {
            if matched[index] {
                continue;
            }
            excluded.push(json!({
                "task_id": exclusion.task_id.clone(),
                "attempt": exclusion.attempt,
                // 不在候选集时没有真实路径可给：写 null，不编造
                "path": Value::Null,
                "reason": "not_in_candidates"
            }));
        }
        PruneScan {
            candidates,
            skipped,
            excluded,
            eligible_total,
            task_count,
        }
    }

    /// 整目录搬进 .prune-trash 后再删：重命名失败则原样不动，避免"只删了一半文件"
    fn delete_attempt_dir(&self, task_id: &str, attempt: u64, batch_id: &str) -> DeleteOutcome {
        let source = self.runs_dir.join(task_id).join(format!("attempt-{attempt}"));
        let stage_root = self.runs_dir.join(".prune-trash").join(batch_id);
        if let Err(error) = fs::create_dir_all(&stage_root) {
            return DeleteOutcome::RetainedAtSource {
                error: format!("创建暂存目录失败（attempt 原样未动）：{error}"),
            };
        }
        let staged = stage_root.join(format!("{task_id}-attempt-{attempt}"));
        let staged_rel = format!("runs/.prune-trash/{batch_id}/{task_id}-attempt-{attempt}");
        if let Err(error) = fs::rename(&source, &staged) {
            return DeleteOutcome::RetainedAtSource {
                error: format!("整体移动失败（attempt 原样未动）：{error}"),
            };
        }
        if let Err(error) = fs::remove_dir_all(&staged) {
            let error_text = format!("已整链移至 {staged_rel}，但删除暂存失败（数据未丢）：{error}");
            return DeleteOutcome::RetainedInTrash {
                trash_path: staged_rel,
                error: error_text,
            };
        }
        // 本批暂存目录空了就顺手移除；失败不影响结果
        let _ = fs::remove_dir(&stage_root);
        DeleteOutcome::Deleted {
            trash_path: staged_rel,
        }
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

    /// 跨进程活跃标记：`create_new` 原子占位；已存在且未过期说明另一个进程在跑，本次只读不删。
    /// 只有创建者在本进程退出/返回/报错时删除（Drop），异常退出留下的标记由 pid + 时间判定残留。
    fn acquire_inflight_marker(
        &self,
        task_id: &str,
        mode: &str,
        args: &Value,
    ) -> Result<Option<InflightGuard>, String> {
        let dir = self.runs_dir.join(INFLIGHT_DIR);
        fs::create_dir_all(&dir).map_err(|error| {
            format!("inflight_marker_failed: 创建活跃标记目录失败：{error}")
        })?;
        let path = dir.join(format!("{task_id}.json"));
        let deadline = inflight_deadline(args, mode);
        for _ in 0..3 {
            let record = json!({
                "version": INFLIGHT_VERSION,
                "task_id": task_id,
                "pid": std::process::id(),
                "mode": mode,
                "state": "active",
                "startedAt": now_stamp(),
                "deadlineAt": deadline
            });
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    use std::io::Write;
                    if let Err(error) = file
                        .write_all(serde_json::to_string(&record).unwrap_or_default().as_bytes())
                        .and_then(|_| file.sync_all())
                    {
                        let _ = fs::remove_file(&path);
                        return Err(format!("inflight_marker_failed: 写入活跃标记失败：{error}"));
                    }
                    return Ok(Some(InflightGuard { path }));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    match read_json(&path) {
                        Ok(existing) if marker_is_stale(&existing, chrono::Utc::now()) => {
                            // 残留：清理后重试占位；并发清理时由 create_new 决出唯一持有者
                            let _ = fs::remove_file(&path);
                            continue;
                        }
                        _ => return Ok(None),
                    }
                }
                Err(error) => {
                    return Err(format!("inflight_marker_failed: 创建活跃标记失败：{error}"))
                }
            }
        }
        Ok(None)
    }

    /// 只读快照：给 prune 与外部调用方判断"该 task 是否有活跃执行"，不改动标记
    fn inflight_snapshot(&self, task_id: &str) -> Option<Value> {
        let path = self.runs_dir.join(INFLIGHT_DIR).join(format!("{task_id}.json"));
        let mut record = read_json(&path).ok()?;
        let stale = marker_is_stale(&record, chrono::Utc::now());
        if let Some(object) = record.as_object_mut() {
            object.insert("stale".to_string(), json!(stale));
        }
        Some(record)
    }

    /// 历史台账：task 目录被整体清理后，仍能凭它判定"这是重跑而不是既有结果"
    fn read_task_history(&self, task_id: &str) -> Option<Value> {
        read_json(
            &self
                .runs_dir
                .join(TASK_HISTORY_DIR)
                .join(format!("{task_id}.json")),
        )
        .ok()
    }

    /// 更新历史台账：失败不影响执行结果（台账是防误判元数据，不是执行证据）
    fn record_task_history(&self, task_id: &str, attempts: u64, status: &str, finished_at: &str) {
        let dir = self.runs_dir.join(TASK_HISTORY_DIR);
        if fs::create_dir_all(&dir).is_err() {
            return;
        }
        let first_seen = self
            .read_task_history(task_id)
            .and_then(|record| {
                record
                    .get("firstSeenAt")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(now_stamp);
        let record = json!({
            "version": "task-history/1",
            "task_id": task_id,
            "firstSeenAt": first_seen,
            "updatedAt": now_stamp(),
            "attempts": attempts,
            "lastStatus": status,
            "lastFinishedAt": finished_at
        });
        let _ = write_json_file(&dir.join(format!("{task_id}.json")), &record);
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
        let standalone = standalone_mode(args)?;
        let mode = if standalone { "standalone" } else { "dispatched" };
        // 活跃标记覆盖整个调用生命周期：正常返回、提前报错、被阻断都会在 Drop 时删除
        let _inflight = self.acquire_inflight_marker(task_id, mode, args)?;
        match standalone {
            false => self.execute_dispatched(args, task_id),
            true => self.execute_standalone(args, task_id),
        }
    }

    /// dispatched（缺省）：外部（PM）派单。AW 只执行、交回执、不验收、不判停（既有契约逐字不变）
    fn execute_dispatched(&self, args: &Value, task_id: &str) -> Result<Value, String> {
        // standalone-only 字段在 dispatched 下必须显式拒绝：不许让调用方误以为 AW 会按这些字段验收
        for field in ["argv", "acceptance", "max_attempts"] {
            if args
                .get(field)
                .map(|value| !value.is_null())
                .unwrap_or(false)
            {
                return Ok(blocked_receipt(
                    task_id,
                    "",
                    format!("dispatched_input_rejected: 字段 {field} 仅限 mode=standalone；dispatched 由 PM 定义判据，AW 不验收"),
                ));
            }
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
        }
        // 模式是硬边界：同一 task_id 既有记录属于另一种模式时，必须具名拒绝，禁止「同一目录既被 PM 当权威、又被 AW 自判」
        let stored_mode = stored
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("dispatched");
        let requested_mode = signature
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("dispatched");
        if stored_mode != requested_mode {
            return Err(format!(
                "mode_conflict: task_id `{task_id}` 已存在且属于 {stored_mode} 模式，不能以 {requested_mode} 模式复用；请换新 task_id"
            ));
        }
        if requested_mode == "dispatched" {
            if let Some(object) = stored.as_object_mut() {
                if !object.contains_key("env") {
                    object.insert("env".to_string(), json!({}));
                }
                // 升级前的签名没有 path_prepend：其语义等价于空数组，补齐后再比对
                if !object.contains_key("path_prepend") {
                    object.insert("path_prepend".to_string(), json!([]));
                }
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

    /// standalone：没有外部派单方时由 AW 自己闭环——执行、按任务声明的 acceptance 判定、失败有界重试
    fn execute_standalone(&self, args: &Value, task_id: &str) -> Result<Value, String> {
        let spec = match StandaloneSpec::parse(args) {
            Ok(spec) => spec,
            Err(reason) => return Ok(standalone_blocked_receipt(task_id, reason)),
        };
        let task_root = self.runs_dir.join(task_id);
        let signature = spec.signature(task_id);
        // 目录已不存在时先读台账：有历史说明此前的全部记录被外部清理，本次是重跑而非回放
        let prior_history = if task_root.exists() {
            None
        } else {
            self.read_task_history(task_id)
        };

        if task_root.exists() {
            return self.replay_standalone(
                task_id,
                &task_root,
                &signature,
                spec.timeout_secs,
                spec.max_attempts,
            );
        }
        fs::create_dir_all(&task_root).map_err(|e| format!("创建运行目录失败：{e}"))?;
        let replay_basis = if prior_history.is_some() {
            "rerun_after_records_missing"
        } else {
            "fresh"
        };

        for attempt in 1..=spec.max_attempts {
            let run_dir = task_root.join(format!("attempt-{attempt}"));
            match fs::create_dir(&run_dir) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // 并发重投：落败方等待胜者的最终结果，不重复执行
                    return self.replay_standalone(
                        task_id,
                        &task_root,
                        &signature,
                        spec.timeout_secs,
                        spec.max_attempts,
                    );
                }
                Err(error) => return Err(format!("创建尝试目录失败：{error}")),
            }
            write_dispatch(&run_dir, &signature)?;

            let commit = git_output(&spec.cwd, &["rev-parse", "--short", "HEAD"])
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let before = git_paths(&spec.cwd);
            let run = run_standalone_command(&spec, &run_dir)?;
            let after = git_paths(&spec.cwd);
            let (changed_files, changed_files_truncated) = changed_between(&before, &after);

            let (criteria, missing, all_passed) = evaluate_acceptance(&spec, &run);
            let (conclusion, action, reason, last_failure) =
                decide_standalone(&run, &missing, all_passed, &spec, attempt);

            let verdict = json!({
                "version": "standalone-verdict/1",
                "mode": "standalone",
                "task_id": task_id,
                "attempt": attempt,
                "replay_basis": replay_basis,
                "prior_attempts": prior_history
                    .as_ref()
                    .and_then(|record| record.get("attempts"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                "prior_last_finished_at": prior_history
                    .as_ref()
                    .and_then(|record| record.get("lastFinishedAt"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "decided_at": now_stamp(),
                "verdict": conclusion,
                "acceptance": spec.acceptance,
                "argv_path_scan": spec.argv_scan,
                "criteria": criteria,
                "observations": {
                    "exit_code": run.exit_code,
                    "stdout_digest": run.stdout_digest,
                    "stderr_digest": run.stderr_digest,
                    "stdout_preview": run.stdout_preview,
                    "stderr_preview": run.stderr_preview,
                    "stdout_bytes": run.stdout_bytes,
                    "stderr_bytes": run.stderr_bytes,
                    "stdout_truncated": run.stdout_truncated,
                    "stderr_truncated": run.stderr_truncated,
                    "changed_files": changed_files,
                    "changed_files_truncated": changed_files_truncated,
                    "duration_ms": run.duration_ms,
                    "timed_out": run.timed_out,
                    "spawn_failed": run.spawn_error
                },
                "missing": missing,
                "decision": {
                    "action": action,
                    "reason": reason,
                    "last_failure": last_failure
                }
            });
            let verdict_path = run_dir.join("verdict.json");
            write_json_file(&verdict_path, &verdict)?;
            let verdict_digest = digest_file(&verdict_path)?;

            let status = match conclusion.as_str() {
                "passed" => "ok",
                "failed" => "failed",
                _ => "blocked",
            };
            let blocked_reason = if status == "blocked" {
                Some(standalone_blocked_detail(&run, &missing))
            } else {
                None
            };
            let finished_at = now_stamp();
            let mut handback = json!({
                "task_id": task_id,
                "attempt": attempt,
                "mode": "standalone",
                "replayed": false,
                "replay_basis": replay_basis,
                "prior_attempts": prior_history
                    .as_ref()
                    .and_then(|record| record.get("attempts"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                "prior_last_finished_at": prior_history
                    .as_ref()
                    .and_then(|record| record.get("lastFinishedAt"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "finishedAt": finished_at,
                "status": status,
                "changed_files": changed_files,
                "changed_files_truncated": changed_files_truncated,
                "cwd": spec.cwd.to_string_lossy(),
                "timeout_secs": spec.timeout_secs,
                "env_keys": [],
                "path_prepend": [],
                "argv": spec.argv,
                "argv_path_scan": spec.argv_scan,
                "max_attempts": spec.max_attempts,
                "verify_cmd": spec.display_cmd(),
                "exit_code": run.exit_code,
                "stdout_digest": run.stdout_digest,
                "stderr_digest": run.stderr_digest,
                "commit": commit,
                "duration_ms": run.duration_ms,
                "blocked_reason": blocked_reason,
                "verdict_conclusion": conclusion,
                "verdict_locator": format!("runs/{task_id}/attempt-{attempt}/verdict.json"),
                "artifacts": [
                    {
                        "kind": "stdout",
                        "locator": format!("runs/{task_id}/attempt-{attempt}/stdout.txt"),
                        "hash": run.stdout_digest
                    },
                    {
                        "kind": "stderr",
                        "locator": format!("runs/{task_id}/attempt-{attempt}/stderr.txt"),
                        "hash": run.stderr_digest
                    },
                    {
                        "kind": "verdict",
                        "locator": format!("runs/{task_id}/attempt-{attempt}/verdict.json"),
                        "hash": verdict_digest
                    }
                ]
            });
            if action == "terminate" {
                if let Some(object) = handback.as_object_mut() {
                    object.insert("attempts_total".to_string(), json!(attempt));
                }
                write_handback(&run_dir, &handback)?;
                // 最终结果标记：重投据此回放，避免把中间失败尝试误当成最终结论
                write_json_file(&task_root.join(STANDALONE_RESULT), &handback)?;
                // 外部若在本 attempt 写入期间把它移走/删掉，这里必须报错而不是把内存里的 ok 返回
                ensure_standalone_attempt_records(&run_dir)?;
                if !task_root.join(STANDALONE_RESULT).is_file() {
                    return Err(attempt_dir_lost_message(&run_dir));
                }
                self.record_task_history(task_id, attempt, status, &finished_at);
                return Ok(handback);
            }
            write_handback(&run_dir, &handback)?;
            ensure_standalone_attempt_records(&run_dir)?;
            self.record_task_history(task_id, attempt, status, &finished_at);
        }
        Err(format!(
            "standalone_retry_exhausted_without_result: task_id `{task_id}` 的尝试循环没有产出终止判定"
        ))
    }

    /// standalone 重投：签名/模式一致则回放最终结果；仍在执行则等待，不重复跑
    fn replay_standalone(
        &self,
        task_id: &str,
        task_root: &Path,
        signature: &Value,
        timeout_secs: u64,
        max_attempts: u64,
    ) -> Result<Value, String> {
        let dispatch_path = task_root.join("attempt-1").join("dispatch.json");
        let stored = match read_json(&dispatch_path) {
            Ok(stored) => stored,
            Err(_) => wait_for_json(&dispatch_path, Duration::from_secs(5)).ok_or_else(|| {
                format!(
                    "task_id_conflict: task_id `{task_id}` 已有运行目录但缺少派单签名（dispatch.json），无法证明参数一致；重跑请使用新 task_id"
                )
            })?,
        };
        let mut stored = stored;
        if let Some(object) = stored.as_object_mut() {
            object.remove("dispatchedAt");
        }
        let stored_mode = stored
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("dispatched");
        if stored_mode != "standalone" {
            return Err(format!(
                "mode_conflict: task_id `{task_id}` 已存在且属于 {stored_mode} 模式，不能以 standalone 模式复用；请换新 task_id"
            ));
        }
        if &stored != signature {
            return Err(format!(
                "task_id_conflict: task_id `{task_id}` 已存在且 standalone 参数不一致（cwd / argv / acceptance / max_attempts / timeout_secs 必须完全一致）；重跑请使用新 task_id"
            ));
        }
        let result_path = task_root.join(STANDALONE_RESULT);
        if let Ok(result) = read_json(&result_path) {
            return Ok(standalone_replay_receipt(result, "existing"));
        }
        // 胜者的总执行上限是 timeout_secs * max_attempts，等待窗口必须覆盖它
        let wait = timeout_secs.saturating_mul(max_attempts).saturating_add(5);
        wait_for_json(&result_path, Duration::from_secs(wait))
            .map(|result| standalone_replay_receipt(result, "waited"))
            .ok_or_else(|| {
                format!("task_in_progress: task_id `{task_id}` 的 standalone 执行仍在进行中，请稍后重试")
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

/// `runs/` 下某目录名是否可当作任务目录（`runs.list` 与 `runs.prune` 共用同一判据）：
/// 以 `_`（`_session` 等预留目录）或 `.`（`.prune-trash` 等内部目录）开头的名字一律不是任务。
fn is_task_dir_name(name: &str) -> bool {
    !name.starts_with('_') && !name.starts_with('.') && is_valid_task_id(name)
}

/// 一次候选枚举的完整结果：合格总数（含被排除项）、候选、跳过与排除分桶
struct PruneScan {
    candidates: Vec<Value>,
    skipped: Vec<Value>,
    excluded: Vec<Value>,
    eligible_total: u64,
    task_count: usize,
}

/// `runs.prune.exclude` 的单条请求：`attempt=None` 表示排除该 task 的全部候选
#[derive(Clone, Debug)]
struct Exclusion {
    task_id: String,
    attempt: Option<u64>,
}

impl Exclusion {
    fn matches(&self, task_id: &str, attempt: u64) -> bool {
        self.task_id == task_id && self.attempt.map(|number| number == attempt).unwrap_or(true)
    }
}

fn exclusion_error(detail: &str) -> String {
    format!(
        "invalid_exclusion: {detail}；exclude 每项必须是 {{task_id, attempt?}}；task_id 必须通过任务目录命名校验（仅字母数字与 . _ -，长度 <=64，且不以 _ 或 . 开头；含路径分隔符、..、绝对路径、通配符一律拒绝）"
    )
}

/// 解析并校验排除参数：任何非法/歧义形态都在写入之前拒绝（零删除、零审计）
fn parse_exclusions(args: &Value) -> Result<Vec<Exclusion>, String> {
    let Some(value) = args.get("exclude") else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| exclusion_error("exclude 必须是非空数组"))?;
    if array.is_empty() {
        return Err(exclusion_error(
            "exclude 必须是非空数组（不需要排除就不要传这个参数）",
        ));
    }
    if array.len() > 100 {
        return Err(exclusion_error("exclude 最多 100 条"));
    }
    let mut parsed: Vec<Exclusion> = Vec::new();
    for item in array {
        let Some(object) = item.as_object() else {
            return Err(exclusion_error(
                "exclude 的每一项必须是对象 {task_id, attempt?}",
            ));
        };
        let Some(task_id) = object.get("task_id").and_then(Value::as_str) else {
            return Err(exclusion_error("exclude 项缺少字符串 task_id"));
        };
        if !is_task_dir_name(task_id) {
            return Err(exclusion_error(&format!(
                "task_id 未通过命名校验：{task_id:?}"
            )));
        }
        let attempt = match object.get("attempt") {
            None => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .filter(|number| *number >= 1)
                    .ok_or_else(|| exclusion_error("attempt 必须是 >=1 的整数"))?,
            ),
        };
        if parsed
            .iter()
            .any(|existing| existing.task_id == task_id && existing.attempt == attempt)
        {
            return Err(exclusion_error("同一 (task_id, attempt) 不得重复"));
        }
        if parsed.iter().any(|existing| {
            existing.task_id == task_id && (existing.attempt.is_none() || attempt.is_none())
        }) {
            return Err(exclusion_error(
                "同一 task_id 不得同时按整任务与单次 attempt 排除",
            ));
        }
        parsed.push(Exclusion {
            task_id: task_id.to_string(),
            attempt,
        });
    }
    Ok(parsed)
}

/// 单次删除的终态：只有 `Deleted` 表示"已从 `runs/` 移除"；其余两种都还在盘上。
enum DeleteOutcome {
    Deleted {
        trash_path: String,
    },
    RetainedAtSource {
        error: String,
    },
    RetainedInTrash {
        trash_path: String,
        error: String,
    },
}

/// 未删除条目的状态：类型层面只可能取这两个值，构造不出 `deleted`
#[derive(Clone, Copy)]
enum RetainedState {
    AtSource,
    InTrash,
}

impl RetainedState {
    fn as_str(self) -> &'static str {
        match self {
            RetainedState::AtSource => "retained_at_source",
            RetainedState::InTrash => "trashed_not_deleted",
        }
    }
}

/// 未删除条目的统一构造入口：`state` 只能来自 `RetainedState`
fn retained_entry(
    state: RetainedState,
    task_id: &str,
    attempt: u64,
    source_path: &str,
    trash_path: Option<&str>,
    error: &str,
    rule: &Value,
    fingerprint: &Value,
) -> Value {
    json!({
        "task_id": task_id,
        "attempt": attempt,
        "path": source_path,
        "source_path": source_path,
        "state": state.as_str(),
        "trash_path": trash_path,
        "error": error,
        "rule": rule,
        "fingerprint": fingerprint
    })
}

/// 删除前指纹：缺文件时如实写 null，不编造值
fn file_sha256_and_len(path: &std::path::Path) -> (Value, Option<u64>) {
    use sha2::{Digest, Sha256};
    match fs::read(path) {
        Ok(bytes) => (
            json!(hex::encode(Sha256::digest(&bytes))),
            Some(bytes.len() as u64),
        ),
        Err(_) => (Value::Null, None),
    }
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

            // 实测锁定：忽略 SIGTERM 且持续 fork 的进程组，一次 SIGKILL 后必须清空。
            // 终止前组长仍存活（因此宽限期结束后必然派发 SIGKILL），SIGKILL 派发时仍在 fork。
            #[test]
            fn single_sigkill_suffices_under_fork_pressure() {
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg("trap '' TERM; while :; do sleep 5 & done")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn fork pressure");
                supervisor.attach(&mut child);
                let pgid = child.id() as i32;
                std::thread::sleep(Duration::from_millis(400));
                let exists_before = group_exists(pgid);
                let started = Instant::now();
                let outcome = supervisor.terminate(&mut child);
                println!(
                    "[evidence] fork_pressure pgid={pgid} group_exists_before={exists_before} outcome={outcome:?} elapsed_ms={}",
                    started.elapsed().as_millis()
                );
                assert!(
                    exists_before,
                    "组长忽略 SIGTERM 且持续 fork，终止前进程组必须存在（宽限期后必须派发 SIGKILL）"
                );
                assert_eq!(
                    outcome,
                    CleanupOutcome::Cleaned,
                    "一次 SIGKILL 后受控集合必须清空"
                );
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

            // 实测锁定：成员持续拉起后代时，一次 TerminateJobObject 后作业必须清空。
            #[test]
            fn single_terminate_suffices_under_spawn_pressure() {
                for trial in 0..6u64 {
                    let mut supervisor = Supervisor::new();
                    assert!(supervisor.job.is_some());
                    let mut command = Command::new("cmd");
                    command
                        .args([
                            "/C",
                            "for /L %i in (1,1,20000) do @start /b ping -n 30 127.0.0.1 > NUL",
                        ])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    supervisor.configure(&mut command);
                    let mut child = command.spawn().expect("spawn spawner");
                    supervisor.attach(&mut child);
                    assert!(supervisor.attached);
                    std::thread::sleep(Duration::from_millis(30 + trial * 20));
                    let before = supervisor.active_processes();
                    let started = Instant::now();
                    let outcome = supervisor.terminate(&mut child);
                    let after = supervisor.active_processes();
                    println!(
                        "[evidence] spawn_pressure trial={trial} ActiveProcesses_before={before:?} outcome={outcome:?} ActiveProcesses_after={after:?} elapsed_ms={}",
                        started.elapsed().as_millis()
                    );
                    assert!(
                        before.unwrap_or(0) > 0,
                        "终止前作业内必须有活跃进程（否则用例空转）"
                    );
                    assert_eq!(
                        outcome,
                        CleanupOutcome::Cleaned,
                        "一次 TerminateJobObject 后作业必须清空"
                    );
                    assert_eq!(after, Some(0));
                }
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

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("序列化 JSON 失败：{e}"))?;
    fs::write(path, text).map_err(|e| format!("写入 {} 失败：{e}", path.display()))
}

/// standalone 的最终结果标记：重投按它回放，不把中间失败尝试当成最终结论
const STANDALONE_RESULT: &str = "standalone-result.json";

/// 模式显式化：缺省 dispatched（PM 派单，行为零变化）；只有显式 standalone 才由 AW 自行闭环
fn standalone_mode(args: &Value) -> Result<bool, String> {
    match args.get("mode") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::String(mode)) if mode == "dispatched" => Ok(false),
        Some(Value::String(mode)) if mode == "standalone" => Ok(true),
        Some(other) => {
            let raw = serde_json::to_string(other).unwrap_or_else(|_| "非字符串值".to_string());
            let repr: String = raw.chars().take(80).collect();
            Err(format!(
                "invalid_mode: mode 只能是 dispatched 或 standalone，收到 {repr}"
            ))
        }
    }
}

struct StandaloneSpec {
    cwd: PathBuf,
    argv: Vec<String>,
    argv_scan: Value,
    acceptance: Vec<Value>,
    max_attempts: u64,
    timeout_secs: u64,
}

impl StandaloneSpec {
    /// 独立模式的输入契约：作用域限定在 cwd、不经 shell、必须有验收标准
    fn parse(args: &Value) -> Result<Self, String> {
        for field in ["cmd", "env", "path_prepend"] {
            if args
                .get(field)
                .map(|value| !value.is_null())
                .unwrap_or(false)
            {
                return Err(match field {
                    "cmd" => "standalone_cmd_rejected: standalone 不接受 cmd（shell 命令）；请用 argv 数组逐项传参，AW 不经 shell 执行",
                    "env" => "standalone_env_rejected: standalone 不接受 env；子进程只继承 bridge 环境",
                    _ => "standalone_path_prepend_rejected: standalone 不接受 path_prepend；执行限定在任务 working directory 内",
                }
                .to_string());
            }
        }

        let raw_cwd = args
            .get("cwd")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "standalone_cwd_required: standalone 必须显式给出 cwd（执行与验收的作用域根目录）".to_string()
            })?;
        let cwd = fs::canonicalize(raw_cwd)
            .map_err(|e| format!("standalone_cwd_invalid: cwd 无法解析为目录：{raw_cwd}（{e}）"))?;
        if !cwd.is_dir() {
            return Err(format!(
                "standalone_cwd_invalid: cwd 不是目录：{}",
                cwd.display()
            ));
        }

        let argv = match args.get("argv") {
            Some(Value::Array(items)) if !items.is_empty() => {
                let mut argv = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    let Some(text) = item.as_str() else {
                        return Err(format!(
                            "standalone_argv_invalid: argv 第 {} 项不是字符串",
                            index + 1
                        ));
                    };
                    if text.is_empty() || text.contains('\0') {
                        return Err(format!(
                            "standalone_argv_invalid: argv 第 {} 项为空或含 NUL",
                            index + 1
                        ));
                    }
                    argv.push(text.to_string());
                }
                argv
            }
            _ => {
                return Err("standalone_argv_required: standalone 必须给出非空 argv 数组（argv[0] 是程序，其余是参数）".to_string())
            }
        };
        ensure_program_in_scope(&argv[0], &cwd)?;
        // 参数里携带的路径同样必须落在 cwd 内；越界在任何执行之前拒绝
        let argv_scan = scan_argv_paths(&argv, &cwd)?;

        let timeout_secs = match args.get("timeout_secs") {
            None | Some(Value::Null) => 120,
            Some(value) => value
                .as_u64()
                .filter(|value| (1..=600).contains(value))
                .ok_or_else(|| {
                    "standalone_timeout_invalid: timeout_secs 必须在 1..=600 之间".to_string()
                })?,
        };
        let max_attempts = match args.get("max_attempts") {
            None | Some(Value::Null) => 3,
            Some(value) => value
                .as_u64()
                .filter(|value| (1..=5).contains(value))
                .ok_or_else(|| {
                    "standalone_max_attempts_invalid: max_attempts 必须是 1..=5 的整数".to_string()
                })?,
        };

        let Some(items) = args.get("acceptance").and_then(Value::as_array) else {
            return Err("standalone_acceptance_required: standalone 必须在任务输入里声明 acceptance；不声明验收标准不得默认通过".to_string());
        };
        if items.is_empty() {
            return Err(
                "standalone_acceptance_required: acceptance 不能为空（没有验收标准无法判定）"
                    .to_string(),
            );
        }
        let mut acceptance = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            validate_acceptance(item, index)?;
            acceptance.push(item.clone());
        }

        Ok(Self {
            cwd,
            argv,
            argv_scan,
            acceptance,
            max_attempts,
            timeout_secs,
        })
    }

    fn signature(&self, task_id: &str) -> Value {
        json!({
            "task_id": task_id,
            "mode": "standalone",
            "cwd": self.cwd.to_string_lossy(),
            "argv": self.argv,
            "acceptance": self.acceptance,
            "max_attempts": self.max_attempts,
            "timeout_secs": self.timeout_secs
        })
    }

    fn display_cmd(&self) -> String {
        self.argv.join(" ")
    }
}

/// 验收标准：四种可核对类型；未知类型/缺字段/路径越界在占号前拒绝
fn validate_acceptance(item: &Value, index: usize) -> Result<(), String> {
    let Some(object) = item.as_object() else {
        return Err(format!(
            "standalone_acceptance_invalid: 第 {} 条验收标准不是对象",
            index + 1
        ));
    };
    let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
    let prefix = format!("standalone_acceptance_invalid: 第 {} 条（{kind}）", index + 1);
    match kind {
        "exit_code" => {
            if object.get("expected").and_then(Value::as_i64).is_none() {
                return Err(format!("{prefix} 缺少整数 expected"));
            }
        }
        "file_exists" => {
            let path = object.get("path").and_then(Value::as_str).unwrap_or("");
            ensure_relative_scope_path(path)?;
        }
        "file_content_match" => {
            let path = object.get("path").and_then(Value::as_str).unwrap_or("");
            ensure_relative_scope_path(path)?;
            if object
                .get("contains")
                .and_then(Value::as_str)
                .map(str::is_empty)
                .unwrap_or(true)
            {
                return Err(format!("{prefix} 缺少非空 contains"));
            }
        }
        "git_diff_non_empty" => {}
        _ => {
            return Err(format!(
                "standalone_acceptance_invalid: 第 {} 条 type 未知（只支持 exit_code / file_exists / file_content_match / git_diff_non_empty）",
                index + 1
            ))
        }
    }
    Ok(())
}

fn ensure_relative_scope_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains('\0') {
        return Err("standalone_acceptance_invalid: 验收路径为空或含 NUL".to_string());
    }
    for component in Path::new(path).components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(format!(
                "standalone_path_escape: 验收路径必须是 working directory 内的相对路径（不允许绝对路径或 ..）：{path}"
            ));
        }
    }
    Ok(())
}

/// 显式程序路径必须落在 cwd 内；裸程序名（不含分隔符）按继承 PATH 解析，允许解释器/运行时
fn ensure_program_in_scope(program: &str, cwd: &Path) -> Result<(), String> {
    let has_separator = program.contains('/') || program.contains('\\');
    let mut rooted = false;
    for component in Path::new(program).components() {
        if matches!(
            component,
            Component::RootDir | Component::ParentDir | Component::Prefix(_)
        ) {
            rooted = true;
        }
    }
    if !has_separator && !rooted {
        return Ok(());
    }
    let path = Path::new(program);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let resolved = fs::canonicalize(&candidate).map_err(|_| {
        format!("standalone_program_unresolved: 无法证明程序位于 working directory 内：{program}")
    })?;
    if !resolved.starts_with(cwd) {
        return Err(format!(
            "standalone_program_outside_scope: 程序路径越出 working directory：{}",
            resolved.display()
        ));
    }
    Ok(())
}

/// argv 中携带路径的参数的识别与作用域校验（v1.28）。
///
/// 识别范围：`key=value` / `key:value` / `--flag=value` 的 value 部分、引号包裹后的
/// 内容、独立参数值本身；`..`、相对/绝对路径、盘符、UNC、含空格路径都按路径处理。
/// 明确不当作路径：URL（`scheme://...`）、正则/通配符形态、普通标量（无分隔符且非
/// 已存在的符号链接）。无法判定真实位置时放行并在回执 `unknown[]` 记
/// `standalone_argv_path_unknown`，不做 fail-closed 误拒。
fn scan_argv_paths(argv: &[String], cwd: &Path) -> Result<Value, String> {
    let mut checked: Vec<Value> = Vec::new();
    let mut unknown: Vec<Value> = Vec::new();
    for (offset, raw) in argv.iter().enumerate().skip(1) {
        let index = offset + 1;
        let Some(token) = classify_argv_token(raw, cwd) else {
            continue;
        };
        let entry_base = json!({
            "index": index,
            "raw": raw,
            "value": token.value,
            "via": token.via,
            "class": token.class,
            "encoded": token.encoded
        });
        if let Some(reason) = token.unknown_reason {
            let mut entry = entry_base;
            if let Some(object) = entry.as_object_mut() {
                object.insert("decision".to_string(), json!("allowed"));
                object.insert("reason_code".to_string(), json!(ARGV_PATH_UNKNOWN));
                object.insert("reason".to_string(), json!(reason));
            }
            unknown.push(entry);
            continue;
        }
        match resolve_argv_path(&token.value, cwd) {
            ArgvPathVerdict::InScope { resolved } => {
                let mut entry = entry_base;
                if let Some(object) = entry.as_object_mut() {
                    object.insert("decision".to_string(), json!("in_scope"));
                    object.insert(
                        "resolved".to_string(),
                        json!(resolved.to_string_lossy().to_string()),
                    );
                }
                checked.push(entry);
            }
            ArgvPathVerdict::Escapes { detail } => {
                return Err(format!(
                    "standalone_argv_path_escape: argv 第 {index} 项（{raw}）解析后越出 working directory：{detail}"
                ));
            }
            ArgvPathVerdict::Unresolved { detail } => {
                let mut entry = entry_base;
                if let Some(object) = entry.as_object_mut() {
                    object.insert("decision".to_string(), json!("allowed"));
                    object.insert("reason_code".to_string(), json!(ARGV_PATH_UNKNOWN));
                    object.insert("reason".to_string(), json!(detail));
                }
                unknown.push(entry);
            }
        }
    }
    let checked_total = checked.len();
    let unknown_total = unknown.len();
    Ok(json!({
        "scope": cwd.to_string_lossy(),
        "checked": checked,
        "unknown": unknown,
        "checked_total": checked_total,
        "unknown_total": unknown_total
    }))
}

const ARGV_PATH_UNKNOWN: &str = "standalone_argv_path_unknown";

struct ArgvTokenScan {
    value: String,
    via: &'static str,
    class: &'static str,
    encoded: bool,
    unknown_reason: Option<String>,
}

enum ArgvPathVerdict {
    InScope { resolved: PathBuf },
    Escapes { detail: String },
    Unresolved { detail: String },
}

fn classify_argv_token(raw: &str, cwd: &Path) -> Option<ArgvTokenScan> {
    let mut value = raw.to_string();
    let mut via = "bare";
    if let Some(inner) = strip_matching_quotes(&value) {
        value = inner.to_string();
    }
    let (split_value, split_via) = split_argument_value(&value);
    if split_via != "bare" {
        via = split_via;
        value = split_value;
        if let Some(inner) = strip_matching_quotes(&value) {
            value = inner.to_string();
        }
    }
    if value.is_empty() {
        return None;
    }
    // Windows 单字母斜杠开关（cmd.exe 的 /C、/S 等）不是路径
    #[cfg(windows)]
    if is_windows_switch(&value) {
        return None;
    }
    // 盘符优先：`C://x` 是 Windows 路径而不是 URL
    let drive = has_drive_prefix(&value);
    if !drive && is_url_like(&value) {
        return None;
    }
    if let Some(reason) = env_expansion_reason(&value) {
        return Some(ArgvTokenScan {
            class: "unknown_env_expansion",
            unknown_reason: Some(reason),
            value,
            via,
            encoded: false,
        });
    }
    // `\\?\` / `\\.\` 是 Windows 逐字前缀，其中的 ? 不是通配符
    let verbatim = value.starts_with("\\\\?\\") || value.starts_with("\\\\.\\");
    // 正则/通配符形态不按路径校验（宁可漏报；盲区见契约 §2.6.8）
    if !verbatim
        && value.chars().any(|c| {
            matches!(c, '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '*' | '+' | '?')
        })
    {
        return None;
    }
    let mut decoded = value.clone();
    let mut encoded = false;
    for _ in 0..2 {
        match percent_decode(&decoded) {
            Some(next) if next != decoded => {
                decoded = next;
                encoded = true;
            }
            _ => break,
        }
    }
    if decoded.is_empty() {
        return None;
    }
    let class = if verbatim {
        "absolute_path"
    } else if decoded.starts_with("\\\\") || decoded.starts_with("//") {
        "unc_path"
    } else if has_drive_prefix(&decoded) {
        "windows_drive_path"
    } else if decoded.starts_with('/') || decoded.starts_with('\\') {
        "absolute_path"
    } else if decoded.contains('/') || decoded.contains('\\') || decoded.starts_with('.') {
        "relative_path"
    } else {
        // 普通标量不是路径；只有它本身是 cwd 下的符号链接时才按路径校验
        match fs::symlink_metadata(cwd.join(&decoded)) {
            Ok(meta) if meta.file_type().is_symlink() => "relative_path",
            _ => return None,
        }
    };
    Some(ArgvTokenScan {
        value: decoded,
        via,
        class,
        encoded,
        unknown_reason: None,
    })
}

/// 取一层配对引号（`"..."` / `'...'`），引号内视为参数值本身
fn strip_matching_quotes(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        if (first == b'"' || first == b'\'') && bytes[bytes.len() - 1] == first {
            return Some(&text[1..text.len() - 1]);
        }
    }
    None
}

fn split_argument_value(token: &str) -> (String, &'static str) {
    if token.starts_with('-') {
        if let Some((_, value)) = token.split_once('=') {
            if !value.is_empty() {
                return (value.to_string(), "flag_equals");
            }
        }
        return (token.to_string(), "bare");
    }
    if has_drive_prefix(token) {
        return (token.to_string(), "bare");
    }
    if let Some((key, value)) = token.split_once('=') {
        if is_plain_key(key) && !value.is_empty() {
            return (value.to_string(), "key_equals");
        }
    }
    if let Some((key, value)) = token.split_once(':') {
        if is_plain_key(key) && !value.is_empty() {
            return (value.to_string(), "key_colon");
        }
    }
    (token.to_string(), "bare")
}

fn is_plain_key(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    text.len() <= 64 && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn has_drive_prefix(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic()) && chars.next() == Some(':')
}

/// Windows 下 `/C` 这类单字母斜杠开关是命令参数而非路径（`/etc/passwd` 仍是路径）
#[cfg(windows)]
fn is_windows_switch(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 2 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic()
}

fn is_url_like(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once("://") else {
        return false;
    };
    !rest.is_empty()
        && scheme
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// 未展开的环境变量/波浪号：不推测真实位置，放行并记 unknown
fn env_expansion_reason(value: &str) -> Option<String> {
    if value.starts_with('~') {
        return Some("波浪号（~）路径未展开，无法判定真实位置".to_string());
    }
    if value.contains("${") {
        return Some("环境变量 ${...} 未展开，无法判定真实位置".to_string());
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if let Some(end) = value[index + 1..].find('%') {
                let name = &value[index + 1..index + 1 + end];
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')'))
                {
                    return Some(format!(
                        "环境变量 %{name}% 未展开，无法判定真实位置"
                    ));
                }
            }
        }
        if byte == b'$' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_alphabetic() {
            return Some("环境变量 $NAME 未展开，无法判定真实位置".to_string());
        }
        index += 1;
    }
    None
}

/// 只解码 `%HH`；无变化返回 None（避免把普通参数误判成路径）
fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut changed = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                out.push(high * 16 + low);
                index += 3;
                changed = true;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    changed.then(|| String::from_utf8_lossy(&out).to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn resolve_argv_path(value: &str, cwd: &Path) -> ArgvPathVerdict {
    let path = Path::new(value);
    let rooted = path.is_absolute()
        || value.starts_with('/')
        || value.starts_with('\\')
        || has_drive_prefix(value);
    if rooted {
        return resolve_rooted_argv_path(path, value, cwd);
    }
    let mut current = cwd.to_path_buf();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let Some(parent) = current.parent() else {
                    return ArgvPathVerdict::Escapes {
                        detail: "已到根目录仍继续上跳".to_string(),
                    };
                };
                current = parent.to_path_buf();
                if !path_within(&current, cwd) {
                    return ArgvPathVerdict::Escapes {
                        detail: format!("上跳后为 {}", current.display()),
                    };
                }
            }
            Component::Normal(name) => {
                let next = current.join(name);
                match fs::symlink_metadata(&next) {
                    Ok(meta) if meta.file_type().is_symlink() => match fs::canonicalize(&next) {
                        Ok(real) => {
                            if !path_within(&real, cwd) {
                                return ArgvPathVerdict::Escapes {
                                    detail: format!(
                                        "符号链接 {} 指向 {}",
                                        next.display(),
                                        real.display()
                                    ),
                                };
                            }
                            current = real;
                        }
                        Err(error) => {
                            return ArgvPathVerdict::Unresolved {
                                detail: format!("符号链接 {} 无法解析：{error}", next.display()),
                            };
                        }
                    },
                    _ => current = next,
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return ArgvPathVerdict::Unresolved {
                    detail: "相对路径中出现根/前缀组件".to_string(),
                };
            }
        }
    }
    if path_within(&current, cwd) {
        ArgvPathVerdict::InScope { resolved: current }
    } else {
        ArgvPathVerdict::Escapes {
            detail: format!("解析到 {}", current.display()),
        }
    }
}

fn resolve_rooted_argv_path(path: &Path, raw: &str, cwd: &Path) -> ArgvPathVerdict {
    let normalized = normalize_lexically(path);
    if !path_within(&normalized, cwd) {
        return ArgvPathVerdict::Escapes {
            detail: format!("字面归一化后为 {}", normalized.display()),
        };
    }
    if let Ok(real) = fs::canonicalize(path) {
        return if path_within(&real, cwd) {
            ArgvPathVerdict::InScope { resolved: real }
        } else {
            ArgvPathVerdict::Escapes {
                detail: format!("解析为 {}", real.display()),
            }
        };
    }
    let mut ancestor = path.to_path_buf();
    loop {
        if let Ok(real) = fs::canonicalize(&ancestor) {
            return if path_within(&real, cwd) {
                ArgvPathVerdict::InScope { resolved: ancestor }
            } else {
                ArgvPathVerdict::Escapes {
                    detail: format!(
                        "已存在的祖先 {} 解析为 {}",
                        ancestor.display(),
                        real.display()
                    ),
                }
            };
        }
        match ancestor.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => ancestor = parent.to_path_buf(),
            _ => break,
        }
    }
    ArgvPathVerdict::Unresolved {
        detail: format!("无法解析路径：{raw}"),
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    normalized
}

/// 组件级比较：Windows 下忽略大小写，并把 `\\?\` / `\\.\` 前缀视同普通盘符前缀
fn path_within(path: &Path, base: &Path) -> bool {
    let mut path_components = path.components();
    for base_component in base.components() {
        match path_components.next() {
            Some(component) if component_key(component) == component_key(base_component) => {}
            _ => return false,
        }
    }
    true
}

fn component_key(component: Component<'_>) -> String {
    let key = match component {
        Component::Prefix(prefix) => {
            let text = prefix.as_os_str().to_string_lossy().to_string();
            let text = text
                .replace('/', "\\")
                .trim_start_matches("\\\\?\\")
                .trim_start_matches("\\\\.\\")
                .to_string();
            format!("prefix:{text}")
        }
        Component::RootDir => "root".to_string(),
        Component::CurDir => "cur".to_string(),
        Component::ParentDir => "parent".to_string(),
        Component::Normal(name) => format!("normal:{}", name.to_string_lossy()),
    };
    #[cfg(windows)]
    {
        key.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        key
    }
}

/// 活跃标记的 RAII 守卫：任何正常返回、提前报错、阻断都会在 Drop 时删除标记
struct InflightGuard {
    path: PathBuf,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 标记的有效期上限：覆盖胜者可能跑满的 timeout * max_attempts，再留 60s 收尾余量
fn inflight_deadline(args: &Value, mode: &str) -> String {
    let timeout = args
        .get("timeout_secs")
        .and_then(Value::as_u64)
        .unwrap_or(120);
    let attempts = if mode == "standalone" {
        args.get("max_attempts")
            .and_then(Value::as_u64)
            .unwrap_or(3)
            .clamp(1, 10)
    } else {
        1
    };
    let total = timeout
        .saturating_mul(attempts)
        .saturating_add(60)
        .clamp(60, 86_400);
    (chrono::Utc::now() + chrono::Duration::seconds(total as i64)).to_rfc3339()
}

/// 残留判定：pid 仍存活 → 绝不判残留；pid 已消失 → 5s 后判残留；无 pid 时退回 deadline + 宽限
fn marker_is_stale(record: &Value, now: chrono::DateTime<chrono::Utc>) -> bool {
    let started = record
        .get("startedAt")
        .and_then(Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc));
    if let Some(pid) = record.get("pid").and_then(Value::as_u64) {
        if process_is_alive(pid as u32) {
            return false;
        }
        return match started {
            Some(started) => now > started + chrono::Duration::seconds(5),
            None => true,
        };
    }
    let deadline = record
        .get("deadlineAt")
        .and_then(Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc));
    deadline
        .map(|deadline| now > deadline + chrono::Duration::seconds(INFLIGHT_STALE_GRACE_SECS))
        .unwrap_or(true)
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code);
        let _ = CloseHandle(handle);
        ok != 0 && code as i32 == STILL_ACTIVE
    }
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(any(windows, unix)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

/// 写完后复核 attempt 的关键文件仍在：目录被外部移走/删除时必须报错，不能把内存中的 ok 返回
fn ensure_standalone_attempt_records(run_dir: &Path) -> Result<(), String> {
    for name in [
        "dispatch.json",
        "verdict.json",
        "handback.json",
        "stdout.txt",
        "stderr.txt",
    ] {
        if !run_dir.join(name).is_file() {
            return Err(attempt_dir_lost_message(run_dir));
        }
    }
    Ok(())
}

fn attempt_dir_lost_message(run_dir: &Path) -> String {
    format!(
        "attempt_dir_lost: attempt 目录在写入过程中消失或被改名（可能被 runs.prune 或外部清理移动）：{}；本次执行结果不可信，未报告 ok",
        run_dir.display()
    )
}

/// 回放既有结果时标注来源：existing=调用时已完成，waited=等待并发胜者落盘
fn standalone_replay_receipt(mut receipt: Value, kind: &str) -> Value {
    if let Some(object) = receipt.as_object_mut() {
        object.insert("replayed".to_string(), json!(true));
        object.insert("replay_kind".to_string(), json!(kind));
    }
    receipt
}

fn standalone_blocked_receipt(task_id: &str, reason: String) -> Value {
    let mut receipt = blocked_receipt(task_id, "", reason);
    if let Some(object) = receipt.as_object_mut() {
        object.insert("mode".to_string(), json!("standalone"));
    }
    receipt
}

struct StandaloneRun {
    exit_code: Option<i32>,
    stdout_digest: String,
    stderr_digest: String,
    stdout_preview: String,
    stderr_preview: String,
    stdout_bytes: u64,
    stderr_bytes: u64,
    stdout_truncated: bool,
    stderr_truncated: bool,
    duration_ms: u64,
    timed_out: bool,
    spawn_error: Option<String>,
    blocked_reason: Option<String>,
}

/// 不经 shell：直接启动 argv[0]，其余参数逐项传入；current_dir 固定为 cwd
fn run_standalone_command(
    spec: &StandaloneSpec,
    run_dir: &Path,
) -> Result<StandaloneRun, String> {
    let stdout_path = run_dir.join("stdout.txt");
    let stderr_path = run_dir.join("stderr.txt");
    let stdout_file =
        fs::File::create(&stdout_path).map_err(|e| format!("创建 stdout 失败：{e}"))?;
    let stderr_file =
        fs::File::create(&stderr_path).map_err(|e| format!("创建 stderr 失败：{e}"))?;

    let mut command = Command::new(&spec.argv[0]);
    command
        .args(&spec.argv[1..])
        .current_dir(&spec.cwd)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let started = Instant::now();
    let deadline = Duration::from_secs(spec.timeout_secs);
    let mut timed_out = false;
    let mut cleanup_outcome = None;
    let mut spawn_error: Option<String> = None;
    let mut supervisor = process_scope::Supervisor::new();
    supervisor.configure(&mut command);
    let exit_status = match command.spawn() {
        Ok(mut child) => {
            supervisor.attach(&mut child);
            if let Some(error) = supervisor.attach_error().map(str::to_string) {
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
            spawn_error = Some(format!("standalone_spawn_failed: 启动程序失败：{error}"));
            None
        }
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    // 目录在子进程运行期间被移走/删除时，按路径读证据会失败；必须具名报 attempt_dir_lost
    if !run_dir.is_dir() {
        return Err(attempt_dir_lost_message(run_dir));
    }
    let stdout_digest = digest_file(&stdout_path).map_err(|error| {
        if !run_dir.is_dir() {
            attempt_dir_lost_message(run_dir)
        } else {
            error
        }
    })?;
    let stderr_digest = digest_file(&stderr_path).map_err(|error| {
        if !run_dir.is_dir() {
            attempt_dir_lost_message(run_dir)
        } else {
            error
        }
    })?;
    let (stdout_preview, stdout_bytes, stdout_truncated) = file_preview(&stdout_path);
    let (stderr_preview, stderr_bytes, stderr_truncated) = file_preview(&stderr_path);
    let blocked_reason = spawn_error
        .clone()
        .or_else(|| timed_out.then(|| timeout_reason(spec.timeout_secs, cleanup_outcome)));
    Ok(StandaloneRun {
        exit_code: exit_status.and_then(|status| status.code()),
        stdout_digest,
        stderr_digest,
        stdout_preview,
        stderr_preview,
        stdout_bytes,
        stderr_bytes,
        stdout_truncated,
        stderr_truncated,
        duration_ms,
        timed_out,
        spawn_error,
        blocked_reason,
    })
}

/// 输出摘要：只给前 200 字符，明确标注是否截断；完整内容以 stdout.txt + 摘要哈希为准
fn file_preview(path: &Path) -> (String, u64, bool) {
    let bytes = fs::read(path).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let truncated = text.chars().count() > 200;
    let preview: String = text.chars().take(200).collect();
    (preview, bytes.len() as u64, truncated)
}

fn changed_between(
    before: &HashSet<String>,
    after: &HashSet<String>,
) -> (Vec<String>, bool) {
    let mut changed: Vec<String> = after.difference(before).cloned().collect();
    changed.sort();
    let truncated = changed.len() > 20;
    changed.truncate(20);
    (changed, truncated)
}

/// 逐条按任务声明的验收标准判定；观测不到的项进入 missing，绝不默认通过
fn evaluate_acceptance(
    spec: &StandaloneSpec,
    run: &StandaloneRun,
) -> (Vec<Value>, Vec<String>, bool) {
    let mut entries = Vec::new();
    let mut missing = Vec::new();
    let mut all_passed = true;
    for (index, criterion) in spec.acceptance.iter().enumerate() {
        let kind = criterion["type"].as_str().unwrap_or("");
        let (observed, passed, unavailable, miss) = match kind {
            "exit_code" => {
                let expected = criterion["expected"].as_i64().unwrap_or(i64::MIN);
                let actual = run.exit_code.map(|code| code as i64);
                (
                    json!({ "exit_code": actual, "expected": expected }),
                    actual == Some(expected),
                    false,
                    None,
                )
            }
            "file_exists" => {
                let path = criterion["path"].as_str().unwrap_or("");
                let meta = fs::metadata(spec.cwd.join(path));
                let exists = meta.is_ok();
                (
                    json!({
                        "path": path,
                        "exists": exists,
                        "bytes": meta.ok().map(|meta| meta.len())
                    }),
                    exists,
                    false,
                    None,
                )
            }
            "file_content_match" => {
                let path = criterion["path"].as_str().unwrap_or("");
                let contains = criterion["contains"].as_str().unwrap_or("");
                match fs::read(spec.cwd.join(path)) {
                    Ok(bytes) => {
                        let matched = String::from_utf8_lossy(&bytes).contains(contains);
                        (
                            json!({
                                "path": path,
                                "exists": true,
                                "contains": contains,
                                "matched": matched,
                                "bytes": bytes.len()
                            }),
                            matched,
                            false,
                            None,
                        )
                    }
                    Err(_) => (
                        json!({
                            "path": path,
                            "exists": false,
                            "contains": contains,
                            "matched": false
                        }),
                        false,
                        false,
                        None,
                    ),
                }
            }
            "git_diff_non_empty" => match git_output(&spec.cwd, &["status", "--porcelain"]) {
                Some(raw) => {
                    let empty = raw.trim().is_empty();
                    (
                        json!({
                            "command": "git status --porcelain",
                            "empty": empty,
                            "bytes": raw.len()
                        }),
                        !empty,
                        false,
                        None,
                    )
                }
                None => (
                    json!({ "command": "git status --porcelain", "available": false }),
                    false,
                    true,
                    Some(
                        "git_diff_non_empty：无法取得 git status --porcelain（cwd 不是可用的 git 仓库或 git 不可执行）"
                            .to_string(),
                    ),
                ),
            },
            _ => unreachable!("acceptance 已在 StandaloneSpec::parse 中校验"),
        };
        if unavailable {
            all_passed = false;
        } else if !passed {
            all_passed = false;
        }
        if let Some(miss) = miss {
            missing.push(miss);
        }
        entries.push(json!({
            "index": index + 1,
            "spec": criterion,
            "observed": observed,
            "passed": passed
        }));
    }
    (entries, missing, all_passed)
}

/// 判定三态与重试/终止：无法判定绝不重试、绝不当通过；失败只在 attempt < max 时重试
fn decide_standalone(
    run: &StandaloneRun,
    missing: &[String],
    all_passed: bool,
    spec: &StandaloneSpec,
    attempt: u64,
) -> (String, String, String, Option<String>) {
    if run.spawn_error.is_some() {
        return (
            "undetermined".to_string(),
            "terminate".to_string(),
            "spawn_failed".to_string(),
            Some("spawn_failed".to_string()),
        );
    }
    if run.timed_out {
        return (
            "undetermined".to_string(),
            "terminate".to_string(),
            "timeout".to_string(),
            Some("timeout".to_string()),
        );
    }
    if !missing.is_empty() {
        return (
            "undetermined".to_string(),
            "terminate".to_string(),
            "observation_unavailable".to_string(),
            Some("observation_unavailable".to_string()),
        );
    }
    let declares_exit_code = spec
        .acceptance
        .iter()
        .any(|criterion| criterion["type"] == "exit_code");
    let command_ok = run.exit_code == Some(0) || declares_exit_code;
    if all_passed && command_ok {
        return (
            "passed".to_string(),
            "terminate".to_string(),
            "acceptance_met".to_string(),
            None,
        );
    }
    let failure = if all_passed {
        "command_failed".to_string()
    } else {
        "acceptance_not_met".to_string()
    };
    if attempt < spec.max_attempts {
        (
            "failed".to_string(),
            "retry".to_string(),
            failure.clone(),
            Some(failure),
        )
    } else {
        (
            "failed".to_string(),
            "terminate".to_string(),
            "retry_exhausted".to_string(),
            Some(failure),
        )
    }
}

fn standalone_blocked_detail(run: &StandaloneRun, missing: &[String]) -> String {
    if let Some(error) = &run.spawn_error {
        return format!("spawn_failed: {error}");
    }
    if run.timed_out {
        return run
            .blocked_reason
            .clone()
            .unwrap_or_else(|| "timeout".to_string());
    }
    format!("observation_unavailable: {}", missing.join("；"))
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

/// 参数诊断（v1.25）：只观测、不拦截，不改变任何调用的成功/失败结果。
///
/// - 未知参数：请求里出现、但 `inputSchema.properties` 未声明；
/// - null 可选字段：schema 声明为可选、调用方显式传 `null`；
/// - 别名：未知参数规范化后能对上某个已声明参数（如 `dryRun` → `dry_run`）；
/// - 值只给类型/长度，绝不回显值本身（未知参数可能携带密钥）。
fn argument_diagnostics(tool_name: &str, arguments: &Value) -> Option<Value> {
    let schema = tool_input_schema(tool_name)?;
    let args = arguments.as_object()?;
    let properties = schema.get("properties")?.as_object()?;
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut unknown: Vec<Value> = Vec::new();
    let mut null_optionals: Vec<Value> = Vec::new();
    for (key, value) in args {
        if let Some(field_schema) = properties.get(key) {
            // 必填字段传 null 由各工具自己的校验负责，不在诊断里重复判定
            if value.is_null() && !required.contains(&key.as_str()) {
                let declared_type = field_schema.get("type").cloned().unwrap_or(Value::Null);
                null_optionals.push(json!({
                    "name": key,
                    "kind": "null_optional",
                    "declared_type": declared_type,
                    "value": argument_value_meta(value)
                }));
            }
            continue;
        }

        let alias_of = alias_target(key, properties);
        let kind = if key == "_meta" {
            "client_metadata"
        } else if alias_of.is_some() {
            "alias"
        } else {
            "unknown"
        };
        let mut entry = json!({
            "name": key,
            "kind": kind,
            "value": argument_value_meta(value)
        });
        if let Some(target) = alias_of {
            entry
                .as_object_mut()
                .expect("entry 是对象")
                .insert("alias_of".to_string(), json!(target));
        }
        unknown.push(entry);
    }

    if unknown.is_empty() && null_optionals.is_empty() {
        return None;
    }

    Some(json!({
        "tool": tool_name,
        "declared_arguments": properties.keys().cloned().collect::<Vec<String>>(),
        "unknown_arguments": unknown,
        "null_optional_arguments": null_optionals,
        "note": "只记录参数名与值类型/长度，不记录值；诊断不改变调用的成功/失败结果"
    }))
}

/// 按工具名取 `inputSchema`；未知工具不做参数诊断（调用本身会返回未知工具错误）
fn tool_input_schema(tool_name: &str) -> Option<Value> {
    tools()
        .as_array()?
        .iter()
        .find(|tool| tool["name"] == tool_name)
        .map(|tool| tool["inputSchema"].clone())
}

/// 别名识别：忽略大小写与 `_` / `-` 后能对上已声明参数，即视为别名而非拼写错误
fn alias_target(key: &str, properties: &serde_json::Map<String, Value>) -> Option<String> {
    let normalized = normalize_argument_name(key);
    if normalized.is_empty() {
        return None;
    }
    properties
        .keys()
        .find(|declared| normalize_argument_name(declared) == normalized)
        .cloned()
}

fn normalize_argument_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '_' | '-'))
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// 只暴露值的元信息：类型 + 长度（字符串字符数 / 数组项数 / 对象键数），永不回显值
fn argument_value_meta(value: &Value) -> Value {
    let (type_name, length) = match value {
        Value::Null => ("null", None),
        Value::Bool(_) => ("boolean", None),
        Value::Number(_) => ("number", None),
        Value::String(text) => ("string", Some(text.chars().count() as u64)),
        Value::Array(items) => ("array", Some(items.len() as u64)),
        Value::Object(map) => ("object", Some(map.len() as u64)),
    };
    json!({ "type": type_name, "length": length })
}

fn attach_argument_diagnostics(mut payload: Value, diagnostics: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert("arguments_diagnostics".to_string(), diagnostics);
    }
    payload
}

/// v1.31 严格模式开关：只由单次调用显式选择，避免同一请求在不同进程里行为不同。
fn strict_arguments_requested(arguments: &Value) -> bool {
    arguments
        .get("_meta")
        .and_then(|meta| meta.get("strict_arguments"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// 严格模式下把 `kind=unknown|alias` 变成具名工具错误。
/// `_meta`（client_metadata）与显式 null 的可选字段不算违规；值只回显名字，不回显值。
fn strict_arguments_error(diagnostics: &Value) -> Option<Value> {
    let entries = diagnostics.get("unknown_arguments")?.as_array()?;
    let violations: Vec<&Value> = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.get("kind").and_then(Value::as_str),
                Some("unknown") | Some("alias")
            )
        })
        .collect();
    if violations.is_empty() {
        return None;
    }

    let names: Vec<&str> = violations
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .collect();
    let declared: Vec<&str> = diagnostics
        .get("declared_arguments")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let suggestions: Vec<String> = violations
        .iter()
        .filter_map(|entry| {
            let name = entry.get("name").and_then(Value::as_str)?;
            let target = entry.get("alias_of").and_then(Value::as_str)?;
            Some(format!("{name} -> {target}"))
        })
        .collect();

    Some(json!({
        "error": format!(
            "invalid_arguments: 未声明参数 {}；已声明参数：{}；strict_arguments=true 时按 inputSchema.additionalProperties=false 拒绝",
            names.join(", "),
            declared.join(", ")
        ),
        "code": "invalid_arguments",
        "unknown_arguments": names,
        "declared_arguments": declared,
        "alias_suggestions": suggestions,
        "hint": "去掉 _meta.strict_arguments 即回到默认宽松策略（只诊断不拒绝）；别名只给出建议、不会自动改名，请改用已声明参数名"
    }))
}

/// 日志侧：单行 JSON 到 stderr，与回执里的 `arguments_diagnostics` 是同一份信息
fn log_argument_diagnostics(diagnostics: &Value) {
    if let Ok(text) = serde_json::to_string(diagnostics) {
        eprintln!("[aw-mcp-bridge] arguments_diagnostics {text}");
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
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置，初始化审计（记录可能多份，按 receivedAt 聚合，顶层回显最近一条 clientInfo；仅审计弱识别，不构成身份），以及连接级计数 connections（unit=connection：进程内无会话概念，stdio 单连接故 active/total/peak 恒为 1，initialize_total 随 initialize 次数累加，多份 initialize 不等于多会话；仅观测，不做准入或上限）；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "task.execute",
            "description": "接单执行，两种运行模式必须显式区分。dispatched（缺省，外部派单）：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）；AW 不定义判据、不做验收。standalone（mode=standalone，无外部派单方）：由 AW 自行闭环，必须显式给出 cwd/argv/acceptance，按每条验收标准逐 attempt 判定，把依据哪条标准、观测到什么、结论一起落盘 verdict.json，结论三态 passed/failed/undetermined；undetermined 即无法判定绝不默认通过并说明缺什么；只有 failed 且 attempt < max_attempts 才重试，每次尝试独立记录、不覆盖，耗尽写 retry_exhausted。两种模式互斥，同一 task_id 不得跨模式复用。超时只向受控集合（POSIX 进程组 / Windows Job Object）发送终止信号；终止只尝试一次（不重试），仅当观测到集合内无存活进程才写「已清理受控集合」，并注明逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建、先加入作业再恢复执行（恢复前不会派生子进程），但 BREAKAWAY 或加入作业失败的作业外进程仍不可观测。不承诺终止整棵进程树",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "派单标识（字母或数字开头，仅含字母数字 . _ -，≤64）" },
                    "mode": {
                        "type": "string",
                        "enum": ["dispatched", "standalone"],
                        "description": "可选：缺省 dispatched（外部派单；AW 只执行不验收，判据由派单方定义），standalone 表示没有外部派单方时由 AW 自行闭环。standalone 拒绝 cmd/env/path_prepend，执行不经 shell 且限定 cwd。模式不靠推断，必须显式传入；同一 task_id 不得跨模式复用，跨模式重投按 mode_conflict 拒绝"
                    },
                    "cmd": { "type": "string", "description": "dispatched：派单方指定的执行/验证命令" },
                    "argv": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 1024,
                        "items": { "type": "string", "minLength": 1, "maxLength": 4096 },
                        "description": "standalone 必填：程序与参数数组，argv[0] 是程序（显式路径必须落在 working directory 内），其余逐项作为参数传给程序，不经 shell 解释。argv[1..] 中携带路径的参数（独立值、--flag=value、=/: 分隔、引号包裹、相对/绝对/UNC/盘符、含空格路径）按规范化后的真实位置校验：越界在占号前以 standalone_argv_path_escape 拒绝；URL、正则/通配符形态与普通标量不按路径校验；无法判定的形态放行并在回执 argv_path_scan.unknown 记 standalone_argv_path_unknown。dispatched 下传入即拒绝（dispatched_input_rejected）"
                    },
                    "acceptance": {
                        "type": "array",
                        "minItems": 1,
                        "description": "standalone 必填：任务声明的验收标准数组（至少 1 条）。类型：exit_code（expected 整数退出码）、file_exists（path 为 cwd 内相对路径）、file_content_match（path + 非空 contains）、git_diff_non_empty（cwd 是 git 仓库且工作区有变更）。绝对路径或 .. 按 standalone_path_escape 拒绝。未声明验收不得默认通过；观测不到所需事实时结论为 undetermined。dispatched 下传入即拒绝",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["exit_code", "file_exists", "file_content_match", "git_diff_non_empty"] },
                                "path": { "type": "string", "description": "cwd 内相对路径（file_exists / file_content_match）" },
                                "contains": { "type": "string", "description": "文件内容必须包含的字面量（file_content_match）" },
                                "expected": { "type": "integer", "description": "期望退出码（exit_code）" }
                            },
                            "required": ["type"]
                        }
                    },
                    "max_attempts": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "description": "standalone 可选：最大尝试次数，默认 3。仅当本次判定 failed 且 attempt < max_attempts 才重试；每次尝试各自目录与 verdict.json，不覆盖上一次记录；耗尽写 retry_exhausted。dispatched 下传入即拒绝"
                    },
                    "cwd": { "type": "string", "description": "工作目录。dispatched 缺省为进程当前目录；standalone 必填且是执行与验收的作用域根目录。回执中的 locator 始终为相对路径" },
                    "timeout_secs": { "type": "integer", "minimum": 1, "maximum": 600, "description": "超时秒数，默认 120；超时记 blocked 并给出原因。原因按受控集合清理结果三态给出：已清理受控集合 / 受控集合内仍有存活进程 / 受控集合未能建立或无法查询（此时仅向直接子进程发送终止信号）；终止只尝试一次（不重试），只有观测到集合内无存活进程才写「已清理受控集合」；逃逸进程是否仍在运行未验证；不承诺终止整棵进程树" },
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
                "required": ["task_id"],
                "anyOf": [
                    { "required": ["cmd"] },
                    { "required": ["mode", "argv"] }
                ],
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
            "description": "显式证据清理：默认 dry_run=true 只列出候选、零写入；真删必须同时 dry_run=false 与 confirm=true。只删非最新 attempt 的整个目录，最新 attempt 永不进入候选；年龄只认 handback 的 finishedAt（缺字段一律不删）；`status` 是可选的状态多值白名单，显式传入才生效，缺省不过滤、无默认状态、无服务端分档，与 older_than_days 为 AND；`exclude` 是可选按条目精确排除（按 task_id，并可精确到某次 attempt），在候选枚举入口生效：命中项只进 excluded[]、不进 candidates[]，PM 可用它把「不可自动判定」的条目留在档内而批量处理其余条目。计数不变式：deleted + failed + trashed + excluded_total == batch_candidates_total，且 candidates_total == batch_candidates_total + not_processed_total（limit 截断部分），回执可直接核对。exclude 的 task_id 与任务目录同一命名校验，越界一律 invalid_exclusion；排除不存在或本就不在候选集的 id 记 not_in_candidates（不报错、不静默忽略）。删除前先写 runs/prune.jsonl 记账；部分失败如实报 partial 且 isError=true。系统内没有定时/启动自动清理",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "older_than_days": { "type": "integer", "minimum": 1, "maximum": 3650, "description": "必填：只清理 finishedAt 距今不少于该天数的 attempt。PM 口径统一推荐 365 天；仍须显式传入，不提供隐式默认阈值，缺省按 invalid_retention 拒绝" },
                    "status": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "type": "string", "enum": ["ok", "failed", "blocked"] }, "description": "可选：状态多值白名单，显式传入才生效；缺省不过滤（与旧行为一致）。仅 handback.status 精确匹配才进入候选，与 older_than_days 为 AND；空数组/null/未知值按 invalid_retention 拒绝；无默认状态、无服务端分档" },
                    "exclude": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 100,
                        "uniqueItems": true,
                        "items": {
                            "type": "object",
                            "properties": {
                                "task_id": { "type": "string", "minLength": 1, "maxLength": 64, "description": "要排除的 task_id" },
                                "attempt": { "type": "integer", "minimum": 1, "description": "可选：只排除该 task 的这一 attempt；缺省表示排除该 task 的全部候选" }
                            },
                            "required": ["task_id"],
                            "additionalProperties": false
                        },
                        "description": "可选：按条目精确排除候选。数组最小 1 条、最多 100 条；同一 (task_id, attempt) 不得重复，同一 task_id 不得同时按整任务与单次 attempt 排除，违者 invalid_exclusion。task_id 必须通过任务目录命名校验（仅字母数字与 . _ -，长度 <=64，且不以 _ 或 . 开头），含路径分隔符、..、绝对路径、通配符一律拒绝。排除在候选枚举入口生效：命中项只进 excluded[]、不进 candidates[]，也不占用 limit；排除不存在或本就不在候选集的 id 记 not_in_candidates（不报错、不静默忽略）。能力位：schema 含本字段才允许携带 exclude 真删"
                    },
                    "dry_run": { "type": "boolean", "description": "默认 true：只列候选、不删不写；false 表示要求真删（还需 confirm=true）" },
                    "confirm": { "type": "boolean", "description": "默认 false：真删必须显式 true；单独 true 不隐含真删" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "description": "本批最多处理的候选数，默认 100；在 status 过滤与 exclude 排除之后生效" }
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

#[cfg(test)]
mod receipt_tests {
    use super::*;

    // 代码保证：未删除条目的 state 只能来自 RetainedState，构造不出 "deleted"
    #[test]
    fn retained_entries_never_use_deleted_state() {
        let rule = json!({
            "older_than_days": 365,
            "status_filter": null,
            "limit": 100,
            "batch_id": "b-test"
        });
        let fingerprint = json!({ "status": "ok", "handback_sha256": "abc" });
        let at_source = retained_entry(
            RetainedState::AtSource,
            "t",
            1,
            "runs/t/attempt-1",
            None,
            "move failed",
            &rule,
            &fingerprint,
        );
        let in_trash = retained_entry(
            RetainedState::InTrash,
            "t",
            1,
            "runs/t/attempt-1",
            Some("runs/.prune-trash/b-test/t-attempt-1"),
            "cleanup failed",
            &rule,
            &fingerprint,
        );
        assert_eq!(at_source["state"], "retained_at_source");
        assert_eq!(in_trash["state"], "trashed_not_deleted");
        for entry in [&at_source, &in_trash] {
            assert_ne!(entry["state"], "deleted", "未删除条目不得自称已删除");
            assert!(
                !entry.to_string().to_lowercase().contains("\"deleted\""),
                "未删除条目不得出现 deleted 字样：{entry}"
            );
            assert_eq!(entry["rule"]["batch_id"], "b-test", "rule 必须自包含");
            assert_eq!(entry["fingerprint"]["status"], "ok");
        }
        assert_eq!(at_source["trash_path"], Value::Null);
        assert!(in_trash["trash_path"].as_str().is_some());
    }

    // 缺文件时指纹必须写 null，不能编造值
    #[test]
    fn missing_files_yield_null_fingerprint_parts() {
        let missing = std::env::temp_dir().join("aw-fingerprint-no-such-file-0123456789");
        let (sha, len) = file_sha256_and_len(&missing);
        assert_eq!(sha, Value::Null);
        assert_eq!(len, None);
    }
}

/// 描述审计自动门禁：审计文档 §21 的机器可读块是唯一来源。
/// 描述/schema 改动必须同步该块（含 `tools_sha256`），否则本模块用例失败。
#[cfg(test)]
mod description_gate {
    use super::*;

    const AUDIT_DOC: &str = include_str!("../../../docs/TOOLS-DESCRIPTION-AUDIT.md");
    const BEGIN: &str = "<!-- description-gate:begin -->";
    const END: &str = "<!-- description-gate:end -->";

    fn gate_block() -> Value {
        let start = AUDIT_DOC
            .find(BEGIN)
            .unwrap_or_else(|| panic!("审计文档缺少 {BEGIN} 标记"));
        let rest = &AUDIT_DOC[start + BEGIN.len()..];
        let stop = rest
            .find(END)
            .unwrap_or_else(|| panic!("审计文档缺少 {END} 标记"));
        let raw = rest[..stop].trim();
        let raw = raw.strip_prefix("```json").unwrap_or(raw);
        let raw = raw.strip_suffix("```").unwrap_or(raw);
        serde_json::from_str(raw.trim()).expect("description-gate 块必须是合法 JSON")
    }

    fn canonical_tools_sha256() -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(tools().to_string().as_bytes()))
    }

    fn tool_by_name<'a>(list: &'a Value, name: &str) -> &'a Value {
        list.as_array()
            .expect("tools 必须是数组")
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("tools/list 缺少工具：{name}"))
    }

    fn text_at<'a>(tool: &'a Value, path: &str) -> &'a str {
        let mut current = tool;
        for segment in path.split('.') {
            current = current
                .get(segment)
                .unwrap_or_else(|| panic!("{} 缺少路径 {path}", tool["name"]));
        }
        current
            .as_str()
            .unwrap_or_else(|| panic!("{} 的 {path} 不是字符串：{current}", tool["name"]))
    }

    #[test]
    fn description_gate_snapshot_matches_audit() {
        let gate = gate_block();
        assert_eq!(gate["version"], "description-gate/1", "门禁块版本不匹配");
        let expected = gate["tools_sha256"]
            .as_str()
            .expect("tools_sha256 必须是字符串");
        let actual = canonical_tools_sha256();
        assert_eq!(
            actual, expected,
            "tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际={actual}"
        );
    }

    #[test]
    fn description_gate_rows_hold_on_tools() {
        let gate = gate_block();
        let list = tools();
        let rows = gate["rows"].as_array().expect("rows 必须是数组");
        let mut covered: Vec<String> = Vec::new();
        for row in rows {
            let id = row["id"].as_str().expect("门禁行缺少 id");
            let name = row["tool"]
                .as_str()
                .unwrap_or_else(|| panic!("{id} 缺少 tool"));
            let path = row["path"]
                .as_str()
                .unwrap_or_else(|| panic!("{id} 缺少 path"));
            let tool = tool_by_name(&list, name);
            let text = text_at(tool, path);
            for phrase in row["contains"].as_array().unwrap_or_else(|| {
                panic!("{id} 缺少 contains 数组")
            }) {
                let phrase = phrase
                    .as_str()
                    .unwrap_or_else(|| panic!("{id} 的 contains 必须是字符串"));
                assert!(
                    text.contains(phrase),
                    "{id}：{name} 的 {path} 缺少必需措辞「{phrase}」：{text}"
                );
            }
            for phrase in row["not_contains"].as_array().unwrap_or_else(|| {
                panic!("{id} 缺少 not_contains 数组")
            }) {
                let phrase = phrase
                    .as_str()
                    .unwrap_or_else(|| panic!("{id} 的 not_contains 必须是字符串"));
                assert!(
                    !text.contains(phrase),
                    "{id}：{name} 的 {path} 不得包含「{phrase}」：{text}"
                );
            }
            if path == "description" {
                covered.push(name.to_string());
            }
        }
        for tool in list.as_array().expect("tools 必须是数组") {
            let name = tool["name"].as_str().expect("工具缺少 name");
            assert!(
                covered.iter().any(|item| item == name),
                "工具 {name} 的 description 没有任何门禁行覆盖"
            );
        }
    }

    #[test]
    fn description_gate_timeout_three_states_match_description() {
        use process_scope::CleanupOutcome;
        let list = tools();
        let description = text_at(
            tool_by_name(&list, "task.execute"),
            "inputSchema.properties.timeout_secs.description",
        );
        let cleaned = timeout_reason(30, Some(CleanupOutcome::Cleaned));
        let still_alive = timeout_reason(30, Some(CleanupOutcome::StillAlive));
        let unavailable = timeout_reason(30, Some(CleanupOutcome::Unavailable));

        for phrase in [
            "已清理受控集合",
            "受控集合内仍有存活进程",
            "受控集合未能建立或无法查询",
            "仅向直接子进程发送终止信号",
            "逃逸进程是否仍在运行未验证",
        ] {
            assert!(
                description.contains(phrase),
                "timeout_secs 描述缺少三态措辞「{phrase}」：{description}"
            );
        }
        assert!(cleaned.contains("已清理受控集合"), "{cleaned}");
        assert!(
            cleaned.contains("逃逸进程是否仍在运行未验证"),
            "{cleaned}"
        );
        assert!(
            !cleaned.contains("受控集合内仍有存活进程"),
            "已清理分支不得同时声明仍有存活进程：{cleaned}"
        );
        assert!(
            still_alive.contains("受控集合内仍有存活进程"),
            "{still_alive}"
        );
        assert!(
            !still_alive.contains("已清理受控集合"),
            "仍有存活进程时不得自称已清理：{still_alive}"
        );
        assert!(
            unavailable.contains("受控集合未能建立或无法查询"),
            "{unavailable}"
        );
        assert!(
            unavailable.contains("仅向直接子进程发送终止信号"),
            "{unavailable}"
        );
        assert!(
            !unavailable.contains("已清理受控集合"),
            "无法查询时不得自称已清理：{unavailable}"
        );
        for reason in [&cleaned, &still_alive, &unavailable] {
            for forbidden in [
                "已终止进程树",
                "无残留",
                "会终止整棵进程树",
                "保证清理全部",
            ] {
                assert!(
                    !reason.contains(forbidden),
                    "超时文案不得含「{forbidden}」：{reason}"
                );
            }
        }
    }
}
