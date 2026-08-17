//! HTTP admission, local-access, trusted-proxy, and OAuth resource-server policy.

use super::{AppState, AuthContext, AuthDecision, CODING_SCOPE, HDR_MCP_METHOD};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use std::sync::Arc;
use std::time::Instant;

use crate::auth;
use crate::auth::{
    oauth_error_response, validate_claims, validate_token_signature, CacheKeyDecision,
    ClaimValidationError, TokenValidationError,
};
use crate::observability::safe_log_field;
use crate::security::enforce_local_access_policy;
use relay_core::error::McpError;
use relay_interfaces::mcp::ErrorResponse;

/// Server-side access policy:
/// If OAuth is configured, it validates the JWT Bearer token.
/// If OAuth is NOT configured (local mode), it validates an optional exact
/// `Origin` plus the mandatory loopback `Host`.
pub(super) async fn access_policy(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> AxumResponse {
    // Only apply policy to /mcp
    if req.uri().path() != "/mcp" {
        return next.run(req).await;
    }

    // Admit before parsing, OAuth/JWKS work, or tool dispatch. This protects
    // the expensive/authenticated path without changing the separate HTTP
    // concurrency and execution semaphore limits.
    if !state.request_admission.try_acquire(Instant::now()) {
        tracing::warn!(event = "relay.admission", outcome = "denied");
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            Json(ErrorResponse::new(
                None,
                &McpError::InvalidRequest("Request temporarily unavailable".into()),
            )),
        )
            .into_response();
    }

    let mut auth_ctx = AuthContext::default();

    if let relay_core::config::SecurityMode::Remote = state.config.mode {
        // This listener is plaintext by design. The only supported HTTPS
        // termination point is an explicitly trusted local edge/tunnel. Do
        // not treat the request URI scheme as proof of TLS: a direct peer can
        // supply an absolute-form HTTP request target. Likewise, forwarded
        // headers are ignored unless the operator explicitly opted in and the
        // configuration validation has restricted the listener to loopback.
        let peer = req
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|peer| peer.0.ip());
        let forwarded_proto = req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok());
        let is_https = crate::security::trusted_proxy_https(
            peer,
            forwarded_proto,
            state.config.trusted_proxy,
            state.config.trusted_proxy_cidr.as_deref(),
        );

        tracing::debug!(
            event = "relay.access.trusted_proxy",
            outcome = if is_https { "https" } else { "not_https" }
        );
        if !is_https {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    None,
                    &McpError::InvalidRequest("Remote mode requires HTTPS".into()),
                )),
            )
                .into_response();
        }

        let oauth_issuer = match &state.config.oauth_issuer {
            Some(i) => i.clone(),
            None => {
                tracing::error!(
                    event = "relay.auth.validate",
                    outcome = "config_missing",
                    reason = "oauth_issuer_missing"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("OAuth configuration is unavailable".into()),
                    )),
                )
                    .into_response();
            }
        };

        let oauth_audience = match &state.config.oauth_audience {
            Some(a) => a.clone(),
            None => {
                tracing::error!(
                    event = "relay.auth.validate",
                    outcome = "config_missing",
                    reason = "oauth_audience_missing"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("OAuth configuration is unavailable".into()),
                    )),
                )
                    .into_response();
            }
        };

        let auth_header = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();

        let is_tools_call = req
            .headers()
            .get(HDR_MCP_METHOD)
            .and_then(|value| value.to_str().ok())
            .map(|method| {
                matches!(
                    method,
                    "tools/call" | "tasks/get" | "tasks/update" | "tasks/cancel"
                )
            })
            .unwrap_or(false);

        if !auth_header.starts_with("Bearer ") {
            if auth_header.is_empty() && is_tools_call {
                auth_ctx.decision = AuthDecision::Missing;
                req.extensions_mut().insert(auth_ctx);
                return next.run(req).await;
            }

            return oauth_error_response(
                StatusCode::UNAUTHORIZED,
                None,
                &state.config,
                (!auth_header.is_empty()).then_some("invalid_token"),
                None,
                &McpError::InvalidRequest("Missing or invalid authorization".into()),
            );
        }

        let token = &auth_header[7..];

        // Cheap structural/JWT-header rejection before any discovery/JWKS
        // network I/O. This must not weaken validation for well-formed
        // tokens: only tokens that could not possibly be a JWT are rejected
        // here, and they get the identical invalid_token/401 response that
        // a missing bearer token receives.
        let header = match auth::parse_structurally_plausible_jwt(token) {
            Some(header) => {
                tracing::debug!(
                    event = "relay.auth.validate",
                    outcome = "structurally_valid"
                );
                header
            }
            None => {
                tracing::warn!(
                    event = "relay.auth.validate",
                    outcome = "structurally_invalid"
                );
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    None,
                    &state.config,
                    Some("invalid_token"),
                    None,
                    &McpError::InvalidRequest("Missing or invalid authorization".into()),
                );
            }
        };

        // P1-2: OAuth metadata and JWKS fetch helpers with timeout. Drops the lock before the HTTP
        // request so a slow IdP cannot hold the write lock and block all auth.
        let client = auth::http_client();

        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            oauth_issuer.trim_end_matches('/')
        );
        let fixture_override = cfg!(debug_assertions)
            && std::env::var("RELAY_AGENT_ALLOW_INSECURE_OAUTH_ISSUER_FIXTURE").as_deref()
                == Ok("1");
        // Check whether the cache is present and fresh (TTL not exceeded).
        let cache_snapshot =
            auth::read_cache_snapshot(&state.jwks_cache, std::time::Instant::now()).await;
        let cache_needs_refresh = cache_snapshot.needs_refresh;

        let jwks_url = if cache_needs_refresh {
            let discovered_url = match auth::fetch_discovery(client.clone(), discovery_url).await {
                Ok(url) => url,
                Err(msg) => {
                    // The raw upstream/network/OIDC error text (`msg`) may
                    // contain hostnames, TLS errors, or HTTP status text —
                    // it goes ONLY into private telemetry, never the
                    // client-visible body (Plan 035 Phase 9, confirmed leak
                    // at the pre-fix baseline).
                    tracing::error!(
                        event = "relay.auth.discovery",
                        outcome = "error",
                        detail = safe_log_field(&msg)
                    );
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            None,
                            &McpError::Internal("OIDC discovery unavailable".into()),
                        )),
                    )
                        .into_response();
                }
            };
            if !auth::validate_jwks_uri(&discovered_url, fixture_override) {
                if url::Url::parse(&discovered_url).is_err() {
                    tracing::error!(event = "relay.auth.discovery", outcome = "invalid_jwks_uri");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            None,
                            &McpError::Internal(
                                "OIDC discovery returned an invalid jwks_uri".into(),
                            ),
                        )),
                    )
                        .into_response();
                }
                tracing::error!(event = "relay.auth.discovery", outcome = "unsafe_jwks_uri");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("OIDC discovery returned an unsafe jwks_uri".into()),
                    )),
                )
                    .into_response();
            }
            tracing::debug!(event = "relay.auth.discovery", outcome = "refreshed");
            discovered_url
        } else {
            cache_snapshot
                .jwks_uri
                .expect("fresh JWKS cache must contain its discovery URI")
        };

        if cache_needs_refresh {
            match auth::refresh_cache(&state.jwks_cache, client.clone(), jwks_url.clone()).await {
                Ok(()) => {
                    tracing::debug!(event = "relay.auth.discovery", outcome = "jwks_refreshed");
                }
                Err(msg) => {
                    // Same private-only-detail rule as the discovery leak
                    // above: `msg` (raw JWKS fetch/parse error) never
                    // reaches the client.
                    tracing::error!(
                        event = "relay.auth.discovery",
                        outcome = "jwks_error",
                        detail = safe_log_field(&msg)
                    );
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            None,
                            &McpError::Internal("JWKS unavailable".into()),
                        )),
                    )
                        .into_response();
                }
            }
        }

        let kid = match header.kid {
            Some(k) => k,
            None => {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    None,
                    &state.config,
                    Some("invalid_token"),
                    None,
                    &McpError::InvalidRequest("Token missing kid".into()),
                );
            }
        };

        // Try to find the JWK for the token's kid. If it's not found in the
        // current cache, do ONE re-fetch (refresh-on-unknown-kid) in case the
        // IdP rotated keys since our last fetch. We never loop to prevent
        // attacker-controlled refresh storms.
        let jwk_opt = auth::lookup_cached_key(&state.jwks_cache, &kid).await;

        let jwk = match auth::key_lookup_decision(jwk_opt.is_some()) {
            CacheKeyDecision::Found => jwk_opt.expect("found cache key must be present"),
            CacheKeyDecision::RefreshOnce => {
                // kid not in cache — attempt a single re-fetch.
                match auth::refresh_cache_for_kid(&state.jwks_cache, client, jwks_url.clone(), &kid)
                    .await
                {
                    Ok(found) => match found {
                        Some(j) => j,
                        None => {
                            return oauth_error_response(
                                StatusCode::UNAUTHORIZED,
                                None,
                                &state.config,
                                Some("invalid_token"),
                                None,
                                &McpError::InvalidRequest(
                                    "Unknown signing key (kid not in JWKS)".into(),
                                ),
                            );
                        }
                    },
                    Err(_) => {
                        return oauth_error_response(
                            StatusCode::UNAUTHORIZED,
                            None,
                            &state.config,
                            Some("invalid_token"),
                            None,
                            &McpError::InvalidRequest(
                                "Unknown signing key (kid not in JWKS)".into(),
                            ),
                        );
                    }
                }
            }
        };

        let claims =
            match validate_token_signature(token, &jwk, header.alg, &oauth_issuer, &oauth_audience)
            {
                Ok(claims) => claims,
                Err(TokenValidationError::InvalidJwk) => {
                    return oauth_error_response(
                        StatusCode::UNAUTHORIZED,
                        None,
                        &state.config,
                        Some("invalid_token"),
                        None,
                        &McpError::InvalidRequest("Invalid JWK".into()),
                    )
                }
                Err(TokenValidationError::UnsupportedAlgorithm) => {
                    return oauth_error_response(
                        StatusCode::UNAUTHORIZED,
                        None,
                        &state.config,
                        Some("invalid_token"),
                        None,
                        &McpError::InvalidRequest("Unsupported token algorithm".into()),
                    )
                }
                Err(TokenValidationError::InvalidToken) => {
                    return oauth_error_response(
                        StatusCode::UNAUTHORIZED,
                        None,
                        &state.config,
                        Some("invalid_token"),
                        None,
                        &McpError::InvalidRequest("Invalid token".into()),
                    )
                }
            };
        tracing::debug!(event = "relay.auth.validate", outcome = "signature_valid");
        auth_ctx.claims = Some(claims);

        let claims = auth_ctx.claims.as_ref().expect("claims were just stored");
        match validate_claims(
            claims,
            state.config.oauth_owner_subject.as_deref(),
            CODING_SCOPE,
        ) {
            Ok(()) => {
                tracing::debug!(event = "relay.auth.validate", outcome = "authorized");
            }
            Err(ClaimValidationError::UnauthorizedSubject) => {
                tracing::warn!(
                    event = "relay.auth.validate",
                    outcome = "unauthorized_subject"
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::InvalidRequest("Authenticated subject is not authorized".into()),
                    )),
                )
                    .into_response();
            }
            Err(ClaimValidationError::InsufficientScope) => {
                tracing::warn!(
                    event = "relay.auth.validate",
                    outcome = "insufficient_scope"
                );
                if is_tools_call {
                    auth_ctx.decision = AuthDecision::InsufficientScope;
                    req.extensions_mut().insert(auth_ctx);
                    return next.run(req).await;
                }

                return oauth_error_response(
                    StatusCode::FORBIDDEN,
                    None,
                    &state.config,
                    Some("insufficient_scope"),
                    Some(CODING_SCOPE),
                    &McpError::InvalidRequest("Insufficient scope".into()),
                );
            }
        }
    } else if let Err(err) = enforce_local_access_policy(req.headers(), &state.config) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new(None, &err))).into_response();
    } else {
        tracing::debug!(event = "relay.access.local", outcome = "allowed");
    }

    req.extensions_mut().insert(auth_ctx);
    next.run(req).await
}
