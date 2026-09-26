use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use url::Url;

use crate::domain::AuthenticatedUser;

use super::{AuthError, McpAccessTokenIssuer, StateGenerator};

pub const CHATGPT_CLIENT_ID: &str = "https://chatgpt.com/oauth/client.json";
pub const CHATGPT_REDIRECT_URI: &str = "https://chatgpt.com/connector_platform_oauth_redirect";
pub const MCP_SCOPE: &str = "identity.read";

#[derive(Clone)]
pub struct McpAuthorizationRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    pub state: String,
    pub resource: String,
}

pub struct McpTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub code_verifier: String,
    pub resource: String,
}

pub struct McpTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub scope: String,
}

struct PendingCode {
    user: AuthenticatedUser,
    request: McpAuthorizationRequest,
    expires_at: u64,
}

pub struct McpOAuthService {
    codes: Mutex<HashMap<String, PendingCode>>,
    states: Arc<dyn StateGenerator>,
    tokens: Arc<dyn McpAccessTokenIssuer>,
    code_ttl_seconds: u64,
    access_ttl_seconds: u64,
}

impl McpOAuthService {
    pub fn new(states: Arc<dyn StateGenerator>, tokens: Arc<dyn McpAccessTokenIssuer>) -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
            states,
            tokens,
            code_ttl_seconds: 300,
            access_ttl_seconds: 3600,
        }
    }

    pub fn validate_authorization_request(
        &self,
        request: &McpAuthorizationRequest,
    ) -> Result<(), AuthError> {
        let valid = request.client_id == CHATGPT_CLIENT_ID
            && request.redirect_uri == CHATGPT_REDIRECT_URI
            && request.code_challenge_method == "S256"
            && !request.state.is_empty()
            && request.code_challenge.len() == 43
            && normalize_scope(&request.scope).as_deref() == Some(MCP_SCOPE)
            && valid_resource(&request.resource);
        if !valid {
            return Err(AuthError::InvalidOAuthRequest);
        }
        Ok(())
    }

    pub fn issue_code(
        &self,
        user: AuthenticatedUser,
        request: McpAuthorizationRequest,
    ) -> Result<String, AuthError> {
        self.validate_authorization_request(&request)?;
        let now = now()?;
        let code = self.states.generate();
        let mut codes = self
            .codes
            .lock()
            .map_err(|_| AuthError::StorageUnavailable)?;
        codes.retain(|_, item| item.expires_at > now);
        codes.insert(
            code.clone(),
            PendingCode {
                user,
                request,
                expires_at: now.saturating_add(self.code_ttl_seconds),
            },
        );
        Ok(code)
    }

    pub fn exchange(&self, request: McpTokenRequest) -> Result<McpTokenResponse, AuthError> {
        if request.grant_type != "authorization_code"
            || request.client_id != CHATGPT_CLIENT_ID
            || request.redirect_uri != CHATGPT_REDIRECT_URI
            || !valid_verifier(&request.code_verifier)
        {
            return Err(AuthError::InvalidAuthorizationCode);
        }

        let now = now()?;
        let mut codes = self
            .codes
            .lock()
            .map_err(|_| AuthError::StorageUnavailable)?;
        codes.retain(|_, item| item.expires_at > now);
        let pending = codes
            .get(&request.code)
            .ok_or(AuthError::InvalidAuthorizationCode)?;
        if pending.request.client_id != request.client_id
            || pending.request.redirect_uri != request.redirect_uri
            || pending.request.resource != request.resource
            || !pkce_matches(&request.code_verifier, &pending.request.code_challenge)
        {
            return Err(AuthError::InvalidAuthorizationCode);
        }

        let pending = codes
            .remove(&request.code)
            .ok_or(AuthError::InvalidAuthorizationCode)?;
        let scope =
            normalize_scope(&pending.request.scope).ok_or(AuthError::InvalidOAuthRequest)?;
        let access_token = self.tokens.issue(
            &pending.user,
            &pending.request.resource,
            &scope,
            self.access_ttl_seconds,
        )?;

        Ok(McpTokenResponse {
            access_token,
            expires_in: self.access_ttl_seconds,
            scope,
        })
    }
}

fn valid_resource(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && !url.cannot_be_a_base())
}

fn normalize_scope(value: &str) -> Option<String> {
    let mut scopes = value.split_whitespace();
    let first = scopes.next()?;
    (first == MCP_SCOPE && scopes.next().is_none()).then(|| first.to_string())
}

fn valid_verifier(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

fn pkce_matches(verifier: &str, expected: &str) -> bool {
    let actual = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    constant_time_eq(actual.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn now() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AuthError::InvalidAuthorizationCode)
}
