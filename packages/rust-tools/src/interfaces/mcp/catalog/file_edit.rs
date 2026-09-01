use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "file_edit",
        title: Some("File Edit"),
        description: "Apply one or more exact anchored UTF-8 text replacements inside an existing contained regular file, then commit the complete result atomically. The legacy old_text/new_text form edits a single occurrence; edits[] batches independent anchors against the original file. By default each anchor must match exactly once; replace_all=true is explicit. Final symlinks, ambiguous/overlapping matches, stale entry identity, oversized content, and root escapes fail before commit.",
        input_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1, "maxLength": 4096 },
                "cwd": { "type": "string", "maxLength": 4096 },
                "old_text": { "type": "string", "minLength": 1, "maxLength": 262144 },
                "new_text": { "type": "string", "maxLength": 262144 },
                "replace_all": { "type": "boolean", "default": false },
                "edits": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 64,
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_text": { "type": "string", "minLength": 1, "maxLength": 262144 },
                            "new_text": { "type": "string", "maxLength": 262144 },
                            "replace_all": { "type": "boolean", "default": false }
                        },
                        "required": ["old_text", "new_text"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["path"],
            "oneOf": [
                { "required": ["old_text", "new_text"], "not": { "required": ["edits"] } },
                { "required": ["edits"], "not": { "anyOf": [
                    { "required": ["old_text"] },
                    { "required": ["new_text"] },
                    { "required": ["replace_all"] }
                ] } }
            ],
            "additionalProperties": false
        }),
        annotations: Some(ToolAnnotations {
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: false,
        }),
        security_schemes: coding_security_scheme(),
        execution: None,
    }
}
