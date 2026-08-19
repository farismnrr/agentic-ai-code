//! Rust-specific semantic normalization on top of the generic LSP session.
//!
//! This adapter deliberately speaks only the small subset needed by the Rust
//! proof. It does not parse Rust or maintain a second semantic index. The
//! actual request/response normalization is shared with every other
//! language adapter through [`super::semantic`]; this module owns only the
//! rust-analyzer-specific readiness handshake.

use super::semantic::{self, Diagnostic, Hover, Location, RenameFilePreview, Symbol};
use super::LspError;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

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

pub struct RustLanguageServer {
    session: std::sync::Arc<super::LspSession>,
}

impl RustLanguageServer {
    pub fn new(session: std::sync::Arc<super::LspSession>) -> Result<Self, LspError> {
        if session.language() != "rust" {
            return Err(LspError::UnsupportedLanguage);
        }
        Ok(Self { session })
    }

    pub fn session(&self) -> &super::LspSession {
        &self.session
    }

    /// Waits, bounded, for rust-analyzer to report
    /// `experimental/serverStatus { quiescent: true }` — the signal that
    /// project/sysroot loading has settled and semantic requests
    /// (definition/hover/references) will return real results rather than
    /// empty ones. This replaces a fixed sleep, which is a race: loading
    /// time varies with sandbox/filesystem warmth and is not a valid
    /// readiness contract.
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
        self.wait_ready().await?;
        semantic::symbols(&self.session, path).await
    }

    pub async fn workspace_symbols(&self, query: &str) -> Result<Vec<Symbol>, LspError> {
        self.wait_ready().await?;
        semantic::workspace_symbols(&self.session, query).await
    }

    pub async fn definition(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        self.wait_ready().await?;
        semantic::definition(&self.session, path, line, utf8_column).await
    }

    pub async fn references(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
        include_declaration: bool,
    ) -> Result<Vec<Location>, LspError> {
        self.wait_ready().await?;
        semantic::references(&self.session, path, line, utf8_column, include_declaration).await
    }

    pub async fn implementations(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        self.wait_ready().await?;
        semantic::implementations(&self.session, path, line, utf8_column).await
    }

    pub async fn hover(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Option<Hover>, LspError> {
        self.wait_ready().await?;
        semantic::hover(&self.session, path, line, utf8_column).await
    }

    pub async fn diagnostics(&self, path: &str) -> Result<Vec<Diagnostic>, LspError> {
        self.wait_ready().await?;
        semantic::diagnostics(&self.session, path).await
    }

    pub async fn diagnostics_after_version(
        &self,
        path: &str,
        min_version: u64,
    ) -> Result<Vec<Diagnostic>, LspError> {
        self.wait_ready().await?;
        semantic::diagnostics_after_version(&self.session, path, min_version).await
    }

    pub async fn rename_preview(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
        new_name: &str,
    ) -> Result<Vec<RenameFilePreview>, LspError> {
        self.wait_ready().await?;
        semantic::rename_preview(&self.session, path, line, utf8_column, new_name).await
    }
}
