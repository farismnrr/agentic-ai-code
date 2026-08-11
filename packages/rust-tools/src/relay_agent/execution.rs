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

pub async fn dispatch_tool_call(
    tool: &Tool,
    arguments: &Value,
) -> Result<ToolCallResult, McpError> {
    let current_exe = env::current_exe()
        .map_err(|e| McpError::Internal(format!("failed to get current exe path: {e}")))?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| McpError::Internal("current exe has no parent directory".to_string()))?;

    let (bin_name, proc_args, to_ms) = match tool.name {
        "terminal_exec" => {
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

            if let Some(cwd) = arguments.get("cwd").and_then(|v| v.as_str()) {
                args.push("--cwd".to_string());
                args.push(cwd.to_string());
            }

            let binary = match shell_words::split(command) {
                Ok(parts) if !parts.is_empty() => parts[0].clone(),
                _ => String::new(),
            };
            if !binary.is_empty() {
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
                        if s == "--no-guard"
                            || s == "--allow-command"
                            || s.starts_with("--allow-command=")
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
            ("terminal-tool", args, to)
        }
        "http_fetch" => {
            let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let method = arguments
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET");
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
            ("curl-tool", args, to)
        }
        "web_search" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let base_url = arguments
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:8888");
            let args = vec![
                "--base-url".to_string(),
                base_url.to_string(),
                query.to_string(),
            ];
            ("searxng-search-tool", args, 30000)
        }
        _ => return Ok(ToolCallResult::not_implemented(tool.name)),
    };

    let bin_path = bin_dir.join(bin_name);
    if !bin_path.exists() {
        return Err(McpError::Internal(format!(
            "tool binary not found: {}",
            bin_path.display()
        )));
    }

    let mut cmd = Command::new(&bin_path);
    cmd.args(&proc_args);
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
                    let is_error = !status.success();
                    let mut stdout_str = String::from_utf8_lossy(&stdout_bytes).into_owned();
                    let mut stderr_str = String::from_utf8_lossy(&stderr_bytes).into_owned();

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

                    Ok(ToolCallResult {
                        content: contents,
                        is_error,
                    })
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

                    Ok(ToolCallResult {
                        content: vec![ToolResultContent {
                            kind: "text",
                            text: format!("execution timed out after {} ms", wait_duration),
                        }],
                        is_error: true,
                    })
                }
            }
        }
        Err(_) => Err(McpError::Internal("failed to spawn tool".to_string())),
    }
}
