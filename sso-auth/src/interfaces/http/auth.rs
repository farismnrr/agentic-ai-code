use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{
        header::CACHE_CONTROL,
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{AuthError, AuthService},
    domain::AuthenticatedUser,
};

use super::auth_cookie::{
    append_set_cookie, clear_cookie, constant_time_eq, cookie_value, session_cookie, state_cookie,
    OAUTH_STATE_COOKIE, SESSION_COOKIE,
};

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
    append_set_cookie(
        response.headers_mut(),
        state_cookie(&login.state, state.cookie_secure),
    );
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
        && query
            .state
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        && cookie_state
            .as_deref()
            .zip(query.state.as_deref())
            .is_some_and(|(cookie, returned)| {
                constant_time_eq(cookie.as_bytes(), returned.as_bytes())
            });

    if !valid_input {
        return callback_redirect(&state, false, None);
    }

    match state
        .auth
        .complete_login(query.code.as_deref().unwrap_or_default())
        .await
    {
        Ok(token) => callback_redirect(&state, true, Some(token)),
        Err(AuthError::Forbidden) => forbidden_response(&state),
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

fn callback_redirect(
    state: &AuthHttpState,
    success: bool,
    session_token: Option<String>,
) -> Response {
    let mut response =
        Redirect::to(if success { "/" } else { "/?auth_error=github" }).into_response();
    append_set_cookie(
        response.headers_mut(),
        clear_cookie(OAUTH_STATE_COOKIE, "/auth/github", state.cookie_secure),
    );
    if let Some(token) = session_token {
        append_set_cookie(
            response.headers_mut(),
            session_cookie(
                &token,
                state.session_ttl_seconds,
                state.cookie_secure,
            ),
        );
    }
    response
}

fn forbidden_response(state: &AuthHttpState) -> Response {
    let mut response = (StatusCode::FORBIDDEN, "Forbidden").into_response();
    append_set_cookie(
        response.headers_mut(),
        clear_cookie(OAUTH_STATE_COOKIE, "/auth/github", state.cookie_secure),
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

