// 三种 Git 模式实现
// 设计文档: docs/TIMEFLOW-DESIGN.md § 7

pub mod stealth;
pub mod sync;
pub mod release;

pub use stealth::StealthMode;
pub use sync::SyncMode;
pub use release::ReleaseMode;
