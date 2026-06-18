// TimeFlow 本地版本控制引擎
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3
//
// W1 阶段: 定义核心数据结构和 trait，提供编译通过的骨架
// W2 阶段: 实现内容寻址存储 + 快照链 + 回滚 + 分支

#![warn(missing_docs)]

mod error;
mod types;
mod storage;
mod snapshot;
mod branch;
mod watcher;

pub use error::Error;
pub use types::*;
pub use storage::Storage;
pub use snapshot::{SnapshotOps, SnapshotFilter};
pub use branch::BranchOps;
pub use watcher::{Watcher, WatcherConfig, DEFAULT_DEBOUNCE_SECS};

use crate::error::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// TimeFlow 引擎主入口
///
/// 所有版本控制操作通过此结构体发起。
pub struct TimeFlow {
    /// 存储引擎
    storage: Storage,
    /// 仓库路径
    repo_path: PathBuf,
}

/// 默认忽略的目录/文件（不纳入版本控制）
const IGNORED_PATHS: &[&str] = &[
    ".yunji",   // TimeFlow 自身数据
    ".git",     // Git 数据
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
];

impl TimeFlow {
    /// 初始化 TimeFlow 引擎
    ///
    /// 打开或创建 `.yunji/timeflow/` 目录结构。
    /// 首次初始化时会自动创建 main 分支（指向空快照）。
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let storage = Storage::new(&repo_path)?;

        // 首次初始化：确保 main 分支存在
        let head_branch = storage.read_head()?;
        if storage.read_branch(&head_branch).is_err() {
            // main 分支不存在，创建一个初始空快照作为起点
            let initial_snapshot = Self::create_initial_snapshot(&storage)?;
            let branch = Branch {
                name: head_branch.clone(),
                head: initial_snapshot,
                created_at: chrono::Utc::now(),
            };
            storage.store_branch(&branch)?;
            tracing::info!("TimeFlow 初始化 main 分支: {}", head_branch);
        }

        tracing::info!("TimeFlow 引擎就绪: {}", repo_path.display());

        Ok(Self { storage, repo_path })
    }

    /// 获取仓库路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// 获取存储引擎引用（供高级用法）
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// 创建初始空快照（仓库起点）
    fn create_initial_snapshot(storage: &Storage) -> Result<SnapshotId> {
        let empty_tree = Tree {
            id: String::new(),
            entries: HashMap::new(),
        };
        let tree_id = storage.store_tree(&empty_tree)?;

        let snapshot = Snapshot {
            id: String::new(),
            parent: None,
            timestamp: chrono::Utc::now(),
            tree: tree_id,
            author: "system".to_string(),
            metadata: SnapshotMetadata {
                trigger: TriggerType::Manual,
                ai_summary: Some("初始快照（空仓库）".to_string()),
                ai_commit_msg: Some("chore: 初始化 TimeFlow 仓库".to_string()),
                build_status: BuildStatus::Unknown,
                snapshot_type: SnapshotType::Progress,
                tags: vec!["init".to_string()],
            },
        };

        let snapshot_id = Storage::compute_snapshot_id(&snapshot);
        let mut snapshot = snapshot;
        snapshot.id = snapshot_id.clone();
        storage.store_snapshot(&snapshot)?;

        Ok(snapshot_id)
    }

    /// 扫描工作区，构建内容寻址树
    ///
    /// 跳过 IGNORED_PATHS 中的目录/文件。
    fn scan_workspace(&self) -> Result<Tree> {
        let mut entries: HashMap<PathBuf, BlobId> = HashMap::new();
        self.walk_dir(&self.repo_path, &mut entries)?;

        let tree = Tree {
            id: String::new(),
            entries,
        };
        Ok(tree)
    }

    /// 递归遍历目录，收集文件 → Blob ID 映射
    fn walk_dir(&self, dir: &Path, entries: &mut HashMap<PathBuf, BlobId>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过忽略项
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if IGNORED_PATHS.contains(&name) {
                    continue;
                }
            }

            if path.is_dir() {
                self.walk_dir(&path, entries)?;
            } else if path.is_file() {
                let content = fs::read(&path)?;
                let blob_id = self.storage.store_blob(&content)?;
                // 存储相对路径
                let rel = path.strip_prefix(&self.repo_path).unwrap_or(&path).to_path_buf();
                entries.insert(rel, blob_id);
            }
        }
        Ok(())
    }

    /// 恢复工作区到指定 tree 的状态
    ///
    /// 1. 写入 tree 中的所有文件
    /// 2. 删除工作区中不在 tree 中的文件
    fn restore_workspace(&self, tree: &Tree) -> Result<()> {
        // 1. 写入 tree 中的所有文件
        for (rel_path, blob_id) in &tree.entries {
            let abs_path = self.repo_path.join(rel_path);
            if let Some(parent) = abs_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let blob = self.storage.read_blob(blob_id)?;
            fs::write(&abs_path, &blob.content)?;
        }

        // 2. 删除工作区中不在 tree 中的文件
        let current_files = self.collect_workspace_files()?;
        for file in current_files {
            if !tree.entries.contains_key(&file) {
                let abs_path = self.repo_path.join(&file);
                if abs_path.exists() {
                    let _ = fs::remove_file(&abs_path);
                }
            }
        }

        // 3. 清理空目录（可选）
        self.cleanup_empty_dirs(&self.repo_path)?;

        Ok(())
    }

    /// 收集工作区所有文件（相对路径）
    fn collect_workspace_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_files(&self.repo_path, &mut files)?;
        Ok(files)
    }

    fn collect_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if IGNORED_PATHS.contains(&name) {
                    continue;
                }
            }

            if path.is_dir() {
                self.collect_files(&path, files)?;
            } else if path.is_file() {
                let rel = path.strip_prefix(&self.repo_path).unwrap_or(&path).to_path_buf();
                files.push(rel);
            }
        }
        Ok(())
    }

    /// 清理空目录（递归）
    fn cleanup_empty_dirs(&self, dir: &Path) -> Result<()> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        // 跳过忽略项
        if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
            if IGNORED_PATHS.contains(&name) {
                return Ok(());
            }
        }

        // 先递归处理子目录
        let mut subdirs = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                subdirs.push(path);
            }
        }
        for subdir in subdirs {
            self.cleanup_empty_dirs(&subdir)?;
        }

        // 再检查当前目录是否为空
        let mut is_empty = true;
        if let Some(entry) = fs::read_dir(dir)?.next() {
            let _entry = entry?;
            is_empty = false;
        }

        if is_empty && dir != self.repo_path {
            let _ = fs::remove_dir(dir);
        }

        Ok(())
    }

    /// 获取当前分支的 HEAD 快照 ID
    fn current_head_snapshot(&self) -> Result<SnapshotId> {
        let branch_name = self.storage.read_head()?;
        let branch = self.storage.read_branch(&branch_name)?;
        Ok(branch.head)
    }

    /// 更新当前分支的 HEAD 指针
    fn update_current_head(&self, snapshot_id: &SnapshotId) -> Result<()> {
        let branch_name = self.storage.read_head()?;
        let mut branch = self.storage.read_branch(&branch_name)?;
        branch.head = snapshot_id.clone();
        self.storage.store_branch(&branch)?;
        Ok(())
    }

    /// 计算两个 Blob 内容的行级差异（additions/deletions）
    ///
    /// 简化实现：旧文件行数 = deletions，新文件行数 = additions。
    /// M3 阶段接入 AST-Native 后可升级为语义级 diff。
    fn compute_line_diff(old_content: &[u8], new_content: &[u8]) -> (u32, u32) {
        let old_lines = old_content.iter().filter(|&&b| b == b'\n').count() as u32
            + if old_content.is_empty() { 0 } else { 1 };
        let new_lines = new_content.iter().filter(|&&b| b == b'\n').count() as u32
            + if new_content.is_empty() { 0 } else { 1 };
        // 简化：新增行数 = 新文件行数，删除行数 = 旧文件行数
        // 真实 diff 算法（Myers）在 M3 接入
        (new_lines, old_lines)
    }
}

impl SnapshotOps for TimeFlow {
    fn snapshot(&self, message: Option<&str>) -> Result<SnapshotId> {
        // 1. 扫描工作区，构建 tree
        let tree = self.scan_workspace()?;
        let tree_id = self.storage.store_tree(&tree)?;
        let file_count = tree.entries.len();

        // 2. 获取当前 HEAD 作为 parent
        let parent = self.current_head_snapshot().ok();

        // 3. 创建快照
        let now = chrono::Utc::now();
        let snapshot = Snapshot {
            id: String::new(),
            parent,
            timestamp: now,
            tree: tree_id.clone(),
            author: "you".to_string(),
            metadata: SnapshotMetadata {
                trigger: TriggerType::Manual,
                ai_summary: None, // M2 阶段接入 AI 生成
                ai_commit_msg: message.map(|s| s.to_string()),
                build_status: BuildStatus::Unknown, // M5 阶段接入验证流水线
                snapshot_type: SnapshotType::Progress,
                tags: vec![],
            },
        };

        // 4. 计算快照 ID 并存储
        let snapshot_id = Storage::compute_snapshot_id(&snapshot);
        let mut snapshot = snapshot;
        snapshot.id = snapshot_id.clone();
        self.storage.store_snapshot(&snapshot)?;

        // 5. 更新当前分支 HEAD
        self.update_current_head(&snapshot_id)?;

        tracing::info!(
            "创建快照: {} (tree={}, files={})",
            snapshot_id,
            tree_id,
            file_count
        );

        Ok(snapshot_id)
    }

    fn rollback(&self, target: &SnapshotId) -> Result<SnapshotId> {
        // 1. 读取目标快照
        let target_snapshot = self.storage.read_snapshot(target)?;
        let target_tree = self.storage.read_tree(&target_snapshot.tree)?;

        // 2. 恢复工作区到目标状态
        self.restore_workspace(&target_tree)?;

        // 3. 创建新快照（保留历史，parent = 当前 HEAD）
        let parent = self.current_head_snapshot().ok();
        let now = chrono::Utc::now();
        let rollback_snapshot = Snapshot {
            id: String::new(),
            parent,
            timestamp: now,
            tree: target_snapshot.tree.clone(),
            author: "you".to_string(),
            metadata: SnapshotMetadata {
                trigger: TriggerType::Manual,
                ai_summary: Some(format!("回滚到快照 {}", &target[..8.min(target.len())])),
                ai_commit_msg: Some(format!("rollback: 回滚到 {}", &target[..8.min(target.len())])),
                build_status: BuildStatus::Unknown,
                snapshot_type: SnapshotType::Progress,
                tags: vec!["rollback".to_string()],
            },
        };

        let snapshot_id = Storage::compute_snapshot_id(&rollback_snapshot);
        let mut rollback_snapshot = rollback_snapshot;
        rollback_snapshot.id = snapshot_id.clone();
        self.storage.store_snapshot(&rollback_snapshot)?;

        // 4. 更新当前分支 HEAD
        self.update_current_head(&snapshot_id)?;

        tracing::info!("回滚到快照: {} (新快照 {})", target, snapshot_id);

        Ok(snapshot_id)
    }

    fn diff(&self, from: &SnapshotId, to: &SnapshotId) -> Result<SnapshotDiff> {
        let from_snapshot = self.storage.read_snapshot(from)?;
        let to_snapshot = self.storage.read_snapshot(to)?;
        let from_tree = self.storage.read_tree(&from_snapshot.tree)?;
        let to_tree = self.storage.read_tree(&to_snapshot.tree)?;

        let mut files = Vec::new();

        // 新增和修改的文件（在 to 中）
        for (path, to_blob_id) in &to_tree.entries {
            match from_tree.entries.get(path) {
                None => {
                    // 新增
                    let blob = self.storage.read_blob(to_blob_id)?;
                    let additions = blob.content.iter().filter(|&&b| b == b'\n').count() as u32
                        + if blob.content.is_empty() { 0 } else { 1 };
                    files.push(FileChange {
                        path: path.clone(),
                        status: ChangeStatus::Added,
                        additions,
                        deletions: 0,
                    });
                }
                Some(from_blob_id) if from_blob_id != to_blob_id => {
                    // 修改
                    let old_blob = self.storage.read_blob(from_blob_id)?;
                    let new_blob = self.storage.read_blob(to_blob_id)?;
                    let (additions, deletions) =
                        Self::compute_line_diff(&old_blob.content, &new_blob.content);
                    files.push(FileChange {
                        path: path.clone(),
                        status: ChangeStatus::Modified,
                        additions,
                        deletions,
                    });
                }
                _ => {} // 未变更
            }
        }

        // 删除的文件（在 from 中但不在 to 中）
        for (path, from_blob_id) in &from_tree.entries {
            if !to_tree.entries.contains_key(path) {
                let blob = self.storage.read_blob(from_blob_id)?;
                let deletions = blob.content.iter().filter(|&&b| b == b'\n').count() as u32
                    + if blob.content.is_empty() { 0 } else { 1 };
                files.push(FileChange {
                    path: path.clone(),
                    status: ChangeStatus::Deleted,
                    additions: 0,
                    deletions,
                });
            }
        }

        Ok(SnapshotDiff {
            from: from.clone(),
            to: to.clone(),
            files,
        })
    }

    fn get_snapshot(&self, id: &SnapshotId) -> Result<Snapshot> {
        self.storage.read_snapshot(id)
    }

    fn list_snapshots(&self, filter: Option<SnapshotFilter>) -> Result<Vec<Snapshot>> {
        let mut snapshots = self.storage.list_snapshots()?;

        if let Some(f) = filter {
            if let Some(st) = f.snapshot_type {
                snapshots.retain(|s| s.metadata.snapshot_type == st);
            }
            if let Some(bs) = f.build_status {
                snapshots.retain(|s| s.metadata.build_status == bs);
            }
            if let Some(limit) = f.limit {
                snapshots.truncate(limit);
            }
        }

        Ok(snapshots)
    }
}

impl BranchOps for TimeFlow {
    fn create_branch(&self, name: &BranchName, from: &SnapshotId) -> Result<Branch> {
        // 验证源快照存在
        let _ = self.storage.read_snapshot(from)?;

        // 检查分支是否已存在
        if self.storage.read_branch(name).is_ok() {
            return Err(Error::Other(format!("分支已存在: {}", name)));
        }

        let branch = Branch {
            name: name.clone(),
            head: from.clone(),
            created_at: chrono::Utc::now(),
        };
        self.storage.store_branch(&branch)?;

        tracing::info!("创建分支: {} (from={})", name, from);
        Ok(branch)
    }

    fn delete_branch(&self, name: &BranchName) -> Result<()> {
        // 不允许删除当前分支
        let current = self.storage.read_head()?;
        if current == *name {
            return Err(Error::Other("不能删除当前所在分支".to_string()));
        }
        self.storage.delete_branch(name)?;
        tracing::info!("删除分支: {}", name);
        Ok(())
    }

    fn switch_branch(&self, name: &BranchName) -> Result<()> {
        // 不允许切换到当前分支
        let current = self.storage.read_head()?;
        if current == *name {
            return Err(Error::Other(format!("已在分支 {} 上", name)));
        }

        // 读取目标分支
        let branch = self.storage.read_branch(name)?;
        let snapshot = self.storage.read_snapshot(&branch.head)?;
        let tree = self.storage.read_tree(&snapshot.tree)?;

        // 恢复工作区到目标分支状态
        self.restore_workspace(&tree)?;

        // 更新 HEAD
        self.storage.write_head(name)?;

        tracing::info!("切换到分支: {}", name);
        Ok(())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        self.storage.list_branches()
    }

    fn current_branch(&self) -> Result<BranchName> {
        self.storage.read_head()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 测试辅助：在仓库中写入文件
    fn write_file(repo: &Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// 测试辅助：读取仓库中的文件
    fn read_file(repo: &Path, rel: &str) -> Option<String> {
        fs::read_to_string(repo.join(rel)).ok()
    }

    #[test]
    fn test_init_creates_main_branch() {
        let tmp = TempDir::new().unwrap();
        let tf = TimeFlow::new(tmp.path()).unwrap();
        assert_eq!(tf.current_branch().unwrap(), "main");

        let branches = tf.list_branches().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn test_snapshot_basic() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        // 准备工作区
        write_file(repo, "src/main.rs", "fn main() {}");
        write_file(repo, "README.md", "# Test");

        let tf = TimeFlow::new(repo).unwrap();

        // 创建快照
        let snap_id = tf.snapshot(Some("初始快照")).unwrap();
        assert!(!snap_id.is_empty());

        // 验证快照存在
        let snap = tf.get_snapshot(&snap_id).unwrap();
        assert_eq!(snap.author, "you");
        assert_eq!(snap.metadata.ai_commit_msg, Some("初始快照".to_string()));

        // 验证 HEAD 已更新
        let branches = tf.list_branches().unwrap();
        assert_eq!(branches[0].head, snap_id);
    }

    #[test]
    fn test_snapshot_chain() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "v1");
        let tf = TimeFlow::new(repo).unwrap();

        let snap1 = tf.snapshot(Some("v1")).unwrap();

        // 修改文件
        write_file(repo, "a.txt", "v2");
        let snap2 = tf.snapshot(Some("v2")).unwrap();

        // 验证快照链
        let s2 = tf.get_snapshot(&snap2).unwrap();
        assert_eq!(s2.parent, Some(snap1.clone()));

        let s1 = tf.get_snapshot(&snap1).unwrap();
        // snap1 的 parent 应该是初始快照
        assert!(s1.parent.is_some());
    }

    #[test]
    fn test_rollback_preserves_history() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "v1");
        let tf = TimeFlow::new(repo).unwrap();

        let snap1 = tf.snapshot(Some("v1")).unwrap();
        assert_eq!(read_file(repo, "a.txt"), Some("v1".to_string()));

        // 修改并快照
        write_file(repo, "a.txt", "v2");
        let snap2 = tf.snapshot(Some("v2")).unwrap();
        assert_eq!(read_file(repo, "a.txt"), Some("v2".to_string()));

        // 回滚到 snap1
        let rollback_snap = tf.rollback(&snap1).unwrap();
        assert_eq!(read_file(repo, "a.txt"), Some("v1".to_string()));

        // 验证历史保留：snap2 仍然存在
        let _ = tf.get_snapshot(&snap2).unwrap();

        // 验证回滚创建的新快照 parent 是 snap2
        let rb = tf.get_snapshot(&rollback_snap).unwrap();
        assert_eq!(rb.parent, Some(snap2));
    }

    #[test]
    fn test_diff() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "line1\nline2\n");
        write_file(repo, "b.txt", "b1\n");
        let tf = TimeFlow::new(repo).unwrap();
        let snap1 = tf.snapshot(Some("v1")).unwrap();

        // 修改 a.txt，删除 b.txt，新增 c.txt
        write_file(repo, "a.txt", "line1\nline2\nline3\n");
        fs::remove_file(repo.join("b.txt")).unwrap();
        write_file(repo, "c.txt", "c1\nc2\n");
        let snap2 = tf.snapshot(Some("v2")).unwrap();

        let diff = tf.diff(&snap1, &snap2).unwrap();
        assert_eq!(diff.from, snap1);
        assert_eq!(diff.to, snap2);

        let mut has_modified = false;
        let mut has_deleted = false;
        let mut has_added = false;
        for change in &diff.files {
            match change.status {
                ChangeStatus::Modified => {
                    assert_eq!(change.path, PathBuf::from("a.txt"));
                    has_modified = true;
                }
                ChangeStatus::Deleted => {
                    assert_eq!(change.path, PathBuf::from("b.txt"));
                    has_deleted = true;
                }
                ChangeStatus::Added => {
                    assert_eq!(change.path, PathBuf::from("c.txt"));
                    has_added = true;
                }
            }
        }
        assert!(has_modified, "应有修改文件");
        assert!(has_deleted, "应有删除文件");
        assert!(has_added, "应有新增文件");
    }

    #[test]
    fn test_branch_create_and_switch() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "main v1");
        let tf = TimeFlow::new(repo).unwrap();
        let snap1 = tf.snapshot(Some("main v1")).unwrap();

        // 创建 feature 分支
        let _branch = tf.create_branch(&"feature".to_string(), &snap1).unwrap();
        assert_eq!(tf.list_branches().unwrap().len(), 2);

        // 切换到 feature 分支
        tf.switch_branch(&"feature".to_string()).unwrap();
        assert_eq!(tf.current_branch().unwrap(), "feature");

        // 在 feature 分支上修改
        write_file(repo, "a.txt", "feature v1");
        let snap2 = tf.snapshot(Some("feature v1")).unwrap();

        // 切换回 main
        tf.switch_branch(&"main".to_string()).unwrap();
        assert_eq!(read_file(repo, "a.txt"), Some("main v1".to_string()));

        // 切换回 feature
        tf.switch_branch(&"feature".to_string()).unwrap();
        assert_eq!(read_file(repo, "a.txt"), Some("feature v1".to_string()));

        // 验证 feature 分支 HEAD 是 snap2
        let branches = tf.list_branches().unwrap();
        let feature_branch = branches.iter().find(|b| b.name == "feature").unwrap();
        assert_eq!(feature_branch.head, snap2);
    }

    #[test]
    fn test_delete_branch() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "v1");
        let tf = TimeFlow::new(repo).unwrap();
        let snap1 = tf.snapshot(Some("v1")).unwrap();

        tf.create_branch(&"temp".to_string(), &snap1).unwrap();
        assert_eq!(tf.list_branches().unwrap().len(), 2);

        tf.delete_branch(&"temp".to_string()).unwrap();
        assert_eq!(tf.list_branches().unwrap().len(), 1);

        // 不能删除当前分支
        let result = tf.delete_branch(&"main".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_snapshots_with_filter() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "a.txt", "v1");
        let tf = TimeFlow::new(repo).unwrap();
        let _ = tf.snapshot(Some("v1")).unwrap();
        let _ = tf.snapshot(Some("v2")).unwrap();
        let _ = tf.snapshot(Some("v3")).unwrap();

        // 全部快照（含初始快照 = 4）
        let all = tf.list_snapshots(None).unwrap();
        assert!(all.len() >= 4);

        // 限制数量
        let limited = tf
            .list_snapshots(Some(SnapshotFilter {
                limit: Some(2),
                ..Default::default()
            }))
            .unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_ignored_paths() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        write_file(repo, "src/main.rs", "fn main() {}");
        write_file(repo, "node_modules/pkg/index.js", "module.exports = {}");
        write_file(repo, ".yunji/timeflow/config.toml", "ignored");

        let tf = TimeFlow::new(repo).unwrap();
        let snap_id = tf.snapshot(Some("v1")).unwrap();
        let snap = tf.get_snapshot(&snap_id).unwrap();
        let tree = tf.storage().read_tree(&snap.tree).unwrap();

        // 只有 src/main.rs 应该被纳入
        assert_eq!(tree.entries.len(), 1);
        assert!(tree.entries.contains_key(&PathBuf::from("src/main.rs")));
        assert!(!tree.entries.contains_key(&PathBuf::from("node_modules/pkg/index.js")));
    }

    #[test]
    fn test_full_workflow() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        // 1. 初始化 + 创建文件 + 快照
        write_file(repo, "src/main.rs", "fn main() {\n    println!(\"hello\");\n}\n");
        write_file(repo, "Cargo.toml", "[package]\nname = \"test\"\n");
        let tf = TimeFlow::new(repo).unwrap();
        let snap1 = tf.snapshot(Some("初始版本")).unwrap();

        // 2. 修改 + 快照
        write_file(repo, "src/main.rs", "fn main() {\n    println!(\"hello world\");\n}\n");
        write_file(repo, "src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n");
        let snap2 = tf.snapshot(Some("添加 lib")).unwrap();

        // 3. 验证 diff
        let diff = tf.diff(&snap1, &snap2).unwrap();
        assert!(diff.files.iter().any(|f| f.path == PathBuf::from("src/main.rs")));
        assert!(diff.files.iter().any(|f| f.path == PathBuf::from("src/lib.rs")));

        // 4. 创建分支
        tf.create_branch(&"dev".to_string(), &snap1).unwrap();

        // 5. 回滚到 snap1
        tf.rollback(&snap1).unwrap();
        assert_eq!(
            read_file(repo, "src/main.rs"),
            Some("fn main() {\n    println!(\"hello\");\n}\n".to_string())
        );
        // src/lib.rs 应该被删除（不在 snap1 中）
        assert!(read_file(repo, "src/lib.rs").is_none());

        // 6. 切换到 dev 分支
        tf.switch_branch(&"dev".to_string()).unwrap();
        assert_eq!(
            read_file(repo, "src/main.rs"),
            Some("fn main() {\n    println!(\"hello\");\n}\n".to_string())
        );

        // 7. 切换回 main
        tf.switch_branch(&"main".to_string()).unwrap();

        // 8. 列出所有快照
        let snapshots = tf.list_snapshots(None).unwrap();
        assert!(snapshots.len() >= 3); // 初始 + snap1 + snap2 + 回滚快照
    }
}
