use super::tool_helpers::{finish_tool_call, ToolCompletionContext};
use super::{AppState, JsonErr2};
use crate::application::activity::ActivityEvent;
use crate::interfaces::mcp;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

pub(super) struct ToolDispatchContext<'a> {
    pub request: &'a mcp::Request,
    pub tool: &'a mcp::Tool,
    pub arguments: &'a Value,
    pub owner: &'a str,
    pub effects: Vec<&'static str>,
    pub activity_start: &'a ActivityEvent,
    pub request_started: Instant,
}

pub(super) async fn handle(state: Arc<AppState>, context: ToolDispatchContext<'_>) -> JsonErr2 {
    let tool_dispatch_started = Instant::now();
    let dispatch_result = crate::application::execution::dispatch_tool_call(
        context.tool,
        context.arguments,
        &state.config,
        &state.jobs,
        &state.lsp,
        &state.hooks,
        context.owner,
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
        tool = context.tool.name,
        duration_ms = dispatch_ms,
    );
    finish_tool_call(ToolCompletionContext {
        request: context.request,
        state,
        tool_name: context.tool.name,
        arguments: context.arguments,
        effects: context.effects,
        activity_start: context.activity_start,
        dispatch_result,
        request_started: context.request_started,
        dispatch_ms,
    })
    .await
}
