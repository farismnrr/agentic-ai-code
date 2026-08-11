//! Streamable HTTP transport for the MCP `2026-07-28` server core.
//!
//! Single JSON-RPC route (`POST /mcp`) plus a plain `/health` probe used by
//! local tooling. Localhost-only binding is enforced by the caller
//! (`src/bin/relay-agent.rs` binds `127.0.0.1` explicitly) — this module
//! only builds the `Router`, it does not bind sockets.

use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::config::ServerConfig;
use super::error::McpError;
use super::mcp::{
    self, parse_request, tool_catalog, ErrorResponse, Id, InitializeParams, Response,
    ToolCallResult, ToolsCallParams,
};

/// Frozen in `.agents/plans/028-phase0-contract-audit.md` section 6: MCP
/// HTTP request body max.
pub const MAX_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

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

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(tower_http::cors::Any);

    let state = Arc::new(AppState { config });

    Router::new()
        .route("/mcp", post(handle_mcp))
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

async fn handle_mcp(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, JsonErr> {
    // MCP-Protocol-Version is required on every request per the frozen audit
    // (section 3): missing or mismatched fails closed, it is not merely
    // validated-if-present.
    let proto_header = headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok());
    match proto_header {
        Some(v) if v == mcp::PROTOCOL_VERSION => {}
        Some(other) => {
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                None,
                &McpError::InvalidRequest(format!(
                    "unsupported MCP-Protocol-Version '{other}', expected '{}'",
                    mcp::PROTOCOL_VERSION
                )),
            ));
        }
        None => {
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                None,
                &McpError::InvalidRequest(format!(
                    "missing required header MCP-Protocol-Version: expected '{}'",
                    mcp::PROTOCOL_VERSION
                )),
            ));
        }
    }

    // Content-Type must be application/json for the Streamable HTTP JSON
    // mode used in this phase (no SSE upgrade implemented yet — see audit
    // doc section 3).
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !content_type.starts_with("application/json") {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            None,
            &McpError::InvalidRequest(format!(
                "unsupported Content-Type '{content_type}', expected application/json"
            )),
        ));
    }

    // Body size is already bounded by DefaultBodyLimit before this handler
    // runs; parse only after that gate, never before.
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| err_response(StatusCode::BAD_REQUEST, None, &McpError::ParseError))?;

    let request = match parse_request(&payload) {
        Ok(req) => req,
        Err(None) => {
            // A notification: no response body is meaningful; return an
            // empty JSON object with 200 rather than a JSON-RPC envelope.
            return Ok(Json(json!({})));
        }
        Err(Some(mcp_err)) => {
            let id = payload
                .get("id")
                .and_then(|v| serde_json::from_value::<Id>(v.clone()).ok());
            return Err(err_response(StatusCode::BAD_REQUEST, id, &mcp_err));
        }
    };

    match request.method.as_str() {
        "server/discover" | "initialize" => handle_initialize(&request),
        "tools/list" => handle_tools_list(&request),
        "tools/call" => handle_tools_call(&request),
        other => Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::MethodNotFound(other.to_string()),
        )),
    }
}

fn handle_initialize(request: &mcp::Request) -> Result<Json<Value>, JsonErr> {
    if request.method == "initialize" {
        let params_val = request.params.clone().ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidParams("missing initialize parameters".to_string()),
            )
        })?;

        let init_params: InitializeParams = serde_json::from_value(params_val).map_err(|e| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidParams(format!("invalid initialize parameters: {e}")),
            )
        })?;

        if init_params.protocol_version != mcp::PROTOCOL_VERSION {
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidParams(format!(
                    "unsupported protocol version '{}', requires '{}'",
                    init_params.protocol_version,
                    mcp::PROTOCOL_VERSION
                )),
            ));
        }
    }

    // Stateless core: this is a capability-announcement convenience, not a
    // session handshake. No Mcp-Session-Id is issued or required, and
    // tools/list + tools/call work identically without ever calling this.
    let response = Response::new(
        request.id.clone(),
        json!({
            "protocolVersion": mcp::PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "relay-agent", "version": env!("CARGO_PKG_VERSION") }
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_tools_list(request: &mcp::Request) -> Result<Json<Value>, JsonErr> {
    let tools = tool_catalog();
    let response = Response::new(request.id.clone(), json!({ "tools": tools }));
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_tools_call(request: &mcp::Request) -> Result<Json<Value>, JsonErr> {
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
