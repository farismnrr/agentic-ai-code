//! Bounded execution lifecycle and application tool dispatch.
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::{watch, Mutex, Semaphore};
use tokio::time::{timeout, Duration};
use uuid::Uuid;
pub mod agent;
mod agent_capabilities;
mod agent_policy;
mod dispatch;
mod paths;
mod process;
mod requests;
pub(crate) mod sandbox;
mod toolchain;
pub(crate) use process::kill_process_group;
const MAX_RETAINED_JOBS: usize = 64;
#[derive(Clone)]
enum InvocationProgram {
    SelfBinary,
    Direct(PathBuf),
}
#[derive(Clone)]
struct ToolInvocation {
    program: InvocationProgram,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    timeout_ms: u64,
    allow_network: bool,
    environment: Vec<(String, String)>,
    auth_roots: Vec<PathBuf>,
    expose_optional_sockets: bool,
}

enum JobKind {
    Process(ToolInvocation),
    Agent(agent::AgentRequest),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}
impl JobState {
    pub fn task_status(self) -> &'static str {
        match self {
            Self::Queued | Self::Running => "working",
            Self::Completed | Self::TimedOut => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub job_id: String,
    pub state: JobState,
    pub created_at: u128,
    pub last_updated_at: u128,
    pub started_at: Option<u128>,
    pub finished_at: Option<u128>,
    pub execution_duration_ms: Option<u64>,
    pub stdout: String,
    pub stderr: String,
    pub omitted_bytes: u64,
    pub exit_code: Option<i32>,
    pub result: Option<ToolCallResult>,
}

impl JobSnapshot {
    pub fn create_task_json(&self) -> Value {
        json!({
            "resultType": "task",
            "taskId": self.job_id,
            "status": self.task_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "ttlMs": Value::Null,
            "pollIntervalMs": 1000
        })
    }

    pub fn task_json(&self, completed_retention_ms: u64) -> Value {
        let ttl_ms = self.finished_at.map(|finished_at| {
            let lifetime = finished_at.saturating_sub(self.created_at);
            u64::try_from(lifetime)
                .unwrap_or(u64::MAX)
                .saturating_add(completed_retention_ms)
        });
        let mut value = json!({
            "resultType": "complete",
            "taskId": self.job_id,
            "status": self.task_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "ttlMs": ttl_ms,
            "pollIntervalMs": 1000
        });
        if let Some(duration_ms) = self.execution_duration_ms {
            value["executionDurationMs"] = json!(duration_ms);
        }
        match self.state {
            JobState::Completed | JobState::TimedOut => {
                if let Some(result) = &self.result {
                    value["result"] = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
                }
            }
            JobState::Failed => {
                value["error"] = json!({
                    "code": -32603,
                    "message": "Tool execution failed"
                });
            }
            JobState::Queued | JobState::Running | JobState::Cancelled => {}
        }
        value
    }

    pub fn job_json(&self) -> Value {
        let mut value = json!({
            "taskId": self.job_id,
            "status": self.job_status(),
            "createdAt": format_timestamp(self.created_at),
            "lastUpdatedAt": format_timestamp(self.last_updated_at),
            "output": {
                "stdout": self.stdout,
                "stderr": self.stderr,
                "omittedBytes": self.omitted_bytes,
                "exitCode": self.exit_code
            }
        });
        if let Some(duration_ms) = self.execution_duration_ms {
            value["executionDurationMs"] = json!(duration_ms);
        }
        if let Some(result) = &self.result {
            value["result"] = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
        }
        value
    }

    fn task_status(&self) -> &'static str {
        self.state.task_status()
    }

    fn job_status(&self) -> &'static str {
        match self.state {
            JobState::Queued => "queued",
            JobState::Running => "working",
            JobState::Completed => "completed",
            JobState::Failed => "failed",
            JobState::TimedOut => "timed_out",
            JobState::Cancelled => "cancelled",
        }
    }

    pub fn output_text(&self) -> String {
        render_output(
            self.exit_code.unwrap_or(-1),
            &self.stdout,
            &self.stderr,
            self.omitted_bytes,
        )
    }
}

struct JobRecord {
    snapshot: JobSnapshot,
    cancel: watch::Sender<bool>,
    stdout: Arc<Mutex<process::OutputBuffer>>,
    stderr: Arc<Mutex<process::OutputBuffer>>,
}

pub struct JobManager {
    jobs: Mutex<HashMap<String, JobRecord>>,
    semaphore: Arc<Semaphore>,
    config: ServerConfig,
}

impl JobManager {
    pub fn new(config: ServerConfig) -> Arc<Self> {
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(config.max_running_jobs)),
            config,
        })
    }

    async fn start(self: &Arc<Self>, job: JobKind) -> Result<String, McpError> {
        self.expire_completed().await;
        let mut jobs = self.jobs.lock().await;
        let mut completed = jobs
            .iter()
            .filter_map(|(id, job)| {
                if matches!(
                    job.snapshot.state,
                    JobState::Completed
                        | JobState::Failed
                        | JobState::TimedOut
                        | JobState::Cancelled
                ) {
                    Some((id.clone(), job.snapshot.finished_at.unwrap_or(u128::MAX)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if completed.len() >= MAX_RETAINED_JOBS {
            completed.sort_by_key(|(_, finished_at)| *finished_at);
            let remove_count = completed.len() - MAX_RETAINED_JOBS + 1;
            for (id, _) in completed.into_iter().take(remove_count) {
                jobs.remove(&id);
            }
        }
        if jobs.len() >= self.config.max_running_jobs + MAX_RETAINED_JOBS {
            return Err(McpError::InvalidRequest(
                "execution job capacity reached".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let (cancel, receiver) = watch::channel(false);
        let created_at = now_ms();
        let stdout = Arc::new(Mutex::new(process::OutputBuffer::new(created_at)));
        let stderr = Arc::new(Mutex::new(process::OutputBuffer::new(created_at)));
        let snapshot = JobSnapshot {
            job_id: id.clone(),
            state: JobState::Queued,
            created_at,
            last_updated_at: created_at,
            started_at: None,
            finished_at: None,
            execution_duration_ms: None,
            stdout: String::new(),
            stderr: String::new(),
            omitted_bytes: 0,
            exit_code: None,
            result: None,
        };
        jobs.insert(
            id.clone(),
            JobRecord {
                snapshot,
                cancel: cancel.clone(),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            },
        );
        drop(jobs);
        let manager = Arc::clone(self);
        let job_id = id.clone();
        tokio::spawn(async move {
            process::run_job(manager, job_id, job, receiver, stdout, stderr).await;
        });
        Ok(id)
    }

    pub async fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.expire_completed().await;
        let (mut snapshot, stdout, stderr) = {
            let jobs = self.jobs.lock().await;
            let job = jobs.get(id)?;
            (job.snapshot.clone(), job.stdout.clone(), job.stderr.clone())
        };
        let out = stdout.lock().await;
        let err = stderr.lock().await;
        snapshot.stdout = String::from_utf8_lossy(&out.bytes).into_owned();
        snapshot.stderr = String::from_utf8_lossy(&err.bytes).into_owned();
        snapshot.omitted_bytes = out.omitted + err.omitted;
        snapshot.last_updated_at = snapshot
            .last_updated_at
            .max(out.updated_at)
            .max(err.updated_at);
        Some(snapshot)
    }

    pub async fn cancel(&self, id: &str) -> Result<JobSnapshot, McpError> {
        let jobs = self.jobs.lock().await;
        let job = jobs
            .get(id)
            .ok_or_else(|| McpError::InvalidParams("unknown task".into()))?;
        if !matches!(
            job.snapshot.state,
            JobState::Completed | JobState::Failed | JobState::TimedOut | JobState::Cancelled
        ) {
            let _ = job.cancel.send(true);
        }
        Ok(job.snapshot.clone())
    }

    pub async fn wait(&self, id: &str) -> Result<JobSnapshot, McpError> {
        loop {
            let snapshot = self
                .get(id)
                .await
                .ok_or_else(|| McpError::Internal("execution job disappeared".into()))?;
            if matches!(
                snapshot.state,
                JobState::Completed | JobState::Failed | JobState::TimedOut | JobState::Cancelled
            ) {
                return Ok(snapshot);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub async fn shutdown(&self) {
        let active_ids = {
            let jobs = self.jobs.lock().await;
            jobs.iter()
                .filter_map(|(id, job)| {
                    if matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
                        let _ = job.cancel.send(true);
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        for id in active_ids {
            let _ = timeout(Duration::from_secs(5), self.wait(&id)).await;
        }
    }

    async fn expire_completed(&self) {
        let cutoff = now_ms().saturating_sub(self.config.completed_job_ttl_ms as u128);
        self.jobs.lock().await.retain(|_, job| {
            !matches!(
                job.snapshot.state,
                JobState::Completed | JobState::Failed | JobState::TimedOut | JobState::Cancelled
            ) || job.snapshot.finished_at.unwrap_or(u128::MAX) > cutoff
        });
    }
}

fn render_output(exit_code: i32, stdout: &str, stderr: &str, omitted: u64) -> String {
    let omitted_note = if omitted > 0 {
        format!("\n... {omitted} earlier output bytes omitted ...")
    } else {
        String::new()
    };
    format!("Exit: {exit_code}\nStdout: {stdout}\nStderr: {stderr}{omitted_note}")
}

fn format_timestamp(milliseconds: u128) -> String {
    let nanos = milliseconds
        .saturating_mul(1_000_000)
        .min(i128::MAX as u128) as i128;
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|timestamp| timestamp.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub fn tool_call_supports_tasks(tool: &Tool, arguments: &Value) -> bool {
    dispatch::supports_tasks(tool, arguments)
}

pub async fn start_tool_task(
    tool: &Tool,
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
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
        "agent_delegate" => JobKind::Agent(agent::build_request(arguments, config)?),
        _ => {
            return Err(McpError::InvalidRequest(
                "tool task execution is not implemented".into(),
            ))
        }
    };
    manager.start(job).await
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
        "agent_delegate" => JobKind::Agent(agent::build_request(arguments, config)?),
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
