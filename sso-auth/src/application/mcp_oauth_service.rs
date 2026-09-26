use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::domain::AuthenticatedUser;

use super::{AuthError, McpAccessTokenIssuer, McpClientMetadataResolver, StateGenerator};

mod validation;

use validation::{
    normalize_scope, now, pkce_matches, valid_challenge, valid_client_id_url, valid_resource,
    valid_verifier,
};

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

pub(crate) struct ValidatedMcpClient {
    client_id: String,
    redirect_uri: String,
}

pub(crate) struct ValidatedMcpAuthorizationRequest(McpAuthorizationRequest);

struct PendingCode {
    user: AuthenticatedUser,
    request: McpAuthorizationRequest,
    expires_at: u64,
}

pub struct McpOAuthService {
    codes: Mutex<HashMap<String, PendingCode>>,
    states: Arc<dyn StateGenerator>,
    tokens: Arc<dyn McpAccessTokenIssuer>,
    client_metadata: Arc<dyn McpClientMetadataResolver>,
    code_ttl_seconds: u64,
    access_ttl_seconds: u64,
}

impl McpOAuthService {
    pub fn new(
        states: Arc<dyn StateGenerator>,
        tokens: Arc<dyn McpAccessTokenIssuer>,
        client_metadata: Arc<dyn McpClientMetadataResolver>,
    ) -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
            states,
            tokens,
            client_metadata,
            code_ttl_seconds: 300,
            access_ttl_seconds: 3600,
        }
    }

    pub(crate) async fn validate_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<ValidatedMcpClient, AuthError> {
        if !valid_client_id_url(client_id) {
            return Err(AuthError::InvalidOAuthRequest);
        }

        let metadata = self.client_metadata.resolve(client_id).await?;
        let valid = metadata.client_id == client_id
            && metadata
                .redirect_uris
                .iter()
                .any(|candidate| candidate == redirect_uri)
            && metadata
                .token_endpoint_auth_methods_supported
                .iter()
                .any(|method| method == "none");
        if !valid {
            return Err(AuthError::InvalidOAuthRequest);
        }

        Ok(ValidatedMcpClient {
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
        })
    }

    pub(crate) fn validate_authorization_request(
        &self,
        request: McpAuthorizationRequest,
        client: &ValidatedMcpClient,
    ) -> Result<ValidatedMcpAuthorizationRequest, AuthError> {
        let valid = request.client_id == client.client_id
            && request.redirect_uri == client.redirect_uri
            && request.code_challenge_method == "S256"
            && !request.state.is_empty()
            && valid_challenge(&request.code_challenge)
            && normalize_scope(&request.scope).as_deref() == Some(MCP_SCOPE)
            && valid_resource(&request.resource);
        if !valid {
            return Err(AuthError::InvalidOAuthRequest);
        }

        Ok(ValidatedMcpAuthorizationRequest(request))
    }

    pub(crate) fn issue_code(
        &self,
        user: AuthenticatedUser,
        request: ValidatedMcpAuthorizationRequest,
    ) -> Result<String, AuthError> {
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
                request: request.0,
                expires_at: now.saturating_add(self.code_ttl_seconds),
            },
        );
        Ok(code)
    }

    pub fn exchange(&self, request: McpTokenRequest) -> Result<McpTokenResponse, AuthError> {
        if request.grant_type != "authorization_code" || !valid_verifier(&request.code_verifier) {
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
