// TimeFlow 文件监控器
// 设计文档: docs/TIMEFLOW-DESIGN.md § 3.5
//
// W3 实现: 基于 notify + 自实现防抖策略的自动快照触发
//
// 工作流程:
// 1. 监控仓库目录的文件变更（notify::RecommendedWatcher 跨平台）
// 2. 防抖：变更后等待 N 秒无新变更才触发
// 3. 触发自动快照（WIP 类型）
// 4. 通过回调通知上层（如 Tauri 事件系统）

use crate::error::{Error, Result};
use crate::snapshot::SnapshotOps;
use crate::TimeFlow;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 默认防抖间隔（秒）
pub const DEFAULT_DEBOUNCE_SECS: u64 = 5;

/// 默认忽略的路径（与 TimeFlow::IGNORED_PATHS 保持一致）
const WATCHER_IGNORED_PATHS: &[&str] = &[
    ".yunji",
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
];

/// 文件监控器
///
/// 监控仓库目录变更，防抖后触发自动快照。
pub struct Watcher {
    /// 内部 notify 监控器
    _watcher: RecommendedWatcher,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 仓库路径
    repo_path: PathBuf,
}

/// 监控器配置
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// 防抖间隔（秒）
    pub debounce_secs: u64,
    /// 自动快照消息前缀
    pub message_prefix: String,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_secs: DEFAULT_DEBOUNCE_SECS,
            message_prefix: "自动快照".to_string(),
        }
    }
}

impl Watcher {
    /// 启动文件监控
    ///
    /// - `timeflow`: TimeFlow 引擎（用于创建快照）
    /// - `config`: 监控配置
    ///
    /// 监控器在后台线程运行，文件变更防抖后自动调用 `timeflow.snapshot()`。
    /// 返回的 Watcher 持有 _watcher，drop 时自动停止监控。
    pub fn start(timeflow: Arc<TimeFlow>, config: WatcherConfig) -> Result<Self> {
        let repo_path = timeflow.repo_path().to_path_buf();
        let running = Arc::new(AtomicBool::new(true));

        // 共享的"最后变更时间"，用于防抖
        let last_change: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let pending_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

        // 创建事件通道：使用 std::sync::mpsc 传递变更事件
        let (tx, rx) = std::sync::mpsc::channel();

        // 创建 notify watcher（事件发送到通道）
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                // 只关注文件修改/创建/删除事件
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = tx.send(event);
                }
            }
        })
        .map_err(|e| Error::Other(format!("创建监控器失败: {}", e)))?;

        // 开始监控仓库目录（递归）
        watcher
            .watch(&repo_path, RecursiveMode::Recursive)
            .map_err(|e| Error::Other(format!("启动监控失败: {}", e)))?;

        // 启动防抖处理线程
        let running_clone = running.clone();
        let message_prefix = config.message_prefix.clone();
        let debounce_duration = Duration::from_secs(config.debounce_secs);
        let tf = timeflow.clone();
        let last_change_clone = last_change.clone();
        let pending_count_clone = pending_count.clone();

        thread::spawn(move || {
            tracing::debug!("防抖处理线程已启动");

            loop {
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                // 非阻塞接收事件
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(event) => {
                        // 过滤忽略路径
                        let relevant: Vec<_> = event
                            .paths
                            .into_iter()
                            .filter(|p| !Self::is_ignored(p))
                            .collect();

                        if relevant.is_empty() {
                            continue;
                        }

                        // 更新最后变更时间 + 累计变更数
                        {
                            let mut lc = last_change_clone.lock().unwrap();
                            *lc = Some(Instant::now());
                        }
                        {
                            let mut pc = pending_count_clone.lock().unwrap();
                            *pc += relevant.len();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // 检查是否应该触发快照
                        let should_snapshot = {
                            let lc = last_change_clone.lock().unwrap();
                            if let Some(last) = *lc {
                                last.elapsed() >= debounce_duration
                            } else {
                                false
                            }
                        };

                        if should_snapshot {
                            // 获取累计变更数并重置
                            let count = {
                                let mut pc = pending_count_clone.lock().unwrap();
                                let c = *pc;
                                *pc = 0;
                                c
                            };
                            // 重置最后变更时间
                            {
                                let mut lc = last_change_clone.lock().unwrap();
                                *lc = None;
                            }

                            if count > 0 {
                                tracing::info!(
                                    "防抖结束，检测到 {} 个文件变更，触发自动快照",
                                    count
                                );

                                let message =
                                    format!("{} ({} 个文件变更)", message_prefix, count);
                                match tf.snapshot(Some(&message)) {
                                    Ok(snapshot_id) => {
                                        tracing::info!("自动快照创建成功: {}", snapshot_id);
                                    }
                                    Err(e) => {
                                        tracing::error!("自动快照创建失败: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        tracing::debug!("事件通道已关闭，防抖线程退出");
                        break;
                    }
                }
            }

            tracing::debug!("防抖处理线程已退出");
        });

        tracing::info!(
            "文件监控器已启动: {} (防抖={}s)",
            repo_path.display(),
            config.debounce_secs
        );

        Ok(Self {
            _watcher: watcher,
            running,
            repo_path,
        })
    }

    /// 停止监控
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("文件监控器已停止: {}", self.repo_path.display());
    }

    /// 判断路径是否应被忽略
    fn is_ignored(path: &Path) -> bool {
        // 检查路径的每一级是否包含忽略项
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    if WATCHER_IGNORED_PATHS.contains(&name_str) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 获取监控的仓库路径
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// 监控器是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_is_ignored() {
        assert!(Watcher::is_ignored(Path::new(".yunji/timeflow/HEAD")));
        assert!(Watcher::is_ignored(Path::new(".git/HEAD")));
        assert!(Watcher::is_ignored(Path::new("node_modules/pkg/index.js")));
        assert!(Watcher::is_ignored(Path::new("target/debug/app")));
        assert!(Watcher::is_ignored(Path::new("project/target/debug/app")));

        assert!(!Watcher::is_ignored(Path::new("src/main.rs")));
        assert!(!Watcher::is_ignored(Path::new("README.md")));
        assert!(!Watcher::is_ignored(Path::new("src/components/Button.tsx")));
    }

    #[test]
    fn test_watcher_auto_snapshot() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        // 准备初始文件
        fs::write(repo.join("a.txt"), "initial").unwrap();

        // 初始化 TimeFlow
        let timeflow = Arc::new(TimeFlow::new(repo).unwrap());

        // 创建一个初始快照（建立基线）
        timeflow.snapshot(Some("初始快照")).unwrap();
        let initial_count = timeflow.list_snapshots(None).unwrap().len();

        // 启动监控器（1 秒防抖，加快测试）
        let config = WatcherConfig {
            debounce_secs: 1,
            message_prefix: "测试自动快照".to_string(),
        };
        let watcher = Watcher::start(timeflow.clone(), config).unwrap();
        assert!(watcher.is_running());

        // 修改文件触发变更
        fs::write(repo.join("a.txt"), "modified content").unwrap();
        fs::write(repo.join("b.txt"), "new file content").unwrap();

        // 等待防抖触发 + 快照创建（防抖 1s + 处理时间 2s）
        thread::sleep(Duration::from_secs(4));

        // 停止监控
        watcher.stop();

        // 验证自动快照已创建
        let snapshots = timeflow.list_snapshots(None).unwrap();
        assert!(
            snapshots.len() > initial_count,
            "应有新快照创建，初始: {}，最终: {}",
            initial_count,
            snapshots.len()
        );

        // 最新的快照应该是自动快照
        let latest = &snapshots[0];
        assert!(
            latest
                .metadata
                .ai_commit_msg
                .as_ref()
                .unwrap()
                .contains("测试自动快照"),
            "最新快照应为自动快照，实际消息: {:?}",
            latest.metadata.ai_commit_msg
        );
    }

    #[test]
    fn test_watcher_ignored_paths() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        // 准备初始文件
        fs::write(repo.join("a.txt"), "initial").unwrap();

        // 初始化 TimeFlow
        let timeflow = Arc::new(TimeFlow::new(repo).unwrap());
        timeflow.snapshot(Some("初始快照")).unwrap();
        let initial_count = timeflow.list_snapshots(None).unwrap().len();

        // 启动监控器
        let config = WatcherConfig {
            debounce_secs: 1,
            message_prefix: "测试自动快照".to_string(),
        };
        let watcher = Watcher::start(timeflow.clone(), config).unwrap();

        // 修改 .yunji 目录下的文件（应被忽略）
        // 注意：.yunji 目录在 TimeFlow::new 时已创建
        let yunji_dir = repo.join(".yunji").join("timeflow");
        fs::write(yunji_dir.join("test.txt"), "ignored change").unwrap();

        // 等待防抖
        thread::sleep(Duration::from_secs(3));

        watcher.stop();

        // 不应创建新快照（因为变更都在忽略路径下）
        let final_count = timeflow.list_snapshots(None).unwrap().len();
        assert_eq!(
            final_count, initial_count,
            "忽略路径的变更不应触发快照，初始: {}，最终: {}",
            initial_count, final_count
        );
    }

    #[test]
    fn test_watcher_stop() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();
        fs::write(repo.join("a.txt"), "initial").unwrap();

        let timeflow = Arc::new(TimeFlow::new(repo).unwrap());
        let config = WatcherConfig::default();
        let watcher = Watcher::start(timeflow, config).unwrap();

        assert!(watcher.is_running());
        watcher.stop();
        // 给线程一点时间退出
        thread::sleep(Duration::from_millis(100));
        assert!(!watcher.is_running());
    }
}
