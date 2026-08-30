use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "ssh_readonly_exec",
        title: Some("Read-Only SSH Diagnostics"),
        description: "Run one server-validated read-only diagnostic command on an operator-configured SSH alias. Alias/config/key resolution is relay-owned; raw SSH options, interactive access, forwarding, and remote mutation are unavailable.",
        input_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "alias": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 255,
                    "description": "Operator-configured SSH alias resolved by the relay."
                },
                "command": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 255,
                    "pattern": "^[^\\s]+$",
                    "description": "One reviewed remote diagnostic executable/family such as docker, git, curl, or uptime."
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string", "maxLength": 65536 },
                    "maxItems": 100,
                    "default": []
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 60000,
                    "default": 30000,
                    "description": "Requested remote diagnostic runtime in milliseconds; 0 uses the bounded relay SSH default."
                },
                "execution_mode": {
                    "type": "string",
                    "enum": ["sync", "async", "auto"],
                    "default": "auto",
                    "description": "Use sync for immediate results, async for task-backed execution, or auto to use MCP Tasks when negotiated."
                }
            },
            "required": ["alias", "command"],
            "additionalProperties": false
        }),
        annotations: Some(ToolAnnotations {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: true,
        }),
        security_schemes: coding_security_scheme(),
        execution: Some(json!({ "taskSupport": "optional" })),
    }
}
