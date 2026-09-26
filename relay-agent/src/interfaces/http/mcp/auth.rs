use axum::{
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
};

use crate::domain::VerifiedPrincipal;

use super::super::{mcp_metadata::MCP_SCOPE, RelayHttpState};

pub(super) fn authenticate(
    state: &RelayHttpState,
    headers: &HeaderMap,
) -> Result<VerifiedPrincipal, Response> {
    let Some(token) = bearer_token(headers) else {
        return Err(oauth_challenge(state, false));
    };
    state
        .mcp_tokens
        .verify(token, MCP_SCOPE)
        .map_err(|_| oauth_challenge(state, true))
}

fn oauth_challenge(state: &RelayHttpState, invalid_token: bool) -> Response {
    let mut challenge = format!(
        "Bearer resource_metadata=\"{}\", scope=\"{}\"",
        state.mcp_resource_metadata_url, MCP_SCOPE
    );
    if invalid_token {
        challenge.push_str(", error=\"invalid_token\"");
    }

    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(WWW_AUTHENTICATE, value);
    }
    (StatusCode::UNAUTHORIZED, headers).into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty())
}
