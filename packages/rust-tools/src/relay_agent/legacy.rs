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
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use uuid::Uuid;

pub struct LegacyState {
    pub pairing_token: String,
    pub pairing_token_expires_at: Instant,
    pub session_credentials: HashSet<String>,
    pub default_cwd: PathBuf,
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
            "[relay-agent] Pairing token: {} (expires in 5 minutes)",
            pairing_token
        );
        println!(
            "[relay-agent] Default directory: {} (not a restriction — commands may target any path this OS user can access)",
            default_cwd.display()
        );

        Self {
            pairing_token,
            pairing_token_expires_at: Instant::now() + Duration::from_secs(5 * 60),
            session_credentials: HashSet::new(),
            default_cwd,
        }
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

    l.session_credentials.insert(session_cred.clone());
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

    if l.session_credentials.remove(&credential) {
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
        let l = state.legacy.lock().unwrap();
        l.session_credentials.contains(&credential)
    };

    if !valid {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_ws(socket, state))
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

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                let ty = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if ty == "exec" {
                    if let Ok(payload) = serde_json::from_value::<ExecPayload>(json) {
                        let id = payload.id.clone();
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

                        let command = payload.command.unwrap();
                        let parts: Vec<&str> = command.split_whitespace().collect();
                        let binary = parts.first().copied().unwrap_or(command.as_str());
                        let mut final_args =
                            parts[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>();
                        final_args.extend(payload.args);

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

                        // We can't easily do set extendEnv: false completely but we can clear_env
                        // and explicitly set PATH, HOME, LANG.
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

                        match tokio::time::timeout(Duration::from_millis(timeout_ms), cmd.output())
                            .await
                        {
                            Ok(Ok(output)) => {
                                let exit_code = output.status.code();
                                let success = output.status.success();
                                let mut res = json!({
                                    "type": "exec_result",
                                    "id": id,
                                    "success": success,
                                    "exitCode": exit_code,
                                    "stdout": String::from_utf8_lossy(&output.stdout),
                                    "stderr": String::from_utf8_lossy(&output.stderr),
                                });
                                if !success {
                                    if let Some(code) = exit_code {
                                        res["error"] = json!(format!(
                                            "Command failed with exit code {}",
                                            code
                                        ));
                                    } else {
                                        res["error"] = json!("Command failed (killed by signal)");
                                    }
                                }
                                let _ = socket
                                    .send(Message::Text(serde_json::to_string(&res).unwrap()))
                                    .await;
                            }
                            Ok(Err(e)) => {
                                let _ = socket
                                    .send(Message::Text(
                                        serde_json::to_string(&json!({
                                            "type": "exec_result",
                                            "id": id,
                                            "success": false,
                                            "error": e.to_string()
                                        }))
                                        .unwrap(),
                                    ))
                                    .await;
                            }
                            Err(_) => {
                                let _ = socket.send(Message::Text(serde_json::to_string(&json!({
                                    "type": "exec_result",
                                    "id": id,
                                    "success": false,
                                    "error": format!("Command timed out after {}s", timeout_ms / 1000)
                                })).unwrap())).await;
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
