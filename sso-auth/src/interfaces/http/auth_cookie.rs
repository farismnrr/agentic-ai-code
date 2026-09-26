use axum::http::{
    header::{COOKIE, SET_COOKIE},
    HeaderMap, HeaderValue,
};

pub(super) const OAUTH_STATE_COOKIE: &str = "sso_oauth_state";
pub(super) const RELAY_CONNECTION_STATE_COOKIE: &str = "sso_relay_connection_state";
pub(super) const CONNECTED_APP_CLIENT_COOKIE: &str = "sso_connected_app_client";
pub(super) const SESSION_COOKIE: &str = "sso_session";
const STATE_MAX_AGE_SECONDS: u64 = 600;

pub(super) fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|item| {
        let (key, value) = item.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

pub(super) fn state_cookie(value: &str, secure: bool) -> String {
    build_cookie(
        OAUTH_STATE_COOKIE,
        value,
        "/auth/github",
        STATE_MAX_AGE_SECONDS,
        secure,
    )
}

pub(super) fn relay_connection_state_cookie(value: &str, secure: bool) -> String {
    build_cookie(
        RELAY_CONNECTION_STATE_COOKIE,
        value,
        "/auth/github",
        STATE_MAX_AGE_SECONDS,
        secure,
    )
}

pub(super) fn connected_app_client_cookie(value: &str, secure: bool) -> String {
    build_cookie(
        CONNECTED_APP_CLIENT_COOKIE,
        value,
        "/auth/github",
        STATE_MAX_AGE_SECONDS,
        secure,
    )
}

pub(super) fn session_cookie(value: &str, max_age: u64, secure: bool) -> String {
    build_cookie(SESSION_COOKIE, value, "/", max_age, secure)
}

fn build_cookie(name: &str, value: &str, path: &str, max_age: u64, secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; Path={path}; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_attribute}"
    )
}

pub(super) fn clear_cookie(name: &str, path: &str, secure: bool) -> String {
    let secure_attribute = if secure { "; Secure" } else { "" };
    format!("{name}=; Path={path}; HttpOnly; SameSite=Lax; Max-Age=0{secure_attribute}")
}

pub(super) fn append_set_cookie(headers: &mut HeaderMap, value: String) {
    if let Ok(value) = HeaderValue::from_str(&value) {
        headers.append(SET_COOKIE, value);
    }
}

pub(super) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let diff = left
        .iter()
        .zip(right)
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b));
    diff == 0
}
