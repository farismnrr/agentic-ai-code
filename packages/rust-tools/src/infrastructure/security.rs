//! Local-access security policy: `Origin` + `Host` validation.
//!
//! `tower_http::cors::CorsLayer` (in `transport.rs`) is a **browser-enforced
//! convenience**, not a security boundary: any non-browser HTTP client can
//! send an arbitrary `Origin` header and CORS will never see or block it —
//! CORS only stops a *browser* from handing the response back to page
//! script. This module is the actual server-side enforcement. Every request
//! to the MCP endpoint must carry:
//!
//! - when present, an `Origin` header that exactly matches the configured
//!   allowed origin (`--origin` / `RELAY_AGENT_ORIGIN`), and
//! - a `Host` header that exactly matches this server's own bind address or an
//!   explicitly configured additional authority (`--allowed-host` /
//!   `RELAY_ALLOWED_HOSTS`),
//!
//! or it is rejected before it ever reaches the MCP handler. A missing
//! `Origin` is permitted only for a non-browser MCP client; the configured
//! origin is still required so browser requests have an explicit allowlist.
//! Every other failure mode here fails closed: duplicated, malformed, or
//! mismatched headers are rejections, never a permissive default.

use axum::http::HeaderMap;
use std::net::IpAddr;

use crate::core::config::{parse_host_authority, ServerConfig};
use crate::core::error::McpError;

/// Validate the `Origin` header against the configured allowed origin.
///
/// Fails closed when:
/// - the server has no `config.origin` configured at all (nothing valid to
///   match against — this is not treated as "allow anything"),
/// - more than one `Origin` header is present,
/// - the header value isn't valid UTF-8,
/// - the header value doesn't parse as a clean `scheme://host[:port]` origin
///   (no path/query/fragment/userinfo) — this is only a *validity* gate, or
/// - the header value is not a **byte-for-byte exact match** of the
///   configured origin string.
///
/// The equality check is deliberately not case-folded or re-serialized: the
/// plan requires an *exact* configured-Origin match, not an equivalence
/// check, so `http://LOCALHOST:3333` is rejected even though it would
/// resolve to the same host as a configured `http://localhost:3333` — the
/// URL-parsing step above exists only to reject garbage input before the
/// exact comparison, never to normalize it into something that then
/// compares equal.
pub fn validate_origin(headers: &HeaderMap, config: &ServerConfig) -> Result<(), McpError> {
    let configured = config.origin.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("server has no allowed Origin configured".to_string())
    })?;

    let mut values = headers.get_all(axum::http::header::ORIGIN).iter();
    let Some(first) = values.next() else {
        // Browser requests normally include Origin for cross-origin POSTs,
        // while non-browser MCP clients commonly omit it. Host validation
        // remains mandatory below and protects the loopback listener against
        // DNS rebinding.
        return Ok(());
    };

    if values.next().is_some() {
        return Err(McpError::InvalidRequest(
            "multiple Origin headers are not permitted".to_string(),
        ));
    }

    let origin_str = first
        .to_str()
        .map_err(|_| McpError::InvalidRequest("Origin header is not valid UTF-8".to_string()))?;

    if !is_well_formed_origin(origin_str) {
        return Err(McpError::InvalidRequest(format!(
            "malformed Origin header: '{origin_str}'"
        )));
    }

    if !is_well_formed_origin(configured) {
        return Err(McpError::InvalidRequest(
            "configured Origin is malformed".to_string(),
        ));
    }

    if origin_str != configured {
        return Err(McpError::InvalidRequest(format!(
            "Origin '{origin_str}' is not the configured allowed origin"
        )));
    }

    Ok(())
}

/// Validity gate only — confirms `raw` is a clean absolute `scheme://host[:port]`
/// origin (`http`/`https`, no path beyond `/`, no query/fragment/userinfo).
/// Deliberately returns a bare `bool`, not a normalized/parsed value: this
/// function exists to reject malformed input, never to produce a
/// case-folded or re-serialized form for comparison — see
/// [`validate_origin`]'s doc comment for why exact string equality is used
/// instead.
fn is_well_formed_origin(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    if !matches!(url.path(), "" | "/") {
        return false;
    }
    if url.query().is_some() || url.fragment().is_some() || !url.username().is_empty() {
        return false;
    }
    if url.password().is_some() {
        return false;
    }
    url.host_str().is_some()
}

/// Validate the `Host` header against this server's own bind address.
///
/// The implicit local allowlist always contains `127.0.0.1:<port>` and
/// `localhost:<port>`. Additional authorities come only from the explicit
/// `--allowed-host` / `RELAY_ALLOWED_HOSTS` configuration.
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
        .map_err(|_| McpError::InvalidRequest("Host header is not valid UTF-8".to_string()))?;

    let received = parse_host_authority(host_str)
        .ok_or_else(|| McpError::InvalidRequest("Host header is malformed".to_string()))?;

    let mut allowed = vec![
        ("127.0.0.1".to_string(), Some(config.port)),
        ("localhost".to_string(), Some(config.port)),
    ];
    allowed.extend(
        config
            .allowed_hosts
            .iter()
            .filter_map(|host| parse_host_authority(host)),
    );

    if !allowed.iter().any(|entry| entry == &received) {
        return Err(McpError::InvalidRequest(format!(
            "Host '{host_str}' is not an allowed relay authority"
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
    if let Err(error) = validate_origin(headers, config) {
        log_local_access_failure("origin", headers, config);
        return Err(error);
    }
    if let Err(error) = validate_host(headers, config) {
        log_local_access_failure("host", headers, config);
        return Err(error);
    }
    Ok(())
}

/// Log only the local access-policy inputs needed to diagnose a rejection.
/// Authentication headers, cookies, request bodies, and arbitrary forwarded
/// headers are deliberately not inspected here.
fn log_local_access_failure(check: &str, headers: &HeaderMap, config: &ServerConfig) {
    tracing::warn!(
        event = "relay.access.local",
        outcome = "denied",
        check,
        received_origin = %header_for_log(headers, axum::http::header::ORIGIN),
        configured_origin = %config.origin.as_deref().map(crate::infrastructure::observability::safe_log_field).unwrap_or_else(|| "<unset>".to_string()),
        received_host = %header_for_log(headers, axum::http::header::HOST),
    );
}

fn header_for_log(headers: &HeaderMap, name: axum::http::header::HeaderName) -> String {
    let mut values = headers.get_all(name).iter();
    let Some(first) = values.next() else {
        return "<missing>".to_string();
    };
    if values.next().is_some() {
        return "<multiple>".to_string();
    }
    let value = first.to_str().unwrap_or("<invalid>");
    crate::infrastructure::observability::safe_log_field(value)
}

/// Decide whether forwarded HTTPS may be trusted for the remote listener.
/// Direct requests do not count as HTTPS: this relay is plaintext and only an
/// explicitly trusted proxy may assert the forwarded scheme.
pub fn trusted_proxy_https(
    peer: Option<IpAddr>,
    forwarded_proto: Option<&str>,
    trusted_proxy: bool,
    trusted_proxy_cidr: Option<&str>,
) -> bool {
    let trusted_peer = peer.zip(trusted_proxy_cidr).is_some_and(|(ip, cidr)| {
        cidr.parse::<ipnet::IpNet>()
            .is_ok_and(|net| net.contains(&ip))
    });
    trusted_proxy && trusted_peer && forwarded_proto == Some("https")
}
