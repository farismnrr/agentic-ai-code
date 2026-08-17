//! Public MCP code-intelligence tool surface (Plan 039C PHASE-05/06):
//! `code_symbols`, `code_definition`, `code_references`,
//! `code_implementations`, `code_hover`, `code_diagnostics`,
//! `code_rename_preview`.
//!
//! This module only adapts already-validated MCP tool arguments into calls
//! against the existing LSP substrate (`crate::lsp`) and normalizes the
//! result/error into the same public-safe `ToolCallResult`/`McpError` shape
//! every other tool uses. It owns no LSP protocol logic itself.

use crate::lsp::semantic::{Diagnostic, Hover, Location, RenameFilePreview, Symbol};
use crate::lsp::{
    LspError, LspSession, LspSessionManager, RustLanguageServer, TypeScriptLanguageServer,
};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_MAX_RESULTS: usize = 50;

/// Dispatches to the Rust or TypeScript/Vue adapter for the same generic
/// LSP session, keeping every `code_*` tool handler language-agnostic.
enum Adapter {
    Rust(RustLanguageServer),
    TypeScript(TypeScriptLanguageServer),
}

impl Adapter {
    fn new(language: &str, session: Arc<LspSession>) -> Result<Self, LspError> {
        if language == "rust" {
            Ok(Self::Rust(RustLanguageServer::new(session)?))
        } else {
            Ok(Self::TypeScript(TypeScriptLanguageServer::new(session)?))
        }
    }

    async fn symbols(&self, path: &str) -> Result<Vec<Symbol>, LspError> {
        match self {
            Self::Rust(server) => server.symbols(path).await,
            Self::TypeScript(server) => server.symbols(path).await,
        }
    }

    async fn definition(
        &self,
        path: &str,
        line: u32,
        column: usize,
    ) -> Result<Vec<Location>, LspError> {
        match self {
            Self::Rust(server) => server.definition(path, line, column).await,
            Self::TypeScript(server) => server.definition(path, line, column).await,
        }
    }

    async fn references(
        &self,
        path: &str,
        line: u32,
        column: usize,
        include_declaration: bool,
    ) -> Result<Vec<Location>, LspError> {
        match self {
            Self::Rust(server) => {
                server
                    .references(path, line, column, include_declaration)
                    .await
            }
            Self::TypeScript(server) => {
                server
                    .references(path, line, column, include_declaration)
                    .await
            }
        }
    }

    async fn implementations(
        &self,
        path: &str,
        line: u32,
        column: usize,
    ) -> Result<Vec<Location>, LspError> {
        match self {
            Self::Rust(server) => server.implementations(path, line, column).await,
            Self::TypeScript(server) => server.implementations(path, line, column).await,
        }
    }

    async fn hover(&self, path: &str, line: u32, column: usize) -> Result<Option<Hover>, LspError> {
        match self {
            Self::Rust(server) => server.hover(path, line, column).await,
            Self::TypeScript(server) => server.hover(path, line, column).await,
        }
    }

    async fn diagnostics(&self, path: &str) -> Result<Vec<Diagnostic>, LspError> {
        match self {
            Self::Rust(server) => server.diagnostics(path).await,
            Self::TypeScript(server) => server.diagnostics(path).await,
        }
    }

    async fn rename_preview(
        &self,
        path: &str,
        line: u32,
        column: usize,
        new_name: &str,
    ) -> Result<Vec<RenameFilePreview>, LspError> {
        match self {
            Self::Rust(server) => server.rename_preview(path, line, column, new_name).await,
            Self::TypeScript(server) => server.rename_preview(path, line, column, new_name).await,
        }
    }
}

pub async fn dispatch_code_tool(
    name: &str,
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<Option<ToolCallResult>, McpError> {
    let result = match name {
        "code_symbols" => code_symbols(arguments, config, lsp).await,
        "code_definition" => code_definition(arguments, config, lsp).await,
        "code_references" => code_references(arguments, config, lsp).await,
        "code_implementations" => code_implementations(arguments, config, lsp).await,
        "code_hover" => code_hover(arguments, config, lsp).await,
        "code_diagnostics" => code_diagnostics(arguments, config, lsp).await,
        "code_rename_preview" => code_rename_preview(arguments, config, lsp).await,
        _ => return Ok(None),
    }?;
    Ok(Some(result))
}

async fn code_symbols(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let cwd = string_arg(arguments, "cwd")?;
    let path = string_arg(arguments, "path")?;
    let query = string_arg(arguments, "query")?;
    match (path, query) {
        (Some(path), None) => {
            let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
            let symbols = adapter.symbols(&path).await.map_err(lsp_error)?;
            paginated_result(arguments, symbols)
        }
        (None, Some(query)) => {
            let root = crate::git::resolve_git_workspace(cwd.as_deref(), config)
                .map_err(|_| McpError::InvalidRequest("cwd is not a usable workspace".into()))?;
            let language = infer_project_language(&root).ok_or_else(|| {
                McpError::InvalidRequest(
                    "cannot determine a language for workspace symbol search; pass path instead"
                        .into(),
                )
            })?;
            if language == "rust" {
                return Err(McpError::InvalidRequest(
                    "workspace symbol search is not supported for rust in this build".into(),
                ));
            }
            let session = lsp
                .session_for(cwd.as_deref(), language)
                .await
                .map_err(lsp_error)?;
            let ts = TypeScriptLanguageServer::new(session).map_err(lsp_error)?;
            let symbols = ts.workspace_symbols(&query).await.map_err(lsp_error)?;
            paginated_result(arguments, symbols)
        }
        (Some(_), Some(_)) => Err(McpError::InvalidRequest(
            "code_symbols accepts either path or query, not both".into(),
        )),
        (None, None) => Err(McpError::InvalidRequest(
            "code_symbols requires path or query".into(),
        )),
    }
}

async fn code_definition(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let (path, line, column, cwd) = position_args(arguments)?;
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let locations = adapter
        .definition(&path, line, column)
        .await
        .map_err(lsp_error)?;
    json_result(&locations_json(
        &workspace_root(cwd.as_deref(), config)?,
        &locations,
    ))
}

async fn code_references(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let (path, line, column, cwd) = position_args(arguments)?;
    let include_declaration = bool_arg(arguments, "include_declaration")?.unwrap_or(true);
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let locations = adapter
        .references(&path, line, column, include_declaration)
        .await
        .map_err(lsp_error)?;
    let root = workspace_root(cwd.as_deref(), config)?;
    let (page, next) = paginate(arguments, locations)?;
    json_result_with(&locations_json(&root, &page), next)
}

async fn code_implementations(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let (path, line, column, cwd) = position_args(arguments)?;
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let locations = adapter
        .implementations(&path, line, column)
        .await
        .map_err(lsp_error)?;
    let root = workspace_root(cwd.as_deref(), config)?;
    let (page, next) = paginate(arguments, locations)?;
    json_result_with(&locations_json(&root, &page), next)
}

async fn code_hover(
    arguments: &Value,
    _config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let (path, line, column, cwd) = position_args(arguments)?;
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let hover = adapter
        .hover(&path, line, column)
        .await
        .map_err(lsp_error)?;
    json_result(&hover)
}

async fn code_diagnostics(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let cwd = string_arg(arguments, "cwd")?;
    let path = string_arg(arguments, "path")?
        .ok_or_else(|| McpError::InvalidRequest("code_diagnostics requires path".into()))?;
    let severity = arguments
        .get("severity")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let diagnostics = adapter.diagnostics(&path).await.map_err(lsp_error)?;
    let diagnostics: Vec<Diagnostic> = diagnostics
        .into_iter()
        .filter(|diagnostic| severity.is_none_or(|s| diagnostic.severity == Some(s)))
        .collect();
    let root = workspace_root(cwd.as_deref(), config)?;
    let (page, next) = paginate(arguments, diagnostics)?;
    let items: Vec<Value> = page
        .into_iter()
        .map(|diagnostic| {
            json!({
                "path": relative_path(&root, &diagnostic.path),
                "range": diagnostic.range,
                "severity": diagnostic.severity,
                "code": diagnostic.code,
                "source": diagnostic.source,
                "message": diagnostic.message,
                "version": diagnostic.version,
            })
        })
        .collect();
    json_result_with(&json!({ "diagnostics": items }), next)
}

async fn code_rename_preview(
    arguments: &Value,
    config: &ServerConfig,
    lsp: &Arc<LspSessionManager>,
) -> Result<ToolCallResult, McpError> {
    let (path, line, column, cwd) = position_args(arguments)?;
    let new_name = arguments
        .get("new_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 4096)
        .ok_or_else(|| McpError::InvalidRequest("code_rename_preview requires new_name".into()))?
        .to_owned();
    let (adapter, _) = adapter_for_path(&path, cwd.as_deref(), lsp).await?;
    let preview = adapter
        .rename_preview(&path, line, column, &new_name)
        .await
        .map_err(lsp_error)?;
    let root = workspace_root(cwd.as_deref(), config)?;
    let files: Vec<Value> = preview
        .into_iter()
        .map(|file: RenameFilePreview| {
            json!({
                "path": relative_path(&root, &file.path),
                "edits": file.edits,
            })
        })
        .collect();
    json_result(&json!({ "files": files, "applied": false }))
}

// ---- shared plumbing ----

fn workspace_root(cwd: Option<&str>, config: &ServerConfig) -> Result<PathBuf, McpError> {
    crate::git::resolve_git_workspace(cwd, config)
        .map_err(|_| McpError::InvalidRequest("cwd is not a usable workspace".into()))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(|value| value.trim_start_matches('/').to_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn locations_json(root: &Path, locations: &[Location]) -> Value {
    json!({
        "locations": locations
            .iter()
            .map(|location| json!({
                "path": relative_path(root, &location.path),
                "range": location.range,
            }))
            .collect::<Vec<_>>()
    })
}

fn string_arg(arguments: &Value, key: &str) -> Result<Option<String>, McpError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.is_empty() && value.len() <= 4096 => {
            Ok(Some(value.clone()))
        }
        Some(Value::String(_)) => Err(McpError::InvalidRequest(format!("{key} is invalid"))),
        _ => Err(McpError::InvalidRequest(format!("{key} must be a string"))),
    }
}

fn bool_arg(arguments: &Value, key: &str) -> Result<Option<bool>, McpError> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(McpError::InvalidRequest(format!("{key} must be a boolean"))),
    }
}

fn position_args(arguments: &Value) -> Result<(String, u32, usize, Option<String>), McpError> {
    let path = string_arg(arguments, "path")?
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;
    let line = arguments
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| McpError::InvalidRequest("line is required".into()))? as u32;
    let column = arguments
        .get("column")
        .and_then(Value::as_u64)
        .ok_or_else(|| McpError::InvalidRequest("column is required".into()))?
        as usize;
    let cwd = string_arg(arguments, "cwd")?;
    Ok((path, line, column, cwd))
}

fn infer_language(path: &str) -> Result<&'static str, McpError> {
    let extension = Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");
    match extension {
        "rs" => Ok("rust"),
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => Ok("typescript"),
        "vue" => Ok("vue"),
        _ => Err(McpError::InvalidRequest(
            "unsupported language for this file extension".into(),
        )),
    }
}

fn infer_project_language(root: &Path) -> Option<&'static str> {
    if root.join("Cargo.toml").is_file() {
        Some("rust")
    } else if root.join("tsconfig.json").is_file() || root.join("package.json").is_file() {
        Some("typescript")
    } else {
        None
    }
}

async fn adapter_for_path(
    path: &str,
    cwd: Option<&str>,
    lsp: &Arc<LspSessionManager>,
) -> Result<(Adapter, &'static str), McpError> {
    let language = infer_language(path)?;
    let session = lsp.session_for(cwd, language).await.map_err(lsp_error)?;
    let adapter = Adapter::new(language, session).map_err(lsp_error)?;
    Ok((adapter, language))
}

fn lsp_error(error: LspError) -> McpError {
    McpError::InvalidRequest(format!(
        "{{\"code\":\"{}\"}} {}",
        error.kind(),
        error.safe_message()
    ))
}

fn paginate<T>(arguments: &Value, mut items: Vec<T>) -> Result<(Vec<T>, Option<String>), McpError> {
    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .min(128);
    let offset = match arguments.get("continuation").and_then(Value::as_str) {
        Some(token) => token
            .parse::<usize>()
            .map_err(|_| McpError::InvalidRequest("invalid continuation token".into()))?,
        None => 0,
    };
    if offset > items.len() {
        return Err(McpError::InvalidRequest(
            "invalid continuation token".into(),
        ));
    }
    let total = items.len();
    let page: Vec<T> = items.drain(offset..).take(max_results).collect();
    let next_offset = offset + page.len();
    let next = if next_offset < total {
        Some(next_offset.to_string())
    } else {
        None
    };
    Ok((page, next))
}

fn paginated_result<T: Serialize>(
    arguments: &Value,
    items: Vec<T>,
) -> Result<ToolCallResult, McpError> {
    let (page, next) = paginate(arguments, items)?;
    json_result_with(&json!({ "symbols": page }), next)
}

fn json_result(value: &impl Serialize) -> Result<ToolCallResult, McpError> {
    json_result_with(value, None)
}

fn json_result_with(
    value: &impl Serialize,
    continuation: Option<String>,
) -> Result<ToolCallResult, McpError> {
    let mut object = serde_json::to_value(value)
        .map_err(|_| McpError::Internal("failed to serialize code result".into()))?;
    if let Some(continuation) = continuation {
        if let Some(map) = object.as_object_mut() {
            map.insert("continuation".into(), Value::String(continuation));
        }
    }
    let text = serde_json::to_string(&object)
        .map_err(|_| McpError::Internal("failed to serialize code result".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}
