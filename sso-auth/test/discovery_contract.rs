use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    application::{
        AuthError, McpAuthorizationRequest, McpClientMetadata, McpClientMetadataResolver,
        McpOAuthService, McpTokenRequest, StateGenerator,
    },
    domain::AuthenticatedUser,
    infrastructure::mcp_oauth::SignedMcpAccessTokenIssuer,
    interfaces::http::OAuthServerMetadata,
};

const SECRET: &str = "0123456789abcdef0123456789abcdef";

struct FixedState;

impl StateGenerator for FixedState {
    fn generate(&self) -> String {
        "fixed-authorization-code".to_string()
    }
}

struct ContractMetadata {
    client_id: String,
    redirect_uri: String,
}

#[async_trait]
impl McpClientMetadataResolver for ContractMetadata {
    async fn resolve(&self, _client_id: &str) -> Result<McpClientMetadata, AuthError> {
        Ok(McpClientMetadata {
            client_id: self.client_id.clone(),
            redirect_uris: vec![self.redirect_uri.clone()],
            token_endpoint_auth_methods_supported: vec!["none".to_string()],
        })
    }
}

fn contract() -> Value {
    serde_json::from_str(include_str!("../../contracts/chatgpt-discovery.json"))
        .expect("shared discovery contract")
}

#[tokio::test]
async fn sso_matches_shared_chatgpt_discovery_contract() {
    let contract = contract();
    let issuer = contract["issuer"].as_str().expect("issuer");
    let resource = contract["resource"].as_str().expect("resource");
    let scope = contract["scope"].as_str().expect("scope");
    let client_id = contract["client_id"].as_str().expect("client ID");
    let redirect_uri = contract["redirect_uri"].as_str().expect("redirect URI");

    let metadata = serde_json::to_value(OAuthServerMetadata::new(issuer))
        .expect("authorization server metadata");
    assert_eq!(metadata["issuer"], issuer);
    assert_eq!(
        metadata["authorization_endpoint"],
        format!("{issuer}/oauth/authorize")
    );
    assert_eq!(metadata["token_endpoint"], format!("{issuer}/oauth/token"));
    assert_eq!(
        metadata["authorization_response_iss_parameter_supported"],
        json!(true)
    );
    assert_eq!(
        metadata["client_id_metadata_document_supported"],
        json!(true)
    );
    assert_eq!(
        metadata["token_endpoint_auth_methods_supported"],
        json!(["none"])
    );
    assert_eq!(
        metadata["code_challenge_methods_supported"],
        json!(["S256"])
    );
    assert_eq!(metadata["scopes_supported"], json!([scope]));

    let token_issuer = Arc::new(
        SignedMcpAccessTokenIssuer::new(SECRET.to_string(), issuer.to_string())
            .expect("token issuer"),
    );
    let oauth = McpOAuthService::new(
        Arc::new(FixedState),
        token_issuer,
        Arc::new(ContractMetadata {
            client_id: client_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
        }),
        resource.to_string(),
    );

    let client = oauth
        .validate_client(client_id, redirect_uri)
        .await
        .expect("CIMD client");
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let authorization = oauth
        .validate_authorization_request(
            McpAuthorizationRequest {
                client_id: client_id.to_string(),
                redirect_uri: redirect_uri.to_string(),
                code_challenge: challenge,
                code_challenge_method: "S256".to_string(),
                scope: scope.to_string(),
                state: "chatgpt-state".to_string(),
                resource: resource.to_string(),
            },
            &client,
        )
        .expect("authorization request");

    let code = oauth
        .issue_code(
            AuthenticatedUser {
                id: 1,
                login: "faris".to_string(),
                avatar_url: None,
            },
            authorization,
        )
        .expect("authorization code");
    let token = oauth
        .exchange(McpTokenRequest {
            grant_type: "authorization_code".to_string(),
            code,
            redirect_uri: redirect_uri.to_string(),
            client_id: client_id.to_string(),
            code_verifier: verifier.to_string(),
            resource: resource.to_string(),
        })
        .expect("token exchange");

    assert_eq!(token.scope, scope);
    let (payload, _) = token.access_token.split_once('.').expect("signed token");
    let claims: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(payload)
            .expect("access token payload"),
    )
    .expect("access token claims");
    assert_eq!(claims["iss"], issuer);
    assert_eq!(claims["aud"], resource);
    assert_eq!(claims["scope"], scope);
}
