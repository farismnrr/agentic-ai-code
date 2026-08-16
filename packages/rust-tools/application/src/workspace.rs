use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_native_path, resolve_existing_path, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub const DEFAULT_DIRECTORY_DEPTH: usize = 2;
pub const MAX_DIRECTORY_DEPTH: usize = 4;
pub const DEFAULT_DIRECTORY_ENTRIES: usize = 100;
pub const MAX_DIRECTORY_ENTRIES: usize = 100;
pub const MAX_DIRECTORY_SCAN_ENTRIES: usize = 4_096;
pub const MAX_DIRECTORY_RESULT_BYTES: usize = 256 * 1024;

#[derive(Debug, Serialize)]
pub struct DirectoryListResult {
    path: String,
    entries: Vec<DirectoryListEntry>,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct DirectoryListEntry {
    path: String,
    #[serde(rename = "type")]
    kind: &'static str,
}

pub fn directory_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<DirectoryListResult, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let requested_path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
    let depth = arguments
        .get("depth")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_DIRECTORY_DEPTH)
        .min(MAX_DIRECTORY_DEPTH);
    let max_entries = arguments
        .get("max_entries")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_DIRECTORY_ENTRIES)
        .clamp(1, MAX_DIRECTORY_ENTRIES);

    let directory =
        resolve_existing_path(&execution_root, cwd, requested_path, EntryKind::Directory)?;
    let mut state = TraversalState {
        execution_root: &execution_root,
        entries: Vec::new(),
        max_entries,
        truncated: false,
    };
    visit_directory(&directory, Path::new(""), depth, &mut state)?;

    Ok(DirectoryListResult {
        path: requested_path.to_owned(),
        entries: state.entries,
        truncated: state.truncated,
    })
}

struct TraversalState<'a> {
    execution_root: &'a Path,
    entries: Vec<DirectoryListEntry>,
    max_entries: usize,
    truncated: bool,
}

fn visit_directory(
    directory: &Path,
    relative: &Path,
    remaining_depth: usize,
    state: &mut TraversalState<'_>,
) -> Result<(), McpError> {
    if remaining_depth == 0 || state.truncated {
        return Ok(());
    }

    let mut children = Vec::new();
    for child in fs::read_dir(directory).map_err(|_| inaccessible_directory_error())? {
        if children.len() >= MAX_DIRECTORY_SCAN_ENTRIES {
            return Err(McpError::InvalidRequest(
                "directory scan exceeds maximum".into(),
            ));
        }
        children.push(child.map_err(|_| inaccessible_directory_error())?);
    }
    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        if state.entries.len() >= state.max_entries {
            state.truncated = true;
            break;
        }

        let metadata =
            fs::symlink_metadata(child.path()).map_err(|_| inaccessible_directory_error())?;
        let file_type = metadata.file_type();
        let child_relative = relative.join(child.file_name());
        let kind = if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_dir() {
            "directory"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };
        state.entries.push(DirectoryListEntry {
            path: display_relative_path(&child_relative),
            kind,
        });

        if file_type.is_dir() && remaining_depth > 1 {
            // Re-resolve immediately before recursion. Static symlinks are
            // already classified above and never followed; this second check
            // also rejects an observed root escape if the entry changed.
            let child_path = child.path();
            let resolved = resolve_existing_native_path(
                state.execution_root,
                &child_path,
                EntryKind::Directory,
            )?;
            visit_directory(&resolved, &child_relative, remaining_depth - 1, state)?;
            if state.truncated {
                break;
            }
        }
    }

    Ok(())
}

fn display_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn inaccessible_directory_error() -> McpError {
    McpError::InvalidRequest("directory is inaccessible".into())
}
