//! Secure, atomic file edits and writes.

use super::secure::SecureDirectory;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_path, resolve_write_target, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub const MAX_FILE_EDIT_BYTES: usize = 1024 * 1024;
const MAX_FILE_EDIT_TEXT_BYTES: usize = 256 * 1024;
const MAX_FILE_EDIT_PATH_BYTES: usize = 4_096;
const MAX_FILE_EDIT_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileEditResult {
    path: String,
    replacements: usize,
    changed: bool,
}

pub fn file_edit(arguments: &Value, config: &ServerConfig) -> Result<FileEditResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit path is required".into()))?;
    let old_text = arguments
        .get("old_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit old_text is required".into()))?;
    let new_text = arguments
        .get("new_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit new_text is required".into()))?;
    let replace_all = arguments
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if path.is_empty() || path.len() > MAX_FILE_EDIT_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit path exceeds allowed bounds".into(),
        ));
    }
    if old_text.is_empty()
        || old_text.len() > MAX_FILE_EDIT_TEXT_BYTES
        || new_text.len() > MAX_FILE_EDIT_TEXT_BYTES
    {
        return Err(McpError::InvalidRequest(
            "file edit text exceeds allowed bounds".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_EDIT_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file edit cwd exceeds maximum".into(),
        ));
    }
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let target = resolve_write_target(&root, cwd, path, EntryKind::File)?;
    let parent = target
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let name = target
        .file_name()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let directory = SecureDirectory::open_relative(&root, parent)?;
    let (mut file, identity, mode) = directory.open_regular_file(name)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_FILE_EDIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| McpError::InvalidRequest("file edit target is inaccessible".into()))?;
    if bytes.len() > MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit target exceeds maximum".into(),
        ));
    }
    let source = String::from_utf8(bytes)
        .map_err(|_| McpError::InvalidRequest("file edit target is not valid UTF-8 text".into()))?;
    let matches = source.match_indices(old_text).count();
    if matches == 0 {
        return Err(McpError::InvalidRequest(
            "file edit text was not found".into(),
        ));
    }
    if !replace_all && matches != 1 {
        return Err(McpError::InvalidRequest(
            "file edit text is ambiguous".into(),
        ));
    }
    let updated = if replace_all {
        source.replace(old_text, new_text)
    } else {
        source.replacen(old_text, new_text, 1)
    };
    if updated.len() > MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit result exceeds maximum".into(),
        ));
    }
    let changed = updated != source;
    if changed {
        directory.atomic_replace_regular_file(name, identity, updated.as_bytes(), mode)?;
    } else {
        directory.verify_regular_entry(name, identity)?;
    }
    Ok(FileEditResult {
        path: path.to_owned(),
        replacements: if replace_all { matches } else { 1 },
        changed,
    })
}

pub const MAX_FILE_WRITE_BYTES: usize = 1024 * 1024;
const MAX_FILE_WRITE_PATH_BYTES: usize = 4_096;
const MAX_FILE_WRITE_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileWriteResult {
    path: String,
    created: bool,
    overwritten: bool,
    bytes: usize,
}

fn normalize_write_path(
    root: &Path,
    cwd: &Path,
    value: &str,
) -> Result<std::path::PathBuf, McpError> {
    use std::path::Component;
    let requested = if Path::new(value).is_absolute() {
        Path::new(value).to_path_buf()
    } else {
        cwd.join(value)
    };
    let mut normalized = std::path::PathBuf::new();
    for component in requested.components() {
        match component {
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(McpError::InvalidRequest(
                        "write target escapes execution root".into(),
                    ));
                }
            }
            Component::Normal(value) => normalized.push(value),
            Component::Prefix(_) => {
                return Err(McpError::InvalidRequest("write target is invalid".into()))
            }
        }
    }
    if !normalized.starts_with(root) || normalized == root {
        return Err(McpError::InvalidRequest(
            "write target escapes execution root".into(),
        ));
    }
    Ok(normalized)
}

fn resolve_write_parent_directory(
    root: &Path,
    cwd: Option<&str>,
    path: &str,
    create_parents: bool,
) -> Result<(SecureDirectory, std::ffi::OsString), McpError> {
    let cwd = resolve_existing_path(root, cwd, ".", EntryKind::Directory)?;
    let normalized = normalize_write_path(root, &cwd, path)?;
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| McpError::InvalidRequest("write target escapes execution root".into()))?;
    let name = relative
        .file_name()
        .ok_or_else(|| McpError::InvalidRequest("write target is invalid".into()))?
        .to_os_string();
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut directory = SecureDirectory::open_relative(root, root)?;
    for component in parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(McpError::InvalidRequest(
                "write target parent is invalid".into(),
            ));
        };
        directory = directory.open_or_create_child(component, create_parents)?;
    }
    Ok((directory, name))
}

pub fn file_write(arguments: &Value, config: &ServerConfig) -> Result<FileWriteResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file write path is required".into()))?;
    let content = arguments
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file write content is required".into()))?;
    if path.is_empty() || path.len() > MAX_FILE_WRITE_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file write path exceeds allowed bounds".into(),
        ));
    }
    if content.len() > MAX_FILE_WRITE_BYTES {
        return Err(McpError::InvalidRequest(
            "file write content exceeds maximum".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_WRITE_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file write cwd exceeds maximum".into(),
        ));
    }
    let create_parents = arguments
        .get("create_parents")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let overwrite = arguments
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let (directory, name) = resolve_write_parent_directory(&root, cwd, path, create_parents)?;
    match directory.entry_type(&name)? {
        Some(entry) if entry.is_symlink() || entry.is_dir() || !entry.is_file() => Err(
            McpError::InvalidRequest("write target has an unsupported entry type".into()),
        ),
        Some(_) if !overwrite => Err(McpError::InvalidRequest(
            "file already exists; overwrite is required".into(),
        )),
        Some(_) => {
            let (_file, identity, mode) = directory.open_regular_file(&name)?;
            directory.atomic_replace_regular_file(&name, identity, content.as_bytes(), mode)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: false,
                overwritten: true,
                bytes: content.len(),
            })
        }
        None => {
            directory.atomic_create_regular_file(&name, content.as_bytes(), 0o644)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: true,
                overwritten: false,
                bytes: content.len(),
            })
        }
    }
}
