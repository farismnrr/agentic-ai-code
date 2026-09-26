use axum::{
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
};

use crate::{application::AuthError, domain::VerifiedPrincipal};

use super::super::{mcp_metadata::MCP_SCOPE, RelayHttpState};

pub(super) fn authenticate(
    state: &RelayHttpState,
    headers: &HeaderMap,
) -> Result<VerifiedPrincipal, Response> {
    let Some(token) = bearer_token(headers) else {
        return Err(oauth_challenge(state, ChallengeKind::Missing));
    };
    match state.mcp_tokens.verify(token, MCP_SCOPE) {
        Ok(principal) => Ok(principal),
        Err(AuthError::InsufficientScope) => {
            Err(oauth_challenge(state, ChallengeKind::InsufficientScope))
        }
        Err(_) => Err(oauth_challenge(state, ChallengeKind::InvalidToken)),
    }
}

enum ChallengeKind {
    Missing,
    InvalidToken,
    InsufficientScope,
}

fn oauth_challenge(state: &RelayHttpState, kind: ChallengeKind) -> Response {
    let mut challenge = format!(
        "Bearer resource_metadata=\"{}\", scope=\"{}\"",
        state.mcp_resource_metadata_url, MCP_SCOPE
    );
    let status = match kind {
        ChallengeKind::Missing => StatusCode::UNAUTHORIZED,
        ChallengeKind::InvalidToken => {
            challenge.push_str(", error=\"invalid_token\"");
            StatusCode::UNAUTHORIZED
        }
        ChallengeKind::InsufficientScope => {
            challenge.push_str(", error=\"insufficient_scope\"");
            StatusCode::FORBIDDEN
        }
    };

    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(WWW_AUTHENTICATE, value);
    }
    (status, headers).into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty())
}
