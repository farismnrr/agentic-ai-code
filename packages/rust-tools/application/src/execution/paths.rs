use crate::workspace::reject_protected_target;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn resolve_authorized_cwd(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<PathBuf, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let _ = config.ensure_workspaces_initialized();
    let cwd = match arguments.get("cwd").and_then(Value::as_str) {
        Some(value) => {
            if let Ok(guard) = config.workspaces.read() {
                relay_core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, Some(value))?
            } else {
                relay_core::terminal_policy::resolve_contained_cwd(&execution_root, Some(value))?
            }
        }
        None => std::fs::canonicalize(
            config
                .resolved_dir()
                .map_err(|_| McpError::Internal("failed to resolve workspace directory".into()))?,
        )
        .map_err(|_| McpError::InvalidRequest("workspace directory is inaccessible".into()))?,
    };
    if !config.is_path_contained(&cwd) {
        return Err(McpError::InvalidRequest(
            "working directory is outside authorized workspace roots".into(),
        ));
    }
    reject_protected_target(&execution_root, &cwd)?;
    Ok(cwd)
}
