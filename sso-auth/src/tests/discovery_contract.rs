use axum::{
    body::to_bytes,
    http::{header::WWW_AUTHENTICATE, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use url::Url;

use crate::{
    application::{McpAuthorizationRequest, McpTokenRequest},
    domain::AuthenticatedUser,
    interfaces::http::OAuthServerMetadata,
};

use super::discovery_contract_support::{
    mcp_request, oauth_service, pkce, relay_router, resource_metadata_url, CLIENT_ID, ISSUER,
    PROTOCOL, REDIRECT_URI, RESOURCE, SCOPE,
};

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

#[tokio::test]
async fn chatgpt_discovery_contract_connects_relay_oauth_and_latest_mcp() {
    let relay = relay_router();

    let challenge_response = relay
        .clone()
        .oneshot(mcp_request("server/discover", None))
        .await
        .expect("unauthenticated discovery response");
    assert_eq!(challenge_response.status(), StatusCode::UNAUTHORIZED);
    let challenge = challenge_response
        .headers()
        .get(WWW_AUTHENTICATE)
        .and_then(|value| value.to_str().ok())
        .expect("OAuth challenge");
    let metadata_url = resource_metadata_url(challenge).expect("resource metadata URL");
    assert_eq!(
        metadata_url,
        "https://relay.example.com/.well-known/oauth-protected-resource/mcp"
    );

    let metadata_path = Url::parse(metadata_url)
        .expect("resource metadata URL")
        .path()
        .to_string();
    let prm_response = relay
        .clone()
        .oneshot(
            Request::get(metadata_path)
                .body(axum::body::Body::empty())
                .expect("PRM request"),
        )
        .await
        .expect("PRM response");
    assert_eq!(prm_response.status(), StatusCode::OK);
    let prm = response_json(prm_response).await;
    assert_eq!(prm["resource"], RESOURCE);
    assert_eq!(prm["authorization_servers"], json!([ISSUER]));
    assert_eq!(prm["scopes_supported"], json!([SCOPE]));

    let as_metadata = serde_json::to_value(OAuthServerMetadata::new(
        prm["authorization_servers"][0]
            .as_str()
            .expect("authorization server"),
    ))
    .expect("authorization server metadata");
    assert_eq!(as_metadata["issuer"], ISSUER);
    assert_eq!(
        as_metadata["authorization_endpoint"],
        format!("{ISSUER}/oauth/authorize")
    );
    assert_eq!(
        as_metadata["token_endpoint"],
        format!("{ISSUER}/oauth/token")
    );
    assert_eq!(
        as_metadata["client_id_metadata_document_supported"],
        json!(true)
    );
    assert_eq!(
        as_metadata["token_endpoint_auth_methods_supported"],
        json!(["none"])
    );
    assert_eq!(
        as_metadata["code_challenge_methods_supported"],
        json!(["S256"])
    );

    let oauth = oauth_service();
    let client = oauth
        .validate_client(CLIENT_ID, REDIRECT_URI)
        .await
        .expect("ChatGPT CIMD client");
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~"
        .chars()
        .take(64)
        .collect::<String>();
    let authorization = oauth
        .validate_authorization_request(
            McpAuthorizationRequest {
                client_id: CLIENT_ID.to_string(),
                redirect_uri: REDIRECT_URI.to_string(),
                code_challenge: pkce(&verifier),
                code_challenge_method: "S256".to_string(),
                scope: SCOPE.to_string(),
                state: "chatgpt-state".to_string(),
                resource: prm["resource"]
                    .as_str()
                    .expect("canonical resource")
                    .to_string(),
            },
            &client,
        )
        .expect("authorization request");

    let code = oauth
        .issue_code(
            AuthenticatedUser {
                id: 1,
                login: "faris".to_string(),
                avatar_url: None,
            },
            authorization,
        )
        .expect("authorization code");
    let token = oauth
        .exchange(McpTokenRequest {
            grant_type: "authorization_code".to_string(),
            code,
            redirect_uri: REDIRECT_URI.to_string(),
            client_id: CLIENT_ID.to_string(),
            code_verifier: verifier,
            resource: RESOURCE.to_string(),
        })
        .expect("access token");
    assert_eq!(token.scope, SCOPE);

    let discover_response = relay
        .clone()
        .oneshot(mcp_request("server/discover", Some(&token.access_token)))
        .await
        .expect("authenticated discovery response");
    assert_eq!(discover_response.status(), StatusCode::OK);
    let discover = response_json(discover_response).await;
    assert_eq!(discover["result"]["supportedVersions"], json!([PROTOCOL]));
    assert_eq!(discover["result"]["resultType"], "complete");

    let tools_response = relay
        .oneshot(mcp_request("tools/list", Some(&token.access_token)))
        .await
        .expect("tools list response");
    assert_eq!(tools_response.status(), StatusCode::OK);
    let tools = response_json(tools_response).await;
    let profile = &tools["result"]["tools"][0];
    assert_eq!(profile["name"], "get_profile");
    assert_eq!(profile["securitySchemes"][0]["type"], "oauth2");
    assert_eq!(profile["securitySchemes"][0]["scopes"], json!([SCOPE]));
    assert_eq!(
        profile["_meta"]["securitySchemes"],
        profile["securitySchemes"]
    );
}
