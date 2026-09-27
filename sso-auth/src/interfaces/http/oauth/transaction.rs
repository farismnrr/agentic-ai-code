use axum::response::Response;
use url::Url;

use crate::application::McpAuthorizationRequest;

use super::{response::authorization_redirect_error, super::AuthHttpState};

pub(crate) async fn authorization_error_from_return_to(
    state: &AuthHttpState,
    return_to: &str,
    error: &'static str,
) -> Option<Response> {
    let request = parse_authorization_return(return_to)?;
    let client = state
        .oauth
        .validate_client(&request.client_id, &request.redirect_uri)
        .await
        .ok()?;
    state
        .oauth
        .validate_authorization_request(request.clone(), &client)
        .ok()?;

    Some(authorization_redirect_error(
        &request.redirect_uri,
        error,
        Some(&request.state),
        &state.oauth_metadata.issuer,
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
    use super::parse_authorization_return;

    #[test]
    fn parses_complete_authorization_return() {
        let return_to = concat!(
            "/oauth/authorize?response_type=code",
            "&client_id=https%3A%2F%2Fchatgpt.com%2Foauth%2Fclient.json",
            "&redirect_uri=https%3A%2F%2Fchatgpt.com%2Fconnector_platform_oauth_redirect",
            "&code_challenge=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "&code_challenge_method=S256",
            "&scope=identity.read",
            "&state=state-123",
            "&resource=https%3A%2F%2Frelay.farismnrr.com%2Fmcp"
        );

        let request = parse_authorization_return(return_to).expect("authorization return");
        assert_eq!(request.state, "state-123");
        assert_eq!(request.resource, "https://relay.farismnrr.com/mcp");
    }

    #[test]
    fn rejects_duplicate_or_unrelated_redirect_targets() {
        let duplicate = concat!(
            "/oauth/authorize?response_type=code",
            "&client_id=https%3A%2F%2Fchatgpt.com%2Foauth%2Fclient.json",
            "&redirect_uri=https%3A%2F%2Fchatgpt.com%2Fconnector_platform_oauth_redirect",
            "&redirect_uri=https%3A%2F%2Fevil.example%2Fcallback",
            "&code_challenge=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "&code_challenge_method=S256",
            "&scope=identity.read",
            "&state=state-123",
            "&resource=https%3A%2F%2Frelay.farismnrr.com%2Fmcp"
        );
        assert!(parse_authorization_return(duplicate).is_none());
        assert!(parse_authorization_return("/?redirect_uri=https://evil.example").is_none());
    }
}
