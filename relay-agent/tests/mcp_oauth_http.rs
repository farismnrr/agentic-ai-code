use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
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
use tower::ServiceExt;
use url::Url;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const ISSUER: &str = "https://sso.farismnrr.com";
const CLIENT_ID: &str = "relay-agent";

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
                "https://relay.example.com/mcp".to_string(),
            )
            .expect("MCP verifier"),
        ),
        ProtectedResourceMetadata::new(
            "https://relay.example.com/mcp".to_string(),
            ISSUER.to_string(),
        ),
        ProtectedResourceMetadata::new("https://relay.example.com".to_string(), ISSUER.to_string()),
        "https://relay.example.com/.well-known/oauth-protected-resource/mcp".to_string(),
    ))
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("JSON")
}

#[tokio::test]
async fn protected_resource_metadata_is_path_specific_for_mcp() {
    let response = router()
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource/mcp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["resource"], "https://relay.example.com/mcp");
    assert_eq!(body["authorization_servers"][0], ISSUER);
    assert_eq!(body["scopes_supported"][0], "identity.read");
}

#[tokio::test]
async fn root_protected_resource_metadata_remains_root_scoped() {
    let response = router()
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["resource"], "https://relay.example.com");
    assert_eq!(body["authorization_servers"][0], ISSUER);
}

#[tokio::test]
async fn auth_challenge_advertises_path_specific_metadata() {
    let response = router()
        .oneshot(
            Request::post("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_profile","arguments":{}}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let challenge = body["result"]["_meta"]["mcp/www_authenticate"][0]
        .as_str()
        .expect("auth challenge");
    assert!(challenge.contains(
        r#"resource_metadata="https://relay.example.com/.well-known/oauth-protected-resource/mcp""#
    ));
    assert!(challenge.contains(r#"scope="identity.read""#));
}
