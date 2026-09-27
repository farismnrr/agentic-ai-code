use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    application::{
        AuthError, McpAccessTokenIssuer, McpAuthorizationRequest, McpClientMetadata,
        McpClientMetadataResolver, McpOAuthService, StateGenerator,
    },
    domain::AuthenticatedUser,
};

const CLIENT_ID: &str = "https://chatgpt.com/oauth/client.json";
const REDIRECT_URI: &str = "https://chatgpt.com/connector_platform_oauth_redirect";
const RESOURCE: &str = "https://relay.farismnrr.com/mcp";

struct FixedState;

impl StateGenerator for FixedState {
    fn generate(&self) -> String {
        "fixed-code".to_string()
    }
}

struct DummyTokens;

impl McpAccessTokenIssuer for DummyTokens {
    fn issue(
        &self,
        _user: &AuthenticatedUser,
        _audience: &str,
        _scope: &str,
        _ttl_seconds: u64,
    ) -> Result<String, AuthError> {
        Ok("token".to_string())
    }
}

struct FixedMetadata {
    client_id: String,
    redirect_uris: Vec<String>,
    methods: Vec<String>,
}

#[async_trait]
impl McpClientMetadataResolver for FixedMetadata {
    async fn resolve(&self, _client_id: &str) -> Result<McpClientMetadata, AuthError> {
        Ok(McpClientMetadata {
            client_id: self.client_id.clone(),
            redirect_uris: self.redirect_uris.clone(),
            token_endpoint_auth_methods_supported: self.methods.clone(),
        })
    }
}

fn service(metadata: FixedMetadata) -> McpOAuthService {
    McpOAuthService::new(
        Arc::new(FixedState),
        Arc::new(DummyTokens),
        Arc::new(metadata),
        RESOURCE.to_string(),
    )
}

fn valid_metadata() -> FixedMetadata {
    FixedMetadata {
        client_id: CLIENT_ID.to_string(),
        redirect_uris: vec![REDIRECT_URI.to_string()],
        methods: vec!["none".to_string(), "private_key_jwt".to_string()],
    }
}

#[tokio::test]
async fn cimd_client_validation_requires_exact_document_identity_and_redirect() {
    let service = service(valid_metadata());
    service
        .validate_client(CLIENT_ID, REDIRECT_URI)
        .await
        .expect("valid ChatGPT CIMD client");

    let invalid_redirect = service
        .validate_client(CLIENT_ID, "https://example.com/callback")
        .await;
    assert!(matches!(
        invalid_redirect,
        Err(AuthError::InvalidOAuthRequest)
    ));
}

#[tokio::test]
async fn cimd_client_validation_requires_supported_token_auth_intersection() {
    let service = service(FixedMetadata {
        client_id: CLIENT_ID.to_string(),
        redirect_uris: vec![REDIRECT_URI.to_string()],
        methods: vec!["private_key_jwt".to_string()],
    });

    let result = service.validate_client(CLIENT_ID, REDIRECT_URI).await;
    assert!(matches!(result, Err(AuthError::InvalidOAuthRequest)));
}

#[tokio::test]
async fn authorization_request_requires_pkce_scope_state_and_resource() {
    let service = service(valid_metadata());
    let client = service
        .validate_client(CLIENT_ID, REDIRECT_URI)
        .await
        .expect("validated client");
    let request = McpAuthorizationRequest {
        client_id: CLIENT_ID.to_string(),
        redirect_uri: REDIRECT_URI.to_string(),
        code_challenge: "a".repeat(43),
        code_challenge_method: "S256".to_string(),
        scope: "identity.read".to_string(),
        state: "state".to_string(),
        resource: RESOURCE.to_string(),
    };

    service
        .validate_authorization_request(request, &client)
        .expect("valid authorization request");
}


#[tokio::test]
async fn authorization_request_rejects_unknown_resource_target() {
    let service = service(valid_metadata());
    let client = service
        .validate_client(CLIENT_ID, REDIRECT_URI)
        .await
        .expect("validated client");
    let request = McpAuthorizationRequest {
        client_id: CLIENT_ID.to_string(),
        redirect_uri: REDIRECT_URI.to_string(),
        code_challenge: "a".repeat(43),
        code_challenge_method: "S256".to_string(),
        scope: "identity.read".to_string(),
        state: "state".to_string(),
        resource: "https://evil.example/mcp".to_string(),
    };

    let result = service.validate_authorization_request(request, &client);
    assert!(matches!(result, Err(AuthError::InvalidOAuthTarget)));
}
