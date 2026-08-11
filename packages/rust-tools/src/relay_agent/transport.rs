//! Streamable HTTP transport for the MCP `2026-07-28` server core.
//!
//! Single JSON-RPC route (`POST /mcp`) plus a plain `/health` probe used by
//! local tooling. Localhost-only binding is enforced by the caller
//! (`src/bin/relay-agent.rs` binds `127.0.0.1` explicitly) — this module
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

pub struct AppState {
    pub config: ServerConfig,
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

    let state = Arc::new(AppState { config });

    // The security middleware is attached to the `/mcp` route only, via
    // `route_layer` on its own sub-router, so `/health` never goes through
    // Origin/Host enforcement.
    let mcp_router =
        Router::new()
            .route("/mcp", post(handle_mcp))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                local_access_policy,
            ));

    Router::new()
        .merge(mcp_router)
        .route("/health", get(health_check))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

type JsonErr = (StatusCode, Json<ErrorResponse>);

fn err_response(status: StatusCode, id: Option<Id>, err: &McpError) -> JsonErr {
    (status, Json(ErrorResponse::new(id, err)))
}

/// Server-side local-access policy: exact `Origin` + `Host` validation.
/// Runs before any MCP/JSON-RPC parsing — a rejected request never reaches
/// [`handle_mcp`]. See `security.rs` for the fail-closed rules and why this
/// is distinct from (and not replaceable by) the `CorsLayer` below.
async fn local_access_policy(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> AxumResponse {
    if let Err(err) = enforce_local_access_policy(req.headers(), &state.config) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new(None, &err))).into_response();
    }
    next.run(req).await
}

async fn handle_mcp(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AxumResponse {
    // Content-Type must be application/json for the Streamable HTTP JSON
    // mode used in this phase (no SSE upgrade implemented yet — see audit
    // doc section 3).
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !content_type.starts_with("application/json") {
        return err_response(
            StatusCode::BAD_REQUEST,
            None,
            &McpError::InvalidRequest(format!(
                "unsupported Content-Type '{content_type}', expected application/json"
            )),
        )
        .into_response();
    }

    // Body size is already bounded by DefaultBodyLimit before this handler
    // runs; parse only after that gate, never before.
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return err_response(StatusCode::BAD_REQUEST, None, &McpError::ParseError)
                .into_response();
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
            return err_response(StatusCode::BAD_REQUEST, id, &mcp_err).into_response();
        }
    };

    if let Err(err) = validate_routing_headers(&headers, &request) {
        return err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err)
            .into_response();
    }

    match request.method.as_str() {
        "server/discover" => handle_discover(&request),
        "tools/list" => handle_tools_list(&request),
        "tools/call" => handle_tools_call(&request),
        other => Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::MethodNotFound(other.to_string()),
        )),
    }
    .into_response()
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
    let response = Response::new(request.id.clone(), json!({ "tools": tools }));
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_tools_call(request: &mcp::Request) -> JsonErr2 {
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

    let Some(_tool) = mcp::find_tool(&call.name) else {
        return Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::InvalidParams(format!("unknown tool '{}'", call.name)),
        ));
    };

    // Tool exists in the registry and the request shape is valid; actual
    // execution is Phase 3 scope. Per the audit doc, this is a *result*
    // with isError:true, not a JSON-RPC protocol error, matching MCP
    // tool-result conventions for a failing (here: unimplemented) call.
    let result = ToolCallResult::not_implemented(&call.name);
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
