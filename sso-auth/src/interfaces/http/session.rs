use axum::{
    extract::State,
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::domain::AuthenticatedUser;

use super::{
    auth_cookie::{
        append_set_cookie, clear_cookie, cookie_value, SESSION_COOKIE,
    },
    AuthHttpState,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    authenticated: bool,
    user: Option<AuthenticatedUser>,
    issued_at: Option<u64>,
    expires_at: Option<u64>,
}

pub async fn get(State(state): State<AuthHttpState>, headers: HeaderMap) -> Response {
    let Some(token) = cookie_value(&headers, SESSION_COOKIE) else {
        return session_response(SessionResponse {
            authenticated: false,
            user: None,
            issued_at: None,
            expires_at: None,
        });
    };

    match state.auth.read_session(&token) {
        Ok(session) => session_response(SessionResponse {
            authenticated: true,
            user: Some(session.user),
            issued_at: Some(session.issued_at),
            expires_at: Some(session.expires_at),
        }),
        Err(_) => {
            let mut response = session_response(SessionResponse {
                authenticated: false,
                user: None,
                issued_at: None,
                expires_at: None,
            });
            append_set_cookie(
                response.headers_mut(),
                clear_cookie(SESSION_COOKIE, "/", state.cookie_secure),
            );
            response
        }
    }
}

pub async fn logout(State(state): State<AuthHttpState>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    append_set_cookie(
        response.headers_mut(),
        clear_cookie(SESSION_COOKIE, "/", state.cookie_secure),
    );
    response
}

fn session_response(body: SessionResponse) -> Response {
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
