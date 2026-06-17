// AgentWork 桌面端 - Tauri 后端入口
// 文档: docs/ARCHITECTURE.md

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aw_desktop_lib::run()
}
