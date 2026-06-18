// 集成测试：验证三种模式的端到端行为
// 重点验证 SyncMode 真实创建 git commit

use std::fs;
use std::path::Path;

use aw_git::{GitAdapter, GitConfig, GitMode, GitModeManager, ReleaseMode, StealthMode, SyncMode};
use chrono::Utc;
use tempfile::TempDir;
use timeflow_core::{
    BuildStatus, Snapshot, SnapshotMetadata, SnapshotOps, SnapshotType, TimeFlow,
    TriggerType,
};

fn make_config(repo_path: &Path) -> GitConfig {
    GitConfig {
        repo_path: repo_path.to_path_buf(),
        author_name: "Test User".to_string(),
        author_email: "test@example.com".to_string(),
        remote_url: None,
        remote_name: "origin".to_string(),
        auto_push: false,
    }
}

fn write_file(repo: &Path, rel: &str, content: &str) {
    let path = repo.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn make_release_snapshot(id: &str, msg: &str, tree: &str) -> Snapshot {
    Snapshot {
        id: id.to_string(),
        parent: None,
        timestamp: Utc::now(),
        tree: tree.to_string(),
        author: "test".to_string(),
        metadata: SnapshotMetadata {
            trigger: TriggerType::Manual,
            ai_summary: None,
            ai_commit_msg: Some(msg.to_string()),
            build_status: BuildStatus::Green,
            snapshot_type: SnapshotType::Release,
            tags: vec![],
        },
    }
}

fn make_wip_snapshot(id: &str, msg: &str, tree: &str) -> Snapshot {
    Snapshot {
        id: id.to_string(),
        parent: None,
        timestamp: Utc::now(),
        tree: tree.to_string(),
        author: "test".to_string(),
        metadata: SnapshotMetadata {
            trigger: TriggerType::Manual,
            ai_summary: None,
            ai_commit_msg: Some(msg.to_string()),
            build_status: BuildStatus::Unknown,
            snapshot_type: SnapshotType::Wip,
            tags: vec![],
        },
    }
}

/// 测试 StealthMode 完全不创建 .git 目录
#[test]
fn test_stealth_never_creates_git() {
    let tmp = TempDir::new().unwrap();
    let mode = StealthMode::new();

    mode.ensure_init(tmp.path()).unwrap();

    // 创建一些文件
    write_file(tmp.path(), "a.txt", "hello");

    // 模拟快照
    let tf = TimeFlow::new(tmp.path()).unwrap();
    let snap = make_wip_snapshot("abc12345", "test", "tree1");
    mode.on_snapshot(&snap, &tf, tmp.path()).unwrap();

    // .git 不应存在
    assert!(!tmp.path().join(".git").exists());
}

/// 测试 SyncMode 真实创建 git commit
#[test]
fn test_sync_creates_real_commit() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = SyncMode::new(config);

    // 初始化 git
    mode.ensure_init(tmp.path()).unwrap();
    assert!(tmp.path().join(".git").exists());

    // 创建文件 + TimeFlow 快照
    write_file(tmp.path(), "src/main.rs", "fn main() {}");
    let tf = TimeFlow::new(tmp.path()).unwrap();
    let snap_id = tf.snapshot(Some("初始版本")).unwrap();
    let snap = tf.get_snapshot(&snap_id).unwrap();

    // SyncMode 处理快照 → 应创建 git commit
    mode.on_snapshot(&snap, &tf, tmp.path()).unwrap();

    // 验证 git 仓库有 1 个 commit
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let head = repo.head().expect("HEAD 应存在");
    let commit = head.peel_to_commit().unwrap();
    let commit_msg = commit.message().unwrap_or("");
    assert!(commit_msg.contains("初始版本"), "commit 消息应包含快照 msg");
    assert!(commit_msg.contains("TimeFlow-Snapshot:"), "commit 消息应包含快照 ID");
}

/// 测试 SyncMode 多次快照创建多个 commit
#[test]
fn test_sync_multiple_commits() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = SyncMode::new(config);

    mode.ensure_init(tmp.path()).unwrap();
    let tf = TimeFlow::new(tmp.path()).unwrap();

    // 第一次快照
    write_file(tmp.path(), "a.txt", "v1");
    let snap1 = tf.snapshot(Some("v1")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&snap1).unwrap(), &tf, tmp.path())
        .unwrap();

    // 第二次快照
    write_file(tmp.path(), "a.txt", "v2");
    let snap2 = tf.snapshot(Some("v2")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&snap2).unwrap(), &tf, tmp.path())
        .unwrap();

    // 验证有 2 个 commit
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    let count = walk.count();
    assert_eq!(count, 2, "应有 2 个 commit，实际 {}", count);
}

/// 测试 SyncMode 无变更时跳过 commit
#[test]
fn test_sync_skips_empty_commit() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = SyncMode::new(config);

    mode.ensure_init(tmp.path()).unwrap();
    let tf = TimeFlow::new(tmp.path()).unwrap();

    // 第一次快照
    write_file(tmp.path(), "a.txt", "v1");
    let snap1 = tf.snapshot(Some("v1")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&snap1).unwrap(), &tf, tmp.path())
        .unwrap();

    // 第二次快照（无变更）
    let snap2 = tf.snapshot(Some("v2-no-change")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&snap2).unwrap(), &tf, tmp.path())
        .unwrap();

    // 应只有 1 个 commit（第二次无变更被跳过）
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    let count = walk.count();
    assert_eq!(count, 1, "无变更应跳过 commit，实际 {}", count);
}

/// 测试 ReleaseMode 只处理 Release 类型快照
#[test]
fn test_release_only_handles_release_snapshots() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = ReleaseMode::new(config);

    mode.ensure_init(tmp.path()).unwrap();
    let tf = TimeFlow::new(tmp.path()).unwrap();

    // WIP 快照 → 不应创建 commit
    write_file(tmp.path(), "a.txt", "wip");
    let wip_snap = tf.snapshot(Some("wip")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&wip_snap).unwrap(), &tf, tmp.path())
        .unwrap();

    // 验证没有 commit
    let repo = git2::Repository::open(tmp.path()).unwrap();
    assert!(repo.head().is_err(), "WIP 快照不应创建 commit");

    // Release 快照 → 应创建 commit + tag
    // 注意：TimeFlow 的 snapshot() 默认创建 Progress 类型，需要手动构造 Release 快照
    write_file(tmp.path(), "a.txt", "release v1.0");
    let _progress_snap = tf.snapshot(Some("progress")).unwrap();
    // 手动构造 Release 类型快照（模拟 AI 识别后的标记）
    let release_snap = make_release_snapshot("release001", "Release v1.0", "tree-fake");
    mode.on_snapshot(&release_snap, &tf, tmp.path())
        .unwrap();

    // 验证有 commit
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let head = repo.head().expect("Release 快照应创建 commit");
    let commit = head.peel_to_commit().unwrap();
    assert!(commit.message().unwrap_or("").contains("Release v1.0"));

    // 验证有 tag
    let tag_ref = "refs/tags/release-release0";
    assert!(
        repo.refname_to_id(tag_ref).is_ok(),
        "应创建 git tag: release-release0"
    );
}

/// 测试模式切换持久化
#[test]
fn test_mode_switch_persists() {
    let tmp = TempDir::new().unwrap();
    let mgr = GitModeManager::new(tmp.path());

    // 默认 stealth
    assert_eq!(mgr.load_mode().unwrap(), GitMode::Stealth);

    // 切换到 sync
    mgr.save_mode(GitMode::Sync).unwrap();
    assert_eq!(mgr.load_mode().unwrap(), GitMode::Sync);

    // 切换到 release
    mgr.save_mode(GitMode::Release).unwrap();
    assert_eq!(mgr.load_mode().unwrap(), GitMode::Release);

    // 切回 stealth
    mgr.save_mode(GitMode::Stealth).unwrap();
    assert_eq!(mgr.load_mode().unwrap(), GitMode::Stealth);
}

/// 测试 .gitignore 自动包含 .yunji
#[test]
fn test_gitignore_excludes_yunji() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = SyncMode::new(config);

    mode.ensure_init(tmp.path()).unwrap();

    // 验证 .gitignore 内容包含 .yunji
    let gitignore = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(
        gitignore.lines().any(|l| l.trim() == ".yunji/"),
        ".gitignore 应包含 .yunji/ 行，实际内容: {}",
        gitignore
    );

    // 创建 .yunji 目录下的文件
    write_file(tmp.path(), ".yunji/timeflow/config.toml", "test");

    // git status 不应包含 .yunji 文件（使用显式选项确保排除 ignored）
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);
    let statuses = repo.statuses(Some(&mut opts)).unwrap();

    let has_yunji = statuses.iter().any(|s| {
        s.path()
            .map(|p| p.starts_with(".yunji"))
            .unwrap_or(false)
    });
    let all_paths: Vec<String> = statuses
        .iter()
        .filter_map(|s| s.path().map(|p| p.to_string()))
        .collect();
    assert!(
        !has_yunji,
        ".yunji 应被 .gitignore 排除，但 statuses 中包含: {:?}",
        all_paths
    );
}

/// 测试 SyncMode status 返回正确信息
#[test]
fn test_sync_status_after_commit() {
    let tmp = TempDir::new().unwrap();
    let config = make_config(tmp.path());
    let mode = SyncMode::new(config);

    mode.ensure_init(tmp.path()).unwrap();
    let tf = TimeFlow::new(tmp.path()).unwrap();

    write_file(tmp.path(), "a.txt", "v1");
    let snap = tf.snapshot(Some("v1")).unwrap();
    mode.on_snapshot(&tf.get_snapshot(&snap).unwrap(), &tf, tmp.path())
        .unwrap();

    let status = mode.status(tmp.path()).unwrap();
    assert!(status.initialized);
    assert!(status.current_branch.is_some());
    assert!(status.last_commit.is_some());
    assert_eq!(status.unpushed_commits, 0); // 没有远程，所以未推送 = 0
}
