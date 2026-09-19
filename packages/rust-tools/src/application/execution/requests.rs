//! Tool-specific request validation and invocation translation.
use super::paths::resolve_authorized_cwd;
use super::{InvocationProgram, InvocationSecurity, ToolInvocation};
mod text_search;
pub(crate) use text_search::run_text_search;

const TERMINAL_HARD_TIMEOUT_MS: u64 = 60_000;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;
const MAX_EXEC_ARGS: usize = 100;
const MAX_EXEC_ARG_BYTES: usize = 64 * 1024;
const MAX_HTTP_HEADERS: usize = 100;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

pub(super) fn build_terminal_exec_invocation(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolInvocation, McpError> {
    build_terminal_invocation(arguments, config, true)
}

pub(super) fn build_terminal_invocation(
    arguments: &Value,
    config: &ServerConfig,
    _legacy_job_path: bool,
) -> Result<ToolInvocation, McpError> {
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let operator_max = match config.max_terminal_timeout_ms {
        0 => TERMINAL_HARD_TIMEOUT_MS,
        configured => configured.min(TERMINAL_HARD_TIMEOUT_MS),
    };
    let requested_timeout_ms = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(config.default_terminal_timeout_ms);
    let timeout_ms = if requested_timeout_ms == 0 {
        operator_max
    } else {
        requested_timeout_ms
    };
    if timeout_ms > operator_max {
        return Err(McpError::InvalidRequest(format!(
            "timeout_ms exceeds terminal maximum of {operator_max} ms"
        )));
    }
    let cwd = resolve_authorized_cwd(arguments, config)?;
    let parts = shell_words::split(command)
        .map_err(|_| McpError::InvalidRequest("command could not be parsed".into()))?;
    let Some(binary) = parts.first() else {
        return Err(McpError::InvalidRequest("command must not be empty".into()));
    };
    let mut args = parts[1..].to_vec();
    if let Some(arr) = arguments.get("args").and_then(Value::as_array) {
        if arr.len() > MAX_EXEC_ARGS {
            return Err(McpError::InvalidRequest(
                "argument count exceeds maximum".into(),
            ));
        }
        let mut bytes = args.iter().map(String::len).sum::<usize>();
        for arg in arr.iter().filter_map(Value::as_str) {
            bytes = bytes.saturating_add(arg.len());
            if bytes > MAX_EXEC_ARG_BYTES {
                return Err(McpError::InvalidRequest(
                    "argument bytes exceed maximum".into(),
                ));
            }
            args.push(arg.into());
        }
    }
    let (args, readonly_docker_socket, stdin_file) = normalize_terminal_args(binary, args, config)?;
    let program = super::toolchain::resolve_safe_executable_for(
        config,
        binary,
        config.allow_docker || readonly_docker_socket,
    )?;
    Ok(ToolInvocation {
        program: InvocationProgram::Direct(program),
        args,
        stdin_file,
        cwd: Some(cwd),
        timeout_ms,
        // Terminal network permission is translated here so other process
        // classes cannot inherit it accidentally inside the sandbox layer.
        allow_network: config.allow_terminal_network,
        expose_optional_sockets: true,
        readonly_docker_socket,
        expose_authorized_siblings: true,
        security: InvocationSecurity::Standard,
    })
}

fn normalize_terminal_args(
    binary: &str,
    args: Vec<String>,
    config: &ServerConfig,
) -> Result<(Vec<String>, bool, Option<std::path::PathBuf>), McpError> {
    let readonly_docker_socket = binary == "docker" && !config.allow_docker;
    if !readonly_docker_socket {
        return Ok((args, false, None));
    }
    let mut tokens = vec![binary.to_owned()];
    tokens.extend(args);
    let normalized = crate::core::ssh_policy::validate_docker_command(
        &tokens,
        config.ssh_readonly_db_user.as_deref(),
        config.ssh_readonly_redis_user.as_deref(),
    )?;
    let stdin_file = if normalized.iter().any(|token| token == "redis-cli") {
        Some(
            config
                .resolved_ssh_redis_password_file()
                .map_err(|_| {
                    McpError::InvalidRequest(
                        "Redis read-only password file is unavailable or unsafe".into(),
                    )
                })?
                .ok_or_else(|| {
                    McpError::InvalidRequest(
                        "Redis diagnostics require a configured password file".into(),
                    )
                })?,
        )
    } else {
        None
    };
    Ok((normalized.into_iter().skip(1).collect(), true, stdin_file))
}

pub(super) fn build_http_fetch_invocation(arguments: &Value) -> Result<ToolInvocation, McpError> {
    let url = arguments.get("url").and_then(Value::as_str).unwrap_or("");
    let method = arguments
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .to_uppercase();
    if !["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"].contains(&method.as_str()) {
        return Err(McpError::InvalidRequest(
            "HTTP method is not allowed".into(),
        ));
    }
    let timeout_ms = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    if !(1..=60_000).contains(&timeout_ms) {
        return Err(McpError::InvalidRequest(
            "timeout_ms must be between 1 and 60000 ms".into(),
        ));
    }
    let mut args = vec![
        "curl".into(),
        "-X".into(),
        method,
        "--timeout".into(),
        timeout_ms.to_string(),
    ];
    if let Some(data) = arguments.get("data").and_then(Value::as_str) {
        args.extend(["-d".into(), data.into()]);
    }
    if let Some(headers) = arguments.get("headers").and_then(Value::as_object) {
        if headers.len() > MAX_HTTP_HEADERS {
            return Err(McpError::InvalidRequest(
                "header count exceeds maximum".into(),
            ));
        }
        let mut bytes = 0;
        for (key, value) in headers {
            if let Some(value) = value.as_str() {
                bytes += key.len() + value.len();
                if bytes > MAX_HTTP_HEADER_BYTES {
                    return Err(McpError::InvalidRequest(
                        "header bytes exceed maximum".into(),
                    ));
                }
                args.extend(["-H".into(), format!("{key}: {value}")]);
            }
        }
    }
    args.push(url.into());
    Ok(ToolInvocation {
        program: InvocationProgram::SelfBinary,
        args,
        stdin_file: None,
        cwd: None,
        timeout_ms,
        allow_network: true,
        expose_optional_sockets: false,
        readonly_docker_socket: false,
        expose_authorized_siblings: false,
        security: InvocationSecurity::NetworkOnly,
    })
}

pub(super) fn build_web_search_invocation(arguments: &Value) -> Result<ToolInvocation, McpError> {
    let timeout_ms = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000);
    if !(1..=60_000).contains(&timeout_ms) {
        return Err(McpError::InvalidRequest(
            "timeout_ms must be between 1 and 60000 ms".into(),
        ));
    }
    Ok(ToolInvocation {
        program: InvocationProgram::SelfBinary,
        args: vec![
            "searxng".into(),
            "--base-url".into(),
            "http://127.0.0.1:8888".into(),
            arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .into(),
        ],
        stdin_file: None,
        cwd: None,
        timeout_ms,
        allow_network: true,
        expose_optional_sockets: false,
        readonly_docker_socket: false,
        expose_authorized_siblings: false,
        security: InvocationSecurity::NetworkOnly,
    })
}
