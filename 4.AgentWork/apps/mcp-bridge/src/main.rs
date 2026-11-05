// 最小版 MCP 服务器入口：stdio 逐行读取 JSON-RPC 请求，写回单行 JSON 响应
use std::io::{BufRead, Write};
use std::path::PathBuf;

fn default_settings_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("HOME").ok().filter(|value| !value.trim().is_empty()));
    match base {
        Some(base) => PathBuf::from(base).join("ai.yunji.agentwork").join("settings.json"),
        None => PathBuf::from("settings.json"),
    }
}

fn main() -> std::io::Result<()> {
    let server = aw_mcp_bridge::Server::new(default_settings_path());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle_line(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
