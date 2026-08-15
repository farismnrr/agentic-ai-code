//! Error types for the relay-agent MCP server.
//!
//! Library code in this crate must never panic or unwrap on data it does not
//! fully control (network input, filesystem state, CLI args). This module is
//! the single place those failure modes are named.

use thiserror::Error;

/// Top-level relay-agent error type.
///
/// Kept deliberately small in Phase 1/2: only the failure modes that can
/// actually occur in the config/MCP-core/transport code implemented so far.
/// Later phases (auth, pairing, execution, pidfile) should add their own
/// variants here rather than reaching for `anyhow`/`Box<dyn Error>`.
#[derive(Debug, Error)]
pub enum RelayError {
    /// The provided configuration could not be validated before binding.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// The configured working directory does not exist or is not accessible.
    #[error("working directory is not accessible: {0}")]
    WorkingDirUnavailable(String),

    /// Binding the localhost listener failed.
    #[error("failed to bind {addr}: {source}")]
    Bind {
        addr: String,
        #[source]
        source: std::io::Error,
    },

    /// The HTTP server encountered a fatal I/O error while serving.
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// JSON-RPC / MCP protocol-level error surfaced to a client.
///
/// This is distinct from [`RelayError`]: `RelayError` represents a failure
/// inside this process (config, I/O, lifecycle); `McpError` represents a
/// well-formed protocol-level rejection that must be serialized back to the
/// caller as a JSON-RPC error object per the frozen historical relay contract,
/// now summarized in `.agents/plans/030-previous-plans-summary.md`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum McpError {
    #[error("Parse error")]
    ParseError,

    #[error("Invalid Request: {0}")]
    InvalidRequest(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    Internal(String),

    /// MCP `2026-07-28`-specific: a `Mcp-Method`/`Mcp-Name`/`MCP-Protocol-Version`
    /// standard header is missing, malformed, or does not match the
    /// corresponding request-body value. Per the official spec
    /// (`basic/transports/streamable-http#server-validation`), this is
    /// always HTTP `400` with JSON-RPC code `-32020`.
    #[error("Header mismatch: {0}")]
    HeaderMismatch(String),

    /// MCP `2026-07-28`-specific: the request's protocol version (from the
    /// `MCP-Protocol-Version` header, mirrored in `_meta`) is not one this
    /// server implements. Per spec (`basic/versioning#protocol-version-negotiation`),
    /// always HTTP `400` with JSON-RPC code `-32022`, and `data` lists the
    /// versions this server does support.
    #[error("Unsupported protocol version: requested '{requested}', supported {supported:?}")]
    UnsupportedProtocolVersion {
        supported: Vec<String>,
        requested: String,
    },
}

impl McpError {
    /// JSON-RPC 2.0 / MCP-reserved error code for this error's category.
    pub fn code(&self) -> i32 {
        match self {
            McpError::ParseError => -32700,
            McpError::InvalidRequest(_) => -32600,
            McpError::MethodNotFound(_) => -32601,
            McpError::InvalidParams(_) => -32602,
            McpError::Internal(_) => -32603,
            McpError::HeaderMismatch(_) => -32020,
            McpError::UnsupportedProtocolVersion { .. } => -32022,
        }
    }

    /// Human-readable JSON-RPC `message` field. Internal diagnostics are
    /// intentionally discarded at this public boundary; callers may retain
    /// the variant's detail for private tracing, but it must never cross the
    /// protocol response.
    pub fn message(&self) -> String {
        match self {
            McpError::InvalidRequest(_) => "Invalid request".to_string(),
            McpError::MethodNotFound(_) => "Method not found".to_string(),
            McpError::InvalidParams(_) => "Invalid parameters".to_string(),
            McpError::Internal(_) => "Internal error".to_string(),
            McpError::HeaderMismatch(_) => {
                "Request headers do not match the request body".to_string()
            }
            McpError::ParseError => "Parse error".to_string(),
            McpError::UnsupportedProtocolVersion { .. } => self.to_string(),
        }
    }

    /// The JSON-RPC error object's optional `data` field. Only
    /// [`McpError::UnsupportedProtocolVersion`] carries structured data per
    /// the spec's `UnsupportedProtocolVersionError` shape
    /// (`{"supported": [...], "requested": "..."}`); every other variant
    /// has no `data`.
    pub fn data(&self) -> Option<serde_json::Value> {
        match self {
            McpError::UnsupportedProtocolVersion {
                supported,
                requested,
            } => Some(serde_json::json!({
                "supported": supported,
                "requested": requested,
            })),
            _ => None,
        }
    }
}
