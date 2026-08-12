use super::config::ServerConfig;
use super::error::McpError;
use super::mcp::{ErrorResponse, Id};
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub const CODING_SCOPE: &str = "relay.coding";
pub const JWKS_TTL_SECS: u64 = 300;

/// Validate the `jwks_uri` returned by OIDC discovery.
///
/// This is intentionally a pure policy seam: fetching, caching, and token
/// validation remain owned by the transport orchestration for now.
pub fn validate_jwks_uri(raw: &str, allow_insecure_fixture: bool) -> bool {
    let Ok(parsed) = url::Url::parse(raw) else {
        return false;
    };
    let Some(authority) = raw
        .split_once("://")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
    else {
        return false;
    };
    !parsed.cannot_be_a_base()
        && parsed.has_authority()
        && parsed.host_str().is_some()
        && !authority.is_empty()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && (allow_insecure_fixture || parsed.scheme() == "https")
}

pub fn cache_is_stale(fetched_at: std::time::Instant, now: std::time::Instant) -> bool {
    now.duration_since(fetched_at).as_secs() >= JWKS_TTL_SECS
}

pub fn protected_resource_metadata_url(config: &ServerConfig) -> Option<String> {
    let audience = config.oauth_audience.as_deref()?;
    let mut resource = url::Url::parse(audience).ok()?;
    let path = resource.path().trim_start_matches('/').to_owned();
    let metadata_path = if path.is_empty() {
        "/.well-known/oauth-protected-resource".to_owned()
    } else {
        format!("/.well-known/oauth-protected-resource/{path}")
    };
    resource.set_path(&metadata_path);
    Some(resource.to_string())
}

pub fn bearer_challenge_value(
    config: &ServerConfig,
    error: Option<&str>,
    scope: Option<&str>,
) -> String {
    let mut parameters = vec!["realm=\"mcp\"".to_owned()];
    if let Some(error) = error {
        parameters.push(format!("error=\"{error}\""));
    }
    let description = match error {
        Some("invalid_token") => "The access token is invalid or expired",
        Some("insufficient_scope") => "The access token lacks the required scope",
        _ => "Authentication is required",
    };
    parameters.push(format!("error_description=\"{description}\""));
    if let Some(scope) = scope {
        parameters.push(format!("scope=\"{scope}\""));
    }
    if let Some(url) = protected_resource_metadata_url(config) {
        parameters.push(format!("resource_metadata=\"{url}\""));
    }
    format!("Bearer {}", parameters.join(", "))
}

pub fn bearer_challenge_headers(
    config: &ServerConfig,
    error: Option<&str>,
    scope: Option<&str>,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(value) = bearer_challenge_value(config, error, scope).parse() {
        headers.insert(axum::http::header::WWW_AUTHENTICATE, value);
    }
    headers
}

pub fn oauth_error_response(
    status: StatusCode,
    id: Option<Id>,
    config: &ServerConfig,
    error: Option<&str>,
    scope: Option<&str>,
    message: &McpError,
) -> Response {
    (
        status,
        bearer_challenge_headers(config, error, scope),
        Json(ErrorResponse::new(id, message)),
    )
        .into_response()
}

pub fn metadata(config: &ServerConfig) -> serde_json::Value {
    let mut value =
        json!({ "resource": config.oauth_audience, "scopes_supported": [CODING_SCOPE] });
    if let Some(issuer) = &config.oauth_issuer {
        value["authorization_servers"] = json!([issuer]);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwks_url_requires_https_and_public_authority() {
        assert!(validate_jwks_uri("https://issuer.example/keys", false));
        assert!(!validate_jwks_uri("http://issuer.example/keys", false));
        assert!(validate_jwks_uri("http://issuer.example/keys", true));
        assert!(!validate_jwks_uri(
            "https://user:pass@issuer.example/keys",
            false
        ));
        assert!(!validate_jwks_uri("https:///keys", false));
        assert!(!validate_jwks_uri("not a url", false));
    }

    #[test]
    fn cache_freshness_is_ttl_based() {
        let start = std::time::Instant::now();
        assert!(!cache_is_stale(
            start,
            start + std::time::Duration::from_secs(JWKS_TTL_SECS - 1)
        ));
        assert!(cache_is_stale(
            start,
            start + std::time::Duration::from_secs(JWKS_TTL_SECS)
        ));
    }
}
