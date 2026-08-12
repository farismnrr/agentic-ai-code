//! Streamable HTTP transport for the MCP `2026-07-28` server core.
//!
//! Single JSON-RPC route (`POST /mcp`) plus a plain `/health` probe used by
//! local tooling. Loopback binding is enforced by the caller/configuration
//! (`src/bin/relay-agent.rs` binds the validated loopback address) — this module
//! only builds the `Router`, it does not bind sockets.
//!
//! `/mcp` is additionally gated by [`super::security::enforce_local_access_policy`]
//! *before* any MCP/JSON-RPC parsing happens. The [`CorsLayer`] below is a
//! browser-side convenience only — it does not stop a non-browser HTTP
//! client from reaching this server, so it must never be relied on as the
//! security boundary. `/health` is intentionally left ungated: it is a
//! liveness probe with no sensitive data or side effects.
//!
//! Header/body validation below follows the official MCP `2026-07-28`
//! specification (`modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http`),
//! verified live for this implementation:
//! - `server/discover` is the modern, optional-to-call discovery method. The
//!   classic `initialize`/`initialized` lifecycle is also accepted for client
//!   compatibility.
//!   every other request is self-contained.
//! - Every request **MUST** carry `MCP-Protocol-Version`, `Mcp-Method`
//!   (mirroring `method`), and — for `tools/call` — `Mcp-Name` (mirroring
//!   `params.name`). A missing or body-mismatched standard header is
//!   rejected with HTTP `400` and JSON-RPC code `-32020` (`HeaderMismatch`).
//! - A protocol version the server doesn't implement is HTTP `400` with
//!   JSON-RPC code `-32022` (`UnsupportedProtocolVersion`), carrying
//!   `data: {supported, requested}`.
//! - Every request's `params._meta` carries
//!   `io.modelcontextprotocol/protocolVersion` (required, cross-checked
//!   against the header) and `io.modelcontextprotocol/clientCapabilities`
//!   (required, may be `{}`); `io.modelcontextprotocol/clientInfo` is
//!   optional. There is no server-side session — every request is
//!   validated independently.
//! - A notification (no `id`) that the server accepts gets `202 Accepted`
//!   with no body, never a JSON envelope.

use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, HeaderName, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::admission::RequestAdmission;
use super::auth;
use super::auth::{
    bearer_challenge_value, oauth_error_response, validate_claims, validate_token_signature,
    ClaimValidationError, Claims, TokenValidationError,
};
use super::auth::{CacheKeyDecision, CachedJwks};
use super::config::ServerConfig;
use super::error::McpError;
use super::mcp::{
    self, parse_request, tool_catalog, DiscoverResult, ErrorResponse, Id, Response, ToolCallResult,
    ToolsCallParams,
};
use super::observability::{audit, CorrelationId};
use super::security::enforce_local_access_policy;

/// Frozen in `.agents/plans/028-phase0-contract-audit.md` section 6: MCP
/// HTTP request body max.
pub const MAX_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

const HDR_PROTOCOL_VERSION: &str = "mcp-protocol-version";
const HDR_MCP_METHOD: &str = "mcp-method";
const HDR_MCP_NAME: &str = "mcp-name";
const TOOLS_LIST_TTL_MS: u64 = 300_000;
/// Resource permission exposed to the external Authorization Server and MCP
/// client for the default full-coding deployment profile.
const CODING_SCOPE: &str = "relay.coding";

/// JWKS cache TTL: 5 minutes. After this duration the cached key set is
/// considered stale and will be re-fetched on the next authentication attempt.
/// Maximum time to wait for a JWKS fetch response. Enforced via
/// `tokio::time::timeout` so a slow IdP endpoint cannot hold the write lock
/// indefinitely and deny authentication to all concurrent requests.
/// Coarse relay-side admission control for the remote MCP edge. The burst is
/// intentionally large enough for an agent's normal request burst, while the
/// refill rate bounds sustained floods. A long-running tool call consumes one
/// token when admitted; it does not consume tokens while it remains active.
pub struct AppState {
    pub config: ServerConfig,
    pub execution_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    /// Coarse request admission is deliberately separate from both the HTTP
    /// concurrency limit and the execution semaphore. It runs before OAuth
    /// validation so unauthenticated floods cannot reach JWKS or execution.
    pub request_admission: std::sync::Arc<RequestAdmission>,
    /// JWKS cache with TTL. `None` means not yet fetched.
    ///
    /// # OAuth boundary note (P1-3 / Phase 16.4)
    /// This server acts as an **OAuth 2.0 Resource Server only**. It validates
    /// Bearer access tokens presented in the `Authorization` header using JWKS
    /// from the configured issuer. The Authorization Code + PKCE S256 flow,
    /// state/CSRF parameter handling, and redirect URI validation are the
    /// responsibility of the Authorization Server or MCP client — not this
    /// server. Never accept authorization-server URLs or PKCE parameters from
    /// MCP tool arguments.
    pub jwks_cache: tokio::sync::RwLock<Option<CachedJwks>>,
}

#[derive(Clone, Debug, Default)]
pub enum AuthDecision {
    #[default]
    Authorized,
    Missing,
    InsufficientScope,
}

#[derive(Clone, Debug, Default)]
pub struct AuthContext {
    pub claims: Option<Claims>,
    pub decision: AuthDecision,
}

pub fn create_router(config: ServerConfig) -> Router {
    let cors_origin = match &config.origin {
        Some(origin) if origin != "*" => match origin.parse() {
            Ok(header_val) => AllowOrigin::exact(header_val),
            Err(_) => AllowOrigin::list(vec![]),
        },
        // Wildcard or missing origin both fail closed — never broaden trust.
        _ => AllowOrigin::list(vec![]),
    };

    // Explicit allow-list, not `Any`: only the headers this server's own
    // clients actually need to send. `Mcp-Param-*` (the optional
    // `x-mcp-header` tool-parameter mirroring extension) is not implemented
    // in this server, so it is deliberately not allow-listed here — add it
    // if/when that extension is implemented.
    let cors_headers = [
        axum::http::header::CONTENT_TYPE,
        HeaderName::from_static(HDR_PROTOCOL_VERSION),
        HeaderName::from_static(HDR_MCP_METHOD),
        HeaderName::from_static(HDR_MCP_NAME),
    ];

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(cors_headers);

    let state = Arc::new(AppState {
        config: config.clone(),
        execution_semaphore: Arc::new(tokio::sync::Semaphore::new(16)),
        request_admission: Arc::new(RequestAdmission::configured()),
        jwks_cache: tokio::sync::RwLock::new(None),
    });

    let mcp_router = Router::new().route("/mcp", post(handle_mcp));
    let mut well_known_router = Router::new().route(
        "/.well-known/oauth-protected-resource",
        get(handle_well_known_oauth),
    );
    // Axum 0.7's catch-all syntax is not usable here because the metadata
    // route must remain a concrete path. Register only the RFC 9728 path
    // derived from the configured resource (normally the resource's `/mcp`
    // path), leaving all other paths unmatched.
    if let Some(metadata_url) = auth::protected_resource_metadata_url(&config) {
        if let Ok(metadata_uri) = metadata_url.parse::<axum::http::Uri>() {
            let metadata_path = metadata_uri.path();
            if metadata_path != "/.well-known/oauth-protected-resource" {
                well_known_router =
                    well_known_router.route(metadata_path, get(handle_path_well_known_oauth));
            }
        }
    }

    Router::new()
        .route("/health", get(handle_health))
        .merge(mcp_router)
        .merge(well_known_router)
        .layer(middleware::from_fn_with_state(state.clone(), access_policy))
        .layer(middleware::from_fn(correlation_middleware))
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(state)
}

async fn handle_health() -> StatusCode {
    StatusCode::OK
}

async fn correlation_middleware(mut req: Request, next: Next) -> AxumResponse {
    let id = CorrelationId::from_request(&req);
    req.extensions_mut().insert(id.clone());
    let mut response = next.run(req).await;
    id.insert_response_header(&mut response);
    response
}

type JsonErr = Box<(StatusCode, HeaderMap, Json<ErrorResponse>)>;

fn err_response(status: StatusCode, id: Option<Id>, err: &McpError) -> JsonErr {
    Box::new((status, HeaderMap::new(), Json(ErrorResponse::new(id, err))))
}

fn json_error_response(error: JsonErr) -> AxumResponse {
    let (status, headers, body) = *error;
    (status, headers, body).into_response()
}

async fn handle_well_known_oauth(State(state): State<Arc<AppState>>) -> AxumResponse {
    Json(auth::metadata(&state.config)).into_response()
}

async fn handle_path_well_known_oauth(
    State(state): State<Arc<AppState>>,
    uri: axum::http::Uri,
) -> AxumResponse {
    let Some(metadata_url) = auth::protected_resource_metadata_url(&state.config) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(metadata_uri) = metadata_url.parse::<axum::http::Uri>() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if uri.path() != metadata_uri.path() {
        return StatusCode::NOT_FOUND.into_response();
    }
    handle_well_known_oauth(State(state)).await.into_response()
}

/// Server-side access policy:
/// If OAuth is configured, it validates the JWT Bearer token.
/// If OAuth is NOT configured (local mode), it runs exact `Origin` + `Host` validation.
async fn access_policy(
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

    if let super::config::SecurityMode::Remote = state.config.mode {
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
        let is_https = super::security::trusted_proxy_https(
            peer,
            forwarded_proto,
            state.config.trusted_proxy,
            state.config.trusted_proxy_cidr.as_deref(),
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
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("oauth_issuer is required for Remote mode".into()),
                    )),
                )
                    .into_response();
            }
        };

        let oauth_audience = match &state.config.oauth_audience {
            Some(a) => a.clone(),
            None => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("oauth_audience is required for Remote mode".into()),
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
            == Some("tools/call");

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
        let cache_needs_refresh = {
            let guard = state.jwks_cache.read().await;
            match guard.as_ref() {
                None => true,
                Some(c) => c.is_stale(std::time::Instant::now()),
            }
        };

        let jwks_url = if cache_needs_refresh {
            let discovered_url = match auth::fetch_discovery(client.clone(), discovery_url).await {
                Ok(url) => url,
                Err(msg) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            None,
                            &McpError::Internal(format!("OIDC discovery unavailable: {msg}")),
                        )),
                    )
                        .into_response();
                }
            };
            if !auth::validate_jwks_uri(&discovered_url, fixture_override) {
                if url::Url::parse(&discovered_url).is_err() {
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
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        None,
                        &McpError::Internal("OIDC discovery returned an unsafe jwks_uri".into()),
                    )),
                )
                    .into_response();
            }
            discovered_url
        } else {
            state
                .jwks_cache
                .read()
                .await
                .as_ref()
                .map(|cached| cached.jwks_uri().to_owned())
                .expect("fresh JWKS cache must contain its discovery URI")
        };

        if cache_needs_refresh {
            match auth::fetch_jwks(client.clone(), jwks_url.clone()).await {
                Ok(new_set) => {
                    let mut w = state.jwks_cache.write().await;
                    *w = Some(CachedJwks {
                        jwk_set: new_set,
                        jwks_uri: jwks_url.clone(),
                        fetched_at: std::time::Instant::now(),
                    });
                }
                Err(msg) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            None,
                            &McpError::Internal(format!("JWKS unavailable: {msg}")),
                        )),
                    )
                        .into_response();
                }
            }
        }

        // Decode token header to extract kid before acquiring the read lock.
        let header = match jsonwebtoken::decode_header(token) {
            Ok(h) => h,
            Err(_) => {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    None,
                    &state.config,
                    Some("invalid_token"),
                    None,
                    &McpError::InvalidRequest("Invalid token header".into()),
                );
            }
        };

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
        let jwk_opt = {
            let guard = state.jwks_cache.read().await;
            guard.as_ref().and_then(|c| c.find_key(&kid))
        };

        let jwk = match auth::key_lookup_decision(jwk_opt.is_some()) {
            CacheKeyDecision::Found => jwk_opt.expect("found cache key must be present"),
            CacheKeyDecision::RefreshOnce => {
                // kid not in cache — attempt a single re-fetch.
                match auth::fetch_jwks(client, jwks_url.clone()).await {
                    Ok(new_set) => {
                        let found = new_set.find(&kid).cloned();
                        let mut w = state.jwks_cache.write().await;
                        *w = Some(CachedJwks {
                            jwk_set: new_set,
                            jwks_uri: jwks_url.clone(),
                            fetched_at: std::time::Instant::now(),
                        });
                        match found {
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
                        }
                    }
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
        auth_ctx.claims = Some(claims);

        let claims = auth_ctx.claims.as_ref().expect("claims were just stored");
        match validate_claims(
            claims,
            state.config.oauth_owner_subject.as_deref(),
            CODING_SCOPE,
        ) {
            Ok(()) => {}
            Err(ClaimValidationError::UnauthorizedSubject) => {
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
    } else {
        if let Err(err) = enforce_local_access_policy(req.headers(), &state.config) {
            return (StatusCode::FORBIDDEN, Json(ErrorResponse::new(None, &err))).into_response();
        }
    }

    req.extensions_mut().insert(auth_ctx);
    next.run(req).await
}

async fn handle_mcp(
    State(_state): State<Arc<AppState>>,
    axum::extract::Extension(auth_ctx): axum::extract::Extension<AuthContext>,
    axum::extract::Extension(correlation_id): axum::extract::Extension<CorrelationId>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AxumResponse {
    let started = Instant::now();
    // Content-Type must be application/json for the Streamable HTTP JSON
    // mode used in this phase (no SSE upgrade implemented yet — see audit
    // doc section 3).
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if let Err(err) = super::transport_validation::validate_content_type(Some(content_type)) {
        let response = err_response(StatusCode::BAD_REQUEST, None, &err).into_response();
        audit(
            correlation_id.as_str(),
            "http",
            None,
            "invalid_content_type",
            StatusCode::BAD_REQUEST,
            started,
            auth_ctx.claims.as_ref().and_then(|c| c.sub.as_deref()),
        );
        return response;
    }

    // Body size is already bounded by DefaultBodyLimit before this handler
    // runs; parse only after that gate, never before.
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            let response = json_error_response(err_response(
                StatusCode::BAD_REQUEST,
                None,
                &McpError::ParseError,
            ));
            audit(
                correlation_id.as_str(),
                "http",
                None,
                "parse_error",
                StatusCode::BAD_REQUEST,
                started,
                auth_ctx.claims.as_ref().and_then(|c| c.sub.as_deref()),
            );
            return response;
        }
    };

    let request = match parse_request(&payload) {
        Ok(req) => req,
        Err(None) => {
            // A notification the server accepts: MCP `2026-07-28` mandates
            // `202 Accepted` with no body, never a JSON envelope. Header
            // requirements for notification POSTs are explicitly left
            // undefined by this revision (see module docs), so no further
            // validation runs here.
            return StatusCode::ACCEPTED.into_response();
        }
        Err(Some(mcp_err)) => {
            let id = payload
                .get("id")
                .and_then(|v| serde_json::from_value::<Id>(v.clone()).ok());
            return json_error_response(err_response(StatusCode::BAD_REQUEST, id, &mcp_err));
        }
    };

    // The legacy lifecycle is intentionally the only path that bypasses the
    // modern 2026 routing headers, allowing standard MCP clients to discover
    // this server before switching to the existing request contract.
    if request.method == "initialize" {
        return handle_initialize(&request)
            .map_or_else(json_error_response, |body| body.into_response());
    }

    // Legacy clients do not send the 2026 request metadata or routing
    // headers on the follow-up tools/list request. Keep this compatibility
    // exception narrow; a tools/list request that presents any modern header
    // or metadata remains subject to the strict 2026 validation below.
    let legacy_tools_list = super::transport_validation::is_legacy_tools_list(&headers, &request);
    if legacy_tools_list {
        return handle_tools_list(&request)
            .map_or_else(json_error_response, |body| body.into_response());
    }

    if let Err(err) = super::transport_validation::validate_routing_headers(&headers, &request) {
        return json_error_response(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &err,
        ));
    }

    match super::dispatcher::dispatch(&request) {
        super::dispatcher::Dispatch::Discover => handle_discover(&request),
        super::dispatcher::Dispatch::ToolsList => handle_tools_list(&request),
        super::dispatcher::Dispatch::ToolsCall => {
            handle_tools_call(&request, _state, auth_ctx, correlation_id.as_str()).await
        }
        super::dispatcher::Dispatch::Unknown(other) => Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::MethodNotFound(other),
        )),
    }
    .map_or_else(json_error_response, |body| body.into_response())
}

fn handle_discover(request: &mcp::Request) -> JsonErr2 {
    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(DiscoverResult::current()).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_initialize(request: &mcp::Request) -> JsonErr2 {
    let params = request.params.as_ref().and_then(Value::as_object);
    let Some(requested) = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
    else {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidRequest("initialize.params.protocolVersion is required".into()),
        ));
    };
    let supported: Vec<&str> = std::iter::once(mcp::PROTOCOL_VERSION)
        .chain(mcp::LEGACY_PROTOCOL_VERSIONS.iter().copied())
        .collect();
    if !supported.contains(&requested) {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::UnsupportedProtocolVersion {
                supported: supported
                    .iter()
                    .map(|version| (*version).to_owned())
                    .collect(),
                requested: requested.to_owned(),
            },
        ));
    }

    let response = Response::new(
        request.id.clone(),
        json!({
            "protocolVersion": requested,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "relay-agent", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Coding server providing a sandboxed coding terminal, configured HTTP requests, and web search within the configured workspace policy."
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_tools_list(request: &mcp::Request) -> JsonErr2 {
    let tools = tool_catalog();
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete",
            "ttlMs": TOOLS_LIST_TTL_MS,
            "cacheScope": "public",
            "tools": tools
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

async fn handle_tools_call(
    request: &mcp::Request,
    state: Arc<AppState>,
    auth_ctx: AuthContext,
    correlation_id: &str,
) -> JsonErr2 {
    let auth_challenge = match auth_ctx.decision {
        AuthDecision::Authorized => None,
        AuthDecision::Missing => Some(("invalid_token", None)),
        AuthDecision::InsufficientScope => Some(("insufficient_scope", Some(CODING_SCOPE))),
    };
    if let Some((error, scope)) = auth_challenge {
        let challenge = bearer_challenge_value(&state.config, Some(error), scope);
        let result = ToolCallResult::error(vec![super::mcp::ToolResultContent {
            kind: "text",
            text: "Authentication is required to use this tool".to_string(),
        }])
        .with_meta(json!({ "mcp/www_authenticate": [challenge] }));
        let response = Response::new(
            request.id.clone(),
            serde_json::to_value(result).unwrap_or(json!({})),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    let params_val = request.params.clone().ok_or_else(|| {
        err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams("missing tools/call parameters".to_string()),
        )
    })?;

    let call: ToolsCallParams = serde_json::from_value(params_val).map_err(|e| {
        err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams(format!("invalid tools/call parameters: {e}")),
        )
    })?;

    let Some(tool) = mcp::find_tool(&call.name) else {
        return Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::InvalidParams(format!("unknown tool '{}'", call.name)),
        ));
    };

    if let Err(err) = mcp::validate_tool_arguments(&tool, &call.arguments) {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &err,
        ));
    }

    if let super::config::SecurityMode::Remote = state.config.mode {
        let claims = auth_ctx
            .claims
            .as_ref()
            .expect("authorized remote requests have validated claims");

        let subject = claims.sub.as_deref().unwrap_or("unknown");
        audit(
            correlation_id,
            "tools/call",
            Some(&call.name),
            "authorized",
            StatusCode::OK,
            Instant::now(),
            Some(subject),
        );
    }

    // Acquire global execution permit
    let _permit = match std::sync::Arc::clone(&state.execution_semaphore)
        .acquire_owned()
        .await
    {
        Ok(p) => p,
        Err(_) => {
            return Err(err_response(
                StatusCode::SERVICE_UNAVAILABLE,
                Some(request.id.clone()),
                &McpError::Internal("Execution system unavailable".to_string()),
            ));
        }
    };

    // Tool exists in the registry and both the request shape and its
    // actual execution is Phase 3 scope.
    let result =
        crate::relay_agent::execution::dispatch_tool_call(&tool, &call.arguments, &state.config)
            .await
            .unwrap_or_else(|e| {
                ToolCallResult::error(vec![super::mcp::ToolResultContent {
                    kind: "text",
                    text: format!("execution failed: {}", e.message()),
                }])
            });

    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(result).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

/// The three dispatch handlers above return `Json<Value>` on success or a
/// pre-built `JsonErr` on failure, and are converted to a full
/// [`AxumResponse`] by `.into_response()` in [`handle_mcp`] — this alias
/// just keeps their signatures short.
type JsonErr2 = Result<Json<Value>, JsonErr>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_initialize_and_tools_list_return_catalog() {
        let initialize = mcp::Request {
            jsonrpc: "2.0".into(),
            id: Id::Number(1),
            method: "initialize".into(),
            params: Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "ChatGPT", "version": "test" }
            })),
        };
        let initialized = handle_initialize(&initialize)
            .expect("initialize succeeds")
            .0;
        assert_eq!(initialized["result"]["protocolVersion"], "2025-03-26");

        let tools_list = mcp::Request {
            jsonrpc: "2.0".into(),
            id: Id::Number(2),
            method: "tools/list".into(),
            params: None,
        };
        let response = handle_tools_list(&tools_list)
            .expect("tools/list succeeds")
            .0;
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tool catalog");
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|tool| tool["name"] == "terminal_exec"));

        let mut legacy_headers = HeaderMap::new();
        legacy_headers.insert(HDR_PROTOCOL_VERSION, "2025-03-26".parse().unwrap());
        assert!(
            crate::relay_agent::transport_validation::is_legacy_tools_list(
                &legacy_headers,
                &tools_list
            )
        );

        let mut modern_headers = HeaderMap::new();
        modern_headers.insert(HDR_PROTOCOL_VERSION, mcp::PROTOCOL_VERSION.parse().unwrap());
        assert!(
            !crate::relay_agent::transport_validation::is_legacy_tools_list(
                &modern_headers,
                &tools_list
            )
        );
    }
}
