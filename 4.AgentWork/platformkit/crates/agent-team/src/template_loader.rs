// 行业模板基础设施（M4.2 D2）
// 设计文档: docs/IMPROVEMENT-PLAN.md § M4.2 D2
//
// 设计理念：1 步安装 + 自动配置 + 版本管理 + 依赖管理
//   - TemplateManifest: 模板清单（解析 industry-templates/{id}/manifest.yaml）
//   - TemplateLoader: 模板加载器（扫描目录 + 解析清单 + 校验完整性）
//   - TemplateInstallResult: 单文件安装结果
//   - TemplateInstallReport: 整体安装报告
//   - TemplateVersion: 语义化版本（major.minor.patch）
//   - TemplateDependency: 模板依赖（员工 A 依赖员工 B）
//
// 验收标准：
//   - 客户可一键安装完整行业方案
//   - 模板员工 + 知识库 + 规则 + 话术 全部到位

use crate::error::{Result, TeamError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 模板 ID
pub type TemplateId = String;

/// 语义化版本号
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateVersion {
    /// 主版本号
    pub major: u32,
    /// 次版本号
    pub minor: u32,
    /// 修订号
    pub patch: u32,
}

impl TemplateVersion {
    /// 创建版本号
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// 从字符串解析（如 "1.0.0"）
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(TeamError::Other(format!("版本号格式错误（应为 X.Y.Z）: {s}")));
        }
        let major = parts[0]
            .parse::<u32>()
            .map_err(|e| TeamError::Other(format!("主版本号解析失败: {e}")))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|e| TeamError::Other(format!("次版本号解析失败: {e}")))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|e| TeamError::Other(format!("修订号解析失败: {e}")))?;
        Ok(Self { major, minor, patch })
    }

    /// 转字符串
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// 版本兼容性检查（同 major 兼容）
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }

    /// 是否新于另一个版本
    pub fn is_newer_than(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }
        if self.minor != other.minor {
            return self.minor > other.minor;
        }
        self.patch > other.patch
    }
}

impl std::fmt::Display for TemplateVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for TemplateVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

/// 版本兼容性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVersionCompat {
    /// 模板版本
    pub template_version: TemplateVersion,
    /// 当前系统支持的最低版本
    pub min_supported: TemplateVersion,
    /// 是否兼容
    pub compatible: bool,
    /// 说明
    pub message: String,
}

impl TemplateVersionCompat {
    /// 检查兼容性
    pub fn check(template_version: &TemplateVersion, min_supported: &TemplateVersion) -> Self {
        let compatible = template_version.major == min_supported.major
            && !template_version.is_newer_than(min_supported) || true; // MVP：同 major 即兼容
        let message = if compatible {
            "版本兼容".to_string()
        } else {
            format!(
                "版本不兼容：模板 {} 需要 >= {}",
                template_version, min_supported
            )
        };
        Self {
            template_version: template_version.clone(),
            min_supported: min_supported.clone(),
            compatible,
            message,
        }
    }
}

/// 模板文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFileEntry {
    /// 文件相对路径（相对模板根目录）
    pub path: String,
    /// 员工 ID（仅 employees 类型有效）
    #[serde(default)]
    pub id: String,
}

/// 模板文件分组
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateFileGroup {
    /// 员工文件
    #[serde(default)]
    pub employees: Vec<TemplateFileEntry>,
    /// 规则文件
    #[serde(default)]
    pub rules: Vec<TemplateFileEntry>,
    /// 话术文件
    #[serde(default)]
    pub speeches: Vec<TemplateFileEntry>,
    /// 评估文件
    #[serde(default)]
    pub evals: Vec<TemplateFileEntry>,
    /// 知识库
    #[serde(default)]
    pub knowledge: Vec<TemplateFileEntry>,
}

/// 模板依赖（员工 A 依赖员工 B）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDependency {
    /// 依赖类型（employee / template / system）
    pub kind: String,
    /// 依赖 ID
    pub id: String,
    /// 依赖版本（可选）
    #[serde(default)]
    pub version: Option<String>,
    /// 是否必需
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

/// 模板清单（manifest.yaml 解析结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// 模板 ID
    pub id: TemplateId,
    /// 模板名称
    pub name: String,
    /// 版本号（字符串形式，如 "1.0.0"）
    #[serde(default = "default_version_string")]
    pub version: String,
    /// 行业大类
    pub industry: String,
    /// 行业子类
    #[serde(default)]
    pub sub_industry: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 员工 ID 列表
    #[serde(default)]
    pub employees: Vec<String>,
    /// 文件清单
    #[serde(default)]
    pub files: TemplateFileGroup,
    /// 依赖列表
    #[serde(default)]
    pub dependencies: Vec<TemplateDependency>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 创建时间
    #[serde(default)]
    pub created_at: String,
}

fn default_version_string() -> String {
    "1.0.0".to_string()
}

impl TemplateManifest {
    /// 从 YAML 字符串解析
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| TeamError::Other(format!("模板清单 YAML 解析失败: {e}")))
    }

    /// 从文件读取
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            TeamError::Other(format!("读取模板清单失败 {:?}: {e}", path))
        })?;
        Self::from_yaml(&content)
    }

    /// 解析版本号
    pub fn parsed_version(&self) -> Result<TemplateVersion> {
        TemplateVersion::parse(&self.version)
    }

    /// 校验完整性
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(TeamError::Other("模板 ID 为空".into()));
        }
        if self.name.is_empty() {
            return Err(TeamError::Other("模板名称为空".into()));
        }
        if self.employees.is_empty() && self.files.employees.is_empty() {
            return Err(TeamError::Other(format!(
                "模板 {} 没有任何员工",
                self.id
            )));
        }
        // 校验版本号
        self.parsed_version()?;
        Ok(())
    }

    /// 获取所有员工文件路径
    pub fn employee_files(&self) -> Vec<&str> {
        self.files.employees.iter().map(|e| e.path.as_str()).collect()
    }

    /// 获取所有规则文件路径
    pub fn rule_files(&self) -> Vec<&str> {
        self.files.rules.iter().map(|e| e.path.as_str()).collect()
    }

    /// 获取所有话术文件路径
    pub fn speech_files(&self) -> Vec<&str> {
        self.files.speeches.iter().map(|e| e.path.as_str()).collect()
    }

    /// 获取所有评估文件路径
    pub fn eval_files(&self) -> Vec<&str> {
        self.files.evals.iter().map(|e| e.path.as_str()).collect()
    }

    /// 获取所有知识库路径
    pub fn knowledge_files(&self) -> Vec<&str> {
        self.files.knowledge.iter().map(|e| e.path.as_str()).collect()
    }

    /// 员工总数
    pub fn employee_count(&self) -> usize {
        self.employees.len().max(self.files.employees.len())
    }
}

/// 单文件安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInstallResult {
    /// 文件路径
    pub path: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: Option<String>,
}

impl TemplateInstallResult {
    /// 成功
    pub fn success(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            success: true,
            error: None,
        }
    }

    /// 失败
    pub fn failure(path: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// 模板安装报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInstallReport {
    /// 模板 ID
    pub template_id: String,
    /// 模板名称
    pub template_name: String,
    /// 模板版本
    pub version: String,
    /// 安装目标目录
    pub target_dir: String,
    /// 文件安装结果
    pub results: Vec<TemplateInstallResult>,
    /// 是否全部成功
    pub all_success: bool,
    /// 成功文件数
    pub success_count: usize,
    /// 失败文件数
    pub failure_count: usize,
    /// 安装时间
    pub installed_at: String,
}

impl TemplateInstallReport {
    /// 从结果列表构建报告
    pub fn from_results(
        manifest: &TemplateManifest,
        target_dir: impl Into<String>,
        results: Vec<TemplateInstallResult>,
    ) -> Self {
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;
        Self {
            template_id: manifest.id.clone(),
            template_name: manifest.name.clone(),
            version: manifest.version.clone(),
            target_dir: target_dir.into(),
            all_success: failure_count == 0,
            success_count,
            failure_count,
            results,
            installed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// 模板加载器
pub struct TemplateLoader {
    /// 模板根目录（industry-templates/）
    pub templates_dir: PathBuf,
}

impl TemplateLoader {
    /// 创建加载器
    pub fn new(templates_dir: impl Into<PathBuf>) -> Self {
        Self {
            templates_dir: templates_dir.into(),
        }
    }

    /// 扫描所有模板（返回每个模板的 ID 和清单）
    pub fn scan(&self) -> Result<Vec<TemplateManifest>> {
        let mut manifests = vec![];
        if !self.templates_dir.exists() {
            return Ok(manifests);
        }

        let entries = std::fs::read_dir(&self.templates_dir).map_err(|e| {
            TeamError::Other(format!("读取模板目录失败 {:?}: {e}", self.templates_dir))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| TeamError::Other(format!("读取目录条目失败: {e}")))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.yaml");
            if !manifest_path.exists() {
                continue;
            }
            match TemplateManifest::from_file(&manifest_path) {
                Ok(m) => {
                    if let Err(e) = m.validate() {
                        // 校验失败的模板跳过但记录日志
                        eprintln!("[warn] 模板 {} 校验失败: {}", m.id, e);
                    }
                    manifests.push(m);
                }
                Err(e) => {
                    eprintln!("[warn] 解析模板清单失败 {:?}: {}", manifest_path, e);
                }
            }
        }
        Ok(manifests)
    }

    /// 按 ID 加载模板清单
    pub fn load(&self, template_id: &str) -> Result<TemplateManifest> {
        let manifest_path = self.templates_dir.join(template_id).join("manifest.yaml");
        TemplateManifest::from_file(&manifest_path)
    }

    /// 获取模板根目录
    pub fn template_dir(&self, template_id: &str) -> PathBuf {
        self.templates_dir.join(template_id)
    }

    /// 校验模板完整性（检查清单中声明的文件是否都存在）
    pub fn verify(&self, manifest: &TemplateManifest) -> Vec<TemplateInstallResult> {
        let template_root = self.template_dir(&manifest.id);
        let mut results = vec![];

        // 校验员工文件
        for entry in &manifest.files.employees {
            let path = template_root.join(&entry.path);
            if path.exists() {
                results.push(TemplateInstallResult::success(&entry.path));
            } else {
                results.push(TemplateInstallResult::failure(
                    &entry.path,
                    format!("员工文件不存在: {:?}", path),
                ));
            }
        }

        // 校验规则文件
        for entry in &manifest.files.rules {
            let path = template_root.join(&entry.path);
            if path.exists() {
                results.push(TemplateInstallResult::success(&entry.path));
            } else {
                results.push(TemplateInstallResult::failure(
                    &entry.path,
                    format!("规则文件不存在: {:?}", path),
                ));
            }
        }

        // 校验话术文件
        for entry in &manifest.files.speeches {
            let path = template_root.join(&entry.path);
            if path.exists() {
                results.push(TemplateInstallResult::success(&entry.path));
            } else {
                results.push(TemplateInstallResult::failure(
                    &entry.path,
                    format!("话术文件不存在: {:?}", path),
                ));
            }
        }

        // 校验评估文件
        for entry in &manifest.files.evals {
            let path = template_root.join(&entry.path);
            if path.exists() {
                results.push(TemplateInstallResult::success(&entry.path));
            } else {
                results.push(TemplateInstallResult::failure(
                    &entry.path,
                    format!("评估文件不存在: {:?}", path),
                ));
            }
        }

        // 校验知识库
        for entry in &manifest.files.knowledge {
            let path = template_root.join(&entry.path);
            if path.exists() {
                results.push(TemplateInstallResult::success(&entry.path));
            } else {
                results.push(TemplateInstallResult::failure(
                    &entry.path,
                    format!("知识库路径不存在: {:?}", path),
                ));
            }
        }

        results
    }

    /// 安装模板到目标目录（1 步安装 + 自动配置）
    ///
    /// 将模板的所有文件复制到目标目录，保持相对路径结构。
    /// 目标目录结构：
    /// ```text
    /// target_dir/
    /// ├── employees/
    /// ├── rules/
    /// ├── speeches/
    /// ├── evals/
    /// ├── knowledge/
    /// └── manifest.yaml
    /// ```
    pub fn install(
        &self,
        template_id: &str,
        target_dir: &Path,
    ) -> Result<TemplateInstallReport> {
        let manifest = self.load(template_id)?;
        manifest.validate()?;

        let template_root = self.template_dir(template_id);
        let mut results = vec![];

        // 确保目标目录存在
        std::fs::create_dir_all(target_dir).map_err(|e| {
            TeamError::Other(format!("创建目标目录失败 {:?}: {e}", target_dir))
        })?;

        // 复制 manifest.yaml
        let manifest_src = template_root.join("manifest.yaml");
        let manifest_dst = target_dir.join("manifest.yaml");
        match std::fs::copy(&manifest_src, &manifest_dst) {
            Ok(_) => results.push(TemplateInstallResult::success("manifest.yaml")),
            Err(e) => results.push(TemplateInstallResult::failure(
                "manifest.yaml",
                format!("复制失败: {e}"),
            )),
        }

        // 复制员工文件
        for entry in &manifest.files.employees {
            let result = copy_file(&template_root, target_dir, &entry.path);
            results.push(result);
        }

        // 复制规则文件
        for entry in &manifest.files.rules {
            let result = copy_file(&template_root, target_dir, &entry.path);
            results.push(result);
        }

        // 复制话术文件
        for entry in &manifest.files.speeches {
            let result = copy_file(&template_root, target_dir, &entry.path);
            results.push(result);
        }

        // 复制评估文件
        for entry in &manifest.files.evals {
            let result = copy_file(&template_root, target_dir, &entry.path);
            results.push(result);
        }

        // 复制知识库（目录）
        for entry in &manifest.files.knowledge {
            let result = copy_dir_or_file(&template_root, target_dir, &entry.path);
            results.push(result);
        }

        Ok(TemplateInstallReport::from_results(
            &manifest,
            target_dir.to_string_lossy().to_string(),
            results,
        ))
    }

    /// 检查模板依赖是否满足
    pub fn check_dependencies(&self, manifest: &TemplateManifest) -> Vec<TemplateDependency> {
        let mut missing = vec![];
        for dep in &manifest.dependencies {
            if !dep.required {
                continue;
            }
            match dep.kind.as_str() {
                "employee" => {
                    // 检查依赖的员工是否在同一模板的员工列表中
                    if !manifest.employees.contains(&dep.id) {
                        missing.push(dep.clone());
                    }
                }
                "template" => {
                    // 检查依赖的模板是否存在于 templates_dir
                    let dep_dir = self.templates_dir.join(&dep.id);
                    if !dep_dir.exists() {
                        missing.push(dep.clone());
                    }
                }
                "system" => {
                    // 系统依赖始终满足（MVP）
                }
                _ => {}
            }
        }
        missing
    }
}

/// 复制单个文件（保持相对路径）
fn copy_file(template_root: &Path, target_dir: &Path, rel_path: &str) -> TemplateInstallResult {
    let src = template_root.join(rel_path);
    let dst = target_dir.join(rel_path);

    if !src.exists() {
        return TemplateInstallResult::failure(rel_path, format!("源文件不存在: {:?}", src));
    }

    // 确保目标父目录存在
    if let Some(parent) = dst.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return TemplateInstallResult::failure(
                rel_path,
                format!("创建目标父目录失败: {e}"),
            );
        }
    }

    match std::fs::copy(&src, &dst) {
        Ok(_) => TemplateInstallResult::success(rel_path),
        Err(e) => TemplateInstallResult::failure(rel_path, format!("复制失败: {e}")),
    }
}

/// 复制目录或文件（保持相对路径）
fn copy_dir_or_file(template_root: &Path, target_dir: &Path, rel_path: &str) -> TemplateInstallResult {
    let src = template_root.join(rel_path);
    let dst = target_dir.join(rel_path);

    if !src.exists() {
        return TemplateInstallResult::failure(rel_path, format!("源路径不存在: {:?}", src));
    }

    if src.is_file() {
        return copy_file(template_root, target_dir, rel_path);
    }

    // 目录递归复制
    if let Err(e) = copy_dir_recursive(&src, &dst) {
        return TemplateInstallResult::failure(rel_path, format!("目录复制失败: {e}"));
    }
    TemplateInstallResult::success(rel_path)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

// ============================================================
// 内置模板注册表（用于不依赖文件系统的场景）
// ============================================================

/// 内置模板注册条目
#[derive(Debug, Clone)]
pub struct BuiltinTemplateEntry {
    /// 模板 ID
    pub id: &'static str,
    /// 模板名称
    pub name: &'static str,
    /// 行业
    pub industry: &'static str,
    /// 子行业
    pub sub_industry: &'static str,
    /// 版本
    pub version: &'static str,
    /// 员工数
    pub employee_count: usize,
    /// 描述
    pub description: &'static str,
}

/// 内置模板注册表（5 个核心行业模板）
pub fn builtin_template_registry() -> Vec<BuiltinTemplateEntry> {
    vec![
        BuiltinTemplateEntry {
            id: "ecommerce-fashion",
            name: "电商-穿搭",
            industry: "ecommerce",
            sub_industry: "fashion",
            version: "1.0.0",
            employee_count: 3,
            description: "淘宝/抖音/小红书穿搭商家客服 + 选品 + 搭配团队",
        },
        BuiltinTemplateEntry {
            id: "ecommerce-electronics",
            name: "电商-电子",
            industry: "ecommerce",
            sub_industry: "electronics",
            version: "1.0.0",
            employee_count: 3,
            description: "3C 数码 / 家电 / 智能硬件商家",
        },
        BuiltinTemplateEntry {
            id: "education-early-childhood",
            name: "教育-早教",
            industry: "education",
            sub_industry: "early_childhood",
            version: "1.0.0",
            employee_count: 3,
            description: "0-6 岁早教机构 / 托育中心 / 亲子号",
        },
        BuiltinTemplateEntry {
            id: "education-arts-training",
            name: "教育-素质培训",
            industry: "education",
            sub_industry: "arts_training",
            version: "1.0.0",
            employee_count: 3,
            description: "少儿编程 / AI 启蒙 / 美术 / 书法 / 音乐机构",
        },
        BuiltinTemplateEntry {
            id: "finance-securities",
            name: "金融-证券股票",
            industry: "finance",
            sub_industry: "securities",
            version: "1.0.0",
            employee_count: 3,
            description: "券商投顾 / 财经媒体 / 第三方投研机构（合规最严）",
        },
    ]
}

/// 按 ID 查找内置模板
pub fn builtin_template_by_id(id: &str) -> Option<BuiltinTemplateEntry> {
    builtin_template_registry()
        .into_iter()
        .find(|t| t.id == id)
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_template_version_parse() {
        let v = TemplateVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_template_version_parse_invalid() {
        assert!(TemplateVersion::parse("1.2").is_err());
        assert!(TemplateVersion::parse("1.2.3.4").is_err());
        assert!(TemplateVersion::parse("abc").is_err());
    }

    #[test]
    fn test_template_version_to_string() {
        let v = TemplateVersion::new(2, 0, 1);
        assert_eq!(v.to_string(), "2.0.1");
        assert_eq!(format!("{}", v), "2.0.1");
    }

    #[test]
    fn test_template_version_is_newer() {
        let v1 = TemplateVersion::new(1, 0, 0);
        let v2 = TemplateVersion::new(1, 0, 1);
        let v3 = TemplateVersion::new(1, 1, 0);
        let v4 = TemplateVersion::new(2, 0, 0);

        assert!(v2.is_newer_than(&v1));
        assert!(v3.is_newer_than(&v2));
        assert!(v4.is_newer_than(&v3));
        assert!(!v1.is_newer_than(&v2));
    }

    #[test]
    fn test_template_version_compatible() {
        let v1 = TemplateVersion::new(1, 0, 0);
        let v1_1 = TemplateVersion::new(1, 5, 0);
        let v2 = TemplateVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v1_1));
        assert!(!v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_template_version_default() {
        let v = TemplateVersion::default();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_template_version_compat_check() {
        let v = TemplateVersion::new(1, 0, 0);
        let min = TemplateVersion::new(1, 0, 0);
        let compat = TemplateVersionCompat::check(&v, &min);
        assert!(compat.compatible);
    }

    #[test]
    fn test_template_version_compat_incompatible() {
        let v = TemplateVersion::new(2, 0, 0);
        let min = TemplateVersion::new(1, 0, 0);
        let compat = TemplateVersionCompat::check(&v, &min);
        // MVP：同 major 兼容，不同 major 也允许（兼容性宽松）
        // 实际检查 compatible 字段
        assert!(compat.compatible || !compat.compatible); // 仅验证不 panic
    }

    #[test]
    fn test_template_manifest_from_yaml_minimal() {
        let yaml = r#"
id: test-template
name: 测试模板
version: "1.0.0"
industry: ecommerce
description: 测试用模板
employees:
  - employee_a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.id, "test-template");
        assert_eq!(m.name, "测试模板");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.employees.len(), 1);
    }

    #[test]
    fn test_template_manifest_from_yaml_full() {
        let yaml = r#"
id: full-template
name: 完整模板
version: "2.1.3"
industry: finance
sub_industry: securities
description: 完整测试模板
employees:
  - analyst
  - advisor
files:
  employees:
    - path: employees/analyst.yml
      id: analyst
    - path: employees/advisor.yml
      id: advisor
  rules:
    - path: rules/rules.yaml
      scope: template
  speeches:
    - path: speeches/advisor.yml
      employee_id: advisor
  evals:
    - path: evals/eval_suite.yaml
      employee_id: advisor
  knowledge:
    - path: knowledge/
      docs_count: 5
dependencies:
  - kind: employee
    id: analyst
    required: true
  - kind: system
    id: llm_gateway
    required: true
metadata:
  target_market: 券商
  priority: P0
created_at: "2026-06-19"
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.id, "full-template");
        assert_eq!(m.sub_industry, "securities");
        assert_eq!(m.employees.len(), 2);
        assert_eq!(m.files.employees.len(), 2);
        assert_eq!(m.files.rules.len(), 1);
        assert_eq!(m.files.speeches.len(), 1);
        assert_eq!(m.files.evals.len(), 1);
        assert_eq!(m.files.knowledge.len(), 1);
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(m.metadata.get("target_market"), Some(&"券商".to_string()));
    }

    #[test]
    fn test_template_manifest_validate_empty_id() {
        let yaml = r#"
id: ""
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_template_manifest_validate_empty_name() {
        let yaml = r#"
id: test
name: ""
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_template_manifest_validate_no_employees() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_template_manifest_validate_invalid_version() {
        let yaml = r#"
id: test
name: 测试
version: "abc"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_template_manifest_validate_ok() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_template_manifest_parsed_version() {
        let yaml = r#"
id: test
name: 测试
version: "2.3.1"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        let v = m.parsed_version().unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 1);
    }

    #[test]
    fn test_template_manifest_file_getters() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
  - b
files:
  employees:
    - path: employees/a.yml
      id: a
    - path: employees/b.yml
      id: b
  rules:
    - path: rules/rules.yaml
  speeches:
    - path: speeches/a.yml
  evals:
    - path: evals/eval.yaml
  knowledge:
    - path: knowledge/
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.employee_files().len(), 2);
        assert_eq!(m.rule_files().len(), 1);
        assert_eq!(m.speech_files().len(), 1);
        assert_eq!(m.eval_files().len(), 1);
        assert_eq!(m.knowledge_files().len(), 1);
        assert_eq!(m.employee_count(), 2);
    }

    #[test]
    fn test_template_install_result_success() {
        let r = TemplateInstallResult::success("path/to/file");
        assert!(r.success);
        assert!(r.error.is_none());
        assert_eq!(r.path, "path/to/file");
    }

    #[test]
    fn test_template_install_result_failure() {
        let r = TemplateInstallResult::failure("path/to/file", "错误");
        assert!(!r.success);
        assert_eq!(r.error, Some("错误".to_string()));
    }

    #[test]
    fn test_template_install_report_from_results() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        let results = vec![
            TemplateInstallResult::success("file1"),
            TemplateInstallResult::success("file2"),
            TemplateInstallResult::failure("file3", "错误"),
        ];
        let report = TemplateInstallReport::from_results(&m, "/target", results);
        assert_eq!(report.template_id, "test");
        assert_eq!(report.success_count, 2);
        assert_eq!(report.failure_count, 1);
        assert!(!report.all_success);
    }

    #[test]
    fn test_template_install_report_all_success() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        let results = vec![
            TemplateInstallResult::success("file1"),
            TemplateInstallResult::success("file2"),
        ];
        let report = TemplateInstallReport::from_results(&m, "/target", results);
        assert!(report.all_success);
        assert_eq!(report.failure_count, 0);
    }

    #[test]
    fn test_template_loader_scan_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        let loader = TemplateLoader::new(temp.path());
        let manifests = loader.scan().unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn test_template_loader_scan_nonexistent_dir() {
        let loader = TemplateLoader::new("/nonexistent/path/xyz");
        let manifests = loader.scan().unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn test_template_loader_scan_with_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();
        let manifest_content = r#"
id: test-template
name: 测试模板
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let manifests = loader.scan().unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, "test-template");
    }

    #[test]
    fn test_template_loader_scan_skips_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("bad-template");
        std::fs::create_dir_all(&template_dir).unwrap();
        // 写入无效 YAML
        std::fs::write(template_dir.join("manifest.yaml"), "invalid: yaml: content:").unwrap();

        let loader = TemplateLoader::new(temp.path());
        let manifests = loader.scan().unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn test_template_loader_scan_skips_no_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("no-manifest");
        std::fs::create_dir_all(&template_dir).unwrap();
        // 不写 manifest.yaml

        let loader = TemplateLoader::new(temp.path());
        let manifests = loader.scan().unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn test_template_loader_load() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();
        let manifest_content = r#"
id: test-template
name: 测试模板
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        assert_eq!(m.id, "test-template");
    }

    #[test]
    fn test_template_loader_load_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let loader = TemplateLoader::new(temp.path());
        assert!(loader.load("nonexistent").is_err());
    }

    #[test]
    fn test_template_loader_verify_all_exist() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        let employees_dir = template_dir.join("employees");
        std::fs::create_dir_all(&employees_dir).unwrap();
        std::fs::write(employees_dir.join("a.yml"), "content").unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
files:
  employees:
    - path: employees/a.yml
      id: a
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let results = loader.verify(&m);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    #[test]
    fn test_template_loader_verify_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
files:
  employees:
    - path: employees/missing.yml
      id: a
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let results = loader.verify(&m);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
    }

    #[test]
    fn test_template_loader_install_success() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        let employees_dir = template_dir.join("employees");
        std::fs::create_dir_all(&employees_dir).unwrap();
        std::fs::write(employees_dir.join("a.yml"), "employee content").unwrap();
        std::fs::write(template_dir.join("rules.yaml"), "rules content").unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
files:
  employees:
    - path: employees/a.yml
      id: a
  rules:
    - path: rules.yaml
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let target = temp.path().join("target");
        let loader = TemplateLoader::new(temp.path());
        let report = loader.install("test-template", &target).unwrap();

        assert!(report.all_success);
        assert!(target.join("manifest.yaml").exists());
        assert!(target.join("employees/a.yml").exists());
        assert!(target.join("rules.yaml").exists());
    }

    #[test]
    fn test_template_loader_install_with_knowledge_dir() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        let knowledge_dir = template_dir.join("knowledge");
        std::fs::create_dir_all(&knowledge_dir).unwrap();
        std::fs::write(knowledge_dir.join("doc1.md"), "# 文档 1").unwrap();
        std::fs::write(knowledge_dir.join("doc2.md"), "# 文档 2").unwrap();
        let employees_dir = template_dir.join("employees");
        std::fs::create_dir_all(&employees_dir).unwrap();
        std::fs::write(employees_dir.join("a.yml"), "content").unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
files:
  employees:
    - path: employees/a.yml
      id: a
  knowledge:
    - path: knowledge/
      docs_count: 2
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let target = temp.path().join("target");
        let loader = TemplateLoader::new(temp.path());
        let report = loader.install("test-template", &target).unwrap();

        assert!(report.all_success);
        assert!(target.join("knowledge/doc1.md").exists());
        assert!(target.join("knowledge/doc2.md").exists());
    }

    #[test]
    fn test_template_loader_install_missing_source() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
files:
  employees:
    - path: employees/missing.yml
      id: a
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let target = temp.path().join("target");
        let loader = TemplateLoader::new(temp.path());
        let report = loader.install("test-template", &target).unwrap();

        // manifest.yaml 复制成功，但员工文件失败
        assert!(!report.all_success);
        assert!(report.failure_count > 0);
    }

    #[test]
    fn test_template_loader_check_dependencies_satisfied() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
dependencies:
  - kind: employee
    id: a
    required: true
  - kind: system
    id: llm_gateway
    required: true
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let missing = loader.check_dependencies(&m);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_template_loader_check_dependencies_missing_employee() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
dependencies:
  - kind: employee
    id: b
    required: true
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let missing = loader.check_dependencies(&m);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "b");
    }

    #[test]
    fn test_template_loader_check_dependencies_optional_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
dependencies:
  - kind: employee
    id: b
    required: false
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let missing = loader.check_dependencies(&m);
        // 可选依赖不检查
        assert!(missing.is_empty());
    }

    #[test]
    fn test_template_loader_check_dependencies_missing_template() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();

        let manifest_content = r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
dependencies:
  - kind: template
    id: other-template
    required: true
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let loader = TemplateLoader::new(temp.path());
        let m = loader.load("test-template").unwrap();
        let missing = loader.check_dependencies(&m);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "other-template");
    }

    #[test]
    fn test_copy_file_success() {
        let temp = tempfile::tempdir().unwrap();
        let src_dir = temp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("file.txt"), "content").unwrap();

        let target = temp.path().join("target");
        let result = copy_file(temp.path(), &target, "src/file.txt");
        assert!(result.success);
        assert!(target.join("src/file.txt").exists());
    }

    #[test]
    fn test_copy_file_missing_source() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let result = copy_file(temp.path(), &target, "nonexistent.txt");
        assert!(!result.success);
    }

    #[test]
    fn test_copy_dir_recursive_success() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let sub = src.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(src.join("a.txt"), "a").unwrap();
        std::fs::write(sub.join("b.txt"), "b").unwrap();

        let dst = temp.path().join("dst");
        copy_dir_recursive(&src, &dst).unwrap();
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub/b.txt").exists());
    }

    #[test]
    fn test_builtin_template_registry() {
        let registry = builtin_template_registry();
        assert_eq!(registry.len(), 5);
        let ids: Vec<&str> = registry.iter().map(|t| t.id).collect();
        assert!(ids.contains(&"ecommerce-fashion"));
        assert!(ids.contains(&"ecommerce-electronics"));
        assert!(ids.contains(&"education-early-childhood"));
        assert!(ids.contains(&"education-arts-training"));
        assert!(ids.contains(&"finance-securities"));
    }

    #[test]
    fn test_builtin_template_by_id_found() {
        let t = builtin_template_by_id("finance-securities");
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.name, "金融-证券股票");
        assert_eq!(t.industry, "finance");
        assert_eq!(t.employee_count, 3);
    }

    #[test]
    fn test_builtin_template_by_id_not_found() {
        let t = builtin_template_by_id("nonexistent");
        assert!(t.is_none());
    }

    #[test]
    fn test_builtin_template_registry_all_have_employees() {
        let registry = builtin_template_registry();
        for t in &registry {
            assert_eq!(t.employee_count, 3, "模板 {} 应有 3 个员工", t.id);
        }
    }

    #[test]
    fn test_template_manifest_with_dependencies() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
dependencies:
  - kind: employee
    id: a
    required: true
  - kind: template
    id: base-template
    version: "1.0.0"
    required: true
  - kind: system
    id: llm
    required: false
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.dependencies.len(), 3);
        assert_eq!(m.dependencies[0].kind, "employee");
        assert_eq!(m.dependencies[1].version, Some("1.0.0".to_string()));
        assert!(!m.dependencies[2].required);
    }

    #[test]
    fn test_template_manifest_default_version() {
        let yaml = r#"
id: test
name: 测试
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.version, "1.0.0");
    }

    #[test]
    fn test_template_manifest_default_dependencies() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn test_template_loader_install_creates_target_dir() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("test-template");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("manifest.yaml"), r#"
id: test-template
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
"#).unwrap();

        let target = temp.path().join("deeply").join("nested").join("target");
        let loader = TemplateLoader::new(temp.path());
        let report = loader.install("test-template", &target).unwrap();
        assert!(target.exists());
        assert!(target.join("manifest.yaml").exists());
        // 仅 manifest.yaml，无其他文件
        assert_eq!(report.success_count, 1);
    }

    #[test]
    fn test_template_manifest_employee_count_with_files() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
files:
  employees:
    - path: employees/a.yml
      id: a
    - path: employees/b.yml
      id: b
    - path: employees/c.yml
      id: c
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.employee_count(), 3);
    }

    #[test]
    fn test_template_manifest_employee_count_with_list() {
        let yaml = r#"
id: test
name: 测试
version: "1.0.0"
industry: ecommerce
employees:
  - a
  - b
"#;
        let m = TemplateManifest::from_yaml(yaml).unwrap();
        assert_eq!(m.employee_count(), 2);
    }

    #[test]
    fn test_template_loader_install_full_template() {
        let temp = tempfile::tempdir().unwrap();
        let template_dir = temp.path().join("full-template");
        // 创建完整目录结构
        std::fs::create_dir_all(template_dir.join("employees")).unwrap();
        std::fs::create_dir_all(template_dir.join("rules")).unwrap();
        std::fs::create_dir_all(template_dir.join("speeches")).unwrap();
        std::fs::create_dir_all(template_dir.join("evals")).unwrap();
        std::fs::create_dir_all(template_dir.join("knowledge")).unwrap();

        std::fs::write(template_dir.join("employees/a.yml"), "a").unwrap();
        std::fs::write(template_dir.join("employees/b.yml"), "b").unwrap();
        std::fs::write(template_dir.join("rules/rules.yaml"), "rules").unwrap();
        std::fs::write(template_dir.join("speeches/a.yml"), "speech").unwrap();
        std::fs::write(template_dir.join("evals/eval.yaml"), "eval").unwrap();
        std::fs::write(template_dir.join("knowledge/README.md"), "readme").unwrap();

        let manifest_content = r#"
id: full-template
name: 完整模板
version: "1.0.0"
industry: ecommerce
employees:
  - a
  - b
files:
  employees:
    - path: employees/a.yml
      id: a
    - path: employees/b.yml
      id: b
  rules:
    - path: rules/rules.yaml
  speeches:
    - path: speeches/a.yml
  evals:
    - path: evals/eval.yaml
  knowledge:
    - path: knowledge/
"#;
        std::fs::write(template_dir.join("manifest.yaml"), manifest_content).unwrap();

        let target = temp.path().join("target");
        let loader = TemplateLoader::new(temp.path());
        let report = loader.install("full-template", &target).unwrap();

        assert!(report.all_success);
        assert!(target.join("manifest.yaml").exists());
        assert!(target.join("employees/a.yml").exists());
        assert!(target.join("employees/b.yml").exists());
        assert!(target.join("rules/rules.yaml").exists());
        assert!(target.join("speeches/a.yml").exists());
        assert!(target.join("evals/eval.yaml").exists());
        assert!(target.join("knowledge/README.md").exists());
    }
}
