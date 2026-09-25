use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{
        header::{CACHE_CONTROL, COOKIE, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::AuthService,
    domain::AuthenticatedUser,
};

const OAUTH_STATE_COOKIE: &str = "sso_oauth_state";
const SESSION_COOKIE: &str = "sso_session";
const STATE_MAX_AGE_SECONDS: u64 = 600;

#[derive(Clone)]
pub struct AuthHttpState {
    pub auth: Arc<AuthService>,
    pub cookie_secure: bool,
    pub session_ttl_seconds: u64,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    authenticated: bool,
    user: Option<AuthenticatedUser>,
    issued_at: Option<u64>,
    expires_at: Option<u64>,
}

pub async fn start(State(state): State<AuthHttpState>) -> Response {
    let login = state.auth.begin_login();
    let mut response = Redirect::temporary(&login.authorization_url).into_response();
    append_set_cookie(response.headers_mut(), state_cookie(&login.state, state.cookie_secure));
    response
}

pub async fn callback(
    State(state): State<AuthHttpState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let cookie_state = cookie_value(&headers, OAUTH_STATE_COOKIE);
    let valid_input = query.error.is_none()
        && query.code.as_deref().is_some_and(|value| !value.is_empty())
        && query.state.as_deref().is_some_and(|value| !value.is_empty())
        && cookie_state
            .as_deref()
            .zip(query.state.as_deref())
            .is_some_and(|(cookie, returned)| constant_time_eq(cookie.as_bytes(), returned.as_bytes()));

    if !valid_input {
        return callback_redirect(&state, false, None);
    }

    match state.auth.complete_login(query.code.as_deref().unwrap_or_default()).await {
        Ok(token) => callback_redirect(&state, true, Some(token)),
        Err(error) => {
            tracing::warn!(error = %error, "github oauth callback failed");
            callback_redirect(&state, false, None)
        }
    }
}

pub async fn session(State(state): State<AuthHttpState>, headers: HeaderMap) -> Response {
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
            append_set_cookie(response.headers_mut(), clear_cookie(SESSION_COOKIE, "/", state.cookie_secure));
            response
        }
    }
}

pub async fn logout(State(state): State<AuthHttpState>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    append_set_cookie(response.headers_mut(), clear_cookie(SESSION_COOKIE, "/", state.cookie_secure));
    response
}

fn callback_redirect(state: &AuthHttpState, success: bool, session_token: Option<String>) -> Response {
    let mut response = Redirect::to(if success { "/" } else { "/?auth_error=github" }).into_response();
    append_set_cookie(
        response.headers_mut(),
        clear_cookie(OAUTH_STATE_COOKIE, "/auth/github", state.cookie_secure),
    );
    if let Some(token) = session_token {
        append_set_cookie(
            response.headers_mut(),
            build_cookie(SESSION_COOKIE, &token, "/", state.session_ttl_seconds, state.cookie_secure),
        );
    }
    response
}

fn session_response(body: SessionResponse) -> Response {
    let mut response = Json(body).into_response();
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|item| {
        let (key, value) = item.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn state_cookie(value: &str, secure: bool) -> String {
    build_cookie(OAUTH_STATE_COOKIE, value, "/auth/github", STATE_MAX_AGE_SECONDS, secure)
}

fn build_cookie(name: &str, value: &str, path: &str, max_age: u64, secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!("{name}={value}; Path={path}; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_attribute}")
}

fn clear_cookie(name: &str, path: &str, secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!("{name}=; Path={path}; HttpOnly; SameSite=Lax; Max-Age=0{secure_attribute}")
}

fn append_set_cookie(headers: &mut HeaderMap, value: String) {
    if let Ok(value) = HeaderValue::from_str(&value) {
        headers.append(SET_COOKIE, value);
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let diff = left.iter().zip(right).fold(0_u8, |acc, (a, b)| acc | (a ^ b));
    diff == 0
}
