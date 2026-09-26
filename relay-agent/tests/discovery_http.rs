use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    body::{to_bytes, Body},
    http::{header::LOCATION, Request, StatusCode},
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
const STATE: &str = "0123456789abcdef0123456789abcdef0123456789a";

struct FixedTokens {
    values: Mutex<VecDeque<String>>,
}

impl FixedTokens {
    fn new(values: Vec<String>) -> Self {
        Self {
            values: Mutex::new(values.into()),
        }
    }
}

impl TokenGenerator for FixedTokens {
    fn generate(&self) -> String {
        self.values
            .lock()
            .expect("token lock")
            .pop_front()
            .expect("fixed token")
    }
}

fn router(tokens: Vec<String>) -> Router {
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
            Arc::new(FixedTokens::new(tokens)),
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

#[tokio::test]
async fn mcp_protected_resource_metadata_is_path_specific() {
    let response = router(vec![])
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource/mcp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(body["resource"], "https://relay.example.com/mcp");
    assert_eq!(body["authorization_servers"][0], ISSUER);
    assert_eq!(body["scopes_supported"][0], "identity.read");
}

#[tokio::test]
async fn root_protected_resource_metadata_remains_root_scoped() {
    let response = router(vec![])
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(body["resource"], "https://relay.example.com");
    assert_eq!(body["authorization_servers"][0], ISSUER);
}

#[tokio::test]
async fn mcp_auth_challenge_advertises_path_specific_metadata() {
    let response = router(vec![])
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
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    let challenge = body["result"]["_meta"]["mcp/www_authenticate"][0]
        .as_str()
        .expect("auth challenge");
    assert!(challenge.contains(
        r#"resource_metadata="https://relay.example.com/.well-known/oauth-protected-resource/mcp""#
    ));
    assert!(challenge.contains(r#"scope="identity.read""#));
}

#[tokio::test]
async fn discovery_exposes_connect_contract_without_tools() {
    let response = router(vec![])
        .oneshot(
            Request::get("/.well-known/relay.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(body["id"], CLIENT_ID);
    assert_eq!(body["connection"]["type"], "sso");
    assert_eq!(
        body["connection"]["connectUrl"],
        "https://relay.example.com/connections/start"
    );
    assert!(body.get("tools").is_none());
}

#[tokio::test]
async fn connect_start_uses_registered_client_and_relay_state() {
    let response = router(vec!["connection-id".to_string(), STATE.to_string()])
        .oneshot(
            Request::get("/connections/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = Url::parse(
        response
            .headers()
            .get(LOCATION)
            .expect("location")
            .to_str()
            .expect("location string"),
    )
    .expect("redirect URL");
    assert_eq!(location.path(), "/connect");
    assert_eq!(
        location
            .query_pairs()
            .find(|(key, _)| key == "client_id")
            .map(|(_, value)| value.into_owned()),
        Some(CLIENT_ID.to_string())
    );
    assert_eq!(
        location
            .query_pairs()
            .find(|(key, _)| key == "connection_state")
            .map(|(_, value)| value.into_owned()),
        Some(STATE.to_string())
    );
    assert!(location.query_pairs().all(|(key, _)| key != "return_to"));
}
