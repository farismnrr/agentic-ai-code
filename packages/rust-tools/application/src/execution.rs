//! One bounded execution lifecycle for synchronous calls, jobs, and tasks.

use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{Tool, ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{watch, Mutex, Semaphore};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const TIMEOUT_GRACE_MS: u64 = 5_000;
const MAX_EXEC_ARGS: usize = 100;
const MAX_EXEC_ARG_BYTES: usize = 64 * 1024;
const MAX_HTTP_HEADERS: usize = 100;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;
const MAX_RETAINED_JOBS: usize = 64;
const DEFAULT_TEXT_SEARCH_RESULTS: usize = 50;
const MAX_TEXT_SEARCH_RESULTS: usize = 100;
const MAX_TEXT_SEARCH_PREVIEW_BYTES: usize = 1024;
const MAX_TEXT_SEARCH_RESULT_BYTES: usize = 256 * 1024;
const MAX_TEXT_SEARCH_QUERY_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_GLOB_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_CWD_BYTES: usize = 4096;
const MAX_TEXT_SEARCH_STDERR_BYTES: usize = 8192;
const TEXT_SEARCH_MAX_COLUMNS: usize = 1024;

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
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
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

    async fn start(self: &Arc<Self>, invocation: ToolInvocation) -> Result<String, McpError> {
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
        let stdout = Arc::new(Mutex::new(OutputBuffer::new(created_at)));
        let stderr = Arc::new(Mutex::new(OutputBuffer::new(created_at)));
        let snapshot = JobSnapshot {
            job_id: id.clone(),
            state: JobState::Queued,
            created_at,
            last_updated_at: created_at,
            started_at: None,
            finished_at: None,
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
            run_job(manager, job_id, invocation, receiver, stdout, stderr).await;
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

async fn run_job(
    manager: Arc<JobManager>,
    id: String,
    invocation: ToolInvocation,
    mut cancel: watch::Receiver<bool>,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
) {
    let semaphore = manager.semaphore.clone();
    let permit = tokio::select! {
        permit = semaphore.acquire_owned() => permit,
        _ = cancel.changed() => {
            finish(
                &manager,
                &id,
                JobState::Cancelled,
                -1,
                String::new(),
                String::new(),
                0,
            )
            .await;
            return;
        }
    };
    if *cancel.borrow() {
        finish(
            &manager,
            &id,
            JobState::Cancelled,
            -1,
            String::new(),
            String::new(),
            0,
        )
        .await;
        return;
    }
    let Ok(_permit) = permit else {
        finish(
            &manager,
            &id,
            JobState::Failed,
            -1,
            "execution semaphore unavailable".into(),
            String::new(),
            0,
        )
        .await;
        return;
    };
    update_state(&manager, &id, JobState::Running, Some(now_ms()), None).await;
    let result = run_process(&manager.config, &invocation, &mut cancel, stdout, stderr).await;
    match result {
        Ok(process) => {
            finish(
                &manager,
                &id,
                process.state,
                process.exit_code,
                process.stdout,
                process.stderr,
                process.omitted,
            )
            .await
        }
        Err(_) => {
            finish(
                &manager,
                &id,
                JobState::Failed,
                -1,
                String::new(),
                String::new(),
                0,
            )
            .await
        }
    }
}

struct ProcessResult {
    state: JobState,
    exit_code: i32,
    stdout: String,
    stderr: String,
    omitted: u64,
}

struct OutputBuffer {
    bytes: Vec<u8>,
    omitted: u64,
    updated_at: u128,
}

impl OutputBuffer {
    fn new(updated_at: u128) -> Self {
        Self {
            bytes: Vec::new(),
            omitted: 0,
            updated_at,
        }
    }

    fn push(&mut self, chunk: &[u8], limit: usize) {
        self.updated_at = now_ms();
        if chunk.len() >= limit {
            self.omitted += (self.bytes.len() + chunk.len() - limit) as u64;
            self.bytes = chunk[chunk.len() - limit..].to_vec();
            return;
        }
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > limit {
            let drop_count = self.bytes.len() - limit;
            self.bytes.drain(..drop_count);
            self.omitted += drop_count as u64;
        }
    }
}

async fn drain_pipe<R: tokio::io::AsyncRead + Unpin>(
    mut pipe: R,
    output: Arc<Mutex<OutputBuffer>>,
    limit: usize,
) {
    let mut buf = [0u8; 8192];
    loop {
        match pipe.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => output.lock().await.push(&buf[..n], limit),
        }
    }
}

async fn run_process(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    cancel: &mut watch::Receiver<bool>,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
) -> Result<ProcessResult, std::io::Error> {
    let current_exe = env::current_exe()?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| std::io::Error::other("missing binary directory"))?;
    let program_path = match &invocation.program {
        InvocationProgram::SelfBinary => current_exe.clone(),
        InvocationProgram::Direct(path) => path.clone(),
    };
    if !program_path.exists() {
        return Err(std::io::Error::other("tool binary unavailable"));
    }
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| std::io::Error::other("invalid execution root"))?;
    let mut cmd = Command::new("bwrap");
    let root = execution_root.to_string_lossy().into_owned();
    let mut args = vec![
        "--ro-bind",
        "/usr",
        "/usr",
        "--ro-bind",
        "/lib",
        "/lib",
        "--ro-bind-try",
        "/lib64",
        "/lib64",
        "--ro-bind-try",
        "/etc",
        "/etc",
        "--ro-bind-try",
        "/bin",
        "/bin",
        "--ro-bind-try",
        "/sbin",
        "/sbin",
        "--ro-bind-try",
        "/opt",
        "/opt",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--bind",
        root.as_str(),
        root.as_str(),
        "--ro-bind",
        bin_dir.to_string_lossy().as_ref(),
        bin_dir.to_string_lossy().as_ref(),
        "--unshare-pid",
        "--die-with-parent",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    // Preserve developer toolchains only when the operator explicitly allows
    // their directories; never inherit the relay process PATH.
    for path in &config.toolchain_paths {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| std::io::Error::other("invalid toolchain path"))?;
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
    }
    if config.allow_docker {
        // Docker daemon access is an explicit local-development escape hatch:
        // possession of the daemon socket is effectively host-level authority.
        // Keep the default sandbox isolated and expose only the configured socket.
        let docker_socket = Path::new(&config.docker_socket);
        if !docker_socket.exists() {
            return Err(std::io::Error::other(format!(
                "Docker access enabled but socket '{}' is unavailable",
                docker_socket.display()
            )));
        }
        let socket = docker_socket.to_string_lossy().into_owned();
        args.extend(["--bind".into(), socket.clone(), socket]);
    }
    if config.allow_tailscale {
        // Tailscale local API access is opt-in and scoped to one Unix socket.
        let tailscale_socket = Path::new(&config.tailscale_socket);
        if !tailscale_socket.exists() {
            return Err(std::io::Error::other(format!(
                "Tailscale access enabled but socket '{}' is unavailable",
                tailscale_socket.display()
            )));
        }
        let socket = tailscale_socket.to_string_lossy().into_owned();
        args.extend(["--bind".into(), socket.clone(), socket]);
    }
    // Broader home scope must not expose common credential stores to commands.
    for relative in [".ssh", ".aws", ".config/gcloud", ".docker", ".kube"] {
        let path = execution_root.join(relative);
        if path.exists() {
            args.extend(["--tmpfs".into(), path.to_string_lossy().into_owned()]);
        }
    }
    for relative in [
        ".npmrc",
        ".netrc",
        ".pypirc",
        ".cargo/credentials",
        ".cargo/credentials.toml",
    ] {
        let path = execution_root.join(relative);
        if path.exists() {
            args.extend([
                "--ro-bind".into(),
                "/dev/null".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
    }
    if let Some(cwd) = &invocation.cwd {
        args.extend(["--chdir".into(), cwd.to_string_lossy().into_owned()]);
    }
    args.push(program_path.to_string_lossy().into_owned());
    args.extend(invocation.args.clone());
    cmd.args(args).env_clear().env("HOME", &root);
    let safe_path = safe_path_entries(config)
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");
    cmd.env("PATH", safe_path)
        .env("LANG", "C.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);
    let mut child = cmd.spawn()?;
    let out_task = tokio::spawn(drain_pipe(
        child.stdout.take().unwrap(),
        stdout.clone(),
        config.max_retained_output_bytes / 2,
    ));
    let err_task = tokio::spawn(drain_pipe(
        child.stderr.take().unwrap(),
        stderr.clone(),
        config.max_retained_output_bytes / 2,
    ));
    let deadline = effective_timeout(config, invocation.timeout_ms);
    let wait_result = if deadline == 0 {
        tokio::select! { result = child.wait() => result.map(|status| (status, JobState::Completed)), _ = cancel.changed() => { kill_process_group(&mut child).await; child.wait().await.map(|status| (status, JobState::Cancelled)) } }
    } else {
        tokio::select! {
            result = timeout(Duration::from_millis(deadline), child.wait()) => match result { Ok(status) => status.map(|s| (s, JobState::Completed)), Err(_) => { kill_process_group(&mut child).await; child.wait().await.map(|s| (s, JobState::TimedOut)) } },
            _ = cancel.changed() => { kill_process_group(&mut child).await; child.wait().await.map(|s| (s, JobState::Cancelled)) }
        }
    }?;
    let _ = out_task.await;
    let _ = err_task.await;
    let out = stdout.lock().await;
    let err = stderr.lock().await;
    Ok(ProcessResult {
        state: wait_result.1,
        exit_code: wait_result.0.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&err.bytes).into_owned(),
        omitted: out.omitted + err.omitted,
    })
}

fn effective_timeout(config: &ServerConfig, requested: u64) -> u64 {
    if config.max_terminal_timeout_ms == 0 {
        requested
    } else if requested == 0 {
        config.max_terminal_timeout_ms
    } else {
        requested.min(config.max_terminal_timeout_ms)
    }
}

async fn kill_process_group(child: &mut Child) {
    if let Some(pid) = child.id() {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
        tokio::time::sleep(Duration::from_millis(TIMEOUT_GRACE_MS.min(100))).await;
    }
}

async fn update_state(
    manager: &JobManager,
    id: &str,
    state: JobState,
    started: Option<u128>,
    finished: Option<u128>,
) {
    if let Some(job) = manager.jobs.lock().await.get_mut(id) {
        job.snapshot.state = state;
        job.snapshot.started_at = started;
        job.snapshot.finished_at = finished;
        job.snapshot.last_updated_at = now_ms();
    }
}

async fn finish(
    manager: &JobManager,
    id: &str,
    state: JobState,
    exit_code: i32,
    stdout: String,
    stderr: String,
    omitted: u64,
) {
    let result = match state {
        JobState::Completed if exit_code == 0 => {
            Some(ToolCallResult::complete(vec![ToolResultContent {
                kind: "text",
                text: render_output(exit_code, &stdout, &stderr, omitted),
            }]))
        }
        JobState::Completed => Some(ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: render_output(exit_code, &stdout, &stderr, omitted),
        }])),
        JobState::TimedOut => Some(ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: "execution timed out".into(),
        }])),
        JobState::Queued | JobState::Running | JobState::Failed | JobState::Cancelled => None,
    };
    if let Some(job) = manager.jobs.lock().await.get_mut(id) {
        let finished_at = now_ms();
        job.snapshot.state = state;
        job.snapshot.finished_at = Some(finished_at);
        job.snapshot.last_updated_at = finished_at;
        job.snapshot.exit_code = Some(exit_code);
        job.snapshot.stdout = stdout;
        job.snapshot.stderr = stderr;
        job.snapshot.omitted_bytes = omitted;
        job.snapshot.result = result;
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

fn safe_path_entries(config: &ServerConfig) -> Vec<PathBuf> {
    let mut entries = [
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    entries.extend(config.toolchain_paths.iter().map(PathBuf::from));
    entries
}

fn resolve_safe_executable(config: &ServerConfig, binary: &str) -> Result<PathBuf, McpError> {
    relay_core::terminal_policy::validate_executable(binary, config.allow_docker)?;
    for directory in safe_path_entries(config) {
        let candidate = directory.join(binary);
        if candidate.is_file() && is_executable(&candidate) {
            return Ok(candidate);
        }
    }
    Err(McpError::InvalidRequest(
        "command is not available in the configured safe PATH".into(),
    ))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn build_terminal_exec_invocation(
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
    if config.max_terminal_timeout_ms > 0 && timeout_ms > config.max_terminal_timeout_ms {
        return Err(McpError::InvalidRequest(
            "timeout_ms exceeds operator maximum".into(),
        ));
    }
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd = relay_core::terminal_policy::resolve_contained_cwd(
        &execution_root,
        arguments.get("cwd").and_then(Value::as_str),
    )?;
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
    })
}

#[derive(Debug, Serialize)]
struct TextSearchResult {
    matches: Vec<TextSearchMatch>,
    count: usize,
    truncated: bool,
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

fn build_text_search_command(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<(Command, usize), McpError> {
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
    let cwd = relay_core::terminal_policy::resolve_contained_cwd(&execution_root, cwd_arg)?;
    let rg = resolve_safe_executable(config, "rg")?;
    let bwrap = resolve_safe_executable(config, "bwrap")?;

    let root = execution_root.to_string_lossy().into_owned();
    let mut sandbox_args = vec![
        "--ro-bind".into(),
        "/usr".into(),
        "/usr".into(),
        "--ro-bind".into(),
        "/lib".into(),
        "/lib".into(),
        "--ro-bind-try".into(),
        "/lib64".into(),
        "/lib64".into(),
        "--ro-bind-try".into(),
        "/etc".into(),
        "/etc".into(),
        "--ro-bind-try".into(),
        "/bin".into(),
        "/bin".into(),
        "--ro-bind-try".into(),
        "/sbin".into(),
        "/sbin".into(),
        "--dev".into(),
        "/dev".into(),
        "--proc".into(),
        "/proc".into(),
        "--tmpfs".into(),
        "/tmp".into(),
        "--ro-bind".into(),
        root.clone(),
        root.clone(),
        "--unshare-pid".into(),
        "--die-with-parent".into(),
    ];
    for path in &config.toolchain_paths {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| McpError::InvalidRequest("configured toolchain path is invalid".into()))?;
        let value = canonical.to_string_lossy().into_owned();
        sandbox_args.extend(["--ro-bind".into(), value.clone(), value]);
    }
    sandbox_args.extend([
        "--chdir".into(),
        cwd.to_string_lossy().into_owned(),
        rg.to_string_lossy().into_owned(),
        "--json".into(),
        "--no-config".into(),
        "--no-messages".into(),
        "--max-columns".into(),
        TEXT_SEARCH_MAX_COLUMNS.to_string(),
        "--max-columns-preview".into(),
        "--sort".into(),
        "path".into(),
    ]);
    if !regex {
        sandbox_args.push("--fixed-strings".into());
    }
    if !case_sensitive {
        sandbox_args.push("--ignore-case".into());
    }
    if let Some(glob) = glob {
        sandbox_args.extend(["--glob".into(), glob.to_owned()]);
    }
    sandbox_args.extend(["--".into(), query.to_owned(), ".".into()]);

    let mut command = Command::new(bwrap);
    command
        .args(sandbox_args)
        .env_clear()
        .env("HOME", &root)
        .env("LANG", "C.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    Ok((command, max_results))
}

async fn run_text_search(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolCallResult, McpError> {
    let (mut command, max_results) = build_text_search_command(arguments, config)?;
    let mut child = command
        .spawn()
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
        if matches.len() >= max_results {
            truncated = true;
            kill_process_group(&mut child).await;
            break;
        }
        let data = &event["data"];
        let path = data["path"]["text"]
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("text search output is invalid".into()))?;
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
            path: path.strip_prefix("./").unwrap_or(path).to_owned(),
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

    let mut result = TextSearchResult {
        count: matches.len(),
        matches,
        truncated,
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

fn build_http_fetch_invocation(arguments: &Value) -> Result<ToolInvocation, McpError> {
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
    })
}

fn build_web_search_invocation(arguments: &Value) -> ToolInvocation {
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
    }
}

pub async fn start_terminal_job(
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
) -> Result<String, McpError> {
    manager
        .start(build_terminal_exec_invocation(arguments, config)?)
        .await
}

pub async fn dispatch_tool_call(
    tool: &Tool,
    arguments: &Value,
    config: &ServerConfig,
    manager: &Arc<JobManager>,
) -> Result<ToolCallResult, McpError> {
    if tool.name == "directory_list" {
        let result = crate::workspace::directory_list(arguments, config)?;
        let text = serde_json::to_string(&result)
            .map_err(|_| McpError::Internal("failed to serialize directory listing".into()))?;
        if text.len() > crate::workspace::MAX_DIRECTORY_RESULT_BYTES {
            return Err(McpError::InvalidRequest(
                "directory listing exceeds output maximum".into(),
            ));
        }
        return Ok(ToolCallResult::complete(vec![ToolResultContent {
            kind: "text",
            text,
        }]));
    }

    if tool.name == "file_search" {
        let result = crate::workspace::file_search(arguments, config)?;
        let text = serde_json::to_string(&result)
            .map_err(|_| McpError::Internal("failed to serialize file search result".into()))?;
        if text.len() > crate::workspace::MAX_FILE_SEARCH_RESULT_BYTES {
            return Err(McpError::InvalidRequest(
                "file search result exceeds output maximum".into(),
            ));
        }
        return Ok(ToolCallResult::complete(vec![ToolResultContent {
            kind: "text",
            text,
        }]));
    }

    if tool.name == "text_search" {
        return run_text_search(arguments, config).await;
    }

    let invocation = match tool.name {
        "terminal_exec" => build_terminal_exec_invocation(arguments, config)?,
        "http_fetch" => build_http_fetch_invocation(arguments)?,
        "web_search" => build_web_search_invocation(arguments),
        _ => return Ok(ToolCallResult::not_implemented(tool.name)),
    };
    let id = manager.start(invocation).await?;
    let snapshot = manager.wait(&id).await?;
    Ok(snapshot.result.unwrap_or_else(|| {
        ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: "Tool execution failed".into(),
        }])
    }))
}
