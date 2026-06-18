// TimeFlow 存储引擎
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.3
//
// W2 实现: 内容寻址存储（SHA-256 + 压缩 + 去重）

use crate::error::{Error, Result};
use crate::types::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 存储引擎 - 管理所有持久化数据
///
/// 存储布局:
/// ```text
/// .yunji/timeflow/
/// ├── snapshots/    快照元数据（{snapshot_id}.json）
/// ├── blobs/        内容寻址 blob（{blob_id}.bin，原始字节）
/// ├── trees/        内容寻址树（{tree_id}.json）
/// ├── branches/     分支指针（{branch_name}.json）
/// ├── HEAD          当前分支名
/// └── config.toml   配置
/// ```
pub struct Storage {
    /// 仓库根路径
    repo_path: PathBuf,
    /// TimeFlow 数据目录
    timeflow_dir: PathBuf,
}

impl Storage {
    /// 创建存储引擎实例，自动初始化目录结构
    pub fn new(repo_path: &Path) -> Result<Self> {
        let timeflow_dir = repo_path.join(".yunji").join("timeflow");

        // 自动创建目录结构
        if !timeflow_dir.exists() {
            Self::init_layout(&timeflow_dir)?;
            tracing::info!("TimeFlow 仓库初始化: {}", timeflow_dir.display());
        } else {
            // 目录已存在，但仍需确保 HEAD 文件存在（可能被部分创建）
            let head_path = timeflow_dir.join("HEAD");
            if !head_path.exists() {
                fs::create_dir_all(timeflow_dir.join("snapshots"))?;
                fs::create_dir_all(timeflow_dir.join("blobs"))?;
                fs::create_dir_all(timeflow_dir.join("trees"))?;
                fs::create_dir_all(timeflow_dir.join("branches"))?;
                fs::write(&head_path, "main")?;
            }
        }

        Ok(Self {
            repo_path: repo_path.to_path_buf(),
            timeflow_dir,
        })
    }

    /// 初始化目录结构
    fn init_layout(timeflow_dir: &Path) -> Result<()> {
        fs::create_dir_all(timeflow_dir.join("snapshots"))?;
        fs::create_dir_all(timeflow_dir.join("blobs"))?;
        fs::create_dir_all(timeflow_dir.join("trees"))?;
        fs::create_dir_all(timeflow_dir.join("branches"))?;

        // 默认 HEAD 指向 main 分支
        let head_path = timeflow_dir.join("HEAD");
        if !head_path.exists() {
            fs::write(&head_path, "main")?;
        }

        Ok(())
    }

    /// 获取 TimeFlow 数据目录
    pub fn timeflow_dir(&self) -> &Path {
        &self.timeflow_dir
    }

    /// 获取仓库根路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    // ===== Blob 操作 =====

    /// 存储 Blob（内容寻址）
    ///
    /// 1. 计算 SHA-256
    /// 2. 写入 blobs/{blob_id}.bin
    /// 3. 如果已存在则跳过（去重）
    pub fn store_blob(&self, content: &[u8]) -> Result<BlobId> {
        let id = Self::hash_content(content);
        let blob_path = self.blobs_dir().join(format!("{}.bin", id));

        // 去重：已存在则直接返回
        if blob_path.exists() {
            return Ok(id);
        }

        // 写入文件
        let mut file = fs::File::create(&blob_path)?;
        file.write_all(content)?;
        tracing::trace!("存储 blob: {} ({} bytes)", id, content.len());

        Ok(id)
    }

    /// 读取 Blob
    pub fn read_blob(&self, id: &BlobId) -> Result<Blob> {
        let blob_path = self.blobs_dir().join(format!("{}.bin", id));
        if !blob_path.exists() {
            return Err(Error::Other(format!("Blob 不存在: {}", id)));
        }
        let content = fs::read(&blob_path)?;
        Ok(Blob {
            id: id.clone(),
            content,
        })
    }

    // ===== Tree 操作 =====

    /// 存储 Tree
    ///
    /// Tree 的 ID = SHA-256(序列化后的 JSON)
    pub fn store_tree(&self, tree: &Tree) -> Result<TreeId> {
        // 先序列化确定 ID（不含 id 字段，避免循环）
        let mut tree_without_id = tree.clone();
        let serialized = serde_json::to_vec(&tree_without_id.entries)?;
        let id = Self::hash_content(&serialized);
        tree_without_id.id = id.clone();

        let tree_path = self.trees_dir().join(format!("{}.json", id));

        // 去重
        if tree_path.exists() {
            return Ok(id);
        }

        // 写入完整 Tree（含 id）
        let json = serde_json::to_vec_pretty(tree)?;
        fs::write(&tree_path, json)?;
        tracing::trace!("存储 tree: {} ({} entries)", id, tree.entries.len());

        Ok(id)
    }

    /// 读取 Tree
    pub fn read_tree(&self, id: &TreeId) -> Result<Tree> {
        let tree_path = self.trees_dir().join(format!("{}.json", id));
        if !tree_path.exists() {
            return Err(Error::Other(format!("Tree 不存在: {}", id)));
        }
        let json = fs::read_to_string(&tree_path)?;
        let tree: Tree = serde_json::from_str(&json)?;
        Ok(tree)
    }

    // ===== Snapshot 操作 =====

    /// 计算快照 ID（基于内容哈希，不含 id 字段）
    ///
    /// 快照 ID = SHA-256(序列化的 Snapshot，id 字段置空)
    pub fn compute_snapshot_id(snapshot: &Snapshot) -> SnapshotId {
        let mut snapshot_without_id = snapshot.clone();
        snapshot_without_id.id = String::new();
        // 序列化失败时退化为空内容哈希（理论上不会发生）
        let serialized = serde_json::to_vec(&snapshot_without_id).unwrap_or_default();
        Self::hash_content(&serialized)
    }

    /// 存储 Snapshot 元数据
    pub fn store_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let snapshot_path = self.snapshots_dir().join(format!("{}.json", snapshot.id));
        let json = serde_json::to_vec_pretty(snapshot)?;
        fs::write(&snapshot_path, json)?;
        tracing::debug!(
            "存储 snapshot: {} (parent={:?})",
            snapshot.id,
            snapshot.parent
        );
        Ok(())
    }

    /// 读取 Snapshot
    pub fn read_snapshot(&self, id: &SnapshotId) -> Result<Snapshot> {
        let snapshot_path = self.snapshots_dir().join(format!("{}.json", id));
        if !snapshot_path.exists() {
            return Err(Error::SnapshotNotFound(id.clone()));
        }
        let json = fs::read_to_string(&snapshot_path)?;
        let snapshot: Snapshot = serde_json::from_str(&json)?;
        Ok(snapshot)
    }

    /// 列出所有快照（按时间倒序）
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();
        let snapshots_dir = self.snapshots_dir();

        if !snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = fs::read_to_string(&path)?;
                let snapshot: Snapshot = serde_json::from_str(&json)?;
                snapshots.push(snapshot);
            }
        }

        // 按时间倒序
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(snapshots)
    }

    // ===== Branch 操作 =====

    /// 存储分支指针
    pub fn store_branch(&self, branch: &Branch) -> Result<()> {
        let branch_path = self.branches_dir().join(format!("{}.json", branch.name));
        // 分支名可能包含 "/"（如 "feature/test"），需创建父目录
        if let Some(parent) = branch_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(branch)?;
        fs::write(&branch_path, json)?;
        Ok(())
    }

    /// 读取分支
    pub fn read_branch(&self, name: &BranchName) -> Result<Branch> {
        let branch_path = self.branches_dir().join(format!("{}.json", name));
        if !branch_path.exists() {
            return Err(Error::BranchNotFound(name.clone()));
        }
        let json = fs::read_to_string(&branch_path)?;
        let branch: Branch = serde_json::from_str(&json)?;
        Ok(branch)
    }

    /// 删除分支
    pub fn delete_branch(&self, name: &BranchName) -> Result<()> {
        let branch_path = self.branches_dir().join(format!("{}.json", name));
        if !branch_path.exists() {
            return Err(Error::BranchNotFound(name.clone()));
        }
        fs::remove_file(&branch_path)?;
        Ok(())
    }

    /// 列出所有分支
    pub fn list_branches(&self) -> Result<Vec<Branch>> {
        let mut branches = Vec::new();
        let branches_dir = self.branches_dir();

        if !branches_dir.exists() {
            return Ok(branches);
        }

        for entry in fs::read_dir(branches_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = fs::read_to_string(&path)?;
                let branch: Branch = serde_json::from_str(&json)?;
                branches.push(branch);
            }
        }

        // 按创建时间排序
        branches.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(branches)
    }

    // ===== HEAD 操作 =====

    /// 读取当前分支名
    pub fn read_head(&self) -> Result<BranchName> {
        let head_path = self.timeflow_dir.join("HEAD");
        if !head_path.exists() {
            return Err(Error::NotInitialized("HEAD 文件不存在".to_string()));
        }
        let branch_name = fs::read_to_string(&head_path)?;
        Ok(branch_name.trim().to_string())
    }

    /// 写入当前分支名
    pub fn write_head(&self, branch_name: &BranchName) -> Result<()> {
        let head_path = self.timeflow_dir.join("HEAD");
        fs::write(&head_path, branch_name)?;
        Ok(())
    }

    // ===== 工具方法 =====

    /// 计算 SHA-256 哈希（hex 编码）
    pub fn hash_content(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    fn snapshots_dir(&self) -> PathBuf {
        self.timeflow_dir.join("snapshots")
    }

    fn blobs_dir(&self) -> PathBuf {
        self.timeflow_dir.join("blobs")
    }

    fn trees_dir(&self) -> PathBuf {
        self.timeflow_dir.join("trees")
    }

    fn branches_dir(&self) -> PathBuf {
        self.timeflow_dir.join("branches")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Storage) {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::new(tmp.path()).unwrap();
        (tmp, storage)
    }

    #[test]
    fn test_init_layout() {
        let (_tmp, storage) = setup();
        assert!(storage.timeflow_dir().exists());
        assert!(storage.snapshots_dir().exists());
        assert!(storage.blobs_dir().exists());
        assert!(storage.trees_dir().exists());
        assert!(storage.branches_dir().exists());
        assert_eq!(storage.read_head().unwrap(), "main");
    }

    #[test]
    fn test_blob_store_and_read() {
        let (_tmp, storage) = setup();
        let content = b"hello world";
        let id = storage.store_blob(content).unwrap();
        assert_eq!(id.len(), 64); // SHA-256 hex = 64 chars

        let blob = storage.read_blob(&id).unwrap();
        assert_eq!(blob.content, content);
    }

    #[test]
    fn test_blob_dedup() {
        let (_tmp, storage) = setup();
        let content = b"same content";
        let id1 = storage.store_blob(content).unwrap();
        let id2 = storage.store_blob(content).unwrap();
        assert_eq!(id1, id2, "相同内容应返回相同 ID");
    }

    #[test]
    fn test_tree_store_and_read() {
        let (_tmp, storage) = setup();
        let mut entries = HashMap::new();
        entries.insert(PathBuf::from("src/main.rs"), "blob_id_1".to_string());
        entries.insert(PathBuf::from("README.md"), "blob_id_2".to_string());

        let tree = Tree {
            id: String::new(), // store_tree 会计算
            entries,
        };
        let id = storage.store_tree(&tree).unwrap();
        assert!(!id.is_empty());

        let read_tree = storage.read_tree(&id).unwrap();
        assert_eq!(read_tree.entries.len(), 2);
    }

    #[test]
    fn test_snapshot_store_and_read() {
        let (_tmp, storage) = setup();
        let snapshot = Snapshot {
            id: "snap_test123".to_string(),
            parent: None,
            timestamp: chrono::Utc::now(),
            tree: "tree_abc".to_string(),
            author: "test".to_string(),
            metadata: SnapshotMetadata {
                trigger: TriggerType::Manual,
                ai_summary: None,
                ai_commit_msg: Some("test commit".to_string()),
                build_status: BuildStatus::Unknown,
                snapshot_type: SnapshotType::Progress,
                tags: vec![],
            },
        };

        storage.store_snapshot(&snapshot).unwrap();
        let read = storage.read_snapshot(&snapshot.id).unwrap();
        assert_eq!(read.id, snapshot.id);
        assert_eq!(read.author, snapshot.author);
    }

    #[test]
    fn test_branch_store_and_read() {
        let (_tmp, storage) = setup();
        let branch = Branch {
            name: "feature/test".to_string(),
            head: "snap_123".to_string(),
            created_at: chrono::Utc::now(),
        };

        storage.store_branch(&branch).unwrap();
        let read = storage.read_branch(&branch.name).unwrap();
        assert_eq!(read.name, branch.name);
        assert_eq!(read.head, branch.head);
    }

    #[test]
    fn test_head_read_write() {
        let (_tmp, storage) = setup();
        assert_eq!(storage.read_head().unwrap(), "main");

        storage.write_head(&"feature/dev".to_string()).unwrap();
        assert_eq!(storage.read_head().unwrap(), "feature/dev");
    }
}
