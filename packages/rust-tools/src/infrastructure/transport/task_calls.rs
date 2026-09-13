use super::super::{err_response, AppState};
use super::tool_helpers::{
    bounded_tool_error, record_activity_outcome, record_activity_outcome_with_detail,
};
use super::JsonErr2;
#[path = "task_calls/auto_terminal.rs"]
mod auto_terminal;
use crate::application::activity::{ActivityEvent, Evidence, Status};
use crate::core::error::McpError;
use crate::interfaces::mcp::{self, Response, Tool, ToolsCallParams};
pub(super) use auto_terminal::automatic_terminal_result;
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

pub(super) struct ToolCallContext<'a> {
    pub(super) request: &'a mcp::Request,
    pub(super) state: Arc<AppState>,
    pub(super) call: &'a ToolsCallParams,
    pub(super) tool: &'a Tool,
    pub(super) activity_start: &'a ActivityEvent,
    pub(super) effects: &'a [&'static str],
    pub(super) execute_async: bool,
    pub(super) automatic_handoff: bool,
    pub(super) sync_wait_ms: u64,
    pub(super) client_has_tasks: bool,
    pub(super) idempotency_key: Option<&'a str>,
    pub(super) request_fingerprint: String,
    pub(super) owner: &'a str,
    pub(super) session: Option<&'a str>,
    pub(super) request_started: Instant,
}

pub(super) struct ToolCallRouting {
    pub(super) automatic_handoff: bool,
    pub(super) sync_wait_ms: u64,
    pub(super) idempotency_key: Option<String>,
}

pub(super) struct IdempotentReplayContext<'a> {
    pub(super) request: &'a mcp::Request,
    pub(super) state: &'a AppState,
    pub(super) idempotency_key: Option<&'a str>,
    pub(super) request_fingerprint: &'a str,
    pub(super) owner: &'a str,
    pub(super) session: Option<&'a str>,
    pub(super) automatic_handoff: bool,
    pub(super) client_has_tasks: bool,
    pub(super) request_started: Instant,
}

pub(super) fn resolve_tool_call_routing(
    request_id: &mcp::Id,
    call: &ToolsCallParams,
    execution_mode: &str,
    client_has_tasks: bool,
    tool_has_tasks: bool,
    owner: &str,
    session: Option<&str>,
) -> ToolCallRouting {
    let automatic_handoff = call.name == "terminal_exec" && execution_mode == "auto";
    let sync_wait_ms = call
        .arguments
        .get("sync_wait_ms")
        .and_then(Value::as_u64)
        .unwrap_or(mcp::DEFAULT_TERMINAL_SYNC_WAIT_MS);
    let explicit_key = call
        .arguments
        .get("idempotency_key")
        .and_then(Value::as_str);
    let session = session.unwrap_or("none");
    let idempotency_key = explicit_key
        .filter(|_| {
            call.name == "terminal_job_start"
                || execution_mode == "async"
                || automatic_handoff
                || (execution_mode == "auto" && client_has_tasks && tool_has_tasks)
        })
        .map(|key| format!("{owner}:{session}:{}:{key}", call.name))
        .or_else(|| {
            automatic_handoff.then(|| {
                format!(
                    "{owner}:{session}:terminal_exec:{}",
                    auto_terminal::automatic_request_key(request_id)
                )
            })
        });
    ToolCallRouting {
        automatic_handoff,
        sync_wait_ms,
        idempotency_key,
    }
}

pub(super) async fn try_replay_idempotent_task(
    context: IdempotentReplayContext<'_>,
) -> Option<JsonErr2> {
    let IdempotentReplayContext {
        request,
        state,
        idempotency_key,
        request_fingerprint,
        owner,
        session,
        automatic_handoff,
        client_has_tasks,
        request_started,
    } = context;
    let key = idempotency_key?;
    match state
        .jobs
        .existing_idempotency_key_for(key, request_fingerprint, owner, session)
        .await
    {
        Ok(Some(task_id)) => {
            let Some(task) = state.jobs.get_for(&task_id, owner, session).await else {
                return Some(bounded_tool_error(
                    &request.id,
                    "accepted task is no longer available",
                    request_started,
                ));
            };
            let result = mcp::with_timing_meta(
                if automatic_handoff {
                    automatic_terminal_result(&task, client_has_tasks)
                } else {
                    task.create_task_json()
                },
                0,
                request_started.elapsed().as_millis() as u64,
            );
            let response = Response::new(request.id.clone(), result);
            Some(Ok(Json(
                serde_json::to_value(response).unwrap_or(json!({})),
            )))
        }
        Ok(None) => None,
        Err(err) => Some(bounded_tool_error(
            &request.id,
            &err.to_string(),
            request_started,
        )),
    }
}

pub(super) async fn try_handle_task_call(context: ToolCallContext<'_>) -> Option<JsonErr2> {
    let ToolCallContext {
        request,
        state,
        call,
        tool,
        activity_start,
        effects,
        execute_async,
        automatic_handoff,
        sync_wait_ms,
        client_has_tasks,
        idempotency_key,
        request_fingerprint,
        owner,
        session,
        request_started,
    } = context;
    if call.name == "terminal_job_start" {
        let tool_dispatch_started = Instant::now();
        let task_id = match match idempotency_key {
            Some(key) => {
                crate::application::execution::start_terminal_job_for_with_idempotency(
                    &call.arguments,
                    &state.config,
                    &state.jobs,
                    key,
                    request_fingerprint.clone(),
                    owner,
                    session,
                )
                .await
            }
            None => {
                crate::application::execution::start_terminal_job_for(
                    &call.arguments,
                    &state.config,
                    &state.jobs,
                    owner,
                    session,
                )
                .await
            }
        } {
            Ok(task_id) => task_id,
            Err(err) => {
                record_activity_outcome(
                    &state,
                    activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "task could not be started",
                    Evidence::Summary,
                    None,
                );
                return Some(Err(err_response(
                    StatusCode::BAD_REQUEST,
                    Some(request.id.clone()),
                    &err,
                )));
            }
        };
        let task = match state.jobs.get_for(&task_id, owner, session).await {
            Some(task) => task,
            None => {
                record_activity_outcome(
                    &state,
                    activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "task creation failed",
                    Evidence::Summary,
                    None,
                );
                return Some(Err(err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(request.id.clone()),
                    &McpError::Internal("task creation failed".into()),
                )));
            }
        };
        let result = mcp::with_timing_meta(
            task.create_task_json(),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let task_detail = serde_json::to_string_pretty(&task.job_json()).unwrap_or_default();
        let task_summary = format!("task {task_id} accepted");
        record_activity_outcome_with_detail(
            &state,
            activity_start,
            Status::Running,
            tool_dispatch_started.elapsed().as_millis() as u64,
            (&task_summary, Some(&task_detail)),
            Evidence::NotApplicable,
            None,
        );
        super::super::task_lifecycle::observe(
            state.clone(),
            task_id,
            call.name.clone(),
            effects.to_vec(),
            call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
            activity_start.clone(),
        );
        let response = Response::new(request.id.clone(), result);
        return Some(Ok(Json(
            serde_json::to_value(response).unwrap_or(json!({})),
        )));
    }

    if call.name == "terminal_job_get" || call.name == "terminal_job_cancel" {
        let tool_dispatch_started = Instant::now();
        let id = match call.arguments.get("taskId").and_then(Value::as_str) {
            Some(id) => id,
            None => {
                record_activity_outcome(
                    &state,
                    activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "task id is missing",
                    Evidence::Summary,
                    None,
                );
                return Some(Err(err_response(
                    StatusCode::BAD_REQUEST,
                    Some(request.id.clone()),
                    &McpError::InvalidParams("taskId is required".into()),
                )));
            }
        };
        let task = if call.name == "terminal_job_cancel" {
            match state.jobs.cancel_for(id, owner, session).await {
                Ok(task) => task,
                Err(err) => {
                    record_activity_outcome(
                        &state,
                        activity_start,
                        Status::Error,
                        request_started.elapsed().as_millis() as u64,
                        "task could not be cancelled",
                        Evidence::Summary,
                        None,
                    );
                    return Some(Err(err_response(
                        StatusCode::BAD_REQUEST,
                        Some(request.id.clone()),
                        &err,
                    )));
                }
            }
        } else {
            match state.jobs.get_for(id, owner, session).await {
                Some(task) => task,
                None => {
                    record_activity_outcome(
                        &state,
                        activity_start,
                        Status::Error,
                        request_started.elapsed().as_millis() as u64,
                        "unknown task",
                        Evidence::Summary,
                        None,
                    );
                    return Some(Err(err_response(
                        StatusCode::NOT_FOUND,
                        Some(request.id.clone()),
                        &McpError::InvalidParams("unknown task".into()),
                    )));
                }
            }
        };
        let result = mcp::with_timing_meta(
            task.task_json(state.config.completed_job_ttl_ms),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let task_detail = crate::core::redaction::redact_credentials(&task.output_text());
        let task_summary = if call.name == "terminal_job_cancel" {
            format!("cancel requested · {id}")
        } else {
            summarize_task_output(&task_detail)
        };
        record_activity_outcome_with_detail(
            &state,
            activity_start,
            if call.name == "terminal_job_cancel" {
                Status::Cancelled
            } else {
                Status::Ok
            },
            tool_dispatch_started.elapsed().as_millis() as u64,
            (&task_summary, Some(&task_detail)),
            Evidence::Summary,
            None,
        );
        let response = Response::new(request.id.clone(), result);
        return Some(Ok(Json(
            serde_json::to_value(response).unwrap_or(json!({})),
        )));
    }

    if automatic_handoff {
        return Some(
            auto_terminal::handle(ToolCallContext {
                request,
                state,
                call,
                tool,
                activity_start,
                effects,
                execute_async,
                automatic_handoff,
                sync_wait_ms,
                client_has_tasks,
                idempotency_key,
                request_fingerprint,
                owner,
                session,
                request_started,
            })
            .await,
        );
    }

    if execute_async {
        let tool_dispatch_started = Instant::now();
        let task_id = match crate::application::execution::start_tool_task_for(
            tool,
            &call.arguments,
            &state.config,
            &state.jobs,
            idempotency_key,
            request_fingerprint,
            owner,
            session,
        )
        .await
        {
            Ok(task_id) => task_id,
            Err(err) => {
                record_activity_outcome(
                    &state,
                    activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "task could not be started",
                    Evidence::Summary,
                    None,
                );
                return Some(Err(err_response(
                    StatusCode::BAD_REQUEST,
                    Some(request.id.clone()),
                    &err,
                )));
            }
        };
        let task = match state.jobs.get_for(&task_id, owner, session).await {
            Some(task) => task,
            None => {
                record_activity_outcome(
                    &state,
                    activity_start,
                    Status::Error,
                    request_started.elapsed().as_millis() as u64,
                    "task creation failed",
                    Evidence::Summary,
                    None,
                );
                return Some(Err(err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(request.id.clone()),
                    &McpError::Internal("task creation failed".into()),
                )));
            }
        };
        record_activity_outcome(
            &state,
            activity_start,
            Status::Running,
            tool_dispatch_started.elapsed().as_millis() as u64,
            "task accepted for execution",
            Evidence::NotApplicable,
            None,
        );
        super::super::task_lifecycle::observe(
            state.clone(),
            task_id,
            call.name.clone(),
            effects.to_vec(),
            call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
            activity_start.clone(),
        );
        let result = mcp::with_timing_meta(
            task.create_task_json(),
            tool_dispatch_started.elapsed().as_millis() as u64,
            request_started.elapsed().as_millis() as u64,
        );
        let response = Response::new(request.id.clone(), result);
        return Some(Ok(Json(
            serde_json::to_value(response).unwrap_or(json!({})),
        )));
    }

    None
}

fn summarize_task_output(detail: &str) -> String {
    let exit = detail.lines().find(|line| line.starts_with("Exit: "));
    let stdout = detail
        .split_once("Stdout:")
        .map(|(_, tail)| tail.split_once("Stderr:").map_or(tail, |(body, _)| body))
        .and_then(first_nonempty_line);
    let stderr = detail
        .split_once("Stderr:")
        .map(|(_, tail)| tail)
        .and_then(first_nonempty_line);
    match (exit, stdout.or(stderr)) {
        (Some(exit), Some(output)) => {
            format!("{exit} · {}", output.chars().take(220).collect::<String>())
        }
        (Some(exit), None) => exit.to_string(),
        (None, Some(output)) => output.chars().take(220).collect(),
        (None, None) => "no output".into(),
    }
}

fn first_nonempty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}
