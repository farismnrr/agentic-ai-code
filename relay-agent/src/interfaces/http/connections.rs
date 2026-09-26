use axum::{
    extract::{Path, Query, State},
    http::{
        header::{ACCEPT, CACHE_CONTROL},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;

use crate::application::AuthError;

use super::RelayHttpState;

#[derive(Deserialize)]
pub struct CallbackQuery {
    assertion: Option<String>,
}

pub async fn start(State(state): State<RelayHttpState>) -> Response {
    match state.start.execute() {
        Ok(start) => Redirect::temporary(&start.authorization_url).into_response(),
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connection could not be started",
        ),
    }
}

pub async fn callback(
    State(state): State<RelayHttpState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    match state.callback.execute(query.assertion.as_deref()) {
        Ok(connection) if accepts_html(&headers) => {
            let mut url = state.sso_dashboard_url.clone();
            url.set_path("/");
            url.set_query(None);
            url.set_fragment(None);
            url.query_pairs_mut()
                .append_pair("relay_connection", "connected")
                .append_pair("connection_id", &connection.id);
            no_store_redirect(url.as_str())
        }
        Ok(connection) => no_store_json(StatusCode::OK, connection),
        Err(_) if accepts_html(&headers) => browser_failure_redirect(&state),
        Err(AuthError::MissingAssertion) => {
            no_store(StatusCode::BAD_REQUEST, "callback assertion is required")
        }
        Err(AuthError::InvalidAssertion | AuthError::ConnectionStateMismatch) => {
            no_store(StatusCode::UNAUTHORIZED, "connection assertion is invalid")
        }
        Err(AuthError::ConnectionNotFound) => {
            no_store(StatusCode::NOT_FOUND, "connection was not found")
        }
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connection could not be completed",
        ),
    }
}

pub async fn status(State(state): State<RelayHttpState>, Path(id): Path<String>) -> Response {
    match state.status.execute(&id) {
        Ok(connection) => no_store_json(StatusCode::OK, connection),
        Err(AuthError::ConnectionNotFound) => {
            no_store(StatusCode::NOT_FOUND, "connection was not found")
        }
        Err(_) => no_store(
            StatusCode::SERVICE_UNAVAILABLE,
            "connection status is unavailable",
        ),
    }
}

fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().starts_with("text/html"))
        })
}

fn browser_failure_redirect(state: &RelayHttpState) -> Response {
    let mut url = state.sso_dashboard_url.clone();
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut()
        .append_pair("relay_connection", "failed");
    no_store_redirect(url.as_str())
}

fn no_store_redirect(location: &str) -> Response {
    let mut response = Redirect::to(location).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
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
