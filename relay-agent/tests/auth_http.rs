use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{
        header::{ACCEPT, LOCATION, SET_COOKIE},
        Request, StatusCode,
    },
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use relay_agent::{
    application::{
        ConnectCallbackUseCase, ConnectStartUseCase, ConnectionRepository, ConnectionStatusUseCase,
        TokenGenerator,
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
const CLIENT_ID: &str = "relay-agent";
const STATE: &str = "0123456789abcdef0123456789abcdef0123456789a";

type HmacSha256 = Hmac<Sha256>;

struct PanicTokens;

impl TokenGenerator for PanicTokens {
    fn generate(&self) -> String {
        panic!("token generation is not used by callback tests")
    }
}

fn router(store: Arc<InMemoryConnectionRepository>) -> Router {
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
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8")
}

#[tokio::test]
async fn browser_callback_returns_to_sso_dashboard_after_connecting() {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    store
        .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
        .unwrap();
    let app = router(store);
    let token = assertion(STATE, CLIENT_ID, future_expiry());

    let response = app
        .oneshot(
            Request::get(format!("/connections/callback?assertion={token}"))
                .header(ACCEPT, "text/html,application/xhtml+xml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = Url::parse(
        response
            .headers()
            .get(LOCATION)
            .expect("location")
            .to_str()
            .expect("location string"),
    )
    .expect("redirect URL");
    assert_eq!(
        location.as_str(),
        "https://sso.farismnrr.com/?relay_connection=connected&connection_id=conn-1"
    );
}

#[tokio::test]
async fn valid_assertion_connects_without_relay_session() {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    store
        .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
        .unwrap();
    let app = router(store);
    let token = assertion(STATE, CLIENT_ID, future_expiry());

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
        .oneshot(
            Request::get("/connections/conn-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_assertions_are_rejected() {
    for token in [
        format!("{}x", assertion(STATE, CLIENT_ID, future_expiry())),
        assertion(STATE, "wrong-audience", future_expiry()),
        assertion(STATE, CLIENT_ID, 1),
    ] {
        let store = Arc::new(InMemoryConnectionRepository::new(300));
        store
            .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
            .unwrap();
        let response = router(store)
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
    let token = assertion("different-state", CLIENT_ID, future_expiry());
    let response = router(Arc::new(InMemoryConnectionRepository::new(300)))
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
    let response = router(Arc::new(InMemoryConnectionRepository::new(300)))
        .oneshot(
            Request::get("/connections/callback")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
