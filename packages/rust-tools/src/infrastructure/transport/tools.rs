use super::{err_response, AppState, AuthContext, AuthDecision, JsonErr, CODING_SCOPE};
use crate::application::activity::{self, Evidence, Status};
use crate::core::error::McpError;
use crate::infrastructure::auth::bearer_challenge_value;
use crate::infrastructure::observability::audit;
use crate::interfaces::mcp::{self, Response, ToolCallResult, ToolsCallParams};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
#[path = "telegram_message.rs"]
mod telegram_message;
#[path = "tool_dispatch.rs"]
mod tool_dispatch;
#[path = "tool_helpers.rs"]
pub(super) mod tool_helpers;
use tool_helpers::{
    agent_session_from_params, bounded_tool_error, deny_activity, record_activity_outcome,
};
pub(super) type JsonErr2 = Result<Json<Value>, JsonErr>;
pub(crate) use tool_helpers::{handle_agent_pre_stop, handle_agent_session_start};
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
        let result = ToolCallResult::error(vec![crate::interfaces::mcp::ToolResultContent {
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
    if let crate::core::config::SecurityMode::Remote = state.config.mode {
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
    let task_owner = auth_ctx
        .claims
        .as_ref()
        .and_then(|claims| claims.sub.as_deref())
        .unwrap_or("local")
        .to_owned();
    let effects = crate::application::hooks::effect_classes_for_call(
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
            crate::application::hooks::SessionStartOutcome::Blocked
                | crate::application::hooks::SessionStartOutcome::SecurityFailure
                | crate::application::hooks::SessionStartOutcome::CapacityExhausted
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
                crate::application::hooks::HookEvent::PreToolUse,
                hook_payload.clone(),
                resume_index,
            )
            .await
    } else {
        state
            .hooks
            .invoke(
                crate::application::hooks::HookEvent::PreToolUse,
                hook_payload.clone(),
            )
            .await
    };
    if !matches!(
        pre.decision,
        crate::application::hooks::HookDecision::Continue
    ) {
        let approval_requested = matches!(
            pre.decision,
            crate::application::hooks::HookDecision::RequestApproval
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
            ToolCallResult::complete(vec![crate::interfaces::mcp::ToolResultContent {
                kind: "text",
                text: text.into(),
            }])
            .with_meta(json!({
                "control": { "type": "approval_required", "reason": "hook_request", "token": approval_token }
            }))
        } else {
            ToolCallResult::error(vec![crate::interfaces::mcp::ToolResultContent {
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
    if call.name == "telegram_send_message" {
        return telegram_message::handle(
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
        state,
        tool_dispatch::ToolDispatchContext {
            request,
            tool: &tool,
            arguments: &call.arguments,
            owner: &task_owner,
            effects,
            activity_start: &activity_start,
            request_started,
        },
    )
    .await
}
