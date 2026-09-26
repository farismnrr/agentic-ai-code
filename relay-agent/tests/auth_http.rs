use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{header::LOCATION, header::SET_COOKIE, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use relay_agent::{
    application::{
        ConnectCallbackUseCase, ConnectStartUseCase, ConnectionRepository,
        ConnectionStatusUseCase, TokenGenerator,
    },
    domain::Connection,
    infrastructure::{
        connection_store::InMemoryConnectionRepository, sso::SsoConnectUrlBuilder,
        verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{build_router, DiscoveryDocument, RelayHttpState},
};
use serde_json::json;
use sha2::Sha256;
use tower::ServiceExt;
use url::Url;

const SECRET: &str = "0123456789abcdef0123456789abcdef";
const ISSUER: &str = "https://sso.farismnrr.com";
const AUDIENCE: &str = "relay-agent";
const STATE: &str = "0123456789abcdef0123456789abcdef0123456789a";

type HmacSha256 = Hmac<Sha256>;

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

fn router(
    store: Arc<InMemoryConnectionRepository>,
    tokens: Vec<String>,
) -> Router {
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

fn assertion(state: &str, audience: &str, expires_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let payload = serde_json::to_vec(&json!({
        "iss": ISSUER,
        "aud": audience,
        "sub": "github:120432426",
        "login": "farismnrr",
        "avatar_url": null,
        "state": state,
        "iat": now,
        "exp": expires_at
    }))
    .expect("claims");
    let mut mac = HmacSha256::new_from_slice(SECRET.as_bytes()).expect("HMAC");
    mac.update(&payload);
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    )
}

fn future_expiry() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 90
}

async fn body_text(response: axum::response::Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body")
            .to_vec(),
    )
    .expect("UTF-8")
}

#[tokio::test]
async fn discovery_exposes_connect_contract_without_tools() {
    let app = router(Arc::new(InMemoryConnectionRepository::new(300)), vec![]);
    let response = app
        .oneshot(Request::get("/.well-known/relay.json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_str(&body_text(response).await).expect("JSON");
    assert_eq!(body["connection"]["type"], "sso");
    assert_eq!(
        body["connection"]["connectUrl"],
        "https://relay.example.com/connections/start"
    );
    assert!(body.get("tools").is_none());
}

#[tokio::test]
async fn connect_start_uses_relay_state_and_no_return_target() {
    let app = router(
        Arc::new(InMemoryConnectionRepository::new(300)),
        vec!["connection-id".to_string(), STATE.to_string()],
    );
    let response = app
        .oneshot(Request::get("/connections/start").body(Body::empty()).unwrap())
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

#[tokio::test]
async fn valid_assertion_connects_without_relay_session() {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    store
        .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
        .unwrap();
    let app = router(store, vec![]);
    let token = assertion(STATE, AUDIENCE, future_expiry());

    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/connections/callback?assertion={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(SET_COOKIE).is_none());
    let body = body_text(response).await;
    assert!(body.contains("\"status\":\"connected\""));
    assert!(body.contains("\"subject\":\"github:120432426\""));

    let status = app
        .oneshot(Request::get("/connections/conn-1").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(status.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_assertions_are_rejected() {
    for token in [
        format!("{}x", assertion(STATE, AUDIENCE, future_expiry())),
        assertion(STATE, "wrong-audience", future_expiry()),
        assertion(STATE, AUDIENCE, 1),
    ] {
        let store = Arc::new(InMemoryConnectionRepository::new(300));
        store
            .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
            .unwrap();
        let response = router(store, vec![])
            .oneshot(
                Request::get(format!("/connections/callback?assertion={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn callback_rejects_state_without_pending_connection() {
    let token = assertion("different-state", AUDIENCE, future_expiry());
    let response = router(Arc::new(InMemoryConnectionRepository::new(300)), vec![])
        .oneshot(
            Request::get(format!("/connections/callback?assertion={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn callback_requires_assertion() {
    let response = router(Arc::new(InMemoryConnectionRepository::new(300)), vec![])
        .oneshot(
            Request::get("/connections/callback")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
