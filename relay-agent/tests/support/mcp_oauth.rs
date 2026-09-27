use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{body::Body, http::Request, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use relay_agent::{
    application::{
        ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase, TokenGenerator,
    },
    infrastructure::{
        connection_store::InMemoryConnectionRepository, mcp_token::SignedMcpAccessTokenVerifier,
        sso::SsoConnectUrlBuilder, verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{
        build_router, DiscoveryDocument, ProtectedResourceMetadata, RelayHttpState,
    },
};
use serde_json::{json, Value};
use sha2::Sha256;
use url::Url;

pub const ISSUER: &str = "https://sso.farismnrr.com";
pub const RESOURCE: &str = "https://relay.farismnrr.com/mcp";
pub const PROTOCOL: &str = "2026-07-28";

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const CLIENT_ID: &str = "relay-agent";

struct PanicTokens;

impl TokenGenerator for PanicTokens {
    fn generate(&self) -> String {
        panic!("token generation is not used by MCP OAuth tests")
    }
}

pub fn router() -> Router {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    let verifier = Arc::new(
        SignedRelayAssertionVerifier::new(
            SECRET.to_string(),
            ISSUER.to_string(),
            CLIENT_ID.to_string(),
        )
        .expect("verifier"),
    );
    let sso = Arc::new(
        SsoConnectUrlBuilder::new(Url::parse(ISSUER).expect("SSO URL"), CLIENT_ID.to_string())
            .expect("connect URL"),
    );

    build_router(RelayHttpState::new(
        Arc::new(ConnectStartUseCase::new(
            store.clone(),
            Arc::new(PanicTokens),
            sso,
        )),
        Arc::new(ConnectCallbackUseCase::new(verifier, store.clone())),
        Arc::new(ConnectionStatusUseCase::new(store)),
        DiscoveryDocument::new(
            CLIENT_ID.to_string(),
            "https://relay.farismnrr.com/connections/start".to_string(),
            "https://relay.farismnrr.com/connections/{connectionId}".to_string(),
        ),
        Url::parse(ISSUER).expect("SSO dashboard URL"),
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
            "https://relay.farismnrr.com".to_string(),
            ISSUER.to_string(),
        ),
        "https://relay.farismnrr.com/.well-known/oauth-protected-resource/mcp".to_string(),
    ))
}

pub fn mcp_request(method: &str, params: Value, authenticated: bool) -> Request<Body> {
    let token = authenticated.then(|| access_token("identity.read"));
    build_request(method, params, token.as_deref(), PROTOCOL)
}

#[allow(dead_code)]
pub fn mcp_request_with_scope(method: &str, params: Value, scope: &str) -> Request<Body> {
    let token = access_token(scope);
    build_request(method, params, Some(&token), PROTOCOL)
}

#[allow(dead_code)]
pub fn mcp_request_with_protocol(method: &str, params: Value, protocol: &str) -> Request<Body> {
    let token = access_token("identity.read");
    build_request(method, params, Some(&token), protocol)
}

fn build_request(
    method: &str,
    params: Value,
    token: Option<&str>,
    protocol: &str,
) -> Request<Body> {
    let mut builder = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol)
        .header("mcp-method", method);
    if method == "tools/call" {
        builder = builder.header("mcp-name", "get_profile");
    }
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }

    let mut params = params.as_object().cloned().unwrap_or_default();
    params.insert(
        "_meta".to_string(),
        json!({
            "io.modelcontextprotocol/protocolVersion": protocol,
            "io.modelcontextprotocol/clientCapabilities": {}
        }),
    );
    builder
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params
            })
            .to_string(),
        ))
        .unwrap()
}

fn access_token(scope: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let payload = serde_json::to_vec(&json!({
        "typ": "mcp_access",
        "iss": ISSUER,
        "aud": RESOURCE,
        "sub": "github:1",
        "login": "faris",
        "avatar_url": null,
        "scope": scope,
        "iat": now,
        "exp": now + 3600
    }))
    .unwrap();
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(&payload);
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    )
}
