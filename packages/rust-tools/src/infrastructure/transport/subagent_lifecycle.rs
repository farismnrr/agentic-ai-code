use super::{err_response, AppState, JsonErr};
use crate::core::error::McpError;
use crate::interfaces::mcp::{self, Response};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;

type JsonErr2 = Result<Json<Value>, JsonErr>;

pub(super) async fn handle_subagent_stop(request: &mcp::Request, state: Arc<AppState>) -> JsonErr2 {
    let params = request
        .params
        .as_ref()
        .ok_or_else(|| error(request, "subagent lifecycle metadata is required"))?;
    let meta = params
        .get("_meta")
        .and_then(Value::as_object)
        .ok_or_else(|| error(request, "subagent lifecycle metadata is required"))?;
    let child = session(
        meta,
        "io.modelcontextprotocol/agentSession",
        request,
        "child",
    )?;
    let parent = session(
        meta,
        "io.modelcontextprotocol/parentAgentSession",
        request,
        "parent",
    )?;
    let status = params
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("failed");
    let allowed = state.hooks.subagent_stop(&parent, &child, status).await;
    let response = Response::new(request.id.clone(), json!({ "allowed": allowed }));
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn session(
    meta: &serde_json::Map<String, Value>,
    key: &str,
    request: &mcp::Request,
    label: &str,
) -> Result<String, JsonErr> {
    meta.get(key)
        .and_then(Value::as_str)
        .map(|value| value.chars().take(128).collect())
        .ok_or_else(|| error(request, &format!("{label} session metadata is required")))
}

fn error(request: &mcp::Request, message: &str) -> JsonErr {
    err_response(
        StatusCode::BAD_REQUEST,
        Some(request.id.clone()),
        &McpError::InvalidParams(message.into()),
    )
}
