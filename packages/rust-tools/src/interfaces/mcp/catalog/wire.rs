use super::Tool;
use crate::core::error::McpError;
use serde_json::{json, Value};

pub fn output_schema_for_tool(name: &str) -> Option<Value> {
    let schema = match name {
        "directory_list" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "entries": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "type": { "type": "string", "enum": ["file", "directory", "symlink", "other"] }
                        },
                        "required": ["path", "type"],
                        "additionalProperties": false
                    }
                },
                "truncated": { "type": "boolean" },
                "continuation": { "type": "string" }
            },
            "required": ["path", "entries", "truncated"],
            "additionalProperties": false
        }),
        "file_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "matches": { "type": "array", "items": { "type": "string" } },
                "count": { "type": "integer", "minimum": 0 },
                "truncated": { "type": "boolean" },
                "continuation": { "type": "string" }
            },
            "required": ["pattern", "matches", "count", "truncated"],
            "additionalProperties": false
        }),
        "file_read" => file_read_output_schema(),
        "file_read_multiple" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "ok": { "type": "boolean" },
                            "result": file_read_output_schema(),
                            "error": { "type": "string" }
                        },
                        "required": ["path", "ok"],
                        "additionalProperties": false
                    }
                },
                "count": { "type": "integer", "minimum": 0 },
                "failed": { "type": "integer", "minimum": 0 }
            },
            "required": ["files", "count", "failed"],
            "additionalProperties": false
        }),
        "file_write" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "created": { "type": "boolean" },
                "overwritten": { "type": "boolean" },
                "bytes": { "type": "integer", "minimum": 0 }
            },
            "required": ["path", "created", "overwritten", "bytes"],
            "additionalProperties": false
        }),
        "file_edit" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "replacements": { "type": "integer", "minimum": 0 },
                "changed": { "type": "boolean" },
                "dry_run": { "type": "boolean" },
                "before_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                "after_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" }
            },
            "required": [
                "path", "replacements", "changed", "dry_run",
                "before_sha256", "after_sha256"
            ],
            "additionalProperties": false
        }),
        "apply_patch" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "dry_run": { "type": "boolean" },
                "changed_paths": { "type": "array", "items": { "type": "string" } },
                "hunks": { "type": "integer", "minimum": 0 },
                "files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "before_hash": { "type": "string" },
                            "after_hash": { "type": "string" },
                            "bytes": { "type": "integer", "minimum": 0 }
                        },
                        "required": ["path", "before_hash", "after_hash", "bytes"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["dry_run", "changed_paths", "hunks", "files"],
            "additionalProperties": false
        }),
        "text_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "matches": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "line": { "type": "integer", "minimum": 1 },
                            "column": { "type": "integer", "minimum": 1 },
                            "preview": { "type": "string" }
                        },
                        "required": ["path", "line", "column", "preview"],
                        "additionalProperties": false
                    }
                },
                "count": { "type": "integer", "minimum": 0 },
                "truncated": { "type": "boolean" },
                "continuation": { "type": "string" }
            },
            "required": ["matches", "count", "truncated"],
            "additionalProperties": false
        }),
        _ => return None,
    };
    Some(schema)
}

fn file_read_output_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "start_line": { "type": "integer", "minimum": 1 },
            "end_line": { "type": ["integer", "null"], "minimum": 1 },
            "content": { "type": "string" },
            "truncated": { "type": "boolean" },
            "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
            "next_offset_line": { "type": "integer", "minimum": 1 }
        },
        "required": ["path", "start_line", "end_line", "content", "truncated"],
        "additionalProperties": false
    })
}

pub fn tool_for_wire(tool: &Tool) -> Value {
    let mut value = serde_json::to_value(tool).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "description".into(),
            Value::String(format!(
                "{} {}",
                tool.description,
                super::super::TOOL_DESCRIPTION_REPORTING_SUFFIX
            )),
        );
        if let Some(output_schema) = output_schema_for_tool(tool.name) {
            object.insert("outputSchema".into(), output_schema);
        }
    }
    value
}

pub fn validate_tool_output(tool: &Tool, output: &Value) -> Result<(), McpError> {
    let Some(schema) = output_schema_for_tool(tool.name) else {
        return Ok(());
    };
    let validator = jsonschema::validator_for(&schema)
        .map_err(|_| McpError::Internal("invalid tool output schema".to_string()))?;
    if validator.iter_errors(output).next().is_some() {
        return Err(McpError::Internal(
            "tool output does not match the declared schema".to_string(),
        ));
    }
    Ok(())
}

/// Validate `arguments` against the declared JSON Schema before execution.
/// Diagnostics are deliberately not returned because schema errors can echo
/// attacker-controlled request values or property names through `Display`.
pub fn validate_tool_arguments(tool: &Tool, arguments: &Value) -> Result<(), McpError> {
    let validator = jsonschema::validator_for(&tool.input_schema)
        .map_err(|_| McpError::Internal("invalid tool schema".to_string()))?;

    if validator.iter_errors(arguments).next().is_some() {
        return Err(McpError::InvalidParams(
            "tool arguments do not match the required schema".to_string(),
        ));
    }

    Ok(())
}
