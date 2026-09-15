use crate::core::error::McpError;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub(super) fn required_str<'a>(arguments: &'a Value, field: &str) -> Result<&'a str, McpError> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))
}

pub(super) fn optional_string(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(super) fn parse_required<T: DeserializeOwned>(
    arguments: &Value,
    field: &str,
) -> Result<T, McpError> {
    let value = arguments
        .get(field)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    serde_json::from_value(value)
        .map_err(|_| McpError::InvalidRequest(format!("creative {field} is invalid")))
}

pub(super) fn parse_optional<T: DeserializeOwned>(
    arguments: &Value,
    field: &str,
) -> Result<Option<T>, McpError> {
    arguments
        .get(field)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| McpError::InvalidRequest(format!("creative {field} is invalid")))
}

pub(super) fn complete(value: Value) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("creative result could not be serialized".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}

pub(super) fn error_result(
    code: &str,
    message: &str,
    detail: Value,
) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&json!({
        "code": code,
        "message": message,
        "detail": detail
    }))
    .map_err(|_| McpError::Internal("creative error could not be serialized".into()))?;
    Ok(ToolCallResult::error(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}
