// 技能系统 V2（M6.1 W3）
//
// 目标：兼容 Markdown 驱动、渐进发现与自进化统计。
// 存储格式：<repo>/.aw/skills/{id}.md
//
//   ---
//   id: refund-flow
//   name: 退款流程
//   tags: [客服, 退款]
//   description: 标准退款处理流程
//   ---
//
//   ## 1. 确认订单
//   ...
//   ## 2. 校验资格
//   ...

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 技能步骤
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SkillStep {
    /// 步骤标题
    pub title: String,
    /// 步骤说明
    #[serde(default)]
    pub content: String,
}

/// 技能元信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillManifest {
    /// 技能 ID
    pub id: String,
    /// 技能名称
    pub name: String,
    /// 分类标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 描述
    #[serde(default)]
    pub description: String,
}

/// 技能 V2
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillV2 {
    /// 技能元信息
    pub manifest: SkillManifest,
    /// 操作步骤
    #[serde(default)]
    pub steps: Vec<SkillStep>,
    /// 成功次数
    #[serde(default)]
    pub success_count: u64,
    /// 失败次数
    #[serde(default)]
    pub failure_count: u64,
}

impl SkillV2 {
    /// 创建空技能
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            manifest: SkillManifest {
                id: id.into(),
                name: name.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// 追加步骤
    pub fn with_step(mut self, title: impl Into<String>, content: impl Into<String>) -> Self {
        self.steps.push(SkillStep {
            title: title.into(),
            content: content.into(),
        });
        self
    }

    /// 追加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.manifest.tags.push(tag.into());
        self
    }

    /// 记录一次成功
    pub fn mark_success(&mut self) {
        self.success_count += 1;
    }

    /// 记录一次失败
    pub fn mark_failure(&mut self) {
        self.failure_count += 1;
    }

    /// 简单成功率（0.0-1.0）
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }

    /// 质量分（成功率 × log(总调用+1) 归一化到 0-1）
    pub fn quality_score(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        let rate = self.success_rate();
        let volume = (total as f64).ln() / 10.0; // 调用越多越可信
        (rate * 0.8 + volume.min(0.2) * 1.0).clamp(0.0, 1.0)
    }

    /// 序列化为 Markdown（front-matter + body）
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(&format!("id: {}\n", escape_yaml(&self.manifest.id)));
        out.push_str(&format!("name: {}\n", escape_yaml(&self.manifest.name)));
        if self.manifest.tags.is_empty() {
            out.push_str("tags: []\n");
        } else {
            out.push_str("tags: [");
            out.push_str(
                &self
                    .manifest
                    .tags
                    .iter()
                    .map(|t| escape_yaml(t))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str("]\n");
        }
        out.push_str(&format!(
            "description: {}\n",
            escape_yaml(&self.manifest.description)
        ));
        out.push_str("---\n\n");
        for step in &self.steps {
            out.push_str(&format!("## {}\n{}\n\n", step.title, step.content));
        }
        out
    }

    /// 从 Markdown 解析
    pub fn from_markdown(md: &str) -> Result<Self> {
        let trimmed = md.trim_start();
        let body = if let Some(rest) = trimmed.strip_prefix("---\n") {
            // 找第二个 ---
            if let Some(end) = rest.find("\n---\n") {
                &rest[end + "\n---\n".len()..]
            } else if let Some(end) = rest.find("\n---") {
                &rest[end + "\n---".len()..]
            } else {
                rest
            }
        } else {
            trimmed
        };

        let manifest = parse_front_matter(md);
        let steps = parse_steps(body);

        Ok(Self {
            manifest,
            steps,
            success_count: 0,
            failure_count: 0,
        })
    }

    /// 保存到 .md 文件（自动创建父目录）
    pub fn save_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, self.to_markdown())?;
        Ok(())
    }

    /// 从 .md 文件加载
    pub fn load_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_markdown(&content)
    }
}

/// 转义 YAML 标量（简单处理引号/反斜杠）
fn escape_yaml(s: &str) -> String {
    let needs_quote = s.contains(':') || s.contains('#') || s.contains('"') || s.contains('\'');
    if needs_quote {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

/// 简单解析 front-matter
fn parse_front_matter(md: &str) -> SkillManifest {
    let mut manifest = SkillManifest::default();
    let trimmed = md.trim_start();
    let Some(rest) = trimmed.strip_prefix("---\n") else {
        return manifest;
    };
    let Some(end) = rest.find("\n---") else {
        return manifest;
    };
    let fm = &rest[..end];
    for line in fm.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let value = v.trim();
            match key {
                "id" => manifest.id = unquote(value).to_string(),
                "name" => manifest.name = unquote(value).to_string(),
                "description" => manifest.description = unquote(value).to_string(),
                "tags" => {
                    manifest.tags = parse_tag_list(value);
                }
                _ => {}
            }
        }
    }
    manifest
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_tag_list(raw: &str) -> Vec<String> {
    let s = raw.trim();
    if s == "[]" || s.is_empty() {
        return Vec::new();
    }
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|p| unquote(p.trim()).to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 解析正文步骤（按 ## 标题切分）
fn parse_steps(body: &str) -> Vec<SkillStep> {
    let mut steps = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = String::new();
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(title) = current_title.take() {
                steps.push(SkillStep {
                    title,
                    content: current_body.trim().to_string(),
                });
                current_body.clear();
            }
            current_title = Some(rest.trim().to_string());
        } else if current_title.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(title) = current_title {
        steps.push(SkillStep {
            title,
            content: current_body.trim().to_string(),
        });
    }
    steps
}

/// 技能注册表（管理多个 SkillV2）
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    /// 按 id 索引
    pub skills: std::collections::HashMap<String, SkillV2>,
}

impl SkillRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个技能（同 id 覆盖）
    pub fn register(&mut self, skill: SkillV2) {
        self.skills.insert(skill.manifest.id.clone(), skill);
    }

    /// 按 id 获取
    pub fn get(&self, id: &str) -> Option<&SkillV2> {
        self.skills.get(id)
    }

    /// 按 id 获取（mut）
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SkillV2> {
        self.skills.get_mut(id)
    }

    /// 按标签筛选
    pub fn by_tag(&self, tag: &str) -> Vec<&SkillV2> {
        self.skills
            .values()
            .filter(|s| s.manifest.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// 列出所有技能 id
    /// 获取所有技能
    pub fn all(&self) -> Vec<&SkillV2> {
        self.skills.values().collect()
    }

    /// 获取所有技能 ID
    pub fn ids(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// 从目录批量加载 .md
    pub fn load_dir(dir: &Path) -> Result<Self> {
        let mut registry = Self::new();
        if !dir.exists() {
            return Ok(registry);
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let skill = SkillV2::load_file(&path)?;
                registry.register(skill);
            }
        }
        Ok(registry)
    }

    /// 全部保存到目录（每个技能一个 .md）
    pub fn save_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir)?;
        let mut saved = Vec::new();
        for skill in self.skills.values() {
            let path = dir.join(format!("{}.md", sanitize_filename(&skill.manifest.id)));
            skill.save_file(&path)?;
            saved.push(path);
        }
        Ok(saved)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_rate_and_quality() {
        let mut s = SkillV2::new("refund", "退款流程");
        for _ in 0..8 {
            s.mark_success();
        }
        for _ in 0..2 {
            s.mark_failure();
        }
        assert_eq!(s.success_rate(), 0.8);
        assert!(s.quality_score() > 0.0 && s.quality_score() <= 1.0);
    }

    #[test]
    fn markdown_roundtrip_basic() {
        let skill = SkillV2::new("refund-flow", "退款流程")
            .with_tag("客服")
            .with_tag("退款")
            .with_step("确认订单", "要求用户提供订单号")
            .with_step("校验资格", "判断是否在 7 天内");
        let md = skill.to_markdown();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("id: refund-flow"));
        assert!(md.contains("tags: ["));
        assert!(md.contains("## 确认订单"));

        let parsed = SkillV2::from_markdown(&md).unwrap();
        assert_eq!(parsed.manifest.id, "refund-flow");
        assert_eq!(parsed.manifest.name, "退款流程");
        assert_eq!(parsed.manifest.tags, vec!["客服", "退款"]);
        assert_eq!(parsed.steps.len(), 2);
        assert_eq!(parsed.steps[0].title, "确认订单");
        assert_eq!(parsed.steps[1].title, "校验资格");
    }

    #[test]
    fn markdown_with_special_chars_quoted() {
        let skill = SkillV2::new("s1", "含: 特殊 # 字符");
        let md = skill.to_markdown();
        assert!(md.contains("name: \"含: 特殊 # 字符\""));
        let parsed = SkillV2::from_markdown(&md).unwrap();
        assert_eq!(parsed.manifest.name, "含: 特殊 # 字符");
    }

    #[test]
    fn empty_tags_serializes() {
        let skill = SkillV2::new("s2", "无标签");
        let md = skill.to_markdown();
        assert!(md.contains("tags: []"));
        let parsed = SkillV2::from_markdown(&md).unwrap();
        assert!(parsed.manifest.tags.is_empty());
    }

    #[test]
    fn file_roundtrip() {
        let dir = std::env::temp_dir().join("aw_skills_test");
        let _ = std::fs::remove_dir_all(&dir);
        let skill = SkillV2::new("refund", "退款流程")
            .with_tag("客服")
            .with_step("步骤一", "做某事");
        let path = dir.join("refund.md");
        skill.save_file(&path).unwrap();
        let loaded = SkillV2::load_file(&path).unwrap();
        assert_eq!(loaded.manifest.id, "refund");
        assert_eq!(loaded.steps.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn registry_register_and_query() {
        let mut reg = SkillRegistry::new();
        reg.register(SkillV2::new("a", "A").with_tag("x"));
        reg.register(SkillV2::new("b", "B").with_tag("x"));
        reg.register(SkillV2::new("c", "C").with_tag("y"));
        assert_eq!(reg.skills.len(), 3);
        assert_eq!(reg.by_tag("x").len(), 2);
        assert!(reg.get("a").is_some());
        reg.get_mut("a").unwrap().mark_success();
        assert_eq!(reg.get("a").unwrap().success_count, 1);
    }

    #[test]
    fn registry_dir_roundtrip() {
        let dir = std::env::temp_dir().join("aw_skills_reg_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut reg = SkillRegistry::new();
        reg.register(SkillV2::new("refund", "退款").with_tag("客服"));
        reg.register(SkillV2::new("complaint", "投诉").with_tag("客服"));
        let saved = reg.save_dir(&dir).unwrap();
        assert_eq!(saved.len(), 2);

        let loaded = SkillRegistry::load_dir(&dir).unwrap();
        assert_eq!(loaded.skills.len(), 2);
        assert!(loaded.get("refund").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_dir_returns_empty() {
        let dir = std::env::temp_dir().join("aw_skills_not_exist");
        let _ = std::fs::remove_dir_all(&dir);
        let reg = SkillRegistry::load_dir(&dir).unwrap();
        assert!(reg.skills.is_empty());
    }
}
