//! Bounded execution lifecycle and application tool dispatch.
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
mod dispatch;
mod jobs;
mod paths;
mod process;
mod requests;
pub(crate) mod sandbox;
mod ssh;
mod toolchain;
pub(crate) use process::kill_process_group;
#[derive(Clone)]
enum InvocationProgram {
    SelfBinary,
    Direct(PathBuf),
}

#[derive(Clone)]
pub(crate) enum InvocationSecurity {
    Standard,
    Ssh {
        identity_file: PathBuf,
        known_hosts_file: PathBuf,
    },
}

#[derive(Clone)]
pub(crate) struct ToolInvocation {
    program: InvocationProgram,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    timeout_ms: u64,
    allow_network: bool,
    expose_optional_sockets: bool,
    expose_authorized_siblings: bool,
    security: InvocationSecurity,
}

pub(super) use jobs::{now_ms, render_output, JobKind};
pub use jobs::{JobManager, JobSnapshot, JobState};
pub fn tool_call_supports_tasks(tool: &Tool, arguments: &Value) -> bool {
    dispatch::supports_tasks(tool, arguments)
}

pub async fn start_tool_task(
    tool: &Tool,
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
    idempotency_key: Option<&str>,
    request_fingerprint: String,
) -> Result<String, McpError> {
    if !tool_call_supports_tasks(tool, arguments) {
        return Err(McpError::InvalidRequest(
            "tool does not support task execution".into(),
        ));
    }
    let job = match tool.name {
        "terminal_exec" => {
            JobKind::Process(requests::build_terminal_exec_invocation(arguments, config)?)
        }
        "http_fetch" => JobKind::Process(requests::build_http_fetch_invocation(arguments)?),
        "web_search" => JobKind::Process(requests::build_web_search_invocation(arguments)),
        _ => {
            return Err(McpError::InvalidRequest(
                "tool task execution is not implemented".into(),
            ))
        }
    };
    if let Some(key) = idempotency_key {
        let (job_id, _) = manager
            .start_with_idempotency_key(key.to_owned(), request_fingerprint, job)
            .await?;
        Ok(job_id)
    } else {
        manager.start(job).await
    }
}

pub async fn start_terminal_job(
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
) -> Result<String, McpError> {
    manager
        .start(JobKind::Process(requests::build_terminal_invocation(
            arguments, config, false,
        )?))
        .await
}

pub async fn dispatch_tool_call(
    tool: &Tool,
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
    lsp: &Arc<crate::lsp::LspSessionManager>,
    hooks: &Arc<crate::hooks::HookManager>,
) -> Result<ToolCallResult, McpError> {
    if let Some(result) = crate::workspace::dispatch_native_tool(tool.name, arguments, config)? {
        if matches!(tool.name, "file_write" | "file_edit" | "apply_patch") && !result.is_error {
            let changed = serde_json::from_str::<Value>(&result.content[0].text).ok();
            let committed = changed.as_ref().is_some_and(|value| {
                value.get("dry_run").and_then(Value::as_bool) != Some(true)
                    && (tool.name == "file_write"
                        || value.get("changed").and_then(Value::as_bool) == Some(true)
                        || value
                            .get("changed_paths")
                            .and_then(Value::as_array)
                            .is_some_and(|paths| !paths.is_empty()))
            });
            if !committed {
                return Ok(result);
            }
            let changed_paths = changed
                .as_ref()
                .and_then(|value| value.get("changed_paths").or_else(|| value.get("path")))
                .cloned()
                .unwrap_or_else(|| json!([]));
            let cwd = arguments.get("cwd").and_then(Value::as_str);
            let paths = changed_paths
                .as_array()
                .map(|paths| paths.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_else(|| changed_paths.as_str().into_iter().collect());
            for path in paths {
                let _ = lsp.refresh_path(cwd, path).await;
            }
            let _ = hooks
                .invoke(
                    crate::hooks::HookEvent::AfterFileChange,
                    json!({
                        "hook_event": "after_file_change",
                        "tool_id": tool.name,
                        "effect_classes": ["workspace_write"],
                        "affected_paths": changed_paths,
                        "success": true,
                    }),
                )
                .await;
        }
        return Ok(result);
    }
    if let Some(result) = crate::git::dispatch_git_tool(tool.name, arguments, config).await? {
        return Ok(result);
    }
    if let Some(result) = crate::code::dispatch_code_tool(tool.name, arguments, config, lsp).await?
    {
        return Ok(result);
    }

    if tool.name == "text_search" {
        return requests::run_text_search(arguments, config).await;
    }

    let job = match tool.name {
        "terminal_exec" => {
            JobKind::Process(requests::build_terminal_exec_invocation(arguments, config)?)
        }
        "http_fetch" => JobKind::Process(requests::build_http_fetch_invocation(arguments)?),
        "web_search" => JobKind::Process(requests::build_web_search_invocation(arguments)),
        _ => return Ok(ToolCallResult::not_implemented(tool.name)),
    };
    let id = manager.start(job).await?;
    let snapshot = manager.wait(&id).await?;
    Ok(snapshot.result.unwrap_or_else(|| {
        ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: "Tool execution failed".into(),
        }])
    }))
}
