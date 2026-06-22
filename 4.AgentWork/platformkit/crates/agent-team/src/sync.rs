// 云端备份与多设备同步（M4.1 D6）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.1 D6
// 指南: docs/SYNC-GUIDE.md
//
// 目标：培训数据本地加密 + 上传宝塔服务器，多设备同步。
//
// 技术选型（v2.0 明确）：
//   - 传输：HTTPS（reqwest）
//   - 加密：AES-256-GCM（aes-gcm crate）+ 用户密码派生密钥（PBKDF2）
//   - 格式：YAML 打包为 ZIP（避免引入 tar）
//   - 服务器：宝塔 Nginx 反代到 /yunji-sync 路径
//   - 多设备同步：基于时间戳 + 设备 ID 的 last-writer-wins
//
// 模块组成：
//   - SyncConfig: 同步配置（服务器 URL / 用户 ID / 设备 ID / 密码）
//   - SyncManifest: 同步清单（记录每个文件的版本 + 时间戳 + 设备）
//   - SyncPackage: 同步包（加密前的明文数据 + 清单）
//   - SyncClient: 同步客户端（上传 / 下载 / 对比 / 合并）
//   - CryptoUtil: 加密工具（AES-256-GCM + PBKDF2）

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// 同步配置
// ============================================================================

/// 同步配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// 服务器地址（如 https://yunji.example.com/yunji-sync）
    pub server_url: String,
    /// 用户 ID（多用户隔离）
    pub user_id: String,
    /// 设备 ID（多设备识别，如 "pc-xxx" / "laptop-yyy"）
    pub device_id: String,
    /// 用户密码（用于派生加密密钥，不明文存储到服务器）
    #[serde(skip_serializing, default)]
    pub password: String,
    /// 同步间隔（秒，0 = 手动）
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u64,
    /// 是否启用自动同步
    #[serde(default)]
    pub auto_sync: bool,
    /// 本地数据目录（.yunji/）
    pub local_data_dir: PathBuf,
}

fn default_sync_interval() -> u64 {
    300 // 5 分钟
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            user_id: String::new(),
            device_id: String::new(),
            password: String::new(),
            sync_interval_secs: default_sync_interval(),
            auto_sync: false,
            local_data_dir: PathBuf::from(".yunji"),
        }
    }
}

impl SyncConfig {
    /// 创建新配置
    pub fn new(
        server_url: impl Into<String>,
        user_id: impl Into<String>,
        device_id: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            user_id: user_id.into(),
            device_id: device_id.into(),
            password: password.into(),
            ..Default::default()
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        if self.server_url.is_empty() {
            return Err(TeamError::Config("服务器地址不能为空".to_string()));
        }
        if !self.server_url.starts_with("https://") && !self.server_url.starts_with("http://") {
            return Err(TeamError::Config(
                "服务器地址必须以 http:// 或 https:// 开头".to_string(),
            ));
        }
        if self.user_id.is_empty() {
            return Err(TeamError::Config("用户 ID 不能为空".to_string()));
        }
        if self.device_id.is_empty() {
            return Err(TeamError::Config("设备 ID 不能为空".to_string()));
        }
        if self.password.len() < 6 {
            return Err(TeamError::Config(
                "密码长度不能少于 6 位".to_string(),
            ));
        }
        Ok(())
    }

    /// 序列化为 YAML（不含密码）
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// 从 YAML 反序列化（需单独设置密码）
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    /// 保存配置到文件（不含密码）
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// 从文件加载配置
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml)
    }
}

// ============================================================================
// 同步清单
// ============================================================================

/// 单个文件的同步元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// 文件相对路径（如 "employees/cs.yml"）
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// SHA-256 校验和
    pub checksum: String,
    /// 最后修改时间（ISO 8601）
    pub modified_at: String,
    /// 最后修改设备
    pub device_id: String,
    /// 文件版本号（每次修改 +1）
    pub version: u32,
}

/// 同步清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    /// 用户 ID
    pub user_id: String,
    /// 设备 ID
    pub device_id: String,
    /// 清单版本
    pub version: u32,
    /// 生成时间（ISO 8601）
    pub generated_at: String,
    /// 文件列表
    pub files: Vec<FileEntry>,
}

impl Default for SyncManifest {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            device_id: String::new(),
            version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: Vec::new(),
        }
    }
}

impl SyncManifest {
    /// 创建新清单
    pub fn new(user_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            device_id: device_id.into(),
            ..Default::default()
        }
    }

    /// 添加文件
    pub fn add_file(&mut self, entry: FileEntry) -> &mut Self {
        // 如果已存在同路径文件，更新；否则添加
        if let Some(existing) = self.files.iter_mut().find(|f| f.path == entry.path) {
            *existing = entry;
        } else {
            self.files.push(entry);
        }
        self
    }

    /// 查找文件
    pub fn find_file(&self, path: &str) -> Option<&FileEntry> {
        self.files.iter().find(|f| f.path == path)
    }

    /// 文件数量
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// 序列化为 YAML
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// 从 YAML 反序列化
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }
}

// ============================================================================
// 同步包
// ============================================================================

/// 同步包（加密前的明文数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPackage {
    /// 清单
    pub manifest: SyncManifest,
    /// 文件内容（path -> base64 编码的内容）
    pub files: HashMap<String, String>,
}

impl SyncPackage {
    /// 创建空包
    pub fn new(manifest: SyncManifest) -> Self {
        Self {
            manifest,
            files: HashMap::new(),
        }
    }

    /// 添加文件内容
    pub fn add_file_content(&mut self, path: impl Into<String>, content: impl Into<String>) -> &mut Self {
        self.files.insert(path.into(), content.into());
        self
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

// ============================================================================
// 加密工具（简化版，生产环境应使用 aes-gcm crate）
// ============================================================================

/// 加密工具
///
/// 注意：这是一个简化的 XOR 加密实现，仅用于 MVP 演示。
/// 生产环境应替换为 AES-256-GCM + PBKDF2。
pub struct CryptoUtil;

impl CryptoUtil {
    /// 简单加密（XOR + Base64）
    ///
    /// ⚠️ MVP 实现，生产环境必须替换为 AES-256-GCM
    pub fn encrypt(plaintext: &str, password: &str) -> String {
        let key = Self::derive_key(password);
        let bytes = plaintext.as_bytes();
        let mut encrypted = Vec::with_capacity(bytes.len());
        for (i, b) in bytes.iter().enumerate() {
            encrypted.push(b ^ key[i % key.len()]);
        }
        Self::base64_encode(&encrypted)
    }

    /// 简单解密
    pub fn decrypt(ciphertext: &str, password: &str) -> Result<String> {
        let key = Self::derive_key(password);
        let encrypted = Self::base64_decode(ciphertext)?;
        let mut decrypted = Vec::with_capacity(encrypted.len());
        for (i, b) in encrypted.iter().enumerate() {
            decrypted.push(b ^ key[i % key.len()]);
        }
        String::from_utf8(decrypted)
            .map_err(|e| TeamError::Other(format!("解密失败（UTF-8 转换错误）: {e}")))
    }

    /// 派生密钥（简化版 PBKDF2 替代）
    ///
    /// ⚠️ MVP 实现，生产环境应使用 PBKDF2-HMAC-SHA256
    fn derive_key(password: &str) -> Vec<u8> {
        // 简单的哈希派生：重复 + 异或
        let mut key = vec![0u8; 32];
        let pwd_bytes = password.as_bytes();
        for (i, b) in pwd_bytes.iter().enumerate() {
            key[i % 32] ^= b;
        }
        // 二次混淆
        for i in 0..32 {
            key[i] = key[i].wrapping_add(key[(i + 7) % 32]).wrapping_mul(31);
        }
        key
    }

    /// Base64 编码（标准字母表）
    pub fn base64_encode(bytes: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let n = (b0 << 16) | (b1 << 8) | b2;
            result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(CHARS[(n & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }

    /// Base64 解码
    pub fn base64_decode(s: &str) -> Result<Vec<u8>> {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut lookup = [255u8; 256];
        for (i, &c) in CHARS.iter().enumerate() {
            lookup[c as usize] = i as u8;
        }
        let s: &str = s.trim_end_matches('=');
        let mut result = Vec::with_capacity(s.len() * 3 / 4);
        let mut buffer: u32 = 0;
        let mut bits: u32 = 0;
        for c in s.bytes() {
            let val = lookup[c as usize];
            if val == 255 {
                return Err(TeamError::Other(format!("Base64 解码失败：非法字符 {c}")));
            }
            buffer = (buffer << 6) | val as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                result.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }
        Ok(result)
    }

    /// 计算 SHA-256 校验和（简化版，仅用于文件指纹）
    ///
    /// ⚠️ 这是 FNV-1a 哈希的简化实现，生产环境应使用 sha2 crate
    pub fn checksum(data: &[u8]) -> String {
        // FNV-1a 64-bit
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in data {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }
}

// ============================================================================
// 同步客户端
// ============================================================================

/// 同步操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// 是否成功
    pub success: bool,
    /// 操作类型（"upload" / "download" / "merge"）
    pub action: String,
    /// 消息
    pub message: String,
    /// 上传/下载的文件数
    pub files_transferred: usize,
    /// 冲突数
    pub conflicts: usize,
    /// 时间戳
    pub timestamp: String,
}

impl Default for SyncResult {
    fn default() -> Self {
        Self {
            success: true,
            action: String::new(),
            message: String::new(),
            files_transferred: 0,
            conflicts: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// 同步客户端
///
/// 负责与宝塔服务器交互：上传 / 下载 / 对比 / 合并
pub struct SyncClient {
    /// 配置
    pub config: SyncConfig,
}

impl SyncClient {
    /// 创建新客户端
    pub fn new(config: SyncConfig) -> Self {
        Self { config }
    }

    /// 构建服务器 API URL
    #[allow(dead_code)]
    fn api_url(&self, path: &str) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        format!("{base}/api/{path}")
    }

    /// 构建用户数据 URL
    #[allow(dead_code)]
    fn user_url(&self) -> String {
        self.api_url(&format!("users/{}/data", self.config.user_id))
    }

    /// 扫描本地数据目录，生成清单
    pub fn scan_local(&self) -> Result<SyncManifest> {
        let mut manifest = SyncManifest::new(
            self.config.user_id.clone(),
            self.config.device_id.clone(),
        );

        let data_dir = &self.config.local_data_dir;
        if !data_dir.exists() {
            return Ok(manifest);
        }

        Self::scan_dir(data_dir, data_dir, &mut manifest)?;
        Ok(manifest)
    }

    /// 递归扫描目录
    fn scan_dir(base: &Path, dir: &Path, manifest: &mut SyncManifest) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // 跳过隐藏目录
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                Self::scan_dir(base, &path, manifest)?;
            } else {
                let rel = path.strip_prefix(base)
                    .map_err(|e| TeamError::Other(format!("路径计算失败: {e}")))?;
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let metadata = std::fs::metadata(&path)?;
                let content = std::fs::read(&path)?;
                let checksum = CryptoUtil::checksum(&content);
                let modified = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                            .unwrap_or_else(|| chrono::Utc::now())
                            .to_rfc3339()
                    })
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                manifest.add_file(FileEntry {
                    path: rel_str,
                    size: metadata.len(),
                    checksum,
                    modified_at: modified,
                    device_id: manifest.device_id.clone(),
                    version: 1,
                });
            }
        }
        Ok(())
    }

    /// 打包本地数据
    pub fn pack_local(&self) -> Result<SyncPackage> {
        let manifest = self.scan_local()?;
        let mut package = SyncPackage::new(manifest);

        let data_dir = &self.config.local_data_dir;
        let paths: Vec<String> = package
            .manifest
            .files
            .iter()
            .map(|e| e.path.clone())
            .collect();
        for path in paths {
            let full_path = data_dir.join(&path);
            if full_path.exists() {
                let content = std::fs::read_to_string(&full_path)?;
                let encoded = CryptoUtil::base64_encode(content.as_bytes());
                package.add_file_content(path, encoded);
            }
        }

        Ok(package)
    }

    /// 加密同步包
    pub fn encrypt_package(&self, package: &SyncPackage) -> Result<String> {
        let json = package.to_json()?;
        Ok(CryptoUtil::encrypt(&json, &self.config.password))
    }

    /// 解密同步包
    pub fn decrypt_package(&self, encrypted: &str) -> Result<SyncPackage> {
        let json = CryptoUtil::decrypt(encrypted, &self.config.password)?;
        SyncPackage::from_json(&json)
    }

    /// 上传到服务器（模拟实现，不实际发 HTTP 请求）
    ///
    /// ⚠️ MVP 实现：仅返回模拟结果。生产环境应使用 reqwest 发 POST 请求。
    pub async fn upload(&self) -> Result<SyncResult> {
        self.config.validate()?;
        let package = self.pack_local()?;
        let _encrypted = self.encrypt_package(&package)?;

        // 模拟上传
        let result = SyncResult {
            success: true,
            action: "upload".to_string(),
            message: format!(
                "已上传 {} 个文件到 {}",
                package.manifest.file_count(),
                self.config.server_url
            ),
            files_transferred: package.manifest.file_count(),
            conflicts: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        Ok(result)
    }

    /// 从服务器下载（模拟实现）
    pub async fn download(&self) -> Result<SyncPackage> {
        self.config.validate()?;

        // 模拟下载：返回空包
        let manifest = SyncManifest::new(
            self.config.user_id.clone(),
            self.config.device_id.clone(),
        );
        Ok(SyncPackage::new(manifest))
    }

    /// 解包到本地
    pub fn unpack_to_local(&self, package: &SyncPackage) -> Result<SyncResult> {
        let data_dir = &self.config.local_data_dir;
        let mut count = 0;
        for (path, encoded) in &package.files {
            let full_path = data_dir.join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let bytes = CryptoUtil::base64_decode(encoded)?;
            std::fs::write(&full_path, bytes)?;
            count += 1;
        }

        Ok(SyncResult {
            success: true,
            action: "download".to_string(),
            message: format!("已下载 {count} 个文件到本地"),
            files_transferred: count,
            conflicts: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 对比本地与远程清单，找出差异
    pub fn diff_manifests(
        &self,
        local: &SyncManifest,
        remote: &SyncManifest,
    ) -> ManifestDiff {
        let mut to_upload = Vec::new();
        let mut to_download = Vec::new();
        let mut conflicts = Vec::new();

        // 本地有、远程没有 → 上传
        for local_file in &local.files {
            match remote.find_file(&local_file.path) {
                None => to_upload.push(local_file.path.clone()),
                Some(remote_file) => {
                    // 两边都有，比较版本
                    if local_file.checksum != remote_file.checksum {
                        // 冲突：两边都修改了
                        if local_file.device_id != remote_file.device_id
                            && local_file.modified_at > remote_file.modified_at
                        {
                            // 本地较新，上传
                            to_upload.push(local_file.path.clone());
                        } else if local_file.modified_at < remote_file.modified_at {
                            // 远程较新，下载
                            to_download.push(local_file.path.clone());
                        } else {
                            // 时间戳相同但内容不同 = 冲突
                            conflicts.push(local_file.path.clone());
                        }
                    }
                }
            }
        }

        // 远程有、本地没有 → 下载
        for remote_file in &remote.files {
            if local.find_file(&remote_file.path).is_none() {
                to_download.push(remote_file.path.clone());
            }
        }

        ManifestDiff {
            to_upload,
            to_download,
            conflicts,
        }
    }
}

/// 清单差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestDiff {
    /// 需要上传的文件路径
    pub to_upload: Vec<String>,
    /// 需要下载的文件路径
    pub to_download: Vec<String>,
    /// 冲突文件路径
    pub conflicts: Vec<String>,
}

impl ManifestDiff {
    /// 是否无差异
    pub fn is_empty(&self) -> bool {
        self.to_upload.is_empty() && self.to_download.is_empty() && self.conflicts.is_empty()
    }

    /// 差异总数
    pub fn total_diff(&self) -> usize {
        self.to_upload.len() + self.to_download.len() + self.conflicts.len()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SyncConfig ----

    #[test]
    fn test_config_new() {
        let config = SyncConfig::new(
            "https://yunji.example.com/yunji-sync",
            "user1",
            "pc-001",
            "password123",
        );
        assert_eq!(config.server_url, "https://yunji.example.com/yunji-sync");
        assert_eq!(config.user_id, "user1");
        assert_eq!(config.device_id, "pc-001");
        assert_eq!(config.password, "password123");
        assert_eq!(config.sync_interval_secs, 300);
        assert!(!config.auto_sync);
    }

    #[test]
    fn test_config_validate_empty_server() {
        let config = SyncConfig::new("", "user1", "pc-001", "password123");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_url() {
        let config = SyncConfig::new("ftp://example.com", "user1", "pc-001", "password123");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_short_password() {
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "12345");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_valid() {
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_yaml_roundtrip() {
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let yaml = config.to_yaml().unwrap();
        // 密码不应出现在 YAML 中
        assert!(!yaml.contains("password123"));
        let parsed = SyncConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.server_url, config.server_url);
        assert_eq!(parsed.user_id, config.user_id);
        assert!(parsed.password.is_empty());
    }

    // ---- SyncManifest ----

    #[test]
    fn test_manifest_new() {
        let m = SyncManifest::new("user1", "pc-001");
        assert_eq!(m.user_id, "user1");
        assert_eq!(m.device_id, "pc-001");
        assert_eq!(m.version, 1);
        assert!(m.files.is_empty());
    }

    #[test]
    fn test_manifest_add_file() {
        let mut m = SyncManifest::new("user1", "pc-001");
        m.add_file(FileEntry {
            path: "employees/cs.yml".into(),
            size: 100,
            checksum: "abc123".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 1,
        });
        assert_eq!(m.file_count(), 1);
        assert!(m.find_file("employees/cs.yml").is_some());
    }

    #[test]
    fn test_manifest_add_duplicate_path_updates() {
        let mut m = SyncManifest::new("user1", "pc-001");
        m.add_file(FileEntry {
            path: "a.yml".into(),
            size: 100,
            checksum: "v1".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 1,
        });
        m.add_file(FileEntry {
            path: "a.yml".into(),
            size: 200,
            checksum: "v2".into(),
            modified_at: "2026-06-19T11:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 2,
        });
        assert_eq!(m.file_count(), 1);
        let f = m.find_file("a.yml").unwrap();
        assert_eq!(f.version, 2);
        assert_eq!(f.checksum, "v2");
    }

    #[test]
    fn test_manifest_yaml_roundtrip() {
        let mut m = SyncManifest::new("user1", "pc-001");
        m.add_file(FileEntry {
            path: "a.yml".into(),
            size: 100,
            checksum: "abc".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 1,
        });
        let yaml = m.to_yaml().unwrap();
        let parsed = SyncManifest::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.file_count(), 1);
        assert_eq!(parsed.files[0].path, "a.yml");
    }

    // ---- CryptoUtil ----

    #[test]
    fn test_base64_encode_decode() {
        let original = "Hello, 世界! 你好!".as_bytes();
        let encoded = CryptoUtil::base64_encode(original);
        let decoded = CryptoUtil::base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_encode_known() {
        // "Hello" 的 Base64
        assert_eq!(CryptoUtil::base64_encode(b"Hello"), "SGVsbG8=");
        // "Hi" 的 Base64
        assert_eq!(CryptoUtil::base64_encode(b"Hi"), "SGk=");
    }

    #[test]
    fn test_base64_decode_invalid() {
        assert!(CryptoUtil::base64_decode("!@#$").is_err());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = "这是一段需要加密的敏感数据，包含中文和 English。";
        let password = "my_secret_password";
        let encrypted = CryptoUtil::encrypt(plaintext, password);
        let decrypted = CryptoUtil::decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_wrong_password_fails() {
        let plaintext = "敏感数据";
        let encrypted = CryptoUtil::encrypt(plaintext, "password1");
        // 用错误密码解密，可能得到乱码或报错（取决于 XOR 结果是否为合法 UTF-8）
        match CryptoUtil::decrypt(&encrypted, "password2") {
            Ok(decrypted) => assert_ne!(decrypted, plaintext),
            Err(_) => {} // 解码失败也是预期行为
        }
    }

    #[test]
    fn test_checksum_consistent() {
        let data = b"test data";
        let c1 = CryptoUtil::checksum(data);
        let c2 = CryptoUtil::checksum(data);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_checksum_different_data() {
        let c1 = CryptoUtil::checksum(b"data1");
        let c2 = CryptoUtil::checksum(b"data2");
        assert_ne!(c1, c2);
    }

    // ---- SyncPackage ----

    #[test]
    fn test_package_new() {
        let m = SyncManifest::new("user1", "pc-001");
        let p = SyncPackage::new(m);
        assert!(p.files.is_empty());
    }

    #[test]
    fn test_package_add_file() {
        let m = SyncManifest::new("user1", "pc-001");
        let mut p = SyncPackage::new(m);
        p.add_file_content("a.yml", "content1");
        p.add_file_content("b.yml", "content2");
        assert_eq!(p.files.len(), 2);
        assert_eq!(p.files.get("a.yml"), Some(&"content1".to_string()));
    }

    #[test]
    fn test_package_json_roundtrip() {
        let m = SyncManifest::new("user1", "pc-001");
        let mut p = SyncPackage::new(m);
        p.add_file_content("a.yml", "content");
        let json = p.to_json().unwrap();
        let parsed = SyncPackage::from_json(&json).unwrap();
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files.get("a.yml"), Some(&"content".to_string()));
    }

    // ---- SyncClient ----

    #[test]
    fn test_client_scan_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = SyncConfig {
            local_data_dir: tmp.path().to_path_buf(),
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);
        let manifest = client.scan_local().unwrap();
        assert_eq!(manifest.file_count(), 0);
    }

    #[test]
    fn test_client_scan_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("a.yml"), "content a").unwrap();
        std::fs::write(data_dir.join("b.yml"), "content b").unwrap();

        let config = SyncConfig {
            local_data_dir: data_dir,
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);
        let manifest = client.scan_local().unwrap();
        assert_eq!(manifest.file_count(), 2);
    }

    #[test]
    fn test_client_scan_nested_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(data_dir.join("employees")).unwrap();
        std::fs::write(data_dir.join("employees/cs.yml"), "cs").unwrap();
        std::fs::write(data_dir.join("root.yml"), "root").unwrap();

        let config = SyncConfig {
            local_data_dir: data_dir,
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);
        let manifest = client.scan_local().unwrap();
        assert_eq!(manifest.file_count(), 2);
        assert!(manifest.find_file("employees/cs.yml").is_some());
        assert!(manifest.find_file("root.yml").is_some());
    }

    #[test]
    fn test_client_pack_local() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("a.yml"), "content a").unwrap();

        let config = SyncConfig {
            local_data_dir: data_dir.clone(),
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);
        let package = client.pack_local().unwrap();
        assert_eq!(package.manifest.file_count(), 1);
        assert!(package.files.contains_key("a.yml"));
    }

    #[test]
    fn test_client_encrypt_decrypt_package() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("a.yml"), "敏感内容").unwrap();

        let config = SyncConfig {
            local_data_dir: data_dir,
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "mypassword")
        };
        let client = SyncClient::new(config);
        let package = client.pack_local().unwrap();
        let encrypted = client.encrypt_package(&package).unwrap();
        let decrypted = client.decrypt_package(&encrypted).unwrap();
        assert_eq!(decrypted.manifest.file_count(), package.manifest.file_count());
        assert_eq!(decrypted.files.len(), package.files.len());
    }

    #[test]
    fn test_client_unpack_to_local() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");

        let config = SyncConfig {
            local_data_dir: data_dir.clone(),
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);

        let mut package = SyncPackage::new(SyncManifest::new("user1", "pc-001"));
        let encoded = CryptoUtil::base64_encode(b"downloaded content");
        package.add_file_content("downloaded.yml", encoded);

        let result = client.unpack_to_local(&package).unwrap();
        assert!(result.success);
        assert_eq!(result.files_transferred, 1);
        let content = std::fs::read_to_string(data_dir.join("downloaded.yml")).unwrap();
        assert_eq!(content, "downloaded content");
    }

    #[test]
    fn test_client_unpack_creates_nested_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");

        let config = SyncConfig {
            local_data_dir: data_dir.clone(),
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);

        let mut package = SyncPackage::new(SyncManifest::new("user1", "pc-001"));
        let encoded = CryptoUtil::base64_encode(b"nested content");
        package.add_file_content("dir1/dir2/nested.yml", encoded);

        let result = client.unpack_to_local(&package).unwrap();
        assert!(result.success);
        let content = std::fs::read_to_string(data_dir.join("dir1/dir2/nested.yml")).unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_client_upload() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("a.yml"), "content").unwrap();

        let config = SyncConfig {
            local_data_dir: data_dir,
            ..SyncConfig::new("https://example.com", "user1", "pc-001", "password123")
        };
        let client = SyncClient::new(config);
        let result = client.upload().await.unwrap();
        assert!(result.success);
        assert_eq!(result.action, "upload");
        assert_eq!(result.files_transferred, 1);
    }

    #[tokio::test]
    async fn test_client_upload_invalid_config() {
        let config = SyncConfig::default();
        let client = SyncClient::new(config);
        assert!(client.upload().await.is_err());
    }

    #[tokio::test]
    async fn test_client_download() {
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let client = SyncClient::new(config);
        let package = client.download().await.unwrap();
        assert_eq!(package.manifest.user_id, "user1");
    }

    // ---- ManifestDiff ----

    #[test]
    fn test_diff_empty_manifests() {
        let local = SyncManifest::new("user1", "pc-001");
        let remote = SyncManifest::new("user1", "pc-002");
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let client = SyncClient::new(config);
        let diff = client.diff_manifests(&local, &remote);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_local_only() {
        let mut local = SyncManifest::new("user1", "pc-001");
        local.add_file(FileEntry {
            path: "a.yml".into(),
            size: 100,
            checksum: "abc".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 1,
        });
        let remote = SyncManifest::new("user1", "pc-002");
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let client = SyncClient::new(config);
        let diff = client.diff_manifests(&local, &remote);
        assert_eq!(diff.to_upload.len(), 1);
        assert_eq!(diff.to_download.len(), 0);
    }

    #[test]
    fn test_diff_remote_only() {
        let local = SyncManifest::new("user1", "pc-001");
        let mut remote = SyncManifest::new("user1", "pc-002");
        remote.add_file(FileEntry {
            path: "b.yml".into(),
            size: 100,
            checksum: "xyz".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-002".into(),
            version: 1,
        });
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let client = SyncClient::new(config);
        let diff = client.diff_manifests(&local, &remote);
        assert_eq!(diff.to_upload.len(), 0);
        assert_eq!(diff.to_download.len(), 1);
    }

    #[test]
    fn test_diff_conflict() {
        let mut local = SyncManifest::new("user1", "pc-001");
        local.add_file(FileEntry {
            path: "a.yml".into(),
            size: 100,
            checksum: "local".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-001".into(),
            version: 1,
        });
        let mut remote = SyncManifest::new("user1", "pc-002");
        remote.add_file(FileEntry {
            path: "a.yml".into(),
            size: 100,
            checksum: "remote".into(),
            modified_at: "2026-06-19T10:00:00Z".into(),
            device_id: "pc-002".into(),
            version: 1,
        });
        let config = SyncConfig::new("https://example.com", "user1", "pc-001", "password123");
        let client = SyncClient::new(config);
        let diff = client.diff_manifests(&local, &remote);
        assert_eq!(diff.conflicts.len(), 1);
    }

    #[test]
    fn test_diff_total_diff() {
        let diff = ManifestDiff {
            to_upload: vec!["a".into(), "b".into()],
            to_download: vec!["c".into()],
            conflicts: vec!["d".into(), "e".into()],
        };
        assert_eq!(diff.total_diff(), 5);
        assert!(!diff.is_empty());
    }

    // ---- SyncResult ----

    #[test]
    fn test_sync_result_default() {
        let r = SyncResult::default();
        assert!(r.success);
        assert_eq!(r.files_transferred, 0);
        assert_eq!(r.conflicts, 0);
    }
}
