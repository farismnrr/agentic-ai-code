use axum::{
    extract::{Query, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::application::{AuthError, LoginCompletion};

use super::{
    auth_cookie::{
        append_set_cookie, clear_cookie, constant_time_eq, cookie_value,
        relay_connection_state_cookie, session_cookie, state_cookie, OAUTH_STATE_COOKIE,
        RELAY_CONNECTION_STATE_COOKIE,
    },
    AuthHttpState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    connection_state: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub async fn start(
    State(state): State<AuthHttpState>,
    Query(query): Query<StartQuery>,
) -> Response {
    let connection_state = match query.connection_state.as_deref() {
        Some(value) if valid_connection_state(value) => Some(value),
        Some(_) => return no_store(StatusCode::BAD_REQUEST, "invalid connection state"),
        None => None,
    };

    let login = state.auth.begin_login();
    let mut response = Redirect::temporary(&login.authorization_url).into_response();
    append_set_cookie(
        response.headers_mut(),
        state_cookie(&login.state, state.cookie_secure),
    );
    match connection_state {
        Some(value) => append_set_cookie(
            response.headers_mut(),
            relay_connection_state_cookie(value, state.cookie_secure),
        ),
        None => append_set_cookie(
            response.headers_mut(),
            clear_cookie(
                RELAY_CONNECTION_STATE_COOKIE,
                "/auth/github",
                state.cookie_secure,
            ),
        ),
    }
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

    let connection_state = cookie_value(&headers, RELAY_CONNECTION_STATE_COOKIE);
    match state
        .auth
        .complete_login(query.code.as_deref().unwrap_or_default())
        .await
    {
        Ok(completion) => complete_callback(&state, completion, connection_state),
        Err(AuthError::Forbidden) => forbidden_response(&state),
        Err(error) => {
            tracing::warn!(error = %error, "github oauth callback failed");
            callback_redirect(&state, false, None)
        }
    }
}

fn complete_callback(
    state: &AuthHttpState,
    completion: LoginCompletion,
    connection_state: Option<String>,
) -> Response {
    let Some(connection_state) = connection_state else {
        return callback_redirect(state, true, Some(completion.session_token));
    };
    if !valid_connection_state(&connection_state) {
        return callback_redirect(state, false, None);
    }

    match state
        .connections
        .issue_assertion(&completion.user, &connection_state)
    {
        Ok(assertion) => connection_redirect(state, completion.session_token, assertion),
        Err(error) => {
            tracing::warn!(error = %error, "relay connection assertion failed");
            callback_redirect(state, false, None)
        }
    }
}

fn connection_redirect(
    state: &AuthHttpState,
    session_token: String,
    assertion: String,
) -> Response {
    let mut url = state.relay_callback_url.clone();
    url.query_pairs_mut().append_pair("assertion", &assertion);
    let mut response = Redirect::to(url.as_str()).into_response();
    clear_auth_flow_cookies(&mut response, state);
    append_set_cookie(
        response.headers_mut(),
        session_cookie(
            &session_token,
            state.session_ttl_seconds,
            state.cookie_secure,
        ),
    );
    no_store_response(response)
}

fn callback_redirect(
    state: &AuthHttpState,
    success: bool,
    session_token: Option<String>,
) -> Response {
    let mut response =
        Redirect::to(if success { "/" } else { "/?auth_error=github" }).into_response();
    clear_auth_flow_cookies(&mut response, state);
    if let Some(token) = session_token {
        append_set_cookie(
            response.headers_mut(),
            session_cookie(&token, state.session_ttl_seconds, state.cookie_secure),
        );
    }
    no_store_response(response)
}

fn forbidden_response(state: &AuthHttpState) -> Response {
    let mut response = (StatusCode::FORBIDDEN, "Forbidden").into_response();
    clear_auth_flow_cookies(&mut response, state);
    no_store_response(response)
}

fn clear_auth_flow_cookies(response: &mut Response, state: &AuthHttpState) {
    for name in [OAUTH_STATE_COOKIE, RELAY_CONNECTION_STATE_COOKIE] {
        append_set_cookie(
            response.headers_mut(),
            clear_cookie(name, "/auth/github", state.cookie_secure),
        );
    }
}

fn valid_connection_state(value: &str) -> bool {
    (20..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn no_store(status: StatusCode, message: &'static str) -> Response {
    no_store_response((status, message).into_response())
}

fn no_store_response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
