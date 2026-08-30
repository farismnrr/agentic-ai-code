use super::super::{err_response, AppState};
use super::JsonErr2;
use axum::{http::StatusCode, Json};
use relay_application::activity::{self, ActivityEvent, Evidence, Status};
use relay_core::error::McpError;
use relay_interfaces::mcp::{self, Response, ToolCallResult};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

pub(super) fn record_activity_outcome(
    state: &AppState,
    start: &ActivityEvent,
    status: Status,
    duration_ms: u64,
    summary: &str,
    evidence: Evidence,
    payload: Option<Vec<u8>>,
) {
    record_activity_outcome_with_detail(
        state,
        start,
        status,
        duration_ms,
        (summary, None),
        evidence,
        payload,
    );
}

pub(super) fn record_activity_outcome_with_detail(
    state: &AppState,
    start: &ActivityEvent,
    status: Status,
    duration_ms: u64,
    presentation: (&str, Option<&str>),
    evidence: Evidence,
    payload: Option<Vec<u8>>,
) {
    let (summary, result_detail) = presentation;
    let event = activity::complete_event(
        start,
        status,
        duration_ms,
        summary,
        result_detail,
        evidence,
        payload.as_ref().map(|_| "activity_evidence:v1".into()),
    );
    let _ = state.activity.record_outcome(event, payload);
}

pub(super) fn deny_activity(
    state: &AppState,
    start: &ActivityEvent,
    request_started: Instant,
    summary: &str,
) {
    record_activity_outcome(
        state,
        start,
        Status::Denied,
        request_started.elapsed().as_millis() as u64,
        summary,
        Evidence::NotApplicable,
        None,
    );
}

pub(super) fn extract_activity_evidence(
    mut result: ToolCallResult,
) -> (ToolCallResult, Option<Vec<u8>>, Evidence, bool) {
    let Some(content) = result.content.first_mut() else {
        return (result, None, Evidence::NotApplicable, false);
    };
    let Ok(mut value) = serde_json::from_str::<Value>(&content.text) else {
        return (result, None, Evidence::Summary, false);
    };
    let Some(object) = value.as_object_mut() else {
        return (result, None, Evidence::Summary, false);
    };
    let Some(activity) = object.remove("_activity") else {
        return (result, None, Evidence::Summary, false);
    };
    let preview = activity
        .get("preview")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let evidence = match activity.get("evidence").and_then(Value::as_str) {
        Some("exact") => Evidence::Exact,
        Some("unavailable") => Evidence::Unavailable,
        _ => Evidence::Summary,
    };
    let payload = (evidence == Evidence::Exact && !preview)
        .then(|| serde_json::to_vec(&activity).ok())
        .flatten()
        .filter(|payload| payload.len() <= 512 * 1024);
    let payload_available = payload.is_some();
    content.text = serde_json::to_string(&value).unwrap_or_else(|_| content.text.clone());
    (
        result,
        payload,
        if payload_available {
            evidence
        } else {
            Evidence::Unavailable
        },
        preview,
    )
}

pub(crate) fn activity_result_detail(
    tool_name: &str,
    result: &ToolCallResult,
    arguments: &Value,
) -> Option<String> {
    if tool_name == "ssh_readonly_exec" {
        return None;
    }
    let raw = result
        .content
        .iter()
        .map(|content| content.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if raw.trim().is_empty() {
        return None;
    }
    let formatted = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or(raw);
    let mut detail = relay_core::redaction::redact_credentials(&formatted);
    if let Some(cwd) = arguments
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|cwd| !cwd.is_empty())
    {
        detail = detail.replace(cwd, ".");
    }
    let detail = detail
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .take(relay_application::activity::MAX_DETAIL)
        .collect::<String>();
    (!detail.trim().is_empty()).then_some(detail)
}

pub(crate) fn activity_result_summary(
    tool_name: &str,
    result: &ToolCallResult,
    preview: bool,
    detail: Option<&str>,
) -> String {
    if preview {
        return "dry-run preview; no workspace mutation was applied".into();
    }
    if let Some(detail) = detail {
        if tool_name == "terminal_exec" {
            let exit = detail.lines().find(|line| line.starts_with("Exit: "));
            let stdout = detail
                .split_once("Stdout:")
                .map(|(_, tail)| tail.split_once("Stderr:").map_or(tail, |(body, _)| body))
                .and_then(first_nonempty_line);
            let stderr = detail
                .split_once("Stderr:")
                .map(|(_, tail)| tail)
                .and_then(first_nonempty_line);
            if let Some(exit) = exit {
                let useful = if result.is_error {
                    stderr.or(stdout)
                } else {
                    stdout.or(stderr)
                };
                return useful
                    .map(|line| format!("{exit} · {}", truncate_summary(line)))
                    .unwrap_or_else(|| exit.to_string());
            }
        }
        if let Ok(value) = serde_json::from_str::<Value>(detail) {
            if let Some(summary) = json_result_summary(&value, 0) {
                return truncate_summary(summary);
            }
        }
        if let Some(line) = first_nonempty_line(detail) {
            return truncate_summary(line);
        }
    }
    if result.is_error {
        "tool execution failed".into()
    } else {
        "completed".into()
    }
}

fn json_result_summary(value: &Value, depth: usize) -> Option<&str> {
    if depth > 3 {
        return None;
    }
    match value {
        Value::String(value) => first_nonempty_line(value),
        Value::Array(values) => values
            .iter()
            .find_map(|value| json_result_summary(value, depth + 1)),
        Value::Object(object) => {
            for key in [
                "message", "text", "content", "result", "status", "path", "output",
            ] {
                if let Some(summary) = object
                    .get(key)
                    .and_then(|value| json_result_summary(value, depth + 1))
                {
                    return Some(summary);
                }
            }
            object
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "meta" | "_meta" | "timing" | "structuredContent"
                    )
                })
                .find_map(|(_, value)| json_result_summary(value, depth + 1))
        }
        _ => None,
    }
}

fn first_nonempty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

fn truncate_summary(value: &str) -> String {
    value.chars().take(220).collect()
}

pub(super) struct ToolCompletionContext<'a> {
    pub(super) request: &'a mcp::Request,
    pub(super) state: Arc<AppState>,
    pub(super) tool_name: &'a str,
    pub(super) arguments: &'a Value,
    pub(super) effects: Vec<&'static str>,
    pub(super) activity_start: &'a ActivityEvent,
    pub(super) dispatch_result: Result<ToolCallResult, McpError>,
    pub(super) request_started: Instant,
    pub(super) dispatch_ms: u64,
}

pub(super) async fn finish_tool_call(context: ToolCompletionContext<'_>) -> JsonErr2 {
    let ToolCompletionContext {
        request,
        state,
        tool_name,
        arguments,
        effects,
        activity_start,
        dispatch_result,
        request_started,
        dispatch_ms,
    } = context;
    let result = dispatch_result.unwrap_or_else(|err| {
        let text = match &err {
            McpError::InvalidRequest(_) | McpError::InvalidParams(_) => err.to_string(),
            _ => "Tool execution failed".to_string(),
        };
        ToolCallResult::error(vec![relay_interfaces::mcp::ToolResultContent {
            kind: "text",
            text,
        }])
    });
    let (result, payload, evidence, preview) = extract_activity_evidence(result);
    let activity_status = if result.is_error {
        Status::Error
    } else {
        Status::Ok
    };
    let activity_detail = activity_result_detail(tool_name, &result, arguments);
    let activity_summary =
        activity_result_summary(tool_name, &result, preview, activity_detail.as_deref());
    record_activity_outcome_with_detail(
        &state,
        activity_start,
        activity_status,
        dispatch_ms,
        (&activity_summary, activity_detail.as_deref()),
        evidence,
        payload,
    );
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
                "tool_id": tool_name,
                "effect_classes": effects,
                "cwd": arguments.get("cwd").cloned().unwrap_or(Value::Null),
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

pub(super) fn bounded_tool_error(
    id: &mcp::Id,
    message: &str,
    request_started: Instant,
) -> JsonErr2 {
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

pub(super) fn agent_session_from_params(params: Option<&Value>) -> Option<String> {
    params?
        .get("_meta")?
        .get("io.modelcontextprotocol/agentSession")?
        .as_str()
        .map(|value| value.chars().take(128).collect())
}

pub(super) fn client_supports_tasks(params: Option<&Value>) -> bool {
    params
        .and_then(|value| value.get("_meta"))
        .and_then(|value| value.get("io.modelcontextprotocol/clientCapabilities"))
        .and_then(|value| value.get("extensions"))
        .and_then(|value| value.get("io.modelcontextprotocol/tasks"))
        .is_some()
}

pub(super) fn requires_idempotency_key(tool: &str, arguments: &Value) -> bool {
    if tool == "ssh_readonly_exec" {
        return false;
    }
    if tool == "terminal_exec" {
        return true;
    }
    if tool != "http_fetch" {
        return false;
    }
    !matches!(
        arguments
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET")
            .to_ascii_uppercase()
            .as_str(),
        "GET" | "HEAD" | "OPTIONS"
    )
}
