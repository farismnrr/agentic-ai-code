//! Bounded Language Server Protocol foundation (Plan 039C phases 01-02).
//!
//! This module intentionally owns no MCP routing and no language-specific
//! semantic operations. It provides approved executable selection, contained
//! per-workspace sessions, JSON-RPC lifecycle/correlation, capability capture,
//! bounded document synchronization, and public-safe normalized failures.

mod document;
mod framing;
mod manager;
mod protocol;

pub use manager::LspSessionManager;
pub use protocol::LspSession;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

pub const MAX_LSP_HEADER_BYTES: usize = 8 * 1024;
pub const MAX_LSP_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_LSP_STDERR_BYTES: usize = 64 * 1024;
pub const MAX_LSP_PENDING_REQUESTS: usize = 64;
pub const MAX_LSP_PROJECTS: usize = 4;
pub const MAX_LSP_SESSIONS: usize = 8;
pub const MAX_LSP_DOCUMENTS_PER_SESSION: usize = 16;
pub const MAX_LSP_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const LSP_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
pub const LSP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const LSP_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
pub const LSP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
pub const LSP_RESTART_WINDOW: Duration = Duration::from_secs(60);
pub const MAX_LSP_RESTARTS_PER_WINDOW: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LspError {
    ServerUnavailable,
    UnsupportedLanguage,
    UnsupportedCapability,
    StartupFailed,
    Timeout,
    Crashed,
    MalformedResponse,
    OversizedResponse,
    StaleDocument,
    ContainedLocationRejected,
    CapacityReached,
    Internal,
}

impl LspError {
    pub fn kind(self) -> &'static str {
        match self {
            Self::ServerUnavailable => "language_server_unavailable",
            Self::UnsupportedLanguage => "unsupported_language",
            Self::UnsupportedCapability => "unsupported_lsp_capability",
            Self::StartupFailed => "language_server_startup_failed",
            Self::Timeout => "language_server_timeout",
            Self::Crashed => "language_server_crashed",
            Self::MalformedResponse => "malformed_language_server_response",
            Self::OversizedResponse => "language_server_response_too_large",
            Self::StaleDocument => "stale_document_result",
            Self::ContainedLocationRejected => "contained_location_rejected",
            Self::CapacityReached => "language_server_capacity_reached",
            Self::Internal => "language_server_internal_error",
        }
    }

    pub fn safe_message(self) -> &'static str {
        match self {
            Self::ServerUnavailable => "Configured language server is unavailable",
            Self::UnsupportedLanguage => "Language is not supported",
            Self::UnsupportedCapability => "Language server capability is not supported",
            Self::StartupFailed => "Language server could not be started",
            Self::Timeout => "Language server request timed out",
            Self::Crashed => "Language server exited unexpectedly",
            Self::MalformedResponse => "Language server returned an invalid response",
            Self::OversizedResponse => "Language server response exceeded the configured limit",
            Self::StaleDocument => "Language server result is stale",
            Self::ContainedLocationRejected => "Language server location is outside the workspace",
            Self::CapacityReached => "Language server session capacity is reached",
            Self::Internal => "Language server request failed",
        }
    }
}

impl std::fmt::Display for LspError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.safe_message())
    }
}
impl std::error::Error for LspError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceIdentity {
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub(super) struct ApprovedServerSpec {
    pub language: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDocumentSyncKind {
    None,
    Full,
    Incremental,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub definition: bool,
    pub references: bool,
    pub implementation: bool,
    pub hover: bool,
    pub document_symbols: bool,
    pub workspace_symbols: bool,
    pub rename: bool,
    pub diagnostic_pull: bool,
    pub text_sync: TextDocumentSyncKind,
    pub open_close: bool,
}

impl ServerCapabilities {
    pub(super) fn from_initialize(value: &serde_json::Value) -> Self {
        let capabilities = value
            .get("capabilities")
            .unwrap_or(&serde_json::Value::Null);
        let provider = |key: &str| match capabilities.get(key) {
            Some(serde_json::Value::Bool(value)) => *value,
            Some(serde_json::Value::Object(_)) => true,
            _ => false,
        };
        let (text_sync, open_close) = match capabilities.get("textDocumentSync") {
            Some(serde_json::Value::Number(value)) => (
                match value.as_u64().unwrap_or(0) {
                    1 => TextDocumentSyncKind::Full,
                    2 => TextDocumentSyncKind::Incremental,
                    _ => TextDocumentSyncKind::None,
                },
                false,
            ),
            Some(serde_json::Value::Object(object)) => {
                let kind = match object
                    .get("change")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                {
                    1 => TextDocumentSyncKind::Full,
                    2 => TextDocumentSyncKind::Incremental,
                    _ => TextDocumentSyncKind::None,
                };
                (
                    kind,
                    object
                        .get("openClose")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                )
            }
            _ => (TextDocumentSyncKind::None, false),
        };
        Self {
            definition: provider("definitionProvider"),
            references: provider("referencesProvider"),
            implementation: provider("implementationProvider"),
            hover: provider("hoverProvider"),
            document_symbols: provider("documentSymbolProvider"),
            workspace_symbols: provider("workspaceSymbolProvider"),
            rename: provider("renameProvider"),
            diagnostic_pull: provider("diagnosticProvider"),
            text_sync,
            open_close,
        }
    }
}
