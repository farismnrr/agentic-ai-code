use super::{coding_security_scheme, Tool, ToolAnnotations};
use serde_json::json;

pub(super) fn tool() -> Tool {
    Tool {
        name: "file_edit",
        title: Some("File Edit"),
        description: "Apply one or more exact anchored UTF-8 text replacements inside an existing contained regular file, then commit the complete result atomically. Supply either legacy old_text/new_text or edits[]; runtime validation rejects mixed/incomplete forms. By default each anchor must match exactly once; replace_all=true is explicit. dry_run validates and returns hashes without mutation. expected_sha256 provides optimistic concurrency against the complete current file. Final symlinks, ambiguous/overlapping matches, stale entry identity, oversized content, and root escapes fail before commit.",
        input_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1, "maxLength": 4096, "description": "Contained UTF-8 file path, relative to cwd or an authorized absolute workspace path." },
                "cwd": { "type": "string", "maxLength": 4096, "description": "Optional authorized workspace directory used to resolve relative paths." },
                "old_text": { "type": "string", "minLength": 1, "maxLength": 262144, "description": "Exact text anchor for the legacy single-edit form." },
                "new_text": { "type": "string", "maxLength": 262144, "description": "Replacement text for the legacy single-edit form." },
                "replace_all": { "type": "boolean", "default": false, "description": "Replace every exact match instead of requiring exactly one match." },
                "dry_run": { "type": "boolean", "default": false, "description": "Validate and return before/after hashes without mutating the file." },
                "expected_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$", "description": "Optional lowercase SHA-256 of the complete current file; stale values reject the edit." },
                "edits": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 64,
                    "description": "One to sixty-four independent exact replacements evaluated against the original file.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_text": { "type": "string", "minLength": 1, "maxLength": 262144, "description": "Exact text anchor evaluated against the original file." },
                            "new_text": { "type": "string", "maxLength": 262144, "description": "Replacement text for this anchor." },
                            "replace_all": { "type": "boolean", "default": false, "description": "Replace every exact match for this anchor." }
                        },
                        "required": ["old_text", "new_text"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["path"],
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
