//! Tool-call authorization adaptation and dispatch for MCP HTTP.

use super::{err_response, AppState, AuthContext, AuthDecision, JsonErr, CODING_SCOPE};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

use crate::auth::bearer_challenge_value;
use crate::observability::audit;
use relay_core::error::McpError;
use relay_interfaces::mcp::{self, Response, ToolCallResult, ToolsCallParams};

type JsonErr2 = Result<Json<Value>, JsonErr>;

pub(super) async fn handle_tools_call(
    request: &mcp::Request,
    state: Arc<AppState>,
    auth_ctx: AuthContext,
    request_id: &str,
    client_hint: Option<&str>,
) -> JsonErr2 {
    let auth_challenge = match auth_ctx.decision {
        AuthDecision::Authorized => None,
        AuthDecision::Missing => Some(("invalid_token", None)),
        AuthDecision::InsufficientScope => Some(("insufficient_scope", Some(CODING_SCOPE))),
    };
    if let Some((error, scope)) = auth_challenge {
        let challenge = bearer_challenge_value(&state.config, Some(error), scope);
        let result = ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
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

    let call: ToolsCallParams = serde_json::from_value(params_val).map_err(|_| {
        err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams("invalid tools/call parameters".to_string()),
        )
    })?;

    let Some(tool) = mcp::find_tool(&call.name) else {
        return Err(err_response(
            StatusCode::NOT_FOUND,
            Some(request.id.clone()),
            &McpError::InvalidParams("unknown tool".to_string()),
        ));
    };

    if let Err(err) = mcp::validate_tool_arguments(&tool, &call.arguments) {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &err,
        ));
    }

    if let relay_core::config::SecurityMode::Remote = state.config.mode {
        let claims = auth_ctx
            .claims
            .as_ref()
            .expect("authorized remote requests have validated claims");

        let subject = claims.sub.as_deref().unwrap_or("unknown");
        audit(
            request_id,
            client_hint,
            "tools/call",
            Some(&call.name),
            "authorized",
            StatusCode::OK,
            Instant::now(),
            Some(subject),
        );
    }

    let session_id = call
        .arguments
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("relay-{}", request_id));
    let effects = relay_application::hooks::effect_classes(
        call.name.as_str(),
        tool.annotations
            .as_ref()
            .is_some_and(|a| a.destructive_hint),
        tool.annotations.as_ref().is_some_and(|a| a.open_world_hint),
    );
    state
        .hooks
        .start_session(
            &session_id,
            state
                .hooks
                .repository_identity()
                .as_deref()
                .unwrap_or("untrusted"),
        )
        .await;
    let pre = state
        .hooks
        .invoke(
            relay_application::hooks::HookEvent::PreToolUse,
            json!({
                "hook_event": "pre_tool_use",
                "tool_id": call.name.as_str(),
                "effect_classes": effects.clone(),
                "cwd": call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
                "success": true,
            }),
        )
        .await;
    if !matches!(
        pre.decision,
        relay_application::hooks::HookDecision::Continue
    ) {
        let approval_requested = matches!(
            pre.decision,
            relay_application::hooks::HookDecision::RequestApproval
        );
        let text = if approval_requested {
            "Approval required before this tool call"
        } else {
            "Hook blocked this tool call"
        };
        let result = if approval_requested {
            ToolCallResult::complete(vec![relay_interfaces::mcp::ToolResultContent {
                kind: "text",
                text: text.into(),
            }])
            .with_meta(json!({
                "approval_required": true,
                "approval_reason": "hook_request"
            }))
        } else {
            ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
                kind: "text",
                text: text.into(),
            }])
        };
        let response = Response::new(
            request.id.clone(),
            serde_json::to_value(result).unwrap_or(json!({})),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    if call.name == "terminal_job_start" {
        let task_id = relay_application::execution::start_terminal_job(
            &call.arguments,
            &state.config,
            &state.jobs,
        )
        .await
        .map_err(|err| err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err))?;
        let task = state.jobs.get(&task_id).await.ok_or_else(|| {
            err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(request.id.clone()),
                &McpError::Internal("task creation failed".into()),
            )
        })?;
        let response = Response::new(
            request.id.clone(),
            json!({ "resultType": "complete", "content": [{ "type": "text", "text": serde_json::to_string(&task.job_json()).unwrap_or_default() }], "isError": false }),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }
    if call.name == "terminal_job_get" || call.name == "terminal_job_cancel" {
        let id = call
            .arguments
            .get("taskId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                err_response(
                    StatusCode::BAD_REQUEST,
                    Some(request.id.clone()),
                    &McpError::InvalidParams("taskId is required".into()),
                )
            })?;
        let task = if call.name == "terminal_job_cancel" {
            state.jobs.cancel(id).await.map_err(|err| {
                err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err)
            })?
        } else {
            state.jobs.get(id).await.ok_or_else(|| {
                err_response(
                    StatusCode::NOT_FOUND,
                    Some(request.id.clone()),
                    &McpError::InvalidParams("unknown task".into()),
                )
            })?
        };
        let response = Response::new(
            request.id.clone(),
            json!({ "resultType": "complete", "content": [{ "type": "text", "text": serde_json::to_string(&task.job_json()).unwrap_or_default() }], "isError": false }),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    if call.name == "terminal_exec" && client_supports_tasks(request.params.as_ref()) {
        let task_id = relay_application::execution::start_terminal_job(
            &call.arguments,
            &state.config,
            &state.jobs,
        )
        .await
        .map_err(|err| err_response(StatusCode::BAD_REQUEST, Some(request.id.clone()), &err))?;
        let task = state.jobs.get(&task_id).await.ok_or_else(|| {
            err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(request.id.clone()),
                &McpError::Internal("task creation failed".into()),
            )
        })?;
        let response = Response::new(request.id.clone(), task.create_task_json());
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    // Tool exists in the registry and both the request shape and its
    // actual execution is Phase 3 scope. Timeout/kill/output-limit outcomes
    // are classified distinctly inside `dispatch_tool_call` itself (never
    // lumped in with a generic "error"), never logging tool arguments/output.
    let tool_dispatch_started = Instant::now();
    let dispatch_result = relay_application::execution::dispatch_tool_call(
        &tool,
        &call.arguments,
        &state.config,
        &state.jobs,
        &state.lsp,
        &state.hooks,
    )
    .await;
    tracing::info!(
        event = "relay.tool.dispatch",
        outcome = if dispatch_result.is_ok() {
            "ok"
        } else {
            "error"
        },
        tool = call.name.as_str(),
        duration_ms = tool_dispatch_started.elapsed().as_millis() as u64,
    );
    let result = dispatch_result.unwrap_or_else(|err| {
        let text = match &err {
            McpError::InvalidRequest(_) | McpError::InvalidParams(_) => err.to_string(),
            _ => "Tool execution failed".to_string(),
        };
        ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
            kind: "text",
            // Policy/argument rejections are safe and useful to return to the
            // caller; internal/provider/process diagnostics remain redacted.
            text,
        }])
    });
    let lifecycle_event = if result.is_error {
        relay_application::hooks::HookEvent::ToolError
    } else {
        relay_application::hooks::HookEvent::PostToolUse
    };
    let _ = state
        .hooks
        .invoke(
            lifecycle_event,
            json!({
                "hook_event": lifecycle_event.name(),
                "tool_id": call.name.as_str(),
                "effect_classes": effects,
                "cwd": call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
                "success": !result.is_error,
                "reason": "tool_result",
            }),
        )
        .await;

    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(result).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn client_supports_tasks(params: Option<&Value>) -> bool {
    params
        .and_then(|value| value.get("_meta"))
        .and_then(|value| value.get("io.modelcontextprotocol/clientCapabilities"))
        .and_then(|value| value.get("extensions"))
        .and_then(|value| value.get("io.modelcontextprotocol/tasks"))
        .is_some()
}
