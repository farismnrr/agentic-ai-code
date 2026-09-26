mod support;

use axum::{
    body::{to_bytes, Body},
    http::{header::WWW_AUTHENTICATE, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use support::mcp_oauth::{mcp_request, router, ISSUER, PROTOCOL, RESOURCE};

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
