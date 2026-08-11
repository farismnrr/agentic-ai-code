use crate::relay_agent::config::ServerConfig;
use crate::relay_agent::transport::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::Duration;

use uuid::Uuid;

const MAX_WS_MESSAGE_LEN: usize = 65536; // 64 KB
const MAX_LEGACY_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MB limit
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60); // 12 hours

pub struct LegacyState {
    pub pairing_token: String,
    pub pairing_token_expires_at: Instant,
    pub session_credentials: HashMap<String, Instant>,
    pub default_cwd: PathBuf,
    pub active_executions: HashMap<String, usize>,
}

impl LegacyState {
    pub fn new(config: &ServerConfig) -> Self {
        let pairing_token = Uuid::new_v4().to_string().replace("-", "");

        let default_cwd = config
            .resolved_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

        println!(
            "[relay-agent] Listening on http://127.0.0.1:{}",
            config.port
        );
        println!(
            "[relay-agent] Allowed Origin: {}",
            config.origin.as_deref().unwrap_or("*")
        );
        println!(
            "[relay-agent] Default directory: {} (not a restriction — commands may target any path this OS user can access)",
            default_cwd.display()
        );
        // Removed credential logging as per Phase 11 requirements.

        Self {
            pairing_token,
            pairing_token_expires_at: Instant::now() + Duration::from_secs(5 * 60),
            session_credentials: HashMap::new(),
            default_cwd,
            active_executions: HashMap::new(),
        }
    }

    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.session_credentials
            .retain(|_, expires_at| *expires_at > now);
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/pair", post(pair))
        .route("/revoke", post(revoke))
        .route("/", get(websocket_handler))
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cwd = {
        let l = state.legacy.lock().unwrap();
        l.default_cwd.clone()
    };
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "agent": "relay-agent",
            "defaultCwd": cwd.to_string_lossy()
        })),
    )
}

#[derive(Deserialize)]
struct PairRequest {
    token: Option<String>,
}

async fn pair(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PairRequest>,
) -> impl IntoResponse {
    let mut l = state.legacy.lock().unwrap();

    let token = match payload.token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid pairing token"})),
            );
        }
    };

    if l.pairing_token.is_empty() || token != l.pairing_token {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid pairing token"})),
        );
    }

    if Instant::now() > l.pairing_token_expires_at {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Pairing token has expired (~5 min TTL)"})),
        );
    }

    let session_cred = Uuid::new_v4().to_string().replace("-", "");

    l.cleanup_expired();
    l.session_credentials
        .insert(session_cred.clone(), Instant::now() + SESSION_TTL);
    l.pairing_token = String::new(); // single use

    (
        StatusCode::OK,
        Json(json!({ "sessionCredential": session_cred })),
    )
}

#[derive(Deserialize)]
struct RevokeRequest {
    credential: Option<String>,
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RevokeRequest>,
) -> impl IntoResponse {
    let mut l = state.legacy.lock().unwrap();

    let credential = match payload.credential {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid credential"})),
            );
        }
    };

    if l.session_credentials.remove(&credential).is_some() {
        (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Session credential revoked" })),
        )
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid credential"})),
        )
    }
}

#[derive(Deserialize)]
struct WsQuery {
    credential: Option<String>,
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let credential = query.credential.unwrap_or_default();

    let valid = {
        let mut l = state.legacy.lock().unwrap();
        l.cleanup_expired();
        l.session_credentials.contains_key(&credential)
    };

    if !valid {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_ws(socket, state, credential))
}

#[derive(Deserialize)]
struct ExecPayload {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>, credential: String) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if text.len() > MAX_WS_MESSAGE_LEN {
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&json!({
                            "type": "error",
                            "error": "Message size exceeded limit"
                        }))
                        .unwrap(),
                    ))
                    .await;
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                let ty = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if ty == "exec" {
                    if let Ok(payload) = serde_json::from_value::<ExecPayload>(json) {
                        let id = payload.id.clone();

                        // Limit size of strings
                        if payload.command.as_ref().map(|s| s.len()).unwrap_or(0) > 4096
                            || payload.cwd.as_ref().map(|s| s.len()).unwrap_or(0) > 4096
                            || payload.args.iter().map(|s| s.len()).sum::<usize>() > 65536
                        {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": "Command or arguments exceeded length limits"
                                    }))
                                    .unwrap(),
                                ))
                                .await;
                            continue;
                        }

                        if payload.command.is_none()
                            || payload.command.as_ref().unwrap().trim().is_empty()
                        {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": "Command is required"
                                    }))
                                    .unwrap(),
                                ))
                                .await;
                            continue;
                        }

                        // Check limits
                        let check_limit_res = {
                            let mut l = state.legacy.lock().unwrap();
                            let count = l.active_executions.entry(credential.clone()).or_insert(0);
                            if *count >= 4 {
                                Err("Per-session execution concurrency limit exceeded")
                            } else {
                                *count += 1;
                                Ok(())
                            }
                        };

                        if let Err(e) = check_limit_res {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": e
                                    }))
                                    .unwrap(),
                                ))
                                .await;
                            continue;
                        }

                        let _permit_global =
                            state.execution_semaphore.clone().acquire_owned().await;

                        let command = payload.command.unwrap();
                        let parts: Vec<&str> = command.split_whitespace().collect();
                        let binary = parts.first().copied().unwrap_or(command.as_str());
                        let mut final_args =
                            parts[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>();
                        final_args.extend(payload.args);

                        // Reject --no-guard for legacy path as well
                        if final_args.iter().any(|arg| arg == "--no-guard") {
                            {
                                let mut l = state.legacy.lock().unwrap();
                                if let Some(c) = l.active_executions.get_mut(&credential) {
                                    *c = c.saturating_sub(1);
                                }
                            }
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": "--no-guard is strictly forbidden"
                                    }))
                                    .unwrap(),
                                ))
                                .await;
                            continue;
                        }

                        let target_cwd = if let Some(c) = payload.cwd {
                            let default_cwd = {
                                let l = state.legacy.lock().unwrap();
                                l.default_cwd.clone()
                            };
                            let p = std::path::PathBuf::from(c);
                            if p.is_absolute() {
                                p
                            } else {
                                default_cwd.join(p)
                            }
                        } else {
                            let l = state.legacy.lock().unwrap();
                            l.default_cwd.clone()
                        };

                        let timeout_ms = 300000; // 5 mins

                        let mut cmd = Command::new(binary);
                        cmd.args(final_args)
                            .current_dir(target_cwd)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .kill_on_drop(true);

                        #[cfg(unix)]
                        {
                            cmd.process_group(0);
                        }

                        cmd.env_clear();
                        if let Ok(v) = std::env::var("PATH") {
                            cmd.env("PATH", v);
                        }
                        if let Ok(v) = std::env::var("HOME") {
                            cmd.env("HOME", v);
                        }
                        if let Ok(v) = std::env::var("LANG") {
                            cmd.env("LANG", v);
                        }

                        let child_opt = cmd.spawn().ok();

                        if let Some(mut child) = child_opt {
                            let pid = child.id();

                            let mut stdout_pipe = child.stdout.take().unwrap();
                            let mut stderr_pipe = child.stderr.take().unwrap();

                            let read_stdout = async {
                                let mut stdout_buf = Vec::new();
                                let mut handle =
                                    (&mut stdout_pipe).take(MAX_LEGACY_OUTPUT_BYTES as u64 + 1);
                                handle
                                    .read_to_end(&mut stdout_buf)
                                    .await
                                    .map(|_| stdout_buf)
                            };
                            let read_stderr = async {
                                let mut stderr_buf = Vec::new();
                                let mut handle =
                                    (&mut stderr_pipe).take(MAX_LEGACY_OUTPUT_BYTES as u64 + 1);
                                handle
                                    .read_to_end(&mut stderr_buf)
                                    .await
                                    .map(|_| stderr_buf)
                            };

                            let read_and_wait = async {
                                let (out_res, err_res) = tokio::join!(read_stdout, read_stderr);
                                let stdout_buf = out_res?;
                                let stderr_buf = err_res?;

                                if stdout_buf.len() > MAX_LEGACY_OUTPUT_BYTES
                                    || stderr_buf.len() > MAX_LEGACY_OUTPUT_BYTES
                                {
                                    if let Some(p) = pid {
                                        #[cfg(unix)]
                                        {
                                            unsafe {
                                                libc::kill(-(p as i32), libc::SIGKILL);
                                            }
                                        }
                                    }
                                }

                                let status = child.wait().await?;
                                Ok::<_, std::io::Error>((status, stdout_buf, stderr_buf))
                            };

                            match tokio::time::timeout(
                                Duration::from_millis(timeout_ms),
                                read_and_wait,
                            )
                            .await
                            {
                                Ok(Ok((status, stdout_bytes, stderr_bytes))) => {
                                    let exit_code = status.code();
                                    let success = status.success();
                                    let mut stdout_str =
                                        String::from_utf8_lossy(&stdout_bytes).into_owned();
                                    let mut stderr_str =
                                        String::from_utf8_lossy(&stderr_bytes).into_owned();
                                    if stdout_str.len() > MAX_LEGACY_OUTPUT_BYTES {
                                        stdout_str.truncate(MAX_LEGACY_OUTPUT_BYTES);
                                        stdout_str.push_str("\n...[truncated due to size limit]");
                                    }
                                    if stderr_str.len() > MAX_LEGACY_OUTPUT_BYTES {
                                        stderr_str.truncate(MAX_LEGACY_OUTPUT_BYTES);
                                        stderr_str.push_str("\n...[truncated due to size limit]");
                                    }

                                    let mut res = json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": success,
                                        "exitCode": exit_code,
                                        "stdout": stdout_str,
                                        "stderr": stderr_str,
                                    });
                                    if !success {
                                        if let Some(code) = exit_code {
                                            res["error"] = json!(format!(
                                                "Command failed with exit code {}",
                                                code
                                            ));
                                        } else {
                                            res["error"] =
                                                json!("Command failed (killed by signal)");
                                        }
                                    }
                                    let _ = socket
                                        .send(Message::Text(serde_json::to_string(&res).unwrap()))
                                        .await;
                                }
                                Ok(Err(_e)) => {
                                    let _ = socket
                                        .send(Message::Text(
                                            serde_json::to_string(&json!({
                                                "type": "exec_result",
                                                "id": id,
                                                "success": false,
                                                "error": "Failed to read command output"
                                            }))
                                            .unwrap(),
                                        ))
                                        .await;
                                }
                                Err(_) => {
                                    if let Some(p) = pid {
                                        #[cfg(unix)]
                                        {
                                            unsafe {
                                                libc::kill(-(p as i32), libc::SIGKILL);
                                            }
                                        }
                                    }
                                    let _ = socket.send(Message::Text(serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": format!("Command timed out after {}s", timeout_ms / 1000)
                                    })).unwrap())).await;
                                }
                            }
                        } else {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "exec_result",
                                        "id": id,
                                        "success": false,
                                        "error": "Failed to spawn command"
                                    }))
                                    .unwrap(),
                                ))
                                .await;
                        }

                        {
                            let mut l = state.legacy.lock().unwrap();
                            if let Some(c) = l.active_executions.get_mut(&credential) {
                                *c = c.saturating_sub(1);
                            }
                        }
                    }
                } else {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::to_string(&json!({
                                "type": "error",
                                "error": format!("Unknown message type: {}", ty)
                            }))
                            .unwrap(),
                        ))
                        .await;
                }
            }
        }
    }
}
