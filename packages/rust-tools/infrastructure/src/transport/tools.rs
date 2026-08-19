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
    let Some(tool) = mcp::find_tool_for_profile(&call.name, state.config.tool_profile) else {
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
    let effects = relay_application::hooks::effect_classes(
        call.name.as_str(),
        tool.annotations
            .as_ref()
            .is_some_and(|a| a.destructive_hint),
        tool.annotations.as_ref().is_some_and(|a| a.open_world_hint),
    );
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
                    return bounded_tool_error(
                        &request.id,
                        "approval token is invalid or expired",
                        request_started,
                    )
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
                return bounded_tool_error(
                    &request.id,
                    "approval requires stable agent session metadata",
                    request_started,
                );
            };
            let Some(resume_index) = pre.approval_checkpoint else {
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
        let response = Response::new(
            request.id.clone(),
            serde_json::to_value(result).unwrap_or(json!({})),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    if call.name == "terminal_job_start" {
        let tool_dispatch_started = Instant::now();
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
        let result = mcp::with_timing_meta(
            json!({ "resultType": "complete", "content": [{ "type": "text", "text": serde_json::to_string(&task.job_json()).unwrap_or_default() }], "isError": false }),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let response = Response::new(request.id.clone(), result);
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }
    if call.name == "terminal_job_get" || call.name == "terminal_job_cancel" {
        let tool_dispatch_started = Instant::now();
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
        let result = mcp::with_timing_meta(
            json!({ "resultType": "complete", "content": [{ "type": "text", "text": serde_json::to_string(&task.job_json()).unwrap_or_default() }], "isError": false }),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let response = Response::new(request.id.clone(), result);
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

    if state.config.tool_profile == relay_core::config::ToolProfile::Full
        && client_supports_tasks(request.params.as_ref())
        && relay_application::execution::tool_call_supports_tasks(&tool, &call.arguments)
    {
        let tool_dispatch_started = Instant::now();
        let task_id = relay_application::execution::start_tool_task(
            &tool,
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
        super::task_lifecycle::observe(
            state.clone(),
            task_id,
            call.name.clone(),
            effects.clone(),
            call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
        );
        let result = mcp::with_timing_meta(
            task.create_task_json(),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let response = Response::new(request.id.clone(), result);
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }

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
    let dispatch_ms = tool_dispatch_started.elapsed().as_millis() as u64;
    tracing::info!(
        event = "relay.tool.dispatch",
        outcome = if dispatch_result.is_ok() {
            "ok"
        } else {
            "error"
        },
        tool = call.name.as_str(),
        duration_ms = dispatch_ms,
    );
    let result = dispatch_result.unwrap_or_else(|err| {
        let text = match &err {
            McpError::InvalidRequest(_) | McpError::InvalidParams(_) => err.to_string(),
            _ => "Tool execution failed".to_string(),
        };
        ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
            kind: "text",
            // Safe policy errors are returned; internal diagnostics stay redacted.
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

    let result = result.with_timing(dispatch_ms, request_started.elapsed().as_millis() as u64);
    let response = Response::new(
        request.id.clone(),
        serde_json::to_value(result).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

pub(super) async fn handle_agent_session_start(
    request: &mcp::Request,
    state: Arc<AppState>,
) -> JsonErr2 {
    let agent_session = agent_session_from_params(request.params.as_ref()).ok_or_else(|| {
        err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams("agent session metadata is required".into()),
        )
    })?;
    let identity = state
        .hooks
        .repository_identity()
        .unwrap_or_else(|| "untrusted".into());
    let outcome = state.hooks.start_session(&agent_session, &identity).await;
    let (context, failure) = match outcome {
        relay_application::hooks::SessionStartOutcome::Started { context } => (context, None),
        relay_application::hooks::SessionStartOutcome::AlreadyStarted => (None, None),
        relay_application::hooks::SessionStartOutcome::Blocked => {
            (None, Some("session start was blocked"))
        }
        relay_application::hooks::SessionStartOutcome::SecurityFailure => {
            (None, Some("session start security check failed"))
        }
        relay_application::hooks::SessionStartOutcome::CapacityExhausted => {
            (None, Some("session start capacity is exhausted"))
        }
    };
    if let Some(message) = failure {
        let response = Response::new(
            request.id.clone(),
            serde_json::to_value(ToolCallResult::error(vec![
                relay_interfaces::mcp::ToolResultContent {
                    kind: "text",
                    text: message.into(),
                },
            ]))
            .unwrap_or(json!({})),
        );
        return Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))));
    }
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete",
            "context": context.unwrap_or_else(|| json!({})),
            "bounded": true
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn bounded_tool_error(id: &mcp::Id, message: &str, request_started: Instant) -> JsonErr2 {
    let result = ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
        kind: "text",
        text: message.into(),
    }])
    .with_timing(0, request_started.elapsed().as_millis() as u64);
    let response = Response::new(
        id.clone(),
        serde_json::to_value(result).unwrap_or(json!({})),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

pub(super) async fn handle_agent_pre_stop(
    request: &mcp::Request,
    state: Arc<AppState>,
) -> JsonErr2 {
    let agent_session = agent_session_from_params(request.params.as_ref()).ok_or_else(|| {
        err_response(
            StatusCode::BAD_REQUEST,
            Some(request.id.clone()),
            &McpError::InvalidParams("agent session metadata is required".into()),
        )
    })?;
    let allowed = state.hooks.pre_agent_stop(&agent_session).await;
    let response = Response::new(
        request.id.clone(),
        json!({
            "resultType": "complete",
            "completion": if allowed { "allowed" } else { "remediation_required" },
            "max_attempts": 2
        }),
    );
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

fn agent_session_from_params(params: Option<&Value>) -> Option<String> {
    params?
        .get("_meta")?
        .get("io.modelcontextprotocol/agentSession")?
        .as_str()
        .map(|value| value.chars().take(128).collect())
}

fn client_supports_tasks(params: Option<&Value>) -> bool {
    params
        .and_then(|value| value.get("_meta"))
        .and_then(|value| value.get("io.modelcontextprotocol/clientCapabilities"))
        .and_then(|value| value.get("extensions"))
        .and_then(|value| value.get("io.modelcontextprotocol/tasks"))
        .is_some()
}
