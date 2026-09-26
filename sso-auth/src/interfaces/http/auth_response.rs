use axum::{
    http::{header::CACHE_CONTROL, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use url::Url;

use crate::application::ConnectionHandoff;

use super::{
    auth_cookie::{
        append_set_cookie, clear_cookie, session_cookie, CONNECTED_APP_CLIENT_COOKIE,
        MCP_OAUTH_RETURN_COOKIE, OAUTH_STATE_COOKIE, RELAY_CONNECTION_STATE_COOKIE,
    },
    AuthHttpState,
};

pub(super) fn connection_redirect(
    state: &AuthHttpState,
    session_token: String,
    handoff: ConnectionHandoff,
) -> Response {
    let Ok(mut url) = Url::parse(&handoff.callback_url) else {
        return callback_redirect(state, false, None);
    };
    url.query_pairs_mut()
        .append_pair("assertion", &handoff.assertion);

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

pub(super) fn oauth_resume_redirect(
    state: &AuthHttpState,
    return_to: &str,
    session_token: String,
) -> Response {
    let mut response = Redirect::to(return_to).into_response();
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

pub(super) fn callback_redirect(
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

pub(super) fn forbidden_response(state: &AuthHttpState) -> Response {
    let mut response = (StatusCode::FORBIDDEN, "Forbidden").into_response();
    clear_auth_flow_cookies(&mut response, state);
    no_store_response(response)
}

pub(super) fn clear_connection_cookies(response: &mut Response, state: &AuthHttpState) {
    for name in [CONNECTED_APP_CLIENT_COOKIE, RELAY_CONNECTION_STATE_COOKIE] {
        append_set_cookie(
            response.headers_mut(),
            clear_cookie(name, "/auth/github", state.cookie_secure),
        );
    }
}

pub(super) fn no_store(status: StatusCode, message: &'static str) -> Response {
    no_store_response((status, message).into_response())
}

fn clear_auth_flow_cookies(response: &mut Response, state: &AuthHttpState) {
    for name in [OAUTH_STATE_COOKIE, MCP_OAUTH_RETURN_COOKIE] {
        append_set_cookie(
            response.headers_mut(),
            clear_cookie(name, "/auth/github", state.cookie_secure),
        );
    }
    clear_connection_cookies(response, state);
}

fn no_store_response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
