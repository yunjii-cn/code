// 构建期注入版本时间戳
// 来源优先级：AW_BUILD_TIME 环境变量 → 桌面端生成的 .build-version → "unknown"
// 不使用当前时间兜底，避免破坏可复现构建
use std::path::PathBuf;

fn is_version_stamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 15 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'.',
        10 => *b == b'.',
        _ => b.is_ascii_digit(),
    })
}

fn read_stamp(path: &PathBuf) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim().to_string();
    if is_version_stamp(&trimmed) {
        Some(trimmed)
    } else {
        None
    }
}

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let desktop_stamp_path = manifest_dir.join("../../apps/desktop/.build-version");

    let bridge_stamp = std::env::var("AW_BUILD_TIME")
        .ok()
        .filter(|value| is_version_stamp(value.trim()))
        .map(|value| value.trim().to_string())
        .or_else(|| read_stamp(&desktop_stamp_path))
        .unwrap_or_else(|| "unknown".to_string());

    let desktop_stamp = read_stamp(&desktop_stamp_path).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=AW_BUILD_TIME={bridge_stamp}");
    println!("cargo:rustc-env=AW_DESKTOP_BUILD_TIME={desktop_stamp}");
    println!("cargo:rerun-if-env-changed=AW_BUILD_TIME");
    println!("cargo:rerun-if-changed={}", desktop_stamp_path.display());
}
