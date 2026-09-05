//! Job execution and the single sandboxed process lifecycle.

use super::sandbox;
use super::{now_ms, render_output, JobKind, JobManager, JobState, ToolInvocation};
use crate::core::config::ServerConfig;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio::sync::{watch, Mutex};
use tokio::time::{timeout, Duration};

const TIMEOUT_GRACE_MS: u64 = 5_000;

pub(super) async fn run_job(
    manager: Arc<JobManager>,
    id: String,
    job: JobKind,
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
                (String::new(), String::new(), 0),
                None,
                None,
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
            (String::new(), String::new(), 0),
            None,
            None,
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
            ("execution semaphore unavailable".into(), String::new(), 0),
            None,
            None,
        )
        .await;
        return;
    };
    let execution_started = Instant::now();
    update_state(&manager, &id, JobState::Running, Some(now_ms()), None).await;
    let result = match job {
        JobKind::Process(invocation) => {
            run_process(&manager.config, &invocation, &mut cancel, stdout, stderr).await
        }
    };
    let execution_duration_ms = execution_started.elapsed().as_millis() as u64;
    match result {
        Ok(process) => {
            finish(
                &manager,
                &id,
                process.state,
                process.exit_code,
                (process.stdout, process.stderr, process.omitted),
                Some(execution_duration_ms),
                process.result,
            )
            .await
        }
        Err(error) => {
            tracing::warn!(
                event = "relay.process.spawn_failed",
                job_id = %id,
                error = %error,
                "process spawn or execution failed"
            );
            finish(
                &manager,
                &id,
                JobState::Failed,
                -1,
                (String::new(), String::new(), 0),
                Some(execution_duration_ms),
                None,
            )
            .await
        }
    }
}

pub(super) struct ProcessResult {
    pub(super) state: JobState,
    pub(super) exit_code: i32,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) omitted: u64,
    pub(super) result: Option<ToolCallResult>,
}

pub(super) struct OutputBuffer {
    pub(super) bytes: Vec<u8>,
    pub(super) omitted: u64,
    pub(super) updated_at: u128,
}

impl OutputBuffer {
    pub(super) fn new(updated_at: u128) -> Self {
        Self {
            bytes: Vec::new(),
            omitted: 0,
            updated_at,
        }
    }

    pub(super) fn push(&mut self, chunk: &[u8], limit: usize) {
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

pub(super) async fn drain_pipe<R: tokio::io::AsyncRead + Unpin>(
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

pub(super) async fn run_process(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    cancel: &mut watch::Receiver<bool>,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
) -> Result<ProcessResult, std::io::Error> {
    let mut child = sandbox::spawn(config, invocation, sandbox::WorkspaceAccess::Writable)?;
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
    let exit_code = wait_result.0.code().unwrap_or(-1);
    let mut stdout_text = String::from_utf8_lossy(&out.bytes).into_owned();
    let mut stderr_text = String::from_utf8_lossy(&err.bytes).into_owned();
    stdout_text = crate::core::redaction::redact_credentials(&stdout_text);
    stderr_text = crate::core::redaction::redact_credentials(&stderr_text);
    if matches!(invocation.security, super::InvocationSecurity::Ssh { .. }) && exit_code != 0 {
        if let Some(message) = super::ssh::normalized_failure(&stderr_text) {
            stdout_text.clear();
            stderr_text = message.into();
        }
    }
    Ok(ProcessResult {
        state: wait_result.1,
        exit_code,
        stdout: stdout_text,
        stderr: stderr_text,
        omitted: out.omitted + err.omitted,
        result: None,
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

pub(crate) async fn kill_process_group(child: &mut Child) {
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
    output: (String, String, u64),
    execution_duration_ms: Option<u64>,
    result_override: Option<ToolCallResult>,
) {
    let (stdout, stderr, omitted) = output;
    let result = result_override.or_else(|| match state {
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
    });
    if let Some(job) = manager.jobs.lock().await.get_mut(id) {
        let finished_at = now_ms();
        // Transport cancellation commits the public state before process
        // reaping completes. A later timeout or exit must not overwrite it.
        if job.snapshot.state != JobState::Cancelled {
            job.snapshot.state = state;
        }
        job.snapshot.finished_at = Some(finished_at);
        job.snapshot.execution_duration_ms = execution_duration_ms;
        job.snapshot.last_updated_at = finished_at;
        job.snapshot.exit_code = Some(exit_code);
        job.snapshot.stdout = stdout;
        job.snapshot.stderr = stderr;
        job.snapshot.omitted_bytes = omitted;
        job.snapshot.result = result;
    }
}
