// 候选版本识别（W6 M2.2 D4）
// 设计文档: docs/ROADMAP.md § W6 M2.2 D4
//
// 目标：从快照历史中自动识别"可发布"的候选版本
//
// 识别规则（综合评分 0-100）：
//   1. 构建状态（40 分）：Green=40, Yellow=20, Red=0
//   2. 改动规模（30 分）：> 100 行=30, 30-100 行=20, < 30 行=5
//   3. 文件多样性（15 分）：> 3 文件=15, 2-3 文件=10, 1 文件=5
//   4. commit msg 质量（15 分）：Conventional Commits 格式=15, 有描述=10, 无=0
//
// 阈值：
//   - score >= 70 → Candidate（候选版本）
//   - score >= 50 → Progress（可考虑）
//   - score <  50 → Wip（不建议发布）
//
// 验收：自动标记候选版本

use crate::classify::ClassifyInput;
use crate::prompts::parse_conventional_commit;
use serde::{Deserialize, Serialize};
use timeflow_core::{BuildStatus, SnapshotType};

/// 候选版本评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScore {
    /// 快照 ID
    pub snapshot_id: String,
    /// 总分（0-100）
    pub score: u32,
    /// 各维度得分
    pub dimensions: ScoreDimensions,
    /// 推荐分类
    pub recommended_type: SnapshotType,
    /// 评分理由
    pub reason: String,
}

/// 各维度得分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDimensions {
    /// 构建状态得分（0-40）
    pub build: u32,
    /// 改动规模得分（0-30）
    pub scale: u32,
    /// 文件多样性得分（0-15）
    pub diversity: u32,
    /// commit msg 质量得分（0-15）
    pub message: u32,
}

/// 候选版本检测器
pub struct CandidateDetector;

impl CandidateDetector {
    /// 创建检测器
    pub fn new() -> Self {
        Self
    }

    /// 评估单个快照
    pub fn score(&self, input: &ClassifyInput) -> CandidateScore {
        let build_score = self.score_build(&input.build_status);
        let scale_score = self.score_scale(input);
        let diversity_score = self.score_diversity(input);
        let message_score = self.score_message(&input.message);

        let total = build_score + scale_score + diversity_score + message_score;
        let (recommended_type, reason) = self.recommend(total, input);

        CandidateScore {
            snapshot_id: input.snapshot_id.clone(),
            score: total,
            dimensions: ScoreDimensions {
                build: build_score,
                scale: scale_score,
                diversity: diversity_score,
                message: message_score,
            },
            recommended_type,
            reason,
        }
    }

    /// 从快照列表中识别所有候选版本
    pub fn detect_candidates(&self, inputs: &[ClassifyInput]) -> Vec<CandidateScore> {
        inputs
            .iter()
            .map(|i| self.score(i))
            .filter(|s| s.recommended_type == SnapshotType::Candidate)
            .collect()
    }

    /// 从快照列表中识别 top-N 候选版本（按分数降序）
    pub fn top_candidates(&self, inputs: &[ClassifyInput], n: usize) -> Vec<CandidateScore> {
        let mut scores: Vec<CandidateScore> = inputs.iter().map(|i| self.score(i)).collect();
        scores.sort_by(|a, b| b.score.cmp(&a.score));
        scores.truncate(n);
        scores
    }

    fn score_build(&self, status: &BuildStatus) -> u32 {
        match status {
            BuildStatus::Green => 40,
            BuildStatus::Yellow => 20,
            BuildStatus::Red => 0,
            BuildStatus::Unknown => 10,
        }
    }

    fn score_scale(&self, input: &ClassifyInput) -> u32 {
        let total: u32 = input
            .files
            .iter()
            .map(|f| f.additions + f.deletions)
            .sum();
        if total >= 100 {
            30
        } else if total >= 30 {
            20
        } else if total >= 10 {
            10
        } else {
            5
        }
    }

    fn score_diversity(&self, input: &ClassifyInput) -> u32 {
        let n = input.files.len();
        if n >= 3 {
            15
        } else if n >= 2 {
            10
        } else if n >= 1 {
            5
        } else {
            0
        }
    }

    fn score_message(&self, msg: &str) -> u32 {
        if msg.trim().is_empty() {
            return 0;
        }
        // 符合 Conventional Commits 格式
        if parse_conventional_commit(msg).is_ok() {
            return 15;
        }
        // 有描述（非空且 > 5 字符）
        if msg.trim().len() > 5 {
            return 10;
        }
        5
    }

    fn recommend(&self, total: u32, input: &ClassifyInput) -> (SnapshotType, String) {
        // 编译失败强制 WIP
        if input.build_status == BuildStatus::Red {
            return (
                SnapshotType::Wip,
                "编译失败，不建议作为候选版本".to_string(),
            );
        }

        if total >= 70 {
            return (
                SnapshotType::Candidate,
                format!("评分 {}/100，建议作为候选版本", total),
            );
        }

        if total >= 50 {
            return (
                SnapshotType::Progress,
                format!("评分 {}/100，可作为进度快照", total),
            );
        }

        (
            SnapshotType::Wip,
            format!("评分 {}/100，建议保留为 WIP", total),
        )
    }
}

impl Default for CandidateDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：检测候选版本
pub fn detect_candidates(inputs: &[ClassifyInput]) -> Vec<CandidateScore> {
    CandidateDetector::new().detect_candidates(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use timeflow_core::{ChangeStatus, FileChange};

    fn make_input(
        id: &str,
        msg: &str,
        build: BuildStatus,
        files: Vec<FileChange>,
    ) -> ClassifyInput {
        ClassifyInput {
            snapshot_id: id.to_string(),
            message: msg.to_string(),
            build_status: build,
            files,
            is_manual: false,
        }
    }

    fn make_file(path: &str, add: u32, del: u32) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            additions: add,
            deletions: del,
        }
    }

    /// 便捷构造：仅指定 additions，deletions 默认 0
    fn make_file_add(path: &str, add: u32) -> FileChange {
        make_file(path, add, 0)
    }

    #[test]
    fn test_high_score_candidate() {
        let detector = CandidateDetector::new();
        let input = make_input(
            "s1",
            "feat(auth): 添加 JWT 登录接口",
            BuildStatus::Green,
            vec![
                make_file("a.rs", 50, 10),
                make_file("b.rs", 30, 5),
                make_file("c.rs", 40, 8),
            ],
        );
        let score = detector.score(&input);
        // Green(40) + >=100行(30) + >=3文件(15) + Conventional(15) = 100
        assert_eq!(score.score, 100);
        assert_eq!(score.recommended_type, SnapshotType::Candidate);
        assert_eq!(score.dimensions.build, 40);
        assert_eq!(score.dimensions.scale, 30);
        assert_eq!(score.dimensions.diversity, 15);
        assert_eq!(score.dimensions.message, 15);
    }

    #[test]
    fn test_build_red_forces_wip() {
        let detector = CandidateDetector::new();
        let input = make_input(
            "s1",
            "feat: 大功能",
            BuildStatus::Red,
            vec![
                make_file("a.rs", 200, 0),
                make_file("b.rs", 100, 0),
            ],
        );
        let score = detector.score(&input);
        assert_eq!(score.recommended_type, SnapshotType::Wip);
        assert!(score.reason.contains("编译失败"));
    }

    #[test]
    fn test_low_score_wip() {
        let detector = CandidateDetector::new();
        let input = make_input(
            "s1",
            "fix",
            BuildStatus::Unknown,
            vec![make_file("a.rs", 2, 1)],
        );
        let score = detector.score(&input);
        // Unknown(10) + <10行(5) + 1文件(5) + 短msg(10) = 30
        assert!(score.score < 50);
        assert_eq!(score.recommended_type, SnapshotType::Wip);
    }

    #[test]
    fn test_medium_score_progress() {
        let detector = CandidateDetector::new();
        let input = make_input(
            "s1",
            "添加登录功能",
            BuildStatus::Yellow,
            vec![make_file("login.rs", 40, 5)],
        );
        let score = detector.score(&input);
        // Yellow(20) + 30-100行(20) + 1文件(5) + 有描述(10) = 55
        assert!(score.score >= 50 && score.score < 70);
        assert_eq!(score.recommended_type, SnapshotType::Progress);
    }

    #[test]
    fn test_detect_candidates_filters() {
        let detector = CandidateDetector::new();
        let inputs = vec![
            // 候选版本
            make_input(
                "s1",
                "feat: 完整功能",
                BuildStatus::Green,
                vec![make_file_add("a.rs", 60), make_file_add("b.rs", 50), make_file_add("c.rs", 40)],
            ),
            // WIP
            make_input("s2", "fix", BuildStatus::Red, vec![make_file_add("a.rs", 5)]),
            // 候选版本
            make_input(
                "s3",
                "feat(api): 添加用户接口",
                BuildStatus::Green,
                vec![make_file_add("a.rs", 80), make_file_add("b.rs", 30), make_file_add("c.rs", 20)],
            ),
            // Progress
            make_input("s4", "改动", BuildStatus::Yellow, vec![make_file_add("a.rs", 40)]),
        ];
        let candidates = detector.detect_candidates(&inputs);
        assert_eq!(candidates.len(), 2);
        let ids: Vec<&str> = candidates.iter().map(|c| c.snapshot_id.as_str()).collect();
        assert!(ids.contains(&"s1"));
        assert!(ids.contains(&"s3"));
    }

    #[test]
    fn test_top_candidates_sorted() {
        let detector = CandidateDetector::new();
        let inputs = vec![
            make_input("low", "fix", BuildStatus::Red, vec![make_file_add("a.rs", 5)]),
            make_input(
                "high",
                "feat: 完整功能",
                BuildStatus::Green,
                vec![make_file_add("a.rs", 60), make_file_add("b.rs", 50), make_file_add("c.rs", 40)],
            ),
            make_input(
                "mid",
                "改动",
                BuildStatus::Yellow,
                vec![make_file_add("a.rs", 40)],
            ),
        ];
        let top = detector.top_candidates(&inputs, 2);
        assert_eq!(top.len(), 2);
        // 应按分数降序
        assert!(top[0].score >= top[1].score);
        assert_eq!(top[0].snapshot_id, "high");
    }

    #[test]
    fn test_conventional_msg_scores_higher() {
        let detector = CandidateDetector::new();

        let conventional = make_input(
            "s1",
            "feat(auth): 添加 JWT 登录",
            BuildStatus::Green,
            vec![make_file_add("a.rs", 50)],
        );
        let plain = make_input(
            "s2",
            "添加登录",
            BuildStatus::Green,
            vec![make_file_add("a.rs", 50)],
        );

        let s1 = detector.score(&conventional);
        let s2 = detector.score(&plain);
        assert!(s1.dimensions.message > s2.dimensions.message);
        assert!(s1.score > s2.score);
    }

    #[test]
    fn test_empty_message_scores_zero() {
        let detector = CandidateDetector::new();
        let input = make_input("s1", "", BuildStatus::Green, vec![make_file_add("a.rs", 50)]);
        let score = detector.score(&input);
        assert_eq!(score.dimensions.message, 0);
    }

    #[test]
    fn test_detect_candidates_convenience_fn() {
        let inputs = vec![make_input(
            "s1",
            "feat: 完整功能",
            BuildStatus::Green,
            vec![make_file_add("a.rs", 60), make_file_add("b.rs", 50), make_file_add("c.rs", 40)],
        )];
        let candidates = detect_candidates(&inputs);
        assert_eq!(candidates.len(), 1);
    }
}
