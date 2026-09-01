//! MCP HTTP request parsing, protocol dispatch, tools, and task lifecycle handlers.

use super::{err_response, json_error_response, AppState, AuthContext, JsonErr, TOOLS_LIST_TTL_MS};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::core::error::McpError;
use crate::infrastructure::observability::{audit, CorrelationId, RequestId};
use crate::infrastructure::telemetry::extract_traceparent;
use crate::interfaces::mcp::{self, parse_request, DiscoverResult, Id, Response};

pub(super) async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    axum::extract::Extension(auth_ctx): axum::extract::Extension<AuthContext>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    axum::extract::Extension(client_hint): axum::extract::Extension<CorrelationId>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AxumResponse {
    // W3C Trace Context extraction (Plan 035 Phase 8/9): joins this span to
    // the caller's distributed trace when a trusted first-party caller sends
    // `traceparent`. No-op (root span) when absent.
    let parent_cx = extract_traceparent(&headers);
    let span = tracing::info_span!("relay.request", request_id = %request_id.as_str());
    let _ = span.set_parent(parent_cx);

    async move {
        let started = Instant::now();
        let request_id_str = request_id.as_str();
        let client_hint_str = client_hint.as_str();
        // Content-Type must be application/json for the Streamable HTTP JSON
        // mode used in this phase (no SSE upgrade implemented yet — see audit
        // doc section 3).
        let content_type = headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if let Err(err) =
            crate::interfaces::transport_validation::validate_content_type(Some(content_type))
        {
            let response = err_response(StatusCode::BAD_REQUEST, None, &err).into_response();
            audit(
                request_id_str,
                client_hint_str,
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
                    request_id_str,
                    client_hint_str,
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
            tracing::info!(event = "relay.mcp.initialize");
            return handle_initialize(&request, &state)
                .map_or_else(json_error_response, |body| body.into_response());
        }

        // Legacy clients do not send the 2026 request metadata or routing
        // headers on the follow-up tools/list request. Keep this compatibility
        // exception narrow; a tools/list request that presents any modern header
        // or metadata remains subject to the strict 2026 validation below.
        let legacy_tools_list =
            crate::interfaces::transport_validation::is_legacy_tools_list(&headers, &request);
        if legacy_tools_list {
            tracing::info!(event = "relay.mcp.tools_list", outcome = "legacy");
            return handle_tools_list(&request, &state)
                .map_or_else(json_error_response, |body| body.into_response());
        }

        if let Err(err) =
            crate::interfaces::transport_validation::validate_routing_headers(&headers, &request)
        {
            tracing::warn!(event = "relay.mcp.header_validation", outcome = "rejected");
            return json_error_response(err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &err,
            ));
        }

        match crate::application::dispatcher::dispatch(&request) {
            crate::application::dispatcher::Dispatch::Discover => {
                tracing::info!(event = "relay.mcp.discover");
                handle_discover(&request)
            }
            crate::application::dispatcher::Dispatch::ToolsList => {
                tracing::info!(event = "relay.mcp.tools_list");
                handle_tools_list(&request, &state)
            }
            crate::application::dispatcher::Dispatch::ToolsCall => {
                tracing::info!(event = "relay.mcp.tools_call");
                super::tools::handle_tools_call(
                    &request,
                    state,
                    auth_ctx,
                    request_id_str,
                    client_hint_str,
                )
                .await
            }
            crate::application::dispatcher::Dispatch::ResourcesList => {
                handle_resources_list(&request, &state)
            }
            crate::application::dispatcher::Dispatch::ResourcesRead => {
                handle_resources_read(&request, &state)
            }
            crate::application::dispatcher::Dispatch::TasksGet => {
                handle_task_get(&request, state).await
            }
            crate::application::dispatcher::Dispatch::TasksUpdate => {
                handle_task_update(&request, state).await
            }
            crate::application::dispatcher::Dispatch::TasksCancel => {
                handle_task_cancel(&request, state).await
            }
            crate::application::dispatcher::Dispatch::AgentSessionStart => {
                super::tools::handle_agent_session_start(&request, state).await
            }
            crate::application::dispatcher::Dispatch::AgentPreStop => {
                super::tools::handle_agent_pre_stop(&request, state).await
            }
            crate::application::dispatcher::Dispatch::AgentSubagentStop => {
                super::subagent_lifecycle::handle_subagent_stop(&request, state).await
            }
            crate::application::dispatcher::Dispatch::ActivityConfigure => {
                handle_activity_configure(&request, &state)
            }
            crate::application::dispatcher::Dispatch::ActivityStatus => {
                handle_activity_status(&request, &state)
            }
            crate::application::dispatcher::Dispatch::Unknown(other) => Err(err_response(
                StatusCode::NOT_FOUND,
                Some(request.id.clone()),
                &McpError::MethodNotFound(other),
            )),
        }
        .map_or_else(json_error_response, |body| body.into_response())
    }
    .instrument(span)
    .await
}

fn handle_discover(request: &mcp::Request) -> JsonErr2 {
    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(DiscoverResult::current()).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_activity_status(request: &mcp::Request, state: &Arc<AppState>) -> JsonErr2 {
    let (configured, source_id) = state.activity_control.status().map_err(|_| {
        err_response(
            StatusCode::SERVICE_UNAVAILABLE,
            Some(request.id.clone()),
            &McpError::InvalidRequest("activity status is unavailable".into()),
        )
    })?;
    let response = Response::new(
        request.id.clone(),
        json!({ "configured": configured, "sourceId": source_id }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_activity_configure(request: &mcp::Request, state: &Arc<AppState>) -> JsonErr2 {
    let params = request
        .params
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidRequest("server/activity_configure params are required".into()),
            )
        })?;
    let sink_url = params
        .get("sinkUrl")
        .and_then(Value::as_str)
        .filter(|value| value.len() <= 4096)
        .ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidRequest("server/activity_configure sinkUrl is invalid".into()),
            )
        })?;
    let source_token = params
        .get("sourceToken")
        .and_then(Value::as_str)
        .filter(|value| {
            value.len() >= 32 && value.len() <= 512 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidRequest(
                    "server/activity_configure sourceToken is invalid".into(),
                ),
            )
        })?;
    state
        .activity_control
        .configure(sink_url.to_owned(), source_token.to_owned())
        .map_err(|_| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidRequest("activity bootstrap could not be applied".into()),
            )
        })?;
    let response = Response::new(request.id.clone(), json!({ "configured": true }));
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_initialize(request: &mcp::Request, _state: &Arc<AppState>) -> JsonErr2 {
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
            "capabilities": json!({
                "tools": { "listChanged": false },
                "resources": {},
                "extensions": {
                    "io.modelcontextprotocol/tasks": {},
                    "io.masihawam/activity-bootstrap": { "version": "1" }
                }
            }),
            "serverInfo": { "name": "relay-agent", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Coding server providing a sandboxed coding terminal, configured HTTP requests, and web search within the configured workspace policy."
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_tools_list(request: &mcp::Request, state: &Arc<AppState>) -> JsonErr2 {
    let tools = mcp::tool_catalog_for_profile(state.config.tool_profile);
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete",
            "ttlMs": 0,
            "cacheScope": "private",
            "tools": tools
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_resources_list(request: &mcp::Request, state: &Arc<AppState>) -> JsonErr2 {
    let resources = crate::application::resources::list(&state.config)
        .map_err(|err| err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err))?;
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete", "ttlMs": TOOLS_LIST_TTL_MS, "cacheScope": "private", "resources": resources
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn handle_resources_read(request: &mcp::Request, state: &Arc<AppState>) -> JsonErr2 {
    let uri = request
        .params
        .as_ref()
        .and_then(|v| v.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidParams("uri is required".into()),
            )
        })?;
    let content = crate::application::resources::read(&state.config, uri)
        .map_err(|err| err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err))?;
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete", "ttlMs": 0, "cacheScope": "private", "contents": [content]
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

/// The three dispatch handlers above return `Json<Value>` on success or a
/// pre-built `JsonErr` on failure, and are converted to a full
/// [`AxumResponse`] by `.into_response()` in [`handle_mcp`] — this alias
/// just keeps their signatures short.
type JsonErr2 = Result<Json<Value>, JsonErr>;

fn task_id(request: &mcp::Request) -> Result<&str, JsonErr> {
    request
        .params
        .as_ref()
        .and_then(|value| value.get("taskId"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            err_response(
                StatusCode::BAD_REQUEST,
                Some(request.id.clone()),
                &McpError::InvalidParams("taskId is required".into()),
            )
        })
}

async fn handle_task_get(request: &mcp::Request, state: Arc<AppState>) -> JsonErr2 {
    let id = task_id(request)?;
    let task = state.jobs.get(id).await.ok_or_else(|| {
        err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::InvalidParams("unknown task".into()),
        )
    })?;
    Ok(Json(
        serde_json::to_value(Response::new(
            request.id.clone(),
            task.task_json(state.config.completed_job_ttl_ms),
        ))
        .unwrap_or(json!({})),
    ))
}

async fn handle_task_update(request: &mcp::Request, state: Arc<AppState>) -> JsonErr2 {
    let id = task_id(request)?;
    let has_input_responses = request
        .params
        .as_ref()
        .and_then(|value| value.get("inputResponses"))
        .and_then(Value::as_object)
        .is_some();
    if !has_input_responses {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams("inputResponses is required".into()),
        ));
    }
    if state.jobs.get(id).await.is_none() {
        return Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::InvalidParams("unknown task".into()),
        ));
    }
    Ok(Json(
        serde_json::to_value(Response::new(
            request.id.clone(),
            json!({ "resultType": "complete" }),
        ))
        .unwrap_or(json!({})),
    ))
}

async fn handle_task_cancel(request: &mcp::Request, state: Arc<AppState>) -> JsonErr2 {
    let id = task_id(request)?;
    state
        .jobs
        .cancel(id)
        .await
        .map_err(|err| err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err))?;
    Ok(Json(
        serde_json::to_value(Response::new(
            request.id.clone(),
            json!({ "resultType": "complete" }),
        ))
        .unwrap_or(json!({})),
    ))
}
