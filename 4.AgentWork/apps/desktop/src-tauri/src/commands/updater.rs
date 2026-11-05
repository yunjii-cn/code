// 版本更新命令（移植自云集智能视频创意站）
// 多源竞速检查 + 应用内下载 + 硬链接切换
//
// 更新源：GitHub 镜像 / GitHub / Gitee
// 版本列表来自远程 versions.json
// 下载 EXE 到 ver/ 目录，通过硬链接切换入口

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

use crate::AppResult;

/// 更新源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    /// 自动竞速（同时请求 3 个源，谁先返回用谁）
    Auto,
    /// GitHub 镜像（国内访问）
    GithubMirror,
    /// GitHub 直连
    Github,
    /// Gitee
    Gitee,
}

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本号（如 2026.06.24.1030）
    pub version: String,
    /// 发布日期
    pub date: String,
    /// 版本摘要
    pub message: String,
    /// 变更列表
    #[serde(default)]
    pub changes: Vec<String>,
    /// EXE 文件名
    pub name: String,
    /// 下载 URL
    #[serde(default)]
    pub download_url: Option<String>,
    /// 产物 SHA-256（小写 64 位十六进制），缺失时拒绝下载
    #[serde(default)]
    pub sha256: Option<String>,
    /// 文件大小（字节）
    #[serde(default)]
    pub size: Option<u64>,
}

/// 版本检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    /// 当前版本
    pub current_version: String,
    /// 最新版本
    pub latest_version: Option<String>,
    /// 是否有更新
    pub has_update: bool,
    /// 所有版本列表
    pub versions: Vec<VersionInfo>,
    /// 使用的更新源
    pub source: String,
    /// 本地已下载的版本列表
    pub local_versions: Vec<String>,
}

/// 更新源配置
struct SourceConfig {
    name: &'static str,
    versions_url: &'static str,
}

/// 3 个更新源
const SOURCES: &[SourceConfig] = &[
    SourceConfig {
        name: "github_mirror",
        versions_url: "https://ghgo.xyz/https://raw.githubusercontent.com/yunjii-cn/aw/main/versions.json",
    },
    SourceConfig {
        name: "github",
        versions_url: "https://raw.githubusercontent.com/yunjii-cn/aw/main/versions.json",
    },
    SourceConfig {
        name: "gitee",
        versions_url: "https://gitee.com/yunjii-cn/aw/raw/main/versions.json",
    },
];

/// 获取当前应用版本
///
/// 与窗口标题共用同一口径：优先使用构建注入的 `VER_BUILD_TIME`（15 位时间戳），
/// 开发态回退到启动时刻的相同格式时间戳，避免与版本清单比较时出现 semver/时间戳混比。
fn get_current_version(_app: &tauri::AppHandle) -> String {
    option_env!("VER_BUILD_TIME")
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%Y.%m.%d.%H%M").to_string())
}

/// 解析 15 位时间戳版本号 `YYYY.MM.DD.HHMM`，非法返回 None
fn parse_version_stamp(value: &str) -> Option<[u32; 4]> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let expected_lens = [4usize, 2, 2, 4];
    let mut numbers = [0u32; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.len() != expected_lens[index] || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        numbers[index] = part.parse().ok()?;
    }
    let (month, day, hhmm) = (numbers[1], numbers[2], numbers[3]);
    let (hour, minute) = (hhmm / 100, hhmm % 100);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    Some(numbers)
}

/// 判断版本清单是否比当前版本新；两侧必须是同一种时间戳口径，否则显式报错
fn compare_version_stamps(current: &str, latest: &str) -> AppResult<bool> {
    let current_stamp = parse_version_stamp(current).ok_or_else(|| {
        crate::AppError::Other(format!(
            "当前版本号格式非法（期望 YYYY.MM.DD.HHMM）：{current}"
        ))
    })?;
    let latest_stamp = parse_version_stamp(latest).ok_or_else(|| {
        crate::AppError::Other(format!(
            "版本清单版本号格式非法（期望 YYYY.MM.DD.HHMM）：{latest}"
        ))
    })?;
    Ok(latest_stamp > current_stamp)
}

/// 获取 ver/ 目录路径（存放各版本 EXE）
fn get_ver_dir(app: &tauri::AppHandle) -> PathBuf {
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let base = if let Ok(exe) = std::env::current_exe() {
        // 优先用 EXE 所在目录的上级（与视频创意站一致）
        exe.parent()
            .and_then(|p| p.parent())
            .unwrap_or(&app_dir)
            .to_path_buf()
    } else {
        app_dir
    };
    base.join("ver")
}

/// 扫描本地已下载的 EXE 版本
fn list_local_versions(ver_dir: &PathBuf) -> Vec<String> {
    let mut versions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(ver_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                // 提取版本号：文件名格式如 aw-desktop-v2026.06.24.1030.exe
                if let Some(start) = name.find("-v") {
                    let ver_part = &name[start + 2..];
                    let ver = ver_part.trim_end_matches(".exe").trim_end_matches(".EXE");
                    if !ver.is_empty() {
                        versions.push(ver.to_string());
                    }
                }
            }
        }
    }
    versions.sort_by(|a, b| b.cmp(a));
    versions
}

/// 检查更新（多源竞速）
#[tauri::command]
pub async fn check_for_updates(
    source: Option<String>,
    app: tauri::AppHandle,
) -> AppResult<UpdateCheckResult> {
    let current_version = get_current_version(&app);
    let ver_dir = get_ver_dir(&app);
    let local_versions = list_local_versions(&ver_dir);

    let source_str = source.unwrap_or_else(|| "auto".to_string());

    // 获取版本列表
    let versions: Vec<VersionInfo> = if source_str == "auto" {
        // 多源竞速
        fetch_versions_race().await?
    } else {
        // 单源
        let src = SOURCES
            .iter()
            .find(|s| s.name == source_str)
            .ok_or_else(|| crate::AppError::Other(format!("未知更新源: {}", source_str)))?;
        fetch_versions(src.versions_url).await?
    };

    // 找最新版本
    let latest_version = versions
        .first()
        .map(|v| v.version.clone());

    let has_update = match latest_version.as_ref() {
        Some(latest) => compare_version_stamps(&current_version, latest)?,
        None => false,
    };

    Ok(UpdateCheckResult {
        current_version,
        latest_version,
        has_update,
        versions,
        source: source_str,
        local_versions,
    })
}

/// 多源竞速获取版本列表
async fn fetch_versions_race() -> AppResult<Vec<VersionInfo>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| crate::AppError::Other(format!("HTTP 客户端创建失败: {}", e)))?;

    let mut tasks = Vec::new();
    for src in SOURCES {
        let url = src.versions_url.to_string();
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            fetch_versions_with_client(&client, &url).await
        }));
    }

    // 等待第一个成功的结果
    for task in tasks {
        match task.await {
            Ok(Ok(versions)) if !versions.is_empty() => return Ok(versions),
            Ok(Err(e)) => tracing::warn!("更新源获取失败: {}", e),
            Err(e) => tracing::warn!("更新源任务异常: {}", e),
            _ => {}
        }
    }

    Err(crate::AppError::Other(
        "所有更新源均不可用，请检查网络连接".to_string(),
    ))
}

/// 单源获取版本列表
async fn fetch_versions(url: &str) -> AppResult<Vec<VersionInfo>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| crate::AppError::Other(format!("HTTP 客户端创建失败: {}", e)))?;

    fetch_versions_with_client(&client, url).await
}

/// 用指定 client 获取版本列表
async fn fetch_versions_with_client(
    client: &reqwest::Client,
    url: &str,
) -> AppResult<Vec<VersionInfo>> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| crate::AppError::Other(format!("请求失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::AppError::Other(format!(
            "HTTP {}",
            resp.status()
        )));
    }

    let versions: Vec<VersionInfo> = resp
        .json()
        .await
        .map_err(|e| crate::AppError::Other(format!("解析版本列表失败: {}", e)))?;

    Ok(versions)
}

/// 计算文件 SHA-256（小写十六进制）
fn sha256_file(path: &Path) -> AppResult<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)
        .map_err(|e| crate::AppError::Other(format!("读取产物失败: {}", e)))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

/// 校验版本清单给出的期望哈希；缺失或格式非法时拒绝下载
fn validate_expected_sha256(expected: &str) -> AppResult<String> {
    let normalized = expected.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(crate::AppError::Other(
            "版本清单缺少 sha256，拒绝安装未经哈希校验的产物".to_string(),
        ));
    }
    if normalized.len() != 64 || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(crate::AppError::Other(format!(
            "版本清单 sha256 格式非法（应为 64 位十六进制）：{normalized}"
        )));
    }
    Ok(normalized)
}

/// 校验临时产物后原子提升为正式文件；校验失败清理临时文件，不留下半装状态
fn promote_verified_download(
    tmp_path: &Path,
    final_path: &Path,
    expected_sha256: &str,
) -> AppResult<()> {
    let verified = validate_expected_sha256(expected_sha256).and_then(|expected| {
        sha256_file(tmp_path).and_then(|actual| {
            if actual != expected {
                Err(crate::AppError::Other(format!(
                    "下载校验失败：SHA-256 与版本清单不符（期望 {}，实际 {}），已拒绝安装",
                    expected, actual
                )))
            } else {
                Ok(())
            }
        })
    });

    if let Err(error) = verified {
        let _ = std::fs::remove_file(tmp_path);
        return Err(error);
    }

    std::fs::rename(tmp_path, final_path)
        .map_err(|e| crate::AppError::Other(format!("重命名文件失败: {}", e)))
}

/// 下载指定版本 EXE
///
/// 下载到 ver/ 目录，文件名格式：aw-desktop-v{version}.exe
/// 下载中通过事件推送进度（event: "download-progress"）
#[tauri::command]
pub async fn download_version(
    version: String,
    download_url: String,
    sha256: Option<String>,
    app: tauri::AppHandle,
) -> AppResult<String> {
    let ver_dir = get_ver_dir(&app);

    // 确保 ver/ 目录存在
    std::fs::create_dir_all(&ver_dir)
        .map_err(|e| crate::AppError::Other(format!("创建 ver 目录失败: {}", e)))?;

    let exe_name = format!("aw-desktop-v{}.exe", version);
    let exe_path = ver_dir.join(&exe_name);
    let tmp_path = ver_dir.join(format!("{}.downloading", exe_name));

    // 如果已存在，直接返回
    if exe_path.exists() {
        return Ok(format!("版本 {} 已下载", version));
    }

    // 下载前先校验期望哈希，缺失或非法时直接拒绝，不发起下载
    let expected_sha256 = validate_expected_sha256(sha256.as_deref().unwrap_or(""))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| crate::AppError::Other(format!("HTTP 客户端创建失败: {}", e)))?;

    let resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| crate::AppError::Other(format!("下载请求失败: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::AppError::Other(format!(
            "下载失败: HTTP {}",
            resp.status()
        )));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| crate::AppError::Other(format!("创建临时文件失败: {}", e)))?;

    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| crate::AppError::Other(format!("读取数据失败: {}", e)))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| crate::AppError::Other(format!("写入文件失败: {}", e)))?;
        downloaded += chunk.len() as u64;

        // 推送进度事件
        let percent = if total_size > 0 {
            (downloaded * 100 / total_size) as u32
        } else {
            0
        };
        let _ = app.emit(
            "download-progress",
            serde_json::json!({
                "version": &version,
                "downloaded": downloaded,
                "total": total_size,
                "percent": percent,
            }),
        );
    }

    file.flush().await
        .map_err(|e| crate::AppError::Other(format!("文件刷新失败: {}", e)))?;
    drop(file);

    // 校验哈希后把临时文件提升为正式文件；不符则清理临时文件并拒绝安装
    promote_verified_download(&tmp_path, &exe_path, &expected_sha256)?;

    tracing::info!("版本 {} 下载完成: {:?}", version, exe_path);
    Ok(format!("版本 {} 下载完成", version))
}

/// 切换到指定版本（创建硬链接）
///
/// 1. 验证 EXE 文件大小（至少 1MB）
/// 2. 备份当前入口 EXE 到 .bak
/// 3. 通过硬链接更新入口
/// 4. 提示用户重启
#[tauri::command]
pub async fn switch_version(
    version: String,
    app: tauri::AppHandle,
) -> AppResult<String> {
    let ver_dir = get_ver_dir(&app);
    let exe_name = format!("aw-desktop-v{}.exe", version);
    let exe_path = ver_dir.join(&exe_name);

    // 验证文件存在且大小足够
    if !exe_path.exists() {
        return Err(crate::AppError::Other(format!(
            "版本 {} 的 EXE 文件不存在，请先下载",
            version
        )));
    }

    let file_size = std::fs::metadata(&exe_path)
        .map_err(|e| crate::AppError::Other(format!("读取文件信息失败: {}", e)))?
        .len();

    if file_size < 1_000_000 {
        return Err(crate::AppError::Other(format!(
            "版本 {} 的 EXE 文件大小异常（{} 字节），可能下载不完整",
            version, file_size
        )));
    }

    // 获取当前入口 EXE 路径
    let current_exe = std::env::current_exe()
        .map_err(|e| crate::AppError::Other(format!("获取当前 EXE 路径失败: {}", e)))?;

    // 备份当前入口
    let bak_path = current_exe.with_extension("exe.bak");
    if current_exe.exists() {
        let _ = std::fs::remove_file(&bak_path);
        std::fs::copy(&current_exe, &bak_path)
            .map_err(|e| crate::AppError::Other(format!("备份当前版本失败: {}", e)))?;
    }

    // 创建硬链接（失败则回退到复制）
    let _ = std::fs::remove_file(&current_exe);
    if let Err(e) = std::fs::hard_link(&exe_path, &current_exe) {
        tracing::warn!("硬链接创建失败，回退到复制: {}", e);
        std::fs::copy(&exe_path, &current_exe)
            .map_err(|e| crate::AppError::Other(format!("复制 EXE 失败: {}", e)))?;
    }

    tracing::info!("已切换到版本 {}", version);
    Ok(format!(
        "已切换到版本 {}，请重启应用以应用更新",
        version
    ))
}

/// 打开 ver/ 目录
#[tauri::command]
pub async fn open_ver_dir(app: tauri::AppHandle) -> AppResult<()> {
    let ver_dir = get_ver_dir(&app);
    std::fs::create_dir_all(&ver_dir)
        .map_err(|e| crate::AppError::Other(format!("创建 ver 目录失败: {}", e)))?;

    let path = ver_dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| crate::AppError::Io(e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "aw-updater-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn same_stamp_is_not_an_update() {
        assert!(!compare_version_stamps("2026.11.05.1905", "2026.11.05.1905").unwrap());
    }

    #[test]
    fn older_manifest_is_not_an_update() {
        assert!(!compare_version_stamps("2026.11.05.1905", "2026.11.05.1800").unwrap());
    }

    #[test]
    fn newer_manifest_is_an_update() {
        assert!(compare_version_stamps("2026.11.05.1905", "2026.11.06.0900").unwrap());
    }

    #[test]
    fn mismatched_version_calibers_are_rejected() {
        let err = compare_version_stamps("0.1.0", "2026.11.05.1905")
            .expect_err("semver 与时间戳口径不一致必须报错");
        assert!(
            err.to_string().contains("当前版本号格式非法"),
            "错误必须点名口径问题，实际: {err}"
        );
    }

    #[test]
    fn sha256_match_promotes_download() {
        let dir = temp_dir("match");
        let tmp = dir.join("pkg.downloading");
        let final_path = dir.join("pkg.exe");
        std::fs::write(&tmp, b"verified payload").unwrap();
        let expected = sha256_file(&tmp).unwrap();

        promote_verified_download(&tmp, &final_path, &expected).expect("哈希一致应放行");
        assert!(final_path.exists(), "校验通过后应生成正式文件");
        assert!(!tmp.exists(), "正式文件生成后应移除临时文件");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_mismatch_is_rejected_without_half_install() {
        let dir = temp_dir("mismatch");
        let tmp = dir.join("pkg.downloading");
        let final_path = dir.join("pkg.exe");
        std::fs::write(&tmp, b"tampered payload").unwrap();

        let err = promote_verified_download(&tmp, &final_path, &"0".repeat(64))
            .expect_err("哈希不符必须拒绝");
        assert!(
            err.to_string().contains("SHA-256"),
            "错误必须具名指出 SHA-256，实际: {err}"
        );
        assert!(!final_path.exists(), "校验失败不得生成正式安装文件");
        assert!(!tmp.exists(), "校验失败必须清理临时文件");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_sha256_is_rejected_and_temp_cleaned() {
        let dir = temp_dir("missing");
        let tmp = dir.join("pkg.downloading");
        let final_path = dir.join("pkg.exe");
        std::fs::write(&tmp, b"unverified payload").unwrap();

        let err = promote_verified_download(&tmp, &final_path, "")
            .expect_err("缺少哈希必须拒绝");
        assert!(
            err.to_string().contains("sha256"),
            "错误必须点名缺失 sha256，实际: {err}"
        );
        assert!(!final_path.exists());
        assert!(!tmp.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
