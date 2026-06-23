// 工具提供者抽象（M6.1 W4 / M6.4 W15）
//
// 目标：统一文件、HTTP、数据库、命令、MCP、Agent-to-Agent 工具调用抽象。
// 无代码工具构建器（M6.2 W8）会基于此 trait 把 6 类数据源统一暴露给 Skill。
// MCP 客户端（M6.4 W15）也实现此 trait，让 Hermes / Claude Code 生态成为 AW 的输入。

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolRequest {
    /// 工具名
    pub tool_name: String,
    /// 参数映射
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl ToolRequest {
    /// 创建请求
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            params: HashMap::new(),
        }
    }

    /// 设置参数
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// 取参数（缺失返回空）
    pub fn param(&self, key: &str) -> &str {
        self.params.get(key).map(|s| s.as_str()).unwrap_or("")
    }
}

/// 工具调用响应
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolResponse {
    /// 是否成功
    pub success: bool,
    /// 文本输出
    #[serde(default)]
    pub output: String,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: String,
}

impl ToolResponse {
    /// 成功响应
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: String::new(),
        }
    }

    /// 失败响应
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: error.into(),
        }
    }
}

/// 工具提供者类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolProviderKind {
    /// 文件系统
    File,
    /// HTTP API
    Http,
    /// 数据库
    Database,
    /// 命令执行
    Command,
    /// MCP 协议
    Mcp,
    /// Agent-to-Agent
    Agent,
}

impl ToolProviderKind {
    /// 中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::File => "文件",
            Self::Http => "HTTP API",
            Self::Database => "数据库",
            Self::Command => "命令",
            Self::Mcp => "MCP",
            Self::Agent => "Agent-to-Agent",
        }
    }
}

/// 统一工具提供者抽象
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &str;

    /// 提供者类型
    fn kind(&self) -> ToolProviderKind;

    /// 执行一次调用
    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse>;
}

/// 回声提供者（测试/演示用，原样返回请求参数）
pub struct EchoProvider {
    name: String,
}

impl EchoProvider {
    /// 创建
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl ToolProvider for EchoProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> ToolProviderKind {
        ToolProviderKind::Agent
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
        let mut pairs: Vec<String> = request
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        pairs.sort();
        Ok(ToolResponse::success(format!(
            "echo:{}|{}",
            request.tool_name,
            pairs.join(",")
        )))
    }
}

/// 文件提供者（本地文件读写，限制在 root 下）
/// 工具：
///   - read   params: path  → 返回文件内容
///   - write  params: path, content  → 写入文件
///   - list   params: path  → 列出目录
pub struct FileToolProvider {
    name: String,
    root: PathBuf,
}

impl FileToolProvider {
    /// 创建（root 为允许访问的根目录）
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            name: "file".to_string(),
            root: root.into(),
        }
    }

    /// 解析并校验路径（必须在 root 之下，防越权）
    fn resolve(&self, raw: &str) -> Result<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(crate::error::TeamError::Config("path 不能为空".to_string()));
        }
        let candidate = if std::path::Path::new(trimmed).is_absolute() {
            PathBuf::from(trimmed)
        } else {
            self.root.join(trimmed)
        };
        // 词法归一化（消除 .. / .），不依赖文件是否真实存在
        let normalized = lexical_normalize(&candidate);
        let normalized_root = lexical_normalize(&self.root);
        if !normalized.starts_with(&normalized_root) {
            return Err(crate::error::TeamError::Config(format!(
                "path 越权：{} 不在 root 之下",
                candidate.display()
            )));
        }
        Ok(normalized)
    }
}

/// 词法归一化路径：展开 . 和 ..，不访问文件系统
fn lexical_normalize(path: &PathBuf) -> PathBuf {
    use std::path::Component;
    let mut out: Vec<Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                // 仅在上一级是正常组件时弹出（保留根前缀）
                if let Some(last) = out.last() {
                    match last {
                        Component::Normal(_) => {
                            out.pop();
                        }
                        // 根 / 前缀之上的 .. 保留
                        _ => out.push(comp),
                    }
                } else {
                    out.push(comp);
                }
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

#[async_trait]
impl ToolProvider for FileToolProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> ToolProviderKind {
        ToolProviderKind::File
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
        let path = request.param("path");
        match request.tool_name.as_str() {
            "read" => {
                let resolved = match self.resolve(path) {
                    Ok(p) => p,
                    Err(e) => return Ok(ToolResponse::failure(e.to_string())),
                };
                match std::fs::read_to_string(&resolved) {
                    Ok(content) => Ok(ToolResponse::success(content)),
                    Err(e) => Ok(ToolResponse::failure(e.to_string())),
                }
            }
            "write" => {
                let content = request.param("content");
                let resolved = match self.resolve(path) {
                    Ok(p) => p,
                    Err(e) => return Ok(ToolResponse::failure(e.to_string())),
                };
                if let Some(parent) = resolved.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&resolved, content) {
                    Ok(_) => Ok(ToolResponse::success(format!("written:{}", resolved.display()))),
                    Err(e) => Ok(ToolResponse::failure(e.to_string())),
                }
            }
            "list" => {
                let resolved = match self.resolve(path) {
                    Ok(p) => p,
                    Err(e) => return Ok(ToolResponse::failure(e.to_string())),
                };
                match std::fs::read_dir(&resolved) {
                    Ok(entries) => {
                        let mut names: Vec<String> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect();
                        names.sort();
                        Ok(ToolResponse::success(names.join("\n")))
                    }
                    Err(e) => Ok(ToolResponse::failure(e.to_string())),
                }
            }
            other => Ok(ToolResponse::failure(format!(
                "未知工具：{other}（支持 read/write/list）"
            ))),
        }
    }
}

/// 工具注册表（聚合多个 provider，按 name 路由）
#[derive(Default, Clone)]
pub struct ToolRegistry {
    providers: HashMap<String, Arc<dyn ToolProvider>>,
}

impl ToolRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 provider（按 name 索引）
    pub fn register(&mut self, provider: Arc<dyn ToolProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// 按 name 查找
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolProvider>> {
        self.providers.get(name).cloned()
    }

    /// 已注册的 provider 名列表
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.providers.keys().cloned().collect();
        v.sort();
        v
    }

    /// 注册数量
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// 执行：先按 request.tool_name 的第一段（split '/'）找 provider，
    /// 也可以直接用 tool_name 作为 provider 名（单工具 provider）
    pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
        // 优先把 "file/read" 视作 provider=file, tool=read
        let (provider_name, tool_name) =
            if let Some((p, t)) = request.tool_name.split_once('/') {
                (p.to_string(), t.to_string())
            } else {
                // 单工具 provider：直接用 tool_name 当 provider 名
                if self.providers.contains_key(&request.tool_name) {
                    (request.tool_name.clone(), request.tool_name.clone())
                } else {
                    (request.tool_name.clone(), request.tool_name.clone())
                }
            };
        let provider = match self.providers.get(&provider_name) {
            Some(p) => p.clone(),
            None => {
                return Ok(ToolResponse::failure(format!(
                    "未注册的工具提供者：{provider_name}"
                )));
            }
        };
        let mut req = request;
        req.tool_name = tool_name;
        provider.execute(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_provider_round_trip() {
        let p = EchoProvider::new("echo");
        assert_eq!(p.kind(), ToolProviderKind::Agent);
        let req = ToolRequest::new("greet").with_param("who", "world");
        let resp = p.execute(req).await.unwrap();
        assert!(resp.success);
        assert!(resp.output.contains("greet"));
        assert!(resp.output.contains("who=world"));
    }

    #[tokio::test]
    async fn file_provider_read_write_list() {
        let dir = std::env::temp_dir().join("aw_tool_provider_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let provider = FileToolProvider::new(&dir);

        // write
        let write_req = ToolRequest::new("write")
            .with_param("path", "a.txt")
            .with_param("content", "hello");
        let resp = provider.execute(write_req).await.unwrap();
        assert!(resp.success, "{}", resp.error);

        // read
        let read_req = ToolRequest::new("read").with_param("path", "a.txt");
        let resp = provider.execute(read_req).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.output, "hello");

        // list
        let list_req = ToolRequest::new("list").with_param("path", ".");
        let resp = provider.execute(list_req).await.unwrap();
        assert!(resp.success);
        assert!(resp.output.contains("a.txt"));

        // unknown tool
        let bad = provider
            .execute(ToolRequest::new("delete").with_param("path", "a.txt"))
            .await
            .unwrap();
        assert!(!bad.success);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn file_provider_blocks_path_escape() {
        let dir = std::env::temp_dir().join("aw_tool_provider_escape_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let provider = FileToolProvider::new(&dir);
        // 尝试用 .. 越权读 root 之外
        let resp = provider
            .execute(ToolRequest::new("read").with_param("path", "../../etc/passwd"))
            .await
            .unwrap();
        assert!(!resp.success);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn registry_routes_slash_namespaced() {
        let dir = std::env::temp_dir().join("aw_tool_registry_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(FileToolProvider::new(&dir)));
        registry.register(Arc::new(EchoProvider::new("echo")));

        // file/write → provider=file tool=write
        let resp = registry
            .execute(
                ToolRequest::new("file/write")
                    .with_param("path", "x.txt")
                    .with_param("content", "y"),
            )
            .await
            .unwrap();
        assert!(resp.success, "{}", resp.error);

        // file/read
        let resp = registry
            .execute(ToolRequest::new("file/read").with_param("path", "x.txt"))
            .await
            .unwrap();
        assert_eq!(resp.output, "y");

        // echo
        let resp = registry
            .execute(ToolRequest::new("echo").with_param("k", "v"))
            .await
            .unwrap();
        assert!(resp.success);
        assert!(resp.output.contains("k=v"));

        // unknown
        let resp = registry
            .execute(ToolRequest::new("nope/read"))
            .await
            .unwrap();
        assert!(!resp.success);

        let names = registry.names();
        assert!(names.contains(&"file".to_string()));
        assert!(names.contains(&"echo".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tool_request_helpers() {
        let req = ToolRequest::new("t").with_param("a", "1").with_param("b", "2");
        assert_eq!(req.param("a"), "1");
        assert_eq!(req.param("missing"), "");
    }

    #[test]
    fn tool_response_helpers() {
        let ok = ToolResponse::success("done");
        assert!(ok.success);
        let err = ToolResponse::failure("boom");
        assert!(!err.success);
        assert_eq!(err.error, "boom");
    }

    #[test]
    fn kind_label() {
        assert_eq!(ToolProviderKind::File.label(), "文件");
        assert_eq!(ToolProviderKind::Mcp.label(), "MCP");
    }
}
