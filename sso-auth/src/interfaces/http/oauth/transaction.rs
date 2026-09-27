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
#[path = "../../../../test/interfaces/http/oauth/transaction.rs"]
mod tests;
