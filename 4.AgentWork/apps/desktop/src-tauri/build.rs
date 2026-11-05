use std::path::PathBuf;

fn main() {
    // 打包时间注入：与前端共用 apps/desktop/.build-version（由 scripts/build-version.mjs 生成），
    // 保证 Rust 窗口标题与界面版本号是同一个值。
    // 文件不存在时（开发期 cargo check）不注入，lib.rs 退回启动时间。
    let stamp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.build-version");
    println!("cargo:rerun-if-changed={}", stamp.display());

    match std::fs::read_to_string(&stamp) {
        Ok(raw) => {
            let version = raw.trim();
            if is_version_stamp(version) {
                println!("cargo:rustc-env=VER_BUILD_TIME={version}");
            } else {
                println!("cargo:warning=忽略格式非法的 .build-version（期望 YYYY.MM.DD.HHMM）");
            }
        }
        Err(_) => {}
    }

    tauri_build::build()
}

/// 校验 YYYY.MM.DD.HHMM（15 位，含 3 个分隔点）
fn is_version_stamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 15
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| match i {
                4 | 7 | 10 => *b == b'.',
                _ => b.is_ascii_digit(),
            })
}
