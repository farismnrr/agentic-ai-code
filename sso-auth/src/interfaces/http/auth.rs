use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::application::{AuthError, LoginCompletion};

use super::{
    auth_cookie::{
        append_set_cookie, connected_app_client_cookie, constant_time_eq, cookie_value,
        relay_connection_state_cookie, state_cookie, CONNECTED_APP_CLIENT_COOKIE,
        OAUTH_STATE_COOKIE, RELAY_CONNECTION_STATE_COOKIE,
    },
    auth_response::{
        callback_redirect, clear_connection_cookies, connection_redirect, forbidden_response,
        no_store,
    },
    AuthHttpState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    client_id: Option<String>,
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
    let connection = match (query.client_id.as_deref(), query.connection_state.as_deref()) {
        (None, None) => None,
        (Some(client_id), Some(connection_state))
            if valid_connection_state(connection_state) =>
        {
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
    match state
        .auth
        .complete_login(query.code.as_deref().unwrap_or_default())
        .await
    {
        Ok(completion) => complete_callback(&state, completion, client_id, connection_state),
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
) -> Response {
    match (client_id, connection_state) {
        (None, None) => callback_redirect(state, true, Some(completion.session_token)),
        (Some(client_id), Some(connection_state))
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

fn valid_connection_state(value: &str) -> bool {
    (20..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
