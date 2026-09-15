use super::{AppState, AuthContext};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    Extension, Json,
};
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

const UPLOAD_TOKEN_HEADER: &str = "x-creative-upload-token";
const MAX_CWD_QUERY_BYTES: usize = 8_192;

#[derive(Debug, Deserialize)]
pub(super) struct UploadQuery {
    #[serde(default)]
    cwd: String,
}

pub(super) async fn handle(
    State(state): State<Arc<AppState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(ticket_id): Path<String>,
    Query(query): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> AxumResponse {
    if !state.config.enable_creative {
        return error(StatusCode::NOT_FOUND, "creative_capability_disabled");
    }
    if query.cwd.len() > MAX_CWD_QUERY_BYTES {
        return error(StatusCode::BAD_REQUEST, "upload_cwd_invalid");
    }
    let cwd_bytes =
        match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(query.cwd.as_bytes()) {
            Ok(value) => value,
            Err(_) => return error(StatusCode::BAD_REQUEST, "upload_cwd_invalid"),
        };
    let cwd = match std::str::from_utf8(&cwd_bytes) {
        Ok("") => None,
        Ok(value) if value.len() <= 4_096 => Some(value),
        _ => return error(StatusCode::BAD_REQUEST, "upload_cwd_invalid"),
    };
    let token = match headers
        .get(UPLOAD_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 256)
    {
        Some(value) => value,
        None => return error(StatusCode::UNAUTHORIZED, "upload_token_required"),
    };
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    let owner = auth_ctx
        .claims
        .as_ref()
        .and_then(|claims| claims.sub.as_deref())
        .unwrap_or("local");
    match crate::application::creative::ingest::accept_upload_bytes(
        cwd,
        &state.config,
        owner,
        &ticket_id,
        token,
        content_type,
        &body,
    ) {
        Ok(receipt) => (StatusCode::CREATED, Json(json!({"upload": receipt}))).into_response(),
        Err(crate::core::error::McpError::InvalidRequest(message)) => {
            tracing::warn!(event = "relay.creative_upload", outcome = "rejected");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"upload_rejected","message":message})),
            )
                .into_response()
        }
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "upload_failed"),
    }
}

fn error(status: StatusCode, code: &str) -> AxumResponse {
    (status, Json(json!({"error": code}))).into_response()
}
