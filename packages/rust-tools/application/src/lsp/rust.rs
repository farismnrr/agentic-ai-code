//! Rust-specific semantic normalization on top of the generic LSP session.
//!
//! This adapter deliberately speaks only the small subset needed by the Rust
//! proof. It does not parse Rust or maintain a second semantic index.

use super::{LspError, LspSession};
use crate::workspace::reject_protected_target;
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

const MAX_TEXT: usize = 16 * 1024;
const MAX_RESULTS: usize = 128;
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// rust-analyzer configuration for the read-only, network-isolated LSP
/// proof: skip the dependency graph fetch (`cargo.noDeps`) and disable
/// check-on-save/flycheck so semantic readiness never depends on
/// crates.io reachability or writing build artifacts back into the
/// workspace.
pub fn workspace_settings() -> Value {
    json!({
        "cargo": { "noDeps": true },
        "checkOnSave": false
    })
}

/// Opts into rust-analyzer's `experimental/serverStatus` notification so
/// readiness can be observed deterministically instead of guessed with a
/// sleep.
pub fn experimental_capabilities() -> Value {
    json!({ "serverStatusNotification": true })
}

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

pub struct RustLanguageServer {
    session: std::sync::Arc<LspSession>,
}

impl RustLanguageServer {
    pub fn new(session: std::sync::Arc<LspSession>) -> Result<Self, LspError> {
        if session.language() != "rust" {
            return Err(LspError::UnsupportedLanguage);
        }
        Ok(Self { session })
    }

    pub fn session(&self) -> &LspSession {
        &self.session
    }

    /// Waits, bounded, for rust-analyzer to report
    /// `experimental/serverStatus { quiescent: true }` — the signal that
    /// project/sysroot loading has settled and semantic requests
    /// (definition/hover/references) will return real results rather than
    /// empty ones. This replaces a fixed sleep, which is a race: loading
    /// time varies with sandbox/filesystem warmth and is not a valid
    /// readiness contract.
    ///
    /// A bounded timeout guards against waiting forever; a reported
    /// `health: "error"` status surfaces as a stable failure rather than
    /// proceeding with a server that can't produce a valid semantic proof.
    pub async fn wait_ready(&self) -> Result<(), LspError> {
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
        loop {
            if self.session.is_faulted() {
                return Err(LspError::Crashed);
            }
            if let Some(status) = self
                .session
                .latest_notification("experimental/serverStatus")
                .await
            {
                let quiescent = status
                    .get("quiescent")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if quiescent {
                    let health = status.get("health").and_then(Value::as_str).unwrap_or("ok");
                    return if health == "error" {
                        Err(LspError::StartupFailed)
                    } else {
                        Ok(())
                    };
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(LspError::Timeout);
            }
            sleep(READY_POLL_INTERVAL).await;
        }
    }

    pub async fn symbols(&self, path: &str) -> Result<Vec<Symbol>, LspError> {
        self.require(self.session.capabilities().document_symbols)?;
        let uri = self.sync(path).await?;
        let value = self
            .session
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

    pub async fn definition(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        self.require(self.session.capabilities().definition)?;
        self.locations("textDocument/definition", path, line, utf8_column)
            .await
    }

    pub async fn references(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        self.require(self.session.capabilities().references)?;
        let mut locations = self
            .request_locations(
                "textDocument/references",
                path,
                line,
                utf8_column,
                json!({"context":{"includeDeclaration":true}}),
            )
            .await?;
        locations.truncate(MAX_RESULTS);
        Ok(locations)
    }

    pub async fn implementations(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        self.require(self.session.capabilities().implementation)?;
        self.locations("textDocument/implementation", path, line, utf8_column)
            .await
    }

    pub async fn hover(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Option<Hover>, LspError> {
        self.require(self.session.capabilities().hover)?;
        let uri = self.sync(path).await?;
        let position = self.position(path, line, utf8_column)?;
        let value = self
            .session
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

    pub async fn diagnostics(&self, path: &str) -> Result<Vec<Diagnostic>, LspError> {
        let uri = self.sync(path).await?;
        if self.session.capabilities().diagnostic_pull {
            let response = self
                .session
                .request(
                    "textDocument/diagnostic",
                    json!({"textDocument":{"uri":uri}}),
                )
                .await?;
            return self.normalize_diagnostics(
                path,
                response.get("items").unwrap_or(&response),
                None,
            );
        }
        for _ in 0..40 {
            if let Some(value) = self.session.latest_diagnostics(&uri).await {
                let version = value.get("version").and_then(Value::as_i64);
                return self.normalize_diagnostics(
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

    async fn locations(
        &self,
        method: &str,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        let mut locations = self
            .request_locations(method, path, line, utf8_column, Value::Null)
            .await?;
        locations.truncate(MAX_RESULTS);
        Ok(locations)
    }

    async fn request_locations(
        &self,
        method: &str,
        path: &str,
        line: u32,
        utf8_column: usize,
        extra: Value,
    ) -> Result<Vec<Location>, LspError> {
        let uri = self.sync(path).await?;
        let position = self.position(path, line, utf8_column)?;
        let mut params = json!({"textDocument":{"uri":uri},"position":position});
        if let Value::Object(extra) = extra {
            params.as_object_mut().unwrap().extend(extra);
        }
        let value = self.session.request(method, params).await?;
        let values = value.as_array().cloned().unwrap_or_else(|| vec![value]);
        values
            .iter()
            .map(|value| self.normalize_location(value))
            .collect()
    }

    async fn sync(&self, path: &str) -> Result<String, LspError> {
        let target = self.safe_path(path)?;
        self.session.sync_document(path).await?;
        url::Url::from_file_path(target)
            .map(|url| url.to_string())
            .map_err(|_| LspError::ContainedLocationRejected)
    }

    fn safe_path(&self, path: &str) -> Result<PathBuf, LspError> {
        let target =
            resolve_existing_path(&self.session.identity().root, None, path, EntryKind::File)
                .map_err(|_| LspError::ContainedLocationRejected)?;
        reject_protected_target(&self.session.identity().root, &target)
            .map_err(|_| LspError::ContainedLocationRejected)?;
        Ok(target)
    }

    fn position(&self, path: &str, line: u32, utf8_column: usize) -> Result<Value, LspError> {
        let target = self.safe_path(path)?;
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

    fn normalize_location(&self, value: &Value) -> Result<Location, LspError> {
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
            &self.session.identity().root,
            None,
            &path.to_string_lossy(),
            EntryKind::File,
        )
        .map_err(|_| LspError::ContainedLocationRejected)?;
        reject_protected_target(&self.session.identity().root, &path)
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
        &self,
        path: &str,
        value: &Value,
        version: Option<i64>,
    ) -> Result<Vec<Diagnostic>, LspError> {
        let target = self.safe_path(path)?;
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

    fn require(&self, supported: bool) -> Result<(), LspError> {
        if supported {
            Ok(())
        } else {
            Err(LspError::UnsupportedCapability)
        }
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

fn parse_range(value: &Value) -> Result<Range, LspError> {
    Ok(Range {
        start: parse_position(value.get("start").ok_or(LspError::MalformedResponse)?)?,
        end: parse_position(value.get("end").ok_or(LspError::MalformedResponse)?)?,
    })
}

fn bounded_text(text: &str) -> String {
    text.chars().take(MAX_TEXT).collect()
}
