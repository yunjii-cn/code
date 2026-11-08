// aw-mcp-bridge 方法分发：只读 health/status + 执行层 task.execute/receipt.get
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "aw-mcp-bridge";
/// 契约版本：与 docs/PM-AW-DISPATCH-CONTRACT.md 标题一致，供 PM 在接口上机器校验对接版本。
pub const CONTRACT_VERSION: &str = "1.51";
/// 会话审计目录名：以 _ 开头，与合法 task_id（必须字母或数字开头）不可能撞名
const SESSION_DIR: &str = "_session";
/// 活跃标记目录：以 . 开头，`is_task_dir_name` 会排除它，runs.list / runs.prune 不会把它当任务
const INFLIGHT_DIR: &str = ".inflight";
/// 任务历史台账目录：即使 task 目录被整体清理，这里仍保留"曾经存在过"的事实
const TASK_HISTORY_DIR: &str = ".task-history";
const INFLIGHT_VERSION: &str = "inflight-marker/1";
/// 标记过期宽限：超过 deadlineAt 且 pid 已不可见才判定为残留
const INFLIGHT_STALE_GRACE_SECS: i64 = 120;
const INFLIGHT_REPORT_VERSION: &str = "inflight-report/1";
const INFLIGHT_SWEEP_VERSION: &str = "inflight-sweep/1";
/// 顺带清扫的 append-only 审计：谁触发、清了哪些 task_id、依据哪个 stale_reason
const INFLIGHT_SWEEP_AUDIT: &str = "inflight-sweep.jsonl";
/// 认领临时名序号：同一进程内多个清扫线程并发 rename 时保证 .tmp 名唯一
static INFLIGHT_CLAIM_SEQ: AtomicU64 = AtomicU64::new(0);
/// prune 批次序号：批次名原本只有「毫秒 + pid」，同进程同一毫秒的并发 prune 会共用
/// `.prune-trash/<batch>` 目录，导致两个认领者 rename 到同一目的地（Windows 实测会
/// 出现 staged 存在但删除失败，被记成 trashed_not_deleted）。加序号保证进程内唯一。
static PRUNE_BATCH_SEQ: AtomicU64 = AtomicU64::new(0);
/// 自动清扫周期来源：未设置默认每小时；`0` 仅启动时一次；`off` 完全关闭；正整数按秒
pub const INFLIGHT_AUTO_SWEEP_ENV: &str = "AW_INFLIGHT_SWEEP_SECS";
/// 默认周期：每小时一次。选默认开启是因为一次清扫只是只读扫描目录，成本低，
/// 而只在启动时清扫会让长驻进程里的残标一直躺着（本条目要解决的正是这个洞）。
const INFLIGHT_AUTO_SWEEP_DEFAULT_SECS: u64 = 3600;
const CAPABILITIES_VERSION: &str = "capabilities/1";
/// pid 已消失后的残留宽限：给正常退出留出收尾时间，避免把刚结束的守卫误判为残留
const INFLIGHT_PID_MISSING_GRACE_SECS: i64 = 5;
/// `runs/_session/initialize-*.json` 保留天数：默认 30 天。阈值来自真实链路的实测增长
/// （见 docs/PM-AW-DISPATCH-CONTRACT.md §8），可用 AW_SESSION_RETENTION_DAYS 覆盖。
pub const SESSION_RETENTION_ENV: &str = "AW_SESSION_RETENTION_DAYS";
pub const SESSION_RETENTION_DEFAULT_DAYS: i64 = 30;
const SESSION_RETENTION_VERSION: &str = "session-retention/1";
const SESSION_RETENTION_AUDIT: &str = "session-retention.jsonl";
/// 取消等待上限：终止本身最多 3s（CLEANUP_DEADLINE），留给回执落盘与调度余量
const CANCEL_WAIT_SECS: u64 = 10;
/// 跨进程取消请求落盘目录与格式；请求文件本身就是「已落盘/已读到/已完成」三态的唯一记录。
const CANCEL_DIR: &str = ".cancel";
const CANCEL_VERSION: &str = "cancel-request/1";
const CANCEL_REQUEST_AUDIT: &str = "cancel-requests.jsonl";
const CANCEL_REQUEST_SWEEP_VERSION: &str = "cancel-requests/1";
/// 目标进程已死但取消请求仍停留在 pending/acknowledged 时，至少等这么久才清扫，避免与执行侧收尾竞态
const CANCEL_REQUEST_STALE_GRACE_SECS: i64 = 5;
/// 终态取消请求（done/ignored）的保留天数：只做磁盘卫生，不参与任何取消判定
const CANCEL_REQUEST_RETENTION_DAYS: i64 = 30;
/// cgroup 作用域归属记录：说明某个 `aw-mcp-bridge-*` 作用域属于哪一轮已知 attempt
#[cfg(target_os = "linux")]
const SCOPE_OWNER_DIR: &str = ".cgroup-scopes";
#[cfg(target_os = "linux")]
const SCOPE_OWNER_VERSION: &str = "cgroup-scope/1";
#[cfg(target_os = "linux")]
const CGROUP_RECLAIM_AUDIT: &str = "cgroup-reclaim.jsonl";
const CGROUP_RECLAIM_VERSION: &str = "cgroup-reclaim/1";
/// 启动回收判据的作用域名前缀；只有这个名字形状才进入判定，其它目录一律不碰
#[cfg(target_os = "linux")]
const CGROUP_SCOPE_PREFIX: &str = "aw-mcp-bridge-";
/// 回收等待成员退出的上限（与 CLEANUP_DEADLINE 同口径）
#[cfg(target_os = "linux")]
const CGROUP_RECLAIM_DEADLINE_SECS: u64 = 3;

// 由 build.rs 注入，缺失时为 "unknown"（不使用当前时间兜底，保证可复现构建）
pub const BUILD_TIME: &str = env!("AW_BUILD_TIME");
pub const DESKTOP_BUILD_TIME: &str = env!("AW_DESKTOP_BUILD_TIME");

/// 一次可取消执行的句柄。取消只置位并唤醒，终止动作仍由持有子进程的那条执行线程完成，
/// 这样 `Supervisor`（非线程安全）不需要跨线程共享，也不会出现两条线程同时终止同一棵树。
struct CancelToken {
    canceled: AtomicBool,
    reason: Mutex<Option<String>>,
    finished: Mutex<bool>,
    finished_cv: Condvar,
}

impl CancelToken {
    fn new() -> Self {
        Self {
            canceled: AtomicBool::new(false),
            reason: Mutex::new(None),
            finished: Mutex::new(false),
            finished_cv: Condvar::new(),
        }
    }

    fn request(&self, reason: Option<String>) {
        let mut slot = self
            .reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if slot.is_none() {
            *slot = reason;
        }
        drop(slot);
        self.canceled.store(true, Ordering::SeqCst);
    }

    fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }

    fn reason(&self) -> Option<String> {
        self.reason
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn finish(&self) {
        let mut done = self
            .finished
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *done = true;
        drop(done);
        self.finished_cv.notify_all();
    }

    /// 等执行线程把回执落盘并退出；超时返回 false，调用方不得据此宣称已清理
    fn wait_finished(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut done = self
            .finished
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !*done {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let (guard, _) = self
                .finished_cv
                .wait_timeout(done, deadline - now)
                .unwrap_or_else(|error| error.into_inner());
            done = guard;
        }
        true
    }
}

/// 注册守卫：只有真正插入登记表的那次调用负责移除并唤醒等待者；
/// 同一个 task_id 的重投（replay）不会抢走胜者的句柄。
struct CancelRegistration<'a> {
    server: &'a Server,
    task_id: String,
    token: Arc<CancelToken>,
    owner: bool,
}

impl Drop for CancelRegistration<'_> {
    fn drop(&mut self) {
        if !self.owner {
            return;
        }
        {
            let mut map = self
                .server
                .cancel_tokens
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if map
                .get(&self.task_id)
                .map(|token| Arc::ptr_eq(token, &self.token))
                .unwrap_or(false)
            {
                map.remove(&self.task_id);
            }
        }
        self.token.finish();
    }
}

pub struct Server {
    settings_path: PathBuf,
    runs_dir: PathBuf,
    /// 连接级计数：进程内没有"会话"概念，只有一条 stdio 连接。
    connection_started: AtomicBool,
    connections_peak: AtomicU64,
    initialize_total: AtomicU64,
    /// 本进程正在执行、可被 task.cancel 点名的 task_id。跨进程不可见（取消只能由控住
    /// 子进程的那个 bridge 执行），因此这里只登记本进程内的执行。
    cancel_tokens: Mutex<HashMap<String, Arc<CancelToken>>>,
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
            cancel_tokens: Mutex::new(HashMap::new()),
        }
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    /// 登记一次可取消执行。返回 (句柄, 是否由本次调用持有)；
    /// 同 task_id 已在执行时复用既有句柄，重投方不拥有移除权。
    fn register_cancel(&self, task_id: &str) -> (Arc<CancelToken>, bool) {
        let mut map = self
            .cancel_tokens
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(existing) = map.get(task_id) {
            return (Arc::clone(existing), false);
        }
        let token = Arc::new(CancelToken::new());
        map.insert(task_id.to_string(), Arc::clone(&token));
        (token, true)
    }

    /// 服务启动时自动清扫一次：与显式 clean / task_start / prune 共用
    /// sweep_inflight_markers 与 stale_rule，审计 trigger=startup。
    /// 清扫失败（含审计写不进去）只记 stderr，不阻断服务启动；审计失败时本轮零删除。
    pub fn run_startup_sweep(&self) {
        self.auto_sweep("startup");
    }

    /// 周期清扫：默认每小时一次（见 auto_sweep_schedule），线程随进程退出。
    /// 与启动清扫共用同一实现、同一审计，只是 trigger=periodic。
    pub fn start_periodic_sweep(self: &Arc<Self>, interval: Duration) {
        let server = Arc::clone(self);
        if let Err(error) = std::thread::Builder::new()
            .name("aw-inflight-sweep".to_string())
            .spawn(move || loop {
                std::thread::sleep(interval);
                server.auto_sweep("periodic");
            })
        {
            eprintln!("[inflight-sweep] trigger=periodic 线程启动失败：{error}");
        }
    }

    /// 自动清扫只负责调用同一个 sweep_inflight_markers；不在这里重复任何判据
    fn auto_sweep(&self, trigger: &str) {
        // 启动回收放在最前：先收掉上一轮 bridge 猝死留下的作用域，再清标记/记录。
        // 证据不足时只写 kept 审计、不发信号；cgroup 不可用则如实报 unsupported。
        if trigger == "startup" {
            match self.reclaim_stale_cgroup_scopes(trigger) {
                Ok(report) => {
                    let reclaimed = report["reclaimed_total"].as_u64().unwrap_or(0);
                    let removed_empty = report["removed_empty_total"].as_u64().unwrap_or(0);
                    let kept = report["kept_total"].as_u64().unwrap_or(0);
                    let failed = report["failed_total"].as_u64().unwrap_or(0);
                    if report["state"].as_str() == Some("ready") {
                        eprintln!(
                            "[cgroup-reclaim] trigger={trigger} reclaimed={reclaimed} removed_empty={removed_empty} kept={kept} failed={failed}"
                        );
                    }
                }
                Err(error) => {
                    eprintln!("[cgroup-reclaim] trigger={trigger} error={error}");
                }
            }
        }
        match self.sweep_inflight_markers(trigger, None, false) {
            Ok(report) => {
                let removed = report["removed_total"].as_u64().unwrap_or(0);
                let failed = report["failed_total"].as_u64().unwrap_or(0);
                if removed > 0 || failed > 0 {
                    eprintln!(
                        "[inflight-sweep] trigger={trigger} removed={removed} failed={failed}"
                    );
                }
            }
            Err(error) => {
                eprintln!("[inflight-sweep] trigger={trigger} error={error}");
            }
        }
        // `_session/initialize-*.json` 与任务目录口径不同：按天数保留，独立审计。
        // 清扫失败同样只记 stderr，不影响服务启动/周期线程。
        match self.sweep_session_records(trigger) {
            Ok(report) => {
                let removed = report["removed_total"].as_u64().unwrap_or(0);
                let failed = report["failed_total"].as_u64().unwrap_or(0);
                if removed > 0 || failed > 0 {
                    eprintln!(
                        "[session-retention] trigger={trigger} removed={removed} failed={failed}"
                    );
                }
            }
            Err(error) => {
                eprintln!("[session-retention] trigger={trigger} error={error}");
            }
        }
        // 取消请求目录卫生：只清目标已死且过宽限、或终态超保留期的文件；每次删除先落审计
        if trigger == "startup" {
            match self.sweep_cancel_requests(trigger) {
                Ok(report) => {
                    let removed = report["removed_total"].as_u64().unwrap_or(0);
                    if removed > 0 {
                        eprintln!("[cancel-request-sweep] trigger={trigger} removed={removed}");
                    }
                }
                Err(error) => {
                    eprintln!("[cancel-request-sweep] trigger={trigger} error={error}");
                }
            }
        }
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
            "capabilities" => (capabilities_payload(), false),
            "runs.inflight" => match self.runs_inflight(&arguments) {
                Ok(payload) => (payload, false),
                Err(error) => (json!({ "error": error }), true),
            },
            "task.execute" => match self.execute_task(&arguments) {
                Ok(payload) => (payload, false),
                Err(error) => (json!({ "error": error }), true),
            },
            "task.cancel" => match self.task_cancel(&arguments) {
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
            "contract_version": CONTRACT_VERSION,
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
                .cmp(
                    right
                        .get("receivedAt")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                )
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
            let entries =
                fs::read_dir(&self.runs_dir).map_err(|e| format!("读取 runs 目录失败：{e}"))?;
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
            Some(value) => value
                .as_u64()
                .ok_or_else(|| "invalid_retention: limit 必须是 1..=100 的整数".to_string())?,
        };
        if !(1..=100).contains(&limit) {
            return Err("invalid_retention: limit 必须在 1..=100 之间".to_string());
        }
        let status_filter = match args.get("status") {
            None => None,
            Some(value) => {
                let array = value.as_array().ok_or_else(|| {
                    "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked/canceled"
                        .to_string()
                })?;
                if array.is_empty() {
                    return Err(
                        "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked/canceled"
                            .to_string(),
                    );
                }
                let mut parsed: Vec<String> = Vec::new();
                for item in array {
                    let Some(text) = item.as_str() else {
                        return Err(
                            "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked/canceled"
                                .to_string(),
                        );
                    };
                    if !matches!(text, "ok" | "failed" | "blocked" | "canceled") {
                        return Err(
                            "invalid_retention: status 必须是非空数组，元素只能取 ok/failed/blocked/canceled"
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
            json!(exclusions
                .iter()
                .map(|exclusion| json!({
                    "task_id": exclusion.task_id,
                    "attempt": exclusion.attempt
                }))
                .collect::<Vec<Value>>())
        };

        let evaluated_at = now_stamp();
        // prune 真删前先按既有判据顺带清扫残标：先记账后删除，审计失败则不继续删目录
        let inflight_sweep = self.sweep_inflight_markers("prune", None, dry_run)?;
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
        // 1) deleted + failed + trashed + lost_race + excluded_total == batch_candidates_total
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
                "inflight_sweep": inflight_sweep,
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
                "inflight_sweep": inflight_sweep,
                "candidates_total": total_candidates,
                "batch_candidates_total": batch_candidates_total,
                "not_processed_total": not_processed_total,
                "truncated": truncated,
                "deleted_total": 0,
                "failed_total": 0,
                "trashed_total": 0,
                "lost_race_total": 0,
                "exclude_requests_total": exclude_requests_total,
                "excluded_total": excluded_total,
                "not_in_candidates_total": not_in_candidates_total,
                "deleted": [],
                "failed": [],
                "trashed": [],
                "lost_race": [],
                "excluded": excluded
            }));
        }

        let batch_id = format!(
            "prune-{}-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            std::process::id(),
            PRUNE_BATCH_SEQ.fetch_add(1, Ordering::Relaxed)
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
        let mut lost_race: Vec<Value> = Vec::new();
        // 每条条目自包含整批 rule，跨批次核对不必回连顶层
        let rule = json!({
            "older_than_days": older_than_days,
            "status_filter": status_filter_json.clone(),
            "limit": limit,
            "batch_id": batch_id.clone()
        });
        for candidate in &batch {
            let task_id = candidate["task_id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
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
                DeleteOutcome::RetainedInTrash { trash_path, error } => {
                    trashed.push(retained_entry(
                        RetainedState::InTrash,
                        &task_id,
                        attempt,
                        &source_path,
                        Some(&trash_path),
                        &error,
                        &rule,
                        &fingerprint,
                    ))
                }
                DeleteOutcome::LostRace { error } => lost_race.push(lost_race_entry(
                    &task_id,
                    attempt,
                    &source_path,
                    &error,
                    &rule,
                    &fingerprint,
                )),
            }
        }
        let deleted_total = deleted.len() as u64;
        let failed_total = failed.len() as u64;
        let trashed_total = trashed.len() as u64;
        let lost_race_total = lost_race.len() as u64;
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
            "lost_race": lost_race.clone(),
            "deleted_total": deleted_total,
            "failed_total": failed_total,
            "trashed_total": trashed_total,
            "lost_race_total": lost_race_total,
            "exclude_requests_total": exclude_requests_total,
            "excluded_total": excluded_total,
            "not_in_candidates_total": not_in_candidates_total,
            "batch_candidates_total": batch_candidates_total,
            "excluded": excluded
        });
        if let Err(error) = self.append_prune_record(&result) {
            return Err(format!(
                "prune_audit_failed: 已删除 {deleted_total} 个、失败 {failed_total} 个、暂存 {trashed_total} 个、竞态失手 {lost_race_total} 个；结果行写入 prune.jsonl 失败：{error}"
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
            "inflight_sweep": inflight_sweep,
            "candidates_total": total_candidates,
            "batch_candidates_total": batch_candidates_total,
            "not_processed_total": not_processed_total,
            "truncated": truncated,
            "deleted_total": deleted_total,
            "failed_total": failed_total,
            "trashed_total": trashed_total,
            "lost_race_total": lost_race_total,
            "exclude_requests_total": exclude_requests_total,
            "excluded_total": excluded_total,
            "not_in_candidates_total": not_in_candidates_total,
            "deleted": deleted,
            "failed": failed,
            "trashed": trashed,
            "lost_race": lost_race,
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
                    if !matches!(value, "ok" | "failed" | "blocked" | "canceled") {
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
        let source = self
            .runs_dir
            .join(task_id)
            .join(format!("attempt-{attempt}"));
        let stage_root = self.runs_dir.join(".prune-trash").join(batch_id);
        if let Err(error) = fs::create_dir_all(&stage_root) {
            return DeleteOutcome::RetainedAtSource {
                error: format!("创建暂存目录失败（attempt 原样未动）：{error}"),
            };
        }
        let staged = stage_root.join(format!("{task_id}-attempt-{attempt}"));
        let staged_rel = format!("runs/.prune-trash/{batch_id}/{task_id}-attempt-{attempt}");
        #[cfg(test)]
        Self::run_prune_before_rename_hook(&source, &staged);
        if let Err(error) = fs::rename(&source, &staged) {
            let _ = fs::remove_dir(&stage_root);
            // 败者不一定拿到 NotFound：Windows 上并发改名的另一半可能返回
            // AccessDenied / SharingViolation。归属以事实为准——源已不在，
            // 就是被另一方认领带走，不能报成「原样未动」。
            let mut lost_race = rename_failure_is_lost_race(&error, source.exists());
            if !lost_race && error.kind() != std::io::ErrorKind::NotFound {
                // 并发窗口：败者拿到 SharingViolation 时，胜者的 MoveFile 可能还没收尾，
                // 此刻 source.exists() 仍为 true。给胜者一个很短的就位窗口再定归属；
                // 窗口内源消失 = 被认领带走（lost_race），始终不消失 = 真·原样未动。
                for _ in 0..50 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    if !source.exists() {
                        lost_race = true;
                        break;
                    }
                }
            }
            if lost_race {
                return DeleteOutcome::LostRace {
                    error: format!(
                        "claim_lost_source_missing: 源目录已被另一认领者改名带走（attempt-{attempt}），本进程零删除：{error}"
                    ),
                };
            }
            return DeleteOutcome::RetainedAtSource {
                error: format!("整体移动失败（attempt 原样未动）：{error}"),
            };
        }
        #[cfg(test)]
        Self::run_prune_after_rename_hook(&source, &staged);
        // Windows 实测：并发 rename 同一源可能都返回 Ok，但只有一个目的目录产生。
        // 归属以目的目录是否存在为准，而不是 rename 的返回值。
        match fs::symlink_metadata(&staged) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let _ = fs::remove_dir(&stage_root);
                return DeleteOutcome::LostRace {
                    error: format!(
                        "claim_lost_after_rename: rename 返回成功但 {staged_rel} 不存在（同源并发改名被另一方认领），本进程零删除"
                    ),
                };
            }
            Err(error) => {
                let _ = fs::remove_dir(&stage_root);
                return DeleteOutcome::LostRace {
                    error: format!(
                        "claim_unverified_after_rename: 无法确认 {staged_rel} 是否存在，本进程零删除：{error}"
                    ),
                };
            }
        }
        if let Err(error) = fs::remove_dir_all(&staged) {
            if error.kind() == std::io::ErrorKind::NotFound {
                let _ = fs::remove_dir(&stage_root);
                return DeleteOutcome::LostRace {
                    error: format!(
                        "claim_lost_after_rename: {staged_rel} 在删除前已被另一方清走，本进程零删除：{error}"
                    ),
                };
            }
            // Windows 并发改名可能让两方都拿到 Ok、暂存名指向同一目录；另一方先删成功时，
            // 本方的 remove_dir_all 会报 AccessDenied，但目录随即消失。归属仍以事实为准：
            // 暂存已不存在 = 本方零删除（lost_race）；仍存在 = 数据确实留在暂存（保留上报）。
            let mut vanished = false;
            for _ in 0..50 {
                if !staged.exists() {
                    vanished = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            if vanished {
                let _ = fs::remove_dir(&stage_root);
                return DeleteOutcome::LostRace {
                    error: format!(
                        "claim_lost_after_rename: {staged_rel} 在删除失败后消失（被另一方清走），本进程零删除：{error}"
                    ),
                };
            }
            let error_text =
                format!("已整链移至 {staged_rel}，但删除暂存失败（数据未丢）：{error}");
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

    /// 并发观测点（仅测试编译）：在 attempt 目录 rename 之前对齐两个认领者，
    /// 让「同一源、同时 rename」的窗口可稳定复现。
    #[cfg(test)]
    fn run_prune_before_rename_hook(source: &Path, staged: &Path) {
        let hook = {
            let armed = PRUNE_BEFORE_RENAME_HOOK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            armed
                .as_ref()
                .filter(|armed| armed.target == source)
                .map(|armed| armed.hook)
        };
        if let Some(hook) = hook {
            hook(source, staged);
        }
    }

    /// 并发观测点（仅测试编译）：在 rename 返回 Ok 之后、目的复核之前注入，
    /// 用于复现 Windows「双方都 Ok、只有一个目的目录」的观测事实。
    #[cfg(test)]
    fn run_prune_after_rename_hook(source: &Path, staged: &Path) {
        let hook = {
            let armed = PRUNE_AFTER_RENAME_HOOK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            armed
                .as_ref()
                .filter(|armed| armed.target == source)
                .map(|armed| armed.hook)
        };
        if let Some(hook) = hook {
            hook(source, staged);
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
        fs::create_dir_all(&dir)
            .map_err(|error| format!("inflight_marker_failed: 创建活跃标记目录失败：{error}"))?;
        let path = dir.join(format!("{task_id}.json"));
        let deadline = inflight_deadline(args, mode);
        for _ in 0..3 {
            let started_at = now_stamp();
            let record = json!({
                "version": INFLIGHT_VERSION,
                "task_id": task_id,
                "pid": std::process::id(),
                "mode": mode,
                "state": "active",
                "startedAt": started_at.clone(),
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
                        .write_all(
                            serde_json::to_string(&record)
                                .unwrap_or_default()
                                .as_bytes(),
                        )
                        .and_then(|_| file.sync_all())
                    {
                        let _ = fs::remove_file(&path);
                        return Err(format!("inflight_marker_failed: 写入活跃标记失败：{error}"));
                    }
                    return Ok(Some(InflightGuard { path, started_at }));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // 同一 task_id 再次启动：先按既有判据顺带清扫残标并留审计；
                    // 清掉后重试 create_new，并发清理时仍由 create_new 决出唯一持有者。
                    let sweep = self.sweep_inflight_markers("task_start", Some(task_id), false)?;
                    if sweep["removed_total"].as_u64().unwrap_or(0) > 0 {
                        continue;
                    }
                    // 活跃或不可读（无法判定）一律不删，本次按既有语义视为已在执行
                    return Ok(None);
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
        let path = self
            .runs_dir
            .join(INFLIGHT_DIR)
            .join(format!("{task_id}.json"));
        let mut record = read_json(&path).ok()?;
        let stale = marker_is_stale(&record, chrono::Utc::now());
        if let Some(object) = record.as_object_mut() {
            object.insert("stale".to_string(), json!(stale));
        }
        Some(record)
    }

    /// 活跃标记的只读查询与残留清理：只删 stale=true 的标记，读不懂的一律保留
    fn runs_inflight(&self, args: &Value) -> Result<Value, String> {
        let action = match args.get("action").and_then(Value::as_str) {
            Some("list") => "list",
            Some("clean") => "clean",
            Some(other) => {
                return Err(format!(
                    "invalid_arguments: action 必须是 list 或 clean，收到：{other}"
                ))
            }
            None => return Err("invalid_arguments: action 必须显式传入 list 或 clean".to_string()),
        };
        let task_filter = match args.get("task_id") {
            None => None,
            Some(value) => {
                let id = value
                    .as_str()
                    .ok_or_else(|| "invalid_arguments: task_id 必须是字符串".to_string())?;
                if !is_task_dir_name(id) {
                    return Err(format!(
                        "invalid_arguments: task_id 不合法（仅字母数字与 . _ -，且不以 _ 或 . 开头）：{id}"
                    ));
                }
                Some(id.to_string())
            }
        };
        let dry_run = match args.get("dry_run") {
            None => true,
            Some(value) => value
                .as_bool()
                .ok_or_else(|| "invalid_arguments: dry_run 必须是布尔值".to_string())?,
        };
        let confirm = match args.get("confirm") {
            None => false,
            Some(value) => value.as_bool().ok_or_else(|| {
                "confirm_required: confirm 必须是布尔值；真删需要显式 confirm=true".to_string()
            })?,
        };
        if action == "clean" && !dry_run && !confirm {
            return Err(
                "confirm_required: 真删必须同时显式 dry_run=false 与 confirm=true；本轮未删除任何标记"
                    .to_string(),
            );
        }

        let (markers, unreadable, dir_exists) = self.scan_inflight_markers(task_filter.as_deref());
        let stale_total = markers
            .iter()
            .filter(|marker| marker.assessment["stale"] == json!(true))
            .count() as u64;
        let state = if !dir_exists {
            "missing"
        } else if markers.is_empty() && unreadable.is_empty() {
            "empty"
        } else {
            "ready"
        };

        if action == "list" {
            return Ok(json!({
                "version": INFLIGHT_REPORT_VERSION,
                "action": "list",
                "state": state,
                "directory": "runs/.inflight",
                "stale_rule": inflight_stale_rule(),
                "liveness_rule": inflight_liveness_rule(),
                "sweep_rule": inflight_sweep_rule(),
                "total": markers.len() as u64,
                "stale_total": stale_total,
                "active_total": markers.len() as u64 - stale_total,
                "unreadable_total": unreadable.len() as u64,
                "markers": markers.iter().map(marker_report).collect::<Vec<Value>>(),
                "unreadable": unreadable
            }));
        }

        if dry_run {
            let sweep = self.sweep_inflight_markers("clean", task_filter.as_deref(), true)?;
            return Ok(json!({
                "version": INFLIGHT_REPORT_VERSION,
                "action": "clean",
                "dry_run": true,
                "state": sweep["state"],
                "directory": "runs/.inflight",
                "stale_rule": inflight_stale_rule(),
                "sweep_rule": inflight_sweep_rule(),
                "total": sweep["total"],
                "stale_total": sweep["stale_total"],
                "active_total": sweep["active_total"],
                "unreadable_total": sweep["unreadable_total"],
                "would_remove_total": sweep["would_remove_total"],
                "removed_total": 0,
                "would_remove": sweep["would_remove"],
                "kept": sweep["kept"]
            }));
        }

        let sweep = self.sweep_inflight_markers("clean", task_filter.as_deref(), false)?;
        Ok(json!({
            "version": INFLIGHT_REPORT_VERSION,
            "action": "clean",
            "dry_run": false,
            "trigger": "clean",
            "state": sweep["state"],
            "directory": "runs/.inflight",
            "stale_rule": inflight_stale_rule(),
            "sweep_rule": inflight_sweep_rule(),
            "total": sweep["total"],
            "stale_total": sweep["stale_total"],
            "active_total": sweep["active_total"],
            "unreadable_total": sweep["unreadable_total"],
            "would_remove_total": sweep["would_remove_total"],
            "removed_total": sweep["removed_total"],
            "kept_total": sweep["kept_total"],
            "failed_total": sweep["failed_total"],
            "would_remove": sweep["would_remove"],
            "removed": sweep["removed"],
            "kept": sweep["kept"],
            "failed": sweep["failed"]
        }))
    }

    /// 顺带清扫：三个显式路径（同 task_id 重启 / prune 真删前 / clean）+ 启动/周期自动触发，
    /// 全部走这一个实现。判据沿用 assess_inflight_marker：pid 存活优先于 deadline；
    /// unreadable 一律保留。真删前先写 plan，逐条 rename 到 .tmp 认领，
    /// 再对认领副本重跑同一份 stale_rule：仍 stale 才删，已不 stale 就地还原并记 not_mine，
    /// 认领副本已不存在记 lost_race。
    fn sweep_inflight_markers(
        &self,
        trigger: &str,
        filter: Option<&str>,
        dry_run: bool,
    ) -> Result<Value, String> {
        let (markers, unreadable, dir_exists) = self.scan_inflight_markers(filter);
        let total = markers.len() as u64;
        let stale_total = markers
            .iter()
            .filter(|marker| marker.assessment["stale"] == json!(true))
            .count() as u64;
        let active_total = total - stale_total;
        let unreadable_total = unreadable.len() as u64;
        let state = if !dir_exists {
            "missing"
        } else if markers.is_empty() && unreadable.is_empty() {
            "empty"
        } else {
            "ready"
        };
        let would_remove: Vec<Value> = markers
            .iter()
            .filter(|marker| marker.assessment["stale"] == json!(true))
            .map(|marker| {
                json!({
                    "task_id": marker.task_id,
                    "file": marker.file,
                    "pid": marker.assessment["pid"],
                    "stale_reason": marker.assessment["stale_reason"],
                    "path": format!("runs/.inflight/{}", marker.file)
                })
            })
            .collect();
        let mut kept: Vec<Value> = Vec::new();
        for marker in &markers {
            if marker.assessment["stale"] == json!(true) {
                continue;
            }
            kept.push(json!({
                "task_id": marker.task_id,
                "file": marker.file,
                "pid": marker.assessment["pid"],
                "stale_reason": marker.assessment["stale_reason"],
                "reason": "not_stale"
            }));
        }
        for entry in &unreadable {
            kept.push(json!({
                "task_id": entry["task_id"],
                "file": entry["file"],
                "pid": Value::Null,
                "stale_reason": "unreadable",
                "reason": "unreadable"
            }));
        }

        if dry_run || stale_total == 0 {
            return Ok(json!({
                "version": INFLIGHT_SWEEP_VERSION,
                "trigger": trigger,
                "dry_run": dry_run,
                "state": state,
                "directory": "runs/.inflight",
                "total": total,
                "stale_total": stale_total,
                "active_total": active_total,
                "unreadable_total": unreadable_total,
                "would_remove_total": would_remove.len() as u64,
                "removed_total": 0,
                "deleted_total": 0,
                "lost_race_total": 0,
                "not_mine_total": 0,
                "kept_total": kept.len() as u64,
                "failed_total": 0,
                "would_remove": would_remove,
                "removed": [],
                "kept": kept,
                "failed": []
            }));
        }

        // 先记账后删除：plan 行写不进去就一条都不删
        let plan = json!({
            "version": INFLIGHT_SWEEP_VERSION,
            "phase": "plan",
            "at": now_stamp(),
            "trigger": trigger,
            "task_id_filter": filter,
            "stale_total": stale_total,
            "unreadable_total": unreadable_total,
            "candidates": would_remove
        });
        self.append_inflight_sweep_record(&plan)
            .map_err(|error| format!("{error}；本轮未删除任何标记"))?;

        let mut removed: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        let now = chrono::Utc::now();
        for marker in &markers {
            if marker.assessment["stale"] != json!(true) {
                continue;
            }
            // 先原子改名为 .tmp 认领这一份文件；认领成功后再重读并重跑 stale_rule。
            // 若期间被刷新成非残标，读到的内容已不 stale，必须还原而不是删。
            let tmp = self.runs_dir.join(INFLIGHT_DIR).join(format!(
                "{}.clean-{}-{}-{}.tmp",
                marker.task_id,
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
                INFLIGHT_CLAIM_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            #[cfg(test)]
            run_sweep_before_claim_hook(&marker.path);
            #[cfg(test)]
            sweep_trace::record(
                &marker.path,
                "claim_attempt",
                format!("file={} tmp={}", marker.file, tmp.display()),
            );
            if let Err(error) = fs::rename(&marker.path, &tmp) {
                #[cfg(test)]
                sweep_trace::record(
                    &marker.path,
                    "claim_failed",
                    format!("kind={:?} error={error}", error.kind()),
                );
                // 认领失败：源已不在 → 别人先认领（lost_race，不算错误）；
                // 源还在但已不 stale → 别人就地刷新（not_mine，绝不删除）；其余才是真失败。
                match reread_marker(&marker.path, now) {
                    MarkerReread::Gone => {
                        kept.push(inflight_sweep_entry(marker, "lost_race", "lost_race"))
                    }
                    MarkerReread::NotStale => {
                        kept.push(inflight_sweep_entry(marker, "not_mine", "marker_changed"))
                    }
                    MarkerReread::Stale | MarkerReread::Unreadable => {
                        failed.push(inflight_sweep_failure(marker, error.to_string()))
                    }
                }
                continue;
            }
            #[cfg(test)]
            run_sweep_after_claim_hook(&marker.path, &tmp);
            // 认领成功后重读并重跑同一份 stale_rule：拿到的这份若已不是残标（别人刚刷新过），
            // 就地还原并记 not_mine，绝不删除。判据只认 stale，不另立身份规则。
            //
            // 注意：Windows 上多个并发 rename 可能都返回成功，但只有一个目的地真的产生文件。
            // 因此「rename 返回 Ok」不等于「这份归我」——认领副本读不到（NotFound）就是
            // 输掉了认领，按 lost_race 记账，既不算错误也不还原任何东西。
            let reread = reread_marker(&tmp, now);
            #[cfg(test)]
            sweep_trace::record(
                &marker.path,
                "reread_claim_copy",
                format!(
                    "result={reread:?} exists={} len={:?}",
                    tmp.exists(),
                    fs::metadata(&tmp).ok().map(|meta| meta.len())
                ),
            );
            #[cfg(test)]
            run_sweep_after_reread_hook(&marker.path, &tmp);
            match reread {
                MarkerReread::Gone => {
                    kept.push(inflight_sweep_entry(marker, "lost_race", "lost_race"));
                }
                MarkerReread::Stale => match fs::remove_file(&tmp) {
                    Ok(()) => {
                        #[cfg(test)]
                        sweep_trace::record(&marker.path, "claim_copy_removed", "outcome=deleted");
                        removed.push(inflight_sweep_entry(marker, "deleted", "stale"))
                    }
                    // 复核之后认领副本消失：另一个清扫者已经处置了这份文件，
                    // 这仍是「别人先认领」的同一种事实，不是错误。v1.49 修复 Windows 并发
                    // rename 下把这种情况记成 failed（审计里出现假失败）。
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        #[cfg(test)]
                        sweep_trace::record(
                            &marker.path,
                            "claim_copy_vanished",
                            "outcome=lost_race",
                        );
                        kept.push(inflight_sweep_entry(marker, "lost_race", "claim_copy_gone"));
                    }
                    Err(error) => {
                        #[cfg(test)]
                        sweep_trace::record(
                            &marker.path,
                            "claim_copy_remove_failed",
                            format!("error={error}"),
                        );
                        // rename 会覆盖原位的新标记；还原同样只能 create-if-absent。
                        let restore = match restore_without_clobber(&tmp, &marker.path) {
                            Ok(RestoreOutcome::Restored) => "标记已还原原位".to_string(),
                            Ok(RestoreOutcome::Superseded) => {
                                "原位已有新标记，认领副本未还原".to_string()
                            }
                            Err(restore_error) => format!("还原失败：{restore_error}"),
                        };
                        failed.push(inflight_sweep_failure(
                            marker,
                            format!("{error}；{restore}"),
                        ));
                    }
                },
                reread @ (MarkerReread::NotStale | MarkerReread::Unreadable) => {
                    // 复核后已不是残标（内容已变 / 不再 stale / 读不出来）：一律保留。
                    // 还原只能是 create-if-absent（create_new 的 O_EXCL 语义），rename 会覆盖新标记。
                    let sub_reason = match reread {
                        MarkerReread::NotStale => "marker_changed",
                        _ => "recheck_unreadable",
                    };
                    match restore_without_clobber(&tmp, &marker.path) {
                        Ok(RestoreOutcome::Restored) => {
                            kept.push(inflight_sweep_entry(marker, "not_mine", sub_reason))
                        }
                        Ok(RestoreOutcome::Superseded) => kept.push(inflight_sweep_entry(
                            marker,
                            "not_mine",
                            "marker_superseded",
                        )),
                        // 认领副本在还原前消失：同「别人已经处置」，按 lost_race 记账。
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            kept.push(inflight_sweep_entry(marker, "lost_race", "claim_copy_gone"))
                        }
                        Err(error) => failed.push(inflight_sweep_failure(
                            marker,
                            format!("marker_changed_restore_failed: {error}"),
                        )),
                    }
                }
            }
        }

        let removed_total = removed.len() as u64;
        let lost_race_total = kept
            .iter()
            .filter(|entry| entry["outcome"] == json!("lost_race"))
            .count() as u64;
        let not_mine_total = kept
            .iter()
            .filter(|entry| entry["outcome"] == json!("not_mine"))
            .count() as u64;
        let result = json!({
            "version": INFLIGHT_SWEEP_VERSION,
            "phase": "result",
            "at": now_stamp(),
            "trigger": trigger,
            "task_id_filter": filter,
            "removed_total": removed_total,
            "deleted_total": removed_total,
            "lost_race_total": lost_race_total,
            "not_mine_total": not_mine_total,
            "kept_total": kept.len() as u64,
            "failed_total": failed.len() as u64,
            "unreadable_total": unreadable_total,
            "removed": removed,
            "kept": kept,
            "failed": failed
        });
        if let Err(error) = self.append_inflight_sweep_record(&result) {
            return Err(format!(
                "inflight_sweep_audit_failed: 已清理 {removed_total} 条、失败 {} 条，但结果行写入 {INFLIGHT_SWEEP_AUDIT} 失败：{error}",
                result["failed_total"].as_u64().unwrap_or(0)
            ));
        }

        Ok(json!({
            "version": INFLIGHT_SWEEP_VERSION,
            "trigger": trigger,
            "dry_run": false,
            "state": state,
            "directory": "runs/.inflight",
            "total": total,
            "stale_total": stale_total,
            "active_total": active_total,
            "unreadable_total": unreadable_total,
            "would_remove_total": stale_total,
            "removed_total": result["removed_total"],
            "deleted_total": result["deleted_total"],
            "lost_race_total": result["lost_race_total"],
            "not_mine_total": result["not_mine_total"],
            "kept_total": result["kept_total"],
            "failed_total": result["failed_total"],
            "would_remove": result["removed"],
            "removed": result["removed"],
            "kept": result["kept"],
            "failed": result["failed"]
        }))
    }

    /// 清扫审计落盘：append-only JSONL，写一行并 fsync；失败必须让调用方看到
    fn append_inflight_sweep_record(&self, record: &Value) -> Result<(), String> {
        use std::io::Write;
        let path = self.runs_dir.join(INFLIGHT_SWEEP_AUDIT);
        let line = serde_json::to_string(record)
            .map_err(|error| format!("inflight_sweep_audit_failed: 序列化审计记录失败：{error}"))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                format!(
                    "inflight_sweep_audit_failed: 打开 runs/{INFLIGHT_SWEEP_AUDIT} 失败：{error}"
                )
            })?;
        writeln!(file, "{line}").map_err(|error| {
            format!("inflight_sweep_audit_failed: 写入 runs/{INFLIGHT_SWEEP_AUDIT} 失败：{error}")
        })?;
        file.sync_all().map_err(|error| {
            format!("inflight_sweep_audit_failed: 同步 runs/{INFLIGHT_SWEEP_AUDIT} 失败：{error}")
        })?;
        Ok(())
    }

    /// `runs/_session/initialize-*.json` 的按天保留清扫。
    ///
    /// 口径与任务目录不同：
    /// - 只处理 `initialize-<pid>-<毫秒时间戳>-<seq>.json`；旧版 `initialize.json`、
    ///   名字解析不出时间、时间戳在未来、文件 mtime 与文件名时间不一致（判不准）一律保留；
    /// - 只有「文件名时间」与「mtime」都早于阈值才删；
    /// - 先写 plan 审计再删；审计写不进去 → 本轮零删除；
    /// - 无可删候选时不写审计（与 inflight sweep 的幂等口径一致）。
    fn sweep_session_records(&self, trigger: &str) -> Result<Value, String> {
        let retention_days = parse_session_retention_days()?;
        let dir = self.runs_dir.join(SESSION_DIR);
        if !dir.is_dir() {
            return Ok(json!({
                "version": SESSION_RETENTION_VERSION,
                "trigger": trigger,
                "state": "missing",
                "directory": "runs/_session",
                "retention_days": retention_days,
                "total": 0,
                "candidates_total": 0,
                "removed_total": 0,
                "failed_total": 0,
                "kept_total": 0,
                "removed": [],
                "failed": [],
                "kept": []
            }));
        }
        let now = chrono::Utc::now();
        let retention = chrono::Duration::days(retention_days);
        let mut candidates: Vec<Value> = Vec::new();
        let mut kept: Vec<Value> = Vec::new();
        let mut within_retention = 0u64;
        let mut total = 0u64;
        let entries = fs::read_dir(&dir).map_err(|error| {
            format!("session_retention_failed: 读取 runs/_session 失败：{error}")
        })?;
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("initialize-") || !name.ends_with(".json") {
                continue;
            }
            total += 1;
            let Some(created) = parse_initialize_stamp(&name) else {
                // 判不准：不认识的文件名绝不删
                kept.push(json!({ "file": name, "reason": "unparsable_name" }));
                continue;
            };
            let name_old = now.signed_duration_since(created) >= retention;
            let mtime = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .map(chrono::DateTime::<chrono::Utc>::from);
            let mtime_old = mtime
                .map(|mtime| now.signed_duration_since(mtime) >= retention)
                .unwrap_or(false);
            if !name_old || !mtime_old {
                if !name_old {
                    // 保留期内是正常形态，只计数，不逐条写审计（避免审计本身无限增长）
                    within_retention += 1;
                } else {
                    kept.push(json!({
                        "file": name,
                        "createdAt": created.to_rfc3339(),
                        "reason": "mtime_not_older_than_threshold"
                    }));
                }
                continue;
            }
            candidates.push(json!({
                "file": name,
                "createdAt": created.to_rfc3339(),
                "age_days": now.signed_duration_since(created).num_days()
            }));
        }

        if candidates.is_empty() {
            return Ok(json!({
                "version": SESSION_RETENTION_VERSION,
                "trigger": trigger,
                "state": "ready",
                "directory": "runs/_session",
                "retention_days": retention_days,
                "total": total,
                "candidates_total": 0,
                "removed_total": 0,
                "failed_total": 0,
                "kept_total": kept.len() as u64,
                "within_retention_total": within_retention,
                "removed": [],
                "failed": [],
                "kept": kept
            }));
        }

        // 先记账后删除：plan 行写不进去就一条都不删
        let plan = json!({
            "version": SESSION_RETENTION_VERSION,
            "phase": "plan",
            "at": now_stamp(),
            "trigger": trigger,
            "retention_days": retention_days,
            "within_retention_total": within_retention,
            "candidates": candidates,
            "kept": kept
        });
        self.append_session_retention_record(&plan)
            .map_err(|error| format!("{error}；本轮未删除任何 _session 记录"))?;

        let mut removed: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        for candidate in &candidates {
            let name = candidate["file"].as_str().unwrap_or_default().to_string();
            match fs::remove_file(dir.join(&name)) {
                Ok(()) => removed.push(json!({
                    "file": name,
                    "createdAt": candidate["createdAt"],
                    "age_days": candidate["age_days"],
                    "outcome": "deleted"
                })),
                Err(error) => failed.push(json!({
                    "file": name,
                    "error": error.to_string()
                })),
            }
        }
        let result = json!({
            "version": SESSION_RETENTION_VERSION,
            "phase": "result",
            "at": now_stamp(),
            "trigger": trigger,
            "retention_days": retention_days,
            "removed_total": removed.len() as u64,
            "failed_total": failed.len() as u64,
            "kept_total": kept.len() as u64,
            "within_retention_total": within_retention,
            "removed": removed,
            "failed": failed,
            "kept": kept
        });
        if let Err(error) = self.append_session_retention_record(&result) {
            return Err(format!(
                "session_retention_audit_failed: 已删除 {} 条、失败 {} 条，但结果行写入 {SESSION_RETENTION_AUDIT} 失败：{error}",
                result["removed_total"].as_u64().unwrap_or(0),
                result["failed_total"].as_u64().unwrap_or(0)
            ));
        }
        Ok(json!({
            "version": SESSION_RETENTION_VERSION,
            "trigger": trigger,
            "state": "ready",
            "directory": "runs/_session",
            "retention_days": retention_days,
            "total": total,
            "candidates_total": candidates.len() as u64,
            "removed_total": result["removed_total"],
            "failed_total": result["failed_total"],
            "kept_total": result["kept_total"],
            "within_retention_total": result["within_retention_total"],
            "removed": result["removed"],
            "failed": result["failed"],
            "kept": kept
        }))
    }

    /// `_session` 保留审计：append-only JSONL，写一行并 fsync；失败必须让调用方看到
    fn append_session_retention_record(&self, record: &Value) -> Result<(), String> {
        use std::io::Write;
        let path = self.runs_dir.join(SESSION_RETENTION_AUDIT);
        let line = serde_json::to_string(record).map_err(|error| {
            format!("session_retention_audit_failed: 序列化审计记录失败：{error}")
        })?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                format!(
                    "session_retention_audit_failed: 打开 runs/{SESSION_RETENTION_AUDIT} 失败：{error}"
                )
            })?;
        writeln!(file, "{line}").map_err(|error| {
            format!(
                "session_retention_audit_failed: 写入 runs/{SESSION_RETENTION_AUDIT} 失败：{error}"
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "session_retention_audit_failed: 同步 runs/{SESSION_RETENTION_AUDIT} 失败：{error}"
            )
        })?;
        Ok(())
    }

    /// 启动回收上一轮遗留的 cgroup 作用域（Linux）。
    ///
    /// 判定只看证据，不看「像不像孤儿」：
    /// - 作用域归属记录（sidecar）存在且与本目录自洽；
    /// - 记录点名的 `runs/<task_id>/attempt-<n>/dispatch.json` 存在；
    /// - 记录的 bridge 进程已死（或 PID 已复用）。
    /// 任一条不满足 → 只写 `phase=kept/outcome=kept/reason=...` 审计，绝不发信号。
    /// 满足后才写 plan 行、kill、复检成员为空、删目录、写 result 行。
    #[cfg(target_os = "linux")]
    fn reclaim_stale_cgroup_scopes(&self, trigger: &str) -> Result<Value, String> {
        self.reclaim_stale_cgroup_scopes_at(Path::new("/sys/fs/cgroup"), trigger)
    }

    /// 可注入 base 的实现，便于用临时目录复现「证据不足只报不删」的判定。
    #[cfg(target_os = "linux")]
    fn reclaim_stale_cgroup_scopes_at(&self, base: &Path, trigger: &str) -> Result<Value, String> {
        if !base.join("cgroup.controllers").is_file() {
            return Ok(json!({
                "version": CGROUP_RECLAIM_VERSION,
                "trigger": trigger,
                "state": "unsupported",
                "reason": "cgroup_v2_unavailable",
                "base": base.to_string_lossy(),
                "scanned_total": 0,
                "reclaimed_total": 0,
                "removed_empty_total": 0,
                "kept_total": 0,
                "failed_total": 0,
                "scopes": []
            }));
        }
        let scopes = process_scope::managed_scope_dirs(base)?;
        let owner_dir = self.runs_dir.join(SCOPE_OWNER_DIR);
        let mut entries: Vec<Value> = Vec::new();
        let mut reclaimed = 0u64;
        let mut removed_empty = 0u64;
        let mut failed = 0u64;
        let mut audit_failed = 0u64;

        for (name, dir) in scopes {
            let owner_path = owner_dir.join(format!("{name}.json"));
            let owner = read_json(&owner_path).ok();
            let mut keep_reason: Option<&str> = None;
            let mut task_id = String::new();
            let mut attempt = 0u64;
            let mut owner_pid = 0u64;
            let mut root_pid = 0u64;
            let mut members_before: Option<Vec<u32>> = None;
            if let Some(owner) = owner.as_ref() {
                let scope_ok = owner.get("version").and_then(Value::as_str)
                    == Some(SCOPE_OWNER_VERSION)
                    && owner.get("scope").and_then(Value::as_str) == Some(name.as_str())
                    && owner.get("cgroup").and_then(Value::as_str)
                        == Some(dir.to_string_lossy().as_ref());
                let task = owner
                    .get("task_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let attempt_value = owner.get("attempt").and_then(Value::as_u64);
                let pid = owner.get("bridge_pid").and_then(Value::as_u64);
                let start = owner.get("bridge_start").and_then(Value::as_u64);
                if !scope_ok || !is_valid_task_id(&task) || attempt_value.is_none() {
                    keep_reason = Some("owner_record_invalid");
                } else {
                    task_id = task;
                    attempt = attempt_value.unwrap_or(0);
                    owner_pid = pid.unwrap_or(0);
                    root_pid = owner.get("root_pid").and_then(Value::as_u64).unwrap_or(0);
                    if !self
                        .runs_dir
                        .join(&task_id)
                        .join(format!("attempt-{attempt}"))
                        .join("dispatch.json")
                        .is_file()
                    {
                        keep_reason = Some("attempt_evidence_missing");
                    } else if process_is_alive(owner_pid as u32) {
                        match (start, process_scope::proc_start(owner_pid as u32)) {
                            (Some(recorded), Some(current)) if recorded == current => {
                                keep_reason = Some("owner_alive");
                            }
                            (Some(_), Some(_)) => {}
                            _ => keep_reason = Some("owner_unverifiable"),
                        }
                    }
                }
            } else {
                keep_reason = Some("owner_record_missing");
            }

            let members = process_scope::read_scope_members(&dir);
            if members.is_ok() {
                members_before = members.as_ref().ok().cloned();
            }
            if keep_reason.is_none() {
                match members.as_ref() {
                    Err(_) => keep_reason = Some("members_unreadable"),
                    Ok(list) if list.contains(&std::process::id()) => {
                        keep_reason = Some("self_member")
                    }
                    _ => {}
                }
            }
            let populated = process_scope::read_scope_populated(&dir);
            if keep_reason.is_none() && populated.is_err() {
                keep_reason = Some("populated_unreadable");
            }

            if let Some(reason) = keep_reason {
                let record = json!({
                    "version": CGROUP_RECLAIM_VERSION,
                    "trigger": trigger,
                    "phase": "kept",
                    "scope": name,
                    "cgroup": dir.to_string_lossy(),
                    "task_id": if task_id.is_empty() { Value::Null } else { json!(task_id) },
                    "attempt": if attempt == 0 { Value::Null } else { json!(attempt) },
                    "owner_pid": if owner_pid == 0 { Value::Null } else { json!(owner_pid) },
                    "root_pid": if root_pid == 0 { Value::Null } else { json!(root_pid) },
                    "members_before": members_before.clone(),
                    "members_after": members_before.clone(),
                    "outcome": "kept",
                    "reason": reason,
                    "action": "none",
                    "at": now_stamp()
                });
                if self.append_cgroup_reclaim_record(&record).is_err() {
                    audit_failed += 1;
                }
                entries.push(record);
                continue;
            }

            let members_before = members.unwrap_or_default();
            let empty_scope = members_before.is_empty();
            if empty_scope && populated.as_ref().copied().unwrap_or(0) != 0 {
                let record = json!({
                    "version": CGROUP_RECLAIM_VERSION,
                    "trigger": trigger,
                    "phase": "kept",
                    "scope": name,
                    "cgroup": dir.to_string_lossy(),
                    "task_id": task_id,
                    "attempt": attempt,
                    "owner_pid": owner_pid,
                    "root_pid": root_pid,
                    "members_before": members_before.clone(),
                    "members_after": members_before.clone(),
                    "outcome": "kept",
                    "reason": "populated_inconsistent",
                    "action": "none",
                    "at": now_stamp()
                });
                if self.append_cgroup_reclaim_record(&record).is_err() {
                    audit_failed += 1;
                }
                entries.push(record);
                continue;
            }

            // 真要动手：plan 行写不进去就不动（审计是动作的前置条件）
            let plan = json!({
                "version": CGROUP_RECLAIM_VERSION,
                "trigger": trigger,
                "phase": "plan",
                "scope": name,
                "cgroup": dir.to_string_lossy(),
                "task_id": task_id,
                "attempt": attempt,
                "owner_pid": owner_pid,
                "root_pid": root_pid,
                "members_before": members_before.clone(),
                "members_after": Value::Null,
                "outcome": if empty_scope { "remove_empty" } else { "pending" },
                "reason": if empty_scope { "empty_scope" } else { "abandoned_scope" },
                "action": if empty_scope { "remove_dir" } else { "kill" },
                "at": now_stamp()
            });
            if let Err(error) = self.append_cgroup_reclaim_record(&plan) {
                audit_failed += 1;
                let record = json!({
                    "version": CGROUP_RECLAIM_VERSION,
                    "trigger": trigger,
                    "phase": "kept",
                    "scope": name,
                    "cgroup": dir.to_string_lossy(),
                    "task_id": task_id,
                    "attempt": attempt,
                    "owner_pid": owner_pid,
                    "root_pid": root_pid,
                    "members_before": members_before.clone(),
                    "members_after": members_before.clone(),
                    "outcome": "kept",
                    "reason": "audit_unwritable",
                    "action": "none",
                    "error": error,
                    "at": now_stamp()
                });
                let _ = self.append_cgroup_reclaim_record(&record);
                entries.push(record);
                continue;
            }

            let outcome;
            if empty_scope {
                outcome = match process_scope::remove_scope(&dir) {
                    Ok(()) => {
                        removed_empty += 1;
                        let _ = fs::remove_file(&owner_path);
                        "removed_empty"
                    }
                    Err(_) => {
                        failed += 1;
                        "remove_failed"
                    }
                };
            } else if process_scope::scope_kill_supported(&dir) {
                outcome = match process_scope::kill_scope(&dir) {
                    Ok(()) => self.wait_scope_empty(&dir, &name, trigger),
                    Err(_) => {
                        failed += 1;
                        "kill_failed"
                    }
                };
            } else {
                // 内核没有 cgroup.kill：逐成员 SIGKILL，每轮重读成员表，只杀当前仍在里面的 pid
                let deadline = Instant::now() + Duration::from_secs(CGROUP_RECLAIM_DEADLINE_SECS);
                loop {
                    let current = process_scope::read_scope_members(&dir).unwrap_or_default();
                    if current.is_empty() {
                        break;
                    }
                    for pid in current {
                        // 重新核对仍在成员表里，缩小 PID 复用窗口
                        if process_scope::read_scope_members(&dir)
                            .map(|list| list.contains(&pid))
                            .unwrap_or(false)
                        {
                            let _ = process_scope::kill_pid(pid);
                        }
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                outcome = self.wait_scope_empty(&dir, &name, trigger);
            }

            let members_after = process_scope::read_scope_members(&dir).ok();
            let final_outcome = outcome;
            if final_outcome == "reclaimed" {
                reclaimed += 1;
                let _ = fs::remove_file(&owner_path);
            }
            let result = json!({
                "version": CGROUP_RECLAIM_VERSION,
                "trigger": trigger,
                "phase": "result",
                "scope": name,
                "cgroup": dir.to_string_lossy(),
                "task_id": task_id,
                "attempt": attempt,
                "owner_pid": owner_pid,
                "root_pid": root_pid,
                "members_before": members_before,
                "members_after": members_after,
                "outcome": final_outcome,
                "reason": if final_outcome == "still_alive" { "members_still_present" } else { "scope_reclaimed" },
                "action": if empty_scope { "remove_dir" } else { "kill" },
                "at": now_stamp()
            });
            if self.append_cgroup_reclaim_record(&result).is_err() {
                audit_failed += 1;
            }
            entries.push(result);
        }

        let kept_total = entries
            .iter()
            .filter(|entry| entry.get("phase").and_then(Value::as_str) == Some("kept"))
            .count() as u64;
        Ok(json!({
            "version": CGROUP_RECLAIM_VERSION,
            "trigger": trigger,
            "state": "ready",
            "base": base.to_string_lossy(),
            "scanned_total": entries.len() as u64,
            "reclaimed_total": reclaimed,
            "removed_empty_total": removed_empty,
            "kept_total": kept_total,
            "failed_total": failed,
            "audit_failed_total": audit_failed,
            "scopes": entries
        }))
    }

    /// 等作用域成员清零并删目录；读不出成员就不宣称清空。
    #[cfg(target_os = "linux")]
    fn wait_scope_empty(&self, dir: &Path, _name: &str, _trigger: &str) -> &'static str {
        let deadline = Instant::now() + Duration::from_secs(CGROUP_RECLAIM_DEADLINE_SECS);
        loop {
            let empty = process_scope::read_scope_members(dir)
                .map(|members| members.is_empty())
                .unwrap_or(false)
                && process_scope::read_scope_populated(dir)
                    .map(|value| value == 0)
                    .unwrap_or(false);
            if empty {
                return match process_scope::remove_scope(dir) {
                    Ok(()) => "reclaimed",
                    Err(_) => "remove_failed",
                };
            }
            if Instant::now() >= deadline {
                return "still_alive";
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn reclaim_stale_cgroup_scopes(&self, trigger: &str) -> Result<Value, String> {
        Ok(json!({
            "version": CGROUP_RECLAIM_VERSION,
            "trigger": trigger,
            "state": "unsupported",
            "reason": "platform_has_no_cgroup_v2",
            "scanned_total": 0,
            "reclaimed_total": 0,
            "removed_empty_total": 0,
            "kept_total": 0,
            "failed_total": 0,
            "scopes": []
        }))
    }

    #[cfg(target_os = "linux")]
    fn append_cgroup_reclaim_record(&self, record: &Value) -> Result<(), String> {
        use std::io::Write;
        let path = self.runs_dir.join(CGROUP_RECLAIM_AUDIT);
        let line = serde_json::to_string(record)
            .map_err(|error| format!("cgroup_reclaim_audit_failed: 序列化失败：{error}"))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                format!("cgroup_reclaim_audit_failed: 打开 {CGROUP_RECLAIM_AUDIT} 失败：{error}")
            })?;
        writeln!(file, "{line}")
            .map_err(|error| format!("cgroup_reclaim_audit_failed: 写入失败：{error}"))?;
        file.sync_all()
            .map_err(|error| format!("cgroup_reclaim_audit_failed: 同步失败：{error}"))
    }

    /// 启动清扫取消请求：只清「目标已死且过了宽限」与「终态超保留期」两类；
    /// 判不准（不可读、目标不可验证）一律保留。先写 plan 审计，写不进去就零删除。
    fn sweep_cancel_requests(&self, trigger: &str) -> Result<Value, String> {
        let dir = self.runs_dir.join(CANCEL_DIR);
        if !dir.is_dir() {
            return Ok(json!({
                "version": CANCEL_REQUEST_SWEEP_VERSION,
                "trigger": trigger,
                "state": "ready",
                "total": 0,
                "removed_total": 0,
                "kept_total": 0,
                "removed": [],
                "kept": []
            }));
        }
        let now = chrono::Utc::now();
        let mut removed: Vec<Value> = Vec::new();
        let mut kept: Vec<Value> = Vec::new();
        let entries = fs::read_dir(&dir)
            .map_err(|error| format!("cancel_request_sweep_failed: 读取目录失败：{error}"))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(task_id) = name.strip_suffix(".json").map(str::to_string) else {
                continue;
            };
            if !entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                continue;
            }
            let Ok(record) = read_json(&entry.path()) else {
                kept.push(json!({ "file": name, "reason": "unreadable" }));
                continue;
            };
            let status = record
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let stale_target = record
                .get("target")
                .and_then(|target| target.get("pid"))
                .and_then(Value::as_u64)
                .map(|pid| !process_is_alive(pid as u32))
                .unwrap_or(false);
            let requested_at = record
                .get("requestedAt")
                .and_then(Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok());
            let age_secs = requested_at
                .map(|time| (now - time.with_timezone(&chrono::Utc)).num_seconds())
                .unwrap_or(0);
            let terminal = status == "done" || status == "ignored";
            let remove = if terminal {
                entry
                    .metadata()
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .map(|modified| {
                        chrono::DateTime::<chrono::Utc>::from(modified)
                            < now - chrono::Duration::days(CANCEL_REQUEST_RETENTION_DAYS)
                    })
                    .unwrap_or(false)
            } else {
                stale_target && age_secs >= CANCEL_REQUEST_STALE_GRACE_SECS
            };
            if !remove {
                kept.push(json!({
                    "file": name,
                    "task_id": task_id,
                    "status": status,
                    "reason": if terminal { "within_retention" } else { "target_alive_or_within_grace" }
                }));
                continue;
            }
            let plan = json!({
                "version": CANCEL_REQUEST_SWEEP_VERSION,
                "trigger": trigger,
                "phase": "plan",
                "file": name,
                "task_id": task_id,
                "status": status,
                "target": record.get("target").cloned().unwrap_or(Value::Null),
                "requested_by": record.get("requested_by").cloned().unwrap_or(Value::Null),
                "reason": if terminal { "terminal_retention_expired" } else { "target_dead_past_grace" },
                "at": now_stamp()
            });
            if let Err(error) = self.append_cancel_request_record(&plan) {
                kept.push(json!({
                    "file": name,
                    "task_id": task_id,
                    "status": status,
                    "reason": "audit_unwritable",
                    "error": error
                }));
                continue;
            }
            match fs::remove_file(entry.path()) {
                Ok(()) => {
                    let result = json!({
                        "version": CANCEL_REQUEST_SWEEP_VERSION,
                        "trigger": trigger,
                        "phase": "result",
                        "file": name,
                        "task_id": task_id,
                        "status": status,
                        "target": record.get("target").cloned().unwrap_or(Value::Null),
                        "outcome": "removed",
                        "at": now_stamp()
                    });
                    let _ = self.append_cancel_request_record(&result);
                    removed.push(result);
                }
                Err(error) => {
                    kept.push(json!({
                        "file": name,
                        "task_id": task_id,
                        "status": status,
                        "reason": "remove_failed",
                        "error": error.to_string()
                    }));
                }
            }
        }
        Ok(json!({
            "version": CANCEL_REQUEST_SWEEP_VERSION,
            "trigger": trigger,
            "state": "ready",
            "total": removed.len() as u64 + kept.len() as u64,
            "removed_total": removed.len() as u64,
            "kept_total": kept.len() as u64,
            "removed": removed,
            "kept": kept
        }))
    }

    fn append_cancel_request_record(&self, record: &Value) -> Result<(), String> {
        use std::io::Write;
        let path = self.runs_dir.join(CANCEL_REQUEST_AUDIT);
        let line = serde_json::to_string(record)
            .map_err(|error| format!("cancel_request_audit_failed: 序列化失败：{error}"))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                format!("cancel_request_audit_failed: 打开 {CANCEL_REQUEST_AUDIT} 失败：{error}")
            })?;
        writeln!(file, "{line}")
            .map_err(|error| format!("cancel_request_audit_failed: 写入失败：{error}"))?;
        file.sync_all()
            .map_err(|error| format!("cancel_request_audit_failed: 同步失败：{error}"))
    }

    /// 只读扫描 `.inflight/*.json`；非 .json 的临时文件不参与枚举，符号链接不跟随
    fn scan_inflight_markers(
        &self,
        filter: Option<&str>,
    ) -> (Vec<InflightMarker>, Vec<Value>, bool) {
        let dir = self.runs_dir.join(INFLIGHT_DIR);
        let exists = dir.is_dir();
        let mut markers: Vec<InflightMarker> = Vec::new();
        let mut unreadable: Vec<Value> = Vec::new();
        if exists {
            if let Ok(entries) = fs::read_dir(&dir) {
                let now = chrono::Utc::now();
                for entry in entries.flatten() {
                    let Ok(file_type) = entry.file_type() else {
                        continue;
                    };
                    if !file_type.is_file() {
                        continue;
                    }
                    let file = entry.file_name().to_string_lossy().to_string();
                    let Some(task_id) = file.strip_suffix(".json").map(str::to_string) else {
                        continue;
                    };
                    if task_id.is_empty() {
                        continue;
                    }
                    if filter.is_some_and(|wanted| wanted != task_id.as_str()) {
                        continue;
                    }
                    let raw = match fs::read_to_string(entry.path()) {
                        Ok(raw) => raw,
                        Err(error) => {
                            unreadable.push(json!({
                                "task_id": task_id,
                                "file": file,
                                "error": error.to_string(),
                                "stale": false,
                                "stale_reason": "unreadable",
                                "removable": false
                            }));
                            continue;
                        }
                    };
                    let record: Value = match serde_json::from_str(&raw) {
                        Ok(record) => record,
                        Err(error) => {
                            unreadable.push(json!({
                                "task_id": task_id,
                                "file": file,
                                "error": error.to_string(),
                                "stale": false,
                                "stale_reason": "unreadable",
                                "removable": false
                            }));
                            continue;
                        }
                    };
                    let assessment = assess_inflight_marker(&record, now);
                    markers.push(InflightMarker {
                        task_id,
                        file,
                        path: entry.path(),
                        assessment,
                    });
                }
            }
        }
        markers.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        unreadable.sort_by(|left, right| {
            left["file"]
                .as_str()
                .unwrap_or("")
                .cmp(right["file"].as_str().unwrap_or(""))
        });
        (markers, unreadable, exists)
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
        let mode = if standalone {
            "standalone"
        } else {
            "dispatched"
        };
        // 活跃标记覆盖整个调用生命周期：正常返回、提前报错、被阻断都会在 Drop 时删除
        let _inflight = self.acquire_inflight_marker(task_id, mode, args)?;
        let started_at = _inflight.as_ref().map(|guard| guard.started_at.clone());
        // 取消登记同样覆盖整个调用；重投方复用胜者句柄，不抢所有权
        let (cancel, cancel_owner) = self.register_cancel(task_id);
        let _cancel_guard = CancelRegistration {
            server: self,
            task_id: task_id.to_string(),
            token: Arc::clone(&cancel),
            owner: cancel_owner,
        };
        // 跨进程取消：启动前先读一次请求；执行循环里每 200ms 再读（同一线程，不另起线程）
        let mut heartbeat = started_at
            .as_deref()
            .map(|started| CancelHeartbeat::new(&self.runs_dir, task_id, started));
        if let Some(heartbeat) = heartbeat.as_mut() {
            heartbeat.poll(&cancel);
        }
        let result = match standalone {
            false => self.execute_dispatched(args, task_id, &cancel, heartbeat.as_mut()),
            true => self.execute_standalone(args, task_id, &cancel, heartbeat.as_mut()),
        };
        // 收尾：有请求文件就推进到 done；handback 读不到时也写 done，调用方按保守口径解释
        let handback = latest_attempt_dir(&self.runs_dir.join(task_id))
            .and_then(|(_, dir)| read_json(&dir.join("handback.json")).ok());
        let handback_status = handback
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let cleanup_status = handback
            .as_ref()
            .and_then(|value| value.get("cleanup_status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(heartbeat) = heartbeat.as_mut() {
            heartbeat.finish(handback_status.as_deref(), cleanup_status.as_deref());
        }
        result
    }

    /// 显式取消：本进程或另一个进程都可发起。
    ///
    /// 语义边界（写死，避免谎报）：
    /// - 请求先落盘 `runs/.cancel/<task_id>.json`，带发起者、时间、target(pid+startedAt)；
    ///   执行侧在心跳检查点读取，三态可区分：`pending`（已落盘未读）→ `acknowledged`
    ///   （已读、正在清理）→ `done`（收尾完成）；
    /// - 找不到本进程句柄、且磁盘上没有仍有效的活跃标记 → `not_running`，不写请求、不发信号；
    /// - 回执状态必须是 `canceled`；若执行在取消生效前已正常结束，如实返回 `completed_before_cancel`；
    /// - 等不到 `done` → `cancel_requested`，用 `request_state` 区分「未读」和「已读在清理」。
    fn task_cancel(&self, args: &Value) -> Result<Value, String> {
        let task_id = required_str(args, "task_id")?;
        if !is_valid_task_id(task_id) {
            return Err(format!("task_id 非法：{task_id}"));
        }
        let reason = args
            .get("reason")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if let Some(value) = &reason {
            // schema 声明 maxLength=1024，这里同源拒绝：超长理由不落盘、不发信号
            if value.chars().count() > 1024 {
                return Err("reason 超过 1024 字符：invalid_reason".to_string());
            }
        }
        let requested_by = match args.get("requested_by") {
            None | Some(Value::Null) => format!("pid:{}", std::process::id()),
            Some(value) => {
                let raw = value
                    .as_str()
                    .ok_or_else(|| "invalid_requested_by: requested_by 必须是字符串".to_string())?
                    .trim();
                if raw.is_empty() {
                    return Err("invalid_requested_by: requested_by 不能为空".to_string());
                }
                if raw.chars().count() > 128 {
                    return Err("invalid_requested_by: requested_by 超过 128 字符".to_string());
                }
                raw.to_string()
            }
        };
        let token = {
            let map = self
                .cancel_tokens
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            map.get(task_id).cloned()
        };
        let marker = self.inflight_snapshot(task_id);
        let marker_live = marker
            .as_ref()
            .map(|record| record.get("stale").and_then(Value::as_bool) != Some(true))
            .unwrap_or(false);
        if token.is_none() && !marker_live {
            // 看不到 = 不作任何清理宣称；也不落请求文件，避免污染后续同 task_id 的执行
            return Ok(json!({
                "task_id": task_id,
                "status": "not_running",
                "canceled": false,
                "cleanup_claimed": false,
                "cross_process": true,
                "request_state": Value::Null,
                "request_file": Value::Null,
                "requested_by": requested_by,
                "inflight_marker": marker,
                "message": "没有该 task_id 的活跃执行：未写取消请求、未向任何进程发送信号、未做任何清理；inflight_marker 只是磁盘观测，不代表进程状态"
            }));
        }
        let target = match marker.as_ref() {
            Some(record) => {
                let pid = record.get("pid").and_then(Value::as_u64);
                let started = record.get("startedAt").and_then(Value::as_str);
                match (pid, started) {
                    (Some(pid), Some(started)) => json!({ "pid": pid, "startedAt": started }),
                    // 标记不完整：无法证明请求指向哪一轮执行，拒绝落盘（判不准就不动）
                    _ => {
                        return Ok(json!({
                            "task_id": task_id,
                            "status": "not_running",
                            "canceled": false,
                            "cleanup_claimed": false,
                            "cross_process": token.is_none(),
                            "request_state": Value::Null,
                            "request_file": Value::Null,
                            "requested_by": requested_by,
                            "inflight_marker": marker,
                            "message": "活跃标记缺少 pid/startedAt，无法证明取消目标；未写请求、未发信号"
                        }))
                    }
                }
            }
            None => json!({ "pid": std::process::id(), "startedAt": Value::Null }),
        };
        let (mut request, path) =
            self.ensure_cancel_request(task_id, &requested_by, reason.as_deref(), target)?;
        if let Some(token) = token.as_ref() {
            token.request(reason.clone());
        }
        let mut finished = false;
        let deadline = Instant::now() + Duration::from_secs(CANCEL_WAIT_SECS);
        loop {
            let state = read_json(&path).ok().and_then(|record| {
                record
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
            if let Some(state) = state.as_deref() {
                if state == "done" || state == "ignored" {
                    finished = true;
                    break;
                }
            }
            if Instant::now() >= deadline {
                break;
            }
            if let Some(token) = token.as_ref() {
                // 本地句柄用条件变量等，跨进程请求只能轮询磁盘
                if token.wait_finished(Duration::from_millis(100)) {
                    finished = true;
                    break;
                }
            } else {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        if let Ok(record) = read_json(&path) {
            request = record;
        }
        let request_state = request
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string);
        let handback = latest_attempt_dir(&self.runs_dir.join(task_id))
            .and_then(|(_, dir)| read_json(&dir.join("handback.json")).ok());
        let handback_status = handback
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        // 终止结果只从回执的 cleanup_status 取，不从措辞猜；旧回执缺字段时为 null（宁可不宣称）
        let cleanup_status = handback
            .as_ref()
            .and_then(|value| value.get("cleanup_status"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let blocked_reason = handback
            .as_ref()
            .and_then(|value| value.get("blocked_reason"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let status = match request_state.as_deref() {
            Some("ignored") => "not_running",
            Some("done") => match handback_status.as_deref() {
                Some("canceled") => "canceled",
                Some("ok") | Some("failed") | Some("blocked") => "completed_before_cancel",
                // 执行侧已收尾但回执不可判定：不宣称清理
                _ => "cancel_requested",
            },
            Some("pending") | Some("acknowledged") => "cancel_requested",
            _ => {
                if finished {
                    match handback_status.as_deref() {
                        Some("canceled") => "canceled",
                        Some("ok") | Some("failed") | Some("blocked") => "completed_before_cancel",
                        _ => "cancel_requested",
                    }
                } else {
                    "cancel_requested"
                }
            }
        };
        Ok(json!({
            "task_id": task_id,
            "status": status,
            "canceled": status == "canceled",
            // 只有回执明确写了 canceled，且受控集合观测到无存活，才允许宣称已清理
            "cleanup_claimed": cleanup_claim_allowed(Some(status), cleanup_status.as_deref()),
            "cleanup_status": cleanup_status,
            "finished_observed": finished,
            "cross_process": token.is_none(),
            // 三态：pending=已落盘未读，acknowledged=已读在清理，done=收尾完成
            "request_state": request_state,
            "request_file": format!("runs/{CANCEL_DIR}/{task_id}.json"),
            "requested_by": request.get("requested_by").cloned().unwrap_or(json!(requested_by)),
            "requested_at": request.get("requestedAt").cloned().unwrap_or(Value::Null),
            "acknowledged_at": request.get("acknowledgedAt").cloned().unwrap_or(Value::Null),
            "finished_at": request.get("finishedAt").cloned().unwrap_or(Value::Null),
            "handback_status": handback_status,
            "handback": handback,
            "cleanup_detail": blocked_reason,
            "requested_reason": reason,
            "message": match status {
                "canceled" => "取消已生效：执行线程完成终止并落盘 canceled 回执",
                "completed_before_cancel" => "取消请求到达前执行已结束；未终止任何仍在运行的进程",
                "not_running" => "取消请求的目标已不是当前执行（上一轮残留或目标已变）；未向任何进程发送信号",
                "cancel_requested" => "已落盘取消请求，但尚未观测到 done；request_state=pending 表示执行侧未读到，acknowledged 表示已在清理",
                _ => ""
            }
        }))
    }

    /// 落盘取消请求：`create_new` 保证同一轮只有一个请求；同 target 复用，上一轮残留先移除再重建。
    fn ensure_cancel_request(
        &self,
        task_id: &str,
        requested_by: &str,
        reason: Option<&str>,
        target: Value,
    ) -> Result<(Value, PathBuf), String> {
        use std::io::Write;
        let dir = self.runs_dir.join(CANCEL_DIR);
        fs::create_dir_all(&dir)
            .map_err(|error| format!("cancel_request_failed: 创建取消请求目录失败：{error}"))?;
        let path = dir.join(format!("{task_id}.json"));
        for _ in 0..3 {
            let record = json!({
                "version": CANCEL_VERSION,
                "task_id": task_id,
                "requestedAt": now_stamp(),
                "requested_by": requested_by,
                "reason": reason,
                "target": target,
                "status": "pending",
                "acknowledgedAt": Value::Null,
                "handledBy": Value::Null,
                "handback_status": Value::Null,
                "finishedAt": Value::Null
            });
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    let text = serde_json::to_string_pretty(&record)
                        .map_err(|error| format!("cancel_request_failed: 序列化失败：{error}"))?;
                    if let Err(error) = file
                        .write_all(text.as_bytes())
                        .and_then(|_| file.sync_all())
                    {
                        let _ = fs::remove_file(&path);
                        return Err(format!("cancel_request_failed: 写入请求失败：{error}"));
                    }
                    return Ok((record, path));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let existing = read_json(&path).map_err(|error| {
                        format!(
                            "cancel_request_unreadable: 已存在但无法解析的取消请求 {}：{error}",
                            path.display()
                        )
                    })?;
                    if existing.get("version").and_then(Value::as_str) != Some(CANCEL_VERSION)
                        || existing.get("task_id").and_then(Value::as_str) != Some(task_id)
                    {
                        return Err(format!(
                            "cancel_request_unreadable: {} 不是本工具写出的取消请求（version/task_id 不符），未覆盖",
                            path.display()
                        ));
                    }
                    if existing.get("target") == Some(&target) {
                        return Ok((existing, path));
                    }
                    // 上一轮残留请求：目标已不是本次执行，移除后重建
                    fs::remove_file(&path).map_err(|error| {
                        format!("cancel_request_failed: 清理上一轮请求失败：{error}")
                    })?;
                }
                Err(error) => return Err(format!("cancel_request_failed: 创建请求失败：{error}")),
            }
        }
        Err("cancel_request_failed: 并发写入冲突，未能建立取消请求".to_string())
    }

    /// dispatched（缺省）：外部（PM）派单。AW 只执行、交回执、不验收、不判停（既有契约逐字不变）
    fn execute_dispatched(
        &self,
        args: &Value,
        task_id: &str,
        cancel: &CancelToken,
        heartbeat: Option<&mut CancelHeartbeat>,
    ) -> Result<Value, String> {
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
        let stdout_file = fs::File::create(&stdout_path)
            .map_err(|e| create_evidence_error(&run_dir, "stdout", e))?;
        let stderr_file = fs::File::create(&stderr_path)
            .map_err(|e| create_evidence_error(&run_dir, "stderr", e))?;

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
        // 启动前就收到取消：不启动任何进程，也就没有任何"已清理"的宣称
        let mut canceled = cancel.is_canceled();
        let mut cleanup_outcome = None;
        let mut spawn_error: Option<String> = None;
        let mut supervisor = process_scope::Supervisor::new();
        supervisor.configure(&mut command);
        let mut heartbeat = heartbeat;
        let exit_status = if canceled {
            None
        } else {
            match command.spawn() {
                Ok(mut child) => {
                    supervisor.attach(&mut child);
                    if let Err(error) =
                        supervisor.publish_scope_owner(&child, &self.runs_dir, task_id, attempt)
                    {
                        // 归属记录写不进去只会让下一次启动「判不准不动」，不影响本次执行
                        eprintln!(
                            "[cgroup-scope] 归属记录写入失败 task_id={task_id} attempt={attempt}: {error}"
                        );
                    }
                    if let Some(error) = supervisor.attach_error().map(str::to_string) {
                        // 恢复执行失败：子进程已被终止，不能把"从未执行"记成命令结果
                        let _ = child.wait();
                        spawn_error = Some(error);
                        None
                    } else {
                        loop {
                            // 跨进程取消的心跳检查点：与本地取消共用同一四态路径
                            if let Some(heartbeat) = heartbeat.as_deref_mut() {
                                heartbeat.poll(cancel);
                            }
                            match child.try_wait() {
                                Ok(Some(status)) => break Some(status),
                                Ok(None) => {
                                    if cancel.is_canceled() {
                                        canceled = true;
                                        cleanup_outcome = Some(supervisor.terminate(&mut child));
                                        break None;
                                    }
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
            }
        };
        let duration_ms = started.elapsed().as_millis() as u64;

        let stdout_digest = digest_file(&stdout_path)?;
        let stderr_digest = digest_file(&stderr_path)?;
        let blocked_reason = spawn_error
            .or_else(|| {
                canceled.then(|| cancel_reason(cancel.reason().as_deref(), cleanup_outcome))
            })
            .or_else(|| timed_out.then(|| timeout_reason(timeout_secs, cleanup_outcome)));
        let exit_code = exit_status.and_then(|status| status.code());
        let cleanup_status = cleanup_status_label(canceled, timed_out, cleanup_outcome);
        let status = if canceled {
            "canceled"
        } else if blocked_reason.is_some() {
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
            // 机器可读的终止结果：cleaned / still_alive / unavailable / not_started / null
            "cleanup_status": cleanup_status,
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
    fn execute_standalone(
        &self,
        args: &Value,
        task_id: &str,
        cancel: &CancelToken,
        heartbeat: Option<&mut CancelHeartbeat>,
    ) -> Result<Value, String> {
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
        let mut heartbeat = heartbeat;

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
            let run = run_standalone_command(
                &spec,
                &run_dir,
                &self.runs_dir,
                task_id,
                attempt,
                cancel,
                heartbeat.as_deref_mut(),
            )?;
            let after = git_paths(&spec.cwd);
            let (changed_files, changed_files_truncated) = changed_between(&before, &after);

            let (criteria, missing, all_passed) = evaluate_acceptance(&spec, &run);
            let (mut conclusion, mut action, reason, last_failure) =
                decide_standalone(&run, &missing, all_passed, &spec, attempt);
            // 显式取消优先于验收判定：被取消的尝试必须停在 canceled，不能因为退码 0 被记成 passed
            if run.canceled {
                conclusion = "canceled".to_string();
                action = "terminate".to_string();
            }

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
                "canceled" => "canceled",
                _ => "blocked",
            };
            let blocked_reason = if status == "blocked" {
                Some(standalone_blocked_detail(&run, &missing))
            } else if status == "canceled" {
                run.blocked_reason.clone()
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
                "cleanup_status": if status == "canceled" { run.cleanup_status } else { None },
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
    let text =
        serde_json::to_string_pretty(&record).map_err(|e| format!("序列化派单签名失败：{e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入派单签名失败：{e}"))
}

/// 证据时间戳统一口径：RFC3339 / UTC / 秒级（`Z` 结尾），`dispatchedAt` 与 `finishedAt` 共用
fn now_stamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// `_session` 保留天数：环境变量覆盖，未设置/空白取默认；非法值明确报错而不是静默回退。
fn parse_session_retention_days() -> Result<i64, String> {
    let raw = match std::env::var(SESSION_RETENTION_ENV) {
        Ok(raw) if !raw.trim().is_empty() => raw,
        _ => return Ok(SESSION_RETENTION_DEFAULT_DAYS),
    };
    let days: i64 = raw.trim().parse().map_err(|_| {
        format!(
            "invalid_session_retention: {SESSION_RETENTION_ENV} 必须是 1..=3650 的整数，收到：{raw}"
        )
    })?;
    if !(1..=3650).contains(&days) {
        return Err(format!(
            "invalid_session_retention: {SESSION_RETENTION_ENV} 必须是 1..=3650 的整数，收到：{days}"
        ));
    }
    Ok(days)
}

/// 从 `initialize-<pid>-<毫秒时间戳>-<seq>.json` 解析创建时刻。
/// 解析不出来返回 None：调用方必须保留，不得按 mtime 猜。
fn parse_initialize_stamp(name: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let body = name.strip_prefix("initialize-")?.strip_suffix(".json")?;
    let mut parts = body.split('-');
    let _pid = parts.next()?;
    let millis = parts.next()?.parse::<i64>().ok()?;
    let _seq = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    chrono::DateTime::from_timestamp_millis(millis)
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
            if latest
                .as_ref()
                .map(|(seen, _)| number > *seen)
                .unwrap_or(true)
            {
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

/// 单次删除的终态：只有 `Deleted` 表示"已从 `runs/` 移除"；`LostRace` 表示另一方先赢了、
/// 本进程零删除（源已不存在或目的目录不存在）；其余两种都还在盘上。
enum DeleteOutcome {
    Deleted { trash_path: String },
    RetainedAtSource { error: String },
    RetainedInTrash { trash_path: String, error: String },
    LostRace { error: String },
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

/// 竞态失手的统一构造入口：本进程零删除，`state` 固定为 `lost_race`，
/// 不进 `failed[]`、不计入 `failed_total`。
fn lost_race_entry(
    task_id: &str,
    attempt: u64,
    source_path: &str,
    error: &str,
    rule: &Value,
    fingerprint: &Value,
) -> Value {
    json!({
        "task_id": task_id,
        "attempt": attempt,
        "path": source_path,
        "source_path": source_path,
        "state": "lost_race",
        "trash_path": null,
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

/// 超时清理承诺"受控集合"（POSIX 进程组 / Windows Job Object）内无存活进程。
/// Linux 在可写 cgroup v2 且内核提供 `cgroup.kill` 时，受控集合扩为整个 cgroup 作用域，
/// 可终止换组/换父的逃逸后代；作用域建不起来时退回窄判据并如实降级。
mod process_scope {
    /// 受控集合内的清理结果。`Unavailable` 表示机制未建立/无法观测，绝不等于"已清理"。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum CleanupOutcome {
        Cleaned,
        StillAlive,
        Unavailable,
    }

    /// 后代观测的作用域。它决定"快照为空"能证明什么，不是实现细节。
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    // 变体按平台构造，单平台编译时另一个变体必然"未使用"
    #[allow(dead_code)]
    pub(crate) enum SnapshotScope {
        /// Windows 作业：成员关系与父进程是否存活无关
        JobObject,
        /// Linux cgroup v2：成员关系在换父被收养后仍然保留
        CgroupV2,
        /// POSIX 进程组：只覆盖同组进程，换组或换父的后代不在其中
        ProcessGroup,
    }

    /// 一次"后代快照"：只读观测，不参与终止动作。
    ///
    /// - `descendants`：本作用域能枚举到的存活后代（父链 ∪ 作用域成员，去根）；
    /// - `controlled_pids`：作用域成员（Windows 作业 / Linux cgroup / POSIX 进程组）；
    ///   `None` 表示该时刻**无法观测**，绝不能当成"空集合"；
    /// - `escaped_scope_unobservable`：期望的"换父后仍可枚举"作用域没建起来的原因。
    ///   非空时 `descendants` 只覆盖父链/进程组，**不代表**没有逃逸后代；
    /// - `unobservable`：非空时整份快照不可用，调用方不得把 `is_empty()` 当成"没有后代"。
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct DescendantSnapshot {
        pub(crate) root: u32,
        pub(crate) descendants: Vec<DescendantProcess>,
        pub(crate) controlled_pids: Option<Vec<u32>>,
        pub(crate) scope: SnapshotScope,
        pub(crate) escaped_scope_unobservable: Option<String>,
        pub(crate) unobservable: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct DescendantProcess {
        pub(crate) pid: u32,
        pub(crate) in_controlled_set: bool,
    }

    impl DescendantSnapshot {
        /// 只有"可观测且确实为空"才算空；不可观测一律不能读成空。
        pub(crate) fn is_empty(&self) -> bool {
            self.unobservable.is_none() && self.descendants.is_empty()
        }

        /// 快照里观测到、但不在受控集合内的后代（即作业/进程组逃逸）。
        /// 受控集合本身不可观测时返回空表，调用方必须先检查 `unobservable`。
        #[cfg(test)]
        pub(crate) fn outside_controlled_set(&self) -> Vec<u32> {
            if self.unobservable.is_some() {
                return Vec::new();
            }
            self.descendants
                .iter()
                .filter(|descendant| !descendant.in_controlled_set)
                .map(|descendant| descendant.pid)
                .collect()
        }

        /// 只表示"枚举到的后代都在受控集合内"，不表示"不存在未枚举到的后代"。
        #[cfg(test)]
        pub(crate) fn observed_descendants_are_controlled(&self) -> bool {
            self.unobservable.is_none()
                && self.controlled_pids.is_some()
                && self
                    .descendants
                    .iter()
                    .all(|descendant| descendant.in_controlled_set)
        }

        /// 该快照的枚举是否与父链无关：只有作业/cgroup 作用域才覆盖"换父被收养"的后代。
        /// 降级到进程组时恒为 false——空快照不能读成"整树已清"。
        #[cfg(test)]
        #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
        pub(crate) fn covers_escaped_descendants(&self) -> bool {
            self.unobservable.is_none()
                && self.escaped_scope_unobservable.is_none()
                && matches!(
                    self.scope,
                    SnapshotScope::JobObject | SnapshotScope::CgroupV2
                )
        }
    }

    #[cfg(unix)]
    mod imp {
        #[cfg(target_os = "linux")]
        use super::SnapshotScope;
        use super::{CleanupOutcome, DescendantProcess, DescendantSnapshot};
        use std::os::unix::process::CommandExt;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};

        const TERM_GRACE: Duration = Duration::from_millis(500);
        const CLEANUP_DEADLINE: Duration = Duration::from_secs(3);

        // 观测/终止过程的线程内时序（仅测试编译）。失败时由用例一次性打印，
        // 回答「谁在什么时候做了什么、当时并发多少、为什么判成 Unavailable」。
        #[cfg(test)]
        thread_local! {
            static OBS_TRACE: std::cell::RefCell<Vec<String>> =
                std::cell::RefCell::new(Vec::new());
        }

        #[cfg(test)]
        fn trace_note(note: impl Into<String>) {
            let note = note.into();
            OBS_TRACE.with(|trace| trace.borrow_mut().push(note));
        }

        #[cfg(test)]
        fn trace_take() -> Vec<String> {
            OBS_TRACE.with(|trace| std::mem::take(&mut *trace.borrow_mut()))
        }

        /// 同一时刻正在做进程观测/终止的实例数（仅测试编译）：回答「失败时并发是多少」。
        #[cfg(test)]
        static ACTIVE_OBSERVATIONS: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        #[cfg(test)]
        static PEAK_OBSERVATIONS: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        #[cfg(test)]
        struct ObservationGuard;

        #[cfg(test)]
        impl ObservationGuard {
            fn enter() -> Self {
                let current =
                    ACTIVE_OBSERVATIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                PEAK_OBSERVATIONS.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                Self
            }
        }

        #[cfg(test)]
        impl Drop for ObservationGuard {
            fn drop(&mut self) {
                ACTIVE_OBSERVATIONS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }

        #[cfg(test)]
        fn observation_stats() -> (usize, usize) {
            (
                ACTIVE_OBSERVATIONS.load(std::sync::atomic::Ordering::SeqCst),
                PEAK_OBSERVATIONS.load(std::sync::atomic::Ordering::SeqCst),
            )
        }

        pub(crate) struct Supervisor {
            pgid: Option<i32>,
            /// 根进程启动时刻（`/proc/<pid>/stat` 的 `starttime`）；父链边校验用，
            /// 拒绝 PID 复用后挂到新进程上的旧子进程。读不到时边校验退回旧行为。
            #[cfg(target_os = "linux")]
            root_start: Option<u64>,
            /// cgroup v2 作用域是可选观测点：建得起来就用，建不起来记原因，绝不当成"没有后代"
            #[cfg(target_os = "linux")]
            cgroup: CgroupState,
        }

        #[cfg(target_os = "linux")]
        pub(crate) struct CgroupState {
            scope: Option<CgroupScope>,
            status: CgroupStatus,
        }

        #[cfg(target_os = "linux")]
        #[derive(Debug)]
        pub(crate) enum CgroupStatus {
            /// 尚未尝试建立（configure 前）
            Pending,
            /// 子进程已在独立 cgroup 内，成员集合可读
            Active,
            /// 建立失败，附原因；观测退回进程组，逃逸后代不可枚举
            Degraded(String),
        }

        #[cfg(target_os = "linux")]
        impl Default for CgroupState {
            fn default() -> Self {
                Self {
                    scope: None,
                    status: CgroupStatus::Pending,
                }
            }
        }

        impl Supervisor {
            pub(crate) fn new() -> Self {
                Self {
                    pgid: None,
                    #[cfg(target_os = "linux")]
                    root_start: None,
                    #[cfg(target_os = "linux")]
                    cgroup: CgroupState::default(),
                }
            }

            /// 在 exec 之前建组：`process_group(0)` 让子进程成为新进程组的组长，
            /// 因此不存在 Windows 那种"spawn 之后再加入"的竞态窗口。
            ///
            /// 同时尝试建独立 cgroup：`pre_exec` 在 fork 之后、exec 之前把自身 PID 写入
            /// `cgroup.procs`，此刻还没有执行任何用户代码，所以也不存在"先派生后代再入组"的窗口。
            pub(crate) fn configure(&mut self, command: &mut Command) {
                command.process_group(0);
                #[cfg(target_os = "linux")]
                install_pdeathsig_pre_exec(command);
                #[cfg(target_os = "linux")]
                {
                    match CgroupScope::prepare() {
                        Ok(scope) => match scope.writer() {
                            Ok(fd) => {
                                install_cgroup_pre_exec(command, fd);
                                self.cgroup.scope = Some(scope);
                                self.cgroup.status = CgroupStatus::Pending;
                            }
                            Err(reason) => {
                                self.cgroup.scope = Some(scope);
                                self.cgroup.status = CgroupStatus::Degraded(reason);
                            }
                        },
                        Err(reason) => {
                            self.cgroup.status = CgroupStatus::Degraded(reason);
                        }
                    }
                }
            }

            pub(crate) fn attach(&mut self, child: &mut Child) {
                self.pgid = i32::try_from(child.id()).ok().filter(|pgid| *pgid > 0);
                #[cfg(target_os = "linux")]
                {
                    self.root_start = proc_start_time(child.id());
                }
                #[cfg(target_os = "linux")]
                {
                    let Some(scope) = self.cgroup.scope.as_ref() else {
                        return;
                    };
                    if let CgroupStatus::Degraded(_) = self.cgroup.status {
                        return;
                    }
                    let pid = child.id();
                    // pre_exec 已经写过一次；这里只做核对，核对失败再补写一次（内核/权限差异）
                    let confirmed = scope.contains(pid)
                        || (scope.write_pid(pid).is_ok() && scope.contains(pid));
                    self.cgroup.status = if confirmed {
                        CgroupStatus::Active
                    } else {
                        CgroupStatus::Degraded("cgroup_membership_not_established".to_string())
                    };
                }
            }

            pub(crate) fn attach_error(&self) -> Option<&str> {
                None
            }

            /// 仅当 cgroup 作用域真正建立（Active）时写归属记录；Degraded 时不写，避免伪造证据。
            pub(crate) fn publish_scope_owner(
                &mut self,
                child: &Child,
                runs_dir: &std::path::Path,
                task_id: &str,
                attempt: u64,
            ) -> Result<(), String> {
                if !matches!(self.cgroup.status, CgroupStatus::Active) {
                    return Ok(());
                }
                let root_pid = child.id();
                let root_start = self.root_start;
                let Some(scope) = self.cgroup.scope.as_mut() else {
                    return Ok(());
                };
                scope.publish_owner(runs_dir, task_id, attempt, root_pid, root_start)
            }

            /// 观测点：`/proc` 父链快照 × 终止作用域成员交叉核对。
            ///
            /// - 父链快照：遍历 `/proc/<pid>/stat` 的 `PPid` 做传递闭包；
            /// - `controlled_pids`：**终止作用域**（本组 pgid）成员，即真正会被 `kill(-pgid)` 打到的进程；
            /// - cgroup v2 可建时，后代集合额外并入 `cgroup.procs` 成员——成员关系不随父进程退出而消失，
            ///   因此换父被收养的后代仍能被枚举，这是 v1.34 最大的观测缺口；
            /// - 任一 `/proc` 条目不可读（非"已退出"）都按不可观测处理，绝不当成"没有后代"。
            pub(crate) fn snapshot_descendants(&self, root: u32) -> DescendantSnapshot {
                #[cfg(test)]
                let _observation = ObservationGuard::enter();
                #[cfg(target_os = "linux")]
                {
                    let Some(pgid) = self.pgid else {
                        return unobservable_snapshot(root, "pgid_unknown");
                    };
                    let Some(rows) = proc_table() else {
                        return unobservable_snapshot(root, "procfs_unavailable");
                    };
                    let group: Vec<u32> = rows
                        .iter()
                        .filter(|row| row.pgrp == pgid as u32)
                        .map(|row| row.pid)
                        .collect();
                    match &self.cgroup.status {
                        CgroupStatus::Active => {
                            let Some(scope) = self.cgroup.scope.as_ref() else {
                                return unobservable_snapshot(root, "cgroup_scope_lost");
                            };
                            let members = match scope.members() {
                                Ok(members) => members,
                                // 作用域读不出来时整份快照不可用：不能退回"空"
                                Err(reason) => {
                                    return unobservable_snapshot(
                                        root,
                                        &format!("cgroup_procs_unreadable:{reason}"),
                                    );
                                }
                            };
                            let mut pids = descendants_of(root, self.root_start, &rows);
                            for pid in &members {
                                if *pid != root && !pids.contains(pid) {
                                    pids.push(*pid);
                                }
                            }
                            pids.sort_unstable();
                            let entries = pids
                                .into_iter()
                                .map(|pid| DescendantProcess {
                                    pid,
                                    in_controlled_set: group.contains(&pid),
                                })
                                .collect();
                            DescendantSnapshot {
                                root,
                                descendants: entries,
                                controlled_pids: Some(group),
                                scope: SnapshotScope::CgroupV2,
                                escaped_scope_unobservable: None,
                                unobservable: None,
                            }
                        }
                        _ => {
                            let entries = descendants_of(root, self.root_start, &rows)
                                .into_iter()
                                .map(|pid| DescendantProcess {
                                    pid,
                                    in_controlled_set: group.contains(&pid),
                                })
                                .collect();
                            DescendantSnapshot {
                                root,
                                descendants: entries,
                                controlled_pids: Some(group),
                                scope: SnapshotScope::ProcessGroup,
                                escaped_scope_unobservable: self.cgroup_degradation(),
                                unobservable: None,
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    unobservable_snapshot(root, "procfs_unavailable")
                }
            }

            /// cgroup 作用域建立失败的原因；`None` 表示作用域可用（或尚未尝试）。
            #[cfg(target_os = "linux")]
            fn cgroup_degradation(&self) -> Option<String> {
                match &self.cgroup.status {
                    CgroupStatus::Degraded(reason) => Some(reason.clone()),
                    _ => None,
                }
            }

            /// 终止前建立「仍可归属本任务」的后代名单：父链闭包 ∪ cgroup 成员，每项附
            /// `/proc/<pid>/stat` 的 `starttime`。逃出 `cgroup.procs` 且 `setsid` 的进程
            /// 不在 `cgroup.kill` 与进程组的作用范围内，根退出后父链也断了——这份终止前
            /// 的名单是唯一还能指认它的依据。
            #[cfg(target_os = "linux")]
            fn tracked_descendants(&self, root: u32) -> Vec<(u32, Option<u64>)> {
                let Some(rows) = proc_table() else {
                    return Vec::new();
                };
                let mut pids = descendants_of(root, self.root_start, &rows);
                if matches!(self.cgroup.status, CgroupStatus::Active) {
                    if let Some(scope) = self.cgroup.scope.as_ref() {
                        if let Ok(members) = scope.members() {
                            pids.extend(members);
                        }
                    }
                }
                pids.push(root);
                pids.sort_unstable();
                pids.dedup();
                let starts: std::collections::HashMap<u32, u64> =
                    rows.iter().map(|row| (row.pid, row.start_time)).collect();
                pids.into_iter()
                    .map(|pid| (pid, starts.get(&pid).copied()))
                    .collect()
            }

            pub(crate) fn terminate(&mut self, child: &mut Child) -> CleanupOutcome {
                #[cfg(test)]
                let _observation = ObservationGuard::enter();
                #[cfg(test)]
                trace_note(format!(
                    "terminate 开始 root={} pgid={:?} cgroup={:?}",
                    child.id(),
                    self.pgid,
                    self.cgroup.status
                ));
                let Some(pgid) = self.pgid else {
                    let _ = child.kill();
                    #[cfg(test)]
                    trace_note("terminate 结果=Unavailable 原因=pgid_unknown");
                    return CleanupOutcome::Unavailable;
                };
                // fail-closed：作用域不可观测时不动进程组，只终止直接子进程并如实报 Unavailable。
                // 与契约措辞「仅向直接子进程发送终止信号」一致；宁可少清理，也不盲杀。
                #[cfg(target_os = "linux")]
                let blind_scope_reason = self.cgroup_degradation().or_else(|| {
                    proc_table()
                        .is_none()
                        .then(|| "procfs_unavailable".to_string())
                });
                #[cfg(not(target_os = "linux"))]
                let blind_scope_reason = Some("procfs_unavailable".to_string());
                if let Some(reason) = blind_scope_reason {
                    let _ = child.kill();
                    let _ = child.wait();
                    #[cfg(test)]
                    trace_note(format!(
                        "terminate 结果=Unavailable 原因=scope_unobservable:{reason}"
                    ));
                    return CleanupOutcome::Unavailable;
                }
                // v1.45：动手前先记下「当前仍可归属本任务」的后代（父链闭包 ∪ cgroup 成员），
                // 附 `/proc/<pid>/stat` 的 `starttime` 防 PID 复用。逃出 `cgroup.procs` 且
                // `setsid` 的后代既不在 `cgroup.kill` 的打击范围、也不在进程组里；根退出被
                // 收养后父链断开，事后无法再枚举，所以名单必须在终止前建立。
                #[cfg(target_os = "linux")]
                let mut tracked: Vec<(u32, Option<u64>)> = self.tracked_descendants(child.id());
                #[cfg(test)]
                trace_note(format!("终止前名单 tracked={tracked:?}"));
                // 先礼后兵：SIGTERM 给宽限期，再 SIGKILL
                unsafe {
                    libc::kill(-pgid, libc::SIGTERM);
                }
                let grace_deadline = Instant::now() + TERM_GRACE;
                while Instant::now() < grace_deadline && group_exists(pgid) {
                    std::thread::sleep(Duration::from_millis(20));
                }
                if group_exists(pgid) {
                    // 宽限期窗口内仍可能新派生并逃逸，SIGKILL 前再抓一次（此时根仍存活，父链还在）
                    #[cfg(target_os = "linux")]
                    merge_tracked(&mut tracked, self.tracked_descendants(child.id()));
                    unsafe {
                        libc::kill(-pgid, libc::SIGKILL);
                    }
                }
                let _ = child.wait();

                // 判据：进程组为空，且（cgroup 作用域可用时）`cgroup.procs` 为空 + `populated 0`。
                // `cgroup.kill` 写失败不影响判定口径：成员只要还在，就会落进 StillAlive。
                let deadline = Instant::now() + CLEANUP_DEADLINE;
                #[cfg(test)]
                let mut round = 0usize;
                loop {
                    #[cfg(test)]
                    {
                        round += 1;
                        if round == 1 {
                            trace_note(format!("清扫循环开始 group_exists={}", group_exists(pgid)));
                        }
                    }
                    #[cfg(target_os = "linux")]
                    if matches!(self.cgroup.status, CgroupStatus::Active) {
                        if let Some(scope) = self.cgroup.scope.as_ref() {
                            if scope.kill_supported() {
                                // 每轮重试：fork 压力下首次 kill 可能漏掉正在派生的后代
                                let _ = scope.kill_members();
                            }
                        }
                    }
                    // 名单里的逃逸后代不在作用域内，`cgroup.kill` 打不到：逐个 SIGKILL 收口。
                    // 先按父链把存活逃逸进程新派生的子进程并入名单，再动手；只有启动时刻
                    // 一致才发信号，避免打到 PID 复用后的无关进程。
                    #[cfg(target_os = "linux")]
                    {
                        expand_tracked(&mut tracked);
                        for (pid, start) in &tracked {
                            if tracked_pid_alive(*pid, *start) {
                                unsafe {
                                    libc::kill(*pid as i32, libc::SIGKILL);
                                }
                            }
                        }
                    }
                    if !group_exists(pgid) {
                        #[cfg(target_os = "linux")]
                        {
                            if let CgroupStatus::Active = &self.cgroup.status {
                                let Some(scope) = self.cgroup.scope.as_ref() else {
                                    #[cfg(test)]
                                    trace_note("terminate 结果=Unavailable 原因=cgroup_scope_lost");
                                    return CleanupOutcome::Unavailable;
                                };
                                let members = match scope.members() {
                                    Ok(members) => members,
                                    Err(_reason) => {
                                        #[cfg(test)]
                                        trace_note(format!(
                                            "terminate 结果=Unavailable 原因=cgroup_procs_unreadable:{_reason}"
                                        ));
                                        return CleanupOutcome::Unavailable;
                                    }
                                };
                                if !members.is_empty() {
                                    if Instant::now() >= deadline {
                                        #[cfg(test)]
                                        trace_note(format!(
                                            "terminate 结果=StillAlive round={round} members={members:?}"
                                        ));
                                        return CleanupOutcome::StillAlive;
                                    }
                                    std::thread::sleep(Duration::from_millis(20));
                                    continue;
                                }
                                match scope.populated() {
                                    Ok(0) => {}
                                    Ok(_value) => {
                                        #[cfg(test)]
                                        trace_note(format!(
                                            "terminate 结果=StillAlive round={round} populated={_value}"
                                        ));
                                        return CleanupOutcome::StillAlive;
                                    }
                                    Err(_reason) => {
                                        #[cfg(test)]
                                        trace_note(format!(
                                            "terminate 结果=Unavailable 原因=cgroup_events:{_reason}"
                                        ));
                                        return CleanupOutcome::Unavailable;
                                    }
                                }
                            }
                            // v1.45：名单里仍有存活的逃逸后代（不在 cgroup / 进程组内）→ 不得宣称清空
                            if tracked
                                .iter()
                                .any(|(pid, start)| tracked_pid_alive(*pid, *start))
                            {
                                if Instant::now() >= deadline {
                                    #[cfg(test)]
                                    trace_note(format!(
                                        "terminate 结果=StillAlive round={round} 原因=tracked_alive"
                                    ));
                                    return CleanupOutcome::StillAlive;
                                }
                                std::thread::sleep(Duration::from_millis(20));
                                continue;
                            }
                        }
                        // 所有 POSIX 平台共同判据（含 macOS）：组记账为空后必须再用独立快照复核。
                        // 整份快照不可观测 → Unavailable；期望的逃逸作用域没建起来
                        // （cgroup_v2_unavailable / procfs 缺失）→ 同样 Unavailable：
                        // 不允许把「降级/看不见作用域」读成 Cleaned 后向 PM 宣称 cleanup_claimed。
                        let snapshot = self.snapshot_descendants(child.id());
                        if let Some(_reason) = snapshot.unobservable.as_ref() {
                            #[cfg(test)]
                            trace_note(format!(
                                "terminate 结果=Unavailable round={round} 原因=snapshot:{_reason}"
                            ));
                            return CleanupOutcome::Unavailable;
                        }
                        if let Some(_reason) = snapshot.escaped_scope_unobservable.as_ref() {
                            #[cfg(test)]
                            trace_note(format!(
                                "terminate 结果=Unavailable round={round} 原因=escaped_scope_unobservable:{_reason}"
                            ));
                            return CleanupOutcome::Unavailable;
                        }
                        if !snapshot.is_empty() {
                            #[cfg(test)]
                            trace_note(format!(
                                "terminate 结果=StillAlive round={round} 原因=snapshot_not_empty"
                            ));
                            return CleanupOutcome::StillAlive;
                        }
                        #[cfg(test)]
                        trace_note(format!("terminate 结果=Cleaned round={round}"));
                        return CleanupOutcome::Cleaned;
                    }
                    if Instant::now() >= deadline {
                        #[cfg(test)]
                        trace_note(format!(
                            "terminate 结果=StillAlive round={round} 原因=deadline"
                        ));
                        return CleanupOutcome::StillAlive;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }

            /// 失败时的现场快照（仅测试编译）：进程/作用域状态、并发度、观测时序。
            /// 只读，不改变任何判定；时序取走后清空，避免跨用例串扰。
            #[cfg(test)]
            pub(crate) fn diagnostics(&self, root: u32) -> String {
                let (active, peak) = observation_stats();
                let mut lines = vec![
                    format!(
                        "root={root} pgid={:?} root_start={:?}",
                        self.pgid, self.root_start
                    ),
                    format!(
                        "观测并发: active={active} peak={peak} cpu={:?}",
                        std::thread::available_parallelism()
                            .map(|count| count.get())
                            .unwrap_or(0)
                    ),
                    format!("cgroup.status={:?}", self.cgroup.status),
                ];
                if let Some(scope) = self.cgroup.scope.as_ref() {
                    let procs = std::fs::read_to_string(&scope.procs)
                        .unwrap_or_else(|error| format!("<读取失败: {error}>"));
                    let events = std::fs::read_to_string(scope.dir.join("cgroup.events"))
                        .unwrap_or_else(|error| format!("<读取失败: {error}>"));
                    lines.push(format!("cgroup.procs={procs:?}"));
                    lines.push(format!("cgroup.events={events:?}"));
                    lines.push(format!("members={:?}", scope.members()));
                    lines.push(format!("populated={:?}", scope.populated()));
                } else {
                    lines.push("cgroup.scope=(无)".to_string());
                }
                lines.push(format!(
                    "/proc 行数={:?}",
                    proc_table().map(|rows| rows.len())
                ));
                lines.push(format!("快照={:?}", self.snapshot_descendants(root)));
                lines.push("时序:".to_string());
                lines.extend(trace_take());
                lines.join("\n")
            }
        }

        fn unobservable_snapshot(root: u32, reason: &str) -> DescendantSnapshot {
            #[cfg(test)]
            trace_note(format!("snapshot 不可观测 root={root} reason={reason}"));
            DescendantSnapshot {
                root,
                descendants: Vec::new(),
                controlled_pids: None,
                scope: SnapshotScope::ProcessGroup,
                escaped_scope_unobservable: Some(reason.to_string()),
                unobservable: Some(reason.to_string()),
            }
        }

        /// 独立 cgroup v2 作用域：既做观测，也在 `cgroup.kill` 可用时承担终止。
        ///
        /// 成员关系写在 `cgroup.procs` 里，与父进程是否存活无关——这正是"换父被收养后
        /// 仍能枚举后代"的依据。目录在 Drop 时尝试删除；成员还在时删不掉，留给后续清理。
        #[cfg(target_os = "linux")]
        pub(crate) struct CgroupScope {
            dir: std::path::PathBuf,
            procs: std::path::PathBuf,
            writer: std::cell::RefCell<Option<std::fs::File>>,
            /// 归属记录路径：写明本作用域属于哪一轮已知 attempt。
            /// 正常收尾且目录删除成功时一并删除；bridge 猝死时留在盘上，供下一次启动判定。
            owner_record: Option<std::path::PathBuf>,
        }

        #[cfg(target_os = "linux")]
        impl CgroupScope {
            /// 建立失败一律返回原因字符串（`cgroup_v2_unavailable` / `cgroup_create_failed:...`），
            /// 调用方把它记进 `escaped_scope_unobservable`，绝不静默当成空集合。
            fn prepare() -> Result<Self, String> {
                Self::prepare_from(std::path::Path::new("/sys/fs/cgroup"))
            }

            fn prepare_from(base: &std::path::Path) -> Result<Self, String> {
                if !base.join("cgroup.controllers").exists() {
                    return Err("cgroup_v2_unavailable".to_string());
                }
                let name = format!(
                    "aw-mcp-bridge-{}-{}",
                    std::process::id(),
                    SCOPE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                );
                let dir = base.join(name);
                std::fs::create_dir(&dir)
                    .map_err(|error| format!("cgroup_create_failed:{error}"))?;
                Ok(Self {
                    procs: dir.join("cgroup.procs"),
                    dir,
                    writer: std::cell::RefCell::new(None),
                    owner_record: None,
                })
            }

            /// 把作用域与「本轮已知 attempt」绑定：这是下一次启动唯一可以据以动手的证据。
            fn publish_owner(
                &mut self,
                runs_dir: &std::path::Path,
                task_id: &str,
                attempt: u64,
                root_pid: u32,
                root_start: Option<u64>,
            ) -> Result<(), String> {
                let owner_dir = runs_dir.join(crate::server::SCOPE_OWNER_DIR);
                std::fs::create_dir_all(&owner_dir)
                    .map_err(|error| format!("scope_owner_dir_failed:{error}"))?;
                let name = self
                    .dir
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "scope_name_unreadable".to_string())?;
                let record = serde_json::json!({
                    "version": crate::server::SCOPE_OWNER_VERSION,
                    "scope": name,
                    "cgroup": self.dir.to_string_lossy(),
                    "bridge_pid": std::process::id(),
                    "bridge_start": proc_start_time(std::process::id()),
                    "task_id": task_id,
                    "attempt": attempt,
                    "root_pid": root_pid,
                    "root_start": root_start,
                    "createdAt": crate::server::now_stamp(),
                });
                let path = owner_dir.join(format!("{name}.json"));
                crate::server::write_json_atomic(&path, &record)?;
                self.owner_record = Some(path);
                Ok(())
            }

            /// 打开写端并取出 fd，供 `pre_exec` 在子进程内写自身 PID。
            fn writer(&self) -> Result<std::os::fd::RawFd, String> {
                use std::os::fd::AsRawFd;
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&self.procs)
                    .map_err(|error| format!("cgroup_procs_unwritable:{error}"))?;
                let fd = file.as_raw_fd();
                self.writer.borrow_mut().replace(file);
                Ok(fd)
            }

            fn members(&self) -> Result<Vec<u32>, String> {
                let text =
                    std::fs::read_to_string(&self.procs).map_err(|error| format!("{error}"))?;
                let mut members: Vec<u32> = text
                    .lines()
                    .filter_map(|line| line.trim().parse::<u32>().ok())
                    .collect();
                members.sort_unstable();
                Ok(members)
            }

            fn contains(&self, pid: u32) -> bool {
                self.members()
                    .map(|members| members.contains(&pid))
                    .unwrap_or(false)
            }

            fn write_pid(&self, pid: u32) -> std::io::Result<()> {
                std::fs::write(&self.procs, format!("{pid}\n"))
            }

            /// 内核是否提供 `cgroup.kill`（v5.14+ 的非 root cgroup 才有该文件）。
            fn kill_supported(&self) -> bool {
                self.dir.join("cgroup.kill").exists()
            }

            /// 终止作用域内全部成员（含换组/换父的逃逸后代）。
            /// 只负责发信号；是否真的清空由 `cgroup.procs` / `cgroup.events` 独立复核。
            fn kill_members(&self) -> Result<(), String> {
                std::fs::write(self.dir.join("cgroup.kill"), "1")
                    .map_err(|error| format!("cgroup_kill_failed:{error}"))
            }

            /// 作用域是否有成员，来自 `cgroup.events` 的 `populated`。
            /// 与 `cgroup.procs` 互为独立观测点：两者都为空才算作用域清空。
            fn populated(&self) -> Result<u32, String> {
                let text = std::fs::read_to_string(self.dir.join("cgroup.events"))
                    .map_err(|error| format!("cgroup_events_unreadable:{error}"))?;
                for line in text.lines() {
                    if let Some(value) = line.trim().strip_prefix("populated ") {
                        return value
                            .trim()
                            .parse::<u32>()
                            .map_err(|error| format!("cgroup_events_unparsable:{error}"));
                    }
                }
                Err("cgroup_events_missing_populated".to_string())
            }
        }

        #[cfg(target_os = "linux")]
        impl Drop for CgroupScope {
            fn drop(&mut self) {
                self.writer.borrow_mut().take();
                // 成员还在（逃逸后代未退出）时删不掉：重试几次，仍失败就留空目录。
                // 不在这里杀进程——观测点不做终止动作。
                for _ in 0..3 {
                    if std::fs::remove_dir(&self.dir).is_ok() {
                        // 目录真的删掉了才删归属记录：成员仍在时保留证据，供启动回收判定
                        if let Some(path) = self.owner_record.take() {
                            let _ = std::fs::remove_file(path);
                        }
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }

        #[cfg(target_os = "linux")]
        static SCOPE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        /// 启动回收用观测：只枚举名字形状为 `aw-mcp-bridge-*` 的目录，其它目录一律不碰。
        #[cfg(target_os = "linux")]
        pub(crate) fn managed_scope_dirs(
            base: &std::path::Path,
        ) -> Result<Vec<(String, std::path::PathBuf)>, String> {
            let entries = std::fs::read_dir(base)
                .map_err(|error| format!("cgroup_base_unreadable:{error}"))?;
            let mut scopes = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with(crate::server::CGROUP_SCOPE_PREFIX) {
                    continue;
                }
                // 只接受 [a-z0-9-]，防止用奇怪目录名做路径拼接
                if !name.chars().all(|value| {
                    value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-'
                }) {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    scopes.push((name, path));
                }
            }
            scopes.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(scopes)
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn read_scope_members(dir: &std::path::Path) -> Result<Vec<u32>, String> {
            let text = std::fs::read_to_string(dir.join("cgroup.procs"))
                .map_err(|error| format!("cgroup_procs_unreadable:{error}"))?;
            let mut members: Vec<u32> = text
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok())
                .collect();
            members.sort_unstable();
            Ok(members)
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn read_scope_populated(dir: &std::path::Path) -> Result<u32, String> {
            let text = std::fs::read_to_string(dir.join("cgroup.events"))
                .map_err(|error| format!("cgroup_events_unreadable:{error}"))?;
            for line in text.lines() {
                if let Some(value) = line.trim().strip_prefix("populated ") {
                    return value
                        .trim()
                        .parse::<u32>()
                        .map_err(|error| format!("cgroup_events_unparsable:{error}"));
                }
            }
            Err("cgroup_events_missing_populated".to_string())
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn scope_kill_supported(dir: &std::path::Path) -> bool {
            dir.join("cgroup.kill").exists()
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn kill_scope(dir: &std::path::Path) -> Result<(), String> {
            std::fs::write(dir.join("cgroup.kill"), "1")
                .map_err(|error| format!("cgroup_kill_failed:{error}"))
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn remove_scope(dir: &std::path::Path) -> Result<(), String> {
            match std::fs::remove_dir(dir) {
                Ok(()) => Ok(()),
                Err(_) => {
                    // 真实 cgroupfs 上 rmdir 会连同控制文件一起删掉；非 cgroupfs 的目录
                    // （测试夹具 / 份外残留文件）会返回 ENOTEMPTY。先清掉已知控制文件再重试一次：
                    // 真实 cgroupfs 上这些 unlink 会被内核拒绝，最终仍按原始失败上报。
                    for name in ["cgroup.procs", "cgroup.events", "cgroup.kill"] {
                        let _ = std::fs::remove_file(dir.join(name));
                    }
                    std::fs::remove_dir(dir)
                        .map_err(|error| format!("cgroup_remove_failed:{error}"))
                }
            }
        }

        /// `/proc/<pid>/stat` 的 starttime；读不到返回 None（调用方按不可验证处理）。
        #[cfg(target_os = "linux")]
        pub(crate) fn proc_start(pid: u32) -> Option<u64> {
            proc_start_time(pid)
        }

        #[cfg(target_os = "linux")]
        pub(crate) fn kill_pid(pid: u32) -> Result<(), String> {
            let rc = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            if rc == 0 {
                Ok(())
            } else {
                Err(format!(
                    "kill_failed:{pid}:{}",
                    std::io::Error::last_os_error()
                ))
            }
        }

        /// bridge 猝死时连带收掉直接子进程（Linux 缺口兜底）。
        ///
        /// `PR_SET_PDEATHSIG` 的特点是：
        /// - 在 fork 出的子进程里设置，跨越 exec 保留；
        /// - 只在"父进程死亡"这一刻由内核投递一次信号，之后不影响正常退出；
        /// - **不会遗传给孙进程**（fork 时清零）。因此这里只保证 bridge 的直接子进程被收掉；
        ///   子进程自己 fork 出来的后代仍会 reparent 到 init 继续运行。该限制必须如实写入
        ///   契约与 capability（Windows 作业对象是整棵树，两者不可混为一谈）。
        ///
        /// 竞态：父进程可能在 fork 与 `prctl` 之间死亡，此时 PDEATHSIG 已来不及生效。
        /// 设完立即复查 `getppid()`；如果不等于预期的 bridge pid，说明父已死，直接 `_exit`
        /// 而不是去执行用户命令。`pre_exec` 里只能用异步信号安全的调用，这里都用 libc。
        #[cfg(target_os = "linux")]
        fn install_pdeathsig_pre_exec(command: &mut Command) {
            use std::os::unix::process::CommandExt;
            let parent_pid = std::process::id() as libc::pid_t;
            unsafe {
                command.pre_exec(move || {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::getppid() != parent_pid {
                        // 父进程已经死了：不再执行任何用户代码，直接退出
                        libc::_exit(128);
                    }
                    Ok(())
                });
            }
        }

        /// 在 fork 之后、exec 之前把自身 PID 写进 `cgroup.procs`。
        ///
        /// 这里只能调用异步信号安全的系统调用：不分配内存、不加锁、不碰 std 的错误类型。
        /// 写失败不阻断 exec；父进程在 `attach` 里核对成员关系，核对不上就降级记录原因。
        #[cfg(target_os = "linux")]
        fn install_cgroup_pre_exec(command: &mut Command, fd: std::os::fd::RawFd) {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(move || {
                    let pid = libc::getpid();
                    if pid <= 0 {
                        return Ok(());
                    }
                    let mut reversed = [0u8; 20];
                    let mut len = 0usize;
                    let mut value = pid as u64;
                    loop {
                        reversed[len] = b'0' + (value % 10) as u8;
                        len += 1;
                        value /= 10;
                        if value == 0 {
                            break;
                        }
                    }
                    let mut buffer = [0u8; 21];
                    for index in 0..len {
                        buffer[index] = reversed[len - 1 - index];
                    }
                    buffer[len] = b'\n';
                    let total = len + 1;
                    let mut written = 0usize;
                    while written < total {
                        let rc =
                            libc::write(fd, buffer[written..].as_ptr().cast(), total - written);
                        if rc <= 0 {
                            break;
                        }
                        written += rc as usize;
                    }
                    Ok(())
                });
            }
        }

        /// 测试注入（v1.50）：模拟「某个 pid 的 `/proc/<pid>/stat` 读取失败」。
        /// 用来确定性地区分两种侧别——已退出进程的读失败必须被跳过（不再让整表 Unavailable），
        /// 存活进程的读失败必须仍然判为不可观测。只编译进测试，不进入生产二进制。
        #[cfg(all(test, target_os = "linux"))]
        mod proc_fault {
            use std::cell::RefCell;

            struct Fault {
                pid: u32,
                gone: bool,
            }

            // 线程内注入：proc_table 与断言在同一线程，避免并行用例互相消费对方的注入。
            thread_local! {
                static FAULT: RefCell<Option<Fault>> = const { RefCell::new(None) };
            }

            pub(super) fn set(pid: u32, gone: bool) {
                FAULT.with(|slot| *slot.borrow_mut() = Some(Fault { pid, gone }));
            }

            /// 命中一次即消费，避免影响后续扫描。
            pub(super) fn take_for(pid: u32) -> Option<bool> {
                FAULT.with(|slot| {
                    let mut guard = slot.borrow_mut();
                    match guard.as_ref() {
                        Some(fault) if fault.pid == pid => {
                            let gone = fault.gone;
                            *guard = None;
                            Some(gone)
                        }
                        _ => None,
                    }
                })
            }
        }

        #[cfg(target_os = "linux")]
        struct ProcRow {
            pid: u32,
            ppid: u32,
            pgrp: u32,
            /// `/proc/<pid>/stat` 第 22 字段 `starttime`：进程启动时刻（自开机起的时钟滴答）。
            /// 用于父链边校验：PID 被复用后，旧子进程的 `ppid` 会指向新进程。
            start_time: u64,
        }

        /// 全量 `/proc` 扫描：读不到 `stat` 且进程仍存在（如 hidepid/权限）都按不可观测处理；
        /// 只有"进程已退出"才跳过。退出判定不只看 `NotFound`：进程在 readdir 与 read 之间
        /// 退出时，内核可能返回 `ESRCH` 或读到空内容（v1.49 高负载下偶发 Unavailable 的
        /// 嫌疑路径）。因此凡是「读失败/解析失败但 `/proc/<pid>` 已不存在」都跳过，
        /// 其余仍然严格返回 None——绝不因为一条残项就宣称某个活进程不可见。
        #[cfg(target_os = "linux")]
        fn proc_table() -> Option<Vec<ProcRow>> {
            let entries = std::fs::read_dir("/proc").ok()?;
            let mut rows = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if !name.chars().all(|ch| ch.is_ascii_digit()) {
                    continue;
                }
                let Ok(pid) = name.parse::<u32>() else {
                    continue;
                };
                #[cfg(test)]
                if let Some(gone) = proc_fault::take_for(pid) {
                    if gone {
                        trace_note(format!("proc_stat_read_injected pid={pid} gone=true"));
                        continue;
                    }
                    trace_note(format!("proc_table_unreadable pid={pid} injected=true"));
                    return None;
                }
                let text = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                    Ok(text) => text,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_error) => {
                        if proc_entry_gone(pid) {
                            continue;
                        }
                        #[cfg(test)]
                        trace_note(format!(
                            "proc_table_unreadable pid={pid} kind={:?} error={_error}",
                            _error.kind()
                        ));
                        return None;
                    }
                };
                let Some(row) = parse_stat(&text, pid) else {
                    if proc_entry_gone(pid) {
                        continue;
                    }
                    #[cfg(test)]
                    trace_note(format!(
                        "proc_table_unparsable pid={pid} len={}",
                        text.len()
                    ));
                    return None;
                };
                rows.push(row);
            }
            if rows.is_empty() {
                None
            } else {
                Some(rows)
            }
        }

        /// 进程是否真的已经退出：`/proc/<pid>` 目录不存在。
        #[cfg(target_os = "linux")]
        fn proc_entry_gone(pid: u32) -> bool {
            !std::path::Path::new(&format!("/proc/{pid}")).exists()
        }

        /// `/proc/<pid>/stat` 关键字段：`pid (comm) state ppid pgrp ... starttime(22)`。
        /// `comm` 可含空格与括号，因此以最后一个 `)` 为界解析。
        #[cfg(target_os = "linux")]
        fn parse_stat(text: &str, pid: u32) -> Option<ProcRow> {
            let close = text.rfind(')')?;
            let recorded_pid: u32 = text.get(..text.find('(')?)?.trim().parse().ok()?;
            if recorded_pid != pid {
                return None;
            }
            let rest = text.get(close + 1..)?.trim_start();
            let mut fields = rest.split_whitespace();
            let _state = fields.next()?;
            let ppid: u32 = fields.next()?.parse().ok()?;
            let pgrp: u32 = fields.next()?.parse().ok()?;
            // 跳过 session..itrealvalue（字段 6..=21），第 22 个字段是 starttime
            let start_time: u64 = fields.nth(16)?.parse().ok()?;
            Some(ProcRow {
                pid,
                ppid,
                pgrp,
                start_time,
            })
        }

        /// 读某进程的启动时刻；读不到返回 `None`（边校验退回旧行为）。
        #[cfg(target_os = "linux")]
        fn proc_start_time(pid: u32) -> Option<u64> {
            let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            parse_stat(&text, pid).map(|row| row.start_time)
        }

        /// 「启动时刻一致」才算同一个进程：PID 被复用后返回 `false`，绝不误杀无关进程。
        /// `start` 为 `None`（当时读不到启动时刻）时只按存在性判断。
        #[cfg(target_os = "linux")]
        fn tracked_pid_alive(pid: u32, start: Option<u64>) -> bool {
            match (proc_start_time(pid), start) {
                (Some(current), Some(expected)) => current == expected,
                (Some(_), None) => true,
                (None, _) => false,
            }
        }

        /// 合并两份「PID + 启动时刻」名单，同一 PID 只保留第一次读到的启动时刻。
        #[cfg(target_os = "linux")]
        fn merge_tracked(into: &mut Vec<(u32, Option<u64>)>, extra: Vec<(u32, Option<u64>)>) {
            for (pid, start) in extra {
                if let Some(slot) = into.iter_mut().find(|(known, _)| *known == pid) {
                    if slot.1.is_none() {
                        slot.1 = start;
                    }
                } else {
                    into.push((pid, start));
                }
            }
        }

        /// 名单里每个仍存活进程在 `/proc` 里的后代也并入名单：逃逸进程在终止过程中继续
        /// fork 时，新子进程同样不在 cgroup / 进程组内，必须一起收口。
        #[cfg(target_os = "linux")]
        fn expand_tracked(into: &mut Vec<(u32, Option<u64>)>) {
            let Some(rows) = proc_table() else {
                return;
            };
            let starts: std::collections::HashMap<u32, u64> =
                rows.iter().map(|row| (row.pid, row.start_time)).collect();
            let mut extra = Vec::new();
            for (pid, start) in into.iter() {
                if !tracked_pid_alive(*pid, *start) {
                    continue;
                }
                for child in descendants_of(*pid, *start, &rows) {
                    extra.push((child, starts.get(&child).copied()));
                }
            }
            merge_tracked(into, extra);
        }

        /// 读某进程当前父 PID；进程不存在或读不到返回 `None`（测试用于确认"已被收养"）。
        #[cfg(all(test, target_os = "linux"))]
        fn proc_ppid(pid: u32) -> Option<u32> {
            let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            parse_stat(&text, pid).map(|row| row.ppid)
        }

        /// 按父链做传递闭包（含环保护），返回排好序的后代 PID。
        ///
        /// 每条边要求「子进程启动时刻 ≥ 父进程启动时刻」。Linux 会复用 PID：旧父进程退出后
        /// 同号 PID 被新进程持有，旧子进程的 `ppid` 会把无关进程树挂到新根上；`starttime`
        /// 能识别并拒绝这类边。只有「这条边的父进程启动时刻读不到」时才退回旧行为（接受该边），
        /// 不因为一个未知时刻把整棵树都放宽。
        #[cfg(target_os = "linux")]
        fn descendants_of(root: u32, root_start_time: Option<u64>, rows: &[ProcRow]) -> Vec<u32> {
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            seen.insert(root);
            let mut frontier = vec![(root, root_start_time)];
            let mut descendants = Vec::new();
            while let Some((parent, parent_start_time)) = frontier.pop() {
                for row in rows {
                    if row.ppid != parent || seen.contains(&row.pid) {
                        continue;
                    }
                    if !edge_respects_start_order(parent_start_time, row.start_time) {
                        continue;
                    }
                    seen.insert(row.pid);
                    descendants.push(row.pid);
                    frontier.push((row.pid, Some(row.start_time)));
                }
            }
            descendants.sort_unstable();
            descendants
        }

        #[cfg(target_os = "linux")]
        fn edge_respects_start_order(parent: Option<u64>, child: u64) -> bool {
            parent.map_or(true, |parent| child >= parent)
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

            /// 起一个真实的新会话（setsid）进程，并返回持有它的 shell 与逃逸进程 PID。
            /// 本机没有 `setsid` 时返回 `None`，由调用方显式跳过，不当成通过。
            #[cfg(target_os = "linux")]
            fn setsid_escapee(dir: &std::path::Path) -> Option<(std::process::Child, u32)> {
                let pid_file = dir.join("pid.txt");
                let script = format!(
                    "if command -v setsid >/dev/null 2>&1; then setsid sleep 30 >/dev/null 2>&1 & echo $! > '{}'; wait; else echo skip > '{}'; fi",
                    pid_file.display(),
                    pid_file.display()
                );
                let mut holder = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn setsid fixture");
                for _ in 0..100 {
                    if let Ok(text) = std::fs::read_to_string(&pid_file) {
                        let text = text.trim().to_string();
                        if text == "skip" {
                            break;
                        }
                        if let Ok(pid) = text.parse::<u32>() {
                            return Some((holder, pid));
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let _ = holder.kill();
                let _ = holder.wait();
                None
            }

            // v1.40 判据：同号 PID 复用后，旧父链留下的子进程启动时刻早于新根，这条边必须
            // 被拒绝；只有启动时刻不早于父进程的边才允许进入后代集合。启动时刻读不到的边
            // 退回旧行为（接受），避免因缺证据扩大误判。
            #[test]
            fn pid_reuse_parent_chain_edge_is_rejected() {
                let root = 500;
                let row = |pid: u32, ppid: u32, start_time: u64| ProcRow {
                    pid,
                    ppid,
                    pgrp: root,
                    start_time,
                };
                let rows = vec![
                    // 旧父链遗留：早于新根，必须拒绝
                    row(900, root, 1000),
                    // 祖先被拒绝，其后代也不可达
                    row(901, 900, 1100),
                    // 真正的新后代
                    row(902, root, 6000),
                    row(903, 902, 7000),
                    // 中间层 PID 复用：905 早于父进程 902，必须拒绝
                    row(905, 902, 4000),
                    // 晚于根启动时刻的直接子进程
                    row(907, root, 6500),
                ];
                println!(
                    "[evidence] linux_pid_reuse rows={} result={:?}",
                    rows.len(),
                    descendants_of(root, Some(5000), &rows)
                );
                assert_eq!(descendants_of(root, Some(5000), &rows), vec![902, 903, 907]);
                // 根启动时刻读不到：只放宽"根出发的边"，更深层边仍按已知时刻校验，
                // 所以 902→905（905 早于 902）依然被拒绝
                assert_eq!(
                    descendants_of(root, None, &rows),
                    vec![900, 901, 902, 903, 907]
                );
                assert!(!edge_respects_start_order(Some(5000), 1000));
                assert!(edge_respects_start_order(Some(5000), 6000));
                assert!(edge_respects_start_order(None, 1));
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
                        let trimmed = text.trim();
                        // 重定向会先创建空文件，`echo $!` 稍后才写入；读到空内容不算就绪。
                        if !trimmed.is_empty() {
                            content = Some(trimmed.to_string());
                            break;
                        }
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
                // sh 写完 PID 就退出，此时 setsid 可能还没真正切到新会话：等它离开原组
                let deadline = Instant::now() + Duration::from_secs(2);
                while group_exists(pgid) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(20));
                }
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

            // 观测点：/proc 父链快照 × 进程组成员交叉核对（终止前后）
            #[test]
            fn descendant_snapshot_cross_checks_process_group() {
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg("sleep 30 & sleep 30 & wait")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn sh");
                supervisor.attach(&mut child);
                let root = child.id();

                let mut observed = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    if snapshot.unobservable.is_none()
                        && snapshot.descendants.len() >= 2
                        && snapshot.observed_descendants_are_controlled()
                    {
                        observed = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = match observed {
                    Some(snapshot) => snapshot,
                    None => {
                        let last = supervisor.snapshot_descendants(root);
                        panic!(
                            "必须能观测到进程组内的后代；最后一次快照={last:?}\n现场诊断：\n{}",
                            supervisor.diagnostics(root)
                        );
                    }
                };
                let mut expected: Vec<u32> =
                    before.descendants.iter().map(|entry| entry.pid).collect();
                expected.push(root);
                expected.sort_unstable();
                let mut actual = before.controlled_pids.clone().expect("进程组成员必须可读");
                actual.sort_unstable();
                println!(
                    "[evidence] posix_descendant_snapshot before root={root} descendants={:?} controlled={:?}",
                    before.descendants, before.controlled_pids
                );
                assert_eq!(
                    actual, expected,
                    "两个独立观测点必须一致：进程组成员 = 根进程 + 父链快照后代"
                );
                assert!(
                    before.outside_controlled_set().is_empty(),
                    "同组后代不应出现逃逸标记"
                );

                let outcome = supervisor.terminate(&mut child);
                let diag = supervisor.diagnostics(root);
                let after = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] posix_descendant_snapshot after terminate={outcome:?} descendants={:?} controlled={:?} unobservable={:?}",
                    after.descendants, after.controlled_pids, after.unobservable
                );
                println!("[evidence][diagnostic] posix_descendant_snapshot 现场\n{diag}");
                assert_eq!(
                    outcome,
                    CleanupOutcome::Cleaned,
                    "终止结果必须为 Cleaned；现场诊断：\n{diag}"
                );
                assert!(
                    after.unobservable.is_none(),
                    "终止后快照必须可观测；现场诊断：\n{diag}"
                );
                assert!(after.is_empty(), "终止后不得有存活后代；现场诊断：\n{diag}");
                assert_eq!(
                    after.controlled_pids,
                    Some(Vec::new()),
                    "终止后进程组必须为空；现场诊断：\n{diag}"
                );
            }

            // 复现驱动（v1.50）：受控负载下循环「建作用域 → 观测 → 终止」，统计
            // `Unavailable` 的出现频率、原因与当时的观测并发峰值。默认不跑（#[ignore]），
            // 由 scripts/posix-timeout-verify/unavailable-repro.sh 多组负载驱动。
            //
            // 环境变量：
            //   AW_REPRO_ROUNDS       每个 observation worker 的轮数（默认 50）
            //   AW_REPRO_WORKERS      并发观测线程数（默认 1；>1 时多个线程同时 snapshot+terminate，
            //                         复现原失败所需的并发观测峰值）
            //   AW_REPRO_LOAD_THREADS 额外负载线程数（默认 0；一半持续 spawn/退出短命进程，一半狂扫 /proc）
            //   AW_REPRO_CHILDREN     每个根进程派生的 sleep 后代数（默认 2）
            //
            // 判定：本用例只测量、不在出现 Unavailable 时失败；它回答的是「能不能复现、哪一侧」。
            // 并发观测峰值来自生产同一套统计（ObservationGuard），与门禁用例口径一致。
            #[cfg(target_os = "linux")]
            #[test]
            #[ignore = "复现驱动：需手动触发，不属于默认门禁"]
            fn unavailable_frequency_under_controlled_load() {
                use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

                fn env_usize(name: &str, default: usize) -> usize {
                    std::env::var(name)
                        .ok()
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(default)
                }

                fn loadavg() -> f64 {
                    std::fs::read_to_string("/proc/loadavg")
                        .ok()
                        .and_then(|text| text.split_whitespace().next()?.parse().ok())
                        .unwrap_or(0.0)
                }

                fn container_kind() -> &'static str {
                    if std::path::Path::new("/.dockerenv").exists() {
                        return "docker";
                    }
                    let cgroup = std::fs::read_to_string("/proc/1/cgroup").unwrap_or_default();
                    for hint in ["kubepods", "docker", "lxc", "containerd"] {
                        if cgroup.contains(hint) {
                            return "container";
                        }
                    }
                    "bare-metal/vm"
                }

                let rounds = env_usize("AW_REPRO_ROUNDS", 50).max(1);
                let workers = env_usize("AW_REPRO_WORKERS", 1).max(1);
                let load_threads = env_usize("AW_REPRO_LOAD_THREADS", 0);
                let children = env_usize("AW_REPRO_CHILDREN", 2).clamp(1, 16);
                let cpus = std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(0);

                // 负载线程：一半 fork/exit 制造 /proc 目录 churn，一半直接狂扫 /proc，
                // 模拟「另一批用例同时在观测」的高并发现场。
                let stop = std::sync::Arc::new(AtomicBool::new(false));
                let spawns = std::sync::Arc::new(AtomicU64::new(0));
                let scans = std::sync::Arc::new(AtomicU64::new(0));
                let mut loaders = Vec::new();
                for index in 0..load_threads {
                    let stop = std::sync::Arc::clone(&stop);
                    let spawns = std::sync::Arc::clone(&spawns);
                    let scans = std::sync::Arc::clone(&scans);
                    loaders.push(std::thread::spawn(move || {
                        let scanner = index % 2 == 1;
                        while !stop.load(Ordering::Relaxed) {
                            if scanner {
                                let _ = proc_table();
                                scans.fetch_add(1, Ordering::Relaxed);
                            } else {
                                let _ = Command::new("sh")
                                    .arg("-c")
                                    .arg("exit 0")
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .status();
                                spawns.fetch_add(1, Ordering::Relaxed);
                            }
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }));
                }

                let cleaned = std::sync::Arc::new(AtomicU64::new(0));
                let unavailable = std::sync::Arc::new(AtomicU64::new(0));
                let still_alive = std::sync::Arc::new(AtomicU64::new(0));
                let observe_fail = std::sync::Arc::new(AtomicU64::new(0));
                let reasons: std::sync::Arc<
                    std::sync::Mutex<std::collections::BTreeMap<String, usize>>,
                > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new()));
                let max_peak = std::sync::Arc::new(AtomicU64::new(0));
                let max_loadavg = std::sync::Arc::new(std::sync::Mutex::new(0.0f64));
                let cgroup_status = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

                let mut script = String::new();
                for _ in 0..children {
                    script.push_str("sleep 30 & ");
                }
                script.push_str("wait");

                // 并发观测线程：每个线程独立建作用域/观测/终止，复现原失败时的高并发观测峰值。
                let mut observers = Vec::new();
                for _ in 0..workers {
                    let script = script.clone();
                    let cleaned = std::sync::Arc::clone(&cleaned);
                    let unavailable = std::sync::Arc::clone(&unavailable);
                    let still_alive = std::sync::Arc::clone(&still_alive);
                    let observe_fail = std::sync::Arc::clone(&observe_fail);
                    let reasons = std::sync::Arc::clone(&reasons);
                    let max_peak = std::sync::Arc::clone(&max_peak);
                    let max_loadavg = std::sync::Arc::clone(&max_loadavg);
                    let cgroup_status = std::sync::Arc::clone(&cgroup_status);
                    observers.push(std::thread::spawn(move || {
                        for _ in 0..rounds {
                            let mut supervisor = Supervisor::new();
                            let mut command = Command::new("sh");
                            command
                                .arg("-c")
                                .arg(&script)
                                .stdout(Stdio::null())
                                .stderr(Stdio::null());
                            supervisor.configure(&mut command);
                            let mut child = command.spawn().expect("spawn sh");
                            supervisor.attach(&mut child);
                            let root = child.id();
                            *cgroup_status.lock().unwrap() =
                                format!("{:?}", supervisor.cgroup.status);

                            let deadline = Instant::now() + Duration::from_secs(2);
                            let mut observed = false;
                            while Instant::now() < deadline {
                                let snapshot = supervisor.snapshot_descendants(root);
                                if snapshot.unobservable.is_none()
                                    && snapshot.descendants.len() >= children
                                {
                                    observed = true;
                                    break;
                                }
                                std::thread::sleep(Duration::from_millis(5));
                            }
                            if !observed {
                                observe_fail.fetch_add(1, Ordering::Relaxed);
                            }
                            // 清掉本轮的观测时序，只保留 terminate 阶段的判定行（TLS 按线程隔离）。
                            let _ = trace_take();
                            let outcome = supervisor.terminate(&mut child);
                            let trace = trace_take();
                            let (active, peak) = observation_stats();
                            max_peak.fetch_max(peak.max(active) as u64, Ordering::Relaxed);
                            let sample = loadavg();
                            let mut max = max_loadavg.lock().unwrap();
                            if sample > *max {
                                *max = sample;
                            }
                            drop(max);
                            match outcome {
                                CleanupOutcome::Cleaned => {
                                    cleaned.fetch_add(1, Ordering::Relaxed);
                                }
                                CleanupOutcome::StillAlive => {
                                    still_alive.fetch_add(1, Ordering::Relaxed);
                                }
                                CleanupOutcome::Unavailable => {
                                    unavailable.fetch_add(1, Ordering::Relaxed);
                                    let reason = trace
                                        .iter()
                                        .find_map(|line| line.split("原因=").nth(1))
                                        .map(str::to_string)
                                        .unwrap_or_else(|| "unknown".to_string());
                                    let side = if reason.contains("procfs_unavailable") {
                                        "procfs"
                                    } else if reason.contains("cgroup_procs_unreadable")
                                        || reason.contains("cgroup_events")
                                    {
                                        "cgroup"
                                    } else if reason.contains("cgroup_scope_lost") {
                                        "scope_lost"
                                    } else {
                                        "other"
                                    };
                                    *reasons
                                        .lock()
                                        .unwrap()
                                        .entry(format!("{side}:{reason}"))
                                        .or_insert(0) += 1;
                                }
                            }
                        }
                    }));
                }
                for observer in observers {
                    let _ = observer.join();
                }

                stop.store(true, Ordering::Relaxed);
                for loader in loaders {
                    let _ = loader.join();
                }

                let cleaned = cleaned.load(Ordering::Relaxed) as usize;
                let unavailable = unavailable.load(Ordering::Relaxed) as usize;
                let still_alive = still_alive.load(Ordering::Relaxed) as usize;
                let observe_fail = observe_fail.load(Ordering::Relaxed) as usize;
                let reasons = reasons.lock().unwrap().clone();
                let max_peak = max_peak.load(Ordering::Relaxed) as usize;
                let max_loadavg = *max_loadavg.lock().unwrap();
                let cgroup_status = cgroup_status.lock().unwrap().clone();
                let total_rounds = rounds * workers;

                let rate = unavailable as f64 / total_rounds as f64;
                println!(
                    "[repro][env] cpus={cpus} workers={workers} rounds_per_worker={rounds} total_rounds={total_rounds} load_threads={load_threads} children={children} cgroup={cgroup_status} container={} spawns={} scans={}",
                    container_kind(),
                    spawns.load(Ordering::Relaxed),
                    scans.load(Ordering::Relaxed)
                );
                println!(
                    "[repro][row] workers={workers} load_threads={load_threads} rounds={total_rounds} cleaned={cleaned} unavailable={unavailable} still_alive={still_alive} observe_fail={observe_fail} peak_obs={max_peak} max_loadavg={max_loadavg:.2} rate={rate:.4}"
                );
                if reasons.is_empty() {
                    println!("[repro][reasons] (无)");
                } else {
                    for (reason, count) in &reasons {
                        println!("[repro][reasons] {reason} x{count}");
                    }
                }
                let procfs: usize = reasons
                    .iter()
                    .filter(|(key, _)| key.starts_with("procfs:"))
                    .map(|(_, count)| count)
                    .sum();
                let cgroup: usize = reasons
                    .iter()
                    .filter(|(key, _)| key.starts_with("cgroup:"))
                    .map(|(_, count)| count)
                    .sum();
                println!(
                    "[repro][side] procfs={procfs} cgroup={cgroup} unavailable_total={unavailable}"
                );
                println!(
                    "[repro][verdict] {}",
                    if unavailable == 0 {
                        "本轮未复现 Unavailable"
                    } else if cgroup > procfs {
                        "复现样本偏向 cgroup 观测侧"
                    } else if procfs > cgroup {
                        "复现样本偏向 /proc 侧"
                    } else {
                        "复现样本两侧都有，需看 reasons 明细"
                    }
                );
            }

            // v1.50 根因定向：用确定性注入把「/proc 侧退出竞态」与「cgroup 侧观测失败」分开。
            // 读失败且进程已退出（等价于 readdir 与 read 之间进程消失）→ 跳过，不让整表不可用。
            #[cfg(target_os = "linux")]
            #[test]
            fn proc_read_failure_after_process_exit_is_skipped_not_unavailable() {
                let pid = std::process::id();
                let mut last_trace = Vec::new();
                for _ in 0..3 {
                    proc_fault::set(pid, true);
                    let _ = trace_take();
                    let table = proc_table();
                    let trace = trace_take();
                    if table.is_some()
                        && trace
                            .iter()
                            .any(|line| line.contains("proc_stat_read_injected"))
                    {
                        return;
                    }
                    last_trace = trace;
                }
                panic!("读失败且进程已退出时必须跳过、整表仍可用；trace={last_trace:?}");
            }

            // 存活进程的读失败 → 整表不可观测：安全侧不变量，绝不为降抖动而放宽。
            #[cfg(target_os = "linux")]
            #[test]
            fn proc_read_failure_for_live_process_is_unavailable() {
                let pid = std::process::id();
                let mut last_trace = Vec::new();
                for _ in 0..3 {
                    proc_fault::set(pid, false);
                    let _ = trace_take();
                    let table = proc_table();
                    let trace = trace_take();
                    if trace.iter().any(|line| {
                        line.contains("proc_table_unreadable") && line.contains("injected=true")
                    }) {
                        assert!(table.is_none(), "存活进程读失败必须判为不可观测");
                        return;
                    }
                    last_trace = trace;
                }
                panic!("注入未生效，无法证明安全侧不变量：{last_trace:?}");
            }

            // 边界负例：逃逸在根存活时能被快照标出；根退出后被收养，快照就枚举不到它
            #[cfg(target_os = "linux")]
            #[test]
            fn setsid_escape_is_flagged_then_unobservable_after_reparent() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-setsid-snapshot-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let pid_file = dir.join("pid.txt");
                // 根进程 wait 住，保证逃逸后代在被观察时仍有父链
                let script = format!(
                    "if command -v setsid >/dev/null 2>&1; then setsid sleep 30 & echo $! > '{}'; wait; else echo skip > '{}'; fi",
                    pid_file.display(),
                    pid_file.display()
                );
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg(script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn setsid fixture");
                supervisor.attach(&mut child);
                let root = child.id();

                let mut content = None;
                for _ in 0..100 {
                    if let Ok(text) = std::fs::read_to_string(&pid_file) {
                        let trimmed = text.trim();
                        // 重定向会先创建空文件，`echo $!` 稍后才写入；读到空内容不算就绪。
                        if !trimmed.is_empty() {
                            content = Some(trimmed.to_string());
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let Some(content) = content else {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };
                if content == "skip" {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                }
                let escaped: i32 = content.parse().expect("setsid 子进程 PID");

                let mut flagged = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    if snapshot.unobservable.is_none()
                        && snapshot
                            .outside_controlled_set()
                            .contains(&(escaped as u32))
                    {
                        flagged = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = flagged.expect("根存活期间，逃逸后代必须能被父链快照标出");
                println!(
                    "[evidence] setsid_snapshot before escaped={escaped} descendants={:?} outside={:?}",
                    before.descendants,
                    before.outside_controlled_set()
                );
                assert!(
                    before.outside_controlled_set().contains(&(escaped as u32)),
                    "逃逸后代在父链仍存在时必须被标为受控集合外"
                );

                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                let kill_supported = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.kill_supported())
                    .unwrap_or(false);
                println!(
                    "[evidence] setsid_snapshot after outcome={outcome:?} escaped_alive={} kill_supported={kill_supported} scope={:?} escaped_scope_unobservable={:?} descendants={:?}",
                    pid_exists(escaped),
                    after.scope,
                    after.escaped_scope_unobservable,
                    after.descendants
                );
                assert!(after.unobservable.is_none());
                if after.covers_escaped_descendants() {
                    if kill_supported {
                        // cgroup.kill 可用：作用域终止覆盖逃逸后代，只有清空后才允许 Cleaned
                        assert!(!pid_exists(escaped), "cgroup.kill 可用时逃逸后代必须被终止");
                        assert_eq!(
                            outcome,
                            CleanupOutcome::Cleaned,
                            "作用域内已无存活成员时才允许报告 Cleaned"
                        );
                        assert!(after.is_empty(), "终止后不得再枚举到后代");
                    } else {
                        // 无 cgroup.kill：逃逸后代存活，必须如实报 StillAlive 且仍可枚举
                        assert!(
                            pid_exists(escaped),
                            "无 cgroup.kill 时逃逸后代必然存活（窄判据）"
                        );
                        assert_eq!(
                            outcome,
                            CleanupOutcome::StillAlive,
                            "作用域内仍有存活进程时不得报告 Cleaned"
                        );
                        assert!(
                            after
                                .descendants
                                .iter()
                                .any(|entry| entry.pid == escaped as u32),
                            "被收养的逃逸后代必须出现在 cgroup 快照里"
                        );
                        assert!(!after.is_empty(), "快照非空，不能读成没有后代");
                    }
                } else {
                    // 作用域降级：必须写明原因，且空快照只能读成"本作用域内没有"
                    assert!(
                        pid_exists(escaped),
                        "setsid 逃逸不在进程组内，窄判据下必然存活"
                    );
                    assert_eq!(
                        outcome,
                        CleanupOutcome::Cleaned,
                        "进程组判据下逃逸不可见，仍报窄口径 Cleaned"
                    );
                    assert!(
                        after.escaped_scope_unobservable.is_some(),
                        "作用域降级必须带原因，不能静默"
                    );
                    assert!(after.is_empty());
                    assert!(
                        !after
                            .descendants
                            .iter()
                            .any(|entry| entry.pid == escaped as u32),
                        "降级时父链已断开，逃逸后代不在快照内"
                    );
                }
                unsafe {
                    libc::kill(escaped, libc::SIGKILL);
                }
                let _ = std::fs::remove_dir_all(&dir);
            }

            // cgroup v2 作用域：换父被收养的后代仍在成员集合里，必须被枚举，且不得误报 Cleaned
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_scope_enumerates_adopted_escapee() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-snapshot-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let pid_file = dir.join("pid.txt");
                let script = format!(
                    "if command -v setsid >/dev/null 2>&1; then setsid sleep 30 & echo $! > '{}'; wait; else echo skip > '{}'; fi",
                    pid_file.display(),
                    pid_file.display()
                );
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg(script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn setsid fixture");
                supervisor.attach(&mut child);
                let root = child.id();

                // 前置：本机必须真的建起了 cgroup 作用域；建不起来就如实失败，不假装覆盖
                let opened = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] cgroup_scope opened root={root} scope={:?} escaped_scope_unobservable={:?} controlled={:?}",
                    opened.scope, opened.escaped_scope_unobservable, opened.controlled_pids
                );
                assert_eq!(
                    opened.scope,
                    SnapshotScope::CgroupV2,
                    "本机未能建立 cgroup v2 观测作用域：{:?}",
                    opened.escaped_scope_unobservable
                );
                assert!(opened.covers_escaped_descendants());

                let mut content = None;
                for _ in 0..100 {
                    if let Ok(text) = std::fs::read_to_string(&pid_file) {
                        let text = text.trim().to_string();
                        if text != "skip" {
                            content = Some(text);
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let Some(content) = content else {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };
                let escaped: u32 = content.parse().expect("setsid 子进程 PID");

                let mut before = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    // 要等 setsid 真正切到新会话：只出现还不够，必须被标成终止作用域外
                    if snapshot.outside_controlled_set().contains(&escaped) {
                        before = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = before.expect("cgroup 作用域必须能枚举逃逸后代");
                println!(
                    "[evidence] cgroup_scope before escaped={escaped} scope={:?} descendants={:?} outside={:?}",
                    before.scope,
                    before.descendants,
                    before.outside_controlled_set()
                );
                assert!(
                    before.outside_controlled_set().contains(&escaped),
                    "setsid 逃逸不在进程组内，必须被标为终止作用域外"
                );

                // 第一步：只杀进程组（v1.36 及以前的终止作用域），让根退出、逃逸后代被收养。
                // 单独保留这一步，是为了证明"换父后仍可枚举"不依赖新的终止机制。
                unsafe {
                    libc::kill(-(root as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                let adopted = supervisor.snapshot_descendants(root);
                let adopted_members = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.members());
                println!(
                    "[evidence] cgroup_scope adopted escaped_alive={} escaped_ppid={:?} root={root} members={adopted_members:?} descendants={:?} controlled={:?} scope={:?}",
                    pid_exists(escaped as i32),
                    proc_ppid(escaped),
                    adopted.descendants,
                    adopted.controlled_pids,
                    adopted.scope
                );
                assert!(
                    pid_exists(escaped as i32),
                    "setsid 逃逸不在进程组内，整组终止后必须仍存活（窄判据）"
                );
                assert_ne!(
                    proc_ppid(escaped),
                    Some(root),
                    "根退出后逃逸后代应被收养，父链不再指向根"
                );
                assert_eq!(adopted.scope, SnapshotScope::CgroupV2);
                assert!(
                    adopted.descendants.iter().any(|entry| entry.pid == escaped),
                    "被收养的逃逸后代必须仍被 cgroup 作用域枚举到"
                );
                // 权威性：枚举来源是 `cgroup.procs` 成员集合本身，不依赖父链。
                // 此刻父链已断（ppid 不再是根），成员集合仍点名该 PID。
                assert_eq!(
                    adopted_members
                        .expect("终止前作用域仍应存在")
                        .expect("cgroup.procs 应可读"),
                    vec![escaped],
                    "被收养的逃逸后代必须仍在 cgroup.procs 成员集合里"
                );
                assert_eq!(adopted.controlled_pids, Some(Vec::new()), "进程组已清空");

                // 第二步：完整 terminate。cgroup.kill 可用时作用域必须清空，否则如实 StillAlive。
                let kill_supported = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.kill_supported())
                    .unwrap_or(false);
                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] cgroup_scope after outcome={outcome:?} escaped_alive={} kill_supported={kill_supported} descendants={:?} controlled={:?} scope={:?}",
                    pid_exists(escaped as i32),
                    after.descendants,
                    after.controlled_pids,
                    after.scope
                );
                if kill_supported {
                    assert!(
                        !pid_exists(escaped as i32),
                        "cgroup.kill 可用时逃逸后代必须被终止"
                    );
                    assert_eq!(
                        outcome,
                        CleanupOutcome::Cleaned,
                        "作用域内已无存活成员时才允许报告 Cleaned"
                    );
                    assert!(after.is_empty(), "终止后不得再枚举到后代");
                } else {
                    assert!(
                        pid_exists(escaped as i32),
                        "无 cgroup.kill 时逃逸后代必然存活"
                    );
                    assert_eq!(
                        outcome,
                        CleanupOutcome::StillAlive,
                        "作用域内仍有存活进程时不得报告 Cleaned"
                    );
                    assert!(
                        after.descendants.iter().any(|entry| entry.pid == escaped),
                        "无 cgroup.kill 时被收养的逃逸后代仍应可枚举"
                    );
                }
                assert_eq!(after.controlled_pids, Some(Vec::new()), "进程组已清空");

                unsafe {
                    libc::kill(escaped as i32, libc::SIGKILL);
                }
                let _ = std::fs::remove_dir_all(&dir);
            }

            // v1.37 判据：终止必须覆盖整个 cgroup 作用域。
            // 可观测量（全部成立才允许 Cleaned）：(a) cgroup.events `populated 0`、
            // (b) cgroup.procs 为空、(c) 逃逸后代 PID 不存在、(d) 快照无可枚举后代。
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_kill_terminates_adopted_escapee() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-kill-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let pid_file = dir.join("pid.txt");
                let script = format!(
                    "if command -v setsid >/dev/null 2>&1; then setsid sleep 30 & echo $! > '{}'; wait; else echo skip > '{}'; fi",
                    pid_file.display(),
                    pid_file.display()
                );
                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg(script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn setsid fixture");
                supervisor.attach(&mut child);
                let root = child.id();

                // 前置：必须真的建起 cgroup 作用域，且内核提供 cgroup.kill；否则如实失败
                let opened = supervisor.snapshot_descendants(root);
                let supported = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.kill_supported())
                    .unwrap_or(false);
                println!(
                    "[evidence] cgroup_kill opened root={root} scope={:?} escaped_scope_unobservable={:?} cgroup_kill_supported={supported}",
                    opened.scope, opened.escaped_scope_unobservable
                );
                assert_eq!(
                    opened.scope,
                    SnapshotScope::CgroupV2,
                    "本机未能建立 cgroup v2 作用域：{:?}",
                    opened.escaped_scope_unobservable
                );
                assert!(
                    supported,
                    "本机内核未提供 cgroup.kill（需 cgroup v2 非 root 作用域 + 内核 >= 5.14）"
                );

                let mut content = None;
                for _ in 0..100 {
                    if let Ok(text) = std::fs::read_to_string(&pid_file) {
                        let text = text.trim().to_string();
                        if text != "skip" {
                            content = Some(text);
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let Some(content) = content else {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };
                let escaped: u32 = content.parse().expect("setsid 子进程 PID");

                let mut before = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    if snapshot.outside_controlled_set().contains(&escaped) {
                        before = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = before.expect("cgroup 作用域必须能枚举逃逸后代");
                println!(
                    "[evidence] cgroup_kill before escaped={escaped} scope={:?} descendants={:?}",
                    before.scope, before.descendants
                );

                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                let scope = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .expect("终止后作用域仍应存在");
                let members = scope.members();
                let populated = scope.populated();
                println!(
                    "[evidence] cgroup_kill after outcome={outcome:?} escaped_alive={} members={:?} populated={:?} descendants={:?} unobservable={:?}",
                    pid_exists(escaped as i32),
                    members,
                    populated,
                    after.descendants,
                    after.unobservable
                );

                assert!(
                    !pid_exists(escaped as i32),
                    "逃逸后代必须在作用域终止后不再存活"
                );
                assert_eq!(
                    members.as_deref(),
                    Ok(&[][..]),
                    "cgroup.procs 必须为空：{members:?}"
                );
                assert_eq!(populated, Ok(0), "cgroup.events 必须 populated 0");
                assert_eq!(
                    outcome,
                    CleanupOutcome::Cleaned,
                    "作用域内已无存活成员时才允许报告 Cleaned"
                );
                assert!(after.unobservable.is_none(), "终止后快照必须可观测");
                assert!(after.descendants.is_empty(), "终止后不得再枚举到后代");
                assert_eq!(after.controlled_pids, Some(Vec::new()));

                unsafe {
                    libc::kill(escaped as i32, libc::SIGKILL);
                }
                let _ = std::fs::remove_dir_all(&dir);
            }

            // v1.45 判据：后代可以主动把自己写进**别的** cgroup，从而离开本作用域的
            // `cgroup.procs`。这样的进程既不在 `cgroup.kill` 的打击范围，`setsid` 后也不在
            // 进程组内；根退出被收养后父链也断了。终止仍必须覆盖它，否则就是真缺陷——
            // 「逃逸进程存活 + 误报 Cleaned」。
            //
            // 观测量（全部成立才允许 Cleaned）：
            // (a) `cgroup.procs` 空、(b) `populated 0`、(c) 逃逸 PID 不存在、(d) 快照无后代。
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_kill_terminates_escapee_that_left_cgroup_procs() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-left-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let grand_pid_file = dir.join("grand.pid");
                let child_pid_file = dir.join("child.pid");
                let move_status = dir.join("move.status");
                let grand_script = dir.join("grandchild.sh");
                let child_script = dir.join("child.sh");
                let root_script = dir.join("root.sh");

                let mut supervisor = Supervisor::new();
                let mut command = Command::new("sh");
                command
                    .arg(&root_script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);

                // 前置：本机必须真的建起 cgroup v2 作用域并支持 `cgroup.kill`；
                // 建不起来就如实失败，不把"没覆盖"写成"已证"。
                let scope_dir = {
                    let scope = supervisor
                        .cgroup
                        .scope
                        .as_ref()
                        .expect("cgroup 作用域未建立");
                    scope.dir.clone()
                };
                let base = scope_dir.parent().expect("作用域父目录").to_path_buf();
                let kill_supported = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.kill_supported())
                    .unwrap_or(false);
                assert!(
                    kill_supported,
                    "本机内核未提供 cgroup.kill（需 cgroup v2 非 root 作用域 + 内核 >= 5.14）"
                );

                // 旁路 cgroup：孙进程主动把自己写进去，等于离开本作用域的 cgroup.procs。
                // 建在同一个父目录下，委派权限与作用域本身一致。
                let sink = base.join(format!(
                    "aw-escape-sink-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir(&sink).expect("创建旁路 cgroup");
                let sink_procs = sink.join("cgroup.procs");

                std::fs::write(
                    &grand_script,
                    format!(
                        "#!/bin/sh\necho \"$$\" > '{}'\nif echo \"$$\" > '{}'; then echo moved > '{}'; else echo failed > '{}'; fi\nexec sleep 30\n",
                        grand_pid_file.display(),
                        sink_procs.display(),
                        move_status.display(),
                        move_status.display()
                    ),
                )
                .expect("写孙进程脚本");
                std::fs::write(
                    &child_script,
                    format!(
                        "#!/bin/sh\nif command -v setsid >/dev/null 2>&1; then\n  setsid sh '{}' &\n  wait\nelse\n  echo skip > '{}'\nfi\n",
                        grand_script.display(),
                        grand_pid_file.display()
                    ),
                )
                .expect("写子进程脚本");
                std::fs::write(
                    &root_script,
                    format!(
                        "#!/bin/sh\nsh '{}' &\necho $! > '{}'\nwait\n",
                        child_script.display(),
                        child_pid_file.display()
                    ),
                )
                .expect("写根进程脚本");

                let mut child = command.spawn().expect("spawn 三层进程树");
                supervisor.attach(&mut child);
                let root = child.id();

                let mut content = None;
                for _ in 0..150 {
                    if let Ok(text) = std::fs::read_to_string(&grand_pid_file) {
                        let text = text.trim().to_string();
                        if text == "skip" {
                            break;
                        }
                        if let Ok(pid) = text.parse::<u32>() {
                            content = Some(pid);
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let Some(escaped) = content else {
                    eprintln!(
                        "[evidence] cgroup_left skipped：本机无 setsid 或孙进程未写出 PID（不构成裁决依据）"
                    );
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&sink);
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };

                // 等孙进程真正离开本作用域：既不在 scope.members，也仍被父链枚举到。
                let mut before = None;
                let mut sink_members = Vec::new();
                for _ in 0..200 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    let in_scope = supervisor
                        .cgroup
                        .scope
                        .as_ref()
                        .map(|scope| scope.contains(escaped))
                        .unwrap_or(false);
                    sink_members = std::fs::read_to_string(&sink_procs)
                        .map(|text| {
                            text.lines()
                                .filter_map(|line| line.trim().parse::<u32>().ok())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if !in_scope
                        && sink_members.contains(&escaped)
                        && snapshot
                            .descendants
                            .iter()
                            .any(|entry| entry.pid == escaped)
                    {
                        before = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = before.expect(
                    "孙进程必须离开本作用域 cgroup.procs（并进入旁路 cgroup）且仍可由父链枚举",
                );
                let move_status_text = std::fs::read_to_string(&move_status).ok();
                println!(
                    "[evidence] cgroup_left before root={root} child={:?} escaped={escaped} scope={:?} sink_members={sink_members:?} move_status={move_status_text:?} descendants={:?} outside={:?}",
                    std::fs::read_to_string(&child_pid_file)
                        .ok()
                        .map(|text| text.trim().to_string()),
                    before.scope,
                    before.descendants,
                    before.outside_controlled_set()
                );
                assert!(
                    before.outside_controlled_set().contains(&escaped),
                    "逃出 cgroup.procs 的孙进程必须被标为终止作用域外"
                );
                assert_eq!(
                    move_status_text.as_deref().map(str::trim),
                    Some("moved"),
                    "孙进程必须成功把自己写进旁路 cgroup"
                );

                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                let scope = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .expect("终止后作用域仍应存在");
                let members = scope.members();
                let populated = scope.populated();
                println!(
                    "[evidence] cgroup_left after outcome={outcome:?} escaped_alive={} members={members:?} populated={populated:?} descendants={:?} unobservable={:?}",
                    pid_exists(escaped as i32),
                    after.descendants,
                    after.unobservable
                );

                assert!(
                    !pid_exists(escaped as i32),
                    "逃出 cgroup.procs 的后代在终止后必须不再存活（cgroup.kill 打不到它，必须另行收口）"
                );
                assert_eq!(
                    members.as_deref(),
                    Ok(&[][..]),
                    "cgroup.procs 必须为空：{members:?}"
                );
                assert_eq!(populated, Ok(0), "cgroup.events 必须 populated 0");
                assert_eq!(
                    outcome,
                    CleanupOutcome::Cleaned,
                    "四个观测量同时成立时才允许报告 Cleaned"
                );
                assert!(after.unobservable.is_none(), "终止后快照必须可观测");
                assert!(after.descendants.is_empty(), "终止后不得再枚举到后代");

                unsafe {
                    libc::kill(escaped as i32, libc::SIGKILL);
                }
                let _ = std::fs::remove_dir_all(&sink);
                let _ = std::fs::remove_dir_all(&dir);
            }

            // 降级必须如实：cgroup v1 / 无权限时记原因，退回进程组观测且不得当成"没有后代"
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_unavailable_degrades_with_reason() {
                let base = std::env::temp_dir().join(format!(
                    "aw-cgroup-v1-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&base).expect("创建临时目录");
                // 没有 cgroup.controllers：模拟 cgroup v1 / 未挂载 v2
                let reason = CgroupScope::prepare_from(&base).err();
                println!("[evidence] cgroup_unavailable reason={reason:?}");
                assert_eq!(reason.as_deref(), Some("cgroup_v2_unavailable"));
                let _ = std::fs::remove_dir_all(&base);

                let mut supervisor = Supervisor::new();
                supervisor.pgid = Some(1);
                supervisor.cgroup.status = CgroupStatus::Degraded("cgroup_v2_unavailable".into());
                let snapshot = supervisor.snapshot_descendants(std::process::id());
                println!(
                    "[evidence] cgroup_degraded scope={:?} escaped_scope_unobservable={:?} unobservable={:?}",
                    snapshot.scope, snapshot.escaped_scope_unobservable, snapshot.unobservable
                );
                assert_eq!(snapshot.scope, SnapshotScope::ProcessGroup);
                assert_eq!(
                    snapshot.escaped_scope_unobservable.as_deref(),
                    Some("cgroup_v2_unavailable"),
                    "降级必须带原因"
                );
                assert!(snapshot.unobservable.is_none(), "降级不等于整份快照不可用");
                assert!(
                    !snapshot.covers_escaped_descendants(),
                    "降级后空快照不得读成整树已清"
                );
            }

            // v1.45 反例：作用域存在但 `cgroup.procs` 读不出（权限/卸载/竞态）时，
            // 快照必须整份标记不可观测，绝不能退化成「没有后代」。
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_members_unreadable_is_not_empty() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-unreadable-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let scope = CgroupScope {
                    procs: dir.join("missing-cgroup.procs"),
                    dir: dir.clone(),
                    writer: std::cell::RefCell::new(None),
                    owner_record: None,
                };
                let me = std::process::id();
                let mut supervisor = Supervisor::new();
                supervisor.pgid = i32::try_from(me).ok();
                supervisor.root_start = proc_start_time(me);
                supervisor.cgroup.scope = Some(scope);
                supervisor.cgroup.status = CgroupStatus::Active;
                let snapshot = supervisor.snapshot_descendants(me);
                println!(
                    "[evidence] cgroup_unreadable scope={:?} unobservable={:?} is_empty={}",
                    snapshot.scope,
                    snapshot.unobservable,
                    snapshot.is_empty()
                );
                assert!(
                    snapshot.unobservable.is_some(),
                    "cgroup.procs 读不出时必须整份标记不可观测"
                );
                assert!(!snapshot.is_empty(), "不可观测绝不能读成空集合");
                assert!(
                    snapshot
                        .unobservable
                        .as_deref()
                        .unwrap_or_default()
                        .starts_with("cgroup_procs_unreadable:"),
                    "原因码必须点名读失败：{:?}",
                    snapshot.unobservable
                );
                let _ = std::fs::remove_dir_all(&dir);
            }

            // v1.41/v1.45 判据：内核没有 `cgroup.kill` 时，终止不得因为「自己杀掉了名单里的
            // 进程」就报告 Cleaned。用临时目录伪造一个「记账永不更新」的作用域（`cgroup.procs`
            // 非空、`populated 1`、无 `cgroup.kill`），配合真实 setsid 逃逸进程：
            // v1.45 起 terminate 会按终止前名单逐个 SIGKILL，进程确实会死；但独立观测点
            // （`cgroup.procs` / `populated`）仍显示有成员 → 必须 StillAlive。
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_without_kill_reports_still_alive() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-no-kill-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let Some((mut holder, escaped)) = setsid_escapee(&dir) else {
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };
                // 伪造作用域：成员只有逃逸进程；本目录没有 cgroup.kill，覆盖 no-kill 分支
                std::fs::write(dir.join("cgroup.procs"), format!("{escaped}\n"))
                    .expect("写 cgroup.procs");
                std::fs::write(dir.join("cgroup.events"), "populated 1\n")
                    .expect("写 cgroup.events");
                let scope = CgroupScope {
                    procs: dir.join("cgroup.procs"),
                    dir: dir.clone(),
                    writer: std::cell::RefCell::new(None),
                    owner_record: None,
                };

                let mut command = Command::new("sleep");
                command
                    .arg("30")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0);
                let mut child = command.spawn().expect("spawn controlled root");
                let root = child.id();
                let mut supervisor = Supervisor::new();
                supervisor.pgid = i32::try_from(root).ok();
                supervisor.root_start = proc_start_time(root);
                supervisor.cgroup.scope = Some(scope);
                supervisor.cgroup.status = CgroupStatus::Active;

                let kill_supported = supervisor
                    .cgroup
                    .scope
                    .as_ref()
                    .map(|scope| scope.kill_supported())
                    .unwrap_or(false);
                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] cgroup_no_kill root={root} escaped={escaped} kill_supported={kill_supported} outcome={outcome:?} escaped_alive={} members={:?} populated={:?} descendants={:?}",
                    pid_exists(escaped as i32),
                    supervisor
                        .cgroup
                        .scope
                        .as_ref()
                        .map(|scope| scope.members()),
                    supervisor
                        .cgroup
                        .scope
                        .as_ref()
                        .map(|scope| scope.populated()),
                    after.descendants
                );
                assert!(
                    !kill_supported,
                    "临时目录必须没有 cgroup.kill，才覆盖无 kill 分支"
                );
                assert!(
                    !pid_exists(escaped as i32),
                    "v1.45：无 cgroup.kill 时，终止前名单里的逃逸成员仍会被逐个 SIGKILL 收口"
                );
                assert_eq!(
                    outcome,
                    CleanupOutcome::StillAlive,
                    "独立观测点仍显示有成员时不得报告 Cleaned（不因自己杀掉了名单就放行）"
                );
                assert!(
                    after.descendants.iter().any(|entry| entry.pid == escaped),
                    "无 cgroup.kill 时仍应枚举到作用域成员"
                );

                unsafe {
                    libc::kill(escaped as i32, libc::SIGKILL);
                }
                let _ = holder.kill();
                let _ = holder.wait();
                drop(supervisor);
                let _ = std::fs::remove_dir_all(&dir);
            }

            // v1.52 判据：作用域建不起来（降级）时**不动进程组**，只向直接子进程发送终止信号，
            // 被收养的逃逸进程既打不到也看不见——按 Unavailable 上报（cleanup_claimed=false），
            // 降级原因始终带在快照里。宁可少清理，也不盲杀不可观测的作用域。
            #[cfg(target_os = "linux")]
            #[test]
            fn cgroup_degraded_terminate_reports_unavailable() {
                let dir = std::env::temp_dir().join(format!(
                    "aw-cgroup-degraded-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                std::fs::create_dir_all(&dir).expect("创建临时目录");
                let Some((mut holder, escaped)) = setsid_escapee(&dir) else {
                    let _ = std::fs::remove_dir_all(&dir);
                    return;
                };

                let mut command = Command::new("sleep");
                command
                    .arg("30")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0);
                let mut child = command.spawn().expect("spawn controlled root");
                let root = child.id();
                let mut supervisor = Supervisor::new();
                supervisor.cgroup.status = CgroupStatus::Degraded("cgroup_v2_unavailable".into());
                supervisor.attach(&mut child);

                let before = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] cgroup_degraded before root={root} escaped={escaped} scope={:?} escaped_scope_unobservable={:?} descendants={:?}",
                    before.scope, before.escaped_scope_unobservable, before.descendants
                );
                assert_eq!(
                    before.scope,
                    SnapshotScope::ProcessGroup,
                    "降级只能是进程组观测"
                );
                assert_eq!(
                    before.escaped_scope_unobservable.as_deref(),
                    Some("cgroup_v2_unavailable"),
                    "降级必须带原因"
                );
                assert!(before.unobservable.is_none(), "降级不等于整份快照不可用");
                assert!(
                    !before.covers_escaped_descendants(),
                    "降级后空快照不得读成整树已清"
                );

                let outcome = supervisor.terminate(&mut child);
                let after = supervisor.snapshot_descendants(root);
                println!(
                    "[evidence] cgroup_degraded after outcome={outcome:?} escaped_alive={} descendants={:?} scope={:?} unobservable={:?} escaped_scope_unobservable={:?}",
                    pid_exists(escaped as i32),
                    after.descendants,
                    after.scope,
                    after.unobservable,
                    after.escaped_scope_unobservable
                );
                assert!(
                    pid_exists(escaped as i32),
                    "降级判据下 setsid 逃逸必然存活（进程组打不到）"
                );
                assert_eq!(
                    outcome,
                    CleanupOutcome::Unavailable,
                    "降级时不得宣称 Cleaned：作用域看不见就不能给 PM cleanup_claimed，按 Unavailable 上报"
                );
                assert!(after.is_empty(), "父链断开的逃逸进程不在降级快照里");
                assert_eq!(
                    after.escaped_scope_unobservable.as_deref(),
                    Some("cgroup_v2_unavailable"),
                    "降级原因必须一直带在快照里，不能静默"
                );

                unsafe {
                    libc::kill(escaped as i32, libc::SIGKILL);
                }
                let _ = holder.kill();
                let _ = holder.wait();
                let _ = std::fs::remove_dir_all(&dir);
            }

            // v1.38 判据：POSIX 上 rename 活跃 attempt 目录的语义必须单独复核。
            // 观测量（全部成立才允许宣称已复核）：
            // (a) 进程 cwd 指向该目录、且持有目录内文件 fd 时，rename 仍成功；
            // (b) /proc/<pid>/cwd 随之指向 trash 新路径；
            // (c) 已打开的 fd 仍读到原内容；
            // (d) 旧路径不存在、新路径存在；
            // (e) 随后对其 remove_dir_all 成功，进程仍存活且 fd 内容不变。
            #[cfg(target_os = "linux")]
            #[test]
            fn posix_rename_active_attempt_semantics() {
                let root = std::env::temp_dir().join(format!(
                    "aw-posix-rename-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                let attempt = root.join("t-task").join("attempt-1");
                let staged = root
                    .join(".prune-trash")
                    .join("b-1")
                    .join("t-task-attempt-1");
                std::fs::create_dir_all(&attempt).expect("创建 attempt 目录");
                std::fs::create_dir_all(staged.parent().expect("暂存父目录"))
                    .expect("创建暂存目录");
                std::fs::write(attempt.join("keep.txt"), b"original").expect("写入文件");
                let ready = root.join("ready.txt");
                let script = format!(
                    "cd '{}' && exec 3<keep.txt && echo ready > '{}' && sleep 30; :",
                    attempt.display(),
                    ready.display()
                );
                let mut child = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn holder");
                for _ in 0..100 {
                    if ready.exists() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                assert!(ready.exists(), "持有 cwd/fd 的进程未就绪");

                let pid = child.id();
                let cwd_before = std::fs::read_link(format!("/proc/{pid}/cwd")).expect("cwd");
                let fd_before = std::fs::read_link(format!("/proc/{pid}/fd/3")).expect("fd3");
                let canonical_attempt = std::fs::canonicalize(&attempt).expect("canonical attempt");
                let canonical_keep =
                    std::fs::canonicalize(attempt.join("keep.txt")).expect("canonical keep.txt");
                println!("[evidence] posix_rename before cwd={cwd_before:?} fd3={fd_before:?}");
                assert_eq!(
                    cwd_before, canonical_attempt,
                    "前置：cwd 必须指向 attempt 目录"
                );
                assert_eq!(fd_before, canonical_keep, "前置：fd 3 必须指向该目录内文件");

                let renamed = std::fs::rename(&attempt, &staged);
                println!(
                    "[evidence] posix_rename rename result={renamed:?} old_exists={} new_exists={}",
                    attempt.exists(),
                    staged.exists()
                );
                renamed.expect("POSIX 上 rename 被活跃进程占用的目录必须成功");

                let cwd_after = std::fs::read_link(format!("/proc/{pid}/cwd")).expect("cwd after");
                let fd_after = std::fs::read_link(format!("/proc/{pid}/fd/3")).expect("fd3 after");
                let content_after =
                    std::fs::read_to_string(format!("/proc/{pid}/fd/3")).expect("read fd3");
                println!(
                    "[evidence] posix_rename after cwd={cwd_after:?} fd3={fd_after:?} content={content_after:?} old_exists={} new_exists={}",
                    attempt.exists(),
                    staged.exists()
                );
                assert!(!attempt.exists(), "rename 后旧路径必须不存在");
                assert!(staged.exists(), "rename 后新路径必须存在");
                assert_eq!(
                    cwd_after,
                    std::fs::canonicalize(&staged).expect("canonical staged"),
                    "rename 后 cwd 必须落在 trash 新路径"
                );
                assert_eq!(
                    fd_after,
                    std::fs::canonicalize(staged.join("keep.txt")).expect("canonical staged keep"),
                    "rename 后 fd 必须落在新路径"
                );
                assert_eq!(content_after, "original", "已打开的 fd 必须仍读到原内容");

                let removed = std::fs::remove_dir_all(&staged);
                let alive_after_delete = child.try_wait().expect("try_wait").is_none();
                let content_deleted = std::fs::read_to_string(format!("/proc/{pid}/fd/3")).ok();
                println!(
                    "[evidence] posix_rename remove result={removed:?} alive={alive_after_delete} fd_content_after_delete={content_deleted:?}"
                );
                removed.expect("prune 的删除步骤在 POSIX 上对活跃目录必须成功");
                assert!(alive_after_delete, "目录被删除后持有 fd 的进程应仍存活");
                assert_eq!(
                    content_deleted.as_deref(),
                    Some("original"),
                    "已打开 fd 在 unlink 后仍应读到原内容"
                );

                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
                let _ = child.wait();
                let _ = std::fs::remove_dir_all(&root);
            }
        }
    }

    #[cfg(windows)]
    mod imp {
        use super::{CleanupOutcome, DescendantProcess, DescendantSnapshot, SnapshotScope};
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};
        use windows_sys::Win32::Foundation::{
            CloseHandle, ERROR_INVALID_PARAMETER, FALSE, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
            STILL_ACTIVE,
        };
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32First, Process32Next, Thread32First, Thread32Next,
            PROCESSENTRY32, TH32CS_SNAPPROCESS, TH32CS_SNAPTHREAD, THREADENTRY32,
        };
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectBasicProcessIdList, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, OpenThread, ResumeThread,
            CREATE_SUSPENDED, PROCESS_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
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
            /// 根进程创建时间（FILETIME 100ns 刻度）；用于父链边校验，拒绝把同号 PID
            /// 复用后的无关进程当成后代。
            root_created_at: Option<u64>,
            /// 根进程退出时间（FILETIME 100ns 刻度，终止时从子进程句柄读取）。
            /// 根 PID 被后来者复用后，后来者的子进程创建时间必然晚于本时间；
            /// 用它给「根 → 直接子进程」的边加上界，避免把并发进程树算成后代。
            root_exited_at: Option<u64>,
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
                            root_created_at: None,
                            root_exited_at: None,
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
                            root_created_at: None,
                            root_exited_at: None,
                        };
                    }
                    Self {
                        job: Some(job),
                        attached: false,
                        suspended: false,
                        attach_error: None,
                        root_created_at: None,
                        root_exited_at: None,
                    }
                }
            }

            pub(crate) fn configure(&mut self, command: &mut Command) {
                command.creation_flags(CREATE_SUSPENDED);
                self.suspended = true;
            }

            pub(crate) fn attach(&mut self, child: &mut Child) {
                self.root_created_at =
                    process_times(child.as_raw_handle() as HANDLE).map(|(created, _)| created);
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

            /// Windows 无 cgroup v2：启动回收不适用，归属记录也不写。
            pub(crate) fn publish_scope_owner(
                &mut self,
                _child: &Child,
                _runs_dir: &std::path::Path,
                _task_id: &str,
                _attempt: u64,
            ) -> Result<(), String> {
                Ok(())
            }

            /// 观测点：Toolhelp 进程树快照 × 作业 PID 列表交叉核对。
            ///
            /// 两个观测点相互独立：进程树按"创建时父链"递归（Windows 不重父化，父进程退出后
            /// 子进程仍记录原 `ParentProcessID`），作业列表给出受控集合成员。两者求交/求差
            /// 即可回答"枚举到的后代是否都在作业内"；任一查询失败都记 `unobservable`。
            pub(crate) fn snapshot_descendants(&self, root: u32) -> DescendantSnapshot {
                let Some(processes) = process_table() else {
                    return unobservable_snapshot(root, "process_snapshot_failed");
                };
                let descendants =
                    descendants_of(root, self.root_created_at, self.root_exited_at, &processes);
                let Some(job) = self.job else {
                    return unobservable_snapshot(root, "job_unavailable");
                };
                let Some(controlled) = job_pid_list(job) else {
                    return unobservable_snapshot(root, "job_pid_list_unavailable");
                };
                let entries = descendants
                    .into_iter()
                    .map(|pid| DescendantProcess {
                        pid,
                        in_controlled_set: controlled.contains(&pid),
                    })
                    .collect();
                DescendantSnapshot {
                    root,
                    descendants: entries,
                    controlled_pids: Some(controlled),
                    scope: SnapshotScope::JobObject,
                    escaped_scope_unobservable: None,
                    unobservable: None,
                }
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
                let root = child.id();
                let _ = child.wait();
                // 用子进程句柄读退出时间：按 PID 读会读到复用该 PID 的后来者。
                self.root_exited_at = process_times(child.as_raw_handle() as HANDLE)
                    .and_then(|(_, exited)| (exited != 0).then_some(exited));

                let deadline = Instant::now() + CLEANUP_DEADLINE;
                loop {
                    match active_processes(job) {
                        Some(0) => {
                            // 作业记账为 0 后再用独立观测点复核：仍有可枚举后代 → 不宣称清空
                            let snapshot = self.snapshot_descendants(root);
                            if snapshot.unobservable.is_some() {
                                return CleanupOutcome::Unavailable;
                            }
                            if !snapshot.is_empty() {
                                return CleanupOutcome::StillAlive;
                            }
                            return CleanupOutcome::Cleaned;
                        }
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

        fn unobservable_snapshot(root: u32, reason: &str) -> DescendantSnapshot {
            DescendantSnapshot {
                root,
                descendants: Vec::new(),
                controlled_pids: None,
                scope: SnapshotScope::JobObject,
                escaped_scope_unobservable: Some(reason.to_string()),
                unobservable: Some(reason.to_string()),
            }
        }

        /// 一次 Toolhelp 进程快照读出的 `(pid, parent_pid, created_at, alive)` 表。
        /// 创建时间取 `GetProcessTimes` 的 FILETIME；`alive` 取 `GetExitCodeProcess`。
        /// 任一项读不到时记 `None`（不放大误判）。
        /// 空表或快照句柄失败都按"不可观测"处理，绝不当成"没有后代"。
        struct ProcEntry {
            pid: u32,
            parent_pid: u32,
            created_at: Option<u64>,
            alive: Option<bool>,
        }

        fn process_table() -> Option<Vec<ProcEntry>> {
            unsafe {
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                if snapshot == INVALID_HANDLE_VALUE {
                    return None;
                }
                let mut entry: PROCESSENTRY32 = std::mem::zeroed();
                entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
                let mut rows = Vec::new();
                let mut ok = Process32First(snapshot, &mut entry);
                while ok != 0 {
                    let pid = entry.th32ProcessID;
                    let (created_at, alive) = process_facts(pid);
                    rows.push(ProcEntry {
                        pid,
                        parent_pid: entry.th32ParentProcessID,
                        created_at,
                        alive,
                    });
                    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
                    ok = Process32Next(snapshot, &mut entry);
                }
                CloseHandle(snapshot);
                if rows.is_empty() {
                    None
                } else {
                    Some(rows)
                }
            }
        }

        /// 一次 `OpenProcess` 读出创建时间与存活状态。
        ///
        /// `alive` 用 `GetExitCodeProcess` 判定：`STILL_ACTIVE` 才算活。已终止但尚未被回收的
        /// 进程仍会出现在 Toolhelp 快照里（句柄未全部关闭前进程对象不删除），如果不剔除，
        /// 终止后的快照会把刚被杀的成员当成"仍可枚举的后代"，误报 `StillAlive`。
        fn process_facts(pid: u32) -> (Option<u64>, Option<bool>) {
            unsafe {
                let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
                if handle.is_null() {
                    // `ERROR_INVALID_PARAMETER` = 该 PID 已不存在。Toolhelp 枚举与逐个
                    // 打开之间进程可能已退出并被回收，这种行必须按已终止处理；权限不足
                    // 等其它错误仍记 `None`（未知），交由调用方保守保留。
                    return (
                        None,
                        liveness_from_open_error(std::io::Error::last_os_error().raw_os_error()),
                    );
                }
                let created_at = process_times(handle).map(|(created, _)| created);
                let mut code: u32 = 0;
                let code_ok = GetExitCodeProcess(handle, &mut code);
                CloseHandle(handle);
                let alive = if code_ok != 0 {
                    Some(code == STILL_ACTIVE as u32)
                } else {
                    None
                };
                (created_at, alive)
            }
        }

        /// 把 `OpenProcess` 失败码映射成存活判断：只把「PID 不存在」判为已终止，
        /// 其余（尤其是权限不足）保持未知。
        fn liveness_from_open_error(code: Option<i32>) -> Option<bool> {
            match code {
                Some(code) if code as u32 == ERROR_INVALID_PARAMETER => Some(false),
                _ => None,
            }
        }

        /// 从已打开的进程句柄读取 `(创建时间, 退出时间)`（FILETIME 100ns 刻度）。
        /// 运行中的进程退出时间为 0；查询失败返回 `None`。
        ///
        /// 用句柄而不是 PID 读取：PID 被复用后按 PID 打开的是后来者，句柄始终指向
        /// 本进程对象。
        fn process_times(handle: HANDLE) -> Option<(u64, u64)> {
            unsafe {
                let mut creation: FILETIME = std::mem::zeroed();
                let mut exit: FILETIME = std::mem::zeroed();
                let mut kernel: FILETIME = std::mem::zeroed();
                let mut user: FILETIME = std::mem::zeroed();
                let ok = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
                if ok == 0 {
                    return None;
                }
                let ticks =
                    |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
                Some((ticks(creation), ticks(exit)))
            }
        }

        /// 按创建时父链做传递闭包（含环保护），返回排好序的后代 PID。
        ///
        /// 三道过滤：
        /// - 只接受「子进程创建时间 ≥ 父进程创建时间」的边。Windows 会复用 PID：旧父进程
        ///   退出后同号 PID 被新进程持有，旧子进程记录的 `ParentProcessID` 会把无关进程树
        ///   挂到新根上；创建时间顺序能识别并拒绝这类边。任一端时间读不到时保留旧行为
        ///   （接受该边），避免因缺证据扩大误判。
        /// - 剔除已终止的进程。Toolhelp 会列出已退出但进程对象尚未删除的成员，杀完立刻
        ///   拍快照会把它们当成存活后代。存活状态读不到（权限不足）时保留该行，宁可保守
        ///   报仍有后代，也不把无法鉴定的进程当成已清理。
        /// - 「根 → 直接子进程」的边还要求子进程创建时间不晚于根进程退出时间：根 PID 被
        ///   后来者复用后，后来者的子进程结构上与本任务无关，但创建时间晚于根，仅靠下界
        ///   无法拒绝；退出时间给出上界。根未退出或读不到退出时间时不加上界。
        fn descendants_of(
            root: u32,
            root_created_at: Option<u64>,
            root_exited_at: Option<u64>,
            rows: &[ProcEntry],
        ) -> Vec<u32> {
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            seen.insert(root);
            let mut frontier = vec![(root, root_created_at)];
            let mut descendants = Vec::new();
            while let Some((parent, parent_created_at)) = frontier.pop() {
                for row in rows {
                    if row.parent_pid != parent || seen.contains(&row.pid) {
                        continue;
                    }
                    if row.alive == Some(false) {
                        continue;
                    }
                    if !edge_respects_creation_order(parent_created_at, row.created_at) {
                        continue;
                    }
                    if parent == root
                        && !child_created_before_root_exit(root_exited_at, row.created_at)
                    {
                        continue;
                    }
                    seen.insert(row.pid);
                    descendants.push(row.pid);
                    frontier.push((row.pid, row.created_at));
                }
            }
            descendants.sort_unstable();
            descendants
        }

        fn child_created_before_root_exit(root_exited_at: Option<u64>, child: Option<u64>) -> bool {
            match (root_exited_at, child) {
                (Some(exited), Some(created)) => created <= exited,
                _ => true,
            }
        }

        fn edge_respects_creation_order(parent: Option<u64>, child: Option<u64>) -> bool {
            match (parent, child) {
                (Some(parent), Some(child)) => child >= parent,
                _ => true,
            }
        }

        /// `JobObjectBasicProcessIdList` 的缓冲区；固定容量，超过容量按截断（不可观测）处理。
        #[repr(C)]
        struct JobPidBuffer {
            assigned: u32,
            count: u32,
            pids: [usize; 512],
        }

        fn job_pid_list(job: HANDLE) -> Option<Vec<u32>> {
            unsafe {
                let mut buffer = JobPidBuffer {
                    assigned: 0,
                    count: 0,
                    pids: [0; 512],
                };
                let ok = QueryInformationJobObject(
                    job,
                    JobObjectBasicProcessIdList,
                    &mut buffer as *mut _ as *mut core::ffi::c_void,
                    std::mem::size_of::<JobPidBuffer>() as u32,
                    std::ptr::null_mut(),
                );
                if ok == 0 {
                    return None;
                }
                let count = buffer.count as usize;
                if count > buffer.pids.len() {
                    return None;
                }
                Some(buffer.pids[..count].iter().map(|pid| *pid as u32).collect())
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
                    return Err(format!("线程快照失败：{}", std::io::Error::last_os_error()));
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
                println!(
                    "[evidence] alive_but_idle attached={} ActiveProcesses_before={before:?}",
                    supervisor.attached
                );
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
                let started = Instant::now();
                let outcome = supervisor.terminate(&mut child);
                let elapsed_ms = started.elapsed().as_millis();
                let after = supervisor.active_processes();
                if outcome != CleanupOutcome::Cleaned {
                    println!(
                        "[probe] suspended_start root={} elapsed_ms={elapsed_ms} active_after={after:?} {}",
                        child.id(),
                        dump_parent_edges(child.id(), supervisor.root_created_at)
                    );
                }
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

            // v1.39 判据：同号 PID 复用后，旧父链留下的子进程创建时间早于新根，
            // 这条边必须被拒绝；只有创建时间不早于父进程的边才允许进入后代集合。
            // 时间读不到时保留旧行为（接受），避免因缺证据扩大误判。
            #[test]
            fn pid_reuse_parent_chain_edge_is_rejected() {
                let root = 500;
                let root_created = Some(5000);
                let rows = vec![
                    // 旧父进程（pid 500）的遗留子进程：创建时间早于新根，必须拒绝
                    ProcEntry {
                        pid: 900,
                        parent_pid: root,
                        created_at: Some(1000),
                        alive: Some(true),
                    },
                    ProcEntry {
                        pid: 901,
                        parent_pid: 900,
                        created_at: Some(1100),
                        alive: Some(true),
                    },
                    // 真正的新后代：创建时间晚于根，必须保留
                    ProcEntry {
                        pid: 902,
                        parent_pid: root,
                        created_at: Some(6000),
                        alive: Some(true),
                    },
                    ProcEntry {
                        pid: 903,
                        parent_pid: 902,
                        created_at: Some(7000),
                        alive: Some(true),
                    },
                    // 中间层 PID 复用：905 早于父进程 902，必须拒绝
                    ProcEntry {
                        pid: 905,
                        parent_pid: 902,
                        created_at: Some(4000),
                        alive: Some(true),
                    },
                    // 创建时间不可读：保留旧行为，接受该边
                    ProcEntry {
                        pid: 906,
                        parent_pid: root,
                        created_at: None,
                        alive: Some(true),
                    },
                    // 根退出后同号 PID 被并发进程复用：后来者的子进程 907 创建时间晚于
                    // 根退出时间（6500），必须拒绝
                    ProcEntry {
                        pid: 907,
                        parent_pid: root,
                        created_at: Some(8000),
                        alive: Some(true),
                    },
                ];
                println!(
                    "[evidence] pid_reuse rows={} result={:?}",
                    rows.len(),
                    descendants_of(root, root_created, Some(6500), &rows)
                );
                assert_eq!(
                    descendants_of(root, root_created, Some(6500), &rows),
                    vec![902, 903, 906]
                );
                assert!(!edge_respects_creation_order(Some(5000), Some(1000)));
                assert!(edge_respects_creation_order(Some(5000), Some(6000)));
                assert!(edge_respects_creation_order(None, Some(1)));
                assert!(!child_created_before_root_exit(Some(6500), Some(8000)));
                assert!(child_created_before_root_exit(Some(6500), Some(6000)));
                assert!(child_created_before_root_exit(None, Some(8000)));
                assert_eq!(liveness_from_open_error(Some(87)), Some(false));
                assert_eq!(liveness_from_open_error(Some(5)), None);
                assert_eq!(liveness_from_open_error(None), None);
            }

            // v1.39 判据：Toolhelp 会列出已终止但尚未回收的进程（进程对象要等句柄全关才删），
            // 终止后立刻拍快照会把它们当成存活后代。存活状态可读且为 `false` 的行必须剔除；
            // 存活状态读不到时保留该行——宁可保守报仍有后代，也不把无法鉴定的进程当已清理。
            #[test]
            fn terminated_rows_are_not_counted_as_descendants() {
                let root = 500;
                let rows = vec![
                    ProcEntry {
                        pid: 910,
                        parent_pid: root,
                        created_at: Some(6000),
                        alive: Some(false),
                    },
                    ProcEntry {
                        pid: 911,
                        parent_pid: 910,
                        created_at: Some(7000),
                        alive: Some(true),
                    },
                    ProcEntry {
                        pid: 912,
                        parent_pid: root,
                        created_at: Some(8000),
                        alive: Some(true),
                    },
                    ProcEntry {
                        pid: 913,
                        parent_pid: root,
                        created_at: Some(9000),
                        alive: None,
                    },
                ];
                println!(
                    "[evidence] terminated_rows rows={} result={:?}",
                    rows.len(),
                    descendants_of(root, Some(5000), None, &rows)
                );
                assert_eq!(
                    descendants_of(root, Some(5000), None, &rows),
                    vec![912, 913],
                    "已终止行连同其子树必须剔除；存活状态不可读时保留"
                );
            }

            /// 失败取证：区分「根 PID 被复用」（当前根行创建时间不等于记录值）与
            /// 「子进程创建时间读不到导致边被接受」（边时间为 None）。
            fn dump_parent_edges(root: u32, recorded: Option<u64>) -> String {
                let Some(rows) = process_table() else {
                    return "process_table_unavailable".to_string();
                };
                let current_root = rows
                    .iter()
                    .find(|row| row.pid == root)
                    .map(|row| row.created_at);
                let edges: Vec<String> = rows
                    .iter()
                    .filter(|row| row.parent_pid == root)
                    .map(|row| format!("{}@{:?} alive={:?}", row.pid, row.created_at, row.alive))
                    .collect();
                format!(
                    "recorded_root_created={recorded:?} current_root_row={current_root:?} edges={edges:?}"
                )
            }

            // 观测点：Toolhelp 进程树快照 × 作业 PID 列表交叉核对（终止前后）
            #[test]
            fn descendant_snapshot_cross_checks_job_accounting() {
                let mut supervisor = Supervisor::new();
                assert!(supervisor.job.is_some(), "作业应创建成功");
                let mut command = Command::new("cmd");
                command
                    .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                supervisor.configure(&mut command);
                let mut child = command.spawn().expect("spawn cmd");
                supervisor.attach(&mut child);
                assert!(supervisor.attached, "cmd 应成功加入作业");
                let root = child.id();

                let mut observed = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    if snapshot.unobservable.is_none()
                        && !snapshot.descendants.is_empty()
                        && snapshot.observed_descendants_are_controlled()
                    {
                        observed = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = observed.expect("必须能观测到作业内的后代");
                let mut expected: Vec<u32> =
                    before.descendants.iter().map(|entry| entry.pid).collect();
                expected.push(root);
                expected.sort_unstable();
                let mut actual = before
                    .controlled_pids
                    .clone()
                    .expect("作业 PID 列表必须可读");
                actual.sort_unstable();
                println!(
                    "[evidence] descendant_snapshot before root={root} descendants={:?} controlled={:?}",
                    before.descendants, before.controlled_pids
                );
                assert_eq!(
                    actual, expected,
                    "两个独立观测点必须一致：作业 PID 列表 = 根进程 + 进程树快照后代"
                );
                assert!(
                    before.outside_controlled_set().is_empty(),
                    "作业覆盖下的后代不应出现逃逸标记"
                );

                let started = Instant::now();
                let outcome = supervisor.terminate(&mut child);
                let elapsed_ms = started.elapsed().as_millis();
                let after = supervisor.snapshot_descendants(root);
                if outcome != CleanupOutcome::Cleaned {
                    println!(
                        "[probe] cross_check root={root} elapsed_ms={elapsed_ms} active_after={:?} {}",
                        supervisor.active_processes(),
                        dump_parent_edges(root, supervisor.root_created_at)
                    );
                }
                println!(
                    "[evidence] descendant_snapshot after terminate={outcome:?} descendants={:?} controlled={:?} unobservable={:?}",
                    after.descendants, after.controlled_pids, after.unobservable
                );
                assert_eq!(outcome, CleanupOutcome::Cleaned);
                assert!(after.unobservable.is_none(), "终止后快照必须可观测");
                assert!(after.is_empty(), "终止后不得有存活后代");
                assert_eq!(after.controlled_pids, Some(Vec::new()));
            }

            // 交叉核对必须能识别"后代不在受控集合"：未加入作业的进程树要被标出
            #[test]
            fn descendant_snapshot_flags_descendants_outside_controlled_set() {
                let supervisor = Supervisor::new();
                assert!(supervisor.job.is_some(), "作业应创建成功");
                let mut command = Command::new("cmd");
                command
                    .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let mut child = command.spawn().expect("spawn detached cmd");
                let root = child.id();

                let mut observed = None;
                for _ in 0..150 {
                    let snapshot = supervisor.snapshot_descendants(root);
                    if snapshot.unobservable.is_none() && !snapshot.descendants.is_empty() {
                        observed = Some(snapshot);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                let before = observed.expect("未加入作业的进程树也必须可枚举");
                let outside = before.outside_controlled_set();
                println!(
                    "[evidence] descendant_snapshot outside_controlled root={root} descendants={:?} outside={:?} controlled={:?}",
                    before.descendants, outside, before.controlled_pids
                );
                assert!(
                    !before.observed_descendants_are_controlled(),
                    "未加入作业的进程树不得被判为受控"
                );
                assert_eq!(
                    outside.len(),
                    before.descendants.len(),
                    "快照里每个后代都必须被标为作业外"
                );

                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &root.to_string()])
                    .output();
                let _ = child.wait();
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    mod imp {
        use super::{CleanupOutcome, DescendantSnapshot, SnapshotScope};
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
            pub(crate) fn publish_scope_owner(
                &mut self,
                _child: &Child,
                _runs_dir: &std::path::Path,
                _task_id: &str,
                _attempt: u64,
            ) -> Result<(), String> {
                Ok(())
            }
            pub(crate) fn snapshot_descendants(&self, root: u32) -> DescendantSnapshot {
                DescendantSnapshot {
                    root,
                    descendants: Vec::new(),
                    controlled_pids: None,
                    scope: SnapshotScope::ProcessGroup,
                    escaped_scope_unobservable: Some(
                        "descendant_snapshot_unsupported_platform".to_string(),
                    ),
                    unobservable: Some("descendant_snapshot_unsupported_platform".to_string()),
                }
            }
            pub(crate) fn terminate(&mut self, child: &mut Child) -> CleanupOutcome {
                let _ = child.kill();
                CleanupOutcome::Unavailable
            }
        }
    }

    pub(crate) use imp::Supervisor;
    #[cfg(target_os = "linux")]
    pub(crate) use imp::{
        kill_pid, kill_scope, managed_scope_dirs, proc_start, read_scope_members,
        read_scope_populated, remove_scope, scope_kill_supported,
    };
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
            "timeout：超过 {timeout_secs}s 未结束，已向受控集合发送终止信号，但受控集合内仍有存活进程，或快照仍枚举到存活后代；集合外后代是否仍存活取决于观测作用域（见 capabilities.timeout.enumerates_escaped_descendants）"
        ),
        _ => format!(
            "timeout：超过 {timeout_secs}s 未结束，受控集合未能建立或无法查询，仅向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"
        ),
    }
}

/// 取消回执措辞：与 timeout 同为三态分流，但前缀固定 `canceled：`，调用方据此与超时/失败区分。
/// `outcome=None` 只有一种合法来源：启动前就收到取消（没有进程需要清理）。
fn cancel_reason(
    requested_reason: Option<&str>,
    outcome: Option<process_scope::CleanupOutcome>,
) -> String {
    use process_scope::CleanupOutcome;
    let suffix = requested_reason
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("；请求方理由：{value}"))
        .unwrap_or_default();
    match outcome {
        Some(CleanupOutcome::Cleaned) => {
            #[cfg(windows)]
            {
                format!(
                    "canceled：收到 task.cancel，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行；BREAKAWAY 或加入作业失败的作业外进程仍不可观测{suffix}"
                )
            }
            #[cfg(not(windows))]
            {
                format!("canceled：收到 task.cancel，已清理受控集合；逃逸进程是否仍在运行未验证{suffix}")
            }
        }
        Some(CleanupOutcome::StillAlive) => format!(
            "canceled：收到 task.cancel，已向受控集合发送终止信号，但受控集合内仍有存活进程，或快照仍枚举到存活后代；集合外后代是否仍存活取决于观测作用域（见 capabilities.timeout.enumerates_escaped_descendants）{suffix}"
        ),
        Some(CleanupOutcome::Unavailable) => format!(
            "canceled：收到 task.cancel，但受控集合未能建立或无法查询，仅向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证{suffix}"
        ),
        None => {
            format!("canceled：在启动任何进程之前收到 task.cancel，未启动任何进程，无清理动作{suffix}")
        }
    }
}

/// 终止结果的可判定标签，与 `timeout_reason` / `cancel_reason` 的三态同源，不另造判据。
/// `None` 表示本次没有发生终止动作；`not_started` 表示取消在启动任何进程之前生效。
fn cleanup_status_label(
    canceled: bool,
    timed_out: bool,
    outcome: Option<process_scope::CleanupOutcome>,
) -> Option<&'static str> {
    use process_scope::CleanupOutcome;
    if !(canceled || timed_out) {
        return None;
    }
    Some(match outcome {
        None => "not_started",
        Some(CleanupOutcome::Cleaned) => "cleaned",
        Some(CleanupOutcome::StillAlive) => "still_alive",
        Some(CleanupOutcome::Unavailable) => "unavailable",
    })
}

/// `cleanup_claimed` 的唯一来源：回执确为 canceled 且观测到受控集合内无存活。
/// 取消成功不等于已清理：StillAlive / Unavailable / not_started / 缺字段都不得置 true。
fn cleanup_claim_allowed(handback_status: Option<&str>, cleanup_status: Option<&str>) -> bool {
    handback_status == Some("canceled") && cleanup_status == Some("cleaned")
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

/// 同目录临时文件 + rename：跨进程读取方永远不会看到半份 JSON。
/// 取消请求、作用域归属记录都要被另一个进程读到，必须用这个而不是 `write_json_file`。
fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    use std::io::Write;
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("序列化 JSON 失败：{e}"))?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("record.json");
    let tmp = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        FILE_WRITE_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<(), String> {
        let mut file =
            fs::File::create(&tmp).map_err(|e| format!("创建 {} 失败：{e}", tmp.display()))?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("写入 {} 失败：{e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| format!("替换 {} 失败：{e}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

static FILE_WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

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
                "standalone_cwd_required: standalone 必须显式给出 cwd（执行与验收的作用域根目录）"
                    .to_string()
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
    let prefix = format!(
        "standalone_acceptance_invalid: 第 {} 条（{kind}）",
        index + 1
    );
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
            matches!(
                c,
                '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '*' | '+' | '?'
            )
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
                    return Some(format!("环境变量 %{name}% 未展开，无法判定真实位置"));
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
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
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

/// 活跃标记的 RAII 守卫：任何正常返回、提前报错、阻断都会在 Drop 时删除标记。
/// `started_at` 同时是跨进程取消请求的绑定凭据：请求必须点名本轮执行，不能误伤后续同 task_id。
struct InflightGuard {
    path: PathBuf,
    started_at: String,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 取消请求文件路径：`runs/.cancel/<task_id>.json`
fn cancel_request_path(runs_dir: &Path, task_id: &str) -> PathBuf {
    runs_dir.join(CANCEL_DIR).join(format!("{task_id}.json"))
}

/// 请求是否点名「本进程的本次执行」：pid + startedAt 双条件，缺一不认。
fn cancel_request_targets(record: &Value, started_at: &str) -> bool {
    record
        .get("target")
        .and_then(|target| target.get("pid"))
        .and_then(Value::as_u64)
        == Some(u64::from(std::process::id()))
        && record
            .get("target")
            .and_then(|target| target.get("startedAt"))
            .and_then(Value::as_str)
            == Some(started_at)
}

/// 执行侧心跳检查点：每 200ms 读一次 `runs/.cancel/<task_id>.json`。
///
/// 三态由请求文件本身承载，执行侧只推进、不臆测：
/// `pending`（已落盘未读）→ `acknowledged`（已读、正在清理）→ `done`（收尾完成）。
/// 目标不符（上一轮残留请求）一律不改写，交给启动清扫按「目标已死 + 宽限」处理。
struct CancelHeartbeat {
    path: PathBuf,
    started_at: String,
    next_check: Instant,
    acknowledged: bool,
}

impl CancelHeartbeat {
    fn new(runs_dir: &Path, task_id: &str, started_at: &str) -> Self {
        Self {
            path: cancel_request_path(runs_dir, task_id),
            started_at: started_at.to_string(),
            next_check: Instant::now(),
            acknowledged: false,
        }
    }

    fn poll(&mut self, cancel: &CancelToken) {
        let now = Instant::now();
        if now < self.next_check {
            return;
        }
        self.next_check = now + Duration::from_millis(200);
        let Ok(mut record) = read_json(&self.path) else {
            return;
        };
        if record.get("version").and_then(Value::as_str) != Some(CANCEL_VERSION)
            || !cancel_request_targets(&record, &self.started_at)
        {
            return;
        }
        let status = record
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let reason = record
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string);
        match status {
            "pending" => {
                if let Some(object) = record.as_object_mut() {
                    object.insert("status".to_string(), json!("acknowledged"));
                    object.insert("acknowledgedAt".to_string(), json!(now_stamp()));
                    object.insert(
                        "handledBy".to_string(),
                        json!({
                            "pid": std::process::id(),
                            "startedAt": self.started_at.clone(),
                        }),
                    );
                }
                if write_json_atomic(&self.path, &record).is_ok() {
                    // 只有 acknowledged 落盘成功才置位取消：调用方看到的三态与真实终态一致
                    cancel.request(reason);
                    self.acknowledged = true;
                }
            }
            "acknowledged" if !self.acknowledged => {
                cancel.request(reason);
                self.acknowledged = true;
            }
            _ => {}
        }
    }

    /// 执行线程返回前收尾：把请求推进到 `done` 并记录盘上回执状态。
    /// 目标不符 / 请求已被清走时什么都不做。
    fn finish(&mut self, handback_status: Option<&str>, cleanup_status: Option<&str>) {
        let Ok(record) = read_json(&self.path) else {
            return;
        };
        if !cancel_request_targets(&record, &self.started_at) {
            return;
        }
        let status = record
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if status == "done" || status == "ignored" {
            return;
        }
        let mut record = record;
        if let Some(object) = record.as_object_mut() {
            object.insert("status".to_string(), json!("done"));
            object.insert("handback_status".to_string(), json!(handback_status));
            // 文件通道的等效判据：只有 cleaned 才把 cleanup_claimed 置 true
            object.insert(
                "cleanup_claimed".to_string(),
                json!(cleanup_claim_allowed(handback_status, cleanup_status)),
            );
            object.insert("cleanup_status".to_string(), json!(cleanup_status));
            object.insert("finishedAt".to_string(), json!(now_stamp()));
            object.insert(
                "handledBy".to_string(),
                json!({
                    "pid": std::process::id(),
                    "startedAt": self.started_at.clone(),
                }),
            );
        }
        let _ = write_json_atomic(&self.path, &record);
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

struct InflightMarker {
    task_id: String,
    file: String,
    path: PathBuf,
    assessment: Value,
}

enum RestoreOutcome {
    /// 原名位置为空，已把副本放回
    Restored,
    /// 原名位置已被更新者占住：保留新标记，丢弃手上的副本
    Superseded,
}

/// 认领/复核时刻对某路径重跑 stale_rule 的结论
#[derive(Clone, Copy, Debug)]
enum MarkerReread {
    /// 文件已不存在：认领权在别人手上
    Gone,
    /// 仍满足 stale_rule
    Stale,
    /// 已不满足 stale_rule：别人刚刷新过
    NotStale,
    /// 读不出内容：按 unreadable 处理，绝不删除
    Unreadable,
}

fn reread_marker(path: &Path, now: chrono::DateTime<chrono::Utc>) -> MarkerReread {
    match fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(record) => {
                if assess_inflight_marker(&record, now)["stale"] == json!(true) {
                    MarkerReread::Stale
                } else {
                    MarkerReread::NotStale
                }
            }
            Err(_) => MarkerReread::Unreadable,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => MarkerReread::Gone,
        Err(_) => MarkerReread::Unreadable,
    }
}

fn inflight_sweep_entry(marker: &InflightMarker, outcome: &str, reason: &str) -> Value {
    json!({
        "task_id": marker.task_id,
        "file": marker.file,
        "pid": marker.assessment["pid"],
        "stale_reason": marker.assessment["stale_reason"],
        "outcome": outcome,
        "reason": reason,
        "path": format!("runs/.inflight/{}", marker.file)
    })
}

fn inflight_sweep_failure(marker: &InflightMarker, error: String) -> Value {
    json!({
        "task_id": marker.task_id,
        "file": marker.file,
        "pid": marker.assessment["pid"],
        "stale_reason": marker.assessment["stale_reason"],
        "error": error
    })
}

/// 还原「被认领但内容已变」的标记：只做 create-if-absent，绝不覆盖此刻已存在的新标记。
/// 没有 compare-and-rename 语义，这里只能保证「不覆盖」，不能保证检查到认领之间没有窗口。
fn restore_without_clobber(source: &Path, target: &Path) -> std::io::Result<RestoreOutcome> {
    use std::io::Write;
    let bytes = fs::read(source)?;
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(source);
            return Ok(RestoreOutcome::Superseded);
        }
        Err(error) => return Err(error),
    };
    if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(target);
        return Err(error);
    }
    drop(file);
    // 副本在写完原位后消失：原位已经是完整内容，不算失败。
    match fs::remove_file(source) {
        Ok(()) => Ok(RestoreOutcome::Restored),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(RestoreOutcome::Restored),
        Err(error) => Err(error),
    }
}

/// 并发观测点（仅测试编译）：在「扫描判定」与「rename 认领」之间执行，
/// 用于让「检查之后、认领之前被刷新」的窗口可稳定复现（多线程用 Barrier 对齐）。
/// 观测点按目标路径武装：只有正在清扫该路径的线程才会触发，避免并行测试互相串扰。
#[cfg(test)]
struct SweepBeforeClaimHook {
    target: PathBuf,
    hook: fn(&Path),
}

#[cfg(test)]
static SWEEP_BEFORE_CLAIM_HOOK: std::sync::Mutex<Option<SweepBeforeClaimHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn set_sweep_before_claim_hook(target: &Path, hook: Option<fn(&Path)>) {
    *SWEEP_BEFORE_CLAIM_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook.map(|hook| SweepBeforeClaimHook {
        target: target.to_path_buf(),
        hook,
    });
}

#[cfg(test)]
fn run_sweep_before_claim_hook(path: &Path) {
    // 只取函数指针，不持锁执行回调：并发用例会在回调用 Barrier 对齐
    let hook = {
        let armed = SWEEP_BEFORE_CLAIM_HOOK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        armed
            .as_ref()
            .filter(|armed| armed.target == path)
            .map(|armed| armed.hook)
    };
    if let Some(hook) = hook {
        hook(path);
    }
}

/// 并发观测点（仅测试编译）：在「rename 认领」与「内容复核」之间注入一次确定性写入，
/// 让「检查 → 认领 → 复核」窗口可被稳定复现，而不是靠竞态碰运气。
/// 参数为（原名位置, 被认领的副本），测试据此构造复核时刻的真实文件状态。
#[cfg(test)]
struct SweepAfterClaimHook {
    target: PathBuf,
    hook: fn(&Path, &Path),
}

#[cfg(test)]
static SWEEP_AFTER_CLAIM_HOOK: std::sync::Mutex<Option<SweepAfterClaimHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn set_sweep_after_claim_hook(target: &Path, hook: Option<fn(&Path, &Path)>) {
    *SWEEP_AFTER_CLAIM_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook.map(|hook| SweepAfterClaimHook {
        target: target.to_path_buf(),
        hook,
    });
}

#[cfg(test)]
fn run_sweep_after_claim_hook(path: &Path, claimed: &Path) {
    let hook = {
        let armed = SWEEP_AFTER_CLAIM_HOOK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        armed
            .as_ref()
            .filter(|armed| armed.target == path)
            .map(|armed| armed.hook)
    };
    if let Some(hook) = hook {
        hook(path, claimed);
    }
}

/// 并发观测点（仅测试编译）：在「认领副本重跑 stale_rule」与「删除/还原」之间执行。
/// 用于确定性地复现「复核之后认领副本被另一方处置」的窗口（v1.49 诊断）。
#[cfg(test)]
struct SweepAfterRereadHook {
    target: PathBuf,
    hook: fn(&Path, &Path),
}

#[cfg(test)]
static SWEEP_AFTER_REREAD_HOOK: std::sync::Mutex<Option<SweepAfterRereadHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn set_sweep_after_reread_hook(target: &Path, hook: Option<fn(&Path, &Path)>) {
    *SWEEP_AFTER_REREAD_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook.map(|hook| SweepAfterRereadHook {
        target: target.to_path_buf(),
        hook,
    });
}

#[cfg(test)]
fn run_sweep_after_reread_hook(path: &Path, claimed: &Path) {
    let hook = {
        let armed = SWEEP_AFTER_REREAD_HOOK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        armed
            .as_ref()
            .filter(|armed| armed.target == path)
            .map(|armed| armed.hook)
    };
    if let Some(hook) = hook {
        hook(path, claimed);
    }
}

/// 清扫并发用例的时序记录（仅测试编译）：谁（线程 id）在什么时候（毫秒）做了什么。
/// 默认门禁失败时把这份时序直接打进断言消息，避免下次只看到「lost_race 实际 2」。
#[cfg(test)]
mod sweep_trace {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::Instant;

    struct TraceState {
        target: PathBuf,
        start: Instant,
        events: Vec<String>,
    }

    static STATE: Mutex<Option<TraceState>> = Mutex::new(None);

    /// 只记录 target 这条标记的事件：并行测试各自清扫不同文件，避免时序互相串扰。
    pub(super) fn reset(target: &Path) {
        *STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(TraceState {
            target: target.to_path_buf(),
            start: Instant::now(),
            events: Vec::new(),
        });
    }

    pub(super) fn record(target: &Path, step: &str, detail: impl std::fmt::Display) {
        let mut state = STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(state) = state.as_mut() else {
            return;
        };
        if state.target != target {
            return;
        }
        state.events.push(format!(
            "[{:>8.3}ms] thread={:?} {step} {detail}",
            state.start.elapsed().as_secs_f64() * 1000.0,
            std::thread::current().id()
        ));
    }

    pub(super) fn dump(target: &Path) -> String {
        let state = STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match state.as_ref().filter(|state| state.target == target) {
            Some(state) if !state.events.is_empty() => state.events.join("\n"),
            _ => "(无时序记录：该标记未被本用例观测，或记录已重置)".to_string(),
        }
    }
}

#[cfg(test)]
struct PruneRenameHook {
    target: PathBuf,
    hook: fn(&Path, &Path),
}

#[cfg(test)]
static PRUNE_BEFORE_RENAME_HOOK: std::sync::Mutex<Option<PruneRenameHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
static PRUNE_AFTER_RENAME_HOOK: std::sync::Mutex<Option<PruneRenameHook>> =
    std::sync::Mutex::new(None);

/// 并发用例的 Barrier：两个认领者各自取克隆后放锁执行，避免持锁等待。
#[cfg(test)]
static PRUNE_BEFORE_RENAME_BARRIER: std::sync::Mutex<Option<std::sync::Arc<std::sync::Barrier>>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
static PRUNE_RACE_HOOK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn set_prune_before_rename_hook(target: &Path, hook: Option<fn(&Path, &Path)>) {
    *PRUNE_BEFORE_RENAME_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook.map(|hook| PruneRenameHook {
        target: target.to_path_buf(),
        hook,
    });
}

#[cfg(test)]
fn set_prune_after_rename_hook(target: &Path, hook: Option<fn(&Path, &Path)>) {
    *PRUNE_AFTER_RENAME_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook.map(|hook| PruneRenameHook {
        target: target.to_path_buf(),
        hook,
    });
}

fn marker_report(marker: &InflightMarker) -> Value {
    let mut report = marker.assessment.clone();
    if let Some(object) = report.as_object_mut() {
        object.insert("task_id".to_string(), json!(marker.task_id));
        object.insert("file".to_string(), json!(marker.file));
    }
    report
}

fn parse_stamp(value: Option<&Value>) -> Option<chrono::DateTime<chrono::Utc>> {
    value
        .and_then(Value::as_str)
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .map(|stamp| stamp.with_timezone(&chrono::Utc))
}

/// 与 `marker_is_stale` 同源判定，但把依据拆成机器可判的原因码与时间量
fn assess_inflight_marker(record: &Value, now: chrono::DateTime<chrono::Utc>) -> Value {
    let pid = record.get("pid").and_then(Value::as_u64);
    let pid_alive = pid.map(|value| process_is_alive(value as u32));
    let started = parse_stamp(record.get("startedAt"));
    let deadline = parse_stamp(record.get("deadlineAt"));
    let stale = marker_is_stale(record, now);
    let stale_reason = if stale {
        if pid.is_some() {
            "pid_missing"
        } else {
            "deadline_expired"
        }
    } else if pid_alive == Some(true) {
        "pid_alive"
    } else if pid.is_some() {
        "pid_missing_grace"
    } else {
        "deadline_in_grace"
    };
    let age_secs = started.map(|started| (now - started).num_seconds().max(0));
    let deadline_overdue_secs = deadline
        .map(|deadline| (now - deadline).num_seconds())
        .filter(|overdue| *overdue > 0);
    json!({
        "pid": pid,
        "pid_alive": pid_alive,
        "stale": stale,
        "stale_reason": stale_reason,
        "removable": stale,
        "mode": record.get("mode").cloned().unwrap_or(Value::Null),
        "state": record.get("state").cloned().unwrap_or(Value::Null),
        "startedAt": record.get("startedAt").cloned().unwrap_or(Value::Null),
        "deadlineAt": record.get("deadlineAt").cloned().unwrap_or(Value::Null),
        "age_secs": age_secs,
        "deadline_overdue_secs": deadline_overdue_secs
    })
}

/// PM 判活约定：权威字段只有 task_id + stale；诊断字段与 attempt 启发式不得参与判活
fn inflight_liveness_rule() -> Value {
    json!({
        "version": "inflight-liveness/1",
        "authoritative_fields": ["markers[].task_id", "markers[].stale"],
        "cleanup_fields": ["markers[].removable"],
        "diagnostic_fields": [
            "markers[].pid",
            "markers[].pid_alive",
            "markers[].stale_reason",
            "markers[].startedAt",
            "markers[].deadlineAt",
            "markers[].age_secs",
            "markers[].deadline_overdue_secs",
            "markers[].mode",
            "markers[].state"
        ],
        "decision": {
            "task_live": "markers[] 中存在该 task_id 且 stale=false",
            "task_not_live": "markers[] 中不存在该 task_id，或存在但 stale=true",
            "not_live_does_not_prove": "只表示当前没有活跃执行者；不表示任务未执行、未完成或记录不可用",
            "result_authority": "任务结果仍以 receipt.get / verdict.json / standalone-result.json 为准"
        },
        "not_liveness_evidence": [
            "attempt_missing_handback",
            "attempt_missing_dispatch",
            "raw_marker_file_presence",
            "marker_age",
            "pid_number"
        ],
        "fallback": {
            "allowed": false,
            "only_when": "runs.inflight 不存在（旧版 AW）",
            "legacy_heuristic": "任一 attempt 缺 handback.json 或 dispatch.json 视为可能活跃",
            "warning": "legacy 结果与 AW 判定不一致，仅作降级"
        }
    })
}

/// 清扫自描述：触发点、周期开关与审计口径（自动路径复用同一判据与同一实现）
fn inflight_sweep_rule() -> Value {
    json!({
        "version": "inflight-sweep/1",
        "triggers": ["startup", "periodic", "task_start", "runs.prune", "runs.inflight.clean"],
        "automatic": {
            "startup": "bridge 进程启动时一次，默认执行",
            "periodic": "默认每 3600 秒一次，默认开启",
            "env": INFLIGHT_AUTO_SWEEP_ENV,
            "env_semantics": "未设置=默认每小时；0=仅启动时一次；off=关闭自动清扫；正整数=周期秒数",
            "opt_out": "设置 off 后只剩 task_start / runs.prune / runs.inflight.clean 三个显式路径"
        },
        "trigger_conditions": {
            "startup": "进程启动、进入 stdio 循环之前执行一次；只按 stale_rule 删除，审计 trigger=startup",
            "periodic": "按周期在后台线程执行；与 startup 共用 sweep_inflight_markers，审计 trigger=periodic",
            "task_start": "同一 task_id 的标记已存在且按 stale_rule 判定 stale=true 时，先清残标再 create_new",
            "runs.prune": "dry_run=false 且 confirm=true 时，枚举候选前先清全部残标",
            "runs.inflight.clean": "dry_run=false 且 confirm=true 时清全部残标（或 task_id 过滤范围内）"
        },
        "audit": "runs/inflight-sweep.jsonl",
        "audit_version": INFLIGHT_SWEEP_VERSION,
        "audit_gate": "plan 行写不进去则本轮零删除；自动触发与显式触发写同一审计，仅 trigger 不同",
        "criteria": "markers[].stale（pid 存活优先于 deadlineAt）；自动路径不另写判据",
        "never_removed": ["active_pid_alive", "unreadable"],
        "atomic_delete": "rename to .tmp 原子认领；认领后重读并重跑同一份 stale_rule：仍 stale 才删（outcome=deleted），已不 stale 就地 create-if-absent 还原（outcome=not_mine，reason=marker_changed/marker_superseded/recheck_unreadable），认领副本已不存在记 outcome=lost_race（不算错误）",
        "idempotent": true,
        "idempotent_semantics": "无可清残标时不删除、不追加审计；重复触发返回 removed_total=0",
        "outcome_totals": ["deleted_total", "lost_race_total", "not_mine_total"],
        "limits": [
            "same_volume_rename_required",
            "transient_name_absence_during_claim",
            "audit_append_not_transactional",
            "unreadable_not_self_healing"
        ]
    })
}

/// 残留判据自描述：调用方不需要读源码即可复算 stale
fn inflight_stale_rule() -> Value {
    json!({
        "pid_alive": "never_stale",
        "pid_missing_grace_secs": INFLIGHT_PID_MISSING_GRACE_SECS,
        "pid_missing": "stale",
        "deadline_grace_secs": INFLIGHT_STALE_GRACE_SECS,
        "no_pid": "stale_only_after_deadlineAt_plus_deadline_grace_secs",
        "live_pid_wins_over_deadline": true,
        "cleanable_only_when": "stale=true",
        "unreadable": "kept_never_deleted"
    })
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

/// 证据文件创建失败时的错误文本。
///
/// `NotFound` 且目录确实已不存在，说明是 attempt 目录被外部清理/改名，必须具名报
/// `attempt_dir_lost`；其它情况（目录还在但文件建不出来）保留原始 IO 错误。
fn create_evidence_error(run_dir: &Path, name: &str, error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound && !run_dir.is_dir() {
        attempt_dir_lost_message(run_dir)
    } else {
        format!("创建 {name} 失败：{error}")
    }
}

#[cfg(test)]
mod evidence_error_tests {
    use super::*;

    // 目录已被外部删除时，创建证据文件必须报具名的 attempt_dir_lost，
    // 而不是把裸 IO 错误（os error 3）直接抛给调用方。
    #[test]
    fn missing_dir_maps_to_attempt_dir_lost() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("attempt-1");
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        let text = create_evidence_error(&dir, "stdout", error);
        println!("[evidence] missing_dir text={text}");
        assert!(text.contains("attempt_dir_lost"), "{text}");
    }

    // 目录还在时不得误报 attempt_dir_lost，保留原始错误。
    #[test]
    fn present_dir_keeps_raw_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        let text = create_evidence_error(dir.path(), "stdout", error);
        assert!(text.contains("创建 stdout 失败"), "{text}");
        assert!(!text.contains("attempt_dir_lost"), "{text}");
    }
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
    canceled: bool,
    cleanup_status: Option<&'static str>,
    spawn_error: Option<String>,
    blocked_reason: Option<String>,
}

/// 不经 shell：直接启动 argv[0]，其余参数逐项传入；current_dir 固定为 cwd
fn run_standalone_command(
    spec: &StandaloneSpec,
    run_dir: &Path,
    runs_dir: &Path,
    task_id: &str,
    attempt: u64,
    cancel: &CancelToken,
    heartbeat: Option<&mut CancelHeartbeat>,
) -> Result<StandaloneRun, String> {
    let stdout_path = run_dir.join("stdout.txt");
    let stderr_path = run_dir.join("stderr.txt");
    let stdout_file =
        fs::File::create(&stdout_path).map_err(|e| create_evidence_error(run_dir, "stdout", e))?;
    let stderr_file =
        fs::File::create(&stderr_path).map_err(|e| create_evidence_error(run_dir, "stderr", e))?;

    let mut command = Command::new(&spec.argv[0]);
    command
        .args(&spec.argv[1..])
        .current_dir(&spec.cwd)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let started = Instant::now();
    let deadline = Duration::from_secs(spec.timeout_secs);
    let mut timed_out = false;
    let mut canceled = cancel.is_canceled();
    let mut cleanup_outcome = None;
    let mut spawn_error: Option<String> = None;
    let mut supervisor = process_scope::Supervisor::new();
    supervisor.configure(&mut command);
    let mut heartbeat = heartbeat;
    let exit_status = if canceled {
        None
    } else {
        match command.spawn() {
            Ok(mut child) => {
                supervisor.attach(&mut child);
                if let Err(error) =
                    supervisor.publish_scope_owner(&child, runs_dir, task_id, attempt)
                {
                    eprintln!(
                        "[cgroup-scope] 归属记录写入失败 task_id={task_id} attempt={attempt}: {error}"
                    );
                }
                if let Some(error) = supervisor.attach_error().map(str::to_string) {
                    let _ = child.wait();
                    spawn_error = Some(error);
                    None
                } else {
                    loop {
                        // 跨进程取消的心跳检查点（standalone 与 dispatched 同一路径）
                        if let Some(heartbeat) = heartbeat.as_deref_mut() {
                            heartbeat.poll(cancel);
                        }
                        match child.try_wait() {
                            Ok(Some(status)) => break Some(status),
                            Ok(None) => {
                                if cancel.is_canceled() {
                                    canceled = true;
                                    cleanup_outcome = Some(supervisor.terminate(&mut child));
                                    break None;
                                }
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
        .or_else(|| canceled.then(|| cancel_reason(cancel.reason().as_deref(), cleanup_outcome)))
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
        canceled,
        cleanup_status: cleanup_status_label(canceled, timed_out, cleanup_outcome),
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

fn changed_between(before: &HashSet<String>, after: &HashSet<String>) -> (Vec<String>, bool) {
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

/// 自动清扫计划：启动时一次总是执行（除非整体关闭），周期由环境变量控制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoSweepSchedule {
    /// 关闭自动清扫（仅保留 task_start / prune / clean 三个显式路径）
    Off,
    /// 只在服务启动时清扫一次
    StartupOnly,
    /// 启动一次 + 每个间隔清扫一次
    Every(Duration),
}

/// 解析 `AW_INFLIGHT_SWEEP_SECS`：
/// - 未设置/空：启动一次 + 默认每 3600 秒一次；
/// - `off`：关闭自动清扫；
/// - `0`：仅启动时一次；
/// - 正整数：启动一次 + 每 N 秒一次；非法值按默认周期处理并写 stderr。
pub fn auto_sweep_schedule() -> AutoSweepSchedule {
    let default = AutoSweepSchedule::Every(Duration::from_secs(INFLIGHT_AUTO_SWEEP_DEFAULT_SECS));
    let Ok(raw) = std::env::var(INFLIGHT_AUTO_SWEEP_ENV) else {
        return default;
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return default;
    }
    if raw.eq_ignore_ascii_case("off") {
        return AutoSweepSchedule::Off;
    }
    match raw.parse::<u64>() {
        Ok(0) => AutoSweepSchedule::StartupOnly,
        Ok(secs) => AutoSweepSchedule::Every(Duration::from_secs(secs)),
        Err(_) => {
            eprintln!(
                "[inflight-sweep] {INFLIGHT_AUTO_SWEEP_ENV}={raw} 不是合法秒数，按默认 {INFLIGHT_AUTO_SWEEP_DEFAULT_SECS} 秒处理"
            );
            default
        }
    }
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

/// 当前平台标识：稳定枚举，供能力声明按平台判定
fn platform_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(unix) {
        "linux"
    } else {
        "other"
    }
}

#[allow(clippy::too_many_arguments)]
fn capability_entry(
    id: &str,
    holds: Option<bool>,
    status: &str,
    scope: &str,
    reason_code: Option<&str>,
    automatic: bool,
    platforms_verified: &[&str],
    platforms_unverified: &[&str],
    evidence: &[&str],
    detail: &str,
) -> Value {
    json!({
        "id": id,
        "holds": holds,
        "status": status,
        "scope": scope,
        "reason_code": reason_code,
        "automatic": automatic,
        "platforms_verified": platforms_verified,
        "platforms_unverified": platforms_unverified,
        "evidence": evidence,
        "detail": detail
    })
}

/// 机器可读的能力与限制声明：每条 claim 用 holds/status/reason_code/平台覆盖表达，
/// 调用方按字段判定，不依赖描述散文。
fn capabilities_payload() -> Value {
    let both = ["windows", "linux"];
    let entries = vec![
        capability_entry(
            "timeout.terminates_controlled_set",
            Some(true),
            "proven",
            "POSIX 进程组 / Windows Job Object（Windows 子进程以挂起态创建，先加入作业再恢复执行）",
            None,
            true,
            &both,
            &[],
            &[
                "timeout_cleanup_three_states",
                "posix_process_group_cleanup",
                "windows_job_object_cleanup",
            ],
            "只向受控集合发送终止信号；仅当观测到集合内无存活进程才写「已清理受控集合」。",
        ),
        capability_entry(
            "timeout.terminates_once_no_retry",
            Some(true),
            "proven",
            "同一 attempt 内的终止动作只执行一次，不自动重试",
            None,
            true,
            &both,
            &[],
            &["timeout_cleanup_three_states"],
            "终止只尝试一次；未观测到集合内无存活进程时如实报「受控集合内仍有存活进程」。",
        ),
        capability_entry(
            "timeout.cleans_entire_process_tree",
            Some(true),
            "proven",
            "仅 Linux（platform=linux）：需可写 cgroup v2 且内核提供 `cgroup.kill`（>=5.14）。整树 = 终止开始时可归属本任务的全部后代（/proc 父链闭包 ∪ cgroup.procs 成员）及其在终止过程中派生的子进程；四个观测量（cgroup.procs 空、populated 0、逃逸 PID 不存在、快照无后代）同时成立才报 Cleaned。Windows 侧由 `timeout.cleans_entire_process_tree_windows` 单独声明，不并入本条",
            None,
            true,
            &["linux"],
            &["windows", "macos"],
            &[
                "timeout_cleanup_three_states",
                "descendant_snapshot_cross_checks_job_accounting",
                "descendant_snapshot_cross_checks_process_group",
                "timeout_cleans_controlled_set_including_grandchild",
                "cgroup_scope_enumerates_adopted_escapee",
                "cgroup_kill_terminates_adopted_escapee",
                "cgroup_kill_terminates_escapee_that_left_cgroup_procs",
                "cgroup_without_kill_reports_still_alive",
                "cgroup_degraded_terminate_reports_unavailable",
            ],
            "platform=linux，WSL Ubuntu 24.04 LTS（root，cgroup2fs，cgroup.kill 存在），内核 6.18.33.2-microsoft-standard-WSL2 实测。v1.49 起 `descendant_snapshot_cross_checks_process_group` 标记为已知不稳定（POSIX 全量并行时偶发 outcome=Unavailable，默认门禁按 WARN 呈现，见 scripts/test-gate/quarantine.txt）；本条 proven 结论由其余独立用例共同支撑，不单独依赖该条。`cgroup_kill_terminates_escapee_that_left_cgroup_procs`：父→子→孙，孙进程 setsid 后把自己写进旁路 cgroup（离开本作用域 cgroup.procs），`cgroup.kill` 打不到它；终止后 cgroup.procs 空、populated 0、逃逸 PID 不存在、快照无后代，四观测量同时成立才报 Cleaned。v1.45 起终止前用「父链闭包 ∪ cgroup.procs」建立名单并附 /proc/<pid>/stat 的 starttime，逃逸进程逐个 SIGKILL 收口；终止过程中新派生的子进程每轮并入名单；starttime 不一致（PID 复用）绝不发信号。内核没有 cgroup.kill：不写 cgroup.kill，v1.45 仍按名单杀掉已知逃逸者，但作用域记账不空时如实 StillAlive（cgroup_without_kill_reports_still_alive），不会谎报 Cleaned。作用域建不起来（cgroup v1 / 非 root / 容器未委派）：不动进程组，只向直接子进程发送终止信号并报 Unavailable（cleanup_status=unavailable、cleanup_claimed=false）；快照仍带 escaped_scope_unobservable，降级原因不静默。残余边界：进程若在 terminate 开始前就同时离开 cgroup.procs 且父链被收养，本机没有 OS 原语可指认它——这是 Linux 侧仅剩的缺口，快照与回执标 unobservable，不读成「无残留」。Windows 见 timeout.cleans_entire_process_tree_windows；macOS 未验证。",
        ),
        capability_entry(
            "timeout.cleans_entire_process_tree_windows",
            Some(false),
            "not_proven",
            "仅 Windows：Job Object 只覆盖加入作业的进程；作业外进程（BREAKAWAY、加入作业失败、不进作业的 conhost.exe 等）可能在终止后存活，整树不成立",
            Some("requires_out_of_job_enumerable_terminable_os_primitive"),
            false,
            &["windows"],
            &[],
            &[
                "timeout_cleanup_three_states",
                "descendant_snapshot_cross_checks_job_accounting",
                "timeout_cleans_controlled_set_including_grandchild",
            ],
            "需要「作业外进程可枚举 + 可终止 + 存活有权威来源」的 OS 原语，本机不存在。已核查 Windows API：`QueryInformationJobObject(JobObjectBasicProcessIdList)` 与 `TerminateJobObject` 都只作用于作业内成员；`CreateToolhelp32Snapshot`/`EnumProcesses`/WMI 能列全系统进程，但父 PID 不携带所有权且会被复用，无法权威归属到本任务。受控集合内的孙进程已由握手夹具证成（timeout_cleans_controlled_set_including_grandchild，ready_delay_ms 实测），但这只覆盖作业内成员，不上调为整树。审计 §56.1 给了替代路径与调用方口径：把 timeout 理解为「受控集合清理」，不要用 cleaned 推导无残留；需要强隔离请放到容器/Windows 服务边界。",
        ),
        capability_entry(
            "timeout.enumerates_escaped_descendants",
            Some(true),
            "proven",
            "仅 Linux（platform=linux）：cgroup v2 可写时，cgroup.procs 成员集合是逃逸后代的权威枚举，不依赖父链，换父被收养后仍可枚举。退化条件：作用域建不起来（无写权限/非 root/容器未委派/cgroup v1）→ 记 escaped_scope_unobservable 并退回进程组旁证；运行期 cgroup.procs 读不出 → 整份快照 unobservable，绝不当空；进程已离开 cgroup.procs（写进别的作用域）→ 作用域枚举不含它，只有仍挂在父链上时由父链快照可见（旁证），根退出被收养后不可见。Windows 侧由 `timeout.enumerates_escaped_descendants_windows` 单独声明",
            None,
            true,
            &["linux"],
            &["windows", "macos"],
            &[
                "descendant_snapshot_flags_descendants_outside_controlled_set",
                "cgroup_scope_enumerates_adopted_escapee",
                "cgroup_members_unreadable_is_not_empty",
                "cgroup_kill_terminates_escapee_that_left_cgroup_procs",
                "setsid_escape_is_flagged_then_unobservable_after_reparent",
                "cgroup_unavailable_degrades_with_reason",
            ],
            "platform=linux，WSL Ubuntu 24.04 LTS（root，cgroup2fs），内核 6.18.33.2-microsoft-standard-WSL2 实测。`cgroup_scope_enumerates_adopted_escapee`：根退出、逃逸后代被收养（ppid 不再指向根）后，cgroup.procs 成员集合仍精确点名该 PID（members=[escaped]，assert_eq），这是成员集合本身的权威性，不是父链旁证。反例实测：`cgroup_unavailable_degrades_with_reason`（作用域建不起来 → Degraded + escaped_scope_unobservable，退回进程组旁证）；`cgroup_members_unreadable_is_not_empty`（Active 但 cgroup.procs 读不出 → 整份 unobservable，不读成空）；`cgroup_kill_terminates_escapee_that_left_cgroup_procs`（进程把自己写进旁路 cgroup 后不在作用域成员里，只能在脱离前靠父链枚举——已脱离即降级为不可见）。无 cgroup.kill 不影响枚举权威性，只影响终止：v1.45 由终止前名单补足已知逃逸者。Windows 见 timeout.enumerates_escaped_descendants_windows；macOS 未验证。",
        ),
        capability_entry(
            "timeout.enumerates_escaped_descendants_windows",
            Some(false),
            "not_proven",
            "仅 Windows：Toolhelp 父链快照是旁证，不是作业外进程的权威枚举；作业外进程（如 conhost.exe）、创建时间或存活状态读不到时仍可能误报",
            Some("requires_out_of_job_enumerable_terminable_os_primitive"),
            false,
            &["windows"],
            &[],
            &[
                "descendant_snapshot_flags_descendants_outside_controlled_set",
                "descendant_snapshot_cross_checks_job_accounting",
            ],
            "需要「作业外进程可枚举 + 可终止 + 存活有权威来源」的 OS 原语，本机不存在。v1.39 起每条父链边要求「子进程创建时间 ≥ 父进程创建时间」、根→直接子进程加「≤ 根退出时间」上界，并剔除已终止或 PID 已不存在的行，同号 PID 复用后的无关树与枚举残行不再被误连（审计 §42），但仍只是旁证：作业外进程与读不到存活状态的进程无法给出权威结论。Windows 侧作业成员枚举只能证明「作业内成员是否还在」，不是作业外进程的权威枚举；审计 §56.1 说明为什么没有等价原语、以及调用方应改用哪些判据。",
        ),
        capability_entry(
            "timeout.cgroup_v2_scope_observation",
            Some(true),
            "proven",
            "Linux：子进程在 exec 前写入独立 cgroup v2，成员集合独立于父链；换父被收养后仍可枚举。建立失败不阻断执行，只降级并记原因",
            None,
            true,
            &["linux"],
            &["windows", "macos"],
            &[
                "cgroup_scope_enumerates_adopted_escapee",
                "cgroup_unavailable_degrades_with_reason",
            ],
            "只声明观测能力；终止能力由 `timeout.cgroup_kill_terminates_scope` 单独声明。实测 WSL Ubuntu 24.04（cgroup v2，root）下被收养逃逸后代仍被枚举。Windows 无 cgroup v2：Job Object 成员枚举只覆盖作业内进程，且无法观测 BREAKAWAY 出去的进程，因此不把本条升级到 windows；Windows 的独立成员枚举由 `timeout.descendant_snapshot_cross_check` 单独声明。审计 §56.2。",
        ),
        capability_entry(
            "timeout.cgroup_kill_terminates_scope",
            Some(true),
            "proven",
            "Linux：可写 cgroup v2 且内核提供 `cgroup.kill`（>=5.14）时，终止覆盖整个作用域，包括换组/换父被收养的逃逸后代；建立失败或内核无该文件时不宣称作用域已清空",
            None,
            true,
            &["linux"],
            &["windows", "macos"],
            &["cgroup_kill_terminates_adopted_escapee"],
            "写 `cgroup.kill` 后按三个独立观测量复核：`cgroup.procs` 为空、`cgroup.events` 为 `populated 0`、逃逸 PID 已不存在；任一不满足或读不出即报 StillAlive/Unavailable，绝不当成已清理。Windows 无 `cgroup.kill` 等价物：`TerminateJobObject` 只杀与作业关联的成员（含嵌套子作业），无法覆盖已离开作业或从未加入作业的进程，AW 也未建立嵌套作业，故不升级到 windows。审计 §56.2。",
        ),
        capability_entry(
            "timeout.descendant_snapshot_cross_check",
            Some(true),
            "proven",
            "从任务根 PID 按创建时父链枚举存活后代，并与受控集合成员（Windows 作业 PID 列表 / Linux 进程组成员，cgroup 可建时并入 cgroup 成员）交叉核对；任一查询失败记 Unavailable",
            None,
            true,
            &both,
            &[],
            &[
                "descendant_snapshot_cross_checks_job_accounting",
                "descendant_snapshot_cross_checks_process_group",
                "descendant_snapshot_flags_descendants_outside_controlled_set",
                "pid_reuse_parent_chain_edge_is_rejected",
                "terminated_rows_are_not_counted_as_descendants",
            ],
            "只回答「枚举到的后代是否都在受控集合内、终止后是否仍可枚举到」；v1.39 起 Windows 父链边带创建时间校验（子 ≥ 父）、根→直接子进程再加「≤ 根退出时间」上界，并剔除 `GetExitCodeProcess` 判为已终止或 `OpenProcess` 返回 `ERROR_INVALID_PARAMETER`（PID 已不存在）的行；v1.40 起 Linux 父链边按 `/proc/<pid>/stat` 的 `starttime` 做同一判据（子 ≥ 父），只放宽该边父进程启动时刻读不到的情形（`pid_reuse_parent_chain_edge_is_rejected` / `terminated_rows_are_not_counted_as_descendants` 确定性覆盖）；不覆盖换父后被收养、作业外进程（如不进作业的 `conhost.exe`）、以及创建时间/存活状态读不到时的同号 PID 复用，也不覆盖集合未能建立的情形。",
        ),
        capability_entry(
            "path.argv_scope_enforced",
            Some(true),
            "proven",
            "standalone argv[1..] 中可判定的路径形态（独立值、--flag=value、引号、相对/绝对/UNC/盘符）",
            None,
            true,
            &both,
            &[],
            &[
                "standalone_argv_path_escape",
                "standalone_argv_path_unknown",
            ],
            "越界在占号前以 standalone_argv_path_escape 拒绝；无法判定的形态放行并记 standalone_argv_path_unknown。",
        ),
        capability_entry(
            "platform.posix_inflight_semantics_verified",
            Some(true),
            "proven",
            "POSIX 上的 inflight create_new、stale 判定、prune 跳过与标记删除保护",
            None,
            false,
            &both,
            &["macos"],
            &[
                "standalone_inflight_marker_is_visible_then_removed",
                "prune_skips_task_with_live_inflight_marker",
                "runs_inflight_lists_and_cleans_only_stale_markers",
                "posix_rename_active_attempt_semantics",
            ],
            "POSIX 与 Windows 均跑通 inflight 标记、stale 判定、prune 跳过与清理保护用例；POSIX 上已单独复核 rename 活跃 attempt 目录：进程 cwd 指向该目录且持有目录内文件 fd 时 rename 与随后的 remove_dir_all 均成功，cwd/fd 跟随新路径且 fd 仍读到原内容。macOS 未验证。",
        ),
        capability_entry(
            "concurrency.prune_skips_known_active_task",
            Some(true),
            "proven",
            "inflight 快照在候选枚举入口优先于一切候选条件（含超龄非最新 attempt）",
            None,
            true,
            &both,
            &[],
            &["prune_skips_task_with_live_inflight_marker"],
            "标记存在时该 task 只进 skipped[]，记 task_inflight / task_inflight_stale。",
        ),
        capability_entry(
            "concurrency.check_then_delete_atomic",
            Some(true),
            "proven",
            "rename 认领 + 认领后重跑 stale_rule：并发清扫同一残标只有一方 deleted、其余 lost_race；被刷新的标记零误删（not_mine）",
            None,
            true,
            &both,
            &[],
            &[
                "sweep_concurrent_claimers_delete_once_others_lost_race",
                "sweep_concurrent_refresher_never_deletes_fresh_marker",
                "sweep_restore_never_clobbers_concurrent_new_marker",
            ],
            "依据：先同卷 rename 把目标改名为 .tmp 认领，再重读认领副本并重跑同一份 stale_rule；仍 stale 才删除，已不 stale 就地 create-if-absent 还原并记 not_mine，认领副本已不存在记 lost_race。红测试证实旧代码在「刷新者只延长 deadlineAt、身份三元组不变」时会误删，现由重跑 stale_rule 拦住。残余边界：rename 必须同卷（跨卷 EXDEV 则认领失败、零删除）；认领与还原之间原名短暂空缺；审计 append 非事务（plan 先写，崩溃时可能缺 result 行）；Windows 上并发 rename 可能都返回成功，故以认领副本是否存在判定归属。",
        ),
        capability_entry(
            "concurrency.sweep_restore_never_clobbers",
            Some(true),
            "proven",
            "清扫者 rename 认领后复核内容不一致时，还原只做 create-if-absent（create_new/O_EXCL），不覆盖此刻已被更新者占住的原名",
            None,
            true,
            &both,
            &["macos"],
            &["sweep_restore_never_clobbers_concurrent_new_marker"],
            "只保证「不删、不覆盖复核后已不 stale 的那份标记」：认领副本重跑 stale_rule 后已不 stale 时，原位已被占住则保留新标记并记 marker_superseded + outcome=not_mine，原位为空才放回并记 marker_changed + outcome=not_mine。不保证认领窗口内不出现短暂文件名空缺，也不保证窗口内不会有第三方同时开跑。",
        ),
        capability_entry(
            "inflight.marker_cross_process_visible",
            Some(true),
            "proven",
            "runs/.inflight/<task_id>.json，create_new 原子占位、异常退出留痕",
            None,
            true,
            &both,
            &[],
            &["standalone_inflight_marker_is_visible_then_removed"],
            "执行期间标记跨进程可见；正常结束由 Drop 删除。",
        ),
        capability_entry(
            "inflight.stale_marker_observable",
            Some(true),
            "supported",
            "runs.inflight action=list：逐个标记给出 stale 与 stale_reason",
            None,
            true,
            &both,
            &["macos"],
            &["runs_inflight_lists_and_cleans_only_stale_markers"],
            "只读查询，不修改任何标记；判据由 stale_rule 机器可读给出。",
        ),
        capability_entry(
            "inflight.stale_marker_cleanable",
            Some(true),
            "supported",
            "runs.inflight action=clean：默认 dry_run，真删需 confirm",
            None,
            false,
            &both,
            &["macos"],
            &["runs_inflight_lists_and_cleans_only_stale_markers"],
            "只删 stale=true 的标记；除显式 clean 外，服务启动/周期清扫与 task 重启、prune 真删前也按同一 stale_rule 清扫，均写 runs/inflight-sweep.jsonl。",
        ),
        capability_entry(
            "inflight.stale_markers_auto_swept",
            Some(true),
            "proven",
            "服务启动时自动清扫一次；默认每 3600 秒再自动清扫；与显式 clean 共用 sweep_inflight_markers 与 stale_rule，审计 trigger=startup|periodic",
            None,
            true,
            &both,
            &["macos"],
            &[
                "inflight_sweep_on_task_start_removes_only_target_stale_marker",
                "inflight_sweep_on_prune_removes_stale_marker_and_unblocks_candidate",
                "inflight_clean_writes_audit_and_is_idempotent",
                "inflight_startup_sweep_removes_stale_and_keeps_active",
                "inflight_periodic_sweep_removes_stale_marker",
                "inflight_auto_sweep_audit_failure_deletes_nothing",
            ],
            "自动路径不另写判据：startup / periodic 都直接调用 sweep_inflight_markers，复用 stale_rule（pid 存活优先于 deadlineAt，unreadable 永不删除）。默认开启每小时周期的理由：一次清扫只是只读扫描 .inflight 目录，成本低；只在启动时清扫会让长驻进程里的残标一直躺着。AW_INFLIGHT_SWEEP_SECS=0 退回仅启动时一次，off 关闭自动清扫，正整数改周期。与显式路径共用 runs/inflight-sweep.jsonl：plan+result 行都带 trigger，可用 startup|periodic|task_start|prune|clean 区分。审计写不进去（plan 行失败）时本轮零删除，自动触发同样适用。跨进程仍无事务，unreadable 标记仍不会自愈。",
        ),
        capability_entry(
            "inflight.clean_preserves_active_markers",
            Some(true),
            "proven",
            "pid 存活优先于 deadline：活跃标记永不进入可删集合",
            None,
            false,
            &both,
            &["macos"],
            &["runs_inflight_lists_and_cleans_only_stale_markers"],
            "删除前把标记原子改名为临时文件，认领后重跑 stale_rule 复核；已不 stale 或读不出来则 create-if-absent 还原保留，只删复核仍 stale 的那一份。",
        ),
        capability_entry(
            "lifecycle.startup_reclaims_abandoned_cgroup_scopes",
            Some(true),
            "proven",
            "仅 Linux：bridge 启动时回收上一轮猝死遗留的 cgroup 作用域。动手前必须同时满足三条证据：runs/.cgroup-scopes/<scope>.json 与本目录自洽、sidecar 点名的 runs/<task_id>/attempt-<n>/dispatch.json 存在、记录的 bridge 进程已死（starttime 不复用）。任一不满足只写 outcome=kept + reason 审计，不发任何信号",
            None,
            true,
            &["linux"],
            &["windows", "macos"],
            &[
                "cgroup_reclaim_keeps_scope_without_owner_record",
                "cgroup_reclaim_keeps_scope_when_owner_alive",
                "cgroup_reclaim_keeps_scope_without_attempt_evidence",
            ],
            "实跑：real-chain B5（bridge kill -9 → 孙进程留存 → 下一次启动后消失），审计 runs/cgroup-reclaim.jsonl 带 trigger=startup、scope、members_before/members_after。无 sidecar 的同类目录只报不删。边界：仅在启动时执行（AW_INFLIGHT_SWEEP_SECS=off 时关闭）；cgroup v2 不可用/只读时 state=unsupported，不宣称收敛；内核无 cgroup.kill 时逐成员 SIGKILL 并每轮重读 cgroup.procs，仍有 PID 复用窗口。Windows 无 cgroup v2：`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 会在桥句柄关闭时清理**作业内**成员，但没有可回收的作用域目录与 sidecar 语义（也不覆盖 BREAKAWAY 出去的进程），故不升级到 windows；macOS 未验证。审计 §56.2。",
        ),
        capability_entry(
            "cancel.cross_process_request_visible",
            Some(true),
            "proven",
            "task.cancel 跨进程可见：请求落盘 runs/.cancel/<task_id>.json，带 requested_by/requestedAt/target(pid+startedAt)；执行侧在心跳检查点读取，pending（已落盘未读）→ acknowledged（已读在清理）→ done（收尾完成）三态可区分。target 不符的上一轮残留请求不会误杀；没有有效活跃标记时不写请求、不发信号",
            None,
            true,
            &both,
            &["macos"],
            &[
                "cancel_request_three_states_are_distinguishable",
                "cancel_request_target_mismatch_is_not_applied",
                "cancel_request_cross_process_is_visible",
            ],
            "实跑：real-chain B6（两个 bridge 共用 runs，A 执行、B 取消），另一个进程的 task.cancel 不再返回 not_running，最终 status=canceled、request_state=done、cleanup_claimed=true；请求文件保留 requestedAt/acknowledgedAt/finishedAt 供判定中间态。等待上限 CANCEL_WAIT_SECS=10；超时仍 pending/acknowledged 时返回 cancel_requested 而不是 canceled。目标已死且过宽限的请求由启动清扫删除（先写 runs/cancel-requests.jsonl plan 行）。跨机器不可见，仅同卷多进程；macOS 未验证。",
        ),
    ];
    let not_proven_total = entries
        .iter()
        .filter(|entry| entry["holds"] == json!(false))
        .count() as u64;

    json!({
        "version": CAPABILITIES_VERSION,
        "platform": {
            "os": platform_os(),
            "arch": std::env::consts::ARCH
        },
        "status_enum": ["proven", "not_proven", "partial", "unverified", "supported"],
        "decision_rule": {
            "holds": {
                "true": "在 scope 与 platforms_verified 内保证成立",
                "false": "明确不成立或未证明，原因见 reason_code",
                "null": "本环境未验证"
            },
            "status": {
                "proven": "有可复现的测试证据",
                "not_proven": "无证据或已证明不成立",
                "partial": "仅部分平台或部分路径有证据",
                "unverified": "本环境未验证",
                "supported": "由本工具运行时提供，可查询或可执行"
            }
        },
        "entries_total": entries.len() as u64,
        "not_proven_total": not_proven_total,
        "entries": entries
    })
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
            "description": "只读汇总桌面端关键状态：配置文件是否就绪、AI 是否启用、启用模型数量、仓库路径是否配置，初始化审计（记录可能多份，按 receivedAt 聚合，顶层回显最近一条 clientInfo；仅审计弱识别，不构成身份），以及连接级计数 connections（unit=connection：进程内无会话概念，stdio 单连接故 active/total/peak 恒为 1，initialize_total 随 initialize 次数累加，多份 initialize 不等于多会话；仅观测，不做准入或上限）；契约版本 contract_version 为机器可读的对接版本（与文档标题一致），握手 serverInfo.version 只是二进制版本；不回显任何配置值",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "capabilities",
            "description": "机器可读的能力与限制声明（只读，无参数）：entries[] 逐条给出 id、holds（true/false/null）、status、scope、reason_code、platforms_verified、platforms_unverified、automatic、evidence、detail。holds=true 表示在 scope 内保证成立；holds=false 表示明确不成立或未证明，原因必须看 reason_code；holds=null 表示本环境未验证。status 取值 proven/not_proven/partial/unverified/supported。调用方按字段直接判定哪些保证成立、哪些不成立及边界，不依赖描述散文；stale 标记、TOCTOU、POSIX 覆盖等限制均以 reason_code 暴露",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "task.execute",
            "description": "接单执行，两种运行模式必须显式区分。dispatched（缺省，外部派单）：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）；AW 不定义判据、不做验收。standalone（mode=standalone，无外部派单方）：由 AW 自行闭环，必须显式给出 cwd/argv/acceptance，按每条验收标准逐 attempt 判定，把依据哪条标准、观测到什么、结论一起落盘 verdict.json，结论三态 passed/failed/undetermined；undetermined 即无法判定绝不默认通过并说明缺什么；只有 failed 且 attempt < max_attempts 才重试，每次尝试独立记录、不覆盖，耗尽写 retry_exhausted。两种模式互斥，同一 task_id 不得跨模式复用。超时只向受控集合（POSIX 进程组 / Windows Job Object）发送终止信号；终止只尝试一次（不重试），仅当观测到集合内无存活进程才写「已清理受控集合」，并注明逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建、先加入作业再恢复执行（恢复前不会派生子进程），但 BREAKAWAY 或加入作业失败的作业外进程仍不可观测。不承诺终止整棵进程树。执行期间可由 task.cancel 按 task_id 显式取消：取消生效时回执 status=canceled（与超时 blocked、执行失败 failed 区分），blocked_reason 以 canceled： 开头；取消请求在进程启动前到达则不启动任何进程",
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
            "name": "task.cancel",
            "description": "按 task_id 显式取消正在执行的任务，跨进程可见：请求先落盘 runs/.cancel/<task_id>.json（含发起者、时间、target=pid+startedAt），执行侧在心跳检查点读取后走与本地取消相同的终止流程。只影响 target 点名的本轮执行；target 不符的上一轮残留请求不会误杀。返回 status：canceled（回执已落盘且确为 canceled）、completed_before_cancel（取消到达前已结束，未终止任何进程）、cancel_requested（已落盘但尚未观测到 done；用 request_state 区分 pending=执行侧未读到 与 acknowledged=已读到正在清理）、not_running（没有仍有效的活跃标记：未写请求、未发送信号、未做清理）。只有 canceled 且受控集合观测到无存活（cleanup_status=cleaned）才允许 cleanup_claimed=true；响应另含 cleanup_status（cleaned / still_alive / unavailable / not_started / null）与完整 handback。取消的观测口径与超时一致：只有观测到受控集合内无存活进程才写「已清理受控集合」，逃逸进程是否仍在运行未验证；看不到的进程不会谎报已清理",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "要取消的派单标识（与 task.execute 同一命名规则：字母或数字开头，仅含字母数字 . _ -，≤64）" },
                    "reason": { "type": "string", "maxLength": 1024, "description": "可选：取消理由，写入回执 blocked_reason 的「请求方理由」并保留在取消请求与响应里；不做任何推断" },
                    "requested_by": { "type": "string", "maxLength": 128, "description": "可选：发起者标识，写入取消请求文件供审计；缺省为 pid:<当前进程>" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "receipt.get",
            "description": "按 task_id 读回执行回执：默认返回最新一次；include_history=true 时返回 attempts 次数与全部历史（每次 exit_code 与耗时）。取消或超时的回执含机器可读的 cleanup_status：cleaned / still_alive / unavailable / not_started / null（未发生终止动作）；只有 cleaned 才允许据此宣布受控集合已清理",
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
            "name": "runs.inflight",
            "description": "活跃标记（runs/.inflight/<task_id>.json）的只读查询与残留清理：action=list 返回每个标记的 pid、pid_alive、stale、stale_reason（pid_alive/pid_missing_grace/pid_missing/deadline_in_grace/deadline_expired/unreadable）、startedAt、deadlineAt、age_secs、deadline_overdue_secs、removable，并给出机器可读的 stale_rule 判据；action=clean 默认 dry_run=true 只报告 would_remove、零删除，真删必须 dry_run=false 与 confirm=true，只删 stale=true 的标记，pid 存活的活跃标记永不删除；删除前把标记原子改名为临时文件并复核 pid+startedAt 未变化，变化则还原并记 marker_changed；不可读标记保留并在 unreadable[] 报告；清理不触碰 attempt 目录，也不进入 runs.prune 候选；并给出 PM 判活约定 liveness_rule：判活只认 markers[].task_id + markers[].stale，pid/age_secs/stale_reason 仅作诊断，attempt 缺 handback/dispatch 启发式与裸看标记文件不得用于判活，fallback.allowed=false；清扫触发点：服务启动时一次、默认每 3600 秒一次（后台线程，周期默认开启；AW_INFLIGHT_SWEEP_SECS 可调：0=仅启动时一次、off=关闭自动清扫、正整数=周期秒数），以及既有动作路径（同 task_id 重启、prune 真删前、clean）；自动与显式路径共用同一 stale_rule 与 runs/inflight-sweep.jsonl（plan+result，含 trigger=startup|periodic|task_start|prune|clean 与 task_id/stale_reason），审计 plan 行写不进去则本轮零删除，无可清残标时不追加审计且重复触发幂等；list 的 sweep_rule 机器可读给出触发条件、周期开关、永不删除项（活跃/不可读）与跨进程边界",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "clean"], "description": "必填：list 只读列出标记；clean 清理 stale=true 的残留标记" },
                    "task_id": { "type": "string", "description": "可选：只处理该 task_id 的标记；须与任务目录同一命名校验（仅字母数字与 . _ -，且不以 _ 或 . 开头）" },
                    "dry_run": { "type": "boolean", "description": "clean 默认 true：只报告 would_remove、不删除；false 才真删（还需 confirm=true）" },
                    "confirm": { "type": "boolean", "description": "clean 真删必须显式 true；单独 true 不隐含真删" }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        },
        {
            "name": "runs.prune",
            "description": "显式证据清理：默认 dry_run=true 只列出候选、零写入；真删必须同时 dry_run=false 与 confirm=true。只删非最新 attempt 的整个目录，最新 attempt 永不进入候选；年龄只认 handback 的 finishedAt（缺字段一律不删）；`status` 是可选的状态多值白名单，显式传入才生效，缺省不过滤、无默认状态、无服务端分档，与 older_than_days 为 AND；`exclude` 是可选按条目精确排除（按 task_id，并可精确到某次 attempt），在候选枚举入口生效：命中项只进 excluded[]、不进 candidates[]，PM 可用它把「不可自动判定」的条目留在档内而批量处理其余条目。计数不变式：deleted + failed + trashed + lost_race + excluded_total == batch_candidates_total，且 candidates_total == batch_candidates_total + not_processed_total（limit 截断部分），回执可直接核对。lost_race 表示同一 attempt 已被另一并发清扫者先认领/删除（rename 缺失源或目的目录不存在），单列 lost_race[] 并计入 lost_race_total，不计入 failed_total、不改变 status（仅 lost_race 时仍为 ok）。exclude 的 task_id 与任务目录同一命名校验，越界一律 invalid_exclusion；排除不存在或本就不在候选集的 id 记 not_in_candidates（不报错、不静默忽略）。删除前先写 runs/prune.jsonl 记账；部分失败如实报 partial 且 isError=true。真删（dry_run=false+confirm=true）时会在枚举候选前先按既有 inflight 判据顺带清残标并写 runs/inflight-sweep.jsonl，dry_run 只报告 would-remove、零删除；清扫审计失败则不删任何目录。服务启动时一次与默认每小时的后台清扫也写同一审计（trigger=startup|periodic），prune 内清扫的 trigger=prune",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "older_than_days": { "type": "integer", "minimum": 1, "maximum": 3650, "description": "必填：只清理 finishedAt 距今不少于该天数的 attempt。PM 口径统一推荐 365 天；仍须显式传入，不提供隐式默认阈值，缺省按 invalid_retention 拒绝" },
                    "status": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "type": "string", "enum": ["ok", "failed", "blocked", "canceled"] }, "description": "可选：状态多值白名单，显式传入才生效；缺省不过滤（与旧行为一致）。仅 handback.status 精确匹配才进入候选，与 older_than_days 为 AND；空数组/null/未知值按 invalid_retention 拒绝；无默认状态、无服务端分档" },
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

/// v1.36 并发观测点：清扫者「rename 认领」之后若发现内容已变，
/// 还原路径不得覆盖此刻已存在的新标记（创建者已合法占住该文件名）。
///
/// v1.43 并发观测点：多个清扫者抢同一个残标只允许一方真正删除，其余记 lost_race；
/// 「正在被刷新」的标记在并发清扫下必须零误删（认领后重跑 stale_rule）。
#[cfg(test)]
mod inflight_sweep_race_tests {
    use super::*;
    use std::sync::{Arc, Barrier, Mutex};

    /// rename 认领前的观测点是进程级全局状态：所有会武装它的用例必须串行。
    static RACE_HOOK_LOCK: Mutex<()> = Mutex::new(());

    /// 多清扫者对齐点（扫描判定之后、rename 认领之前）
    static CLAIM_BARRIER: Mutex<Option<Arc<Barrier>>> = Mutex::new(None);
    /// 「刷新已落盘」对齐点：确保所有清扫者都在刷新之后才 rename
    static REFRESH_BARRIER: Mutex<Option<Arc<Barrier>>> = Mutex::new(None);

    fn wait_on(slot: &Mutex<Option<Arc<Barrier>>>) {
        let barrier = slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if let Some(barrier) = barrier {
            barrier.wait();
        }
    }

    fn align_claimers(_path: &Path) {
        wait_on(&CLAIM_BARRIER);
    }

    fn wait_refresh_then_claim(_path: &Path) {
        wait_on(&CLAIM_BARRIER);
        wait_on(&REFRESH_BARRIER);
    }

    fn reset_race_barriers() {
        *CLAIM_BARRIER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        *REFRESH_BARRIER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

    fn marker_record(started_at: &str) -> Value {
        json!({
            "version": "inflight-marker/1",
            "task_id": "race",
            "pid": std::process::id(),
            "mode": "dispatched",
            "state": "active",
            "startedAt": started_at,
            "deadlineAt": "2099-01-01T01:00:00+00:00"
        })
    }

    /// 无 pid、deadline 早已过期：按 stale_rule 必然 stale（与平台无关）
    fn stale_record() -> Value {
        json!({
            "version": "inflight-marker/1",
            "task_id": "race",
            "mode": "dispatched",
            "state": "active",
            "startedAt": "2000-01-01T00:00:00+00:00",
            "deadlineAt": "2000-01-01T01:00:00+00:00"
        })
    }

    /// 刷新者写入的新标记：task_id / pid / startedAt 与残标完全一致，只延长 deadlineAt。
    /// 这正是「按身份三元组复核」看不出来、而重跑 stale_rule 才能识别的窗口。
    fn refreshed_record() -> Value {
        json!({
            "version": "inflight-marker/1",
            "task_id": "race",
            "mode": "dispatched",
            "state": "active",
            "startedAt": "2000-01-01T00:00:00+00:00",
            "deadlineAt": "2099-01-01T01:00:00+00:00"
        })
    }

    /// 复核时刻的真实文件状态（两侧都由别的进程写入）：
    /// - 副本里是 W1——在「检查」之后、「认领」之前替换进来的活跃标记（被 rename 带走了）；
    /// - 原名位置是 W2——在「认领」之后、「复核」之前新建的活跃标记。
    fn two_writers_interleaved(path: &Path, claimed: &Path) {
        fs::write(
            claimed,
            serde_json::to_string(&marker_record("2098-01-01T00:00:00+00:00")).expect("序列化 W1"),
        )
        .expect("构造被认领副本里的 W1");
        fs::write(
            path,
            serde_json::to_string(&marker_record("2099-01-01T00:00:00+00:00")).expect("序列化 W2"),
        )
        .expect("并发写入者必须能在空出的文件名上创建 W2");
    }

    /// 模拟「复核之后、删除之前」认领副本被另一方处置（Windows 并发 rename 的真实窗口）。
    fn drop_claim_copy(_path: &Path, claimed: &Path) {
        fs::remove_file(claimed).expect("模拟另一方在复核后处置认领副本");
    }

    struct RaceFixture {
        root: PathBuf,
        runs: PathBuf,
        dir: PathBuf,
        marker: PathBuf,
    }

    impl RaceFixture {
        fn new(tag: &str) -> Self {
            let unique = format!(
                "aw-sweep-race-{tag}-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            let root = std::env::temp_dir().join(unique);
            let runs = root.join("runs");
            let dir = runs.join(INFLIGHT_DIR);
            fs::create_dir_all(&dir).expect("创建 .inflight");
            Self {
                root,
                runs,
                marker: dir.join("race.json"),
                dir,
            }
        }

        fn write_marker(&self, record: &Value) {
            write_json_file(&self.marker, record).expect("写入标记");
        }

        fn server(&self) -> Server {
            Server::with_runs_dir(self.root.join("settings.json"), self.runs.clone())
        }

        fn tmp_files(&self) -> usize {
            fs::read_dir(&self.dir)
                .expect("读取 .inflight")
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count()
        }

        fn audit_text(&self) -> String {
            fs::read_to_string(self.runs.join(INFLIGHT_SWEEP_AUDIT)).unwrap_or_default()
        }

        fn cleanup(self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn sweep_concurrently(fixture: &RaceFixture, sweepers: usize) -> Vec<Value> {
        let handles: Vec<_> = (0..sweepers)
            .map(|_| {
                let server = fixture.server();
                std::thread::spawn(move || server.sweep_inflight_markers("clean", None, false))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .expect("清扫线程不得 panic")
                    .expect("清扫必须返回结果")
            })
            .collect()
    }

    /// 失败时一次性给出「当时的进程/文件状态 + 并发数 + 谁在什么时候做了什么」。
    /// 只读现场，不改变任何判定。
    fn sweep_diagnostics(fixture: &RaceFixture, sweepers: usize, results: &[Value]) -> String {
        let mut dir_entries = Vec::new();
        if let Ok(entries) = fs::read_dir(&fixture.dir) {
            for entry in entries.flatten() {
                let meta = entry.metadata().ok();
                dir_entries.push(format!(
                    "{} len={:?} modified={:?}",
                    entry.file_name().to_string_lossy(),
                    meta.as_ref().map(|meta| meta.len()),
                    meta.and_then(|meta| meta.modified().ok())
                ));
            }
        }
        dir_entries.sort();
        format!(
            "并发数: sweepers={sweepers} cpu={:?}\n\
             标记: exists={} len={:?}\n\
             .inflight 目录:\n{}\n\
             各清扫者结果:\n{}\n\
             审计:\n{}\n\
             时序（相对毫秒 / 线程 id / 动作）:\n{}",
            std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(0),
            fixture.marker.exists(),
            fs::metadata(&fixture.marker).ok().map(|meta| meta.len()),
            if dir_entries.is_empty() {
                "(空)".to_string()
            } else {
                dir_entries.join("\n")
            },
            results
                .iter()
                .enumerate()
                .map(|(index, result)| format!("#{index}: {result}"))
                .collect::<Vec<_>>()
                .join("\n"),
            fixture.audit_text(),
            sweep_trace::dump(&fixture.marker),
        )
    }

    #[test]
    fn sweep_restore_never_clobbers_concurrent_new_marker() {
        let _serial = RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let fixture = RaceFixture::new("restore");
        fixture.write_marker(&stale_record());
        let marker = fixture.marker.clone();
        let dir = fixture.dir.clone();

        let server = fixture.server();
        set_sweep_after_claim_hook(&marker, Some(two_writers_interleaved));
        let sweep = server.sweep_inflight_markers("clean", None, false);
        set_sweep_after_claim_hook(&marker, None);
        let sweep = sweep.expect("清扫必须返回结果");

        let after = read_json(&marker).expect("并发写入者的 W2 必须仍在原位");
        assert_eq!(
            after["startedAt"],
            json!("2099-01-01T00:00:00+00:00"),
            "还原不得覆盖并发写入的 W2（覆盖后会是 W1 的时间戳）：{after}"
        );
        assert_eq!(
            after["pid"],
            json!(std::process::id()),
            "原位必须是 W2 的完整记录：{after}"
        );
        assert_eq!(
            sweep["removed_total"].as_u64(),
            Some(0),
            "被并发替换的标记不得计入已删：{sweep}"
        );
        assert!(
            sweep["kept"].to_string().contains("marker_changed")
                || sweep["kept"].to_string().contains("marker_superseded"),
            "必须具名记录复核不一致：{sweep}"
        );
        assert_eq!(
            fs::read_dir(&dir)
                .expect("读取 .inflight")
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count(),
            0,
            "不得留下 .tmp 认领副本"
        );

        fixture.cleanup();
    }

    /// 红→绿：N 个清扫者同时抢同一个残标。原子 rename 只有一个赢家，
    /// 其余必须记 lost_race（而不是错误，也不是「消失」含混过关）。
    #[test]
    fn sweep_concurrent_claimers_delete_once_others_lost_race() {
        let _serial = RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        const SWEEPERS: usize = 4;
        let fixture = RaceFixture::new("claim");
        fixture.write_marker(&stale_record());

        *CLAIM_BARRIER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(Arc::new(Barrier::new(SWEEPERS)));
        sweep_trace::reset(&fixture.marker);
        set_sweep_before_claim_hook(&fixture.marker, Some(align_claimers));
        let results = sweep_concurrently(&fixture, SWEEPERS);
        set_sweep_before_claim_hook(&fixture.marker, None);
        reset_race_barriers();

        let diag = sweep_diagnostics(&fixture, SWEEPERS, &results);
        println!("[evidence][sweep-race] 现场快照\n{diag}");

        let deleted: u64 = results
            .iter()
            .map(|result| result["deleted_total"].as_u64().unwrap_or(0))
            .sum();
        let lost_race: u64 = results
            .iter()
            .map(|result| result["lost_race_total"].as_u64().unwrap_or(0))
            .sum();
        assert_eq!(
            deleted, 1,
            "同一个残标只允许一方真正删除，实际 {deleted}：\n{diag}"
        );
        assert_eq!(
            lost_race,
            (SWEEPERS - 1) as u64,
            "其余清扫者必须记 lost_race，实际 {lost_race}：\n{diag}"
        );
        assert!(
            !fixture.marker.exists(),
            "赢家必须已经把残标删掉，而不是留在原位：\n{diag}"
        );
        assert_eq!(fixture.tmp_files(), 0, "不得留下 .tmp 认领副本：\n{diag}");

        let audit = fixture.audit_text();
        assert!(
            audit.contains("\"outcome\":\"deleted\""),
            "审计必须能区分 deleted：{audit}\n{diag}"
        );
        assert!(
            audit.contains("\"outcome\":\"lost_race\""),
            "审计必须能区分 lost_race：{audit}\n{diag}"
        );

        fixture.cleanup();
    }

    /// 红→绿（v1.49）：复核之后认领副本消失。Windows 上并发 rename 可能都返回成功，
    /// 但只有一个目的地真的产生文件；另一个清扫者可能已处置同名副本。这属于「别人先认领」，
    /// 必须记 lost_race，不得记 failed——否则审计出现假失败，门禁随机红。
    #[test]
    fn sweep_claim_copy_vanishing_after_reread_is_lost_race_not_failure() {
        let _serial = RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let fixture = RaceFixture::new("vanish");
        fixture.write_marker(&stale_record());

        sweep_trace::reset(&fixture.marker);
        set_sweep_after_reread_hook(&fixture.marker, Some(drop_claim_copy));
        let sweep = fixture
            .server()
            .sweep_inflight_markers("clean", None, false)
            .expect("清扫必须返回结果");
        set_sweep_after_reread_hook(&fixture.marker, None);

        let diag = format!(
            "{sweep}\n审计:\n{}\n时序:\n{}",
            fixture.audit_text(),
            sweep_trace::dump(&fixture.marker)
        );
        println!("[evidence][sweep-vanish] {diag}");

        assert_eq!(
            sweep["failed_total"],
            json!(0),
            "认领副本消失不是错误，不得记 failed：\n{diag}"
        );
        assert_eq!(
            sweep["deleted_total"],
            json!(0),
            "本清扫者没有真正删除任何东西：\n{diag}"
        );
        assert_eq!(
            sweep["lost_race_total"],
            json!(1),
            "必须记 lost_race：\n{diag}"
        );
        assert!(
            sweep["kept"].to_string().contains("claim_copy_gone"),
            "必须具名记录 claim_copy_gone：\n{diag}"
        );
        assert_eq!(fixture.tmp_files(), 0, "不得留下 .tmp 认领副本：\n{diag}");

        fixture.cleanup();
    }

    /// 红→绿：残标在「扫描判定」之后被别的进程刷新（身份三元组不变，只延长 deadline），
    /// 并发清扫者必须零误删：认领后重跑 stale_rule 发现已不 stale，就地还原并记 not_mine。
    #[test]
    fn sweep_concurrent_refresher_never_deletes_fresh_marker() {
        let _serial = RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        const SWEEPERS: usize = 3;
        let fixture = RaceFixture::new("refresh");
        fixture.write_marker(&stale_record());

        let claim_barrier = Arc::new(Barrier::new(SWEEPERS + 1));
        let refresh_barrier = Arc::new(Barrier::new(SWEEPERS + 1));
        *CLAIM_BARRIER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(claim_barrier.clone());
        *REFRESH_BARRIER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(refresh_barrier.clone());
        set_sweep_before_claim_hook(&fixture.marker, Some(wait_refresh_then_claim));

        let refresher = {
            let marker = fixture.marker.clone();
            let dir = fixture.dir.clone();
            let claim_barrier = claim_barrier.clone();
            let refresh_barrier = refresh_barrier.clone();
            std::thread::spawn(move || {
                claim_barrier.wait();
                // 刷新者用「同目录临时文件 + rename」原子替换，绝不部分写入
                let staging = dir.join("refresher-staging.tmp");
                fs::write(
                    &staging,
                    serde_json::to_string(&refreshed_record()).expect("序列化刷新记录"),
                )
                .expect("写入刷新副本");
                fs::rename(&staging, &marker).expect("刷新者必须能原子替换标记");
                refresh_barrier.wait();
            })
        };
        let results = sweep_concurrently(&fixture, SWEEPERS);
        refresher.join().expect("刷新线程不得 panic");
        set_sweep_before_claim_hook(&fixture.marker, None);
        reset_race_barriers();

        let deleted: u64 = results
            .iter()
            .map(|result| result["deleted_total"].as_u64().unwrap_or(0))
            .sum();
        let not_mine: u64 = results
            .iter()
            .map(|result| result["not_mine_total"].as_u64().unwrap_or(0))
            .sum();
        assert_eq!(
            deleted, 0,
            "正在被刷新的标记不得被任何清扫者删除，实际删了 {deleted}：{results:?}"
        );
        assert!(
            not_mine >= 1,
            "认领后必须能识别出「已不是我的」，实际 {not_mine}：{results:?}"
        );

        let after = read_json(&fixture.marker).expect("刷新后的标记必须仍在原位");
        assert_eq!(
            after["deadlineAt"],
            json!("2099-01-01T01:00:00+00:00"),
            "原位必须是刷新后的内容，不能被残标副本覆盖：{after}"
        );
        assert_eq!(
            after["startedAt"],
            json!("2000-01-01T00:00:00+00:00"),
            "刷新未改身份字段：{after}"
        );
        assert_eq!(fixture.tmp_files(), 0, "不得留下 .tmp 认领副本");

        let audit = fixture.audit_text();
        assert!(
            audit.contains("\"outcome\":\"not_mine\""),
            "审计必须能区分 not_mine：{audit}"
        );

        fixture.cleanup();
    }
}

/// 清理宣称的代码边界：`cleanup_claimed` 只能由 canceled + cleaned 推出。
#[cfg(test)]
mod cleanup_claim_tests {
    use super::*;
    use process_scope::CleanupOutcome;

    #[test]
    fn cleanup_status_labels_cover_every_outcome() {
        assert_eq!(cleanup_status_label(true, false, Some(CleanupOutcome::Cleaned)), Some("cleaned"));
        assert_eq!(
            cleanup_status_label(true, false, Some(CleanupOutcome::StillAlive)),
            Some("still_alive")
        );
        assert_eq!(
            cleanup_status_label(false, true, Some(CleanupOutcome::Unavailable)),
            Some("unavailable")
        );
        assert_eq!(cleanup_status_label(true, false, None), Some("not_started"));
        assert_eq!(cleanup_status_label(false, false, None), None);
        assert_eq!(cleanup_status_label(false, false, Some(CleanupOutcome::Cleaned)), None);
    }

    #[test]
    fn cleanup_claim_requires_canceled_and_cleaned() {
        assert!(cleanup_claim_allowed(Some("canceled"), Some("cleaned")));
        assert!(!cleanup_claim_allowed(Some("canceled"), Some("still_alive")));
        assert!(!cleanup_claim_allowed(Some("canceled"), Some("unavailable")));
        assert!(!cleanup_claim_allowed(Some("canceled"), Some("not_started")));
        assert!(!cleanup_claim_allowed(Some("canceled"), None));
        assert!(!cleanup_claim_allowed(Some("ok"), Some("cleaned")));
        assert!(!cleanup_claim_allowed(None, Some("cleaned")));
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
            for phrase in row["contains"]
                .as_array()
                .unwrap_or_else(|| panic!("{id} 缺少 contains 数组"))
            {
                let phrase = phrase
                    .as_str()
                    .unwrap_or_else(|| panic!("{id} 的 contains 必须是字符串"));
                assert!(
                    text.contains(phrase),
                    "{id}：{name} 的 {path} 缺少必需措辞「{phrase}」：{text}"
                );
            }
            for phrase in row["not_contains"]
                .as_array()
                .unwrap_or_else(|| panic!("{id} 缺少 not_contains 数组"))
            {
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
        assert!(cleaned.contains("逃逸进程是否仍在运行未验证"), "{cleaned}");
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
            for forbidden in ["已终止进程树", "无残留", "会终止整棵进程树", "保证清理全部"]
            {
                assert!(
                    !reason.contains(forbidden),
                    "超时文案不得含「{forbidden}」：{reason}"
                );
            }
        }
    }
}

/// rename 失败的归属判定：`NotFound` 或源已不在，都表示源被另一方认领带走。
/// Windows 上并发 rename 的败者常拿到 `AccessDenied` / `SharingViolation`，
/// 只看错误类型会把「源已被搬走」误记成「原样未动」。
fn rename_failure_is_lost_race(error: &std::io::Error, source_still_present: bool) -> bool {
    error.kind() == std::io::ErrorKind::NotFound || !source_still_present
}

/// prune 认领竞态用例：验证「rename 返回 Ok」不被当成归属依据。
#[cfg(test)]
mod prune_claim_race_tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn outcome_state(outcome: &DeleteOutcome) -> &'static str {
        match outcome {
            DeleteOutcome::Deleted { .. } => "deleted",
            DeleteOutcome::RetainedAtSource { .. } => "retained_at_source",
            DeleteOutcome::RetainedInTrash { .. } => "trashed_not_deleted",
            DeleteOutcome::LostRace { .. } => "lost_race",
        }
    }

    /// 失败证据用：带上错误原文与暂存路径，避免只看到 "retained_at_source" 三个字。
    fn describe_outcome(outcome: &DeleteOutcome) -> String {
        match outcome {
            DeleteOutcome::Deleted { trash_path } => format!("deleted:{trash_path}"),
            DeleteOutcome::RetainedAtSource { error } => format!("retained_at_source:{error}"),
            DeleteOutcome::RetainedInTrash { trash_path, error } => {
                format!("trashed_not_deleted:{trash_path}:{error}")
            }
            DeleteOutcome::LostRace { error } => format!("lost_race:{error}"),
        }
    }

    /// 模拟 Windows 实测：rename 返回 Ok，但目的目录并不存在（被同源的另一方认领）。
    fn simulate_peer_won(_source: &Path, staged: &Path) {
        let _ = fs::remove_dir_all(staged);
    }

    /// 两个认领者在 rename 前对齐，稳定复现「同一源、同时 rename」。
    fn wait_for_peer(_source: &Path, _staged: &Path) {
        let barrier = {
            let armed = PRUNE_BEFORE_RENAME_BARRIER
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            armed.clone()
        };
        if let Some(barrier) = barrier {
            barrier.wait();
        }
    }

    struct PruneFixture {
        root: PathBuf,
        runs: PathBuf,
        attempt: PathBuf,
    }

    impl PruneFixture {
        fn new(tag: &str) -> Self {
            let unique = format!(
                "aw-prune-race-{tag}-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            let root = std::env::temp_dir().join(unique);
            let runs = root.join("runs");
            let attempt = runs.join("t").join("attempt-1");
            fs::create_dir_all(&attempt).expect("创建 attempt 目录");
            fs::write(attempt.join("payload.txt"), b"evidence").expect("写入 attempt 内容");
            Self {
                root,
                runs,
                attempt,
            }
        }

        fn server(&self) -> Server {
            Server::with_runs_dir(self.root.join("settings.json"), self.runs.clone())
        }

        fn cleanup(self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn prune_claim_lost_is_named_not_trashed() {
        let _serial = PRUNE_RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let fixture = PruneFixture::new("missing-dest");
        let server = fixture.server();
        set_prune_after_rename_hook(&fixture.attempt, Some(simulate_peer_won));
        let outcome = server.delete_attempt_dir("t", 1, "batch-missing");
        set_prune_after_rename_hook(&fixture.attempt, None);

        let state = outcome_state(&outcome);
        assert!(
            !fixture.attempt.exists(),
            "源目录必须已经被 rename 带走：{state}"
        );
        assert_ne!(
            state, "trashed_not_deleted",
            "目的目录不存在不是「删除暂存失败」：{state}"
        );
        assert_ne!(
            state, "retained_at_source",
            "源已被搬走，不能报「原样未动」：{state}"
        );
        assert_eq!(
            state, "lost_race",
            "目的不存在必须具名记为 lost_race：{state}"
        );
        fixture.cleanup();
    }

    #[test]
    fn prune_concurrent_claimers_delete_once_others_lost_race() {
        let _serial = PRUNE_RACE_HOOK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rounds = 20usize;
        let mut deleted_rounds = 0usize;
        let mut lost_rounds = 0usize;
        let mut other_rounds = 0usize;

        for round in 0..rounds {
            let fixture = PruneFixture::new(&format!("round-{round}"));
            let barrier = Arc::new(Barrier::new(2));
            *PRUNE_BEFORE_RENAME_BARRIER
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(barrier);
            set_prune_before_rename_hook(&fixture.attempt, Some(wait_for_peer));

            let handles: Vec<_> = (0..2)
                .map(|index| {
                    let server = fixture.server();
                    let batch = format!("round-{round}-{index}");
                    std::thread::spawn(move || server.delete_attempt_dir("t", 1, &batch))
                })
                .collect();
            let outcomes: Vec<String> = handles
                .into_iter()
                .map(|handle| {
                    let outcome = handle.join().expect("认领线程不得 panic");
                    describe_outcome(&outcome)
                })
                .collect();
            let states: Vec<String> = outcomes
                .iter()
                .map(|text| {
                    text.split(':')
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                })
                .collect();

            set_prune_before_rename_hook(&fixture.attempt, None);
            *PRUNE_BEFORE_RENAME_BARRIER
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;

            let deleted = states.iter().filter(|state| state.as_str() == "deleted").count();
            assert_eq!(
                deleted, 1,
                "第 {round} 轮必须恰好一方真删，实际 {outcomes:?}"
            );
            let loser = states
                .iter()
                .find(|state| state.as_str() != "deleted")
                .expect("必须存在未删除的一方");
            assert_eq!(
                loser.as_str(), "lost_race",
                "第 {round} 轮败者必须具名记为 lost_race：{outcomes:?}"
            );
            assert!(
                !fixture.attempt.exists(),
                "第 {round} 轮源目录必须已被搬走：{outcomes:?}"
            );
            for index in 0..2 {
                let stage_root = fixture
                    .runs
                    .join(".prune-trash")
                    .join(format!("round-{round}-{index}"));
                assert!(
                    !stage_root.exists(),
                    "第 {round} 轮不得留下半截暂存目录 {}：{states:?}",
                    stage_root.display()
                );
            }

            match loser.as_str() {
                "lost_race" => lost_rounds += 1,
                _ => other_rounds += 1,
            }
            deleted_rounds += deleted;
            fixture.cleanup();
        }

        println!(
            "[evidence] prune_claim_race rounds={rounds} deleted={deleted_rounds} \
             lost_race={lost_rounds} other={other_rounds}"
        );
        assert_eq!(deleted_rounds, rounds, "每轮都必须有一方真删");
        assert_eq!(other_rounds, 0, "败者只能是 lost_race");
    }

    /// 固定 §55.10 的修复口径：非 `NotFound` 的 rename 失败，只要源已不在，
    /// 归属仍是 `lost_race`，不得报成 `retained_at_source`。
    #[test]
    fn rename_failure_treats_missing_source_as_lost_race() {
        let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
        // Windows 的 ERROR_ACCESS_DENIED / ERROR_SHARING_VIOLATION 都映射成非 NotFound。
        let access_denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "AccessDenied");
        let sharing_violation = std::io::Error::new(std::io::ErrorKind::Other, "SharingViolation");

        assert!(rename_failure_is_lost_race(&not_found, true), "NotFound 必须判 lost_race");
        assert!(rename_failure_is_lost_race(&not_found, false), "NotFound + 源缺失必须判 lost_race");
        assert!(
            rename_failure_is_lost_race(&access_denied, false),
            "AccessDenied 且源已被搬走时必须判 lost_race"
        );
        assert!(
            rename_failure_is_lost_race(&sharing_violation, false),
            "SharingViolation 且源已被搬走时必须判 lost_race"
        );
        // 对照组：源仍在就是真的「原样未动」。
        assert!(
            !rename_failure_is_lost_race(&access_denied, true),
            "源仍存在时不得记 lost_race"
        );
        assert!(
            !rename_failure_is_lost_race(&sharing_violation, true),
            "源仍存在时不得记 lost_race"
        );
    }
}

#[cfg(test)]
mod v148_tests {
    use super::*;

    fn test_server() -> (Server, tempfile::TempDir, tempfile::TempDir) {
        let settings = tempfile::TempDir::new().expect("settings");
        let runs = tempfile::TempDir::new().expect("runs");
        let server = Server::with_runs_dir(
            settings.path().join("settings.json"),
            runs.path().to_path_buf(),
        );
        (server, settings, runs)
    }

    #[cfg(target_os = "linux")]
    fn read_jsonl(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .map(|text| {
                text.lines()
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 三态必须是三个可判定的磁盘状态，而不是一个 cancel_requested 糊过去。
    #[test]
    fn cancel_request_three_states_are_distinguishable() {
        let (_server, _settings, runs) = test_server();
        let task_id = "v148-three-states";
        let path = runs.path().join(CANCEL_DIR).join(format!("{task_id}.json"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let started_at = "start-token-1";
        write_json_atomic(
            &path,
            &json!({
                "version": CANCEL_VERSION,
                "task_id": task_id,
                "requestedAt": now_stamp(),
                "requested_by": "test",
                "reason": "三态",
                "target": { "pid": std::process::id(), "startedAt": started_at },
                "status": "pending",
                "acknowledgedAt": Value::Null,
                "handledBy": Value::Null,
                "handback_status": Value::Null,
                "finishedAt": Value::Null
            }),
        )
        .unwrap();
        let pending = read_json(&path).unwrap();
        assert_eq!(pending["status"], "pending");
        assert!(pending["acknowledgedAt"].is_null());

        let cancel = CancelToken::new();
        let mut heartbeat = CancelHeartbeat::new(runs.path(), task_id, started_at);
        heartbeat.poll(&cancel);
        let acknowledged = read_json(&path).unwrap();
        assert_eq!(acknowledged["status"], "acknowledged");
        assert!(acknowledged["acknowledgedAt"].is_string());
        assert_eq!(acknowledged["handledBy"]["pid"], json!(std::process::id()));
        assert!(cancel.is_canceled(), "acknowledged 后必须置位取消");

        heartbeat.finish(Some("canceled"), Some("cleaned"));
        let done = read_json(&path).unwrap();
        assert_eq!(done["status"], "done");
        assert_eq!(done["handback_status"], "canceled");
        assert_eq!(done["cleanup_claimed"], json!(true));
        assert_eq!(done["cleanup_status"], "cleaned");
        assert!(done["finishedAt"].is_string());
        let parse = |value: &Value| {
            chrono::DateTime::parse_from_rfc3339(value.as_str().unwrap())
                .unwrap()
                .with_timezone(&chrono::Utc)
        };
        assert!(parse(&done["requestedAt"]) <= parse(&done["acknowledgedAt"]));
        assert!(parse(&done["acknowledgedAt"]) <= parse(&done["finishedAt"]));
    }

    /// target 不符的请求（上一轮残留）不得被本轮执行误认成取消。
    #[test]
    fn cancel_request_target_mismatch_is_not_applied() {
        let (_server, _settings, runs) = test_server();
        let task_id = "v148-target-mismatch";
        let path = runs.path().join(CANCEL_DIR).join(format!("{task_id}.json"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_json_atomic(
            &path,
            &json!({
                "version": CANCEL_VERSION,
                "task_id": task_id,
                "requestedAt": now_stamp(),
                "requested_by": "old-run",
                "reason": "旧请求",
                "target": { "pid": std::process::id(), "startedAt": "start-token-old" },
                "status": "pending"
            }),
        )
        .unwrap();
        let cancel = CancelToken::new();
        let mut heartbeat = CancelHeartbeat::new(runs.path(), task_id, "start-token-new");
        heartbeat.poll(&cancel);
        assert!(!cancel.is_canceled(), "target 不符不得触发取消");
        let record = read_json(&path).unwrap();
        assert_eq!(record["status"], "pending", "非目标进程不得改写请求状态");
    }

    #[cfg(target_os = "linux")]
    fn fake_scope(base: &Path, name: &str) -> PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cgroup.procs"), "").unwrap();
        fs::write(dir.join("cgroup.events"), "populated 0\n").unwrap();
        dir
    }

    #[cfg(target_os = "linux")]
    fn owner_record(
        runs: &Path,
        scope: &str,
        task_id: &str,
        bridge_pid: u32,
        bridge_start: Option<u64>,
    ) -> PathBuf {
        let dir = runs.join(SCOPE_OWNER_DIR);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{scope}.json"));
        let base = Path::new("/definitely-not-used");
        write_json_atomic(
            &path,
            &json!({
                "version": SCOPE_OWNER_VERSION,
                "scope": scope,
                "cgroup": base.join(scope).to_string_lossy(),
                "bridge_pid": bridge_pid,
                "bridge_start": bridge_start,
                "task_id": task_id,
                "attempt": 1,
                "root_pid": 1,
                "root_start": null,
                "createdAt": now_stamp()
            }),
        )
        .unwrap();
        path
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_reclaim_keeps_scope_without_owner_record() {
        let (server, _settings, runs) = test_server();
        let base = tempfile::TempDir::new().unwrap();
        fs::write(base.path().join("cgroup.controllers"), "cpu\n").unwrap();
        let scope = "aw-mcp-bridge-424242-0";
        let dir = fake_scope(base.path(), scope);
        let report = server
            .reclaim_stale_cgroup_scopes_at(base.path(), "startup")
            .unwrap();
        assert_eq!(report["kept_total"].as_u64(), Some(1));
        assert_eq!(report["reclaimed_total"].as_u64(), Some(0));
        assert!(dir.is_dir(), "证据不足不得删目录");
        let rows = read_jsonl(&runs.path().join(CGROUP_RECLAIM_AUDIT));
        assert!(
            rows.iter().any(|row| row["scope"] == scope
                && row["outcome"] == "kept"
                && row["reason"] == "owner_record_missing"),
            "必须写明为什么不动：{rows:?}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_reclaim_keeps_scope_when_owner_alive() {
        let (server, _settings, runs) = test_server();
        let base = tempfile::TempDir::new().unwrap();
        fs::write(base.path().join("cgroup.controllers"), "cpu\n").unwrap();
        let scope = "aw-mcp-bridge-424243-0";
        let dir = fake_scope(base.path(), scope);
        let task_id = "v148-owner-alive";
        fs::create_dir_all(runs.path().join(task_id).join("attempt-1")).unwrap();
        fs::write(
            runs.path()
                .join(task_id)
                .join("attempt-1")
                .join("dispatch.json"),
            "{}",
        )
        .unwrap();
        let path = owner_record(
            runs.path(),
            scope,
            task_id,
            std::process::id(),
            process_scope::proc_start(std::process::id()),
        );
        // 修正 cgroup 字段与临时 base 自洽
        let mut record = read_json(&path).unwrap();
        record["cgroup"] = json!(dir.to_string_lossy());
        write_json_atomic(&path, &record).unwrap();
        let report = server
            .reclaim_stale_cgroup_scopes_at(base.path(), "startup")
            .unwrap();
        assert_eq!(report["kept_total"].as_u64(), Some(1));
        assert!(dir.is_dir(), "仍在运行的 bridge 的作用域绝不动");
        let rows = read_jsonl(&runs.path().join(CGROUP_RECLAIM_AUDIT));
        assert!(rows
            .iter()
            .any(|row| row["scope"] == scope && row["reason"] == "owner_alive"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_reclaim_keeps_scope_without_attempt_evidence() {
        let (server, _settings, runs) = test_server();
        let base = tempfile::TempDir::new().unwrap();
        fs::write(base.path().join("cgroup.controllers"), "cpu\n").unwrap();
        let scope = "aw-mcp-bridge-424244-0";
        let dir = fake_scope(base.path(), scope);
        let path = owner_record(runs.path(), scope, "v148-missing-attempt", 424244, None);
        let mut record = read_json(&path).unwrap();
        record["cgroup"] = json!(dir.to_string_lossy());
        write_json_atomic(&path, &record).unwrap();
        let report = server
            .reclaim_stale_cgroup_scopes_at(base.path(), "startup")
            .unwrap();
        assert_eq!(report["kept_total"].as_u64(), Some(1));
        assert!(dir.is_dir(), "没有 attempt 目录证据不得删");
        let rows = read_jsonl(&runs.path().join(CGROUP_RECLAIM_AUDIT));
        assert!(rows
            .iter()
            .any(|row| row["scope"] == scope && row["reason"] == "attempt_evidence_missing"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_reclaim_removes_empty_scope_with_evidence() {
        let (server, _settings, runs) = test_server();
        let base = tempfile::TempDir::new().unwrap();
        fs::write(base.path().join("cgroup.controllers"), "cpu\n").unwrap();
        let scope = "aw-mcp-bridge-424245-0";
        let dir = fake_scope(base.path(), scope);
        let task_id = "v148-empty-scope";
        fs::create_dir_all(runs.path().join(task_id).join("attempt-1")).unwrap();
        fs::write(
            runs.path()
                .join(task_id)
                .join("attempt-1")
                .join("dispatch.json"),
            "{}",
        )
        .unwrap();
        let path = owner_record(runs.path(), scope, task_id, 424245, None);
        let mut record = read_json(&path).unwrap();
        record["cgroup"] = json!(dir.to_string_lossy());
        write_json_atomic(&path, &record).unwrap();
        let report = server
            .reclaim_stale_cgroup_scopes_at(base.path(), "startup")
            .unwrap();
        assert_eq!(report["removed_empty_total"].as_u64(), Some(1));
        assert!(!dir.exists(), "空作用域且证据齐全才允许删目录");
        assert!(!path.exists(), "目录删掉后 sidecar 一并删除");
        let rows = read_jsonl(&runs.path().join(CGROUP_RECLAIM_AUDIT));
        assert!(rows.iter().any(|row| row["scope"] == scope
            && row["phase"] == "result"
            && row["outcome"] == "removed_empty"
            && row["members_before"] == json!([])));
    }
}
