// AgentWork Git 兼容层
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7
//
// W4 M1.4: 实现 TimeFlow ↔ Git 双模兼容
// 三种模式：
//   - Stealth（隐身）：纯本地 TimeFlow，不碰 git（默认）
//   - Sync（同步）：每个 TimeFlow 快照自动镜像为 git commit
//   - Release（发布）：只把 Release 类型的快照推送到 git

#![warn(missing_docs)]

mod error;
mod modes;
mod adapter;
mod config;

pub use error::{GitError, Result};
pub use adapter::{GitAdapter, GitStatus};
pub use modes::{StealthMode, SyncMode, ReleaseMode};
pub use config::{GitMode, GitConfig, GitModeManager};

/// 根据模式创建对应的 GitAdapter
pub fn create_adapter(mode: GitMode, config: GitConfig) -> Box<dyn GitAdapter> {
    match mode {
        GitMode::Stealth => Box::new(StealthMode::new()),
        GitMode::Sync => Box::new(SyncMode::new(config)),
        GitMode::Release => Box::new(ReleaseMode::new(config)),
    }
}
