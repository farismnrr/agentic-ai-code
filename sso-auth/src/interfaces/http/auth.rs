use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::application::{AuthError, LoginCompletion};

use super::{
    auth_cookie::{
        append_set_cookie, clear_cookie, connected_app_client_cookie, constant_time_eq,
        cookie_value, decode_mcp_oauth_return, mcp_oauth_return_cookie,
        relay_connection_state_cookie, state_cookie, CONNECTED_APP_CLIENT_COOKIE,
        MCP_OAUTH_RETURN_COOKIE, OAUTH_STATE_COOKIE, RELAY_CONNECTION_STATE_COOKIE,
    },
    auth_response::{
        callback_redirect, clear_connection_cookies, connection_redirect, forbidden_response,
        no_store, oauth_resume_redirect,
    },
    AuthHttpState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    client_id: Option<String>,
    connection_state: Option<String>,
    return_to: Option<String>,
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
    let oauth_return = match query.return_to.as_deref() {
        Some(value)
            if query.client_id.is_none()
                && query.connection_state.is_none()
                && valid_oauth_return_path(value) =>
        {
            Some(value)
        }
        Some(_) => return no_store(StatusCode::BAD_REQUEST, "invalid OAuth return path"),
        None => None,
    };

    let connection = match (
        query.client_id.as_deref(),
        query.connection_state.as_deref(),
    ) {
        (None, None) => None,
        (Some(client_id), Some(connection_state)) if valid_connection_state(connection_state) => {
            if state.connections.authorize(client_id).is_err() {
                return no_store(StatusCode::BAD_REQUEST, "connected app is unavailable");
            }
            Some((client_id, connection_state))
        }
        _ => return no_store(StatusCode::BAD_REQUEST, "invalid connection request"),
    };

    let login = state.auth.begin_login();
    let mut response = Redirect::temporary(&login.authorization_url).into_response();
    append_set_cookie(
        response.headers_mut(),
        state_cookie(&login.state, state.cookie_secure),
    );

    match connection {
        Some((client_id, connection_state)) => {
            append_set_cookie(
                response.headers_mut(),
                connected_app_client_cookie(client_id, state.cookie_secure),
            );
            append_set_cookie(
                response.headers_mut(),
                relay_connection_state_cookie(connection_state, state.cookie_secure),
            );
        }
        None => clear_connection_cookies(&mut response, &state),
    }

    if let Some(return_to) = oauth_return {
        append_set_cookie(
            response.headers_mut(),
            mcp_oauth_return_cookie(return_to, state.cookie_secure),
        );
    } else {
        append_set_cookie(
            response.headers_mut(),
            clear_cookie(MCP_OAUTH_RETURN_COOKIE, "/auth/github", state.cookie_secure),
        );
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

    let client_id = cookie_value(&headers, CONNECTED_APP_CLIENT_COOKIE);
    let connection_state = cookie_value(&headers, RELAY_CONNECTION_STATE_COOKIE);
    let oauth_return = cookie_value(&headers, MCP_OAUTH_RETURN_COOKIE)
        .and_then(|value| decode_mcp_oauth_return(&value))
        .filter(|value| valid_oauth_return_path(value));

    match state
        .auth
        .complete_login(query.code.as_deref().unwrap_or_default())
        .await
    {
        Ok(completion) => complete_callback(
            &state,
            completion,
            client_id,
            connection_state,
            oauth_return,
        ),
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
    client_id: Option<String>,
    connection_state: Option<String>,
    oauth_return: Option<String>,
) -> Response {
    match (client_id, connection_state, oauth_return) {
        (None, None, Some(return_to)) => {
            oauth_resume_redirect(state, &return_to, completion.session_token)
        }
        (None, None, None) => callback_redirect(state, true, Some(completion.session_token)),
        (Some(client_id), Some(connection_state), None)
            if valid_connection_state(&connection_state) =>
        {
            match state
                .connections
                .issue_handoff(&completion.user, &client_id, &connection_state)
            {
                Ok(handoff) => connection_redirect(state, completion.session_token, handoff),
                Err(error) => {
                    tracing::warn!(error = %error, "connected app handoff failed");
                    callback_redirect(state, false, None)
                }
            }
        }
        _ => callback_redirect(state, false, None),
    }
}

fn valid_oauth_return_path(value: &str) -> bool {
    value.len() <= 2048 && value.starts_with("/oauth/authorize?")
}

fn valid_connection_state(value: &str) -> bool {
    (20..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
