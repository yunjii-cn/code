// 最小版 MCP 服务器入口：stdio 逐行读取 JSON-RPC 请求，写回单行 JSON 响应
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use aw_mcp_bridge::{auto_sweep_schedule, AutoSweepSchedule, Server};

fn default_settings_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    match base {
        Some(base) => PathBuf::from(base)
            .join("ai.yunji.agentwork")
            .join("settings.json"),
        None => PathBuf::from("settings.json"),
    }
}

/// 只有 task.execute 需要后台线程：它可以跑满整个 timeout，而同一条连接上的
/// task.cancel 必须能在这期间被读到。其余请求仍在主线程串行处理。
fn is_background_task_execute(line: &str) -> bool {
    let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    message.get("method").and_then(|value| value.as_str()) == Some("tools/call")
        && message
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(|name| name.as_str())
            == Some("task.execute")
}

/// stdio 写入必须整行原子：主线程与多个执行线程共用这一把锁。
fn emit(gate: &Mutex<()>, response: &serde_json::Value) {
    let _guard = gate.lock().unwrap_or_else(|error| error.into_inner());
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{response}");
    let _ = stdout.flush();
}

/// stdin 关闭时不能立刻退出：先等在跑的执行线程收尾，保持旧的"任务跑完才退出"语义
struct ActiveExecutions {
    state: Arc<(Mutex<usize>, Condvar)>,
}

impl ActiveExecutions {
    fn new() -> Self {
        Self {
            state: Arc::new((Mutex::new(0), Condvar::new())),
        }
    }

    fn enter(&self) -> ActiveGuard {
        let (lock, _) = &*self.state;
        let mut count = lock.lock().unwrap_or_else(|error| error.into_inner());
        *count += 1;
        drop(count);
        ActiveGuard {
            state: Arc::clone(&self.state),
        }
    }

    fn wait_idle(&self) {
        let (lock, ready) = &*self.state;
        let mut count = lock.lock().unwrap_or_else(|error| error.into_inner());
        while *count > 0 {
            count = ready.wait(count).unwrap_or_else(|error| error.into_inner());
        }
    }
}

struct ActiveGuard {
    state: Arc<(Mutex<usize>, Condvar)>,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        let (lock, ready) = &*self.state;
        let mut count = lock.lock().unwrap_or_else(|error| error.into_inner());
        *count = count.saturating_sub(1);
        drop(count);
        ready.notify_all();
    }
}

fn main() -> std::io::Result<()> {
    let server = Arc::new(Server::new(default_settings_path()));
    // 自动清扫：启动一次总是执行（off 除外）；周期默认每小时（AW_INFLIGHT_SWEEP_SECS 可调）
    match auto_sweep_schedule() {
        AutoSweepSchedule::Off => {}
        AutoSweepSchedule::StartupOnly => server.run_startup_sweep(),
        AutoSweepSchedule::Every(interval) => {
            server.run_startup_sweep();
            server.start_periodic_sweep(interval);
        }
    }
    let write_gate = Arc::new(Mutex::new(()));
    let active = ActiveExecutions::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if is_background_task_execute(&line) {
            let server = Arc::clone(&server);
            let gate = Arc::clone(&write_gate);
            let _running = active.enter();
            std::thread::spawn(move || {
                let _running = _running;
                if let Some(response) = server.handle_line(&line) {
                    emit(&gate, &response);
                }
            });
            continue;
        }
        if let Some(response) = server.handle_line(&line) {
            emit(&write_gate, &response);
        }
    }
    active.wait_idle();
    Ok(())
}
