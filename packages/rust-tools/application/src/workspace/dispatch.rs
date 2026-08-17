//! Workspace-owned MCP result adaptation for native workspace capabilities.

use relay_core::{config::ServerConfig, error::McpError};
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::Value;

use super::{
    directory_list, file_edit, file_read, file_search, file_write, MAX_DIRECTORY_RESULT_BYTES,
    MAX_FILE_READ_BYTES, MAX_FILE_SEARCH_RESULT_BYTES,
};

/// Dispatch a native workspace tool while keeping result serialization and
/// workspace-specific output limits owned by the workspace capability layer.
pub fn dispatch_native_tool(
    name: &str,
    arguments: &Value,
    config: &ServerConfig,
) -> Result<Option<ToolCallResult>, McpError> {
    let result = match name {
        "directory_list" => complete_json(
            &directory_list(arguments, config)?,
            "failed to serialize directory listing",
            Some((
                MAX_DIRECTORY_RESULT_BYTES,
                "directory listing exceeds output maximum",
            )),
        )?,
        "file_search" => complete_json(
            &file_search(arguments, config)?,
            "failed to serialize file search result",
            Some((
                MAX_FILE_SEARCH_RESULT_BYTES,
                "file search result exceeds output maximum",
            )),
        )?,
        "file_write" => complete_json(
            &file_write(arguments, config)?,
            "failed to serialize file write result",
            None,
        )?,
        "file_edit" => complete_json(
            &file_edit(arguments, config)?,
            "failed to serialize file edit result",
            None,
        )?,
        "file_read" => complete_json(
            &file_read(arguments, config)?,
            "failed to serialize file read result",
            Some((
                MAX_FILE_READ_BYTES + 16 * 1024,
                "file read result exceeds output maximum",
            )),
        )?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}

fn complete_json<T: Serialize>(
    result: &T,
    serialization_error: &'static str,
    output_limit: Option<(usize, &'static str)>,
) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(result)
        .map_err(|_| McpError::Internal(serialization_error.to_owned()))?;
    if let Some((max_bytes, error)) = output_limit {
        if text.len() > max_bytes {
            return Err(McpError::InvalidRequest(error.to_owned()));
        }
    }
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}
