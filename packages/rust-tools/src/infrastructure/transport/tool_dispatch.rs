use super::tool_helpers::{finish_tool_call, ToolCompletionContext};
use super::{AppState, JsonErr2};
use crate::application::activity::ActivityEvent;
use crate::interfaces::mcp;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

pub(super) async fn handle(
    request: &mcp::Request,
    state: Arc<AppState>,
    tool: &mcp::Tool,
    arguments: &Value,
    effects: Vec<&'static str>,
    activity_start: &ActivityEvent,
    request_started: Instant,
) -> JsonErr2 {
    let tool_dispatch_started = Instant::now();
    let dispatch_result = crate::application::execution::dispatch_tool_call(
        tool,
        arguments,
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
        tool = tool.name,
        duration_ms = dispatch_ms,
    );
    finish_tool_call(ToolCompletionContext {
        request,
        state,
        tool_name: tool.name,
        arguments,
        effects,
        activity_start,
        dispatch_result,
        request_started,
        dispatch_ms,
    })
    .await
}
