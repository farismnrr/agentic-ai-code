//! Tool-call authorization adaptation and dispatch for MCP HTTP.

use super::{err_response, AppState, AuthContext, AuthDecision, JsonErr, CODING_SCOPE};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

use crate::auth::bearer_challenge_value;
use crate::observability::audit;
use relay_application::activity::{self, Evidence, Status};
use relay_core::error::McpError;
use relay_interfaces::mcp::{self, Response, ToolCallResult, ToolsCallParams};

#[path = "task_calls.rs"]
mod task_calls;
#[path = "task_completion.rs"]
mod task_completion;
#[path = "tool_dispatch.rs"]
mod tool_dispatch;
#[path = "tool_helpers.rs"]
mod tool_helpers;
use tool_helpers::deny_activity;
pub(super) use tool_helpers::{activity_result_detail, activity_result_summary};
use tool_helpers::{
    agent_session_from_params, bounded_tool_error, client_supports_tasks, record_activity_outcome,
    requires_idempotency_key,
};

pub(super) type JsonErr2 = Result<Json<Value>, JsonErr>;

pub(super) async fn handle_agent_session_start(
    request: &mcp::Request,
    state: Arc<AppState>,
) -> JsonErr2 {
    tool_helpers::handle_agent_session_start(request, state).await
}

pub(super) async fn handle_agent_pre_stop(
    request: &mcp::Request,
    state: Arc<AppState>,
) -> JsonErr2 {
    tool_helpers::handle_agent_pre_stop(request, state).await
}
pub(super) async fn handle_tools_call(
    request: &mcp::Request,
    state: Arc<AppState>,
    auth_ctx: AuthContext,
    request_id: &str,
    client_hint: Option<&str>,
) -> JsonErr2 {
    let request_started = Instant::now();
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
        let result = result.with_timing(0, request_started.elapsed().as_millis() as u64);
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
    let Some(tool) = state.tool_for_name(&call.name) else {
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

    let agent_session = agent_session_from_params(request.params.as_ref());
    let effects = relay_application::hooks::effect_classes_for_call(
        call.name.as_str(),
        tool.annotations
            .as_ref()
            .is_some_and(|a| a.destructive_hint),
        tool.annotations.as_ref().is_some_and(|a| a.open_world_hint),
        &call.arguments,
    );
    let client_info = request
        .params
        .as_ref()
        .and_then(|params| params.get("_meta"))
        .and_then(|meta| meta.get("io.modelcontextprotocol/clientInfo"))
        .and_then(|info| Some((info.get("name")?.as_str()?, info.get("version")?.as_str()?)));
    let execution_mode = call
        .arguments
        .get("execution_mode")
        .and_then(Value::as_str)
        .unwrap_or("auto");
    let client_has_tasks = client_supports_tasks(request.params.as_ref());
    let tool_has_tasks =
        relay_application::execution::tool_call_supports_tasks(&tool, &call.arguments);
    let idempotency_key = call
        .arguments
        .get("idempotency_key")
        .and_then(Value::as_str);
    let idempotency_scope = auth_ctx
        .claims
        .as_ref()
        .and_then(|claims| claims.sub.as_deref())
        .unwrap_or("local");
    let idempotency_key = idempotency_key
        .filter(|_| {
            execution_mode == "async"
                || (execution_mode == "auto" && client_has_tasks && tool_has_tasks)
        })
        .map(|key| format!("{idempotency_scope}:{}:{key}", call.name));
    let request_fingerprint = serde_json::to_string(&call.arguments).unwrap_or_default();
    if let Some(key) = idempotency_key.as_deref() {
        match state
            .jobs
            .existing_idempotency_key(key, &request_fingerprint)
            .await
        {
            Ok(Some(task_id)) => {
                let Some(task) = state.jobs.get(&task_id).await else {
                    return bounded_tool_error(
                        &request.id,
                        "accepted task is no longer available",
                        request_started,
                    );
                };
                let result = mcp::with_timing_meta(
                    task.create_task_json(),
                    0,
                    request_started.elapsed().as_millis() as u64,
                );
                let response = Response::new(request.id.clone(), result);
                return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
            }
            Ok(None) => {}
            Err(err) => return bounded_tool_error(&request.id, &err.to_string(), request_started),
        }
    }
    let activity_start = activity::event_for_tool(
        &state.config,
        &call.name,
        &effects,
        &call.arguments,
        client_info,
    );
    let activity_start = match state.activity.record_start(activity_start, None) {
        Ok(event) => event,
        Err(_) => {
            return bounded_tool_error(
                &request.id,
                "activity history is unavailable; tool execution was not started",
                request_started,
            )
        }
    };
    let execute_async = match execution_mode {
        "sync" => false,
        "async" => {
            if !client_has_tasks {
                record_activity_outcome(
                    &state,
                    &activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "async execution requires MCP Tasks capability",
                    Evidence::NotApplicable,
                    None,
                );
                return bounded_tool_error(
                    &request.id,
                    "async execution requires MCP Tasks capability",
                    request_started,
                );
            }
            if !tool_has_tasks {
                record_activity_outcome(
                    &state,
                    &activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "async execution is not supported for this tool or request",
                    Evidence::NotApplicable,
                    None,
                );
                return bounded_tool_error(
                    &request.id,
                    "async execution is not supported for this tool or request",
                    request_started,
                );
            }
            if requires_idempotency_key(&call.name, &call.arguments) && idempotency_key.is_none() {
                record_activity_outcome(
                    &state,
                    &activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "async mutation requires an idempotency key",
                    Evidence::NotApplicable,
                    None,
                );
                return bounded_tool_error(
                    &request.id,
                    "async mutation requires an idempotency key",
                    request_started,
                );
            }
            true
        }
        _ => {
            client_has_tasks
                && tool_has_tasks
                && (!requires_idempotency_key(&call.name, &call.arguments)
                    || idempotency_key.is_some())
        }
    };
    let hook_payload = json!({
        "hook_event": "pre_tool_use",
        "tool_id": call.name.as_str(),
        "effect_classes": effects.clone(),
        "cwd": call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
        "arguments": call.arguments.clone(),
        "success": true,
    });
    let hook_approval_token = request
        .params
        .as_ref()
        .and_then(|params| params.get("_meta"))
        .and_then(|meta| meta.get("io.modelcontextprotocol/hookApprovalToken"))
        .and_then(Value::as_str);
    let approval_resume = match hook_approval_token {
        Some(token) => {
            let Some(agent_session) = agent_session.as_deref() else {
                deny_activity(
                    &state,
                    &activity_start,
                    request_started,
                    "approval requires stable agent session metadata",
                );
                return bounded_tool_error(
                    &request.id,
                    "approval requires stable agent session metadata",
                    request_started,
                );
            };
            match state
                .hooks
                .consume_approval(token, agent_session, &call.name, &hook_payload)
                .await
            {
                Some(index) => Some(index),
                None => {
                    deny_activity(
                        &state,
                        &activity_start,
                        request_started,
                        "approval token is invalid or expired",
                    );
                    return bounded_tool_error(
                        &request.id,
                        "approval token is invalid or expired",
                        request_started,
                    );
                }
            }
        }
        None => None,
    };
    if let Some(agent_session) = agent_session.as_deref() {
        let session_outcome = state
            .hooks
            .start_session(
                agent_session,
                state
                    .hooks
                    .repository_identity()
                    .as_deref()
                    .unwrap_or("untrusted"),
            )
            .await;
        if matches!(
            session_outcome,
            relay_application::hooks::SessionStartOutcome::Blocked
                | relay_application::hooks::SessionStartOutcome::SecurityFailure
                | relay_application::hooks::SessionStartOutcome::CapacityExhausted
        ) {
            deny_activity(
                &state,
                &activity_start,
                request_started,
                "agent session security lifecycle did not start",
            );
            return bounded_tool_error(
                &request.id,
                "agent session security lifecycle did not start",
                request_started,
            );
        }
    }
    let pre = if let Some(resume_index) = approval_resume {
        state
            .hooks
            .invoke_from(
                relay_application::hooks::HookEvent::PreToolUse,
                hook_payload.clone(),
                resume_index,
            )
            .await
    } else {
        state
            .hooks
            .invoke(
                relay_application::hooks::HookEvent::PreToolUse,
                hook_payload.clone(),
            )
            .await
    };
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
            let Some(agent_session) = agent_session.as_deref() else {
                deny_activity(
                    &state,
                    &activity_start,
                    request_started,
                    "approval requires stable agent session metadata",
                );
                return bounded_tool_error(
                    &request.id,
                    "approval requires stable agent session metadata",
                    request_started,
                );
            };
            let Some(resume_index) = pre.approval_checkpoint else {
                deny_activity(
                    &state,
                    &activity_start,
                    request_started,
                    "approval checkpoint is unavailable",
                );
                return bounded_tool_error(
                    &request.id,
                    "approval checkpoint is unavailable",
                    request_started,
                );
            };
            let Some(approval_token) = state
                .hooks
                .issue_approval(agent_session, &call.name, &hook_payload, resume_index)
                .await
            else {
                deny_activity(
                    &state,
                    &activity_start,
                    request_started,
                    "approval capacity is exhausted",
                );
                return bounded_tool_error(
                    &request.id,
                    "approval capacity is exhausted",
                    request_started,
                );
            };
            ToolCallResult::complete(vec![relay_interfaces::mcp::ToolResultContent {
                kind: "text",
                text: text.into(),
            }])
            .with_meta(json!({
                "control": { "type": "approval_required", "reason": "hook_request", "token": approval_token }
            }))
        } else {
            ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
                kind: "text",
                text: text.into(),
            }])
        };
        let result = result.with_timing(0, request_started.elapsed().as_millis() as u64);
        record_activity_outcome(
            &state,
            &activity_start,
            Status::Denied,
            request_started.elapsed().as_millis() as u64,
            "tool call denied by relay policy",
            Evidence::NotApplicable,
            None,
        );
        let response = Response::new(
            request.id.clone(),
            serde_json::to_value(result).unwrap_or(json!({})),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    if let Some(response) = task_calls::try_handle_task_call(task_calls::ToolCallContext {
        request,
        state: state.clone(),
        call: &call,
        tool: &tool,
        activity_start: &activity_start,
        effects: &effects,
        execute_async,
        idempotency_key: idempotency_key.as_deref(),
        request_fingerprint,
        request_started,
    })
    .await
    {
        return response;
    }
    if call.name == "task_completed" {
        return task_completion::handle(
            request,
            state,
            &call.arguments,
            effects,
            &activity_start,
            request_started,
        )
        .await;
    }
    tool_dispatch::handle(
        request,
        state,
        &tool,
        &call.arguments,
        effects,
        &activity_start,
        request_started,
    )
    .await
}
