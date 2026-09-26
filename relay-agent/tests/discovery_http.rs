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
        connection_store::InMemoryConnectionRepository, sso::SsoConnectUrlBuilder,
        verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{build_router, DiscoveryDocument, RelayHttpState},
};
use tower::ServiceExt;
use url::Url;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const ISSUER: &str = "https://sso.farismnrr.com";
const AUDIENCE: &str = "relay-agent";
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
            AUDIENCE.to_string(),
        )
        .expect("verifier"),
    );
    let sso = Arc::new(
        SsoConnectUrlBuilder::new(Url::parse(ISSUER).expect("SSO URL"))
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
            "https://relay.example.com/connections/start".to_string(),
            "https://relay.example.com/connections/{connectionId}".to_string(),
        ),
    ))
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
    assert_eq!(body["connection"]["type"], "sso");
    assert_eq!(
        body["connection"]["connectUrl"],
        "https://relay.example.com/connections/start"
    );
    assert!(body.get("tools").is_none());
}

#[tokio::test]
async fn connect_start_uses_relay_state_and_no_return_target() {
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
    assert_eq!(location.path(), "/auth/github");
    assert_eq!(
        location
            .query_pairs()
            .find(|(key, _)| key == "connection_state")
            .map(|(_, value)| value.into_owned()),
        Some(STATE.to_string())
    );
    assert!(location.query_pairs().all(|(key, _)| key != "return_to"));
}
