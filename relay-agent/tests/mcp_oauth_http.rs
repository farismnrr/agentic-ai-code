use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{header::WWW_AUTHENTICATE, Request, StatusCode},
    Router,
};
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
use tower::ServiceExt;
use url::Url;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const ISSUER: &str = "https://sso.farismnrr.com";
const CLIENT_ID: &str = "relay-agent";
const RESOURCE: &str = "https://relay.example.com/mcp";
const PROTOCOL: &str = "2026-07-28";

struct PanicTokens;

impl TokenGenerator for PanicTokens {
    fn generate(&self) -> String {
        panic!("token generation is not used by MCP OAuth tests")
    }
}

fn router() -> Router {
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
            "https://relay.example.com/connections/start".to_string(),
            "https://relay.example.com/connections/{connectionId}".to_string(),
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
            "https://relay.example.com".to_string(),
            ISSUER.to_string(),
        ),
        "https://relay.example.com/.well-known/oauth-protected-resource/mcp".to_string(),
    ))
}

fn access_token() -> String {
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
        "scope": "identity.read",
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

fn mcp_request(method: &str, params: Value, authenticated: bool) -> Request<Body> {
    let mut builder = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", PROTOCOL)
        .header("mcp-method", method);
    if method == "tools/call" {
        builder = builder.header("mcp-name", "get_profile");
    }
    if authenticated {
        builder = builder.header("authorization", format!("Bearer {}", access_token()));
    }

    let mut params = params.as_object().cloned().unwrap_or_default();
    params.insert(
        "_meta".to_string(),
        json!({
            "io.modelcontextprotocol/protocolVersion": PROTOCOL,
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("JSON")
}

#[tokio::test]
async fn protected_resource_metadata_aliases_canonical_mcp_resource() {
    for path in [
        "/.well-known/oauth-protected-resource/mcp",
        "/.well-known/oauth-protected-resource",
    ] {
        let response = router()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["resource"], RESOURCE);
        assert_eq!(body["authorization_servers"][0], ISSUER);
        assert_eq!(body["scopes_supported"][0], "identity.read");
    }
}

#[tokio::test]
async fn unauthenticated_modern_request_returns_oauth_discovery_challenge() {
    let response = router()
        .oneshot(mcp_request("server/discover", json!({}), false))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let challenge = response
        .headers()
        .get(WWW_AUTHENTICATE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(challenge.contains(
        r#"resource_metadata="https://relay.example.com/.well-known/oauth-protected-resource/mcp""#
    ));
    assert!(challenge.contains(r#"scope="identity.read""#));
}

#[tokio::test]
async fn authenticated_discovery_advertises_only_latest_protocol() {
    let response = router()
        .oneshot(mcp_request("server/discover", json!({}), true))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["result"]["supportedVersions"], json!([PROTOCOL]));
    assert_eq!(body["result"]["resultType"], "complete");
}

#[tokio::test]
async fn header_body_mismatch_is_rejected_with_modern_error() {
    let request = mcp_request("tools/list", json!({}), true);
    let (mut parts, body) = request.into_parts();
    parts
        .headers
        .insert("mcp-method", "server/discover".parse().unwrap());
    let response = router()
        .oneshot(Request::from_parts(parts, body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn legacy_initialize_is_not_part_of_latest_protocol() {
    let response = router()
        .oneshot(mcp_request("initialize", json!({}), true))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], -32601);
}
