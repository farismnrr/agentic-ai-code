//! Deferred lifecycle completion for MCP task-backed tool calls.

use super::tools::{activity_result_detail, activity_result_summary};
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
        let waited = state.jobs.wait(&task_id).await;
        let (event, success, status) = match &waited {
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
        let snapshot = waited.as_ref().ok();
        let duration_ms = snapshot
            .and_then(|snapshot| snapshot.execution_duration_ms)
            .unwrap_or(0);
        let arguments = json!({ "cwd": cwd.clone() });
        let result_detail = if tool_id == "ssh_readonly_exec" {
            None
        } else {
            snapshot
                .and_then(|snapshot| snapshot.result.as_ref())
                .and_then(|result| activity_result_detail(&tool_id, result, &arguments))
                .or_else(|| {
                    snapshot.map(|snapshot| {
                        relay_core::redaction::redact_credentials(&snapshot.output_text())
                    })
                })
        };
        let result_summary = snapshot
            .and_then(|snapshot| snapshot.result.as_ref())
            .map(|result| {
                activity_result_summary(&tool_id, result, false, result_detail.as_deref())
            })
            .unwrap_or_else(|| summarize_deferred_result(result_detail.as_deref(), success));
        let _ = state.activity.record_outcome(
            activity::complete_event(
                &activity_start,
                status,
                duration_ms,
                &result_summary,
                result_detail.as_deref(),
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

fn summarize_deferred_result(detail: Option<&str>, success: bool) -> String {
    detail
        .and_then(|detail| detail.lines().map(str::trim).find(|line| !line.is_empty()))
        .map(|line| line.chars().take(220).collect())
        .unwrap_or_else(|| {
            if success {
                "completed".into()
            } else {
                "failed".into()
            }
        })
}
