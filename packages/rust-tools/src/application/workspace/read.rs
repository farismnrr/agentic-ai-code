//! Bounded UTF-8 file reads.

use super::{evidence::content_sha256, protected::reject_protected_path};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::EntryKind;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
pub const DEFAULT_FILE_READ_LINES: usize = 200;
pub const MAX_FILE_READ_LINES: usize = 1_000;
pub const MAX_FILE_READ_BYTES: usize = 256 * 1024;
pub const MAX_FILE_READ_MULTIPLE_BYTES: usize = 512 * 1024;
const MAX_FILE_READ_MULTIPLE_PATHS: usize = 16;
const MAX_FILE_READ_LINE_BYTES: usize = 64 * 1024;
const MAX_FILE_READ_PATH_BYTES: usize = 4_096;
const MAX_FILE_READ_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileReadResult {
    path: String,
    start_line: u64,
    end_line: Option<u64>,
    content: String,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_offset_line: Option<u64>,
}

fn complete_file_sha256(path: &std::path::Path) -> Result<Option<String>, McpError> {
    let file = std::fs::File::open(path)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    let mut bytes = Vec::new();
    std::io::Read::take(file, (super::mutate::MAX_FILE_EDIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    if bytes.len() > super::mutate::MAX_FILE_EDIT_BYTES {
        return Ok(None);
    }
    Ok(Some(content_sha256(&bytes)))
}

fn read_bounded_line_sync<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, McpError> {
    let mut line = Vec::new();
    loop {
        let buffer = reader
            .fill_buf()
            .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let take = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(buffer.len(), |index| index + 1);
        if line.len().saturating_add(take) > max_bytes {
            return Err(McpError::InvalidRequest("file line exceeds maximum".into()));
        }
        line.extend_from_slice(&buffer[..take]);
        reader.consume(take);
        if line.last() == Some(&b'\n') {
            return Ok(Some(line));
        }
    }
}

pub(crate) fn read_contained_text(
    path: &str,
    cwd: Option<&str>,
    config: &ServerConfig,
    max_bytes: usize,
) -> Result<String, McpError> {
    if max_bytes == 0 || max_bytes > super::mutate::MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "contained text read bound is invalid".into(),
        ));
    }
    let _ = config.ensure_workspaces_initialized();
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let target = crate::core::workspace_path::resolve_existing_path_in_allowlist(
        &guard,
        cwd,
        path,
        EntryKind::File,
    )?;
    let root = guard.containing_root(&target).ok_or_else(|| {
        McpError::InvalidRequest("file is outside authorized workspace roots".into())
    })?;
    reject_protected_path(root, &target)?;
    let file = std::fs::File::open(&target)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    let mut bytes = Vec::new();
    std::io::Read::take(file, (max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    if bytes.len() > max_bytes {
        return Err(McpError::InvalidRequest(
            "contained text file exceeds maximum".into(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| McpError::InvalidRequest("file is not valid UTF-8 text".into()))
}

#[derive(Debug, Serialize)]
pub struct FileReadMultipleResult {
    files: Vec<FileReadMultipleItem>,
    count: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct FileReadMultipleItem {
    path: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<FileReadResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn file_read_multiple(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<FileReadMultipleResult, McpError> {
    let paths = arguments
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("file read multiple paths are required".into()))?;
    if paths.is_empty() || paths.len() > MAX_FILE_READ_MULTIPLE_PATHS {
        return Err(McpError::InvalidRequest(
            "file read multiple path count exceeds allowed bounds".into(),
        ));
    }
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let path = path.as_str().ok_or_else(|| {
            McpError::InvalidRequest("file read multiple paths must be strings".into())
        })?;
        let mut request = serde_json::Map::new();
        request.insert("path".into(), Value::String(path.to_owned()));
        for key in ["cwd", "offset_line", "limit_lines"] {
            if let Some(value) = arguments.get(key) {
                request.insert(key.into(), value.clone());
            }
        }
        match file_read(&Value::Object(request), config) {
            Ok(result) => files.push(FileReadMultipleItem {
                path: path.to_owned(),
                ok: true,
                result: Some(result),
                error: None,
            }),
            Err(error) => files.push(FileReadMultipleItem {
                path: path.to_owned(),
                ok: false,
                result: None,
                error: Some(error.to_string()),
            }),
        }
    }
    Ok(FileReadMultipleResult {
        count: files.len(),
        failed: files.iter().filter(|item| !item.ok).count(),
        files,
    })
}

pub fn file_read(arguments: &Value, config: &ServerConfig) -> Result<FileReadResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file read path is required".into()))?;
    if path.is_empty() || path.len() > MAX_FILE_READ_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file read path exceeds allowed bounds".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_READ_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file read cwd exceeds maximum".into(),
        ));
    }
    let offset_line = arguments
        .get("offset_line")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if offset_line == 0 {
        return Err(McpError::InvalidRequest(
            "offset_line must be at least 1".into(),
        ));
    }
    let limit_lines = arguments
        .get("limit_lines")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_FILE_READ_LINES)
        .clamp(1, MAX_FILE_READ_LINES);
    let _ = config.ensure_workspaces_initialized();
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let target = crate::core::workspace_path::resolve_existing_path_in_allowlist(
        &guard,
        cwd,
        path,
        EntryKind::File,
    )?;
    let root = guard.containing_root(&target).ok_or_else(|| {
        McpError::InvalidRequest("file is outside authorized workspace roots".into())
    })?;
    reject_protected_path(root, &target)?;
    let sha256 = complete_file_sha256(&target)?;
    let file = std::fs::File::open(&target)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    let mut reader = BufReader::new(file);
    let mut current = 1u64;
    while current < offset_line {
        if read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)?.is_none() {
            return Ok(FileReadResult {
                path: path.to_owned(),
                start_line: offset_line,
                end_line: None,
                content: String::new(),
                truncated: false,
                sha256,
                next_offset_line: None,
            });
        }
        current += 1;
    }
    let mut content = Vec::new();
    let mut lines_read = 0usize;
    let mut end_line = None;
    let mut truncated = false;
    while lines_read < limit_lines {
        let Some(line) = read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)? else {
            break;
        };
        if std::str::from_utf8(&line).is_err() {
            return Err(McpError::InvalidRequest(
                "file is not valid UTF-8 text".into(),
            ));
        }
        if content.len().saturating_add(line.len()) > MAX_FILE_READ_BYTES {
            truncated = true;
            break;
        }
        content.extend_from_slice(&line);
        end_line = Some(offset_line + lines_read as u64);
        lines_read += 1;
    }
    if !truncated && lines_read == limit_lines {
        truncated = read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)?.is_some();
    }
    let content = String::from_utf8(content)
        .map_err(|_| McpError::InvalidRequest("file is not valid UTF-8 text".into()))?;
    Ok(FileReadResult {
        path: path.to_owned(),
        start_line: offset_line,
        end_line,
        content,
        truncated,
        sha256,
        next_offset_line: if truncated {
            end_line.map(|line| line.saturating_add(1))
        } else {
            None
        },
    })
}
