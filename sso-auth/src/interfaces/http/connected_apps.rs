use axum::{
    extract::{Path, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{application::AuthError, domain::ConnectedApp};

use super::{
    auth_cookie::{cookie_value, SESSION_COOKIE},
    AuthHttpState,
};

pub async fn list(State(state): State<AuthHttpState>, headers: HeaderMap) -> Response {
    if !is_authenticated(&state, &headers) {
        return no_store(StatusCode::UNAUTHORIZED, "authentication required");
    }

    match state.connected_apps.list() {
        Ok(apps) => no_store_json(StatusCode::OK, apps),
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connected app registry is unavailable",
        ),
    }
}

pub async fn save(
    State(state): State<AuthHttpState>,
    Path(client_id): Path<String>,
    headers: HeaderMap,
    Json(app): Json<ConnectedApp>,
) -> Response {
    if !is_authenticated(&state, &headers) {
        return no_store(StatusCode::UNAUTHORIZED, "authentication required");
    }
    if client_id != app.client_id {
        return no_store(StatusCode::BAD_REQUEST, "client ID cannot change");
    }

    match state.connected_apps.save(app) {
        Ok(saved) => no_store_json(StatusCode::OK, saved),
        Err(AuthError::InvalidConnectedApp(message)) => {
            no_store(StatusCode::BAD_REQUEST, message)
        }
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connected app could not be saved",
        ),
    }
}

fn is_authenticated(state: &AuthHttpState, headers: &HeaderMap) -> bool {
    cookie_value(headers, SESSION_COOKIE)
        .and_then(|token| state.auth.read_session(&token).ok())
        .is_some()
}

fn no_store(status: StatusCode, message: &'static str) -> Response {
    let mut response = (status, message).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn no_store_json<T: serde::Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
