use std::sync::Arc;

use axum::{
    body::Body,
    http::{header::LOCATION, header::SET_COOKIE, Request, StatusCode},
    Router,
};
use relay_agent::{
    application::{AuthCallbackUseCase, AuthError, AuthStartUseCase, SignedAssertionVerifier},
    domain::VerifiedPrincipal,
    infrastructure::{
        sso::SsoLoginUrlBuilder, verification::PendingSignedAssertionVerifier,
    },
    interfaces::http::{build_router, AuthHttpState},
};
use tower::ServiceExt;
use url::Url;

fn router_with(verifier: Arc<dyn SignedAssertionVerifier>) -> Router {
    let sso_login = Arc::new(
        SsoLoginUrlBuilder::new(
            Url::parse("https://sso.farismnrr.com").expect("valid SSO URL"),
            Url::parse("https://relay.example.com").expect("valid Relay URL"),
        )
        .expect("valid login URL"),
    );

    build_router(AuthHttpState {
        start: Arc::new(AuthStartUseCase::new(sso_login)),
        callback: Arc::new(AuthCallbackUseCase::new(verifier)),
    })
}

#[tokio::test]
async fn login_redirects_to_sso_with_configured_return_target() {
    let response = router_with(Arc::new(PendingSignedAssertionVerifier))
        .oneshot(
            Request::builder()
                .uri("/auth/login")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get(LOCATION)
        .expect("location header")
        .to_str()
        .expect("location string");
    let location = Url::parse(location).expect("redirect URL");

    assert_eq!(
        location.origin().ascii_serialization(),
        "https://sso.farismnrr.com"
    );
    assert_eq!(location.path(), "/auth/github");
    assert_eq!(
        location
            .query_pairs()
            .find(|(key, _)| key == "return_to")
            .map(|(_, value)| value.into_owned()),
        Some("https://relay.example.com/auth/callback".to_string())
    );
}

#[tokio::test]
async fn callback_without_assertion_is_rejected() {
    let response = router_with(Arc::new(PendingSignedAssertionVerifier))
        .oneshot(
            Request::builder()
                .uri("/auth/callback")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().get(SET_COOKIE).is_none());
}

#[tokio::test]
async fn callback_with_assertion_is_not_authenticated_without_verification() {
    let response = router_with(Arc::new(PendingSignedAssertionVerifier))
        .oneshot(
            Request::builder()
                .uri("/auth/callback?assertion=opaque-value")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    assert!(response.headers().get(SET_COOKIE).is_none());
}

struct AcceptingVerifier;

impl SignedAssertionVerifier for AcceptingVerifier {
    fn verify(&self, _assertion: &str) -> Result<VerifiedPrincipal, AuthError> {
        Ok(VerifiedPrincipal::new("github:120432426"))
    }
}

#[tokio::test]
async fn verified_assertion_still_does_not_create_session_in_skeleton() {
    let response = router_with(Arc::new(AcceptingVerifier))
        .oneshot(
            Request::builder()
                .uri("/auth/callback?assertion=verified-for-test")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    assert!(response.headers().get(SET_COOKIE).is_none());
}
