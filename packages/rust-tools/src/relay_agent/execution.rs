use crate::relay_agent::error::McpError;
use crate::relay_agent::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde_json::Value;
use std::env;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const MAX_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MB limit

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
            let mut args = vec![
                "--no-guard".to_string(),
                "--timeout".to_string(),
                to.to_string(),
            ];

            if let Some(cwd) = arguments.get("cwd").and_then(|v| v.as_str()) {
                args.push("--cwd".to_string());
                args.push(cwd.to_string());
            }
            args.push(command.to_string());
            if let Some(arr) = arguments.get("args").and_then(|v| v.as_array()) {
                for arg in arr {
                    if let Some(s) = arg.as_str() {
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

            let mut args = vec![
                "--no-guard".to_string(),
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
                for (k, v) in headers {
                    if let Some(vs) = v.as_str() {
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
        Ok(child) => {
            let pid = child.id();
            let output_res = timeout(
                Duration::from_millis(to_ms + 5000),
                child.wait_with_output(),
            )
            .await;
            match output_res {
                Ok(Ok(output)) => {
                    let is_error = !output.status.success();
                    let mut stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
                    let mut stderr_str = String::from_utf8_lossy(&output.stderr).into_owned();

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
                            text: format!("Process exited with status: {}", output.status),
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
                Ok(Err(e)) => Err(McpError::Internal(format!(
                    "failed to read tool output: {e}"
                ))),
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
                            text: format!("execution timed out after {} ms", to_ms + 5000),
                        }],
                        is_error: true,
                    })
                }
            }
        }
        Err(e) => Err(McpError::Internal(format!("failed to spawn tool: {e}"))),
    }
}
