//! Fail-closed SSH diagnostic policy.
//!
//! This module deliberately separates SSH connection normalization from remote
//! command validation. Raw OpenSSH configuration and arbitrary remote shell
//! text are never treated as trusted execution contracts.

mod config;
mod remote;

pub use config::{openssh_args, resolve_connection_spec, validate_alias, SshConnectionSpec};
pub use remote::{validate_remote_command, ValidatedRemoteCommand};

pub(crate) fn policy_error(message: &str) -> crate::error::McpError {
    crate::error::McpError::InvalidRequest(format!("SSH diagnostic policy: {message}"))
}
