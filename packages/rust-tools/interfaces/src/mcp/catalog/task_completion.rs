use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "task_completed",
        title: Some("Task Completed"),
        description: "Signal once that the entire implementation task or plan has completed successfully. This is not a progress, activity, or per-tool notification; call it at most once for the logical task.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string", "minLength": 1, "maxLength": 128 },
                "title": { "type": "string", "minLength": 1, "maxLength": 160 },
                "summary": { "type": "string", "minLength": 1, "maxLength": 2000 },
                "resultUrl": { "type": "string", "format": "uri", "maxLength": 2048 }
            },
            "required": ["taskId", "title", "summary"],
            "additionalProperties": false
        }),
        annotations: Some(ToolAnnotations {
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: true,
        }),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}
