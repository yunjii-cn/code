// 安全增强（M6.4 W17）
//
// 安全策略：
//   - 危险命令白名单审批
//   - 路径逃逸检测
//   - 凭证过滤（API key 等敏感信息自动脱敏）
//   - 跨 Session 隔离
//   - 输入净化（防注入）

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 安全级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// 宽松（开发模式）
    Permissive = 0,
    /// 标准（默认）
    Standard = 1,
    /// 严格（生产模式）
    Strict = 2,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// 危险命令列表
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf",
    "rm -r",
    "del /f",
    "del /q",
    "format",
    "mkfs",
    "dd if=",
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
    "chmod 777",
    "chown",
    "sudo",
    "su ",
    "docker rm",
    "docker rmi",
    "docker system prune",
    "kubectl delete",
    "DROP TABLE",
    "DROP DATABASE",
    "TRUNCATE",
    "ALTER TABLE",
    "GRANT ALL",
    "REVOKE",
    ":(){ :|:& };:", // fork bomb
    "> /dev/sda",
    "> /dev/null",
    "mkfs.",
    "wget",
    "curl",
    "nc ",
    "ncat",
];

/// 危险路径模式
const DANGEROUS_PATHS: &[&str] = &[
    "/etc/",
    "/boot/",
    "/sys/",
    "/proc/",
    "/dev/",
    "C:\\Windows\\",
    "C:\\Windows\\System32\\",
    "~/.ssh",
    "~/.gnupg",
    "~/.aws",
    ".env",
    "credentials",
    "id_rsa",
    "id_ed25519",
];

/// 凭证模式（用于自动脱敏）
const CREDENTIAL_PATTERNS: &[&str] = &[
    "api_key",
    "api_secret",
    "password",
    "token",
    "secret",
    "private_key",
    "access_key",
    "auth_token",
    "bearer",
    "credential",
];

/// 命令检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheckResult {
    /// 是否通过
    pub allowed: bool,
    /// 风险等级（0-100）
    pub risk_score: u8,
    /// 拒绝原因
    #[serde(default)]
    pub reason: String,
    /// 建议
    #[serde(default)]
    pub suggestion: String,
}

impl Default for SecurityCheckResult {
    fn default() -> Self {
        Self {
            allowed: true,
            risk_score: 0,
            reason: String::new(),
            suggestion: String::new(),
        }
    }
}

/// 安全检查器
#[derive(Debug, Clone)]
pub struct SecurityChecker {
    level: SecurityLevel,
    /// 自定义危险命令
    custom_dangerous_commands: HashSet<String>,
    /// 自定义危险路径
    custom_dangerous_paths: HashSet<String>,
    /// 白名单命令（即使在危险列表中也可执行）
    command_whitelist: HashSet<String>,
}

impl SecurityChecker {
    /// 创建检查器
    pub fn new(level: SecurityLevel) -> Self {
        Self {
            level,
            custom_dangerous_commands: HashSet::new(),
            custom_dangerous_paths: HashSet::new(),
            command_whitelist: HashSet::new(),
        }
    }

    /// 添加自定义危险命令
    pub fn add_dangerous_command(&mut self, cmd: impl Into<String>) {
        self.custom_dangerous_commands.insert(cmd.into());
    }

    /// 添加自定义危险路径
    pub fn add_dangerous_path(&mut self, path: impl Into<String>) {
        self.custom_dangerous_paths.insert(path.into());
    }

    /// 添加白名单命令
    pub fn add_whitelist_command(&mut self, cmd: impl Into<String>) {
        self.command_whitelist.insert(cmd.into());
    }

    /// 检查命令是否危险
    pub fn check_command(&self, command: &str) -> SecurityCheckResult {
        if self.level == SecurityLevel::Permissive {
            return SecurityCheckResult::default();
        }

        let cmd_lower = command.to_ascii_lowercase();

        // 检查白名单
        if self.command_whitelist.iter().any(|w| cmd_lower.contains(w)) {
            return SecurityCheckResult::default();
        }

        // 检查内置危险命令
        for dangerous in DANGEROUS_COMMANDS {
            let d = dangerous.to_ascii_lowercase();
            if cmd_lower.contains(&d) {
                return SecurityCheckResult {
                    allowed: false,
                    risk_score: 90,
                    reason: format!("命令包含危险操作: {}", dangerous),
                    suggestion: "请使用更安全的替代方案，或手动确认执行".to_string(),
                };
            }
        }

        // 检查自定义危险命令
        for dangerous in &self.custom_dangerous_commands {
            if cmd_lower.contains(&dangerous.to_ascii_lowercase()) {
                return SecurityCheckResult {
                    allowed: false,
                    risk_score: 85,
                    reason: format!("命令包含自定义危险操作: {}", dangerous),
                    suggestion: "请手动确认执行".to_string(),
                };
            }
        }

        // 严格模式：额外检查
        if self.level == SecurityLevel::Strict {
            if cmd_lower.contains("curl") || cmd_lower.contains("wget") {
                return SecurityCheckResult {
                    allowed: false,
                    risk_score: 60,
                    reason: "严格模式下禁止网络下载命令".to_string(),
                    suggestion: "请在标准模式下执行，或手动确认".to_string(),
                };
            }
        }

        SecurityCheckResult::default()
    }

    /// 检查路径是否安全
    pub fn check_path(&self, path: &str) -> SecurityCheckResult {
        if self.level == SecurityLevel::Permissive {
            return SecurityCheckResult::default();
        }

        let path_lower = path.to_ascii_lowercase();

        // 检查路径逃逸
        if path.contains("..") {
            return SecurityCheckResult {
                allowed: false,
                risk_score: 80,
                reason: "路径包含 '..'（路径逃逸风险）".to_string(),
                suggestion: "请使用绝对路径".to_string(),
            };
        }

        // 检查危险路径
        for dangerous in DANGEROUS_PATHS {
            let d = dangerous.to_ascii_lowercase();
            if path_lower.contains(&d) || path_lower.starts_with(&d) {
                return SecurityCheckResult {
                    allowed: false,
                    risk_score: 85,
                    reason: format!("路径涉及敏感区域: {}", dangerous),
                    suggestion: "该路径受保护，不允许访问".to_string(),
                };
            }
        }

        for dangerous in &self.custom_dangerous_paths {
            if path_lower.contains(&dangerous.to_ascii_lowercase()) {
                return SecurityCheckResult {
                    allowed: false,
                    risk_score: 80,
                    reason: format!("路径涉及自定义敏感区域: {}", dangerous),
                    suggestion: "该路径受保护".to_string(),
                };
            }
        }

        SecurityCheckResult::default()
    }

    /// 净化输入（脱敏凭证信息）
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();
        for pattern in CREDENTIAL_PATTERNS {
            let pattern_lower = pattern.to_ascii_lowercase();
            // 查找包含凭证模式的行并脱敏
            let lines: Vec<String> = result
                .lines()
                .map(|line| {
                    if line.to_ascii_lowercase().contains(&pattern_lower) {
                        // 保留键名，脱敏值
                        if let Some(idx) = line.find(&['=', ':'][..]) {
                            format!("{}[REDACTED]", &line[..=idx])
                        } else {
                            format!("[REDACTED: {}]", pattern)
                        }
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            result = lines.join("\n");
        }
        result
    }

    /// 检查是否跨 Session 隔离
    pub fn check_session_isolation(&self, session_id: &str, allowed_sessions: &[String]) -> bool {
        if self.level == SecurityLevel::Permissive {
            return true;
        }
        allowed_sessions.contains(&session_id.to_string())
    }
}

impl Default for SecurityChecker {
    fn default() -> Self {
        Self::new(SecurityLevel::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_command_blocked() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let result = checker.check_command("rm -rf /tmp/test");
        assert!(!result.allowed);
        assert!(result.risk_score > 0);
    }

    #[test]
    fn test_safe_command_allowed() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let result = checker.check_command("ls -la");
        assert!(result.allowed);
    }

    #[test]
    fn test_permissive_allows_all() {
        let checker = SecurityChecker::new(SecurityLevel::Permissive);
        let result = checker.check_command("rm -rf /");
        assert!(result.allowed);
    }

    #[test]
    fn test_whitelist_bypass() {
        let mut checker = SecurityChecker::new(SecurityLevel::Standard);
        checker.add_whitelist_command("rm -rf /tmp/safe");
        let result = checker.check_command("rm -rf /tmp/safe");
        assert!(result.allowed);
    }

    #[test]
    fn test_path_escape_blocked() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let result = checker.check_path("../etc/passwd");
        assert!(!result.allowed);
    }

    #[test]
    fn test_sensitive_path_blocked() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let result = checker.check_path("/etc/passwd");
        assert!(!result.allowed);
    }

    #[test]
    fn test_safe_path_allowed() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let result = checker.check_path("/home/user/project/src/main.rs");
        assert!(result.allowed);
    }

    #[test]
    fn test_sanitize_credentials() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let input = "api_key=sk-1234567890\nname=test";
        let result = checker.sanitize(input);
        assert!(result.contains("REDACTED"));
        assert!(result.contains("name=test"));
    }

    #[test]
    fn test_session_isolation() {
        let checker = SecurityChecker::new(SecurityLevel::Standard);
        let allowed = vec!["session-1".to_string(), "session-2".to_string()];
        assert!(checker.check_session_isolation("session-1", &allowed));
        assert!(!checker.check_session_isolation("session-3", &allowed));
    }

    #[test]
    fn test_strict_mode_blocks_network() {
        let checker = SecurityChecker::new(SecurityLevel::Strict);
        let result = checker.check_command("curl http://example.com");
        assert!(!result.allowed);
    }
}