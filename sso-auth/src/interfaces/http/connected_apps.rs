use axum::{
    extract::{Path, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{application::AuthError, domain::ConnectedApp};

use super::{
    auth_cookie::{cookie_value, SESSION_COOKIE},
    AuthHttpState,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionAppInfo {
    client_id: String,
    name: String,
    description: String,
}

pub async fn connection_info(
    State(state): State<AuthHttpState>,
    Path(client_id): Path<String>,
) -> Response {
    match state.connections.authorize(&client_id) {
        Ok(app) => no_store_json(
            StatusCode::OK,
            ConnectionAppInfo {
                client_id: app.client_id,
                name: app.name,
                description: app.description,
            },
        ),
        Err(AuthError::ConnectedAppNotFound | AuthError::ConnectedAppDisabled) => {
            no_store(StatusCode::NOT_FOUND, "connected app is unavailable")
        }
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connected app registry is unavailable",
        ),
    }
}

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
        Err(AuthError::InvalidConnectedApp(message)) => no_store(StatusCode::BAD_REQUEST, message),
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connected app could not be saved",
        ),
    }
}

pub async fn delete(
    State(state): State<AuthHttpState>,
    Path(client_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !is_authenticated(&state, &headers) {
        return no_store(StatusCode::UNAUTHORIZED, "authentication required");
    }

    match state.connected_apps.delete(&client_id) {
        Ok(true) => no_store_empty(StatusCode::NO_CONTENT),
        Ok(false) => no_store(StatusCode::NOT_FOUND, "connected app was not found"),
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connected app could not be disconnected",
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
    apply_no_store(&mut response);
    response
}

fn no_store_empty(status: StatusCode) -> Response {
    let mut response = status.into_response();
    apply_no_store(&mut response);
    response
}

fn no_store_json<T: serde::Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    apply_no_store(&mut response);
    response
}

fn apply_no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
}
