//! Bounded directory listing over the shared secure traversal foundation.

use super::protected::{is_protected_discovered_path, reject_protected_path};
use super::secure::SecureDirectory;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use serde::Serialize;
use serde_json::Value;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
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
    reject_protected_path(&execution_root, &directory)?;
    let scope = directory.to_string_lossy().into_owned();
    let directory = SecureDirectory::open_relative(&execution_root, &directory)?;
    let mut state = TraversalState {
        entries: Vec::new(),
        max_entries: MAX_DIRECTORY_SCAN_ENTRIES.min(crate::continuation::MAX_TOTAL_ENTRIES),
        truncated: false,
    };
    visit_directory(&directory, Path::new(""), depth, &mut state)?;

    let (entries, continuation) = crate::continuation::paginate(
        arguments,
        state.entries,
        max_entries,
        config,
        "directory_list",
        &scope,
        None,
    )?;
    Ok(DirectoryListResult {
        path: requested_path.to_owned(),
        entries,
        truncated: continuation.is_some() || state.truncated,
        continuation,
    })
}

struct TraversalState {
    entries: Vec<DirectoryListEntry>,
    max_entries: usize,
    truncated: bool,
}

fn visit_directory(
    directory: &SecureDirectory,
    relative: &Path,
    remaining_depth: usize,
    state: &mut TraversalState,
) -> Result<(), McpError> {
    if remaining_depth == 0 || state.truncated {
        return Ok(());
    }

    let children =
        directory.read_entries(MAX_DIRECTORY_SCAN_ENTRIES, "directory scan exceeds maximum")?;

    for child in children {
        if state.entries.len() >= state.max_entries {
            state.truncated = true;
            break;
        }

        // MCP paths are UTF-8 strings. Native entries that cannot be represented
        // exactly are omitted, including any descendants below such a directory;
        // never collapse distinct native names through lossy conversion.
        if child.name.to_str().is_none() {
            continue;
        }
        let child_relative = relative.join(&child.name);
        let child_path = directory.path_for_child(&child.name);
        if is_protected_discovered_path(directory.root(), &child_path) {
            continue;
        }
        let file_type = child.file_type;
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
            let child_directory = directory.open_child(&child)?;
            visit_directory(
                &child_directory,
                &child_relative,
                remaining_depth - 1,
                state,
            )?;
            if state.truncated {
                break;
            }
        }
    }

    Ok(())
}

fn display_relative_path(path: &Path) -> String {
    path.to_str()
        .expect("directory traversal filters non-UTF-8 components")
        .replace(std::path::MAIN_SEPARATOR, "/")
}
