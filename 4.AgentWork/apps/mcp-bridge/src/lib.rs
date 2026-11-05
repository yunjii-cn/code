// aw-mcp-bridge 最小版：以 MCP 协议（stdio / JSON-RPC 2.0）暴露只读工具
pub mod server;

pub use server::Server;
