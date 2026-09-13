use super::{format_timestamp, render_output, JobSnapshot, JobState};
use crate::core::redaction::redact_credentials;
use serde_json::{json, Value};

impl JobSnapshot {
    pub fn create_task_json(&self) -> Value {
        let mut value = json!({
            "resultType": "task",
            "taskId": self.job_id,
            "status": self.task_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "ttlMs": Value::Null,
            "pollIntervalMs": 1000,
            "output": self.task_output_json()
        });
        self.add_execution_status(&mut value);
        value
    }
    pub fn task_json(&self, completed_retention_ms: u64) -> Value {
        let ttl_ms = self.finished_at.map(|finished_at| {
            let lifetime = finished_at.saturating_sub(self.created_at);
            u64::try_from(lifetime)
                .unwrap_or(u64::MAX)
                .saturating_add(completed_retention_ms)
        });
        let mut value = json!({
            "resultType": "complete",
            "taskId": self.job_id,
            "status": self.task_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "ttlMs": ttl_ms,
            "pollIntervalMs": 1000
        });
        value["output"] = self.task_output_json();
        self.add_execution_status(&mut value);
        if let Some(duration_ms) = self.execution_duration_ms {
            value["executionDurationMs"] = json!(duration_ms);
        }
        match self.state {
            JobState::Completed | JobState::TimedOut => {
                if let Some(result) = &self.result {
                    value["result"] = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
                }
            }
            JobState::Failed => {
                if let Some(result) = &self.result {
                    value["result"] = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
                } else {
                    value["error"] = json!({
                        "code": -32603,
                        "message": "Tool execution failed"
                    });
                }
            }
            JobState::Queued | JobState::Running | JobState::Cancelled => {}
        }
        value
    }
    pub fn job_json(&self) -> Value {
        let mut value = json!({
            "taskId": self.job_id,
            "status": self.job_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "output": {
                "stdout": redact_credentials(&self.stdout),
                "stderr": redact_credentials(&self.stderr),
                "omittedBytes": self.omitted_bytes,
                "exitCode": self.exit_code
            }
        });
        if let Some(duration_ms) = self.execution_duration_ms {
            value["executionDurationMs"] = json!(duration_ms);
        }
        if let Some(result) = &self.result {
            value["result"] = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
        }
        value
    }
    fn task_status(&self) -> &'static str {
        self.state.task_status()
    }
    fn add_execution_status(&self, value: &mut Value) {
        if self.state == JobState::TimedOut {
            value["executionStatus"] = json!("timed_out");
        }
    }
    fn task_output_json(&self) -> Value {
        json!({
            "stdout": redact_credentials(&self.stdout),
            "stderr": redact_credentials(&self.stderr),
            "omittedBytes": self.omitted_bytes,
            "exitCode": self.exit_code
        })
    }
    fn job_status(&self) -> &'static str {
        match self.state {
            JobState::Queued => "queued",
            JobState::Running => "working",
            JobState::Completed => "completed",
            JobState::Failed => "failed",
            JobState::TimedOut => "timed_out",
            JobState::Cancelled => "cancelled",
        }
    }
    pub fn output_text(&self) -> String {
        render_output(
            self.exit_code.unwrap_or(-1),
            &redact_credentials(&self.stdout),
            &redact_credentials(&self.stderr),
            self.omitted_bytes,
        )
    }
}
