//! Local-access security policy: `Origin` + `Host` validation.
//!
//! `tower_http::cors::CorsLayer` (in `transport.rs`) is a **browser-enforced
//! convenience**, not a security boundary: any non-browser HTTP client can
//! send an arbitrary `Origin` header and CORS will never see or block it —
//! CORS only stops a *browser* from handing the response back to page
//! script. This module is the actual server-side enforcement. Every request
//! to the MCP endpoint must carry:
//!
//! - an `Origin` header that exactly matches the configured allowed origin
//!   (`--origin` / `RELAY_AGENT_ORIGIN`), and
//! - a `Host` header that exactly matches this server's own bind address,
//!
//! or it is rejected before it ever reaches the MCP handler. Every failure
//! mode here fails closed: missing, duplicated, malformed, or mismatched
//! headers are all rejections, never a permissive default.

use axum::http::HeaderMap;

use super::config::ServerConfig;
use super::error::McpError;

/// Validate the `Origin` header against the configured allowed origin.
///
/// Fails closed when:
/// - the server has no `config.origin` configured at all (nothing valid to
///   match against — this is not treated as "allow anything"),
/// - the `Origin` header is missing,
/// - more than one `Origin` header is present,
/// - the header value isn't valid UTF-8,
/// - the header value doesn't parse as a clean `scheme://host[:port]` origin
///   (no path/query/fragment/userinfo), or
/// - the parsed origin doesn't exactly equal the configured origin.
pub fn validate_origin(headers: &HeaderMap, config: &ServerConfig) -> Result<(), McpError> {
    let configured = config.origin.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("server has no allowed Origin configured".to_string())
    })?;

    let mut values = headers.get_all(axum::http::header::ORIGIN).iter();
    let first = values
        .next()
        .ok_or_else(|| McpError::InvalidRequest("missing required Origin header".to_string()))?;

    if values.next().is_some() {
        return Err(McpError::InvalidRequest(
            "multiple Origin headers are not permitted".to_string(),
        ));
    }

    let origin_str = first
        .to_str()
        .map_err(|_| McpError::InvalidRequest("Origin header is not valid UTF-8".to_string()))?;

    let parsed = normalize_origin(origin_str).ok_or_else(|| {
        McpError::InvalidRequest(format!("malformed Origin header: '{origin_str}'"))
    })?;

    let expected = normalize_origin(configured)
        .ok_or_else(|| McpError::InvalidRequest("configured Origin is malformed".to_string()))?;

    if parsed != expected {
        return Err(McpError::InvalidRequest(format!(
            "Origin '{origin_str}' is not the configured allowed origin"
        )));
    }

    Ok(())
}

/// Parse `scheme://host[:port]` into a normalized, comparable tuple.
///
/// Returns `None` for anything that isn't a clean absolute origin: a path
/// beyond `/`, a query, a fragment, or embedded userinfo all make the value
/// ambiguous rather than a bare origin, so they are rejected rather than
/// silently stripped.
fn normalize_origin(raw: &str) -> Option<(String, String, Option<u16>)> {
    let url = url::Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    if !matches!(url.path(), "" | "/") {
        return None;
    }
    if url.query().is_some() || url.fragment().is_some() || !url.username().is_empty() {
        return None;
    }
    if url.password().is_some() {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    Some((url.scheme().to_string(), host, url.port()))
}

/// Validate the `Host` header against this server's own bind address.
///
/// Only `127.0.0.1:<port>` and `localhost:<port>` are accepted — this
/// server only ever binds `127.0.0.1` (security invariant #1 in the plan),
/// so no other `Host` value can be legitimate.
///
/// `X-Forwarded-Host` and any other proxy-supplied header are never read by
/// this function; it only inspects the actual `Host` header of the request
/// that reached this process. Since this server is not deployed behind a
/// reverse proxy (it binds `127.0.0.1` directly for local Nuxt/MCP-host
/// access), there is nothing legitimate a proxy header could add — reading
/// it would only open a spoofing vector, so it is intentionally ignored.
pub fn validate_host(headers: &HeaderMap, config: &ServerConfig) -> Result<(), McpError> {
    let mut values = headers.get_all(axum::http::header::HOST).iter();
    let first = values
        .next()
        .ok_or_else(|| McpError::InvalidRequest("missing required Host header".to_string()))?;

    if values.next().is_some() {
        return Err(McpError::InvalidRequest(
            "multiple Host headers are not permitted".to_string(),
        ));
    }

    let host_str = first
        .to_str()
        .map_err(|_| McpError::InvalidRequest("Host header is not valid UTF-8".to_string()))?
        .to_ascii_lowercase();

    let allowed = [
        format!("127.0.0.1:{}", config.port),
        format!("localhost:{}", config.port),
    ];

    if !allowed.iter().any(|a| a == &host_str) {
        return Err(McpError::InvalidRequest(format!(
            "Host '{host_str}' is not this server's bind address"
        )));
    }

    Ok(())
}

/// Run both checks in the order the plan specifies (Origin, then Host).
/// Either failure fails the whole policy closed.
pub fn enforce_local_access_policy(
    headers: &HeaderMap,
    config: &ServerConfig,
) -> Result<(), McpError> {
    validate_origin(headers, config)?;
    validate_host(headers, config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn config() -> ServerConfig {
        ServerConfig {
            port: 47821,
            dir: None,
            origin: Some("http://localhost:3333".to_string()),
        }
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    #[test]
    fn origin_allowed_when_exact_match() {
        let h = headers(&[("origin", "http://localhost:3333")]);
        assert!(validate_origin(&h, &config()).is_ok());
    }

    #[test]
    fn origin_rejected_when_missing() {
        let h = HeaderMap::new();
        assert!(validate_origin(&h, &config()).is_err());
    }

    #[test]
    fn origin_rejected_when_wrong() {
        let h = headers(&[("origin", "http://localhost:9999")]);
        assert!(validate_origin(&h, &config()).is_err());
    }

    #[test]
    fn origin_rejected_when_wrong_scheme() {
        let h = headers(&[("origin", "https://localhost:3333")]);
        assert!(validate_origin(&h, &config()).is_err());
    }

    #[test]
    fn origin_rejected_when_malformed() {
        let h = headers(&[("origin", "not-a-url")]);
        assert!(validate_origin(&h, &config()).is_err());
    }

    #[test]
    fn origin_rejected_when_server_has_none_configured() {
        let h = headers(&[("origin", "http://localhost:3333")]);
        let mut cfg = config();
        cfg.origin = None;
        assert!(validate_origin(&h, &cfg).is_err());
    }

    #[test]
    fn origin_rejected_when_path_present() {
        let h = headers(&[("origin", "http://localhost:3333/mcp")]);
        assert!(validate_origin(&h, &config()).is_err());
    }

    #[test]
    fn host_allowed_when_matches_bind_address() {
        let h = headers(&[("host", "127.0.0.1:47821")]);
        assert!(validate_host(&h, &config()).is_ok());
    }

    #[test]
    fn host_allowed_for_localhost_alias() {
        let h = headers(&[("host", "localhost:47821")]);
        assert!(validate_host(&h, &config()).is_ok());
    }

    #[test]
    fn host_rejected_when_missing() {
        let h = HeaderMap::new();
        assert!(validate_host(&h, &config()).is_err());
    }

    #[test]
    fn host_rejected_when_external() {
        let h = headers(&[("host", "evil.example.com:47821")]);
        assert!(validate_host(&h, &config()).is_err());
    }

    #[test]
    fn host_rejected_when_lookalike() {
        let h = headers(&[("host", "127.0.0.1.evil.example.com:47821")]);
        assert!(validate_host(&h, &config()).is_err());
    }

    #[test]
    fn host_rejected_when_wrong_port() {
        let h = headers(&[("host", "127.0.0.1:9999")]);
        assert!(validate_host(&h, &config()).is_err());
    }

    #[test]
    fn host_ignores_x_forwarded_host_and_still_rejects_bad_real_host() {
        let h = headers(&[
            ("host", "evil.example.com:47821"),
            ("x-forwarded-host", "127.0.0.1:47821"),
        ]);
        // A spoofed X-Forwarded-Host claiming a valid host must not rescue
        // an invalid real Host header.
        assert!(validate_host(&h, &config()).is_err());
    }

    #[test]
    fn host_ignores_x_forwarded_host_when_real_host_is_valid() {
        let h = headers(&[
            ("host", "127.0.0.1:47821"),
            ("x-forwarded-host", "evil.example.com:47821"),
        ]);
        // A spoofed X-Forwarded-Host claiming an attacker host must not
        // override an otherwise-valid real Host header either — it's just
        // never consulted.
        assert!(validate_host(&h, &config()).is_ok());
    }

    #[test]
    fn policy_passes_only_when_both_checks_pass() {
        let h = headers(&[
            ("origin", "http://localhost:3333"),
            ("host", "127.0.0.1:47821"),
        ]);
        assert!(enforce_local_access_policy(&h, &config()).is_ok());
    }

    #[test]
    fn policy_fails_when_origin_ok_but_host_bad() {
        let h = headers(&[
            ("origin", "http://localhost:3333"),
            ("host", "evil.example.com:47821"),
        ]);
        assert!(enforce_local_access_policy(&h, &config()).is_err());
    }
}
