// Commit msg 生成器
// 设计文档: docs/TIMEFLOW-DESIGN.md § 4.1
//
// 流程：
// 1. 从 TimeFlow 获取 diff（与父快照对比）
// 2. 压缩 diff（避免 token 爆炸）
// 3. 构建 prompt（含历史 commit 风格参考）
// 4. 调用 LLM 生成 commit msg
// 5. 解析为 Conventional Commits 格式
// 6. 返回标准格式字符串

use crate::error::Result;
use crate::llm::LlmEngine;
use crate::prompts::{build_commit_msg_prompt, parse_conventional_commit, ConventionalCommit};
use timeflow_core::{FileChange, SnapshotDiff, SnapshotId, SnapshotOps, TimeFlow};

/// diff 压缩阈值（超过则截断）
const MAX_DIFF_CHARS: usize = 4000;

/// Commit msg 生成器
pub struct CommitMsgGenerator<'a> {
    engine: &'a dyn LlmEngine,
}

impl<'a> CommitMsgGenerator<'a> {
    /// 创建生成器
    pub fn new(engine: &'a dyn LlmEngine) -> Self {
        Self { engine }
    }

    /// 为 diff 生成 commit msg
    ///
    /// 参数：
    /// - `diff`: 变更摘要文本
    /// - `history`: 历史 commit msg（风格参考）
    pub async fn generate(&self, diff: &str, history: &[String]) -> Result<String> {
        if diff.trim().is_empty() {
            return Ok("chore: 空变更".to_string());
        }

        let (system, user) = build_commit_msg_prompt(diff, history);
        let raw = self.engine.complete(&system, &user).await?;

        // 尝试解析为 Conventional Commit
        match parse_conventional_commit(&raw) {
            Ok(commit) => Ok(commit.to_message()),
            Err(_) => {
                // 解析失败，尝试清理后重试
                let cleaned = raw.trim();
                let first_line = cleaned.lines().next().unwrap_or(cleaned);

                // 如果第一行看起来像 commit msg（有冒号），直接用
                if first_line.contains(':') && !first_line.contains(' ') {
                    return Ok(first_line.to_string());
                }

                // 否则包装为 chore
                tracing::warn!("LLM 返回不符合 Conventional Commits 格式: {}", raw);
                Ok(format!("chore: {}", first_line))
            }
        }
    }

    /// 从 TimeFlow 快照差异生成 commit msg
    ///
    /// 自动从 diff 中提取变更摘要，调用 LLM 生成
    pub async fn generate_from_diff(
        &self,
        timeflow: &TimeFlow,
        from: &SnapshotId,
        to: &SnapshotId,
        history: &[String],
    ) -> Result<String> {
        let diff = timeflow.diff(from, to)?;
        let diff_text = format_diff(&diff);
        self.generate(&diff_text, history).await
    }

    /// 解析 LLM 响应为 ConventionalCommit 结构
    pub async fn generate_structured(
        &self,
        diff: &str,
        history: &[String],
    ) -> Result<ConventionalCommit> {
        if diff.trim().is_empty() {
            return Ok(ConventionalCommit {
                r#type: crate::prompts::CommitType::Chore,
                scope: None,
                description: "空变更".to_string(),
                body: None,
            });
        }

        let (system, user) = build_commit_msg_prompt(diff, history);
        let raw = self.engine.complete(&system, &user).await?;
        parse_conventional_commit(&raw)
    }
}

/// 将 SnapshotDiff 格式化为文本摘要
///
/// 输出格式：
/// ```text
/// 修改 3 个文件，新增 50 行，删除 12 行
///
/// M src/auth.rs (+30 -5)
/// A src/login.rs (+20)
/// D src/old_auth.rs (-12)
/// ```
pub fn format_diff(diff: &SnapshotDiff) -> String {
    if diff.files.is_empty() {
        return "无文件变更".to_string();
    }

    let total_additions: u32 = diff.files.iter().map(|f| f.additions).sum();
    let total_deletions: u32 = diff.files.iter().map(|f| f.deletions).sum();

    let mut output = format!(
        "修改 {} 个文件，新增 {} 行，删除 {} 行\n\n",
        diff.files.len(),
        total_additions,
        total_deletions
    );

    for change in &diff.files {
        let status_char = match change.status {
            timeflow_core::ChangeStatus::Added => 'A',
            timeflow_core::ChangeStatus::Modified => 'M',
            timeflow_core::ChangeStatus::Deleted => 'D',
        };
        output.push_str(&format!(
            "{} {} (+{} -{})\n",
            status_char,
            change.path.display(),
            change.additions,
            change.deletions
        ));
    }

    // 截断过长的 diff
    if output.len() > MAX_DIFF_CHARS {
        let truncated = &output[..MAX_DIFF_CHARS];
        format!("{}\n... (diff 已截断)", truncated)
    } else {
        output
    }
}

/// 从 FileChange 列表构建 diff 文本（不依赖 TimeFlow 实例）
pub fn format_file_changes(changes: &[FileChange]) -> String {
    if changes.is_empty() {
        return "无文件变更".to_string();
    }

    let total_additions: u32 = changes.iter().map(|f| f.additions).sum();
    let total_deletions: u32 = changes.iter().map(|f| f.deletions).sum();

    let mut output = format!(
        "修改 {} 个文件，新增 {} 行，删除 {} 行\n\n",
        changes.len(),
        total_additions,
        total_deletions
    );

    for change in changes {
        let status_char = match change.status {
            timeflow_core::ChangeStatus::Added => 'A',
            timeflow_core::ChangeStatus::Modified => 'M',
            timeflow_core::ChangeStatus::Deleted => 'D',
        };
        output.push_str(&format!(
            "{} {} (+{} -{})\n",
            status_char,
            change.path.display(),
            change.additions,
            change.deletions
        ));
    }

    if output.len() > MAX_DIFF_CHARS {
        let truncated = &output[..MAX_DIFF_CHARS];
        format!("{}\n... (diff 已截断)", truncated)
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockEngine;
    use std::path::PathBuf;
    use timeflow_core::{ChangeStatus, FileChange, SnapshotDiff};

    fn make_test_diff() -> SnapshotDiff {
        SnapshotDiff {
            from: "abc".to_string(),
            to: "def".to_string(),
            files: vec![
                FileChange {
                    path: PathBuf::from("src/auth.rs"),
                    status: ChangeStatus::Modified,
                    additions: 30,
                    deletions: 5,
                },
                FileChange {
                    path: PathBuf::from("src/login.rs"),
                    status: ChangeStatus::Added,
                    additions: 20,
                    deletions: 0,
                },
                FileChange {
                    path: PathBuf::from("src/old_auth.rs"),
                    status: ChangeStatus::Deleted,
                    additions: 0,
                    deletions: 12,
                },
            ],
        }
    }

    #[test]
    fn test_format_diff() {
        let diff = make_test_diff();
        let text = format_diff(&diff);
        assert!(text.contains("修改 3 个文件"));
        assert!(text.contains("新增 50 行"));
        assert!(text.contains("删除 17 行"));
        assert!(text.contains("M src/auth.rs (+30 -5)"));
        assert!(text.contains("A src/login.rs (+20 -0)"));
        assert!(text.contains("D src/old_auth.rs (+0 -12)"));
    }

    #[test]
    fn test_format_diff_empty() {
        let diff = SnapshotDiff {
            from: "a".to_string(),
            to: "b".to_string(),
            files: vec![],
        };
        let text = format_diff(&diff);
        assert_eq!(text, "无文件变更");
    }

    #[test]
    fn test_format_file_changes() {
        let changes = vec![FileChange {
            path: PathBuf::from("test.rs"),
            status: ChangeStatus::Added,
            additions: 10,
            deletions: 0,
        }];
        let text = format_file_changes(&changes);
        assert!(text.contains("A test.rs (+10 -0)"));
    }

    #[tokio::test]
    async fn test_generate_with_mock() {
        let engine = MockEngine::new("feat(auth): 添加 JWT 登录");
        let gen = CommitMsgGenerator::new(&engine);
        let result = gen.generate("test diff", &[]).await.unwrap();
        assert_eq!(result, "feat(auth): 添加 JWT 登录");
    }

    #[tokio::test]
    async fn test_generate_empty_diff() {
        let engine = MockEngine::new("should not be called");
        let gen = CommitMsgGenerator::new(&engine);
        let result = gen.generate("", &[]).await.unwrap();
        assert_eq!(result, "chore: 空变更");
    }

    #[tokio::test]
    async fn test_generate_structured() {
        let engine = MockEngine::new("feat(auth): 添加 JWT 登录");
        let gen = CommitMsgGenerator::new(&engine);
        let commit = gen.generate_structured("test diff", &[]).await.unwrap();
        assert_eq!(commit.r#type, crate::prompts::CommitType::Feat);
        assert_eq!(commit.scope, Some("auth".to_string()));
        assert_eq!(commit.description, "添加 JWT 登录");
    }

    #[tokio::test]
    async fn test_generate_fallback_on_invalid_format() {
        // LLM 返回不符合格式的文本
        let engine = MockEngine::new("这是一个普通的描述文本");
        let gen = CommitMsgGenerator::new(&engine);
        let result = gen.generate("test diff", &[]).await.unwrap();
        // 应回退为 chore: 前缀
        assert!(result.starts_with("chore:") || result.contains("这是一个普通的描述文本"));
    }

    #[tokio::test]
    async fn test_generate_with_history() {
        let engine = MockEngine::new("fix: 修复登录 bug");
        let gen = CommitMsgGenerator::new(&engine);
        let history = vec!["feat: 添加登录".to_string()];
        let result = gen.generate("test diff", &history).await.unwrap();
        assert_eq!(result, "fix: 修复登录 bug");
    }
}
