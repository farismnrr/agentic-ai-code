use super::super::{err_response, AppState};
use super::tool_helpers::record_activity_outcome;
use super::JsonErr2;
use crate::application::activity::{ActivityEvent, Evidence, Status};
use crate::application::execution::JobSnapshot;
use crate::core::error::McpError;
use crate::interfaces::mcp::{
    self, Response, Tool, ToolCallResult, ToolResultContent, ToolsCallParams,
};
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
    pub(super) idempotency_key: Option<&'a str>,
    pub(super) request_fingerprint: String,
    pub(super) owner: &'a str,
    pub(super) session: Option<&'a str>,
    pub(super) request_started: Instant,
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
        idempotency_key,
        request_fingerprint,
        owner,
        session,
        request_started,
    } = context;
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

pub(super) fn terminal_job_tool_result(
    task: &JobSnapshot,
    dispatch_ms: u64,
    server_total_ms: u64,
) -> Value {
    let text = serde_json::to_string(&task.job_json()).unwrap_or_else(|_| "{}".into());
    let result = ToolCallResult::complete(vec![ToolResultContent { kind: "text", text }]);
    mcp::with_timing_meta(
        serde_json::to_value(result).unwrap_or_else(|_| json!({})),
        dispatch_ms,
        server_total_ms,
    )
}
