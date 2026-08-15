use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{ErrorResponse, Id};
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

#[derive(Debug, PartialEq, Eq)]
pub enum TokenValidationError {
    InvalidJwk,
    UnsupportedAlgorithm,
    InvalidToken,
}

pub fn validate_token_signature(
    token: &str,
    jwk: &jsonwebtoken::jwk::Jwk,
    algorithm: jsonwebtoken::Algorithm,
    issuer: &str,
    audience: &str,
) -> Result<Claims, TokenValidationError> {
    if !matches!(
        algorithm,
        jsonwebtoken::Algorithm::RS256 | jsonwebtoken::Algorithm::ES256
    ) {
        return Err(TokenValidationError::UnsupportedAlgorithm);
    }
    let decoding_key =
        jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|_| TokenValidationError::InvalidJwk)?;
    let mut validation = jsonwebtoken::Validation::new(algorithm);
    validation.set_audience(&[audience]);
    validation.set_issuer(&[issuer]);
    validation.validate_nbf = true;
    jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|_| TokenValidationError::InvalidToken)
}
/// Cheap, purely structural/syntactic pre-check for a bearer token, run before
/// any discovery/JWKS network I/O. This intentionally does NOT parse claims
/// such as `exp`/`iss`/`aud` — it only rejects tokens that could not possibly
/// be a JWT, so malformed input fails closed without triggering outbound
/// authentication work. Well-formed JWTs (even semantically invalid ones,
/// e.g. expired or wrong-issuer) must continue to the full validation path.
pub fn parse_structurally_plausible_jwt(token: &str) -> Option<jsonwebtoken::Header> {
    use base64::Engine;

    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() != 3 || segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }

    let Ok(header_bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(segments[0])
    else {
        return None;
    };
    // The payload and signature segments must also be valid base64url, even
    // though we do not need their decoded contents here.
    if base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segments[1])
        .is_err()
        || base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(segments[2])
            .is_err()
    {
        return None;
    }

    let header = serde_json::from_slice::<jsonwebtoken::Header>(&header_bytes).ok()?;
    if header.kid.is_none()
        || !matches!(
            header.alg,
            // Discovery/JWKS is only supported for the asymmetric algorithms
            // accepted by the full verifier. Reject every other algorithm
            // before any network I/O (including symmetric algorithms).
            jsonwebtoken::Algorithm::RS256 | jsonwebtoken::Algorithm::ES256
        )
    {
        return None;
    }
    Some(header)
}

pub fn is_structurally_plausible_jwt(token: &str) -> bool {
    parse_structurally_plausible_jwt(token).is_some()
}

pub const JWKS_FETCH_TIMEOUT_SECS: u64 = 10;

pub struct CachedJwks {
    pub jwk_set: jsonwebtoken::jwk::JwkSet,
    pub jwks_uri: String,
    pub fetched_at: std::time::Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CacheSnapshot {
    pub needs_refresh: bool,
    pub jwks_uri: Option<String>,
}

pub async fn read_cache_snapshot(
    cache: &tokio::sync::RwLock<Option<CachedJwks>>,
    now: std::time::Instant,
) -> CacheSnapshot {
    let guard = cache.read().await;
    match guard.as_ref() {
        None => CacheSnapshot {
            needs_refresh: true,
            jwks_uri: None,
        },
        Some(cached) => CacheSnapshot {
            needs_refresh: cached.is_stale(now),
            jwks_uri: Some(cached.jwks_uri().to_owned()),
        },
    }
}

pub async fn lookup_cached_key(
    cache: &tokio::sync::RwLock<Option<CachedJwks>>,
    kid: &str,
) -> Option<jsonwebtoken::jwk::Jwk> {
    cache
        .read()
        .await
        .as_ref()
        .and_then(|cached| cached.find_key(kid))
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

/// Maps a `reqwest::Error` to a bounded, low-cardinality failure class.
/// `reqwest`'s `Display` impl for network errors embeds the request URL
/// (e.g. "error sending request for url (https://host/path)"), so it must
/// never be interpolated into a log/trace field directly — the frozen Plan
/// 035 observability contract forbids arbitrary URLs in telemetry/stderr
/// even when private-only. This never reveals the upstream host/path.
fn classify_reqwest_error(e: &reqwest::Error) -> &'static str {
    if e.is_timeout() {
        "timeout"
    } else if e.is_connect() {
        "connect_failed"
    } else if e.is_status() {
        "http_error_status"
    } else if e.is_decode() {
        "response_decode_failed"
    } else if e.is_body() {
        "body_error"
    } else {
        "request_failed"
    }
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
        .map_err(|e| format!("oidc_discovery_fetch_failed:{}", classify_reqwest_error(&e)))?
        .error_for_status()
        .map_err(|e| {
            format!(
                "oidc_discovery_request_failed:{}",
                classify_reqwest_error(&e)
            )
        })?;
    let metadata = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("oidc_discovery_parse_failed:{}", classify_reqwest_error(&e)))?;
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
        .map_err(|e| format!("jwks_fetch_failed:{}", classify_reqwest_error(&e)))?
        .error_for_status()
        .map_err(|e| format!("jwks_request_failed:{}", classify_reqwest_error(&e)))?;
    response
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await
        .map_err(|e| format!("jwks_parse_failed:{}", classify_reqwest_error(&e)))
}

pub async fn refresh_cache(
    cache: &tokio::sync::RwLock<Option<CachedJwks>>,
    client: Result<reqwest::Client, String>,
    jwks_url: String,
) -> Result<(), String> {
    let new_set = fetch_jwks(client, jwks_url.clone()).await?;
    let mut guard = cache.write().await;
    *guard = Some(CachedJwks {
        jwk_set: new_set,
        jwks_uri: jwks_url,
        fetched_at: std::time::Instant::now(),
    });
    Ok(())
}

pub async fn refresh_cache_for_kid(
    cache: &tokio::sync::RwLock<Option<CachedJwks>>,
    client: Result<reqwest::Client, String>,
    jwks_url: String,
    kid: &str,
) -> Result<Option<jsonwebtoken::jwk::Jwk>, String> {
    let new_set = fetch_jwks(client, jwks_url.clone()).await?;
    let found = new_set.find(kid).cloned();
    let mut guard = cache.write().await;
    *guard = Some(CachedJwks {
        jwk_set: new_set,
        jwks_uri: jwks_url,
        fetched_at: std::time::Instant::now(),
    });
    Ok(found)
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
