// 快照分类（WIP / 进度 / 候选 / 正式）
// 设计文档: docs/ROADMAP.md § W6 M2.2 D1
//
// 目标：根据 diff 规模、构建状态、改动文件类型，自动将快照分为四类：
//   - Wip       改动小或未完成（< 10 行 / 仅注释/空格）
//   - Progress  有意义但非正式（10-100 行 / 单文件功能）
//   - Candidate 编译通过 + 测试通过 + 改动充分（> 100 行 / 多文件）
//   - Release   用户确认或触发词（"release"/"发布"/"v1.0"）
//
// 分类策略：
//   1. 规则优先：触发词、构建状态、diff 规模
//   2. AI 辅助：规则不确定时调用 LLM 判断
//
// 验收：AI 分类准确率 > 80%

use crate::error::Result;
use crate::llm::LlmEngine;
use crate::prompts::build_classify_prompt;
use timeflow_core::{BuildStatus, FileChange, Snapshot, SnapshotType};

/// 分类输入（不依赖 TimeFlow 实例，便于测试）
#[derive(Debug, Clone)]
pub struct ClassifyInput {
    /// 快照 ID
    pub snapshot_id: String,
    /// commit message（用户手动输入或 AI 生成）
    pub message: String,
    /// 构建状态
    pub build_status: BuildStatus,
    /// 文件变更列表
    pub files: Vec<FileChange>,
    /// 是否用户手动触发
    pub is_manual: bool,
}

/// 分类结果
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    /// 分类后的快照类型
    pub snapshot_type: SnapshotType,
    /// 分类原因（用于 UI 展示）
    pub reason: String,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
    /// 是否由 AI 决定（false = 规则决定）
    pub by_ai: bool,
}

/// 快照分类器
pub struct SnapshotClassifier<'a> {
    /// LLM 引擎（可选，规则不确定时使用）
    engine: Option<&'a dyn LlmEngine>,
}

impl<'a> SnapshotClassifier<'a> {
    /// 创建分类器（无 LLM，仅规则）
    pub fn new_rules_only() -> Self {
        Self { engine: None }
    }

    /// 创建分类器（带 LLM 辅助）
    pub fn new_with_llm(engine: &'a dyn LlmEngine) -> Self {
        Self { engine: Some(engine) }
    }

    /// 分类
    pub async fn classify(&self, input: &ClassifyInput) -> Result<ClassifyResult> {
        // 1. 触发词检测 → Release
        if let Some(result) = self.check_release_trigger(input) {
            return Ok(result);
        }

        // 2. 规则分类
        if let Some(result) = self.classify_by_rules(input) {
            return Ok(result);
        }

        // 3. AI 辅助分类（规则不确定时）
        if let Some(engine) = self.engine {
            return self.classify_by_ai(engine, input).await;
        }

        // 4. 兜底：Progress
        Ok(ClassifyResult {
            snapshot_type: SnapshotType::Progress,
            reason: "规则无法确定，默认为进度快照".to_string(),
            confidence: 0.5,
            by_ai: false,
        })
    }
}

/// 触发词列表（中英双语）
const RELEASE_TRIGGERS: &[&str] = &[
    "release",
    "发布",
    "ship",
    "deploy",
    "v1.0",
    "v2.0",
    "正式版",
    "stable",
];

/// WIP 触发词
const WIP_TRIGGERS: &[&str] = &["wip", "todo", "临时", "草稿", "draft", "tmp"];

impl<'a> SnapshotClassifier<'a> {
    /// 检测 Release 触发词
    fn check_release_trigger(&self, input: &ClassifyInput) -> Option<ClassifyResult> {
        let msg_lower = input.message.to_lowercase();
        for trigger in RELEASE_TRIGGERS {
            if msg_lower.contains(trigger) {
                return Some(ClassifyResult {
                    snapshot_type: SnapshotType::Release,
                    reason: format!("检测到触发词 '{}'", trigger),
                    confidence: 0.95,
                    by_ai: false,
                });
            }
        }
        None
    }

    /// 规则分类
    fn classify_by_rules(&self, input: &ClassifyInput) -> Option<ClassifyResult> {
        // WIP 触发词
        let msg_lower = input.message.to_lowercase();
        for trigger in WIP_TRIGGERS {
            if msg_lower.contains(trigger) {
                return Some(ClassifyResult {
                    snapshot_type: SnapshotType::Wip,
                    reason: format!("检测到 WIP 触发词 '{}'", trigger),
                    confidence: 0.9,
                    by_ai: false,
                });
            }
        }

        // 统计改动规模
        let total_additions: u32 = input.files.iter().map(|f| f.additions).sum();
        let total_deletions: u32 = input.files.iter().map(|f| f.deletions).sum();
        let total_changes = total_additions + total_deletions;
        let file_count = input.files.len();

        // 编译失败 → WIP
        if input.build_status == BuildStatus::Red {
            return Some(ClassifyResult {
                snapshot_type: SnapshotType::Wip,
                reason: "编译失败，标记为 WIP".to_string(),
                confidence: 0.85,
                by_ai: false,
            });
        }

        // 改动 < 10 行 → WIP
        if total_changes < 10 {
            return Some(ClassifyResult {
                snapshot_type: SnapshotType::Wip,
                reason: format!("改动仅 {} 行，标记为 WIP", total_changes),
                confidence: 0.8,
                by_ai: false,
            });
        }

        // 编译通过 + 测试通过 + 改动充分（> 100 行 + 多文件）→ Candidate
        if input.build_status == BuildStatus::Green
            && total_changes >= 100
            && file_count >= 3
        {
            return Some(ClassifyResult {
                snapshot_type: SnapshotType::Candidate,
                reason: format!(
                    "编译+测试通过，改动 {} 行 / {} 文件，标记为候选版本",
                    total_changes, file_count
                ),
                confidence: 0.85,
                by_ai: false,
            });
        }

        // 10-100 行 → Progress
        if total_changes >= 10 && total_changes < 100 {
            return Some(ClassifyResult {
                snapshot_type: SnapshotType::Progress,
                reason: format!("改动 {} 行，标记为进度快照", total_changes),
                confidence: 0.75,
                by_ai: false,
            });
        }

        // 其余情况返回 None，交给 AI
        None
    }

    /// AI 辅助分类
    async fn classify_by_ai(
        &self,
        engine: &dyn LlmEngine,
        input: &ClassifyInput,
    ) -> Result<ClassifyResult> {
        let (system, user) = build_classify_prompt(input);
        let raw = engine.complete(&system, &user).await?;
        let raw = raw.trim();

        // 解析 AI 返回（期望单字符或单词：wip/progress/candidate/release）
        let snapshot_type = parse_snapshot_type(raw).unwrap_or(SnapshotType::Progress);

        let reason = format!("AI 判断为 {}", type_name(&snapshot_type));
        Ok(ClassifyResult {
            snapshot_type,
            reason,
            confidence: 0.7,
            by_ai: true,
        })
    }
}

/// 从 Snapshot 构建 ClassifyInput
pub fn snapshot_to_input(snapshot: &Snapshot, files: Vec<FileChange>) -> ClassifyInput {
    let message = snapshot
        .metadata
        .ai_commit_msg
        .clone()
        .unwrap_or_else(|| "(无 commit msg)".to_string());
    let is_manual = snapshot.metadata.trigger == timeflow_core::TriggerType::Manual;

    ClassifyInput {
        snapshot_id: snapshot.id.clone(),
        message,
        build_status: snapshot.metadata.build_status.clone(),
        files,
        is_manual,
    }
}

/// 解析 AI 返回的快照类型
fn parse_snapshot_type(s: &str) -> Option<SnapshotType> {
    let lower = s.to_lowercase();
    // 取第一行第一词
    let first = lower.split_whitespace().next()?;
    match first {
        "wip" | "1" => Some(SnapshotType::Wip),
        "progress" | "2" => Some(SnapshotType::Progress),
        "candidate" | "3" => Some(SnapshotType::Candidate),
        "release" | "4" => Some(SnapshotType::Release),
        _ => None,
    }
}

/// 类型中文名
fn type_name(t: &SnapshotType) -> &'static str {
    match t {
        SnapshotType::Wip => "WIP",
        SnapshotType::Progress => "进度",
        SnapshotType::Candidate => "候选",
        SnapshotType::Release => "正式",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockEngine;
    use std::path::PathBuf;
    use timeflow_core::ChangeStatus;

    fn make_input(message: &str, build: BuildStatus, files: Vec<FileChange>) -> ClassifyInput {
        ClassifyInput {
            snapshot_id: "test".to_string(),
            message: message.to_string(),
            build_status: build,
            files,
            is_manual: false,
        }
    }

    fn make_file(path: &str, status: ChangeStatus, add: u32, del: u32) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            status,
            additions: add,
            deletions: del,
        }
    }

    #[tokio::test]
    async fn test_release_trigger() {
        let clf = SnapshotClassifier::new_rules_only();
        let input = make_input("release v1.0", BuildStatus::Green, vec![]);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Release);
        assert!(result.confidence > 0.9);
    }

    #[tokio::test]
    async fn test_wip_trigger() {
        let clf = SnapshotClassifier::new_rules_only();
        let input = make_input("wip: 临时改动", BuildStatus::Unknown, vec![]);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Wip);
    }

    #[tokio::test]
    async fn test_small_change_is_wip() {
        let clf = SnapshotClassifier::new_rules_only();
        let input = make_input(
            "fix: 拼写错误",
            BuildStatus::Green,
            vec![make_file("a.rs", ChangeStatus::Modified, 3, 1)],
        );
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Wip);
    }

    #[tokio::test]
    async fn test_build_red_is_wip() {
        let clf = SnapshotClassifier::new_rules_only();
        let input = make_input(
            "feat: 大功能",
            BuildStatus::Red,
            vec![make_file("a.rs", ChangeStatus::Added, 200, 0)],
        );
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Wip);
    }

    #[tokio::test]
    async fn test_medium_change_is_progress() {
        let clf = SnapshotClassifier::new_rules_only();
        let input = make_input(
            "feat: 添加登录",
            BuildStatus::Green,
            vec![make_file("login.rs", ChangeStatus::Added, 50, 0)],
        );
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Progress);
    }

    #[tokio::test]
    async fn test_large_green_is_candidate() {
        let clf = SnapshotClassifier::new_rules_only();
        let files = vec![
            make_file("a.rs", ChangeStatus::Added, 60, 0),
            make_file("b.rs", ChangeStatus::Modified, 30, 10),
            make_file("c.rs", ChangeStatus::Modified, 20, 5),
        ];
        let input = make_input("feat: 完整功能", BuildStatus::Green, files);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Candidate);
        assert!(result.reason.contains("候选"));
    }

    #[tokio::test]
    async fn test_ai_fallback() {
        // 大改动但编译失败 → 规则返回 WIP；这里测试 AI 路径
        // 改动 100+ 行但单文件 → 规则不命中 candidate，进入 AI
        let engine = MockEngine::new("candidate");
        let clf = SnapshotClassifier::new_with_llm(&engine);
        let files = vec![make_file("big.rs", ChangeStatus::Added, 150, 0)];
        let input = make_input("feat: 大改动单文件", BuildStatus::Green, files);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Candidate);
        assert!(result.by_ai);
    }

    #[tokio::test]
    async fn test_ai_fallback_progress_default() {
        // AI 返回无法解析时，默认 Progress
        let engine = MockEngine::new("unknown garbage");
        let clf = SnapshotClassifier::new_with_llm(&engine);
        let files = vec![make_file("big.rs", ChangeStatus::Added, 150, 0)];
        let input = make_input("feat: 大改动单文件", BuildStatus::Green, files);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Progress);
    }

    #[tokio::test]
    async fn test_no_llm_fallback_progress() {
        // 无 LLM 且规则不命中 → 默认 Progress
        let clf = SnapshotClassifier::new_rules_only();
        let files = vec![make_file("big.rs", ChangeStatus::Added, 150, 0)];
        let input = make_input("feat: 大改动单文件", BuildStatus::Green, files);
        let result = clf.classify(&input).await.unwrap();
        assert_eq!(result.snapshot_type, SnapshotType::Progress);
        assert!(!result.by_ai);
    }

    #[test]
    fn test_parse_snapshot_type() {
        assert_eq!(parse_snapshot_type("wip"), Some(SnapshotType::Wip));
        assert_eq!(parse_snapshot_type("WIP"), Some(SnapshotType::Wip));
        assert_eq!(parse_snapshot_type("1"), Some(SnapshotType::Wip));
        assert_eq!(parse_snapshot_type("progress"), Some(SnapshotType::Progress));
        assert_eq!(parse_snapshot_type("candidate"), Some(SnapshotType::Candidate));
        assert_eq!(parse_snapshot_type("release"), Some(SnapshotType::Release));
        assert_eq!(parse_snapshot_type("garbage"), None);
        assert_eq!(parse_snapshot_type(""), None);
    }
}
