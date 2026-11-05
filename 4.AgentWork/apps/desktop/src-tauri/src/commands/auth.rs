// 用户认证命令（UM 统一身份系统）
// 流程：启动本地回调服务器 -> 获取 UM 登录二维码/URL -> 用户扫码 -> UM 回调本地服务器 -> 换取用户信息

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::Emitter;

use crate::AppResult;

// 直接调用 UM 官方 API 域名，不经过 sl.yunjii.cn 反代
const UM_CONNECT_URL: &str = "https://um.yunjii.cn/php/connect.php";
const UM_USER_INFO_URL: &str = "https://um.yunjii.cn/php/check_token.php";

/// UM 登录二维码/URL 响应
#[derive(Serialize, Clone)]
pub struct UmLoginStartResult {
    /// 登录会话 ID（用于前端轮询或事件匹配）
    pub session_id: String,
    /// 完整登录 URL（微信内打开）
    pub login_url: String,
    /// PC 端二维码 URL（UM 接口返回的原始字符串，可能是 jump 链接或图片直链）
    pub qrcode_url: String,
    /// 二维码图片直链（构造自 qrcode 中的 state，便于 <img> 直接渲染）
    pub qr_image_url: String,
    /// 本地回调地址
    pub redirect_uri: String,
    /// UM oauth_code（用于 callback 校验）
    pub oauth_code: String,
}

/// UM 用户信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UmUserInfo {
    pub social_uid: Option<String>,
    pub user_id: Option<u64>,
    pub nickname: Option<String>,
    pub faceimg: Option<String>,
    pub avatar: Option<String>,
    pub gender: Option<String>,
    pub location: Option<String>,
    pub access_token: Option<String>,
    pub token: Option<String>,
}

/// 当前登录会话状态
#[derive(Default)]
pub struct UmLoginSession {
    pub session_id: Option<String>,
    pub user_info: Option<UmUserInfo>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// 启动 UM 登录流程
/// 启动本地 HTTP 回调服务器，调用 UM login API 返回二维码/URL
#[tauri::command]
pub async fn start_um_login(
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
) -> AppResult<UmLoginStartResult> {
    let appid = read_required_env(&["UM_APPID", "um_appid"], "UM_APPID")?;
    let appkey = read_required_env(&["UM_APPKEY", "um_appkey"], "UM_APPKEY")?;

    let session_id = uuid::Uuid::new_v4().to_string();

    // 启动本地 HTTP 回调服务器
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await
        .map_err(|e| crate::AppError::Other(format!("启动本地回调服务器失败: {e}")))?;
    let addr: SocketAddr = listener.local_addr()
        .map_err(|e| crate::AppError::Other(format!("获取本地地址失败: {e}")))?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", addr.port());

    let appid_clone = appid.clone();
    let appkey_clone = appkey.clone();
    let session_id_clone = session_id.clone();
    let app_handle = app.clone();

    let server_handle = tokio::spawn(async move {
        let app = axum::Router::new()
            .route("/callback", axum::routing::get(handle_um_callback))
            .layer(axum::Extension(CallbackState {
                appid: appid_clone,
                appkey: appkey_clone,
                session_id: session_id_clone,
                app_handle,
            }));

        let server = axum::serve(listener, app);
        let server = server.with_graceful_shutdown(async {
            let _ = rx.await;
        });

        if let Err(e) = server.await {
            tracing::warn!("本地回调服务器异常退出: {}", e);
        }
    });

    // 保存会话状态
    {
        let mut session = state.um_login_session.lock().await;
        if let Some(prev) = session.server_handle.take() {
            let _ = prev.abort();
        }
        session.session_id = Some(session_id.clone());
        session.user_info = None;
        session.server_handle = Some(server_handle);
        session.shutdown_tx = Some(tx);
    }

    // 调用 UM login API 获取二维码/URL
    let client = reqwest::Client::new();
    let res = client
        .get(UM_CONNECT_URL)
        .query(&[
            ("act", "login"),
            ("appid", &appid),
            ("appkey", &appkey),
            ("type", "wx"),
            ("redirect_uri", &redirect_uri),
            ("state", &session_id),
        ])
        .send()
        .await
        .map_err(|e| crate::AppError::Other(format!("请求 UM 登录接口失败: {e}")))?;

    let json: serde_json::Value = res.json().await
        .map_err(|e| crate::AppError::Other(format!("解析 UM 登录响应失败: {e}")))?;

    if json.get("code").and_then(|v| v.as_i64()) != Some(0) {
        let msg = json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("UM 登录接口返回错误");
        return Err(crate::AppError::Other(msg.to_string()));
    }

    let data_block = json.get("data").cloned().unwrap_or(json.clone());

    let login_url = pick_string(&data_block, &["login_url", "loginUrl", "url", "scan_url"])
        .or_else(|| pick_string(&json, &["login_url", "loginUrl", "url"]))
        .unwrap_or_default();

    let qrcode_url = pick_string(
        &data_block,
        &[
            "qrcode",
            "qrcode_url",
            "qrcodeUrl",
            "qr_url",
            "qrUrl",
            "qr_img",
            "qrImg",
            "qrcode_img",
            "qrcodeImg",
            "img",
        ],
    )
    .or_else(|| {
        pick_string(
            &json,
            &[
                "qrcode",
                "qrcode_url",
                "qrcodeUrl",
                "qr_url",
                "qrUrl",
                "qr_img",
                "qrImg",
                "qrcode_img",
                "qrcodeImg",
            ],
        )
    })
    .unwrap_or_default();

    let qrcode_url = if qrcode_url.is_empty() {
        String::new()
    } else if qrcode_url.starts_with("http://") || qrcode_url.starts_with("https://") {
        qrcode_url
    } else if qrcode_url.starts_with("//") {
        format!("https:{}", qrcode_url)
    } else if qrcode_url.starts_with('/') {
        format!("https://sl.yunjii.cn{}", qrcode_url)
    } else {
        qrcode_url
    };

    if qrcode_url.is_empty() {
        tracing::warn!(
            "UM 登录接口未返回二维码字段，原始响应: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );
    }

    // 把 qrcode_url 生成二维码 PNG base64 data URI，前端 <img> 直接渲染
    let qr_source = if !qrcode_url.is_empty() { &qrcode_url } else { &login_url };
    let qr_image_url = generate_qr_data_uri(qr_source);

    let oauth_code = json
        .get("oauth_code")
        .and_then(|v| v.as_str())
        .or_else(|| data_block.get("oauth_code").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    Ok(UmLoginStartResult {
        session_id,
        login_url,
        qrcode_url,
        qr_image_url,
        redirect_uri,
        oauth_code,
    })
}

/// 把 URL 生成二维码 PNG base64 data URI，供前端 <img> 直接渲染
fn generate_qr_data_uri(url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    let code = match qrcode::QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("生成二维码失败: {}", e);
            return String::new();
        }
    };
    let img = code
        .render::<image::Luma<u8>>()
        .min_dimensions(256, 256)
        .build();
    let mut buf = Vec::new();
    if image::DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .is_err()
    {
        tracing::warn!("二维码 PNG 编码失败");
        return String::new();
    }
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/png;base64,{}", b64)
}

fn pick_string(json: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(v) = json.get(*k).and_then(|x| x.as_str()) {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

#[derive(Clone)]
struct CallbackState {
    appid: String,
    appkey: String,
    session_id: String,
    app_handle: tauri::AppHandle,
}

async fn handle_um_callback(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    axum::Extension(state): axum::Extension<CallbackState>,
) -> impl axum::response::IntoResponse {
    let code = params.get("code").cloned().unwrap_or_default();
    let recv_state = params.get("state").cloned().unwrap_or_default();

    if code.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing code parameter".to_string(),
        );
    }

    // 用 code 换取用户信息
    let client = reqwest::Client::new();
    let res = client
        .get(UM_CONNECT_URL)
        .query(&[
            ("act", "callback"),
            ("appid", &state.appid),
            ("appkey", &state.appkey),
            ("code", &code),
        ])
        .send()
        .await;

    match res {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                if json.get("code").and_then(|v| v.as_i64()) == Some(0) {
                    let user = parse_um_user(&json);
                    let _ = state
                        .app_handle
                        .emit("um:login-success", serde_json::json!({
                            "session_id": state.session_id,
                            "state": recv_state,
                            "user": user,
                        }));
                    (
                        axum::http::StatusCode::OK,
                        "登录成功，请返回应用".to_string(),
                    )
                } else {
                    let msg = json
                        .get("msg")
                        .and_then(|v| v.as_str())
                        .unwrap_or("登录失败")
                        .to_string();
                    let _ = state.app_handle.emit("um:login-failed", msg.clone());
                    (axum::http::StatusCode::OK, msg)
                }
            }
            Err(e) => {
                let _ = state
                    .app_handle
                    .emit("um:login-failed", format!("解析响应失败: {e}"));
                (
                    axum::http::StatusCode::OK,
                    format!("解析响应失败: {e}"),
                )
            }
        },
        Err(e) => {
            let _ = state
                .app_handle
                .emit("um:login-failed", format!("网络错误: {e}"));
            (axum::http::StatusCode::OK, format!("网络错误: {e}"))
        }
    }
}

fn parse_um_user(json: &serde_json::Value) -> UmUserInfo {
    UmUserInfo {
        social_uid: json.get("social_uid").and_then(|v| v.as_str()).map(String::from),
        user_id: json.get("user_id").and_then(|v| v.as_u64()),
        nickname: json.get("nickname").and_then(|v| v.as_str()).map(String::from),
        faceimg: json.get("faceimg").and_then(|v| v.as_str()).map(String::from),
        avatar: json.get("avatar").and_then(|v| v.as_str()).map(String::from),
        gender: json.get("gender").and_then(|v| v.as_str()).map(String::from),
        location: json.get("location").and_then(|v| v.as_str()).map(String::from),
        access_token: json.get("access_token").and_then(|v| v.as_str()).map(String::from),
        token: json.get("token").and_then(|v| v.as_str()).map(String::from),
    }
}

/// 用 token 获取 UM 用户信息
#[tauri::command]
pub async fn get_um_user_info(token: String) -> AppResult<UmUserInfo> {
    let client = reqwest::Client::new();
    let res = client
        .post(UM_USER_INFO_URL)
        .query(&[("act", "get")])
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| crate::AppError::Other(format!("请求 UM 用户信息失败: {e}")))?;

    let json: serde_json::Value = res.json().await
        .map_err(|e| crate::AppError::Other(format!("解析 UM 用户信息失败: {e}")))?;

    if json.get("code").and_then(|v| v.as_i64()) != Some(0) {
        let msg = json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("获取用户信息失败");
        return Err(crate::AppError::Other(msg.to_string()));
    }

    let data = json.get("data").cloned().unwrap_or(serde_json::Value::Null);
    Ok(UmUserInfo {
        social_uid: data.get("openid").and_then(|v| v.as_str()).map(String::from),
        user_id: data.get("id").and_then(|v| v.as_u64()),
        nickname: data.get("nickname").and_then(|v| v.as_str()).map(String::from),
        faceimg: data.get("avatar").and_then(|v| v.as_str()).map(String::from),
        avatar: data.get("avatar").and_then(|v| v.as_str()).map(String::from),
        gender: data.get("gender").and_then(|v| v.as_str()).map(String::from),
        location: data.get("location").and_then(|v| v.as_str()).map(String::from),
        access_token: Some(token.clone()),
        token: Some(token),
    })
}

/// 取消当前登录会话
#[tauri::command]
pub async fn cancel_um_login(
    state: tauri::State<'_, crate::AppState>,
) -> AppResult<()> {
    let mut session = state.um_login_session.lock().await;
    if let Some(tx) = session.shutdown_tx.take() {
        let _ = tx.send(());
    }
    if let Some(handle) = session.server_handle.take() {
        handle.abort();
    }
    session.session_id = None;
    session.user_info = None;
    Ok(())
}

/// 读取必需的环境变量（按候选名依次尝试，兼容大小写写法）。
/// 缺失或为空时返回显式错误，不使用任何内置默认值。
fn read_required_env(names: &[&str], label: &str) -> Result<String, crate::AppError> {
    for name in names {
        if let Ok(value) = std::env::var(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }
    Err(crate::AppError::Other(format!(
        "缺少 UM 凭据：环境变量 {label} 未设置或为空。已移除内置默认值，请通过启动脚本或系统环境变量注入后再登录"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 凭据缺失时必须是显式失败，而不是回落到内置值或空字符串
    #[test]
    fn missing_env_must_fail_explicitly() {
        // 该变量名在真实环境中不存在，保证测试稳定
        let names = ["AW_TEST_UNSET_APPKEY_A"];
        match read_required_env(&names, "UM_APPKEY") {
            Ok(v) => panic!("缺少环境变量时不应成功，实际返回长度 {}", v.len()),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("UM_APPKEY"),
                    "错误信息必须指明缺失的变量名，实际: {msg}"
                );
            }
        }
    }

    /// 变量存在时应原样使用（去掉首尾空白），不被默认值覆盖
    #[test]
    fn present_env_is_used() {
        let names = ["AW_TEST_PRESENT_APPKEY_B"];
        std::env::set_var(names[0], "  dummy-value  ");
        let got = read_required_env(&names, "UM_APPKEY");
        std::env::remove_var(names[0]);
        assert_eq!(got.expect("变量已设置时应成功"), "dummy-value");
    }
}
