use super::super::tool_helpers::{record_activity_outcome, record_activity_outcome_with_detail};
use super::{JsonErr2, ToolCallContext};
use crate::application::activity::{Evidence, Status};
use crate::core::error::McpError;
use crate::interfaces::mcp::{self, Response, ToolCallResult, ToolResultContent};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(super) async fn handle(context: ToolCallContext<'_>) -> JsonErr2 {
    let tool_dispatch_started = Instant::now();
    let task_id = match crate::application::execution::start_tool_task_for(
        context.tool,
        &context.call.arguments,
        &context.state.config,
        &context.state.jobs,
        context.idempotency_key,
        context.request_fingerprint.clone(),
        context.owner,
        context.session,
    )
    .await
    {
        Ok(task_id) => task_id,
        Err(err) => {
            record_activity_outcome(
                &context.state,
                context.activity_start,
                Status::Error,
                context.request_started.elapsed().as_millis() as u64,
                "terminal task could not be started",
                Evidence::Summary,
                None,
            );
            return Err(super::super::super::err_response(
                StatusCode::BAD_REQUEST,
                Some(context.request.id.clone()),
                &err,
            ));
        }
    };
    let completed = match context
        .state
        .jobs
        .wait_for_completion_within(&task_id, Duration::from_millis(context.sync_wait_ms))
        .await
    {
        Ok(snapshot) => snapshot,
        Err(err) => {
            record_activity_outcome(
                &context.state,
                context.activity_start,
                Status::Error,
                context.request_started.elapsed().as_millis() as u64,
                "terminal task disappeared before handoff",
                Evidence::Summary,
                None,
            );
            return Err(super::super::super::err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(context.request.id.clone()),
                &err,
            ));
        }
    };

    if let Some(task) = completed.filter(|task| task.finished_at.is_some()) {
        let dispatch_ms = tool_dispatch_started.elapsed().as_millis() as u64;
        return finish_completed(context, dispatch_ms, task).await;
    }

    let task = match context
        .state
        .jobs
        .get_for(&task_id, context.owner, context.session)
        .await
    {
        Some(task) => task,
        None => {
            record_activity_outcome(
                &context.state,
                context.activity_start,
                Status::Error,
                context.request_started.elapsed().as_millis() as u64,
                "terminal task disappeared before handoff",
                Evidence::Summary,
                None,
            );
            return Err(super::super::super::err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(context.request.id.clone()),
                &McpError::Internal("terminal task disappeared before handoff".into()),
            ));
        }
    };
    if task.finished_at.is_some() {
        let dispatch_ms = tool_dispatch_started.elapsed().as_millis() as u64;
        return finish_completed(context, dispatch_ms, task).await;
    }

    let ToolCallContext {
        request,
        state,
        call,
        activity_start,
        effects,
        client_has_tasks,
        request_started,
        ..
    } = context;
    let task_detail = serde_json::to_string_pretty(&task.job_json()).unwrap_or_default();
    let task_summary = format!("command handed off as task {task_id}");
    record_activity_outcome_with_detail(
        &state,
        activity_start,
        Status::Running,
        tool_dispatch_started.elapsed().as_millis() as u64,
        (&task_summary, Some(&task_detail)),
        Evidence::NotApplicable,
        None,
    );
    super::super::super::task_lifecycle::observe(
        state.clone(),
        task_id,
        call.name.clone(),
        effects.to_vec(),
        call.arguments.get("cwd").cloned().unwrap_or(Value::Null),
        activity_start.clone(),
    );
    let result = mcp::with_timing_meta(
        automatic_terminal_result(&task, client_has_tasks),
        tool_dispatch_started.elapsed().as_millis() as u64,
        request_started.elapsed().as_millis() as u64,
    );
    let response = Response::new(request.id.clone(), result);
    Ok(Json(serde_json::to_value(response).unwrap_or(json!({}))))
}

pub(super) fn automatic_request_key(id: &mcp::Id) -> String {
    let encoded = serde_json::to_vec(id).unwrap_or_default();
    let digest = ring::digest::digest(&ring::digest::SHA256, &encoded);
    let short_digest = digest.as_ref()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("auto:{short_digest}")
}

async fn finish_completed(
    context: ToolCallContext<'_>,
    dispatch_ms: u64,
    task: crate::application::execution::JobSnapshot,
) -> JsonErr2 {
    let ToolCallContext {
        request,
        state,
        call,
        activity_start,
        effects,
        request_started,
        ..
    } = context;
    let result = task.result.unwrap_or_else(|| {
        ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: "terminal execution finished without a result".into(),
        }])
    });
    super::super::tool_helpers::finish_tool_call(
        super::super::tool_helpers::ToolCompletionContext {
            request,
            state,
            tool_name: &call.name,
            arguments: &call.arguments,
            effects: effects.to_vec(),
            activity_start,
            dispatch_result: Ok(result),
            request_started,
            dispatch_ms,
        },
    )
    .await
}

pub fn automatic_terminal_result(
    task: &crate::application::execution::JobSnapshot,
    client_has_tasks: bool,
) -> Value {
    if task.finished_at.is_some() {
        return serde_json::to_value(task.result.clone().unwrap_or_else(|| {
            ToolCallResult::error(vec![ToolResultContent {
                kind: "text",
                text: "terminal execution finished without a result".into(),
            }])
        }))
        .unwrap_or_else(|_| json!({}));
    }
    if client_has_tasks {
        return task.create_task_json();
    }

    let task_id = &task.job_id;
    let result = ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text: format!(
            "Command is still running as terminal task {task_id}. Poll terminal_job_get with {{\"taskId\":\"{task_id}\"}}; do not rerun this command."
        ),
    }])
    .with_meta(json!({
        "terminalJob": {
            "taskId": task_id,
            "status": task.state.task_status(),
            "pollTool": "terminal_job_get",
            "pollArguments": { "taskId": task_id },
            "pollIntervalMs": 1000
        }
    }));
    serde_json::to_value(result).unwrap_or_else(|_| json!({}))
}
