use axum::http::{header::LOCATION, StatusCode};

use super::authorization_redirect_error;

#[test]
fn authorization_error_redirect_includes_state_and_issuer() {
    let response = authorization_redirect_error(
        "https://chatgpt.com/connector_platform_oauth_redirect",
        "invalid_request",
        Some("state-123"),
        "https://sso.farismnrr.com",
    );

    assert!(response.status().is_redirection());
    assert_ne!(response.status(), StatusCode::BAD_REQUEST);
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("redirect location");
    assert!(location.contains("error=invalid_request"));
    assert!(location.contains("state=state-123"));
    assert!(location.contains("iss=https%3A%2F%2Fsso.farismnrr.com"));
}
