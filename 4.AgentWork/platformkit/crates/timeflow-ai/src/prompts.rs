// Prompt 模板 + Conventional Commits 解析
// 设计文档: docs/TIMEFLOW-DESIGN.md § 4.1
//
// Conventional Commits 格式：
//   <type>[optional scope]: <description>
//   [optional body]
//   [optional footer(s)]
//
// type: feat | fix | docs | style | refactor | perf | test | chore | ci | build

use crate::error::{AiError, Result};
use serde::{Deserialize, Serialize};

/// Conventional Commit 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitType {
    /// 新功能
    Feat,
    /// Bug 修复
    Fix,
    /// 文档
    Docs,
    /// 代码格式（不影响功能）
    Style,
    /// 重构
    Refactor,
    /// 性能优化
    Perf,
    /// 测试
    Test,
    /// 杂务（构建、依赖等）
    Chore,
    /// CI 配置
    Ci,
    /// 构建系统
    Build,
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Feat => "feat",
            Self::Fix => "fix",
            Self::Docs => "docs",
            Self::Style => "style",
            Self::Refactor => "refactor",
            Self::Perf => "perf",
            Self::Test => "test",
            Self::Chore => "chore",
            Self::Ci => "ci",
            Self::Build => "build",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for CommitType {
    type Err = AiError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "feat" => Ok(Self::Feat),
            "fix" => Ok(Self::Fix),
            "docs" => Ok(Self::Docs),
            "style" => Ok(Self::Style),
            "refactor" => Ok(Self::Refactor),
            "perf" => Ok(Self::Perf),
            "test" => Ok(Self::Test),
            "chore" => Ok(Self::Chore),
            "ci" => Ok(Self::Ci),
            "build" => Ok(Self::Build),
            _ => Err(AiError::InvalidResponse(format!("未知 commit 类型: {}", s))),
        }
    }
}

/// 解析后的 Conventional Commit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConventionalCommit {
    /// 类型
    pub r#type: CommitType,
    /// 可选 scope（影响范围，如 auth / api / ui）
    pub scope: Option<String>,
    /// 描述（简短一行）
    pub description: String,
    /// 可选 body（详细说明）
    pub body: Option<String>,
}

impl ConventionalCommit {
    /// 渲染为标准 Conventional Commits 字符串
    pub fn to_message(&self) -> String {
        let mut msg = match &self.scope {
            Some(scope) => format!("{}({}): {}", self.r#type, scope, self.description),
            None => format!("{}: {}", self.r#type, self.description),
        };
        if let Some(body) = &self.body {
            msg.push_str("\n\n");
            msg.push_str(body);
        }
        msg
    }
}

/// 从 LLM 响应解析 Conventional Commit
///
/// 支持两种格式：
/// 1. 纯文本：`feat(auth): 添加 JWT 登录`
/// 2. JSON：`{"type":"feat","scope":"auth","description":"添加 JWT 登录"}`
pub fn parse_conventional_commit(text: &str) -> Result<ConventionalCommit> {
    let text = text.trim();

    // 尝试 JSON 格式
    if text.starts_with('{') {
        if let Ok(commit) = serde_json::from_str::<ConventionalCommit>(text) {
            return Ok(commit);
        }
    }

    // 尝试纯文本格式：`type(scope): description` 或 `type: description`
    parse_text_format(text)
}

fn parse_text_format(text: &str) -> Result<ConventionalCommit> {
    // 取第一行作为 header
    let first_line = text.lines().next().ok_or(AiError::EmptyResponse)?;
    let first_line = first_line.trim();

    // 解析 type
    let colon_pos = first_line
        .find(':')
        .ok_or_else(|| AiError::InvalidResponse(format!("缺少冒号分隔符: {}", first_line)))?;

    let type_part = first_line[..colon_pos].trim();
    let description = first_line[colon_pos + 1..].trim().to_string();

    if description.is_empty() {
        return Err(AiError::InvalidResponse("描述为空".to_string()));
    }

    // 解析 type 和 scope
    let (commit_type, scope) = if let Some(open) = type_part.find('(') {
        if let Some(close) = type_part.find(')') {
            if close > open {
                let t = &type_part[..open];
                let s = &type_part[open + 1..close];
                (
                    t.parse::<CommitType>()?,
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    },
                )
            } else {
                (type_part.parse::<CommitType>()?, None)
            }
        } else {
            (type_part.parse::<CommitType>()?, None)
        }
    } else {
        (type_part.parse::<CommitType>()?, None)
    };

    // 解析 body（如果有）
    let body = if text.lines().count() > 1 {
        let body_text: String = text
            .lines()
            .skip(1)
            .skip_while(|l| l.trim().is_empty()) // 跳过空行
            .collect::<Vec<_>>()
            .join("\n");
        if body_text.trim().is_empty() {
            None
        } else {
            Some(body_text)
        }
    } else {
        None
    };

    Ok(ConventionalCommit {
        r#type: commit_type,
        scope,
        description,
        body,
    })
}

/// 构建 commit msg 生成的 prompt
///
/// 参数：
/// - `diff`: 文件变更摘要（不是完整 diff，避免 token 爆炸）
/// - `history`: 最近几次 commit msg（提供风格参考）
pub fn build_commit_msg_prompt(diff: &str, history: &[String]) -> (String, String) {
    let system = r#"你是一个专业的代码提交信息生成助手。你的任务是根据代码变更生成符合 Conventional Commits 规范的提交信息。

Conventional Commits 格式：
<type>[optional scope]: <description>

type 可选值：
- feat: 新功能
- fix: Bug 修复
- docs: 文档变更
- style: 代码格式（不影响功能）
- refactor: 重构（既不是新功能也不是修 Bug）
- perf: 性能优化
- test: 测试相关
- chore: 杂务（构建、依赖、配置等）
- ci: CI 配置
- build: 构建系统

要求：
1. 只输出一行提交信息，不要解释
2. description 用中文，简明扼要（不超过 50 字）
3. scope 可选，表示影响范围（如 auth、api、ui）
4. 优先参考历史提交的风格

示例：
- feat(auth): 添加 JWT 登录接口
- fix(api): 修复用户列表分页错误
- docs: 更新 README 安装步骤
- refactor(core): 提取快照存储逻辑
- chore: 升级 tokio 到 1.40"#;

    let history_text = if history.is_empty() {
        "（无历史提交）".to_string()
    } else {
        history
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, h)| format!("{}. {}", i + 1, h))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let user = format!(
        r#"最近提交历史：
{}

代码变更：
```
{}
```

请生成一个 Conventional Commits 格式的提交信息（只输出一行）："#,
        history_text, diff
    );

    (system.to_string(), user)
}

/// 构建变更摘要的 prompt
///
/// 返回一句话描述变更内容（用于快照列表展示）
pub fn build_summary_prompt(diff: &str) -> (String, String) {
    let system = "你是一个代码变更摘要助手。请用一句话（不超过 30 字）描述代码变更的内容。只输出摘要，不要解释。";

    let user = format!(
        r#"代码变更：
```
{}
```

请用一句话描述这次变更："#,
        diff
    );

    (system.to_string(), user)
}

/// 构建快照分类 prompt（W6 M2.2 D1）
///
/// 输入：快照的 commit msg、构建状态、文件变更
/// 输出：期望 LLM 返回单个词：wip / progress / candidate / release
pub fn build_classify_prompt(input: &crate::classify::ClassifyInput) -> (String, String) {
    let system = r#"你是代码版本分类助手。请将快照分类为以下四种之一，只输出一个单词：
- wip        改动小/未完成/编译失败
- progress   有意义但非正式（10-100 行改动）
- candidate  候选版本（编译+测试通过，改动充分，多文件）
- release    正式版本（用户确认发布）

只输出一个单词，不要解释。"#;

    let total_add: u32 = input.files.iter().map(|f| f.additions).sum();
    let total_del: u32 = input.files.iter().map(|f| f.deletions).sum();
    let file_list: Vec<String> = input
        .files
        .iter()
        .map(|f| format!("  {} (+{} -{})", f.path.display(), f.additions, f.deletions))
        .collect();

    let build = match input.build_status {
        timeflow_core::BuildStatus::Green => "编译通过+测试通过",
        timeflow_core::BuildStatus::Yellow => "编译通过+测试失败",
        timeflow_core::BuildStatus::Red => "编译失败",
        timeflow_core::BuildStatus::Unknown => "未知",
    };

    let user = format!(
        r#"commit msg: {}
构建状态: {}
文件数: {}
总改动: +{} -{}

文件列表:
{}

请输出分类（wip/progress/candidate/release）："#,
        input.message,
        build,
        input.files.len(),
        total_add,
        total_del,
        if file_list.is_empty() {
            "(无)".to_string()
        } else {
            file_list.join("\n")
        }
    );

    (system.to_string(), user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_type_display() {
        assert_eq!(CommitType::Feat.to_string(), "feat");
        assert_eq!(CommitType::Fix.to_string(), "fix");
        assert_eq!(CommitType::Docs.to_string(), "docs");
    }

    #[test]
    fn test_commit_type_parse() {
        assert_eq!("feat".parse::<CommitType>().unwrap(), CommitType::Feat);
        assert_eq!("FIX".parse::<CommitType>().unwrap(), CommitType::Fix);
        assert!("invalid".parse::<CommitType>().is_err());
    }

    #[test]
    fn test_parse_text_no_scope() {
        let commit = parse_conventional_commit("feat: 添加登录功能").unwrap();
        assert_eq!(commit.r#type, CommitType::Feat);
        assert_eq!(commit.scope, None);
        assert_eq!(commit.description, "添加登录功能");
        assert_eq!(commit.body, None);
    }

    #[test]
    fn test_parse_text_with_scope() {
        let commit = parse_conventional_commit("feat(auth): 添加 JWT 登录").unwrap();
        assert_eq!(commit.r#type, CommitType::Feat);
        assert_eq!(commit.scope, Some("auth".to_string()));
        assert_eq!(commit.description, "添加 JWT 登录");
    }

    #[test]
    fn test_parse_text_with_body() {
        let text = "fix(api): 修复分页错误\n\n详细说明：offset 应该从 0 开始";
        let commit = parse_conventional_commit(text).unwrap();
        assert_eq!(commit.r#type, CommitType::Fix);
        assert_eq!(commit.scope, Some("api".to_string()));
        assert_eq!(commit.description, "修复分页错误");
        assert!(commit.body.is_some());
        assert!(commit.body.unwrap().contains("offset"));
    }

    #[test]
    fn test_parse_json_format() {
        let json = r#"{"type":"feat","scope":"auth","description":"添加 JWT 登录","body":null}"#;
        let commit = parse_conventional_commit(json).unwrap();
        assert_eq!(commit.r#type, CommitType::Feat);
        assert_eq!(commit.scope, Some("auth".to_string()));
        assert_eq!(commit.description, "添加 JWT 登录");
    }

    #[test]
    fn test_parse_invalid_missing_colon() {
        let result = parse_conventional_commit("feat 添加登录");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_empty_description() {
        let result = parse_conventional_commit("feat:   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_type() {
        let result = parse_conventional_commit("invalid: 添加登录");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_message_no_scope() {
        let commit = ConventionalCommit {
            r#type: CommitType::Feat,
            scope: None,
            description: "添加登录".to_string(),
            body: None,
        };
        assert_eq!(commit.to_message(), "feat: 添加登录");
    }

    #[test]
    fn test_to_message_with_scope() {
        let commit = ConventionalCommit {
            r#type: CommitType::Feat,
            scope: Some("auth".to_string()),
            description: "添加 JWT".to_string(),
            body: None,
        };
        assert_eq!(commit.to_message(), "feat(auth): 添加 JWT");
    }

    #[test]
    fn test_to_message_with_body() {
        let commit = ConventionalCommit {
            r#type: CommitType::Fix,
            scope: Some("api".to_string()),
            description: "修复分页".to_string(),
            body: Some("offset 从 0 开始".to_string()),
        };
        assert_eq!(
            commit.to_message(),
            "fix(api): 修复分页\n\noffset 从 0 开始"
        );
    }

    #[test]
    fn test_build_commit_msg_prompt_with_history() {
        let diff = "+ fn login() {}";
        let history = vec!["feat: 添加注册".to_string(), "fix: 修复密码".to_string()];
        let (system, user) = build_commit_msg_prompt(diff, &history);
        assert!(system.contains("Conventional Commits"));
        assert!(user.contains("feat: 添加注册"));
        assert!(user.contains("fn login"));
    }

    #[test]
    fn test_build_commit_msg_prompt_no_history() {
        let diff = "+ fn login() {}";
        let (system, user) = build_commit_msg_prompt(diff, &[]);
        assert!(system.contains("Conventional Commits"));
        assert!(user.contains("无历史提交"));
    }

    #[test]
    fn test_build_summary_prompt() {
        let diff = "+ fn login() {}";
        let (system, user) = build_summary_prompt(diff);
        assert!(system.contains("一句话"));
        assert!(user.contains("fn login"));
    }
}
