//! Workspace allowlist inspection and authorization tools.

use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde_json::{json, Value};
use std::path::Path;

pub fn workspace_add(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let path_str = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;

    let mut guard = config
        .workspaces
        .write()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;

    let canonical = guard.add(Path::new(path_str))?;
    let list = guard.list();

    Ok(json!({
        "path": canonical.to_string_lossy().into_owned(),
        "authorized": true,
        "is_primary": canonical == guard.primary_root(),
        "total_authorized_workspaces": list.len(),
    }))
}

pub fn workspace_list(_arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;

    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;

    let workspaces = guard.list();
    Ok(json!({
        "workspaces": workspaces,
        "total": workspaces.len(),
    }))
}

pub fn workspace_get(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let path_str = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;

    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;

    let target_canonical = std::fs::canonicalize(path_str)
        .map_err(|_| McpError::InvalidRequest("workspace path is inaccessible".into()))?;

    let match_entry = guard
        .list()
        .into_iter()
        .find(|entry| entry.canonical_path == target_canonical);

    match match_entry {
        Some(entry) => Ok(json!({
            "workspace": entry,
            "authorized": true,
        })),
        None => Err(McpError::InvalidRequest(
            "workspace path is not in the authorized allowlist".into(),
        )),
    }
}

pub fn workspace_remove(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let path_str = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;

    let mut guard = config
        .workspaces
        .write()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;

    let removed = guard.remove(Path::new(path_str))?;
    let remaining = guard.list();

    Ok(json!({
        "path": path_str,
        "removed": removed,
        "total_authorized_workspaces": remaining.len(),
    }))
}
