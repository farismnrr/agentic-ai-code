//! TypeScript/Vue semantic normalization on top of the generic LSP session
//! (Plan 039C PHASE-04).
//!
//! Two approved, already-installed servers back this adapter:
//! `typescript-language-server` for `.ts`/`.tsx` and `@vue/language-server`
//! (Volar) for `.vue` in standalone/take-over mode. Both are configured with
//! an explicit `typescript.tsdk` pointing at the repository's already
//! reviewed TypeScript install so neither server silently resolves a
//! different TypeScript version. No new server framework, no second daemon:
//! this reuses the exact same generic session/document-sync substrate as
//! [`super::rust`], only the language-specific readiness/settings differ.

use super::semantic::{self, Diagnostic, Hover, Location, RenameFilePreview, Symbol};
use super::LspError;
use crate::core::config::ServerConfig;
use serde_json::{json, Value};

/// Locates the reviewed TypeScript install's `lib` directory (containing
/// `tsserverlibrary.js`) among the operator-approved toolchain paths, so
/// both `typescript-language-server` and `vue-language-server` bind to one
/// single TypeScript version rather than each resolving their own.
pub(super) fn tsdk_path(config: &ServerConfig) -> Option<String> {
    config
        .toolchain_paths
        .iter()
        .map(std::path::PathBuf::from)
        .find(|path| path.join("tsserverlibrary.js").is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

/// `@vue/language-server` resolves its own bundled `typescript` unless
/// given an explicit `--tsdk=<dir>` CLI argument, which it needs to see
/// TypeScript's server-side `ts.server` namespace (used for the hybrid-mode
/// tsserver-request bridge below). This must be a CLI arg, not
/// `initializationOptions` — verified against the installed
/// `@vue/language-server@3.3.8` build, which reads `--tsdk=` from
/// `process.argv` directly in its own bootstrap, before the LSP connection
/// (and therefore `initializationOptions`) even exists.
pub(super) fn tsdk_arg(config: &ServerConfig) -> Option<String> {
    tsdk_path(config).map(|tsdk| format!("--tsdk={tsdk}"))
}

pub fn typescript_workspace_settings(config: &ServerConfig) -> Value {
    match tsdk_path(config) {
        Some(tsdk) => json!({ "typescript": { "tsdk": tsdk } }),
        None => Value::Null,
    }
}

/// `@vue/language-server` standalone/take-over mode: it answers TypeScript
/// semantics for `<script>`/`<script setup>` blocks itself, so no second,
/// conflicting `typescript-language-server` instance is started for the
/// same `.vue` files.
pub fn vue_workspace_settings(config: &ServerConfig) -> Value {
    match tsdk_path(config) {
        Some(tsdk) => json!({ "typescript": { "tsdk": tsdk }, "vue": { "hybridMode": false } }),
        None => Value::Null,
    }
}

pub struct TypeScriptLanguageServer {
    session: std::sync::Arc<super::LspSession>,
}

impl TypeScriptLanguageServer {
    /// Accepts either `typescript` or `vue` sessions: both speak the same
    /// generic LSP semantic subset used here, and neither needs the
    /// rust-analyzer-specific `experimental/serverStatus` readiness
    /// handshake — the `initialize` response already blocks until the
    /// server has advertised its capabilities.
    pub fn new(session: std::sync::Arc<super::LspSession>) -> Result<Self, LspError> {
        if session.language() != "typescript" && session.language() != "vue" {
            return Err(LspError::UnsupportedLanguage);
        }
        Ok(Self { session })
    }

    pub fn session(&self) -> &super::LspSession {
        &self.session
    }

    pub async fn symbols(&self, path: &str) -> Result<Vec<Symbol>, LspError> {
        semantic::symbols(&self.session, path).await
    }

    pub async fn workspace_symbols(&self, query: &str) -> Result<Vec<Symbol>, LspError> {
        semantic::workspace_symbols(&self.session, query).await
    }

    pub async fn definition(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        semantic::definition(&self.session, path, line, utf8_column).await
    }

    pub async fn references(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
        include_declaration: bool,
    ) -> Result<Vec<Location>, LspError> {
        semantic::references(&self.session, path, line, utf8_column, include_declaration).await
    }

    pub async fn implementations(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Vec<Location>, LspError> {
        semantic::implementations(&self.session, path, line, utf8_column).await
    }

    pub async fn hover(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
    ) -> Result<Option<Hover>, LspError> {
        semantic::hover(&self.session, path, line, utf8_column).await
    }

    pub async fn diagnostics(&self, path: &str) -> Result<Vec<Diagnostic>, LspError> {
        semantic::diagnostics(&self.session, path).await
    }

    pub async fn diagnostics_after_version(
        &self,
        path: &str,
        min_version: u64,
    ) -> Result<Vec<Diagnostic>, LspError> {
        semantic::diagnostics_after_version(&self.session, path, min_version).await
    }

    pub async fn rename_preview(
        &self,
        path: &str,
        line: u32,
        utf8_column: usize,
        new_name: &str,
    ) -> Result<Vec<RenameFilePreview>, LspError> {
        semantic::rename_preview(&self.session, path, line, utf8_column, new_name).await
    }
}
