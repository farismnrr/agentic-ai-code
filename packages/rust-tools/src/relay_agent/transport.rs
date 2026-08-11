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
//! - There is no `initialize`/`initialized` handshake in this revision.
//!   `server/discover` is the modern, optional-to-call discovery method;
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

use super::config::ServerConfig;
use super::error::McpError;
use super::mcp::{
    self, decode_header_value, extract_meta, parse_request, tool_catalog, DiscoverResult,
    ErrorResponse, Id, Response, ToolCallResult, ToolsCallParams,
};
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
const JWKS_TTL_SECS: u64 = 300;

/// Maximum time to wait for a JWKS fetch response. Enforced via
/// `tokio::time::timeout` so a slow IdP endpoint cannot hold the write lock
/// indefinitely and deny authentication to all concurrent requests.
const JWKS_FETCH_TIMEOUT_SECS: u64 = 10;
const MAX_LOG_FIELD: usize = 128;
const HDR_CORRELATION_ID: &str = "x-correlation-id";

fn safe_log_field(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_LOG_FIELD)
        .collect()
}

fn privacy_id(value: Option<&str>) -> &'static str {
    // Deliberately do not emit subject, client identifiers, commands, arguments,
    // source, or token material. Presence is sufficient for operational metrics.
    if value.is_some() {
        "present"
    } else {
        "absent"
    }
}

fn audit(
    correlation_id: &str,
    method: &str,
    tool: Option<&str>,
    outcome: &str,
    status: StatusCode,
    started: Instant,
    subject: Option<&str>,
) {
    let tool = tool.map(safe_log_field).unwrap_or_else(|| "-".into());
    eprintln!(
        "{}",
        json!({
            "event": "relay_request",
            "correlation_id": safe_log_field(correlation_id),
            "method": safe_log_field(method),
            "tool": tool,
            "outcome": safe_log_field(outcome),
            "status": status.as_u16(),
            "latency_ms": started.elapsed().as_millis(),
            "subject": privacy_id(subject)
        })
    );
}

/// Cached JWKS with a fetch timestamp for TTL enforcement.
/// Only used within this module; making it `pub` satisfies the
/// `private_interfaces` lint because `AppState::jwks_cache` is `pub`.
pub struct CachedJwks {
    pub(super) jwk_set: jsonwebtoken::jwk::JwkSet,
    pub(super) fetched_at: std::time::Instant,
}

pub struct AppState {
    pub config: ServerConfig,
    pub execution_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Claims {
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
}

#[derive(Clone, Default)]
pub struct AuthContext {
    pub claims: Option<Claims>,
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
        jwks_cache: tokio::sync::RwLock::new(None),
    });

    let mcp_router = Router::new().route("/mcp", post(handle_mcp));
    let well_known_router = Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(handle_well_known_oauth),
        )
        .route(
            "/.well-known/oauth-protected-resource/{*resource_path}",
            get(handle_path_well_known_oauth),
        );

    Router::new()
        .merge(mcp_router)
        .merge(well_known_router)
        .layer(middleware::from_fn_with_state(state.clone(), access_policy))
        .layer(middleware::from_fn(correlation_middleware))
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(state)
}

async fn correlation_middleware(mut req: Request, next: Next) -> AxumResponse {
    let id = req
        .headers()
        .get(HDR_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() <= MAX_LOG_FIELD && v.chars().all(|c| c.is_ascii_graphic() && c != '"'))
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    req.extensions_mut().insert(id.clone());
    let mut response = next.run(req).await;
    if let Ok(value) = id.parse() {
        response
            .headers_mut()
            .insert(HeaderName::from_static(HDR_CORRELATION_ID), value);
    }
    response
}

type JsonErr = Box<(StatusCode, HeaderMap, Json<ErrorResponse>)>;

fn protected_resource_metadata_url(config: &ServerConfig) -> Option<String> {
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

fn bearer_challenge(config: &ServerConfig, error: Option<&str>, scope: Option<&str>) -> HeaderMap {
    let mut parameters = vec!["realm=\"mcp\"".to_owned()];
    if let Some(error) = error {
        parameters.push(format!("error=\"{error}\""));
    }
    if let Some(scope) = scope {
        parameters.push(format!("scope=\"{scope}\""));
    }
    if let Some(metadata_url) = protected_resource_metadata_url(config) {
        parameters.push(format!("resource_metadata=\"{metadata_url}\""));
    }

    let mut headers = HeaderMap::new();
    if let Ok(value) = format!("Bearer {}", parameters.join(", ")).parse() {
        headers.insert(axum::http::header::WWW_AUTHENTICATE, value);
    }
    headers
}

fn oauth_error_response(
    status: StatusCode,
    id: Option<Id>,
    config: &ServerConfig,
    error: Option<&str>,
    scope: Option<&str>,
    message: &McpError,
) -> AxumResponse {
    (
        status,
        bearer_challenge(config, error, scope),
        Json(ErrorResponse::new(id, message)),
    )
        .into_response()
}

fn request_uses_trusted_https(req: &Request, config: &ServerConfig) -> bool {
    config.trusted_proxy
        && req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https")
}

fn err_response(status: StatusCode, id: Option<Id>, err: &McpError) -> JsonErr {
    Box::new((status, HeaderMap::new(), Json(ErrorResponse::new(id, err))))
}

fn json_error_response(error: JsonErr) -> AxumResponse {
    let (status, headers, body) = *error;
    (status, headers, body).into_response()
}

async fn handle_well_known_oauth(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let issuer = state.config.oauth_issuer.clone();
    let resource = state.config.oauth_audience.clone();
    let mut metadata = json!({
        "resource": resource,
        "scopes_supported": [CODING_SCOPE]
    });
    if let Some(issuer) = issuer {
        metadata["authorization_servers"] = json!([issuer]);
    }
    Json(metadata)
}

async fn handle_path_well_known_oauth(
    State(state): State<Arc<AppState>>,
    uri: axum::http::Uri,
) -> AxumResponse {
    let Some(metadata_url) = protected_resource_metadata_url(&state.config) else {
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

    let mut auth_ctx = AuthContext::default();

    if let super::config::SecurityMode::Remote = state.config.mode {
        // This listener is plaintext by design. The only supported HTTPS
        // termination point is an explicitly trusted local edge/tunnel. Do
        // not treat the request URI scheme as proof of TLS: a direct peer can
        // supply an absolute-form HTTP request target. Likewise, forwarded
        // headers are ignored unless the operator explicitly opted in and the
        // configuration validation has restricted the listener to loopback.
        let is_https = request_uses_trusted_https(&req, &state.config);

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

        if !auth_header.starts_with("Bearer ") {
            let error = if auth_header.is_empty() {
                None
            } else {
                Some("invalid_token")
            };
            return oauth_error_response(
                StatusCode::UNAUTHORIZED,
                None,
                &state.config,
                error,
                None,
                &McpError::InvalidRequest("Missing or invalid authorization".into()),
            );
        }

        let token = &auth_header[7..];

        // P1-2: JWKS fetch helper with timeout. Drops the lock before the HTTP
        // request so a slow IdP cannot hold the write lock and block all auth.
        let fetch_jwks = |jwks_url: String| async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(JWKS_FETCH_TIMEOUT_SECS))
                .build()
                .map_err(|_| "failed to build HTTP client".to_string())?;
            let resp = client
                .get(&jwks_url)
                .send()
                .await
                .map_err(|e| format!("JWKS fetch failed: {e}"))?;
            let jwks = resp
                .json::<jsonwebtoken::jwk::JwkSet>()
                .await
                .map_err(|e| format!("JWKS parse failed: {e}"))?;
            Ok::<jsonwebtoken::jwk::JwkSet, String>(jwks)
        };

        let jwks_url = format!(
            "{}/.well-known/jwks.json",
            oauth_issuer.trim_end_matches('/')
        );

        // Check whether the cache is present and fresh (TTL not exceeded).
        let cache_needs_refresh = {
            let guard = state.jwks_cache.read().await;
            match guard.as_ref() {
                None => true,
                Some(c) => c.fetched_at.elapsed().as_secs() >= JWKS_TTL_SECS,
            }
        };

        if cache_needs_refresh {
            match fetch_jwks(jwks_url.clone()).await {
                Ok(new_set) => {
                    let mut w = state.jwks_cache.write().await;
                    *w = Some(CachedJwks {
                        jwk_set: new_set,
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
            guard.as_ref().and_then(|c| c.jwk_set.find(&kid)).cloned()
        };

        let jwk = match jwk_opt {
            Some(j) => j,
            None => {
                // kid not in cache — attempt a single re-fetch.
                match fetch_jwks(jwks_url).await {
                    Ok(new_set) => {
                        let found = new_set.find(&kid).cloned();
                        let mut w = state.jwks_cache.write().await;
                        *w = Some(CachedJwks {
                            jwk_set: new_set,
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

        let decoding_key = match jsonwebtoken::DecodingKey::from_jwk(&jwk) {
            Ok(k) => k,
            Err(_) => {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    None,
                    &state.config,
                    Some("invalid_token"),
                    None,
                    &McpError::InvalidRequest("Invalid JWK".into()),
                );
            }
        };

        let mut validation = jsonwebtoken::Validation::new(header.alg);
        validation.algorithms = vec![
            jsonwebtoken::Algorithm::RS256,
            jsonwebtoken::Algorithm::ES256,
        ];
        validation.set_audience(&[oauth_audience]);
        validation.set_issuer(&[oauth_issuer]);
        validation.validate_nbf = true;

        let token_data = match jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation) {
            Ok(data) => data,
            Err(_) => {
                return oauth_error_response(
                    StatusCode::UNAUTHORIZED,
                    None,
                    &state.config,
                    Some("invalid_token"),
                    None,
                    &McpError::InvalidRequest("Invalid token".into()),
                );
            }
        };
        auth_ctx.claims = Some(token_data.claims);
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
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AxumResponse {
    let started = Instant::now();
    let correlation_id = headers
        .get(HDR_CORRELATION_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    // Content-Type must be application/json for the Streamable HTTP JSON
    // mode used in this phase (no SSE upgrade implemented yet — see audit
    // doc section 3).
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !content_type.starts_with("application/json") {
        let response = err_response(
            StatusCode::BAD_REQUEST,
            None,
            &McpError::InvalidRequest(format!(
                "unsupported Content-Type '{content_type}', expected application/json"
            )),
        )
        .into_response();
        audit(
            correlation_id,
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
                correlation_id,
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

    if let Err(err) = validate_routing_headers(&headers, &request) {
        return json_error_response(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &err,
        ));
    }

    match request.method.as_str() {
        "server/discover" => handle_discover(&request),
        "tools/list" => handle_tools_list(&request),
        "tools/call" => handle_tools_call(&request, _state, auth_ctx, correlation_id).await,
        other => Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::MethodNotFound(other.to_string()),
        )),
    }
    .map_or_else(json_error_response, |body| body.into_response())
}

/// Validate the MCP `2026-07-28` standard request-metadata headers against
/// the parsed JSON-RPC body, per `streamable-http#server-validation`. Every
/// failure here is `-32020 HeaderMismatch` — the spec does not distinguish
/// "missing" from "mismatched" at the error-code level, only in the human
/// `message`.
///
/// Order: `MCP-Protocol-Version` (including the separate `-32022`
/// unsupported-version case), then `Mcp-Method`, then `Mcp-Name` (only for
/// methods that carry a name — `tools/call` in this server's scope).
fn validate_routing_headers(headers: &HeaderMap, request: &mcp::Request) -> Result<(), McpError> {
    let protocol_header = headers
        .get(HDR_PROTOCOL_VERSION)
        .and_then(|v| v.to_str().ok());

    let protocol_value = match protocol_header {
        None => {
            return Err(McpError::HeaderMismatch(format!(
                "required standard header '{HDR_PROTOCOL_VERSION}' is missing"
            )));
        }
        Some(v) if v != mcp::PROTOCOL_VERSION => {
            return Err(McpError::UnsupportedProtocolVersion {
                supported: vec![mcp::PROTOCOL_VERSION.to_string()],
                requested: v.to_string(),
            });
        }
        Some(v) => v,
    };

    let meta = extract_meta(request.params.as_ref());
    let meta_protocol_version = meta.as_ref().and_then(|m| m.protocol_version.as_deref());
    match meta_protocol_version {
        Some(v) if v == protocol_value => {}
        Some(v) => {
            return Err(McpError::HeaderMismatch(format!(
                "'{HDR_PROTOCOL_VERSION}' header value '{protocol_value}' does not match body \
                 params._meta['io.modelcontextprotocol/protocolVersion'] value '{v}'"
            )));
        }
        None => {
            return Err(McpError::HeaderMismatch(
                "required params._meta['io.modelcontextprotocol/protocolVersion'] is missing \
                 from the request body"
                    .to_string(),
            ));
        }
    }

    if meta
        .as_ref()
        .and_then(|m| m.client_capabilities.as_ref())
        .is_none()
    {
        return Err(McpError::HeaderMismatch(
            "required params._meta['io.modelcontextprotocol/clientCapabilities'] is missing \
             from the request body"
                .to_string(),
        ));
    }

    let mcp_method_header = headers.get(HDR_MCP_METHOD).and_then(|v| v.to_str().ok());
    match mcp_method_header {
        None => {
            return Err(McpError::HeaderMismatch(format!(
                "required standard header '{HDR_MCP_METHOD}' is missing"
            )));
        }
        Some(v) if v != request.method => {
            return Err(McpError::HeaderMismatch(format!(
                "'{HDR_MCP_METHOD}' header value '{v}' does not match body method '{}'",
                request.method
            )));
        }
        Some(_) => {}
    }

    // Mcp-Name is only required for methods that carry a name/uri in their
    // params — of the three listed in the spec (`tools/call`,
    // `resources/read`, `prompts/get`), only `tools/call` is implemented by
    // this server.
    if request.method == "tools/call" {
        let expected_name = request
            .params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str());

        let header_raw = headers.get(HDR_MCP_NAME).and_then(|v| v.to_str().ok());

        match (header_raw, expected_name) {
            (None, _) => {
                return Err(McpError::HeaderMismatch(format!(
                    "required standard header '{HDR_MCP_NAME}' is missing for method 'tools/call'"
                )));
            }
            (Some(raw), expected) => {
                let decoded = decode_header_value(raw).ok_or_else(|| {
                    McpError::HeaderMismatch(format!(
                        "'{HDR_MCP_NAME}' header value '{raw}' is not valid Base64-sentinel or ASCII"
                    ))
                })?;
                if Some(decoded.as_str()) != expected {
                    return Err(McpError::HeaderMismatch(format!(
                        "'{HDR_MCP_NAME}' header value '{decoded}' does not match body params.name"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn handle_discover(request: &mcp::Request) -> JsonErr2 {
    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(DiscoverResult::current()).unwrap_or(json!({})),
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
    _state: Arc<AppState>,
    auth_ctx: AuthContext,
    correlation_id: &str,
) -> JsonErr2 {
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

    if let super::config::SecurityMode::Remote = _state.config.mode {
        let claims = auth_ctx.claims.as_ref().unwrap();
        let scopes = claims.scope.as_deref().unwrap_or("");
        let scope_list: Vec<&str> = scopes.split_whitespace().collect();

        let configured_owner = _state.config.oauth_owner_subject.as_deref();
        if claims.sub.as_deref() != configured_owner {
            return Err(Box::new((
                StatusCode::FORBIDDEN,
                HeaderMap::new(),
                Json(ErrorResponse::new(
                    Some(request.id.clone()),
                    &McpError::InvalidRequest(
                        "Authenticated subject is not the configured owner".into(),
                    ),
                )),
            )));
        }
        if !scope_list.contains(&CODING_SCOPE) {
            return Err(Box::new((
                StatusCode::FORBIDDEN,
                bearer_challenge(
                    &_state.config,
                    Some("insufficient_scope"),
                    Some(CODING_SCOPE),
                ),
                Json(ErrorResponse::new(
                    Some(request.id.clone()),
                    &McpError::InvalidRequest("Insufficient scope: requires 'relay.coding'".into()),
                )),
            )));
        }

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
    let _permit = match std::sync::Arc::clone(&_state.execution_semaphore)
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
        crate::relay_agent::execution::dispatch_tool_call(&tool, &call.arguments, &_state.config)
            .await
            .unwrap_or_else(|e| ToolCallResult {
                content: vec![super::mcp::ToolResultContent {
                    kind: "text",
                    text: format!("execution failed: {}", e.message()),
                }],
                is_error: true,
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
    fn forwarded_https_is_ignored_without_explicit_proxy_trust() {
        let request = Request::builder()
            .uri("https://relay.example/mcp")
            .header("x-forwarded-proto", "https")
            .header("x-forwarded-host", "relay.example")
            .body(axum::body::Body::empty())
            .expect("request should build");
        let config = ServerConfig::default();

        assert!(!request_uses_trusted_https(&request, &config));
    }

    #[test]
    fn explicitly_trusted_loopback_edge_can_assert_https() {
        let request = Request::builder()
            .uri("/mcp")
            .header("x-forwarded-proto", "https")
            .body(axum::body::Body::empty())
            .expect("request should build");
        let config = ServerConfig {
            mode: super::super::config::SecurityMode::Remote,
            trusted_proxy: true,
            ..ServerConfig::default()
        };

        assert!(request_uses_trusted_https(&request, &config));
    }

    #[test]
    fn bearer_challenge_points_to_path_derived_metadata() {
        let config = ServerConfig {
            mode: super::super::config::SecurityMode::Remote,
            oauth_audience: Some("https://relay.example/mcp".into()),
            ..ServerConfig::default()
        };

        let headers = bearer_challenge(&config, Some("insufficient_scope"), Some(CODING_SCOPE));
        let challenge = headers
            .get(axum::http::header::WWW_AUTHENTICATE)
            .and_then(|value| value.to_str().ok())
            .expect("challenge should be present");
        assert!(challenge.contains("error=\"insufficient_scope\""));
        assert!(challenge.contains("scope=\"relay.coding\""));
        assert!(challenge.contains(
            "resource_metadata=\"https://relay.example/.well-known/oauth-protected-resource/mcp\""
        ));
        assert!(!challenge.contains("offline_access"));
    }
}
