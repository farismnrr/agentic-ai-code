use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    tool_for_providers(&["external-mcp", "agy", "external-mcp"])
}

pub(super) fn tool_for_providers(providers: &[&str]) -> Tool {
    let providers = providers
        .iter()
        .map(|provider| json!(provider))
        .collect::<Vec<_>>();
    Tool {
        name: "agent_delegate",
        title: Some("Delegate Coding Agent"),
        description: "Delegate a bounded coding prompt to an operator-configured coding CLI in the authorized workspace. Providers run serially; automatic fallback is limited to quota, authentication, or availability failures and stops if the workspace may have changed.",
        input_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "minLength": 1, "maxLength": 65536 },
                "providers": {
                    "type": "array",
                    "items": { "type": "string", "enum": providers },
                    "minItems": 1,
                    "maxItems": 3,
                    "default": providers
                },
                "cwd": { "type": "string", "maxLength": 4096 },
                "timeout_ms": { "type": "integer", "minimum": 0, "maximum": 600000, "default": 30000 },
                "max_turns": { "type": "integer", "minimum": 1, "maximum": 50, "default": 20 },
                "fallback": { "type": "boolean", "default": true }
            },
            "required": ["prompt"],
            "additionalProperties": false
        }),
        annotations: Some(ToolAnnotations {
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: true,
        }),
        security_schemes: coding_security_scheme(),
        execution: Some(json!({ "taskSupport": "optional" })),
    }
}
