use std::sync::Arc;

use async_trait::async_trait;
use axum::{body::Body, http::Request, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use relay_agent::{
    application::{
        ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase, TokenGenerator,
    },
    infrastructure::{
        connection_store::InMemoryConnectionRepository, mcp_token::SignedMcpAccessTokenVerifier,
        sso::SsoConnectUrlBuilder, verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{
        build_router as build_relay_router, DiscoveryDocument, ProtectedResourceMetadata,
        RelayHttpState,
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    application::{
        AuthError, McpClientMetadata, McpClientMetadataResolver, McpOAuthService, StateGenerator,
    },
    infrastructure::mcp_oauth::SignedMcpAccessTokenIssuer,
};

pub(super) const ISSUER: &str = "https://sso.farismnrr.com";
pub(super) const RESOURCE: &str = "https://relay.example.com/mcp";
pub(super) const CLIENT_ID: &str = "https://chatgpt.com/oauth/client.json";
pub(super) const REDIRECT_URI: &str = "https://chatgpt.com/connector_platform_oauth_redirect";
pub(super) const SCOPE: &str = "identity.read";
pub(super) const PROTOCOL: &str = "2026-07-28";
pub(super) const SECRET: &str = "0123456789abcdef0123456789abcdef";

struct FixedState;

impl StateGenerator for FixedState {
    fn generate(&self) -> String {
        "fixed-authorization-code".to_string()
    }
}

struct ChatGptMetadata;

#[async_trait]
impl McpClientMetadataResolver for ChatGptMetadata {
    async fn resolve(&self, client_id: &str) -> Result<McpClientMetadata, AuthError> {
        Ok(McpClientMetadata {
            client_id: client_id.to_string(),
            redirect_uris: vec![REDIRECT_URI.to_string()],
            token_endpoint_auth_methods_supported: vec!["none".to_string()],
        })
    }
}

struct PanicTokens;

impl TokenGenerator for PanicTokens {
    fn generate(&self) -> String {
        panic!("connection token generation is unused in the discovery contract")
    }
}

pub(super) fn oauth_service() -> McpOAuthService {
    let issuer = Arc::new(
        SignedMcpAccessTokenIssuer::new(SECRET.to_string(), ISSUER.to_string())
            .expect("token issuer"),
    );
    McpOAuthService::new(
        Arc::new(FixedState),
        issuer,
        Arc::new(ChatGptMetadata),
    )
}

pub(super) fn relay_router() -> Router {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    let assertion_verifier = Arc::new(
        SignedRelayAssertionVerifier::new(
            SECRET.to_string(),
            ISSUER.to_string(),
            "relay-agent".to_string(),
        )
        .expect("assertion verifier"),
    );
    let sso = Arc::new(
        SsoConnectUrlBuilder::new(
            Url::parse(ISSUER).expect("issuer URL"),
            "relay-agent".to_string(),
        )
        .expect("connect URL"),
    );

    build_relay_router(RelayHttpState::new(
        Arc::new(ConnectStartUseCase::new(
            store.clone(),
            Arc::new(PanicTokens),
            sso,
        )),
        Arc::new(ConnectCallbackUseCase::new(
            assertion_verifier,
            store.clone(),
        )),
        Arc::new(ConnectionStatusUseCase::new(store)),
        DiscoveryDocument::new(
            "relay-agent".to_string(),
            "https://relay.example.com/connections/start".to_string(),
            "https://relay.example.com/connections/{connectionId}".to_string(),
        ),
        Url::parse(ISSUER).expect("issuer URL"),
        Arc::new(
            SignedMcpAccessTokenVerifier::new(
                SECRET.to_string(),
                ISSUER.to_string(),
                RESOURCE.to_string(),
            )
            .expect("MCP verifier"),
        ),
        ProtectedResourceMetadata::new(RESOURCE.to_string(), ISSUER.to_string()),
        ProtectedResourceMetadata::new(
            "https://relay.example.com".to_string(),
            ISSUER.to_string(),
        ),
        "https://relay.example.com/.well-known/oauth-protected-resource/mcp".to_string(),
    ))
}

pub(super) fn mcp_request(method: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", PROTOCOL)
        .header("mcp-method", method);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }

    builder
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": {
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": PROTOCOL,
                        "io.modelcontextprotocol/clientCapabilities": {}
                    }
                }
            })
            .to_string(),
        ))
        .expect("MCP request")
}

pub(super) fn pkce(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(super) fn resource_metadata_url(challenge: &str) -> Option<&str> {
    challenge
        .split("resource_metadata=\"")
        .nth(1)?
        .split('"')
        .next()
}

pub(super) fn json_body(value: Value) -> Value {
    value
}
