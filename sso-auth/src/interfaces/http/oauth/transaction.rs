use axum::response::Response;
use url::Url;

use crate::application::{McpAuthorizationRequest, McpOAuthService};

use super::response::authorization_redirect_error;

pub(crate) async fn authorization_error_from_return_to(
    oauth: &McpOAuthService,
    issuer: &str,
    return_to: &str,
    error: &'static str,
) -> Option<Response> {
    let request = parse_authorization_return(return_to)?;
    let client = oauth
        .validate_client(&request.client_id, &request.redirect_uri)
        .await
        .ok()?;
    oauth
        .validate_authorization_request(request.clone(), &client)
        .ok()?;

    Some(authorization_redirect_error(
        &request.redirect_uri,
        error,
        Some(&request.state),
        issuer,
    ))
}

fn parse_authorization_return(value: &str) -> Option<McpAuthorizationRequest> {
    if value.len() > 2048 || !value.starts_with("/oauth/authorize?") {
        return None;
    }

    let url = Url::parse(&format!("https://sso.invalid{value}")).ok()?;
    if url.path() != "/oauth/authorize" || url.fragment().is_some() {
        return None;
    }
    if single_query_value(&url, "response_type")?.as_str() != "code" {
        return None;
    }

    Some(McpAuthorizationRequest {
        client_id: single_query_value(&url, "client_id")?,
        redirect_uri: single_query_value(&url, "redirect_uri")?,
        code_challenge: single_query_value(&url, "code_challenge")?,
        code_challenge_method: single_query_value(&url, "code_challenge_method")?,
        scope: single_query_value(&url, "scope")?,
        state: single_query_value(&url, "state")?,
        resource: single_query_value(&url, "resource")?,
    })
}

fn single_query_value(url: &Url, key: &str) -> Option<String> {
    let mut values = url.query_pairs().filter(|(name, _)| name == key);
    let value = values.next()?.1.into_owned();
    (!value.is_empty() && values.next().is_none()).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::http::header::LOCATION;

    use crate::{
        application::{
            AuthError, McpAccessTokenIssuer, McpClientMetadata, McpClientMetadataResolver,
            McpOAuthService, StateGenerator,
        },
        domain::AuthenticatedUser,
    };

    use super::{authorization_error_from_return_to, parse_authorization_return};

    const ISSUER: &str = "https://sso.farismnrr.com";
    const RESOURCE: &str = "https://relay.farismnrr.com/mcp";
    const CLIENT_ID: &str = "https://chatgpt.com/oauth/client.json";
    const REDIRECT_URI: &str = "https://chatgpt.com/connector_platform_oauth_redirect";

    struct FixedState;

    impl StateGenerator for FixedState {
        fn generate(&self) -> String {
            "unused".to_string()
        }
    }

    struct DummyTokens;

    impl McpAccessTokenIssuer for DummyTokens {
        fn issue(
            &self,
            _user: &AuthenticatedUser,
            _audience: &str,
            _scope: &str,
            _ttl_seconds: u64,
        ) -> Result<String, AuthError> {
            Ok("unused".to_string())
        }
    }

    struct ChatGptMetadata;

    #[async_trait]
    impl McpClientMetadataResolver for ChatGptMetadata {
        async fn resolve(&self, client_id: &str) -> Result<McpClientMetadata, AuthError> {
            Ok(McpClientMetadata {
                client_id: client_id.to_string(),
                redirect_uris: vec![REDIRECT_URI.to_string()],
                token_endpoint_auth_methods_supported: vec!["none".to_string()],
            })
        }
    }

    fn oauth() -> McpOAuthService {
        McpOAuthService::new(
            Arc::new(FixedState),
            Arc::new(DummyTokens),
            Arc::new(ChatGptMetadata),
            RESOURCE.to_string(),
        )
    }

    fn return_to(resource: &str) -> String {
        let challenge = "A".repeat(43);
        url::form_urlencoded::Serializer::new("/oauth/authorize?".to_string())
            .append_pair("response_type", "code")
            .append_pair("client_id", CLIENT_ID)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("scope", "identity.read")
            .append_pair("state", "state-123")
            .append_pair("resource", resource)
            .finish()
    }

    #[test]
    fn parses_complete_authorization_return() {
        let request = parse_authorization_return(&return_to(RESOURCE))
            .expect("authorization return");
        assert_eq!(request.state, "state-123");
        assert_eq!(request.resource, RESOURCE);
    }

    #[test]
    fn rejects_duplicate_or_unrelated_redirect_targets() {
        let duplicate = format!(
            "{}&redirect_uri=https%3A%2F%2Fevil.example%2Fcallback",
            return_to(RESOURCE)
        );
        assert!(parse_authorization_return(&duplicate).is_none());
        assert!(parse_authorization_return("/?redirect_uri=https://evil.example").is_none());
    }

    #[tokio::test]
    async fn validated_provider_error_returns_state_and_issuer_to_chatgpt() {
        let response = authorization_error_from_return_to(
            &oauth(),
            ISSUER,
            &return_to(RESOURCE),
            "access_denied",
        )
        .await
        .expect("OAuth error redirect");
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .expect("location");
        let url = url::Url::parse(location).expect("redirect URL");
        let query = url.query_pairs().collect::<std::collections::HashMap<_, _>>();

        assert_eq!(url.origin().ascii_serialization(), "https://chatgpt.com");
        assert_eq!(query.get("error").map(|value| value.as_ref()), Some("access_denied"));
        assert_eq!(query.get("state").map(|value| value.as_ref()), Some("state-123"));
        assert_eq!(query.get("iss").map(|value| value.as_ref()), Some(ISSUER));
    }

    #[tokio::test]
    async fn provider_error_handoff_rejects_tampered_resource() {
        let response = authorization_error_from_return_to(
            &oauth(),
            ISSUER,
            &return_to("https://evil.example/mcp"),
            "access_denied",
        )
        .await;
        assert!(response.is_none());
    }
}
