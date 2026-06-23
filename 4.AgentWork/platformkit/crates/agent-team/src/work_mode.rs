// 工作模式（M6.2 W5）
//
// 目标：让同一个员工支持学习 / 教学 / 协作 / 自动 / 审批五种工作风格。
// 这是 AW 相对 Hermes 的"用户可选模式"差异化能力：
//   - Learn      Agent 主动观察用户操作、记录偏好
//   - Teach      用户手把手教，Agent 记录为 Skill
//   - Collaborate 日常协作，Agent 主动建议
//   - Auto       全自动执行
//   - Approval   每步都要用户确认（对应 zcode Plan Mode）

use serde::{Deserialize, Serialize};

/// 工作模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkMode {
    /// 引导式学习（Agent 主动观察）
    Learn,
    /// 讲解式教学（用户教 Agent）
    Teach,
    /// 讨论式协作
    Collaborate,
    /// 自动执行
    Auto,
    /// 先计划后审批（对应 zcode Plan Mode）
    Approval,
}

impl Default for WorkMode {
    fn default() -> Self {
        Self::Collaborate
    }
}

impl WorkMode {
    /// 中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::Learn => "学习模式",
            Self::Teach => "教学模式",
            Self::Collaborate => "协作模式",
            Self::Auto => "自动模式",
            Self::Approval => "审批模式",
        }
    }

    /// 图标 emoji（前端展示用）
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Learn => "🧠",
            Self::Teach => "👨‍🏫",
            Self::Collaborate => "🤝",
            Self::Auto => "🚀",
            Self::Approval => "🔒",
        }
    }

    /// 英文标识（序列化/配置文件用）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Learn => "learn",
            Self::Teach => "teach",
            Self::Collaborate => "collaborate",
            Self::Auto => "auto",
            Self::Approval => "approval",
        }
    }

    /// 从字符串解析
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "learn" | "learning" | "学习" => Self::Learn,
            "teach" | "teaching" | "教学" => Self::Teach,
            "auto" | "autonomous" | "自动" => Self::Auto,
            "approval" | "审批" | "approve" => Self::Approval,
            _ => Self::Collaborate,
        }
    }

    /// 是否需要审批（高风险操作需用户确认）
    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Approval)
    }

    /// 是否应该记录学习（用于自进化触发）
    pub fn should_learn(&self) -> bool {
        matches!(self, Self::Learn | Self::Teach)
    }

    /// 用户参与度描述（前端展示用）
    pub fn user_involvement(&self) -> &'static str {
        match self {
            Self::Learn => "低（用户正常工作）",
            Self::Teach => "高（手把手教）",
            Self::Collaborate => "中（互动协作）",
            Self::Auto => "极低（只看结果）",
            Self::Approval => "极高（每步审批）",
        }
    }

    /// 注入 system_prompt 的模式提示
    pub fn system_prompt_hint(&self) -> &'static str {
        match self {
            Self::Learn => {
                "【学习模式】你应该主动观察用户操作，记录偏好与习惯，但不要打断用户。\
                 在任务结束后，把可沉淀的事实/偏好/坑点提炼出来。"
            }
            Self::Teach => {
                "【教学模式】用户正在教你。请认真记录用户给出的步骤，\
                 并在结束时把这些步骤封装为一个可复用的 Skill。"
            }
            Self::Collaborate => {
                "【协作模式】与用户一起完成任务，主动给出建议，\
                 但关键决策请用户拍板。"
            }
            Self::Auto => {
                "【自动模式】全自动执行任务，完成后统一汇报结果。\
                 遇到不可恢复的错误才停下来询问用户。"
            }
            Self::Approval => {
                "【审批模式】先输出完整计划等待用户批准，批准后再执行。\
                 每一步执行前都要让用户确认。"
            }
        }
    }

    /// 全部模式（前端选择器用）
    pub fn all() -> [WorkMode; 5] {
        [
            Self::Learn,
            Self::Teach,
            Self::Collaborate,
            Self::Auto,
            Self::Approval,
        ]
    }
}

impl std::fmt::Display for WorkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_and_icon_distinct() {
        let labels: Vec<_> = WorkMode::all().iter().map(|m| m.label()).collect();
        let icons: Vec<_> = WorkMode::all().iter().map(|m| m.icon()).collect();
        let label_set: std::collections::HashSet<_> = labels.iter().collect();
        let icon_set: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(label_set.len(), 5);
        assert_eq!(icon_set.len(), 5);
    }

    #[test]
    fn as_str_roundtrip() {
        for mode in WorkMode::all() {
            let s = mode.as_str();
            assert_eq!(WorkMode::from_str_lossy(s), mode);
        }
    }

    #[test]
    fn from_str_chinese() {
        assert_eq!(WorkMode::from_str_lossy("学习"), WorkMode::Learn);
        assert_eq!(WorkMode::from_str_lossy("教学"), WorkMode::Teach);
        assert_eq!(WorkMode::from_str_lossy("自动"), WorkMode::Auto);
        assert_eq!(WorkMode::from_str_lossy("审批"), WorkMode::Approval);
    }

    #[test]
    fn from_str_unknown_defaults_collaborate() {
        assert_eq!(WorkMode::from_str_lossy("xxx"), WorkMode::Collaborate);
        assert_eq!(WorkMode::from_str_lossy(""), WorkMode::Collaborate);
    }

    #[test]
    fn requires_approval_only_for_approval() {
        for mode in WorkMode::all() {
            assert_eq!(mode.requires_approval(), mode == WorkMode::Approval);
        }
    }

    #[test]
    fn should_learn_for_learn_and_teach() {
        assert!(WorkMode::Learn.should_learn());
        assert!(WorkMode::Teach.should_learn());
        assert!(!WorkMode::Auto.should_learn());
        assert!(!WorkMode::Approval.should_learn());
        assert!(!WorkMode::Collaborate.should_learn());
    }

    #[test]
    fn system_prompt_hint_non_empty() {
        for mode in WorkMode::all() {
            assert!(!mode.system_prompt_hint().is_empty());
            assert!(mode.system_prompt_hint().contains("【"));
        }
    }

    #[test]
    fn display_uses_as_str() {
        assert_eq!(format!("{}", WorkMode::Auto), "auto");
        assert_eq!(format!("{}", WorkMode::Approval), "approval");
    }

    #[test]
    fn default_is_collaborate() {
        assert_eq!(WorkMode::default(), WorkMode::Collaborate);
    }
}
