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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Claims {
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub nbf: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClaimValidationError {
    UnauthorizedSubject,
    InsufficientScope,
}

pub fn validate_claims(
    claims: &Claims,
    owner_subject: Option<&str>,
    required_scope: &str,
) -> Result<(), ClaimValidationError> {
    if claims.sub.as_deref() != owner_subject {
        return Err(ClaimValidationError::UnauthorizedSubject);
    }
    if !claims
        .scope
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .any(|scope| scope == required_scope)
    {
        return Err(ClaimValidationError::InsufficientScope);
    }
    Ok(())
}
pub const JWKS_FETCH_TIMEOUT_SECS: u64 = 10;

pub struct CachedJwks {
    pub jwk_set: jsonwebtoken::jwk::JwkSet,
    pub jwks_uri: String,
    pub fetched_at: std::time::Instant,
}

impl CachedJwks {
    pub fn is_stale(&self, now: std::time::Instant) -> bool {
        cache_is_stale(self.fetched_at, now)
    }
    pub fn jwks_uri(&self) -> &str {
        &self.jwks_uri
    }
    pub fn find_key(&self, kid: &str) -> Option<jsonwebtoken::jwk::Jwk> {
        self.jwk_set.find(kid).cloned()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CacheKeyDecision {
    Found,
    RefreshOnce,
}

pub fn key_lookup_decision(key_found: bool) -> CacheKeyDecision {
    if key_found {
        CacheKeyDecision::Found
    } else {
        CacheKeyDecision::RefreshOnce
    }
}

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(JWKS_FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|_| "failed to build HTTP client".to_string())
}

pub fn parse_discovery_metadata(metadata: &serde_json::Value) -> Result<String, String> {
    metadata
        .get("jwks_uri")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "OIDC discovery missing jwks_uri".to_string())
}

pub async fn fetch_discovery(
    client: Result<reqwest::Client, String>,
    discovery_url: String,
) -> Result<String, String> {
    let client = client?;
    let response = client
        .get(&discovery_url)
        .send()
        .await
        .map_err(|e| format!("OIDC discovery fetch failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("OIDC discovery request failed: {e}"))?;
    let metadata = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("OIDC discovery parse failed: {e}"))?;
    parse_discovery_metadata(&metadata)
}

pub async fn fetch_jwks(
    client: Result<reqwest::Client, String>,
    jwks_url: String,
) -> Result<jsonwebtoken::jwk::JwkSet, String> {
    let client = client?;
    let response = client
        .get(&jwks_url)
        .send()
        .await
        .map_err(|e| format!("JWKS fetch failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("JWKS request failed: {e}"))?;
    response
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await
        .map_err(|e| format!("JWKS parse failed: {e}"))
}

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

    #[test]
    fn challenge_and_metadata_preserve_oauth_projection() {
        let mut config = ServerConfig::default();
        config.oauth_issuer = Some("https://issuer.example".into());
        config.oauth_audience = Some("https://resource.example/mcp".into());

        assert_eq!(
            protected_resource_metadata_url(&config).as_deref(),
            Some("https://resource.example/.well-known/oauth-protected-resource/mcp")
        );
        let challenge =
            bearer_challenge_value(&config, Some("insufficient_scope"), Some(CODING_SCOPE));
        assert!(challenge.starts_with("Bearer realm=\"mcp\", error=\"insufficient_scope\""));
        assert!(challenge.contains("scope=\"relay.coding\""));
        assert!(challenge.contains("resource_metadata=\"https://resource.example/.well-known/oauth-protected-resource/mcp\""));
        assert_eq!(
            metadata(&config)["resource"],
            json!("https://resource.example/mcp")
        );
        assert_eq!(
            metadata(&config)["authorization_servers"],
            json!(["https://issuer.example"])
        );
        assert_eq!(bearer_challenge_headers(&config, None, None).len(), 1);
    }

    #[test]
    fn discovery_metadata_requires_jwks_uri() {
        assert_eq!(
            parse_discovery_metadata(&json!({"jwks_uri": "https://issuer.example/keys"})).unwrap(),
            "https://issuer.example/keys"
        );
        assert_eq!(
            parse_discovery_metadata(&json!({})).unwrap_err(),
            "OIDC discovery missing jwks_uri"
        );
    }

    #[test]
    fn claim_policy_accepts_owner_scope_and_rejects_each_failure() {
        let valid = Claims {
            iss: None,
            sub: Some("owner".into()),
            client_id: None,
            scope: Some("read relay.coding".into()),
            exp: None,
            iat: None,
            nbf: None,
        };
        assert_eq!(validate_claims(&valid, Some("owner"), CODING_SCOPE), Ok(()));
        assert_eq!(
            validate_claims(&valid, Some("other"), CODING_SCOPE),
            Err(ClaimValidationError::UnauthorizedSubject)
        );
        let no_scope = Claims {
            scope: None,
            ..valid.clone()
        };
        assert_eq!(
            validate_claims(&no_scope, Some("owner"), CODING_SCOPE),
            Err(ClaimValidationError::InsufficientScope)
        );
    }

    #[test]
    fn cache_key_lookup_refreshes_once_only_when_missing() {
        assert_eq!(key_lookup_decision(true), CacheKeyDecision::Found);
        assert_eq!(key_lookup_decision(false), CacheKeyDecision::RefreshOnce);
    }
}
