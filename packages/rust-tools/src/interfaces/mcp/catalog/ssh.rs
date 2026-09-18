use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "ssh_readonly_exec",
        title: Some("Read-Only SSH Diagnostics"),
        description: "Run one server-validated read-only diagnostic command on an operator-configured SSH alias, optionally through a bounded relay-owned SSH jump chain. Alias/config/key resolution is relay-owned; raw SSH options, interactive access, user-selected forwarding, and remote mutation are unavailable.",
        input_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "alias": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 255,
                    "description": "Final operator-configured SSH alias where the diagnostic command executes."
                },
                "via": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 255,
                        "pattern": "^[A-Za-z0-9._-]+$"
                    },
                    "maxItems": 8,
                    "default": [],
                    "description": "Optional ordered SSH alias chain traversed before the final alias. No diagnostic command executes on intermediate hops."
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
                    "minimum": 1,
                    "maximum": 60000,
                    "default": 30000,
                    "description": "Requested synchronous remote diagnostic runtime in milliseconds. The absolute maximum is 60000 ms."
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
        execution: None,
    }
}
