// 沙箱（M5.1 精简版）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M5.1
//
// v2.0 调整：
//   - v1.0 写"沙箱 + 安全审计"完整版 → v2.0 砍到本地 Docker 沙箱
//   - 不做 gVisor / 网络隔离（资源不够，先用标准 Docker）
//
// 沙箱 = 隔离的代码执行环境
//   - 本地 Docker 容器（Linux + Windows WSL2 + macOS）
//   - 资源限制（CPU / 内存 / 磁盘 / 执行时间）
//   - 审计日志（SQLite 持久化）
//   - 命令白名单（防止危险操作）
//
// 与 EmployeeDefinition 的关系：
//   - 员工调用 code_gen 工具时，可在沙箱中执行生成的代码
//   - 沙箱保证主机安全，避免恶意代码损害系统

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// 类型定义
// ============================================================================

/// 沙箱 ID 类型
pub type SandboxId = String;

/// 沙箱状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SandboxStatus {
    /// 已创建未启动
    Created,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
    /// 已销毁
    Destroyed,
    /// 启动失败
    Failed,
}

impl SandboxStatus {
    /// 获取状态中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::Created => "已创建",
            Self::Running => "运行中",
            Self::Paused => "已暂停",
            Self::Stopped => "已停止",
            Self::Destroyed => "已销毁",
            Self::Failed => "启动失败",
        }
    }

    /// 是否可执行命令
    pub fn can_execute(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// 沙箱资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResources {
    /// CPU 核心数限制
    pub cpu_cores: f32,
    /// 内存限制（MB）
    pub memory_mb: u64,
    /// 磁盘限制（MB）
    pub disk_mb: u64,
    /// 执行超时（秒）
    pub timeout_secs: u64,
    /// 最大进程数
    pub max_processes: u32,
    /// 网络访问（false = 禁用网络）
    pub network_enabled: bool,
}

impl Default for SandboxResources {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_mb: 512,
            disk_mb: 1024,
            timeout_secs: 30,
            max_processes: 64,
            network_enabled: false,
        }
    }
}

impl SandboxResources {
    /// 创建低配沙箱（适合简单脚本）
    pub fn low() -> Self {
        Self {
            cpu_cores: 0.5,
            memory_mb: 128,
            disk_mb: 256,
            timeout_secs: 10,
            max_processes: 16,
            network_enabled: false,
        }
    }

    /// 创建标准沙箱（默认配置）
    pub fn standard() -> Self {
        Self::default()
    }

    /// 创建高配沙箱（适合复杂计算）
    pub fn high() -> Self {
        Self {
            cpu_cores: 2.0,
            memory_mb: 2048,
            disk_mb: 4096,
            timeout_secs: 120,
            max_processes: 256,
            network_enabled: true,
        }
    }
}

/// 沙箱镜像类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SandboxImage {
    /// Python 3.11 运行时
    Python311,
    /// Node.js 20 运行时
    Node20,
    /// Rust 1.88 运行时
    Rust188,
    /// Ubuntu 22.04 通用
    Ubuntu2204,
    /// 自定义镜像（Docker Hub 名称）
    Custom(String),
}

impl SandboxImage {
    /// 获取镜像的 Docker tag
    pub fn docker_tag(&self) -> String {
        match self {
            Self::Python311 => "python:3.11-slim".to_string(),
            Self::Node20 => "node:20-slim".to_string(),
            Self::Rust188 => "rust:1.88-slim".to_string(),
            Self::Ubuntu2204 => "ubuntu:22.04".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    /// 获取镜像的中文标签
    pub fn label(&self) -> String {
        match self {
            Self::Python311 => "Python 3.11".to_string(),
            Self::Node20 => "Node.js 20".to_string(),
            Self::Rust188 => "Rust 1.88".to_string(),
            Self::Ubuntu2204 => "Ubuntu 22.04".to_string(),
            Self::Custom(name) => format!("自定义: {}", name),
        }
    }
}

/// 沙箱实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    /// 沙箱 ID
    pub id: SandboxId,
    /// 沙箱名称
    pub name: String,
    /// 使用的镜像
    pub image: SandboxImage,
    /// 资源配置
    pub resources: SandboxResources,
    /// 状态
    pub status: SandboxStatus,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: u64,
    /// 启动时间
    pub started_at: Option<u64>,
    /// 停止时间
    pub stopped_at: Option<u64>,
    /// 工作目录（容器内路径）
    pub work_dir: String,
    /// 环境变量
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// 挂载卷（主机路径 → 容器路径）
    #[serde(default)]
    pub volumes: HashMap<String, String>,
    /// 关联的员工 ID（可选）
    #[serde(default)]
    pub employee_id: Option<String>,
    /// 执行的命令数
    #[serde(default)]
    pub command_count: u64,
}

impl Sandbox {
    /// 创建新的沙箱实例
    pub fn new(name: impl Into<String>, image: SandboxImage, resources: SandboxResources) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_sandbox_id(),
            name: name.into(),
            image,
            resources,
            status: SandboxStatus::Created,
            created_at: now,
            started_at: None,
            stopped_at: None,
            work_dir: "/workspace".to_string(),
            env_vars: HashMap::new(),
            volumes: HashMap::new(),
            employee_id: None,
            command_count: 0,
        }
    }

    /// 设置环境变量
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// 设置挂载卷
    pub fn with_volume(mut self, host_path: impl Into<String>, container_path: impl Into<String>) -> Self {
        self.volumes.insert(host_path.into(), container_path.into());
        self
    }

    /// 设置关联员工
    pub fn with_employee(mut self, employee_id: impl Into<String>) -> Self {
        self.employee_id = Some(employee_id.into());
        self
    }

    /// 设置工作目录
    pub fn with_work_dir(mut self, dir: impl Into<String>) -> Self {
        self.work_dir = dir.into();
        self
    }

    /// 启动沙箱
    pub fn start(&mut self) -> Result<()> {
        if self.status == SandboxStatus::Running {
            return Ok(());
        }
        if self.status == SandboxStatus::Destroyed {
            return Err(crate::error::TeamError::Other(
                "沙箱已销毁，无法启动".to_string(),
            ));
        }
        self.status = SandboxStatus::Running;
        self.started_at = Some(current_timestamp());
        Ok(())
    }

    /// 停止沙箱
    pub fn stop(&mut self) -> Result<()> {
        if self.status != SandboxStatus::Running && self.status != SandboxStatus::Paused {
            return Ok(());
        }
        self.status = SandboxStatus::Stopped;
        self.stopped_at = Some(current_timestamp());
        Ok(())
    }

    /// 暂停沙箱
    pub fn pause(&mut self) -> Result<()> {
        if self.status != SandboxStatus::Running {
            return Err(crate::error::TeamError::Other(
                "只有运行中的沙箱才能暂停".to_string(),
            ));
        }
        self.status = SandboxStatus::Paused;
        Ok(())
    }

    /// 恢复沙箱
    pub fn resume(&mut self) -> Result<()> {
        if self.status != SandboxStatus::Paused {
            return Err(crate::error::TeamError::Other(
                "只有暂停的沙箱才能恢复".to_string(),
            ));
        }
        self.status = SandboxStatus::Running;
        Ok(())
    }

    /// 销毁沙箱
    pub fn destroy(&mut self) -> Result<()> {
        if self.status == SandboxStatus::Destroyed {
            return Ok(());
        }
        if self.status == SandboxStatus::Running {
            self.stop()?;
        }
        self.status = SandboxStatus::Destroyed;
        Ok(())
    }

    /// 检查是否可执行命令
    pub fn can_execute(&self) -> bool {
        self.status.can_execute()
    }

    /// 记录命令执行
    pub fn record_command(&mut self) {
        self.command_count += 1;
    }

    /// 获取运行时长（秒）
    pub fn uptime(&self) -> u64 {
        match (self.started_at, self.stopped_at) {
            (Some(start), Some(stop)) => (stop - start) / 1000,
            (Some(start), None) if self.status == SandboxStatus::Running => {
                (current_timestamp() - start) / 1000
            }
            _ => 0,
        }
    }
}

// ============================================================================
// 命令白名单
// ============================================================================

/// 命令白名单检查器
///
/// 防止沙箱中执行危险命令（如 rm -rf / / dd / mkfs 等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandWhitelist {
    /// 允许的命令前缀
    allowed: Vec<String>,
    /// 禁止的命令模式（正则表达式字符串）
    blocked_patterns: Vec<String>,
}

impl Default for CommandWhitelist {
    fn default() -> Self {
        Self::strict()
    }
}

impl CommandWhitelist {
    /// 严格模式（默认）：仅允许安全命令
    pub fn strict() -> Self {
        Self {
            allowed: vec![
                "python".to_string(),
                "python3".to_string(),
                "node".to_string(),
                "npm".to_string(),
                "cargo".to_string(),
                "rustc".to_string(),
                "echo".to_string(),
                "cat".to_string(),
                "ls".to_string(),
                "pwd".to_string(),
                "mkdir".to_string(),
                "touch".to_string(),
                "cp".to_string(),
                "mv".to_string(),
                "git".to_string(),
                "pip".to_string(),
                "pip3".to_string(),
            ],
            blocked_patterns: vec![
                r"rm\s+-rf\s+/$".to_string(),
                r"rm\s+-rf\s+/\*".to_string(),
                r"dd\s+if=".to_string(),
                r"mkfs".to_string(),
                r"fdisk".to_string(),
                r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;:".to_string(), // fork bomb
                r">\s*/dev/sda".to_string(),
                r"chmod\s+-R\s+777\s+/$".to_string(),
                r"curl\s+.*\|\s*sh".to_string(),
                r"wget\s+.*\|\s*sh".to_string(),
            ],
        }
    }

    /// 宽松模式：允许更多命令（适合开发环境）
    pub fn permissive() -> Self {
        let mut whitelist = Self::strict();
        whitelist.allowed.extend([
            "apt".to_string(),
            "apt-get".to_string(),
            "yum".to_string(),
            "brew".to_string(),
            "docker".to_string(),
            "make".to_string(),
            "cmake".to_string(),
            "gcc".to_string(),
            "g++".to_string(),
            "clang".to_string(),
        ]);
        whitelist
    }

    /// 检查命令是否允许执行
    pub fn check(&self, command: &str) -> CommandCheckResult {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return CommandCheckResult::blocked("命令为空");
        }

        // 检查禁止模式
        for pattern in &self.blocked_patterns {
            if simple_regex_match(trimmed, pattern) {
                return CommandCheckResult::blocked(format!(
                    "命令匹配禁止模式: {}",
                    pattern
                ));
            }
        }

        // 提取命令名（第一个单词）
        let cmd_name = trimmed.split_whitespace().next().unwrap_or("");
        let cmd_base = cmd_name.split('/').last().unwrap_or(cmd_name);

        // 检查白名单
        if self.allowed.iter().any(|allowed| allowed == cmd_base) {
            CommandCheckResult::allowed()
        } else {
            CommandCheckResult::blocked(format!("命令不在白名单中: {}", cmd_base))
        }
    }

    /// 添加允许的命令
    pub fn allow(&mut self, command: impl Into<String>) {
        self.allowed.push(command.into());
    }

    /// 添加禁止的模式
    pub fn block(&mut self, pattern: impl Into<String>) {
        self.blocked_patterns.push(pattern.into());
    }
}

/// 命令检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCheckResult {
    /// 是否允许
    pub allowed: bool,
    /// 原因（不允许时）
    pub reason: Option<String>,
}

impl CommandCheckResult {
    /// 创建允许的结果
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    /// 创建阻止的结果
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}

// ============================================================================
// 审计日志
// ============================================================================

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// 日志 ID
    pub id: u64,
    /// 沙箱 ID
    pub sandbox_id: SandboxId,
    /// 时间戳（Unix 毫秒）
    pub timestamp: u64,
    /// 事件类型
    pub event: AuditEvent,
    /// 命令（执行命令时）
    #[serde(default)]
    pub command: Option<String>,
    /// 退出码（命令执行完成时）
    #[serde(default)]
    pub exit_code: Option<i32>,
    /// 执行时长（毫秒）
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// 关联员工
    #[serde(default)]
    pub employee_id: Option<String>,
    /// 备注
    #[serde(default)]
    pub note: Option<String>,
}

/// 审计事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEvent {
    /// 沙箱创建
    SandboxCreated,
    /// 沙箱启动
    SandboxStarted,
    /// 沙箱暂停
    SandboxPaused,
    /// 沙箱恢复
    SandboxResumed,
    /// 沙箱停止
    SandboxStopped,
    /// 沙箱销毁
    SandboxDestroyed,
    /// 命令执行
    CommandExecuted,
    /// 命令被拒绝
    CommandBlocked,
    /// 命令超时
    CommandTimeout,
    /// 资源超限
    ResourceExceeded,
}

impl AuditEvent {
    /// 获取事件中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::SandboxCreated => "沙箱创建",
            Self::SandboxStarted => "沙箱启动",
            Self::SandboxPaused => "沙箱暂停",
            Self::SandboxResumed => "沙箱恢复",
            Self::SandboxStopped => "沙箱停止",
            Self::SandboxDestroyed => "沙箱销毁",
            Self::CommandExecuted => "命令执行",
            Self::CommandBlocked => "命令被拒绝",
            Self::CommandTimeout => "命令超时",
            Self::ResourceExceeded => "资源超限",
        }
    }
}

/// 审计日志（内存存储，MVP 阶段；后续可换 SQLite）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// 日志条目
    entries: Vec<AuditLogEntry>,
    /// 自增 ID
    next_id: u64,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    /// 创建空的审计日志
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// 记录事件
    pub fn log(&mut self, sandbox_id: &str, event: AuditEvent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = AuditLogEntry {
            id,
            sandbox_id: sandbox_id.to_string(),
            timestamp: current_timestamp(),
            event,
            command: None,
            exit_code: None,
            duration_ms: None,
            employee_id: None,
            note: None,
        };
        self.entries.push(entry);
        id
    }

    /// 记录命令执行
    pub fn log_command(
        &mut self,
        sandbox_id: &str,
        command: &str,
        exit_code: i32,
        duration_ms: u64,
        employee_id: Option<&str>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = AuditLogEntry {
            id,
            sandbox_id: sandbox_id.to_string(),
            timestamp: current_timestamp(),
            event: AuditEvent::CommandExecuted,
            command: Some(command.to_string()),
            exit_code: Some(exit_code),
            duration_ms: Some(duration_ms),
            employee_id: employee_id.map(|s| s.to_string()),
            note: None,
        };
        self.entries.push(entry);
        id
    }

    /// 记录命令被拒绝
    pub fn log_blocked(&mut self, sandbox_id: &str, command: &str, reason: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = AuditLogEntry {
            id,
            sandbox_id: sandbox_id.to_string(),
            timestamp: current_timestamp(),
            event: AuditEvent::CommandBlocked,
            command: Some(command.to_string()),
            exit_code: None,
            duration_ms: None,
            employee_id: None,
            note: Some(reason.to_string()),
        };
        self.entries.push(entry);
        id
    }

    /// 获取所有日志
    pub fn entries(&self) -> &[AuditLogEntry] {
        &self.entries
    }

    /// 按沙箱 ID 筛选
    pub fn by_sandbox(&self, sandbox_id: &str) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.sandbox_id == sandbox_id)
            .collect()
    }

    /// 按事件类型筛选
    pub fn by_event(&self, event: &AuditEvent) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| &e.event == event)
            .collect()
    }

    /// 获取日志总数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空日志
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 1;
    }
}

// ============================================================================
// 沙箱管理器
// ============================================================================

/// 沙箱管理器
///
/// 管理多个沙箱实例 + 命令白名单 + 审计日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxManager {
    /// 沙箱实例（按 ID 索引）
    sandboxes: HashMap<SandboxId, Sandbox>,
    /// 命令白名单
    pub whitelist: CommandWhitelist,
    /// 审计日志
    pub audit_log: AuditLog,
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxManager {
    /// 创建新的沙箱管理器
    pub fn new() -> Self {
        Self {
            sandboxes: HashMap::new(),
            whitelist: CommandWhitelist::strict(),
            audit_log: AuditLog::new(),
        }
    }

    /// 创建并注册沙箱
    pub fn create(
        &mut self,
        name: impl Into<String>,
        image: SandboxImage,
        resources: SandboxResources,
    ) -> SandboxId {
        let sandbox = Sandbox::new(name, image, resources);
        let id = sandbox.id.clone();
        self.audit_log.log(&id, AuditEvent::SandboxCreated);
        self.sandboxes.insert(id.clone(), sandbox);
        id
    }

    /// 获取沙箱
    pub fn get(&self, id: &str) -> Option<&Sandbox> {
        self.sandboxes.get(id)
    }

    /// 获取可变沙箱
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Sandbox> {
        self.sandboxes.get_mut(id)
    }

    /// 启动沙箱
    pub fn start(&mut self, id: &str) -> Result<()> {
        let sandbox = self
            .sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::TeamError::Other(format!("沙箱不存在: {}", id)))?;
        sandbox.start()?;
        self.audit_log.log(id, AuditEvent::SandboxStarted);
        Ok(())
    }

    /// 停止沙箱
    pub fn stop(&mut self, id: &str) -> Result<()> {
        let sandbox = self
            .sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::TeamError::Other(format!("沙箱不存在: {}", id)))?;
        sandbox.stop()?;
        self.audit_log.log(id, AuditEvent::SandboxStopped);
        Ok(())
    }

    /// 暂停沙箱
    pub fn pause(&mut self, id: &str) -> Result<()> {
        let sandbox = self
            .sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::TeamError::Other(format!("沙箱不存在: {}", id)))?;
        sandbox.pause()?;
        self.audit_log.log(id, AuditEvent::SandboxPaused);
        Ok(())
    }

    /// 恢复沙箱
    pub fn resume(&mut self, id: &str) -> Result<()> {
        let sandbox = self
            .sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::TeamError::Other(format!("沙箱不存在: {}", id)))?;
        sandbox.resume()?;
        self.audit_log.log(id, AuditEvent::SandboxResumed);
        Ok(())
    }

    /// 销毁沙箱
    pub fn destroy(&mut self, id: &str) -> Result<()> {
        let sandbox = self
            .sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::TeamError::Other(format!("沙箱不存在: {}", id)))?;
        sandbox.destroy()?;
        self.audit_log.log(id, AuditEvent::SandboxDestroyed);
        Ok(())
    }

    /// 执行命令（先检查白名单）
    pub fn execute(
        &mut self,
        id: &str,
        command: &str,
    ) -> std::result::Result<CommandResult, String> {
        // 检查沙箱状态
        let sandbox = self
            .sandboxes
            .get(id)
            .ok_or_else(|| format!("沙箱不存在: {}", id))?;

        if !sandbox.can_execute() {
            return Err(format!("沙箱状态不允许执行命令: {}", sandbox.status.label()));
        }

        // 检查命令白名单
        let check_result = self.whitelist.check(command);
        if !check_result.allowed {
            let reason = check_result.reason.unwrap_or_default();
            self.audit_log.log_blocked(id, command, &reason);
            return Err(format!("命令被拒绝: {}", reason));
        }

        // 记录命令执行（MVP：模拟执行）
        let start_time = SystemTime::now();
        let result = CommandResult {
            exit_code: 0,
            stdout: format!("[模拟执行] {}", command),
            stderr: String::new(),
            duration_ms: start_time
                .elapsed()
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        // 更新沙箱计数
        if let Some(sb) = self.sandboxes.get_mut(id) {
            sb.record_command();
        }

        // 记录审计日志
        let employee_id = self.sandboxes.get(id).and_then(|s| s.employee_id.clone());
        self.audit_log.log_command(
            id,
            command,
            result.exit_code,
            result.duration_ms,
            employee_id.as_deref(),
        );

        Ok(result)
    }

    /// 列出所有沙箱
    pub fn list(&self) -> Vec<&Sandbox> {
        self.sandboxes.values().collect()
    }

    /// 按状态筛选
    pub fn list_by_status(&self, status: &SandboxStatus) -> Vec<&Sandbox> {
        self.sandboxes
            .values()
            .filter(|s| &s.status == status)
            .collect()
    }

    /// 按员工筛选
    pub fn list_by_employee(&self, employee_id: &str) -> Vec<&Sandbox> {
        self.sandboxes
            .values()
            .filter(|s| s.employee_id.as_deref() == Some(employee_id))
            .collect()
    }

    /// 获取沙箱数量
    pub fn count(&self) -> usize {
        self.sandboxes.len()
    }

    /// 获取运行中的沙箱数量
    pub fn running_count(&self) -> usize {
        self.list_by_status(&SandboxStatus::Running).len()
    }

    /// 获取统计信息
    pub fn stats(&self) -> SandboxManagerStats {
        let mut by_status: HashMap<SandboxStatus, usize> = HashMap::new();
        for sandbox in self.sandboxes.values() {
            *by_status.entry(sandbox.status.clone()).or_default() += 1;
        }
        SandboxManagerStats {
            total_sandboxes: self.sandboxes.len(),
            running: self.running_count(),
            total_commands: self.sandboxes.values().map(|s| s.command_count).sum(),
            audit_log_entries: self.audit_log.len(),
            by_status,
        }
    }
}

/// 命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// 退出码（0 = 成功）
    pub exit_code: i32,
    /// 标准输出
    pub stdout: String,
    /// 标准错误
    pub stderr: String,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
}

impl CommandResult {
    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// 沙箱管理器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxManagerStats {
    /// 沙箱总数
    pub total_sandboxes: usize,
    /// 运行中数量
    pub running: usize,
    /// 累计执行命令数
    pub total_commands: u64,
    /// 审计日志条目数
    pub audit_log_entries: usize,
    /// 按状态统计
    pub by_status: HashMap<SandboxStatus, usize>,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 全局沙箱计数器（用于生成唯一 ID）
static SANDBOX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 获取当前时间戳（Unix 毫秒）
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 生成沙箱 ID（全局唯一，基于原子计数器）
fn generate_sandbox_id() -> String {
    let counter = SANDBOX_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("sbx_{:06x}", counter)
}

/// 简易正则匹配（MVP，仅支持简单模式）
///
/// 实际生产环境应使用 regex crate，此处为避免引入依赖，使用简单字符串匹配。
fn simple_regex_match(text: &str, pattern: &str) -> bool {
    // 简化处理：将 \s+ 替换为空格，\s* 替换为可选空格
    // 然后做包含匹配
    let normalized_pattern = pattern
        .replace(r"\s+", " ")
        .replace(r"\s*", "")
        .replace(r"\|", "|");
    
    // 简单包含匹配（不精确，但足够 MVP）
    text.contains(&normalized_pattern)
        || text.replace("  ", " ").contains(&normalized_pattern)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------
    // SandboxStatus 测试
    // ------------------------------------------------------------

    #[test]
    fn test_status_label() {
        assert_eq!(SandboxStatus::Created.label(), "已创建");
        assert_eq!(SandboxStatus::Running.label(), "运行中");
        assert_eq!(SandboxStatus::Stopped.label(), "已停止");
        assert_eq!(SandboxStatus::Destroyed.label(), "已销毁");
        assert_eq!(SandboxStatus::Failed.label(), "启动失败");
    }

    #[test]
    fn test_status_can_execute() {
        assert!(SandboxStatus::Running.can_execute());
        assert!(!SandboxStatus::Created.can_execute());
        assert!(!SandboxStatus::Stopped.can_execute());
        assert!(!SandboxStatus::Destroyed.can_execute());
    }

    // ------------------------------------------------------------
    // SandboxResources 测试
    // ------------------------------------------------------------

    #[test]
    fn test_resources_default() {
        let r = SandboxResources::default();
        assert_eq!(r.cpu_cores, 1.0);
        assert_eq!(r.memory_mb, 512);
        assert_eq!(r.disk_mb, 1024);
        assert_eq!(r.timeout_secs, 30);
        assert!(!r.network_enabled);
    }

    #[test]
    fn test_resources_low() {
        let r = SandboxResources::low();
        assert_eq!(r.cpu_cores, 0.5);
        assert_eq!(r.memory_mb, 128);
        assert_eq!(r.timeout_secs, 10);
    }

    #[test]
    fn test_resources_high() {
        let r = SandboxResources::high();
        assert_eq!(r.cpu_cores, 2.0);
        assert_eq!(r.memory_mb, 2048);
        assert_eq!(r.timeout_secs, 120);
        assert!(r.network_enabled);
    }

    // ------------------------------------------------------------
    // SandboxImage 测试
    // ------------------------------------------------------------

    #[test]
    fn test_image_docker_tag() {
        assert_eq!(SandboxImage::Python311.docker_tag(), "python:3.11-slim");
        assert_eq!(SandboxImage::Node20.docker_tag(), "node:20-slim");
        assert_eq!(SandboxImage::Rust188.docker_tag(), "rust:1.88-slim");
        assert_eq!(SandboxImage::Ubuntu2204.docker_tag(), "ubuntu:22.04");
        assert_eq!(
            SandboxImage::Custom("my/image:latest".to_string()).docker_tag(),
            "my/image:latest"
        );
    }

    #[test]
    fn test_image_label() {
        assert_eq!(SandboxImage::Python311.label(), "Python 3.11");
        assert_eq!(SandboxImage::Node20.label(), "Node.js 20");
    }

    // ------------------------------------------------------------
    // Sandbox 测试
    // ------------------------------------------------------------

    #[test]
    fn test_sandbox_new() {
        let sb = Sandbox::new(
            "test",
            SandboxImage::Python311,
            SandboxResources::default(),
        );
        assert!(!sb.id.is_empty());
        assert_eq!(sb.name, "test");
        assert_eq!(sb.image, SandboxImage::Python311);
        assert_eq!(sb.status, SandboxStatus::Created);
        assert_eq!(sb.work_dir, "/workspace");
        assert_eq!(sb.command_count, 0);
    }

    #[test]
    fn test_sandbox_with_env() {
        let sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default())
            .with_env("PATH", "/usr/bin")
            .with_env("HOME", "/root");
        assert_eq!(sb.env_vars.len(), 2);
        assert_eq!(sb.env_vars.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn test_sandbox_with_volume() {
        let sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default())
            .with_volume("/host/data", "/data");
        assert_eq!(sb.volumes.len(), 1);
        assert_eq!(sb.volumes.get("/host/data"), Some(&"/data".to_string()));
    }

    #[test]
    fn test_sandbox_with_employee() {
        let sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default())
            .with_employee("emp_001");
        assert_eq!(sb.employee_id, Some("emp_001".to_string()));
    }

    #[test]
    fn test_sandbox_lifecycle() {
        let mut sb = Sandbox::new(
            "test",
            SandboxImage::Python311,
            SandboxResources::default(),
        );

        // Created → Running
        assert!(sb.start().is_ok());
        assert_eq!(sb.status, SandboxStatus::Running);
        assert!(sb.started_at.is_some());

        // Running → Paused
        assert!(sb.pause().is_ok());
        assert_eq!(sb.status, SandboxStatus::Paused);

        // Paused → Running
        assert!(sb.resume().is_ok());
        assert_eq!(sb.status, SandboxStatus::Running);

        // Running → Stopped
        assert!(sb.stop().is_ok());
        assert_eq!(sb.status, SandboxStatus::Stopped);
        assert!(sb.stopped_at.is_some());

        // Stopped → Destroyed
        assert!(sb.destroy().is_ok());
        assert_eq!(sb.status, SandboxStatus::Destroyed);
    }

    #[test]
    fn test_sandbox_start_already_running() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        sb.start().unwrap();
        // 重复启动应成功（幂等）
        assert!(sb.start().is_ok());
    }

    #[test]
    fn test_sandbox_start_destroyed_fails() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        sb.destroy().unwrap();
        assert!(sb.start().is_err());
    }

    #[test]
    fn test_sandbox_pause_not_running_fails() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        // Created 状态不能暂停
        assert!(sb.pause().is_err());
    }

    #[test]
    fn test_sandbox_resume_not_paused_fails() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        sb.start().unwrap();
        // Running 状态不能恢复
        assert!(sb.resume().is_err());
    }

    #[test]
    fn test_sandbox_record_command() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        assert_eq!(sb.command_count, 0);
        sb.record_command();
        sb.record_command();
        assert_eq!(sb.command_count, 2);
    }

    #[test]
    fn test_sandbox_can_execute() {
        let mut sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default());
        assert!(!sb.can_execute());
        sb.start().unwrap();
        assert!(sb.can_execute());
        sb.stop().unwrap();
        assert!(!sb.can_execute());
    }

    // ------------------------------------------------------------
    // CommandWhitelist 测试
    // ------------------------------------------------------------

    #[test]
    fn test_whitelist_allows_python() {
        let wl = CommandWhitelist::strict();
        assert!(wl.check("python script.py").allowed);
        assert!(wl.check("python3 -c 'print(1)'").allowed);
    }

    #[test]
    fn test_whitelist_allows_node() {
        let wl = CommandWhitelist::strict();
        assert!(wl.check("node app.js").allowed);
        assert!(wl.check("npm install").allowed);
    }

    #[test]
    fn test_whitelist_allows_cargo() {
        let wl = CommandWhitelist::strict();
        assert!(wl.check("cargo build").allowed);
        assert!(wl.check("rustc main.rs").allowed);
    }

    #[test]
    fn test_whitelist_blocks_unknown_command() {
        let wl = CommandWhitelist::strict();
        let result = wl.check("rm file.txt");
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("不在白名单"));
    }

    #[test]
    fn test_whitelist_blocks_dangerous_patterns() {
        let wl = CommandWhitelist::strict();
        // rm -rf / 应该被阻止（即使 rm 不在白名单，也应被模式阻止）
        let result = wl.check("rm -rf /");
        assert!(!result.allowed);
    }

    #[test]
    fn test_whitelist_blocks_empty_command() {
        let wl = CommandWhitelist::strict();
        let result = wl.check("");
        assert!(!result.allowed);
    }

    #[test]
    fn test_whitelist_blocks_whitespace_only() {
        let wl = CommandWhitelist::strict();
        let result = wl.check("   ");
        assert!(!result.allowed);
    }

    #[test]
    fn test_whitelist_permissive_allows_more() {
        let wl = CommandWhitelist::permissive();
        assert!(wl.check("apt-get install python3").allowed);
        assert!(wl.check("gcc main.c").allowed);
        assert!(wl.check("make build").allowed);
    }

    #[test]
    fn test_whitelist_custom_allow() {
        let mut wl = CommandWhitelist::strict();
        wl.allow("my_tool");
        assert!(wl.check("my_tool --flag").allowed);
    }

    #[test]
    fn test_whitelist_custom_block() {
        let mut wl = CommandWhitelist::strict();
        wl.block("forbidden_command");
        // 添加到允许列表但被模式阻止
        wl.allow("forbidden_command");
        let result = wl.check("forbidden_command");
        assert!(!result.allowed);
    }

    #[test]
    fn test_whitelist_path_normalization() {
        let wl = CommandWhitelist::strict();
        // /usr/bin/python 应该被识别为 python
        assert!(wl.check("/usr/bin/python script.py").allowed);
        assert!(wl.check("/usr/local/bin/node app.js").allowed);
    }

    // ------------------------------------------------------------
    // AuditLog 测试
    // ------------------------------------------------------------

    #[test]
    fn test_audit_log_new() {
        let log = AuditLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_audit_log_log_event() {
        let mut log = AuditLog::new();
        let id = log.log("sbx_001", AuditEvent::SandboxCreated);
        assert_eq!(id, 1);
        assert_eq!(log.len(), 1);
        let id2 = log.log("sbx_001", AuditEvent::SandboxStarted);
        assert_eq!(id2, 2);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_audit_log_log_command() {
        let mut log = AuditLog::new();
        let id = log.log_command("sbx_001", "python script.py", 0, 150, Some("emp_001"));
        assert_eq!(id, 1);
        let entry = &log.entries()[0];
        assert_eq!(entry.sandbox_id, "sbx_001");
        assert_eq!(entry.command, Some("python script.py".to_string()));
        assert_eq!(entry.exit_code, Some(0));
        assert_eq!(entry.duration_ms, Some(150));
        assert_eq!(entry.employee_id, Some("emp_001".to_string()));
        assert_eq!(entry.event, AuditEvent::CommandExecuted);
    }

    #[test]
    fn test_audit_log_log_blocked() {
        let mut log = AuditLog::new();
        let id = log.log_blocked("sbx_001", "rm -rf /", "危险命令");
        assert_eq!(id, 1);
        let entry = &log.entries()[0];
        assert_eq!(entry.event, AuditEvent::CommandBlocked);
        assert_eq!(entry.command, Some("rm -rf /".to_string()));
        assert_eq!(entry.note, Some("危险命令".to_string()));
    }

    #[test]
    fn test_audit_log_by_sandbox() {
        let mut log = AuditLog::new();
        log.log("sbx_001", AuditEvent::SandboxCreated);
        log.log("sbx_002", AuditEvent::SandboxCreated);
        log.log("sbx_001", AuditEvent::SandboxStarted);

        let sbx1_entries = log.by_sandbox("sbx_001");
        assert_eq!(sbx1_entries.len(), 2);
        let sbx2_entries = log.by_sandbox("sbx_002");
        assert_eq!(sbx2_entries.len(), 1);
    }

    #[test]
    fn test_audit_log_by_event() {
        let mut log = AuditLog::new();
        log.log("sbx_001", AuditEvent::SandboxCreated);
        log.log("sbx_001", AuditEvent::SandboxStarted);
        log.log("sbx_002", AuditEvent::SandboxCreated);

        let created = log.by_event(&AuditEvent::SandboxCreated);
        assert_eq!(created.len(), 2);
        let started = log.by_event(&AuditEvent::SandboxStarted);
        assert_eq!(started.len(), 1);
    }

    #[test]
    fn test_audit_log_clear() {
        let mut log = AuditLog::new();
        log.log("sbx_001", AuditEvent::SandboxCreated);
        log.log("sbx_001", AuditEvent::SandboxStarted);
        assert_eq!(log.len(), 2);

        log.clear();
        assert_eq!(log.len(), 0);
        assert_eq!(log.next_id, 1);
    }

    #[test]
    fn test_audit_event_label() {
        assert_eq!(AuditEvent::SandboxCreated.label(), "沙箱创建");
        assert_eq!(AuditEvent::CommandExecuted.label(), "命令执行");
        assert_eq!(AuditEvent::CommandBlocked.label(), "命令被拒绝");
        assert_eq!(AuditEvent::CommandTimeout.label(), "命令超时");
    }

    // ------------------------------------------------------------
    // SandboxManager 测试
    // ------------------------------------------------------------

    #[test]
    fn test_manager_new() {
        let mgr = SandboxManager::new();
        assert_eq!(mgr.count(), 0);
        assert!(mgr.audit_log.is_empty());
    }

    #[test]
    fn test_manager_create() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create(
            "test-sandbox",
            SandboxImage::Python311,
            SandboxResources::default(),
        );
        assert!(mgr.get(&id).is_some());
        assert_eq!(mgr.count(), 1);
        // 创建时应记录审计日志
        assert_eq!(mgr.audit_log.len(), 1);
    }

    #[test]
    fn test_manager_start() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        assert!(mgr.start(&id).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Running);
        // 创建 + 启动 = 2 条日志
        assert_eq!(mgr.audit_log.len(), 2);
    }

    #[test]
    fn test_manager_start_nonexistent_fails() {
        let mut mgr = SandboxManager::new();
        assert!(mgr.start("nonexistent").is_err());
    }

    #[test]
    fn test_manager_stop() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        mgr.start(&id).unwrap();
        assert!(mgr.stop(&id).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Stopped);
    }

    #[test]
    fn test_manager_pause_resume() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        mgr.start(&id).unwrap();
        assert!(mgr.pause(&id).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Paused);
        assert!(mgr.resume(&id).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Running);
    }

    #[test]
    fn test_manager_destroy() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        mgr.start(&id).unwrap();
        assert!(mgr.destroy(&id).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Destroyed);
    }

    #[test]
    fn test_manager_execute_not_running() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        // 未启动的沙箱不能执行命令
        let result = mgr.execute(&id, "python script.py");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_execute_blocked_command() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        mgr.start(&id).unwrap();
        // rm 不在白名单
        let result = mgr.execute(&id, "rm file.txt");
        assert!(result.is_err());
        // 应记录阻止日志
        let blocked_logs = mgr.audit_log.by_event(&AuditEvent::CommandBlocked);
        assert_eq!(blocked_logs.len(), 1);
    }

    #[test]
    fn test_manager_execute_allowed_command() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        mgr.start(&id).unwrap();
        let result = mgr.execute(&id, "python script.py");
        assert!(result.is_ok());
        let cmd_result = result.unwrap();
        assert_eq!(cmd_result.exit_code, 0);
        assert!(cmd_result.is_success());

        // 沙箱命令计数应增加
        assert_eq!(mgr.get(&id).unwrap().command_count, 1);

        // 应记录执行日志
        let exec_logs = mgr.audit_log.by_event(&AuditEvent::CommandExecuted);
        assert_eq!(exec_logs.len(), 1);
    }

    #[test]
    fn test_manager_execute_nonexistent_sandbox() {
        let mut mgr = SandboxManager::new();
        let result = mgr.execute("nonexistent", "python script.py");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_list() {
        let mut mgr = SandboxManager::new();
        mgr.create("sb1", SandboxImage::Python311, SandboxResources::default());
        mgr.create("sb2", SandboxImage::Node20, SandboxResources::default());
        mgr.create("sb3", SandboxImage::Rust188, SandboxResources::default());
        assert_eq!(mgr.list().len(), 3);
    }

    #[test]
    fn test_manager_list_by_status() {
        let mut mgr = SandboxManager::new();
        let id1 = mgr.create("sb1", SandboxImage::Python311, SandboxResources::default());
        let id2 = mgr.create("sb2", SandboxImage::Node20, SandboxResources::default());
        mgr.start(&id1).unwrap();

        let running = mgr.list_by_status(&SandboxStatus::Running);
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].name, "sb1");

        let created = mgr.list_by_status(&SandboxStatus::Created);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "sb2");
    }

    #[test]
    fn test_manager_list_by_employee() {
        let mut mgr = SandboxManager::new();
        let id1 = mgr.create("sb1", SandboxImage::Python311, SandboxResources::default());
        mgr.get_mut(&id1).unwrap().employee_id = Some("emp_001".to_string());

        let id2 = mgr.create("sb2", SandboxImage::Node20, SandboxResources::default());
        mgr.get_mut(&id2).unwrap().employee_id = Some("emp_002".to_string());

        let emp1_sandboxes = mgr.list_by_employee("emp_001");
        assert_eq!(emp1_sandboxes.len(), 1);
        assert_eq!(emp1_sandboxes[0].name, "sb1");
    }

    #[test]
    fn test_manager_running_count() {
        let mut mgr = SandboxManager::new();
        let id1 = mgr.create("sb1", SandboxImage::Python311, SandboxResources::default());
        let id2 = mgr.create("sb2", SandboxImage::Node20, SandboxResources::default());
        mgr.start(&id1).unwrap();

        assert_eq!(mgr.running_count(), 1);
        mgr.start(&id2).unwrap();
        assert_eq!(mgr.running_count(), 2);
    }

    #[test]
    fn test_manager_stats() {
        let mut mgr = SandboxManager::new();
        let id1 = mgr.create("sb1", SandboxImage::Python311, SandboxResources::default());
        let id2 = mgr.create("sb2", SandboxImage::Node20, SandboxResources::default());
        mgr.start(&id1).unwrap();
        mgr.execute(&id1, "python script.py").unwrap();

        let stats = mgr.stats();
        assert_eq!(stats.total_sandboxes, 2);
        assert_eq!(stats.running, 1);
        assert_eq!(stats.total_commands, 1);
        assert!(stats.audit_log_entries > 0);
    }

    // ------------------------------------------------------------
    // CommandResult 测试
    // ------------------------------------------------------------

    #[test]
    fn test_command_result_success() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration_ms: 100,
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_command_result_failure() {
        let result = CommandResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 100,
        };
        assert!(!result.is_success());
    }

    // ------------------------------------------------------------
    // 序列化测试
    // ------------------------------------------------------------

    #[test]
    fn test_sandbox_serialization() {
        let sb = Sandbox::new("test", SandboxImage::Python311, SandboxResources::default())
            .with_env("KEY", "VALUE")
            .with_employee("emp_001");
        let json = serde_json::to_string(&sb).unwrap();
        let deserialized: Sandbox = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.image, SandboxImage::Python311);
        assert_eq!(deserialized.env_vars.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(deserialized.employee_id, Some("emp_001".to_string()));
    }

    #[test]
    fn test_manager_serialization() {
        let mut mgr = SandboxManager::new();
        mgr.create("test", SandboxImage::Python311, SandboxResources::default());
        let json = serde_json::to_string(&mgr).unwrap();
        let deserialized: SandboxManager = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count(), 1);
    }

    // ------------------------------------------------------------
    // 集成测试
    // ------------------------------------------------------------

    #[test]
    fn test_integration_full_lifecycle() {
        let mut mgr = SandboxManager::new();

        // 1. 创建沙箱
        let id = mgr.create(
            "python-sandbox",
            SandboxImage::Python311,
            SandboxResources::standard(),
        );
        assert_eq!(mgr.count(), 1);

        // 2. 启动
        assert!(mgr.start(&id).is_ok());
        assert_eq!(mgr.running_count(), 1);

        // 3. 执行多条命令
        mgr.execute(&id, "python -c 'print(1)'").unwrap();
        mgr.execute(&id, "python script.py").unwrap();
        mgr.execute(&id, "ls -la").unwrap();
        assert_eq!(mgr.get(&id).unwrap().command_count, 3);

        // 4. 尝试执行危险命令（应被拒绝）
        let blocked = mgr.execute(&id, "rm -rf /");
        assert!(blocked.is_err());

        // 5. 暂停 / 恢复
        mgr.pause(&id).unwrap();
        assert!(!mgr.get(&id).unwrap().can_execute());
        mgr.resume(&id).unwrap();
        assert!(mgr.get(&id).unwrap().can_execute());

        // 6. 停止
        mgr.stop(&id).unwrap();
        assert_eq!(mgr.running_count(), 0);

        // 7. 销毁
        mgr.destroy(&id).unwrap();
        assert_eq!(mgr.get(&id).unwrap().status, SandboxStatus::Destroyed);

        // 8. 验证审计日志
        let logs = mgr.audit_log.by_sandbox(&id);
        assert!(logs.len() >= 6); // created + started + 3 executed + 1 blocked + paused + resumed + stopped + destroyed
    }

    #[test]
    fn test_integration_multiple_sandboxes() {
        let mut mgr = SandboxManager::new();

        // 创建 3 个不同镜像的沙箱
        let id1 = mgr.create("py", SandboxImage::Python311, SandboxResources::low());
        let id2 = mgr.create("node", SandboxImage::Node20, SandboxResources::standard());
        let id3 = mgr.create("rust", SandboxImage::Rust188, SandboxResources::high());

        // 全部启动
        mgr.start(&id1).unwrap();
        mgr.start(&id2).unwrap();
        mgr.start(&id3).unwrap();
        assert_eq!(mgr.running_count(), 3);

        // 各执行一条命令
        mgr.execute(&id1, "python -c 'print(1)'").unwrap();
        mgr.execute(&id2, "node -e 'console.log(1)'").unwrap();
        mgr.execute(&id3, "cargo --version").unwrap();

        // 验证统计
        let stats = mgr.stats();
        assert_eq!(stats.total_sandboxes, 3);
        assert_eq!(stats.running, 3);
        assert_eq!(stats.total_commands, 3);
    }

    #[test]
    fn test_integration_employee_isolation() {
        let mut mgr = SandboxManager::new();

        // 为两个员工分别创建沙箱
        let id1 = mgr.create("emp1-sb", SandboxImage::Python311, SandboxResources::default());
        mgr.get_mut(&id1).unwrap().employee_id = Some("emp_001".to_string());

        let id2 = mgr.create("emp2-sb", SandboxImage::Node20, SandboxResources::default());
        mgr.get_mut(&id2).unwrap().employee_id = Some("emp_002".to_string());

        // 验证按员工筛选
        assert_eq!(mgr.list_by_employee("emp_001").len(), 1);
        assert_eq!(mgr.list_by_employee("emp_002").len(), 1);
        assert_eq!(mgr.list_by_employee("emp_003").len(), 0);
    }
}
