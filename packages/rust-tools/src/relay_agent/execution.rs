use crate::relay_agent::error::McpError;
use crate::relay_agent::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde_json::Value;
use std::env;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const MAX_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MB limit
const MAX_TIMEOUT_MS: u64 = 300_000; // 5 mins
const TIMEOUT_GRACE_MS: u64 = 5_000;
const MAX_EXEC_ARGS: usize = 100;
const MAX_EXEC_ARG_BYTES: usize = 64 * 1024; // 64 KB limit
const MAX_HTTP_HEADERS: usize = 100;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024; // 64 KB limit

/// A resolved, tool-specific invocation: the sibling binary name, the CLI
/// arguments to pass it, and the caller-requested timeout. Building this is
/// the only part of tool dispatch that varies per tool; everything after it
/// (bwrap containment, spawn, output capture, timeout/kill, cleanup) is one
/// shared process-safety path in [`run_sandboxed`] so no tool can bypass it.
struct ToolInvocation {
    bin_name: &'static str,
    args: Vec<String>,
    timeout_ms: u64,
}

/// Build the sandboxed `terminal-tool` invocation for `terminal_exec`:
/// validates timeout bounds, resolves/contains `cwd` under the execution
/// root, validates the leading executable, and bounds argument count/size.
fn build_terminal_exec_invocation(
    arguments: &Value,
    config: &crate::relay_agent::config::ServerConfig,
) -> Result<ToolInvocation, McpError> {
    let command = arguments
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to = arguments
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30000);
    if to > MAX_TIMEOUT_MS {
        return Err(McpError::InvalidRequest(format!(
            "timeout_ms exceeds maximum of {} ms",
            MAX_TIMEOUT_MS
        )));
    }
    let mut args = vec!["--timeout".to_string(), to.to_string()];

    let execution_root = config
        .resolved_execution_root()
        .map_err(|e| McpError::Internal(e.to_string()))?;

    let canonical_cwd = crate::relay_agent::terminal_policy::resolve_contained_cwd(
        &execution_root,
        arguments.get("cwd").and_then(|v| v.as_str()),
    )?;

    args.push("--cwd".to_string());
    args.push(canonical_cwd.to_string_lossy().into_owned());

    let parts = shell_words::split(command).unwrap_or_default();
    let binary = if !parts.is_empty() {
        parts[0].clone()
    } else {
        String::new()
    };

    if !binary.is_empty() {
        crate::relay_agent::terminal_policy::validate_executable(&binary)?;

        // NOTE: Shell/interpreter flags like `bash -c` are intentionally NOT
        // blocked here. bwrap is the containment boundary — it provides
        // filesystem isolation, PID namespace, and process group kill-on-timeout.
        // Blocking `bash -c` only breaks legitimate coding workflows (e.g.
        // `bash -c 'npm install && npm run build'`) without adding meaningful
        // security when the sandbox is already active. If bwrap is not available
        // the server refuses to start (see relay-agent.rs startup check).

        args.push("--allow-command".to_string());
        args.push(binary);
    }

    args.push(command.to_string());
    if let Some(arr) = arguments.get("args").and_then(|v| v.as_array()) {
        if arr.len() > MAX_EXEC_ARGS {
            return Err(McpError::InvalidRequest(format!(
                "argument count exceeds maximum of {}",
                MAX_EXEC_ARGS
            )));
        }
        let mut total_arg_bytes = 0;
        for arg in arr {
            if let Some(s) = arg.as_str() {
                if s == "--no-guard" || s == "--allow-command" || s.starts_with("--allow-command=")
                {
                    return Err(McpError::InvalidRequest(format!(
                        "argument {} is forbidden",
                        s
                    )));
                }
                total_arg_bytes += s.len();
                if total_arg_bytes > MAX_EXEC_ARG_BYTES {
                    return Err(McpError::InvalidRequest(format!(
                        "total argument bytes exceed maximum of {}",
                        MAX_EXEC_ARG_BYTES
                    )));
                }
                args.push(s.to_string());
            }
        }
    }

    Ok(ToolInvocation {
        bin_name: "terminal-tool",
        args,
        timeout_ms: to,
    })
}

/// Build the sandboxed `curl-tool` invocation for `http_fetch`: validates
/// the HTTP method and timeout bounds, and bounds header count/size.
fn build_http_fetch_invocation(arguments: &Value) -> Result<ToolInvocation, McpError> {
    let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let method = arguments
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");

    let method_upper = method.to_uppercase();
    match method_upper.as_str() {
        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => {}
        _ => {
            return Err(McpError::InvalidRequest(format!(
                "HTTP method {} is not allowed",
                method
            )));
        }
    }

    let to = arguments
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30000);
    if to > MAX_TIMEOUT_MS {
        return Err(McpError::InvalidRequest(format!(
            "timeout_ms exceeds maximum of {} ms",
            MAX_TIMEOUT_MS
        )));
    }

    let mut args = vec![
        "-X".to_string(),
        method.to_string(),
        "--timeout".to_string(),
        to.to_string(),
    ];

    if let Some(data) = arguments.get("data").and_then(|v| v.as_str()) {
        args.push("-d".to_string());
        args.push(data.to_string());
    }
    if let Some(headers) = arguments.get("headers").and_then(|v| v.as_object()) {
        if headers.len() > MAX_HTTP_HEADERS {
            return Err(McpError::InvalidRequest(format!(
                "header count exceeds maximum of {}",
                MAX_HTTP_HEADERS
            )));
        }
        let mut total_header_bytes = 0;
        for (k, v) in headers {
            if let Some(vs) = v.as_str() {
                total_header_bytes += k.len() + vs.len();
                if total_header_bytes > MAX_HTTP_HEADER_BYTES {
                    return Err(McpError::InvalidRequest(format!(
                        "total header bytes exceed maximum of {}",
                        MAX_HTTP_HEADER_BYTES
                    )));
                }
                args.push("-H".to_string());
                args.push(format!("{k}: {vs}"));
            }
        }
    }
    args.push(url.to_string());

    Ok(ToolInvocation {
        bin_name: "curl-tool",
        args,
        timeout_ms: to,
    })
}

/// Build the sandboxed `searxng-search-tool` invocation for `web_search`.
fn build_web_search_invocation(arguments: &Value) -> ToolInvocation {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let base_url = "http://127.0.0.1:8888";
    let args = vec![
        "--base-url".to_string(),
        base_url.to_string(),
        query.to_string(),
    ];
    ToolInvocation {
        bin_name: "searxng-search-tool",
        args,
        timeout_ms: 30000,
    }
}

pub async fn dispatch_tool_call(
    tool: &Tool,
    arguments: &Value,
    config: &crate::relay_agent::config::ServerConfig,
) -> Result<ToolCallResult, McpError> {
    let current_exe = env::current_exe()
        .map_err(|e| McpError::Internal(format!("failed to get current exe path: {e}")))?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| McpError::Internal("current exe has no parent directory".to_string()))?;

    let ToolInvocation {
        bin_name,
        args: proc_args,
        timeout_ms: to_ms,
    } = match tool.name {
        "terminal_exec" => build_terminal_exec_invocation(arguments, config)?,
        "http_fetch" => build_http_fetch_invocation(arguments)?,
        "web_search" => build_web_search_invocation(arguments),
        _ => return Ok(ToolCallResult::not_implemented(tool.name)),
    };

    let bin_path = bin_dir.join(bin_name);
    if !bin_path.exists() {
        return Err(McpError::Internal(format!(
            "tool binary not found: {}",
            bin_path.display()
        )));
    }

    let execution_root_str = config
        .resolved_execution_root()
        .map_err(|e| McpError::Internal(format!("failed to resolve execution root: {e}")))?
        .to_string_lossy()
        .into_owned();
    let bin_dir_str = bin_dir.to_string_lossy().into_owned();

    let mut cmd = Command::new("bwrap");

    let mut bwrap_args = vec![
        "--ro-bind".to_string(),
        "/usr".to_string(),
        "/usr".to_string(),
        "--ro-bind".to_string(),
        "/lib".to_string(),
        "/lib".to_string(),
        "--ro-bind-try".to_string(),
        "/lib64".to_string(),
        "/lib64".to_string(),
        "--ro-bind-try".to_string(),
        "/etc".to_string(),
        "/etc".to_string(),
        "--ro-bind-try".to_string(),
        "/bin".to_string(),
        "/bin".to_string(),
        "--ro-bind-try".to_string(),
        "/sbin".to_string(),
        "/sbin".to_string(),
        "--ro-bind-try".to_string(),
        "/opt".to_string(),
        "/opt".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--bind".to_string(),
        execution_root_str.clone(),
        execution_root_str,
        "--ro-bind".to_string(),
        bin_dir_str.clone(),
        bin_dir_str,
        "--unshare-pid".to_string(),
        "--die-with-parent".to_string(),
        bin_path.to_string_lossy().into_owned(),
    ];
    bwrap_args.extend(proc_args);

    cmd.args(&bwrap_args);
    cmd.env_clear();

    // P1-4: Use a hardcoded safe PATH instead of inheriting the parent process PATH.
    // Inheriting the parent PATH could allow an attacker who controls the relay-agent
    // launch environment to substitute malicious binaries via PATH manipulation.
    // The bwrap binary is invoked by its name above; callers should ensure bwrap is
    // installed to a standard system path (/usr/bin/bwrap is typical).
    cmd.env(
        "PATH",
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    );

    // Explicitly strip environment variables that might alter execution trust
    cmd.env_remove("LD_PRELOAD");
    cmd.env_remove("LD_LIBRARY_PATH");
    cmd.env_remove("NODE_OPTIONS");
    cmd.env_remove("PYTHONPATH");
    cmd.env_remove("RUBYLIB");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Kill the immediate child on drop (safety net)
    cmd.kill_on_drop(true);

    #[cfg(unix)]
    {
        // Put the child in its own process group so we can kill the tree
        cmd.process_group(0);
    }

    let child_res = cmd.spawn();
    match child_res {
        Ok(mut child) => {
            let pid = child.id();

            let mut stdout_pipe = child.stdout.take().unwrap();
            let mut stderr_pipe = child.stderr.take().unwrap();

            let read_stdout = async {
                let mut stdout_buf = Vec::new();
                let mut handle = (&mut stdout_pipe).take(MAX_OUTPUT_BYTES as u64 + 1);
                handle
                    .read_to_end(&mut stdout_buf)
                    .await
                    .map(|_| stdout_buf)
            };
            let read_stderr = async {
                let mut stderr_buf = Vec::new();
                let mut handle = (&mut stderr_pipe).take(MAX_OUTPUT_BYTES as u64 + 1);
                handle
                    .read_to_end(&mut stderr_buf)
                    .await
                    .map(|_| stderr_buf)
            };

            let read_and_wait = async {
                let (out_res, err_res) = tokio::join!(read_stdout, read_stderr);
                let stdout_buf = out_res?;
                let stderr_buf = err_res?;

                if stdout_buf.len() > MAX_OUTPUT_BYTES || stderr_buf.len() > MAX_OUTPUT_BYTES {
                    if let Some(p) = pid {
                        #[cfg(unix)]
                        {
                            unsafe {
                                libc::kill(-(p as i32), libc::SIGKILL);
                            }
                        }
                    }
                }

                let status = child.wait().await?;
                Ok::<_, std::io::Error>((status, stdout_buf, stderr_buf))
            };

            let wait_duration = to_ms.saturating_add(TIMEOUT_GRACE_MS);
            let output_res = timeout(Duration::from_millis(wait_duration), read_and_wait).await;
            match output_res {
                Ok(Ok((status, stdout_bytes, stderr_bytes))) => {
                    let mut stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();
                    let mut stderr_str = String::from_utf8_lossy(&stderr_bytes).into_owned();
                    let is_error = !status.success()
                        || (bin_name == "terminal-tool" && !stdout_str.starts_with("Exit: 0\n"));

                    if stdout_str.len() > MAX_OUTPUT_BYTES {
                        stdout_str.truncate(MAX_OUTPUT_BYTES);
                        stdout_str.push_str("\n...[truncated due to size limit]");
                    }
                    if stderr_str.len() > MAX_OUTPUT_BYTES {
                        stderr_str.truncate(MAX_OUTPUT_BYTES);
                        stderr_str.push_str("\n...[truncated due to size limit]");
                    }

                    let mut contents = vec![];
                    if !stdout_str.is_empty() {
                        contents.push(ToolResultContent {
                            kind: "text",
                            text: stdout_str,
                        });
                    }
                    if !stderr_str.is_empty() {
                        contents.push(ToolResultContent {
                            kind: "text",
                            text: stderr_str,
                        });
                    }
                    if contents.is_empty() && is_error {
                        contents.push(ToolResultContent {
                            kind: "text",
                            text: format!("Process exited with status: {}", status),
                        });
                    }

                    if contents.is_empty() {
                        contents.push(ToolResultContent {
                            kind: "text",
                            text: "".to_string(),
                        });
                    }

                    if is_error {
                        Ok(ToolCallResult::error(contents))
                    } else {
                        Ok(ToolCallResult::complete(contents))
                    }
                }
                Ok(Err(_e)) => Err(McpError::Internal("failed to read tool output".to_string())),
                Err(_) => {
                    // Timeout occurred
                    if let Some(p) = pid {
                        #[cfg(unix)]
                        {
                            unsafe {
                                // Kill the entire process group
                                libc::kill(-(p as i32), libc::SIGKILL);
                            }
                        }
                    }

                    Ok(ToolCallResult::error(vec![ToolResultContent {
                        kind: "text",
                        text: format!("execution timed out after {} ms", wait_duration),
                    }]))
                }
            }
        }
        Err(_) => Err(McpError::Internal("failed to spawn tool".to_string())),
    }
}
