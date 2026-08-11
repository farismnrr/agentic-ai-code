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
/// caller as a JSON-RPC error object per the frozen contract in
/// `.agents/plans/028-phase0-contract-audit.md`.
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
}

impl McpError {
    /// JSON-RPC 2.0 reserved error code for this error's category.
    pub fn code(&self) -> i32 {
        match self {
            McpError::ParseError => -32700,
            McpError::InvalidRequest(_) => -32600,
            McpError::MethodNotFound(_) => -32601,
            McpError::InvalidParams(_) => -32602,
            McpError::Internal(_) => -32603,
        }
    }

    /// Human-readable JSON-RPC `message` field. Never includes secrets,
    /// stack traces, or raw process/environment internals (security
    /// invariant #12 in the plan) — callers should keep any detail passed
    /// into these variants free of that data.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_json_rpc_reserved_range() {
        assert_eq!(McpError::ParseError.code(), -32700);
        assert_eq!(McpError::InvalidRequest("x".into()).code(), -32600);
        assert_eq!(McpError::MethodNotFound("x".into()).code(), -32601);
        assert_eq!(McpError::InvalidParams("x".into()).code(), -32602);
        assert_eq!(McpError::Internal("x".into()).code(), -32603);
    }
}
