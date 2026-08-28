//! Streamable HTTP transport for the MCP `2026-07-28` server core.
//!
//! Single JSON-RPC route (`POST /mcp`) plus a plain `/health` probe used by
//! local tooling. Loopback binding is enforced by the caller/configuration
//! (`src/bin/relay-agent.rs` binds the validated loopback address) — this module
//! only builds the `Router`, it does not bind sockets.
//!
//! `/mcp` is additionally gated by [`crate::security::enforce_local_access_policy`]
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
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::activity::ReloadableActivityRecorder;
use crate::auth;
use crate::auth::{CachedJwks, Claims};
use crate::notifications::TaskNotificationService;
use crate::observability::{CorrelationId, RequestId};
use relay_application::activity::SharedActivityRecorder;
use relay_application::admission::RequestAdmission;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{ErrorResponse, Id};

mod access;
mod mcp_http;
mod subagent_lifecycle;
mod task_lifecycle;
mod tools;

/// Frozen in `.agents/plans/028-phase0-contract-audit.md` section 6: MCP
/// HTTP request body max.
pub const MAX_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

const HDR_PROTOCOL_VERSION: &str = "mcp-protocol-version";
pub(super) const HDR_MCP_METHOD: &str = "mcp-method";
const HDR_MCP_NAME: &str = "mcp-name";
pub(super) const TOOLS_LIST_TTL_MS: u64 = 300_000;
/// Resource permission exposed to the external Authorization Server and MCP
/// client for the default full-coding deployment profile.
pub(super) use relay_interfaces::mcp::CODING_SCOPE;

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
    pub jobs: std::sync::Arc<relay_application::execution::JobManager>,
    /// Bounded, per-workspace LSP session manager backing the public
    /// `code_*` code-intelligence tools (Plan 039C). Constructed once per
    /// router the same way `jobs` is, so all `code_*` calls in a process
    /// share the same capped session pool.
    pub lsp: std::sync::Arc<relay_application::lsp::LspSessionManager>,
    /// Inert unless explicitly enabled and identity-validated at startup.
    pub hooks: std::sync::Arc<relay_application::hooks::HookManager>,
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
    /// Durable relay-boundary activity recorder. Required mode is opened
    /// before the router is returned, so a failed journal cannot admit calls.
    pub activity: SharedActivityRecorder,
    /// Private control plane for reloading the recorder after an authenticated
    /// first-party activity bootstrap. This is never exposed through tools/list.
    pub activity_control: Arc<ReloadableActivityRecorder>,
    /// Relay-owned task completion queue. Telegram is intentionally not a
    /// generic MCP capability and never accepts a caller-supplied destination.
    pub notifications: Arc<TaskNotificationService>,
}

impl AppState {
    pub fn tool_for_name(&self, name: &str) -> Option<relay_interfaces::mcp::Tool> {
        relay_interfaces::mcp::tool_catalog_for_profile(self.config.tool_profile)
            .into_iter()
            .find(|tool| tool.name == name)
    }
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
    let jobs = relay_application::execution::JobManager::new(config.clone());
    create_router_with_jobs(config, jobs)
}

pub fn create_router_with_jobs(
    config: ServerConfig,
    jobs: Arc<relay_application::execution::JobManager>,
) -> Router {
    let hooks = relay_application::hooks::HookManager::load(Arc::new(config.clone()))
        .expect("enabled agent hook configuration must be valid before router construction");
    create_router_with_jobs_and_hooks(config, jobs, hooks)
}

pub fn create_router_with_jobs_and_hooks(
    config: ServerConfig,
    jobs: Arc<relay_application::execution::JobManager>,
    hooks: Arc<relay_application::hooks::HookManager>,
) -> Router {
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

    // A misconfigured `--lsp-server` mapping (e.g. an executable missing
    // from the safe PATH) must not prevent the relay from starting — it
    // should only mean the `code_*` tools report that language as
    // unavailable. Fall back to an LSP manager with no configured servers
    // rather than failing router construction.
    let lsp = relay_application::lsp::LspSessionManager::new(config.clone()).unwrap_or_else(|_| {
        let mut unconfigured = config.clone();
        unconfigured.lsp_servers = Vec::new();
        relay_application::lsp::LspSessionManager::new(unconfigured)
            .expect("LSP session manager with no configured servers must construct")
    });
    let activity_control = crate::activity::ReloadableActivityRecorder::open(&config)
        .expect("required activity journal must be available before relay startup");
    let activity: SharedActivityRecorder = activity_control.clone();
    let notifications = TaskNotificationService::open(&config)
        .expect("enabled task notification ledger must be available before relay startup");
    notifications.spawn_worker();
    let state = Arc::new(AppState {
        config: config.clone(),
        jobs,
        lsp,
        hooks,
        request_admission: Arc::new(RequestAdmission::configured()),
        jwks_cache: tokio::sync::RwLock::new(None),
        activity,
        activity_control,
        notifications,
    });

    let mcp_router = Router::new().route("/mcp", post(mcp_http::handle_mcp));
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            access::access_policy,
        ))
        .layer(middleware::from_fn(correlation_middleware))
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(state)
}

async fn handle_health() -> StatusCode {
    StatusCode::OK
}

/// Establishes the per-request identity. `RequestId` is always
/// server-generated and is the sole authoritative correlation value returned
/// to the client (Plan 035 Phase 9); a client-supplied `x-correlation-id`, if
/// present and well-formed, is retained only as untrusted optional metadata
/// for private telemetry and is never echoed back on the response.
async fn correlation_middleware(mut req: Request, next: Next) -> AxumResponse {
    let client_hint = CorrelationId::from_request(&req);
    let request_id = RequestId::generate();
    req.extensions_mut().insert(client_hint);
    req.extensions_mut().insert(request_id.clone());
    let mut response = next.run(req).await;
    request_id.insert_response_header(&mut response);
    response
}

pub(super) type JsonErr = Box<(StatusCode, HeaderMap, Json<ErrorResponse>)>;

pub(super) fn err_response(status: StatusCode, id: Option<Id>, err: &McpError) -> JsonErr {
    Box::new((status, HeaderMap::new(), Json(ErrorResponse::new(id, err))))
}

pub(super) fn json_error_response(error: JsonErr) -> AxumResponse {
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
