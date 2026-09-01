use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "telegram_send_message",
        title: Some("Telegram Send Message"),
        description: "Send one bounded plain-text message to the relay's fixed operator-configured Telegram destination. Every call must include the absolute working directory associated with the message; the relay validates it against the currently authorized workspace roots and prepends the canonical directory server-side. Telegram credentials, destination, topic, and endpoint remain relay-owned and cannot be supplied by callers.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "working_directory": { "type": "string", "minLength": 1, "maxLength": 4096 },
                "message": { "type": "string", "minLength": 1, "maxLength": 4000 }
            },
            "required": ["working_directory", "message"],
            "additionalProperties": false
        }),
        annotations: Some(ToolAnnotations {
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: true,
        }),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}
