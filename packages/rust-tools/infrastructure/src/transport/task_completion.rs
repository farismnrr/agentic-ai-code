use super::tool_helpers::{finish_tool_call, ToolCompletionContext};
use super::{AppState, JsonErr2};
use relay_application::activity::ActivityEvent;
use relay_core::error::McpError;
use relay_interfaces::mcp::{self, ToolCallResult};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

pub(super) async fn handle(
    request: &mcp::Request,
    state: Arc<AppState>,
    arguments: &Value,
    effects: Vec<&'static str>,
    activity_start: &ActivityEvent,
    request_started: Instant,
) -> JsonErr2 {
    let tool_dispatch_started = Instant::now();
    let dispatch_result = match crate::notifications::payload_from_arguments(arguments, "external-mcp") {
        Ok(payload) => state
            .notifications
            .enqueue(payload)
            .await
            .map(|status| {
                ToolCallResult::complete(vec![relay_interfaces::mcp::ToolResultContent {
                    kind: "text",
                    text: serde_json::to_string(&json!({
                        "resultType": "complete",
                        "status": status.as_str()
                    }))
                    .unwrap_or_else(|_| "{\"status\":\"queued\"}".into()),
                }])
            })
            .map_err(|_| McpError::Internal("task completion notification is unavailable".into())),
        Err(_) => Err(McpError::InvalidParams(
            "invalid task completion payload".into(),
        )),
    };
    let dispatch_ms = tool_dispatch_started.elapsed().as_millis() as u64;
    finish_tool_call(ToolCompletionContext {
        request,
        state,
        tool_name: "task_completed",
        arguments,
        effects,
        activity_start,
        dispatch_result,
        request_started,
        dispatch_ms,
    })
    .await
}
