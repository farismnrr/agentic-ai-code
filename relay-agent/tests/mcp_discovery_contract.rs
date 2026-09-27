mod support;

use axum::{
    body::{to_bytes, Body},
    http::{header::WWW_AUTHENTICATE, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use support::mcp_oauth::{mcp_request, router, ISSUER, PROTOCOL, RESOURCE};

fn contract() -> Value {
    serde_json::from_str(include_str!("../../contracts/chatgpt-discovery.json"))
        .expect("shared discovery contract")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}

#[tokio::test]
async fn relay_matches_shared_chatgpt_discovery_contract() {
    let contract = contract();
    assert_eq!(ISSUER, contract["issuer"].as_str().unwrap());
    assert_eq!(RESOURCE, contract["resource"].as_str().unwrap());
    assert_eq!(PROTOCOL, contract["protocol_version"].as_str().unwrap());

    let response = router()
        .oneshot(mcp_request("server/discover", json!({}), false))
        .await
        .expect("unauthenticated discovery");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let challenge = response
        .headers()
        .get(WWW_AUTHENTICATE)
        .and_then(|value| value.to_str().ok())
        .expect("OAuth challenge");
    assert!(challenge.contains(
        contract["resource_metadata"]
            .as_str()
            .expect("resource metadata URL")
    ));
    assert!(challenge.contains(contract["scope"].as_str().expect("scope")));

    let response = router()
        .oneshot(
            Request::get("/.well-known/oauth-protected-resource/mcp")
                .body(Body::empty())
                .expect("PRM request"),
        )
        .await
        .expect("PRM response");
    assert_eq!(response.status(), StatusCode::OK);
    let prm = response_json(response).await;
    assert_eq!(prm["resource"], contract["resource"]);
    assert_eq!(prm["authorization_servers"][0], contract["issuer"]);
    assert_eq!(prm["scopes_supported"][0], contract["scope"]);

    let response = router()
        .oneshot(mcp_request("server/discover", json!({}), true))
        .await
        .expect("authenticated discovery");
    assert_eq!(response.status(), StatusCode::OK);
    let discover = response_json(response).await;
    assert_eq!(
        discover["result"]["supportedVersions"][0],
        contract["protocol_version"]
    );

    let response = router()
        .oneshot(mcp_request("tools/list", json!({}), true))
        .await
        .expect("tools list");
    assert_eq!(response.status(), StatusCode::OK);
    let tools = response_json(response).await;
    let profile = &tools["result"]["tools"][0];
    assert_eq!(profile["securitySchemes"][0]["type"], "oauth2");
    assert_eq!(
        profile["securitySchemes"][0]["scopes"][0],
        contract["scope"]
    );
    assert_eq!(
        profile["_meta"]["securitySchemes"],
        profile["securitySchemes"]
    );
}
