//! Deferred lifecycle completion for MCP task-backed tool calls.

use super::AppState;
use relay_application::hooks::HookEvent;
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn observe(
    state: Arc<AppState>,
    task_id: String,
    tool_id: String,
    effects: Vec<&'static str>,
    cwd: Value,
) {
    tokio::spawn(async move {
        let (event, success) = match state.jobs.wait(&task_id).await {
            Ok(snapshot) => {
                let failed = !matches!(
                    snapshot.state,
                    relay_application::execution::JobState::Completed
                ) || snapshot
                    .result
                    .as_ref()
                    .is_some_and(|result| result.is_error);
                if failed {
                    (HookEvent::ToolError, false)
                } else {
                    (HookEvent::PostToolUse, true)
                }
            }
            Err(_) => (HookEvent::ToolError, false),
        };
        let _ = state
            .hooks
            .invoke(
                event,
                json!({
                    "hook_event": event.name(),
                    "tool_id": tool_id,
                    "effect_classes": effects,
                    "cwd": cwd,
                    "success": success,
                    "reason": "task_result",
                }),
            )
            .await;
    });
}
