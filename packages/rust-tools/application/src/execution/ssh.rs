//! Translation from the familiar `terminal_exec` SSH shape into the dedicated
//! read-only SSH execution class.

use super::paths::resolve_authorized_cwd;
use super::{InvocationProgram, InvocationSecurity, ToolInvocation};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde_json::Value;

const MAX_EXEC_ARGS: usize = 100;
const MAX_EXEC_ARG_BYTES: usize = 64 * 1024;
const MAX_SSH_TIMEOUT_MS: u64 = 60_000;

pub(super) fn build_invocation(
    arguments: &Value,
    config: &ServerConfig,
    command_parts: &[String],
    requested_timeout_ms: u64,
) -> Result<ToolInvocation, McpError> {
    if !config.allow_ssh {
        return Err(McpError::InvalidRequest(
            "SSH diagnostics are disabled; set RELAY_ALLOW_SSH=true explicitly".into(),
        ));
    }
    let timeout_ms = if requested_timeout_ms == 0 {
        config
            .default_terminal_timeout_ms
            .clamp(1, MAX_SSH_TIMEOUT_MS)
    } else {
        requested_timeout_ms
    };
    if timeout_ms > MAX_SSH_TIMEOUT_MS {
        return Err(McpError::InvalidRequest(
            "timeout_ms exceeds SSH diagnostic maximum".into(),
        ));
    }

    let mut tokens = command_parts[1..].to_vec();
    append_explicit_args(arguments, &mut tokens)?;
    let alias = tokens
        .first()
        .ok_or_else(|| McpError::InvalidRequest("bare interactive SSH is forbidden".into()))?;
    relay_core::ssh_policy::validate_alias(alias)?;
    let remote_tokens = &tokens[1..];
    if remote_tokens.is_empty() {
        return Err(McpError::InvalidRequest(
            "bare interactive SSH is forbidden; a validated diagnostic command is required".into(),
        ));
    }
    let remote_raw = render_remote_request(remote_tokens);
    let remote = relay_core::ssh_policy::validate_remote_command(
        &remote_raw,
        config.ssh_readonly_db_user.as_deref(),
        config.ssh_readonly_redis_user.as_deref(),
    )?;
    let ssh_root = config
        .resolved_ssh_root()
        .map_err(|_| McpError::InvalidRequest("SSH credential root is unavailable".into()))?;
    let ssh_config = config
        .resolved_ssh_config()
        .map_err(|_| McpError::InvalidRequest("SSH config is unavailable".into()))?;
    let spec = relay_core::ssh_policy::resolve_connection_spec(&ssh_root, &ssh_config, alias)?;
    let args = relay_core::ssh_policy::openssh_args(&spec, &remote);
    let cwd = resolve_authorized_cwd(arguments, config)?;

    Ok(ToolInvocation {
        program: InvocationProgram::Direct(resolve_openssh_client()?),
        args,
        cwd: Some(cwd),
        timeout_ms,
        allow_network: true,
        expose_optional_sockets: false,
        expose_authorized_siblings: false,
        security: InvocationSecurity::Ssh {
            identity_file: spec.identity_file,
            known_hosts_file: spec.known_hosts_file,
        },
    })
}

fn resolve_openssh_client() -> Result<std::path::PathBuf, McpError> {
    const CANDIDATES: &[&str] = &["/usr/bin/ssh", "/bin/ssh"];
    const TRUSTED_ROOTS: &[&str] = &["/usr/bin", "/bin", "/usr/lib/openssh"];
    for candidate in CANDIDATES {
        let path = std::path::Path::new(candidate);
        if !path.is_file() || !is_executable(path) {
            continue;
        }
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| McpError::InvalidRequest("system OpenSSH client is unavailable".into()))?;
        if TRUSTED_ROOTS
            .iter()
            .map(std::path::Path::new)
            .any(|root| canonical.starts_with(root))
        {
            return Ok(path.to_path_buf());
        }
    }
    Err(McpError::InvalidRequest(
        "trusted system OpenSSH client is unavailable".into(),
    ))
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn append_explicit_args(arguments: &Value, args: &mut Vec<String>) -> Result<(), McpError> {
    if let Some(arr) = arguments.get("args").and_then(Value::as_array) {
        if arr.len() > MAX_EXEC_ARGS {
            return Err(McpError::InvalidRequest(
                "argument count exceeds maximum".into(),
            ));
        }
        let mut bytes = args.iter().map(String::len).sum::<usize>();
        for value in arr {
            let arg = value.as_str().ok_or_else(|| {
                McpError::InvalidRequest("terminal args must contain only strings".into())
            })?;
            bytes = bytes.saturating_add(arg.len());
            if bytes > MAX_EXEC_ARG_BYTES {
                return Err(McpError::InvalidRequest(
                    "argument bytes exceed maximum".into(),
                ));
            }
            args.push(arg.into());
        }
    }
    Ok(())
}

pub(super) fn normalized_failure(stderr: &str) -> Option<&'static str> {
    let value = stderr.to_ascii_lowercase();
    if value.contains("host key verification failed")
        || value.contains("remote host identification has changed")
    {
        return Some("SSH host-key verification failed");
    }
    if value.contains("permission denied")
        || value.contains("authentication failed")
        || value.contains("no more authentication methods")
        || value.contains("sign_and_send_pubkey")
    {
        return Some("SSH key authentication failed non-interactively");
    }
    if value.contains("passphrase") || value.contains("askpass") {
        return Some("SSH key requires interactive authentication; execution stopped");
    }
    if value.contains("could not resolve hostname") || value.contains("name or service not known") {
        return Some("SSH host resolution failed");
    }
    if value.contains("connection timed out") || value.contains("connection refused") {
        return Some("SSH connection failed");
    }
    None
}

fn render_remote_request(tokens: &[String]) -> String {
    if tokens.len() == 1 && tokens[0].chars().any(char::is_whitespace) {
        return tokens[0].clone();
    }
    tokens
        .iter()
        .map(|token| match token.as_str() {
            "|" | "&&" | "||" => token.clone(),
            _ => shell_words::quote(token).into_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
