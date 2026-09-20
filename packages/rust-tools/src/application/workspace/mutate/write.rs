use super::parse_expected_sha256;
use crate::application::workspace::protected::reject_protected_path;
use crate::application::workspace::secure::SecureDirectory;
use crate::application::workspace::{
    activity_evidence, evidence::content_sha256, ActivityEvidence,
};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub const MAX_FILE_WRITE_BYTES: usize = 1024 * 1024;
pub const MAX_INTERNAL_BINARY_WRITE_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILE_WRITE_PATH_BYTES: usize = 4_096;
const MAX_FILE_WRITE_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileWriteResult {
    path: String,
    created: bool,
    overwritten: bool,
    bytes: usize,
    #[serde(rename = "_activity")]
    activity: ActivityEvidence,
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
                        "write target escapes directory boundary".into(),
                    ));
                }
            }
            Component::Normal(component) => normalized.push(component),
            Component::Prefix(_) => {
                return Err(McpError::InvalidRequest(
                    "write target uses an unsupported path prefix".into(),
                ));
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
    cwd_resolved: &Path,
    path: &str,
    create_parents: bool,
) -> Result<(SecureDirectory, std::ffi::OsString), McpError> {
    let normalized = normalize_write_path(root, cwd_resolved, path)?;
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
    let expected_sha256 = parse_expected_sha256(arguments)?;
    let _ = config.ensure_workspaces_initialized();
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd_resolved =
        crate::core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd)?;
    let root = if Path::new(path).is_absolute() {
        guard
            .containing_root(Path::new(path))
            .unwrap_or_else(|| guard.primary_root())
    } else {
        guard
            .containing_root(&cwd_resolved)
            .unwrap_or_else(|| guard.primary_root())
    };
    let normalized = normalize_write_path(root, &cwd_resolved, path)?;
    reject_protected_path(root, &normalized)?;
    let (directory, name) =
        resolve_write_parent_directory(root, &cwd_resolved, path, create_parents)?;
    let evidence_path = normalized
        .strip_prefix(root)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned());
    match directory.entry_type(&name)? {
        Some(entry) if entry.is_symlink() || entry.is_dir() || !entry.is_file() => Err(
            McpError::InvalidRequest("write target has an unsupported entry type".into()),
        ),
        Some(_) if !overwrite => Err(McpError::InvalidRequest(
            "file already exists; overwrite is required".into(),
        )),
        Some(_) => {
            let (mut file, identity, mode) = directory.open_regular_file(&name)?;
            let mut before = Vec::new();
            Read::by_ref(&mut file)
                .take((MAX_FILE_WRITE_BYTES + 1) as u64)
                .read_to_end(&mut before)
                .map_err(|_| {
                    McpError::InvalidRequest("file write target is inaccessible".into())
                })?;
            if before.len() > MAX_FILE_WRITE_BYTES {
                return Err(McpError::InvalidRequest(
                    "file write target exceeds maximum".into(),
                ));
            }
            let before_sha256 = content_sha256(&before);
            if expected_sha256.is_some_and(|expected| expected != before_sha256) {
                return Err(McpError::InvalidRequest(
                    "file write expected_sha256 does not match current file".into(),
                ));
            }
            directory.atomic_replace_regular_file(&name, identity, content.as_bytes(), mode)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: false,
                overwritten: true,
                bytes: content.len(),
                activity: activity_evidence(&evidence_path, Some(&before), content.as_bytes()),
            })
        }
        None => {
            if expected_sha256.is_some() {
                return Err(McpError::InvalidRequest(
                    "file write expected_sha256 requires an existing overwrite target".into(),
                ));
            }
            directory.atomic_create_regular_file(&name, content.as_bytes(), 0o644)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: true,
                overwritten: false,
                bytes: content.len(),
                activity: activity_evidence(&evidence_path, None, content.as_bytes()),
            })
        }
    }
}

/// Internal binary-safe sibling of `file_write` for reviewed application
/// capabilities such as media ingest. It deliberately remains crate-private:
/// arbitrary MCP callers must use an owning typed capability instead of
/// acquiring a generic large binary write primitive.
pub(crate) fn write_contained_bytes(
    path: &str,
    cwd: Option<&str>,
    bytes: &[u8],
    create_parents: bool,
    overwrite: bool,
    config: &ServerConfig,
) -> Result<(), McpError> {
    if path.is_empty() || path.len() > MAX_FILE_WRITE_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "binary write path exceeds allowed bounds".into(),
        ));
    }
    if bytes.len() > MAX_INTERNAL_BINARY_WRITE_BYTES {
        return Err(McpError::InvalidRequest(
            "binary write content exceeds maximum".into(),
        ));
    }
    if cwd.is_some_and(|value| value.len() > MAX_FILE_WRITE_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "binary write cwd exceeds maximum".into(),
        ));
    }
    config
        .ensure_workspaces_initialized()
        .map_err(|error| McpError::Internal(error.to_string()))?;
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd_resolved =
        crate::core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd)?;
    let root = if Path::new(path).is_absolute() {
        guard
            .containing_root(Path::new(path))
            .unwrap_or_else(|| guard.primary_root())
    } else {
        guard
            .containing_root(&cwd_resolved)
            .unwrap_or_else(|| guard.primary_root())
    };
    let normalized = normalize_write_path(root, &cwd_resolved, path)?;
    reject_protected_path(root, &normalized)?;
    let (directory, name) =
        resolve_write_parent_directory(root, &cwd_resolved, path, create_parents)?;
    match directory.entry_type(&name)? {
        Some(entry) if entry.is_symlink() || entry.is_dir() || !entry.is_file() => Err(
            McpError::InvalidRequest("binary write target has an unsupported entry type".into()),
        ),
        Some(_) if !overwrite => Err(McpError::InvalidRequest(
            "binary file already exists; overwrite is required".into(),
        )),
        Some(_) => {
            let (_file, identity, mode) = directory.open_regular_file(&name)?;
            directory.atomic_replace_regular_file(&name, identity, bytes, mode)?;
            Ok(())
        }
        None => {
            directory.atomic_create_regular_file(&name, bytes, 0o644)?;
            Ok(())
        }
    }
}
