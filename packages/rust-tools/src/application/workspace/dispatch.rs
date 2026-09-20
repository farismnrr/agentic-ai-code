//! Workspace-owned MCP result adaptation for native workspace capabilities.

use crate::core::{config::ServerConfig, error::McpError};
use crate::interfaces::mcp::ToolCallResult;
use serde::Serialize;
use serde_json::Value;

use super::{
    apply_patch, directory_list, file_edit, file_read, file_read_multiple, file_search, file_write,
    workspace_bootstrap, MAX_DIRECTORY_RESULT_BYTES, MAX_FILE_READ_BYTES,
    MAX_FILE_READ_MULTIPLE_BYTES, MAX_FILE_SEARCH_RESULT_BYTES,
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
        "apply_patch" => complete_json(
            &apply_patch(arguments, config)?,
            "failed to serialize patch result",
            Some((
                MAX_FILE_READ_BYTES + 16 * 1024,
                "patch result exceeds output maximum",
            )),
        )?,
        "file_read" => complete_json(
            &file_read(arguments, config)?,
            "failed to serialize file read result",
            Some((
                MAX_FILE_READ_BYTES + 16 * 1024,
                "file read result exceeds output maximum",
            )),
        )?,
        "file_read_multiple" => complete_json(
            &file_read_multiple(arguments, config)?,
            "failed to serialize multiple file read result",
            Some((
                MAX_FILE_READ_MULTIPLE_BYTES,
                "multiple file read result exceeds output maximum",
            )),
        )?,
        "workspace_bootstrap" => complete_json(
            &workspace_bootstrap(arguments, config)?,
            "failed to serialize workspace bootstrap result",
            None,
        )?,
        "workspace_add" => complete_json(
            &super::allowlist::workspace_add(arguments, config)?,
            "failed to serialize workspace add result",
            None,
        )?,
        "workspace_list" => complete_json(
            &super::allowlist::workspace_list(arguments, config)?,
            "failed to serialize workspace list result",
            None,
        )?,
        "workspace_get" => complete_json(
            &super::allowlist::workspace_get(arguments, config)?,
            "failed to serialize workspace get result",
            None,
        )?,
        "workspace_remove" => complete_json(
            &super::allowlist::workspace_remove(arguments, config)?,
            "failed to serialize workspace remove result",
            None,
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
    let structured_content = serde_json::to_value(result)
        .map_err(|_| McpError::Internal(serialization_error.to_owned()))?;
    if let Some((max_bytes, error)) = output_limit {
        let bytes = serde_json::to_vec(&structured_content)
            .map_err(|_| McpError::Internal(serialization_error.to_owned()))?;
        if bytes.len() > max_bytes {
            return Err(McpError::InvalidRequest(error.to_owned()));
        }
    }
    Ok(ToolCallResult::complete(Vec::new()).with_structured_content(structured_content))
}
