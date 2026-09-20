//! Secure, atomic file edits and writes.

use super::protected::reject_protected_path;
use super::secure::SecureDirectory;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::EntryKind;
use serde::Serialize;
use serde_json::Value;
use std::io::Read;

use super::{activity_evidence, evidence::content_sha256, ActivityEvidence};

mod write;
pub(crate) use write::write_contained_bytes;
pub use write::{file_write, MAX_FILE_WRITE_BYTES, MAX_INTERNAL_BINARY_WRITE_BYTES};

pub const MAX_FILE_EDIT_BYTES: usize = 1024 * 1024;
const MAX_FILE_EDIT_TEXT_BYTES: usize = 256 * 1024;
const MAX_FILE_EDIT_PATH_BYTES: usize = 4_096;
const MAX_FILE_EDIT_CWD_BYTES: usize = 4_096;
const MAX_FILE_EDIT_OPERATIONS: usize = 64;

struct EditOperation {
    old_text: String,
    new_text: String,
    replace_all: bool,
}

#[derive(Debug, Serialize)]
pub struct FileEditResult {
    path: String,
    replacements: usize,
    changed: bool,
    dry_run: bool,
    before_sha256: String,
    after_sha256: String,
    #[serde(rename = "_activity")]
    activity: ActivityEvidence,
}

pub fn file_edit(arguments: &Value, config: &ServerConfig) -> Result<FileEditResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit path is required".into()))?;
    let operations = parse_edit_operations(arguments)?;
    let dry_run = arguments
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let expected_sha256 = parse_expected_sha256(arguments)?;
    if path.is_empty() || path.len() > MAX_FILE_EDIT_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit path exceeds allowed bounds".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_EDIT_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file edit cwd exceeds maximum".into(),
        ));
    }
    let _ = config.ensure_workspaces_initialized();
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let target = crate::core::workspace_path::resolve_write_target_in_allowlist(
        &guard,
        cwd,
        path,
        EntryKind::File,
    )?;
    let root = guard.containing_root(&target).ok_or_else(|| {
        McpError::InvalidRequest("file edit target is outside authorized workspace roots".into())
    })?;
    reject_protected_path(root, &target)?;
    let parent = target
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let name = target
        .file_name()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let directory = SecureDirectory::open_relative(root, parent)?;
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
    let before_sha256 = content_sha256(&bytes);
    if expected_sha256.is_some_and(|expected| expected != before_sha256) {
        return Err(McpError::InvalidRequest(
            "file edit expected_sha256 does not match current file".into(),
        ));
    }
    let before_bytes = bytes.clone();
    let source = String::from_utf8(bytes)
        .map_err(|_| McpError::InvalidRequest("file edit target is not valid UTF-8 text".into()))?;
    let (updated, replacements) = apply_edit_operations(&source, &operations)?;
    if updated.len() > MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit result exceeds maximum".into(),
        ));
    }
    let changed = updated != source;
    let after_sha256 = content_sha256(updated.as_bytes());
    if dry_run || !changed {
        directory.verify_regular_entry(name, identity)?;
    } else {
        directory.atomic_replace_regular_file(name, identity, updated.as_bytes(), mode)?;
    }
    let evidence_path = target
        .strip_prefix(root)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned());
    Ok(FileEditResult {
        path: path.to_owned(),
        replacements,
        changed,
        dry_run,
        before_sha256,
        after_sha256,
        activity: if dry_run {
            ActivityEvidence::preview(&evidence_path, Some(&before_bytes), updated.as_bytes())
        } else if changed {
            activity_evidence(&evidence_path, Some(&before_bytes), updated.as_bytes())
        } else {
            ActivityEvidence::no_change()
        },
    })
}

fn parse_expected_sha256(arguments: &Value) -> Result<Option<&str>, McpError> {
    let Some(value) = arguments.get("expected_sha256") else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| McpError::InvalidRequest("expected_sha256 must be a string".into()))?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(McpError::InvalidRequest(
            "expected_sha256 must be 64 lowercase hex characters".into(),
        ));
    }
    Ok(Some(value))
}

fn parse_edit_operations(arguments: &Value) -> Result<Vec<EditOperation>, McpError> {
    if let Some(edits) = arguments.get("edits") {
        if arguments.get("old_text").is_some()
            || arguments.get("new_text").is_some()
            || arguments.get("replace_all").is_some()
        {
            return Err(McpError::InvalidRequest(
                "file edit cannot mix edits with old_text/new_text".into(),
            ));
        }
        let edits = edits
            .as_array()
            .ok_or_else(|| McpError::InvalidRequest("file edit edits must be an array".into()))?;
        if edits.is_empty() || edits.len() > MAX_FILE_EDIT_OPERATIONS {
            return Err(McpError::InvalidRequest(
                "file edit operation count exceeds allowed bounds".into(),
            ));
        }
        edits.iter().map(parse_edit_operation).collect()
    } else {
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
        validate_edit_text(old_text, new_text)?;
        Ok(vec![EditOperation {
            old_text: old_text.to_owned(),
            new_text: new_text.to_owned(),
            replace_all,
        }])
    }
}

fn parse_edit_operation(value: &Value) -> Result<EditOperation, McpError> {
    let object = value
        .as_object()
        .ok_or_else(|| McpError::InvalidRequest("file edit operation must be an object".into()))?;
    let old_text = object
        .get("old_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit old_text is required".into()))?;
    let new_text = object
        .get("new_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit new_text is required".into()))?;
    validate_edit_text(old_text, new_text)?;
    let replace_all = object
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(EditOperation {
        old_text: old_text.to_owned(),
        new_text: new_text.to_owned(),
        replace_all,
    })
}

fn validate_edit_text(old_text: &str, new_text: &str) -> Result<(), McpError> {
    if old_text.is_empty()
        || old_text.len() > MAX_FILE_EDIT_TEXT_BYTES
        || new_text.len() > MAX_FILE_EDIT_TEXT_BYTES
    {
        return Err(McpError::InvalidRequest(
            "file edit text exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}

fn apply_edit_operations(
    source: &str,
    operations: &[EditOperation],
) -> Result<(String, usize), McpError> {
    let mut replacements = Vec::new();
    for operation in operations {
        let matches = source
            .match_indices(&operation.old_text)
            .map(|(start, _)| {
                (
                    start,
                    start + operation.old_text.len(),
                    operation.new_text.as_str(),
                )
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(McpError::InvalidRequest(
                "file edit text was not found".into(),
            ));
        }
        if !operation.replace_all && matches.len() != 1 {
            return Err(McpError::InvalidRequest(
                "file edit text is ambiguous".into(),
            ));
        }
        replacements.extend(if operation.replace_all {
            matches
        } else {
            matches.into_iter().take(1).collect()
        });
    }
    replacements.sort_by_key(|(start, end, _)| (*start, *end));
    let mut previous_end = 0;
    for (start, end, _) in &replacements {
        if *start < previous_end {
            return Err(McpError::InvalidRequest(
                "file edit operations overlap".into(),
            ));
        }
        previous_end = *end;
    }
    let mut updated = String::with_capacity(source.len());
    let mut cursor = 0;
    for (start, end, replacement) in &replacements {
        updated.push_str(&source[cursor..*start]);
        updated.push_str(replacement);
        cursor = *end;
    }
    updated.push_str(&source[cursor..]);
    Ok((updated, replacements.len()))
}
