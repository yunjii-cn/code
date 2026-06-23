// MCP 客户端（M6.4 W15）
//
// 兼容 Hermes MCP 生态：
//   - 支持 stdio MCP Server（进程通信）
//   - 支持 HTTP MCP Server（REST API）
//   - Per-server 工具过滤
//   - 自动发现 + 注册
//
// 降维打击点：AW 用户可以无代码接入 Hermes 生态的所有 MCP 服务器。

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP 服务器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpServerType {
    /// 标准输入输出进程
    Stdio,
    /// HTTP REST API
    Http,
    /// SSE（Server-Sent Events）
    Sse,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// 服务器 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 服务器类型
    pub server_type: McpServerType,
    /// 命令（Stdio 模式）
    #[serde(default)]
    pub command: Option<String>,
    /// 命令参数
    #[serde(default)]
    pub args: Vec<String>,
    /// HTTP URL
    #[serde(default)]
    pub url: Option<String>,
    /// 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 工具过滤（空 = 全部启用）
    #[serde(default)]
    pub tool_filter: Vec<String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 自动发现
    #[serde(default)]
    pub auto_discover: bool,
}

fn default_true() -> bool {
    true
}

/// MCP 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// 工具名
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入参数 schema（JSON Schema）
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// 所属服务器 ID
    pub server_id: String,
}

/// MCP 客户端状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientState {
    /// 已注册的服务器
    pub servers: Vec<McpServerConfig>,
    /// 已发现的工具
    pub tools: Vec<McpTool>,
}

impl Default for McpClientState {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            tools: Vec::new(),
        }
    }
}

/// MCP 客户端
pub struct McpClient {
    state: McpClientState,
}

impl McpClient {
    /// 创建客户端
    pub fn new() -> Self {
        Self {
            state: McpClientState::default(),
        }
    }

    /// 注册服务器
    pub fn register_server(&mut self, config: McpServerConfig) -> Result<()> {
        if self.state.servers.iter().any(|s| s.id == config.id) {
            return Err(crate::error::TeamError::Config(format!(
                "MCP 服务器 '{}' 已注册",
                config.id
            )));
        }
        self.state.servers.push(config);
        Ok(())
    }

    /// 注销服务器
    pub fn unregister_server(&mut self, id: &str) -> bool {
        let len_before = self.state.servers.len();
        self.state.servers.retain(|s| s.id != id);
        self.state.tools.retain(|t| t.server_id != id);
        self.state.servers.len() != len_before
    }

    /// 获取服务器配置
    pub fn get_server(&self, id: &str) -> Option<&McpServerConfig> {
        self.state.servers.iter().find(|s| s.id == id)
    }

    /// 列出所有启用的服务器
    pub fn active_servers(&self) -> Vec<&McpServerConfig> {
        self.state.servers.iter().filter(|s| s.enabled).collect()
    }

    /// 注册工具
    pub fn register_tool(&mut self, tool: McpTool) {
        // 去重
        if !self.state.tools.iter().any(|t| t.name == tool.name && t.server_id == tool.server_id) {
            self.state.tools.push(tool);
        }
    }

    /// 获取工具（应用过滤）
    pub fn get_tools(&self, server_id: &str) -> Vec<&McpTool> {
        let server = match self.get_server(server_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        self.state
            .tools
            .iter()
            .filter(|t| t.server_id == server_id)
            .filter(|t| {
                server.tool_filter.is_empty() || server.tool_filter.contains(&t.name)
            })
            .collect()
    }

    /// 列出所有可用工具
    pub fn all_tools(&self) -> Vec<&McpTool> {
        let mut tools = Vec::new();
        for server in &self.state.servers {
            if server.enabled {
                tools.extend(self.get_tools(&server.id));
            }
        }
        tools
    }

    /// 搜索工具（按名称或描述）
    pub fn search_tools(&self, query: &str) -> Vec<&McpTool> {
        let q = query.to_ascii_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        self.all_tools()
            .into_iter()
            .filter(|t| {
                t.name.to_ascii_lowercase().contains(&q)
                    || t.description.to_ascii_lowercase().contains(&q)
            })
            .collect()
    }

    /// 获取状态快照
    pub fn state(&self) -> &McpClientState {
        &self.state
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_server() {
        let mut client = McpClient::new();
        let config = McpServerConfig {
            id: "filesystem".to_string(),
            name: "文件系统".to_string(),
            server_type: McpServerType::Stdio,
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
            url: None,
            env: HashMap::new(),
            tool_filter: vec![],
            enabled: true,
            auto_discover: true,
        };
        client.register_server(config).unwrap();
        assert_eq!(client.active_servers().len(), 1);
    }

    #[test]
    fn test_register_duplicate_fails() {
        let mut client = McpClient::new();
        let config = McpServerConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            server_type: McpServerType::Http,
            command: None,
            args: vec![],
            url: Some("http://localhost:8080".to_string()),
            env: HashMap::new(),
            tool_filter: vec![],
            enabled: true,
            auto_discover: false,
        };
        client.register_server(config.clone()).unwrap();
        assert!(client.register_server(config).is_err());
    }

    #[test]
    fn test_tool_filter() {
        let mut client = McpClient::new();
        let config = McpServerConfig {
            id: "srv".to_string(),
            name: "Server".to_string(),
            server_type: McpServerType::Http,
            command: None,
            args: vec![],
            url: Some("http://localhost".to_string()),
            env: HashMap::new(),
            tool_filter: vec!["read".to_string()],
            enabled: true,
            auto_discover: false,
        };
        client.register_server(config).unwrap();

        client.register_tool(McpTool {
            name: "read".to_string(),
            description: "读取文件".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "srv".to_string(),
        });
        client.register_tool(McpTool {
            name: "write".to_string(),
            description: "写入文件".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "srv".to_string(),
        });

        let tools = client.get_tools("srv");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read");
    }

    #[test]
    fn test_unregister_server_removes_tools() {
        let mut client = McpClient::new();
        let config = McpServerConfig {
            id: "srv".to_string(),
            name: "Server".to_string(),
            server_type: McpServerType::Http,
            command: None,
            args: vec![],
            url: Some("http://localhost".to_string()),
            env: HashMap::new(),
            tool_filter: vec![],
            enabled: true,
            auto_discover: false,
        };
        client.register_server(config).unwrap();
        client.register_tool(McpTool {
            name: "read".to_string(),
            description: "读取".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "srv".to_string(),
        });

        client.unregister_server("srv");
        assert!(client.active_servers().is_empty());
        assert!(client.all_tools().is_empty());
    }

    #[test]
    fn test_search_tools() {
        let mut client = McpClient::new();
        let config = McpServerConfig {
            id: "srv".to_string(),
            name: "Server".to_string(),
            server_type: McpServerType::Http,
            command: None,
            args: vec![],
            url: Some("http://localhost".to_string()),
            env: HashMap::new(),
            tool_filter: vec![],
            enabled: true,
            auto_discover: false,
        };
        client.register_server(config).unwrap();
        client.register_tool(McpTool {
            name: "file_read".to_string(),
            description: "读取文件".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "srv".to_string(),
        });
        client.register_tool(McpTool {
            name: "shell_exec".to_string(),
            description: "执行命令".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "srv".to_string(),
        });

        let results = client.search_tools("文件");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "file_read");
    }
}