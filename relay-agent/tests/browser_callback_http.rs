use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    http::{
        header::{ACCEPT, LOCATION},
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
        connection_store::InMemoryConnectionRepository, mcp_token::SignedMcpAccessTokenVerifier,
        sso::SsoConnectUrlBuilder, verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{
        build_router, DiscoveryDocument, ProtectedResourceMetadata, RelayHttpState,
    },
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
        Arc::new(
            SignedMcpAccessTokenVerifier::new(
                SECRET.to_string(),
                ISSUER.to_string(),
                "https://relay.example.com".to_string(),
            )
            .expect("MCP verifier"),
        ),
        ProtectedResourceMetadata::new("https://relay.example.com".to_string(), ISSUER.to_string()),
    ))
}

fn assertion(state: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let payload = serde_json::to_vec(&json!({
        "iss": ISSUER,
        "aud": CLIENT_ID,
        "sub": "github:120432426",
        "login": "farismnrr",
        "avatar_url": null,
        "state": state,
        "iat": now,
        "exp": now + 90
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

#[tokio::test]
async fn browser_callback_returns_to_sso_dashboard_after_connecting() {
    let store = Arc::new(InMemoryConnectionRepository::new(300));
    store
        .create(Connection::pending("conn-1".to_string(), STATE.to_string()))
        .unwrap();
    let token = assertion(STATE);

    let response = router(store)
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
async fn browser_callback_returns_failure_to_sso_dashboard() {
    let response = router(Arc::new(InMemoryConnectionRepository::new(300)))
        .oneshot(
            Request::get("/connections/callback?assertion=invalid")
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
        "https://sso.farismnrr.com/?relay_connection=failed"
    );
}
