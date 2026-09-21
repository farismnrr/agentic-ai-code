//! Bounded execution lifecycle and application tool dispatch.
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
mod jobs;
mod paths;
mod process;
mod requests;
pub(crate) mod sandbox;
mod ssh;
mod toolchain;
pub(crate) use process::kill_process_group;
pub use toolchain::resolve_safe_executable;

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
#[doc(hidden)]
pub mod protected_index_test_support {
    use super::sandbox;
    use std::io;
    use std::path::Path;
    use std::time::Duration;

    pub fn prime_with_entry_limit(root: &Path, max_entries: usize) -> io::Result<usize> {
        sandbox::test_prime_protected_path_index_with_entry_limit(
            root,
            Duration::from_secs(5),
            max_entries,
        )
    }

    pub fn discover(root: &Path) -> io::Result<()> {
        sandbox::test_discover_protected_path_index(root)
    }

    pub fn snapshot_rejects_new_protected_path(root: &Path, relative: &Path) -> io::Result<()> {
        sandbox::test_snapshot_rejects_new_protected_path(root, relative)
    }

    pub fn schedule_initialization(root: &Path) -> io::Result<()> {
        sandbox::test_schedule_protected_path_index(root, Duration::from_secs(5))
    }

    pub fn is_permanent_error(kind: io::ErrorKind) -> bool {
        sandbox::test_is_permanent_index_error(kind)
    }
}

#[derive(Clone)]
enum InvocationProgram {
    SelfBinary,
    Direct(PathBuf),
}

#[derive(Clone)]
pub(crate) enum InvocationSecurity {
    Standard,
    /// Dedicated outbound HTTP/search requests have no need to expose any
    /// owner workspace or host-backed project files.
    NetworkOnly,
    Ssh {
        material_files: Vec<PathBuf>,
    },
}

#[derive(Clone)]
pub(crate) struct ToolInvocation {
    program: InvocationProgram,
    args: Vec<String>,
    stdin_file: Option<PathBuf>,
    cwd: Option<PathBuf>,
    timeout_ms: u64,
    allow_network: bool,
    expose_optional_sockets: bool,
    readonly_docker_socket: bool,
    expose_authorized_siblings: bool,
    execution_deadline: Option<Instant>,
    security: InvocationSecurity,
}

pub(super) use jobs::{now_ms, render_output, JobKind};
pub use jobs::{JobManager, JobSnapshot, JobState};

/// Synchronous compatibility helper for integration tests: the call does not
/// return until the bounded terminal execution reaches a terminal state.
#[doc(hidden)]
pub async fn start_terminal_job(
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
) -> Result<String, McpError> {
    start_terminal_job_for(arguments, config, manager, "local", None).await
}

/// Owner/session-aware synchronous compatibility helper for integration tests.
#[doc(hidden)]
pub async fn start_terminal_job_for(
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
    owner: &str,
    session: Option<&str>,
) -> Result<String, McpError> {
    let id = manager
        .start_for(
            JobKind::Process(requests::build_terminal_invocation(
                arguments, config, false,
            )?),
            owner,
            session,
        )
        .await?;
    let _ = manager.wait(&id).await?;
    Ok(id)
}

pub async fn dispatch_tool_call(
    tool: &Tool,
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
    lsp: &Arc<crate::application::lsp::LspSessionManager>,
    hooks: &Arc<crate::application::hooks::HookManager>,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    if let Some(result) =
        crate::application::workspace::dispatch_native_tool(tool.name, arguments, config)?
    {
        if matches!(tool.name, "file_write" | "file_edit" | "apply_patch") && !result.is_error {
            let changed = result.structured_content.clone().or_else(|| {
                result
                    .content
                    .first()
                    .and_then(|content| serde_json::from_str::<Value>(&content.text).ok())
            });
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
                    crate::application::hooks::HookEvent::AfterFileChange,
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
    if let Some(result) =
        crate::application::git::dispatch_git_tool(tool.name, arguments, config).await?
    {
        return Ok(result);
    }
    if let Some(result) =
        crate::application::code::dispatch_code_tool(tool.name, arguments, config, lsp).await?
    {
        return Ok(result);
    }
    if let Some(result) =
        crate::application::creative::dispatch_tool(tool.name, arguments, config, owner).await?
    {
        return Ok(result);
    }
    if tool.name.starts_with("blender_") {
        let bounded_config = crate::application::blender::bounded_mcp_config(config, tool.name);
        if let Some(result) =
            crate::application::blender::dispatch_tool(tool.name, arguments, &bounded_config, owner)
                .await?
        {
            return Ok(result);
        }
    }

    if tool.name == "text_search" {
        return requests::run_text_search(arguments, config).await;
    }

    let job = match tool.name {
        "terminal_exec" => {
            JobKind::Process(requests::build_terminal_exec_invocation(arguments, config)?)
        }
        "ssh_readonly_exec" => JobKind::Process(ssh::build_invocation(arguments, config)?),
        "http_fetch" => JobKind::Process(requests::build_http_fetch_invocation(arguments)?),
        "web_search" => JobKind::Process(requests::build_web_search_invocation(arguments)?),
        _ => return Ok(ToolCallResult::not_implemented(tool.name)),
    };
    let id = manager.start(job).await?;
    let mut cancel_on_drop = CancelJobOnDrop {
        manager: manager.clone(),
        id: id.clone(),
        active: true,
    };
    let snapshot = manager.wait(&id).await?;
    cancel_on_drop.active = false;
    Ok(snapshot.result.unwrap_or_else(|| {
        ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: "Tool execution failed".into(),
        }])
    }))
}

struct CancelJobOnDrop {
    manager: Arc<JobManager>,
    id: String,
    active: bool,
}

impl Drop for CancelJobOnDrop {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let manager = Arc::clone(&self.manager);
        let id = self.id.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                manager.request_cancel_internal(&id).await;
            });
        }
    }
}
