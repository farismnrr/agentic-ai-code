//! Shared semantic normalization logic reused by every language-specific
//! adapter (Rust, TypeScript, Vue). This module intentionally speaks only
//! the small, stable LSP subset the public `code_*` tool surface needs: it
//! does not parse source, maintain a second index, or vary behavior by
//! language beyond the generic LSP capability negotiation already captured
//! on the session.

use super::{LspError, LspSession};
use crate::application::workspace::reject_protected_target;
use crate::core::workspace_path::{resolve_existing_path, EntryKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

pub const MAX_TEXT: usize = 16 * 1024;
pub const MAX_RESULTS: usize = 128;
pub const MAX_RENAME_FILES: usize = 32;
pub const MAX_RENAME_EDITS: usize = 256;
pub const MAX_RENAME_TEXT: usize = 4 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: u32,
    pub range: Range,
    pub selection_range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hover {
    pub text: String,
    pub range: Option<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub range: Range,
    pub severity: Option<u32>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub version: Option<i64>,
}

/// One text replacement inside a single file, as part of a bounded,
/// non-mutating rename preview.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenameEdit {
    pub range: Range,
    pub new_text: String,
}

/// A bounded, normalized, non-mutating rename preview for one file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenameFilePreview {
    pub path: PathBuf,
    pub edits: Vec<RenameEdit>,
}

pub async fn symbols(session: &LspSession, path: &str) -> Result<Vec<Symbol>, LspError> {
    require(session.capabilities().document_symbols)?;
    let uri = sync(session, path).await?;
    let value = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument":{"uri":uri}}),
        )
        .await?;
    let mut result = Vec::new();
    for item in value.as_array().ok_or(LspError::MalformedResponse)?.iter() {
        if item.get("name").and_then(Value::as_str).is_none() {
            continue;
        }
        let range = item
            .get("range")
            .or_else(|| {
                item.get("location")
                    .and_then(|location| location.get("range"))
            })
            .ok_or(LspError::MalformedResponse)?;
        result.push(Symbol {
            name: bounded_text(item.get("name").and_then(Value::as_str).unwrap_or("")),
            kind: item.get("kind").and_then(Value::as_u64).unwrap_or(0) as u32,
            range: parse_range(range)?,
            selection_range: parse_range(
                item.get("selectionRange")
                    .or_else(|| {
                        item.get("location")
                            .and_then(|location| location.get("range"))
                    })
                    .unwrap_or(range),
            )?,
        });
        if result.len() == MAX_RESULTS {
            break;
        }
    }
    Ok(result)
}

pub async fn workspace_symbols(session: &LspSession, query: &str) -> Result<Vec<Symbol>, LspError> {
    require(session.capabilities().workspace_symbols)?;
    let value = session
        .request("workspace/symbol", json!({"query": bounded_query(query)}))
        .await?;
    let mut result = Vec::new();
    for item in value.as_array().ok_or(LspError::MalformedResponse)?.iter() {
        let location = item.get("location").ok_or(LspError::MalformedResponse)?;
        let range = location.get("range").ok_or(LspError::MalformedResponse)?;
        result.push(Symbol {
            name: bounded_text(item.get("name").and_then(Value::as_str).unwrap_or("")),
            kind: item.get("kind").and_then(Value::as_u64).unwrap_or(0) as u32,
            range: parse_range(range)?,
            selection_range: parse_range(range)?,
        });
        if result.len() == MAX_RESULTS {
            break;
        }
    }
    Ok(result)
}

pub async fn definition(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
) -> Result<Vec<Location>, LspError> {
    require(session.capabilities().definition)?;
    let mut locations = request_locations(
        session,
        "textDocument/definition",
        path,
        line,
        utf8_column,
        Value::Null,
    )
    .await?;
    locations.truncate(MAX_RESULTS);
    Ok(locations)
}

pub async fn references(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
    include_declaration: bool,
) -> Result<Vec<Location>, LspError> {
    require(session.capabilities().references)?;
    let mut locations = request_locations(
        session,
        "textDocument/references",
        path,
        line,
        utf8_column,
        json!({"context":{"includeDeclaration":include_declaration}}),
    )
    .await?;
    locations.truncate(MAX_RESULTS);
    Ok(locations)
}

pub async fn implementations(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
) -> Result<Vec<Location>, LspError> {
    require(session.capabilities().implementation)?;
    let mut locations = request_locations(
        session,
        "textDocument/implementation",
        path,
        line,
        utf8_column,
        Value::Null,
    )
    .await?;
    locations.truncate(MAX_RESULTS);
    Ok(locations)
}

pub async fn hover(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
) -> Result<Option<Hover>, LspError> {
    require(session.capabilities().hover)?;
    let uri = sync(session, path).await?;
    let position = position(session, path, line, utf8_column)?;
    let value = session
        .request(
            "textDocument/hover",
            json!({"textDocument":{"uri":uri},"position":position}),
        )
        .await?;
    if value.is_null() {
        return Ok(None);
    }
    let contents = value.get("contents").ok_or(LspError::MalformedResponse)?;
    let text = match contents {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("value").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => contents
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        _ => return Err(LspError::MalformedResponse),
    };
    Ok(Some(Hover {
        text: bounded_text(&text),
        range: value.get("range").map(parse_range).transpose()?,
    }))
}

pub async fn diagnostics(session: &LspSession, path: &str) -> Result<Vec<Diagnostic>, LspError> {
    let uri = sync(session, path).await?;
    if session.capabilities().diagnostic_pull {
        let response = session
            .request(
                "textDocument/diagnostic",
                json!({"textDocument":{"uri":uri}}),
            )
            .await?;
        return normalize_diagnostics(
            session,
            path,
            response.get("items").unwrap_or(&response),
            None,
        );
    }
    for _ in 0..40 {
        if let Some(value) = session.latest_diagnostics(&uri).await {
            let version = value.get("version").and_then(Value::as_i64);
            return normalize_diagnostics(
                session,
                path,
                value
                    .get("diagnostics")
                    .unwrap_or(&Value::Array(Vec::new())),
                version,
            );
        }
        sleep(Duration::from_millis(50)).await;
    }
    Ok(Vec::new())
}

/// Fresh diagnostics for a document that was just synchronized, requiring a
/// diagnostic version/publish observed *after* the given document version —
/// used to prove diagnostics are not stale after a native mutation (Plan
/// 039C PHASE-07). Falls back to a single fresh read for pull-model servers,
/// which are inherently synchronous with the current document.
pub async fn diagnostics_after_version(
    session: &LspSession,
    path: &str,
    min_version: u64,
) -> Result<Vec<Diagnostic>, LspError> {
    let uri = sync(session, path).await?;
    if session.capabilities().diagnostic_pull {
        let response = session
            .request(
                "textDocument/diagnostic",
                json!({"textDocument":{"uri":uri}}),
            )
            .await?;
        return normalize_diagnostics(
            session,
            path,
            response.get("items").unwrap_or(&response),
            None,
        );
    }
    for _ in 0..100 {
        if let Some(value) = session.latest_diagnostics(&uri).await {
            let version = value.get("version").and_then(Value::as_i64);
            if version.map(|v| v as u64 >= min_version).unwrap_or(false) {
                return normalize_diagnostics(
                    session,
                    path,
                    value
                        .get("diagnostics")
                        .unwrap_or(&Value::Array(Vec::new())),
                    version,
                );
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(LspError::StaleDocument)
}

pub use super::rename::rename_preview;

async fn request_locations(
    session: &LspSession,
    method: &str,
    path: &str,
    line: u32,
    utf8_column: usize,
    extra: Value,
) -> Result<Vec<Location>, LspError> {
    let uri = sync(session, path).await?;
    let position = position(session, path, line, utf8_column)?;
    let mut params = json!({"textDocument":{"uri":uri},"position":position});
    if let Value::Object(extra) = extra {
        params.as_object_mut().unwrap().extend(extra);
    }
    let value = session.request(method, params).await?;
    let values = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        Value::Object(_) => vec![value],
        _ => return Err(LspError::MalformedResponse),
    };
    values
        .iter()
        .map(|value| normalize_location(session, value))
        .collect()
}

pub async fn sync(session: &LspSession, path: &str) -> Result<String, LspError> {
    let target = safe_path(session, path)?;
    session.sync_document(path).await?;
    url::Url::from_file_path(target)
        .map(|url| url.to_string())
        .map_err(|_| LspError::ContainedLocationRejected)
}

pub fn safe_path(session: &LspSession, path: &str) -> Result<PathBuf, LspError> {
    let target = resolve_existing_path(&session.identity().root, None, path, EntryKind::File)
        .map_err(|_| LspError::ContainedLocationRejected)?;
    reject_protected_target(&session.identity().root, &target)
        .map_err(|_| LspError::ContainedLocationRejected)?;
    Ok(target)
}

pub fn position(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
) -> Result<Value, LspError> {
    let target = safe_path(session, path)?;
    let content =
        std::fs::read_to_string(target).map_err(|_| LspError::ContainedLocationRejected)?;
    let source_line = content
        .lines()
        .nth(line as usize)
        .ok_or(LspError::MalformedResponse)?;
    if utf8_column > source_line.len() || !source_line.is_char_boundary(utf8_column) {
        return Err(LspError::MalformedResponse);
    }
    let character = source_line[..utf8_column].encode_utf16().count() as u32;
    Ok(json!({"line":line,"character":character}))
}

fn normalize_location(session: &LspSession, value: &Value) -> Result<Location, LspError> {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))
        .and_then(Value::as_str)
        .ok_or(LspError::MalformedResponse)?;
    let path = url::Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .ok_or(LspError::MalformedResponse)?;
    let path = resolve_existing_path(
        &session.identity().root,
        None,
        &path.to_string_lossy(),
        EntryKind::File,
    )
    .map_err(|_| LspError::ContainedLocationRejected)?;
    reject_protected_target(&session.identity().root, &path)
        .map_err(|_| LspError::ContainedLocationRejected)?;
    Ok(Location {
        path,
        range: parse_range(
            value
                .get("range")
                .or_else(|| value.get("targetSelectionRange"))
                .ok_or(LspError::MalformedResponse)?,
        )?,
    })
}

fn normalize_diagnostics(
    session: &LspSession,
    path: &str,
    value: &Value,
    version: Option<i64>,
) -> Result<Vec<Diagnostic>, LspError> {
    let target = safe_path(session, path)?;
    let values = value.as_array().ok_or(LspError::MalformedResponse)?;
    values
        .iter()
        .take(MAX_RESULTS)
        .map(|item| {
            Ok(Diagnostic {
                path: target.clone(),
                range: parse_range(item.get("range").ok_or(LspError::MalformedResponse)?)?,
                severity: item
                    .get("severity")
                    .and_then(Value::as_u64)
                    .map(|v| v as u32),
                code: item.get("code").map(|code| match code {
                    Value::String(v) => v.clone(),
                    _ => code.to_string(),
                }),
                source: item.get("source").and_then(Value::as_str).map(bounded_text),
                message: bounded_text(
                    item.get("message")
                        .and_then(Value::as_str)
                        .ok_or(LspError::MalformedResponse)?,
                ),
                version,
            })
        })
        .collect()
}

pub fn require(supported: bool) -> Result<(), LspError> {
    if supported {
        Ok(())
    } else {
        Err(LspError::UnsupportedCapability)
    }
}

fn parse_position(value: &Value) -> Result<Position, LspError> {
    Ok(Position {
        line: value
            .get("line")
            .and_then(Value::as_u64)
            .ok_or(LspError::MalformedResponse)? as u32,
        character: value
            .get("character")
            .and_then(Value::as_u64)
            .ok_or(LspError::MalformedResponse)? as u32,
    })
}

pub fn parse_range(value: &Value) -> Result<Range, LspError> {
    Ok(Range {
        start: parse_position(value.get("start").ok_or(LspError::MalformedResponse)?)?,
        end: parse_position(value.get("end").ok_or(LspError::MalformedResponse)?)?,
    })
}

pub fn bounded_text(text: &str) -> String {
    text.chars().take(MAX_TEXT).collect()
}

fn bounded_query(query: &str) -> String {
    query.chars().take(256).collect()
}
