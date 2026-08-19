use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn issue_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "issue_list",
            title: Some("Issue List"),
            description: "List bounded issue summaries for the validated forge repository using a provider-neutral result contract.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "state": { "type": "string", "enum": ["open", "closed", "all"], "default": "open" },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                        "maxItems": 10
                    }
                },
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_get",
            title: Some("Issue Get"),
            description: "Read one bounded issue summary and body for the validated forge repository.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "number": { "type": "integer", "minimum": 1 }
                },
                "required": ["number"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_create",
            title: Some("Issue Create"),
            description: "Create a new issue in the validated forge repository without arbitrary provider API access.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "title": { "type": "string", "minLength": 1, "maxLength": 256 },
                    "body": { "type": "string", "maxLength": 65536, "default": "" },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                        "maxItems": 50
                    }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_update",
            title: Some("Issue Update"),
            description: "Update bounded title, body, and labels on one validated issue; arbitrary provider flags are unavailable.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "number": { "type": "integer", "minimum": 1 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 256 },
                    "body": { "type": "string", "maxLength": 65536 },
                    "add_labels": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                        "maxItems": 50
                    },
                    "remove_labels": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                        "maxItems": 50
                    }
                },
                "required": ["number"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_comment",
            title: Some("Issue Comment"),
            description: "Add a bounded comment to one validated issue without returning or querying full comment threads.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "number": { "type": "integer", "minimum": 1 },
                    "body": { "type": "string", "minLength": 1, "maxLength": 65536 }
                },
                "required": ["number", "body"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_close",
            title: Some("Issue Close"),
            description: "Close one validated issue with an explicit normalized reason and optional atomic comment; verify closed post-state before returning.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "number": { "type": "integer", "minimum": 1 },
                    "reason": { "type": "string", "enum": ["completed", "not_planned", "duplicate"] },
                    "duplicate_of": { "type": "integer", "minimum": 1 },
                    "comment": { "type": "string", "minLength": 1, "maxLength": 65536 }
                },
                "required": ["number", "reason"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "issue_reopen",
            title: Some("Issue Reopen"),
            description: "Reopen one validated issue with optional atomic comment; verify open post-state before returning.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "remote": { "type": "string", "minLength": 1, "maxLength": 64, "default": "origin" },
                    "number": { "type": "integer", "minimum": 1 },
                    "comment": { "type": "string", "minLength": 1, "maxLength": 65536 }
                },
                "required": ["number"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
    ]
}

pub(super) fn action_tools() -> Vec<Tool> {
    vec![
        read_tool(
            "workflow_list",
            "Workflow List",
            "List bounded GitHub Actions workflows for the validated repository.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"}},"additionalProperties":false}),
        ),
        read_tool(
            "workflow_get",
            "Workflow Get",
            "Read one workflow by immutable numeric ID.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"workflow_id":{"type":"integer","minimum":1}},"required":["workflow_id"],"additionalProperties":false}),
        ),
        read_tool(
            "workflow_run_list",
            "Workflow Run List",
            "List bounded GitHub Actions runs with reviewed filters.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"workflow_id":{"type":"integer","minimum":1},"branch":{"type":"string","minLength":1,"maxLength":256},"commit_sha":{"type":"string","pattern":"^[0-9a-fA-F]{40}$"},"status":{"type":"string","enum":["queued","in_progress","completed","requested","waiting","pending","success","failure","cancelled","skipped","timed_out","action_required","neutral","stale","startup_failure"]}},"additionalProperties":false}),
        ),
        read_tool(
            "workflow_run_get",
            "Workflow Run Get",
            "Read one bounded workflow run without duplicating job payloads.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"run_id":{"type":"integer","minimum":1}},"required":["run_id"],"additionalProperties":false}),
        ),
        read_tool(
            "workflow_run_jobs",
            "Workflow Run Jobs",
            "Read bounded jobs and steps for one workflow run.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"run_id":{"type":"integer","minimum":1}},"required":["run_id"],"additionalProperties":false}),
        ),
        read_tool(
            "workflow_job_log_preview",
            "Workflow Job Log Preview",
            "Return a bounded credential-redacted diagnostic preview for one job.",
            json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"job_id":{"type":"integer","minimum":1},"failed_only":{"type":"boolean","default":true},"max_lines":{"type":"integer","minimum":1,"maximum":200,"default":100}},"required":["job_id"],"additionalProperties":false}),
        ),
    ]
}

fn read_tool(
    name: &'static str,
    title: &'static str,
    description: &'static str,
    input_schema: serde_json::Value,
) -> Tool {
    Tool {
        name,
        title: Some(title),
        description,
        input_schema,
        annotations: Some(ToolAnnotations {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: true,
        }),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}
