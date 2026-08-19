//! Tool-specific request validation and invocation translation.

use super::now_ms;
use super::process::{drain_pipe, kill_process_group, OutputBuffer};
use super::sandbox;
use super::{InvocationProgram, ToolInvocation};
use crate::workspace::reject_protected_target;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

const MAX_EXEC_ARGS: usize = 100;
const MAX_EXEC_ARG_BYTES: usize = 64 * 1024;
const MAX_HTTP_HEADERS: usize = 100;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;
const DEFAULT_TEXT_SEARCH_RESULTS: usize = 50;
const MAX_TEXT_SEARCH_RESULTS: usize = 100;
const MAX_TEXT_SEARCH_PREVIEW_BYTES: usize = 1024;
const MAX_TEXT_SEARCH_RESULT_BYTES: usize = 256 * 1024;
const MAX_TEXT_SEARCH_QUERY_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_GLOB_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_CWD_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_STDERR_BYTES: usize = 8192;
const TEXT_SEARCH_MAX_COLUMNS: usize = 1024;

fn resolve_safe_executable(config: &ServerConfig, binary: &str) -> Result<PathBuf, McpError> {
    sandbox::resolve_safe_executable(config, binary)
}

pub(super) fn build_terminal_exec_invocation(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolInvocation, McpError> {
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let timeout_ms = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(config.default_terminal_timeout_ms);
    if config.tool_profile == relay_core::config::ToolProfile::Primary
        && (timeout_ms == 0 || timeout_ms > 30_000)
    {
        return Err(McpError::InvalidRequest(
            "primary terminal_exec timeout_ms must be between 1 and 30000; use terminal_job_start for long-running work".into(),
        ));
    }
    if config.max_terminal_timeout_ms > 0 && timeout_ms > config.max_terminal_timeout_ms {
        return Err(McpError::InvalidRequest(
            "timeout_ms exceeds operator maximum".into(),
        ));
    }
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let _ = config.ensure_workspaces_initialized();
    let cwd = match arguments.get("cwd").and_then(Value::as_str) {
        Some(cwd) => {
            if let Ok(guard) = config.workspaces.read() {
                relay_core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, Some(cwd))?
            } else {
                relay_core::terminal_policy::resolve_contained_cwd(&execution_root, Some(cwd))?
            }
        }
        None => std::fs::canonicalize(
            config
                .resolved_dir()
                .map_err(|_| McpError::Internal("failed to resolve workspace directory".into()))?,
        )
        .map_err(|_| McpError::InvalidRequest("workspace directory is inaccessible".into()))?,
    };
    if !config.is_path_contained(&cwd) {
        return Err(McpError::InvalidRequest(
            "working directory is outside authorized workspace roots".into(),
        ));
    }
    reject_protected_target(&execution_root, &cwd)?;
    let parts = shell_words::split(command)
        .map_err(|_| McpError::InvalidRequest("command could not be parsed".into()))?;
    let Some(binary) = parts.first() else {
        return Err(McpError::InvalidRequest("command must not be empty".into()));
    };
    let program = resolve_safe_executable(config, binary)?;
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
    Ok(ToolInvocation {
        program: InvocationProgram::Direct(program),
        args,
        cwd: Some(cwd),
        timeout_ms,
        allow_network: false,
    })
}

#[derive(Debug, Serialize)]
struct TextSearchResult {
    matches: Vec<TextSearchMatch>,
    count: usize,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}

#[derive(Debug, Serialize)]
struct TextSearchMatch {
    path: String,
    line: u64,
    column: u64,
    preview: String,
}

async fn read_bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, McpError> {
    let mut line = Vec::new();
    loop {
        let buffer = reader
            .fill_buf()
            .await
            .map_err(|_| McpError::InvalidRequest("text search output is invalid".into()))?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let take = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(buffer.len(), |index| index + 1);
        if line.len().saturating_add(take) > max_bytes {
            return Err(McpError::InvalidRequest(
                "text search match line exceeds maximum".into(),
            ));
        }
        line.extend_from_slice(&buffer[..take]);
        reader.consume(take);
        if line.last() == Some(&b'\n') {
            return Ok(Some(line));
        }
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn build_text_search_invocation(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<(ToolInvocation, usize), McpError> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("text search query is required".into()))?;
    if query.is_empty() || query.len() > MAX_TEXT_SEARCH_QUERY_BYTES {
        return Err(McpError::InvalidRequest(
            "text search query exceeds allowed bounds".into(),
        ));
    }
    let cwd_arg = arguments.get("cwd").and_then(Value::as_str);
    if cwd_arg.is_some_and(|cwd| cwd.len() > MAX_TEXT_SEARCH_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "text search cwd exceeds maximum".into(),
        ));
    }
    let glob = arguments.get("glob").and_then(Value::as_str);
    if glob.is_some_and(|glob| glob.is_empty() || glob.len() > MAX_TEXT_SEARCH_GLOB_BYTES) {
        return Err(McpError::InvalidRequest(
            "text search glob exceeds allowed bounds".into(),
        ));
    }
    let regex = arguments
        .get("regex")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let case_sensitive = arguments
        .get("case_sensitive")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_TEXT_SEARCH_RESULTS)
        .clamp(1, MAX_TEXT_SEARCH_RESULTS);

    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let _ = config.ensure_workspaces_initialized();
    let cwd = match cwd_arg {
        Some(cwd_str) => {
            if let Ok(guard) = config.workspaces.read() {
                relay_core::workspace_path::resolve_contained_cwd_in_allowlist(
                    &guard,
                    Some(cwd_str),
                )?
            } else {
                relay_core::terminal_policy::resolve_contained_cwd(&execution_root, Some(cwd_str))?
            }
        }
        None => std::fs::canonicalize(
            config
                .resolved_dir()
                .map_err(|_| McpError::Internal("failed to resolve workspace directory".into()))?,
        )
        .map_err(|_| McpError::InvalidRequest("workspace directory is inaccessible".into()))?,
    };
    if !config.is_path_contained(&cwd) {
        return Err(McpError::InvalidRequest(
            "working directory is outside authorized workspace roots".into(),
        ));
    }
    reject_protected_target(&execution_root, &cwd)?;
    let mut args = vec![
        "--json".into(),
        "--no-config".into(),
        "--no-messages".into(),
        "--max-columns".into(),
        TEXT_SEARCH_MAX_COLUMNS.to_string(),
        "--max-columns-preview".into(),
        "--sort".into(),
        "path".into(),
    ];
    for excluded in relay_core::protected_paths::ripgrep_exclusion_globs() {
        args.extend(["--glob".into(), excluded]);
    }
    // Arbitrary `.env.*` variants are filtered again from parsed match paths
    // below. Keeping `.env.example` searchable avoids an unsafe positive-glob
    // re-include that would otherwise narrow the entire ripgrep search set.
    if !regex {
        args.push("--fixed-strings".into());
    }
    if !case_sensitive {
        args.push("--ignore-case".into());
    }
    if let Some(glob) = glob {
        args.extend(["--glob".into(), glob.to_owned()]);
    }
    args.extend(["--".into(), query.to_owned(), ".".into()]);
    Ok((
        ToolInvocation {
            program: InvocationProgram::Direct(resolve_safe_executable(config, "rg")?),
            args,
            cwd: Some(cwd),
            timeout_ms: 0,
            allow_network: false,
        },
        max_results,
    ))
}

pub(super) async fn run_text_search(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolCallResult, McpError> {
    let (invocation, max_results) = build_text_search_invocation(arguments, config)?;
    let mut child = sandbox::spawn(config, &invocation, sandbox::WorkspaceAccess::ReadOnly)
        .map_err(|_| McpError::Internal("failed to start text search".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Internal("text search stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| McpError::Internal("text search stderr unavailable".into()))?;
    let stderr_buffer = Arc::new(Mutex::new(OutputBuffer::new(now_ms())));
    let stderr_task = tokio::spawn(drain_pipe(
        stderr,
        stderr_buffer.clone(),
        MAX_TEXT_SEARCH_STDERR_BYTES,
    ));

    let mut lines = BufReader::new(stdout);
    let mut matches = Vec::new();
    let mut truncated = false;
    loop {
        let line =
            match read_bounded_line(&mut lines, MAX_TEXT_SEARCH_PREVIEW_BYTES.saturating_mul(8))
                .await
            {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(error) => {
                    kill_process_group(&mut child).await;
                    let _ = child.wait().await;
                    let _ = stderr_task.await;
                    return Err(error);
                }
            };
        let event: Value = serde_json::from_slice(&line)
            .map_err(|_| McpError::InvalidRequest("text search output is invalid".into()))?;
        if event.get("type").and_then(Value::as_str) != Some("match") {
            continue;
        }
        let data = &event["data"];
        let path = data["path"]["text"]
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("text search output is invalid".into()))?;
        let relative_path = path.strip_prefix("./").unwrap_or(path);
        if relay_core::protected_paths::is_protected_relative(std::path::Path::new(relative_path)) {
            continue;
        }
        if matches.len() >= crate::continuation::MAX_TOTAL_ENTRIES {
            truncated = true;
            kill_process_group(&mut child).await;
            break;
        }
        let line_number = data["line_number"]
            .as_u64()
            .ok_or_else(|| McpError::InvalidRequest("text search output is invalid".into()))?;
        let column = data["submatches"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["start"].as_u64())
            .unwrap_or(0)
            .saturating_add(1);
        let preview = data["lines"]["text"]
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("text search output is invalid".into()))?;
        matches.push(TextSearchMatch {
            path: relative_path.to_owned(),
            line: line_number,
            column,
            preview: truncate_utf8(
                preview.trim_end_matches(['\r', '\n']),
                MAX_TEXT_SEARCH_PREVIEW_BYTES,
            ),
        });
    }

    let status = child
        .wait()
        .await
        .map_err(|_| McpError::Internal("text search process failed".into()))?;
    let _ = stderr_task.await;
    if !truncated {
        match status.code() {
            Some(0 | 1) => {}
            Some(2) => {
                return Err(McpError::InvalidRequest(
                    "text search pattern is invalid".into(),
                ))
            }
            _ => return Err(McpError::InvalidRequest("text search failed".into())),
        }
    }

    let scope = invocation
        .cwd
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (matches, continuation) = crate::continuation::paginate(
        arguments,
        matches,
        max_results,
        config,
        "text_search",
        &scope,
        None,
    )?;
    let mut result = TextSearchResult {
        count: matches.len(),
        matches,
        truncated: continuation.is_some() || truncated,
        continuation,
    };
    while !result.matches.is_empty()
        && serde_json::to_vec(&result)
            .map_err(|_| McpError::Internal("failed to serialize text search result".into()))?
            .len()
            > MAX_TEXT_SEARCH_RESULT_BYTES
    {
        result.matches.pop();
        result.count = result.matches.len();
        result.truncated = true;
    }
    let text = serde_json::to_string(&result)
        .map_err(|_| McpError::Internal("failed to serialize text search result".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
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
        cwd: None,
        timeout_ms,
        allow_network: true,
    })
}

pub(super) fn build_web_search_invocation(arguments: &Value) -> ToolInvocation {
    ToolInvocation {
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
        cwd: None,
        timeout_ms: 30_000,
        allow_network: true,
    }
}
