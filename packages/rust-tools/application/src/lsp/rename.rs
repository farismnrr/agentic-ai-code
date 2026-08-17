//! Bounded, non-mutating rename-preview normalization (Plan 039C PHASE-06).
//!
//! Split out of [`super::semantic`] purely to keep both files under the
//! repository's per-file maintainability budget; this is still part of the
//! same shared semantic layer every language adapter reuses.

use super::semantic::{
    parse_range, position, require, sync, RenameEdit, RenameFilePreview, MAX_RENAME_EDITS,
    MAX_RENAME_FILES, MAX_RENAME_TEXT,
};
use super::{LspError, LspSession};
use crate::workspace::reject_protected_target;
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Requests a rename WorkspaceEdit from the server and normalizes it into a
/// bounded, safe, non-mutating preview. Supports both common WorkspaceEdit
/// shapes (`changes` and `documentChanges` text edits). Any unsupported
/// resource operation (create/rename/delete) fails the whole preview closed
/// rather than silently dropping it.
pub async fn rename_preview(
    session: &LspSession,
    path: &str,
    line: u32,
    utf8_column: usize,
    new_name: &str,
) -> Result<Vec<RenameFilePreview>, LspError> {
    require(session.capabilities().rename)?;
    if new_name.is_empty() || new_name.len() > MAX_RENAME_TEXT {
        return Err(LspError::MalformedResponse);
    }
    let uri = sync(session, path).await?;
    let position = position(session, path, line, utf8_column)?;
    let value = session
        .request(
            "textDocument/rename",
            json!({"textDocument":{"uri":uri},"position":position,"newName":new_name}),
        )
        .await?;
    if value.is_null() {
        return Ok(Vec::new());
    }
    let mut by_uri: HashMap<String, Vec<RenameEdit>> = HashMap::new();

    if let Some(changes) = value.get("changes").and_then(Value::as_object) {
        for (uri, edits) in changes {
            let edits = edits.as_array().ok_or(LspError::MalformedResponse)?;
            let list = by_uri.entry(uri.clone()).or_default();
            for edit in edits {
                list.push(parse_text_edit(edit)?);
            }
        }
    } else if let Some(document_changes) = value.get("documentChanges").and_then(Value::as_array) {
        for change in document_changes {
            if change.get("kind").is_some() {
                // CreateFile/RenameFile/DeleteFile resource operations are
                // not representable in a safe preview-only model. Fail
                // closed rather than silently omitting a mutation the
                // caller did not ask for.
                return Err(LspError::UnsupportedCapability);
            }
            let text_document = change
                .get("textDocument")
                .ok_or(LspError::MalformedResponse)?;
            let uri = text_document
                .get("uri")
                .and_then(Value::as_str)
                .ok_or(LspError::MalformedResponse)?
                .to_owned();
            let edits = change
                .get("edits")
                .and_then(Value::as_array)
                .ok_or(LspError::MalformedResponse)?;
            let list = by_uri.entry(uri).or_default();
            for edit in edits {
                list.push(parse_text_edit(edit)?);
            }
        }
    } else {
        return Err(LspError::MalformedResponse);
    }

    if by_uri.len() > MAX_RENAME_FILES {
        return Err(LspError::OversizedResponse);
    }
    let total_edits: usize = by_uri.values().map(Vec::len).sum();
    if total_edits > MAX_RENAME_EDITS {
        return Err(LspError::OversizedResponse);
    }

    let mut previews = Vec::with_capacity(by_uri.len());
    for (uri, mut edits) in by_uri {
        let path = url::Url::parse(&uri)
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
        edits.sort_by(|a, b| {
            (a.range.start.line, a.range.start.character)
                .cmp(&(b.range.start.line, b.range.start.character))
        });
        for window in edits.windows(2) {
            let previous_end = (window[0].range.end.line, window[0].range.end.character);
            let next_start = (window[1].range.start.line, window[1].range.start.character);
            if next_start < previous_end {
                return Err(LspError::MalformedResponse);
            }
        }
        previews.push(RenameFilePreview { path, edits });
    }
    previews.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(previews)
}

fn parse_text_edit(value: &Value) -> Result<RenameEdit, LspError> {
    let range = parse_range(value.get("range").ok_or(LspError::MalformedResponse)?)?;
    let new_text = value
        .get("newText")
        .and_then(Value::as_str)
        .ok_or(LspError::MalformedResponse)?;
    if new_text.len() > MAX_RENAME_TEXT {
        return Err(LspError::OversizedResponse);
    }
    Ok(RenameEdit {
        range,
        new_text: new_text.to_owned(),
    })
}
