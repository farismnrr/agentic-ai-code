//! Deferred lifecycle completion for MCP task-backed tool calls.

use super::AppState;
use relay_application::activity::{self, ActivityEvent, Evidence, Status};
use relay_application::hooks::HookEvent;
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn observe(
    state: Arc<AppState>,
    task_id: String,
    tool_id: String,
    effects: Vec<&'static str>,
    cwd: Value,
    activity_start: ActivityEvent,
) {
    tokio::spawn(async move {
        let (event, success, status) = match state.jobs.wait(&task_id).await {
            Ok(snapshot) => {
                let failed = !matches!(
                    snapshot.state,
                    relay_application::execution::JobState::Completed
                ) || snapshot
                    .result
                    .as_ref()
                    .is_some_and(|result| result.is_error);
                if failed {
                    (
                        HookEvent::ToolError,
                        false,
                        if matches!(
                            snapshot.state,
                            relay_application::execution::JobState::Cancelled
                        ) {
                            Status::Cancelled
                        } else {
                            Status::Error
                        },
                    )
                } else {
                    (HookEvent::PostToolUse, true, Status::Ok)
                }
            }
            Err(_) => (HookEvent::ToolError, false, Status::Interrupted),
        };
        let duration_ms = state
            .jobs
            .get(&task_id)
            .await
            .and_then(|snapshot| snapshot.execution_duration_ms)
            .unwrap_or(0);
        let _ = state.activity.record_outcome(
            activity::complete_event(
                &activity_start,
                status,
                duration_ms,
                if success {
                    "task completed"
                } else {
                    "task failed"
                },
                Evidence::Summary,
                None,
            ),
            None,
        );
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
