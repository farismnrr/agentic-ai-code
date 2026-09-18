//! Fail-closed SSH diagnostic policy.
//!
//! This module deliberately separates SSH connection normalization from remote
//! command validation. Raw OpenSSH configuration and arbitrary remote shell
//! text are never treated as trusted execution contracts.

mod config;
mod remote;

pub use config::{
    openssh_args, openssh_args_for_chain, openssh_args_for_chain_with_program,
    resolve_connection_chain, resolve_connection_spec, validate_alias, SshConnectionSpec,
};
pub(crate) use remote::validate_docker_command;
pub use remote::{validate_remote_command, ValidatedRemoteCommand};

pub(crate) fn policy_error(message: &str) -> crate::core::error::McpError {
    crate::core::error::McpError::InvalidRequest(format!("read-only diagnostic policy: {message}"))
}
