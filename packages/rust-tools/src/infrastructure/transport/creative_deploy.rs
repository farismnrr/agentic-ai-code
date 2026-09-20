use super::{AppState, AuthContext};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    Extension,
};
use std::sync::Arc;

pub(super) async fn handle(
    State(state): State<Arc<AppState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((deployment_id, path)): Path<(String, String)>,
) -> AxumResponse {
    if !state.config.enable_creative {
        return StatusCode::NOT_FOUND.into_response();
    }
    let owner = auth_ctx
        .claims
        .as_ref()
        .and_then(|claims| claims.sub.as_deref())
        .unwrap_or("local");
    let file = match crate::application::creative::deployment::read_deployed_file(
        &state.config,
        owner,
        &deployment_id,
        &path,
    ) {
        Ok(file) => file,
        Err(crate::core::error::McpError::InvalidRequest(_)) => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut response = AxumResponse::new(Body::from(file.bytes));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(file.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; media-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response
}
