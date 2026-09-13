//! Job execution and the single sandboxed process lifecycle.

use super::sandbox;
use super::{now_ms, render_output, JobManager, JobState, ToolInvocation};
use crate::core::config::ServerConfig;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use std::io;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::{watch, Mutex};
use tokio::time::{timeout, timeout_at, Duration, Instant as TokioInstant};

mod job;
mod output;
pub(super) use job::run_job;
pub(super) use output::drain_pipe;
pub(super) use output::OutputBuffer;

pub(crate) const PROCESS_CLEANUP_GRACE_MS: u64 = 1_500;
/// Sandbox discovery and process setup are bounded separately from the
/// caller's command runtime timeout.
pub(crate) const PROCESS_PREPARATION_TIMEOUT_MS: u64 = 120_000;
const PROCESS_REAP_GRACE_MS: u64 = 500;
const PIPE_DRAIN_GRACE_MS: u64 = 500;

#[derive(Debug)]
pub(super) struct ProcessFailure {
    pub(super) stage: &'static str,
    pub(super) kind: io::ErrorKind,
}

impl ProcessFailure {
    fn new(stage: &'static str, kind: io::ErrorKind) -> Self {
        Self { stage, kind }
    }

    fn from_io(stage: &'static str, error: io::Error) -> Self {
        Self::new(stage, error.kind())
    }

    fn diagnostic(&self) -> String {
        format!(
            "terminal execution failed at {}: {:?}",
            self.stage, self.kind
        )
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

pub(super) struct ProcessOutput {
    pub(super) stdout: Arc<Mutex<OutputBuffer>>,
    pub(super) stderr: Arc<Mutex<OutputBuffer>>,
}

pub(super) struct ProcessTimeouts {
    pub(super) preparation_deadline: Option<Instant>,
    pub(super) command_timeout_ms: u64,
}

pub(super) async fn run_process(
    manager: &JobManager,
    id: &str,
    config: &ServerConfig,
    invocation: &ToolInvocation,
    cancel: &mut watch::Receiver<bool>,
    output: ProcessOutput,
    timeouts: ProcessTimeouts,
) -> Result<ProcessResult, ProcessFailure> {
    let ProcessOutput { stdout, stderr } = output;
    let spawn_started = Instant::now();
    let mut child = sandbox::spawn(
        config,
        invocation,
        sandbox::WorkspaceAccess::Writable,
        timeouts.preparation_deadline,
        cancel,
    )
    .map_err(|error| ProcessFailure::new(error.stage, error.kind()))?;
    let child_pid = child.id();
    manager.register_child_pid(id, child_pid).await;
    tracing::info!(
        event = "relay.process.stage",
        stage = "child_start",
        outcome = "completed",
        duration_ms = spawn_started.elapsed().as_millis() as u64,
    );

    // timeout_ms is the requested command runtime. Starting it before sandbox
    // discovery made cold protected-path scans consume the command's budget.
    let command_deadline = (timeouts.command_timeout_ms > 0)
        .then(|| Instant::now() + Duration::from_millis(timeouts.command_timeout_ms));

    // Close stdin immediately so non-interactive commands observe EOF.
    drop(child.stdin.take());
    let Some(stdout_pipe) = child.stdout.take() else {
        let failure = ProcessFailure::new("stdout_pipe_setup", io::ErrorKind::BrokenPipe);
        return match kill_and_reap(&mut child, child_pid).await {
            Ok(_) => Err(failure),
            Err(cleanup_failure) => Err(cleanup_failure),
        };
    };
    let Some(stderr_pipe) = child.stderr.take() else {
        let failure = ProcessFailure::new("stderr_pipe_setup", io::ErrorKind::BrokenPipe);
        return match kill_and_reap(&mut child, child_pid).await {
            Ok(_) => Err(failure),
            Err(cleanup_failure) => Err(cleanup_failure),
        };
    };

    let mut out_task = tokio::spawn(drain_pipe(
        stdout_pipe,
        stdout.clone(),
        config.max_retained_output_bytes / 2,
    ));
    let mut err_task = tokio::spawn(drain_pipe(
        stderr_pipe,
        stderr.clone(),
        config.max_retained_output_bytes / 2,
    ));
    let wait_started = Instant::now();
    let wait_result = if let Some(deadline) = command_deadline {
        tokio::select! {
            biased;
            result = timeout_at(TokioInstant::from_std(deadline), child.wait()) => match result {
                Ok(Ok(status)) => Ok((status, JobState::Completed)),
                Ok(Err(error)) => {
                    tracing::warn!(
                        event = "relay.process.stage",
                        stage = "child_wait",
                        outcome = "failed",
                        error_kind = ?error.kind(),
                    );
                    let failure = ProcessFailure::from_io("child_wait", error);
                    match kill_and_reap(&mut child, child_pid).await {
                        Ok(_) => Err(failure),
                        Err(cleanup_failure) => Err(cleanup_failure),
                    }
                }
                Err(_) => {
                    tracing::warn!(
                        event = "relay.process.stage",
                        stage = "timeout_path",
                        outcome = "timed_out",
                    );
                    let status = kill_and_reap(&mut child, child_pid).await?;
                    Ok((status, JobState::TimedOut))
                }
            },
            _ = cancel.changed() => {
                tracing::info!(
                    event = "relay.process.stage",
                    stage = "cancellation_path",
                    outcome = "requested",
                );
                let status = kill_and_reap(&mut child, child_pid).await?;
                Ok((status, JobState::Cancelled))
            }
        }
    } else {
        tokio::select! {
            result = child.wait() => match result {
                Ok(status) => Ok((status, JobState::Completed)),
                Err(error) => {
                    tracing::warn!(
                        event = "relay.process.stage",
                        stage = "child_wait",
                        outcome = "failed",
                        error_kind = ?error.kind(),
                    );
                    let failure = ProcessFailure::from_io("child_wait", error);
                    match kill_and_reap(&mut child, child_pid).await {
                        Ok(_) => Err(failure),
                        Err(cleanup_failure) => Err(cleanup_failure),
                    }
                }
            },
            _ = cancel.changed() => {
                tracing::info!(
                    event = "relay.process.stage",
                    stage = "cancellation_path",
                    outcome = "requested",
                );
                let status = kill_and_reap(&mut child, child_pid).await?;
                Ok((status, JobState::Cancelled))
            }
        }
    }?;
    tracing::info!(
        event = "relay.process.stage",
        stage = "child_wait",
        outcome = match wait_result.1 {
            JobState::Completed => "completed",
            JobState::TimedOut => "timed_out",
            JobState::Cancelled => "cancelled",
            _ => "failed",
        },
        duration_ms = wait_started.elapsed().as_millis() as u64,
    );

    let drain_started = Instant::now();
    let drain_result = timeout(Duration::from_millis(PIPE_DRAIN_GRACE_MS), async {
        let (outcome, error) = tokio::join!(&mut out_task, &mut err_task);
        outcome
            .map_err(|_| ProcessFailure::new("stdout_drain", io::ErrorKind::Other))?
            .map_err(|error| ProcessFailure::from_io("stdout_drain", error))?;
        error
            .map_err(|_| ProcessFailure::new("stderr_drain", io::ErrorKind::Other))?
            .map_err(|error| ProcessFailure::from_io("stderr_drain", error))?;
        Ok::<(), ProcessFailure>(())
    })
    .await;
    let drain_failure = match drain_result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(_) => Some(ProcessFailure::new(
            "stdout_stderr_drain",
            io::ErrorKind::TimedOut,
        )),
    };
    if let Some(failure) = drain_failure {
        kill_process_group_by_pid(child_pid).await;
        out_task.abort();
        err_task.abort();
        tracing::warn!(
            event = "relay.process.stage",
            stage = "stdout_stderr_drain",
            outcome = "failed",
            error_kind = ?failure.kind,
            duration_ms = drain_started.elapsed().as_millis() as u64,
        );
        return Err(failure);
    }
    tracing::info!(
        event = "relay.process.stage",
        stage = "stdout_stderr_drain",
        outcome = "completed",
        duration_ms = drain_started.elapsed().as_millis() as u64,
    );

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
        result: match wait_result.1 {
            JobState::TimedOut => Some(ToolCallResult::error(vec![ToolResultContent {
                kind: "text",
                text: "terminal execution failed at child_wait: TimedOut".into(),
            }])),
            JobState::Cancelled => Some(ToolCallResult::error(vec![ToolResultContent {
                kind: "text",
                text: "terminal execution failed at request_cancellation: Interrupted".into(),
            }])),
            _ => None,
        },
    })
}

pub(crate) async fn kill_process_group(child: &mut Child) {
    kill_process_group_by_pid(child.id()).await;
}

pub(crate) async fn kill_process_group_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        #[cfg(unix)]
        let outcome = unsafe {
            if libc::kill(-(pid as i32), libc::SIGKILL) == 0 {
                ("signalled", None)
            } else {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ESRCH) {
                    ("already_exited", None)
                } else {
                    ("failed", Some(error.kind()))
                }
            }
        };
        #[cfg(not(unix))]
        let outcome: (&str, Option<io::ErrorKind>) = ("unsupported", None);
        tracing::info!(
            event = "relay.process.stage",
            stage = "process_group_kill",
            outcome = outcome.0,
            error_kind = ?outcome.1,
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn kill_and_reap(
    child: &mut Child,
    child_pid: Option<u32>,
) -> Result<std::process::ExitStatus, ProcessFailure> {
    kill_process_group_by_pid(child_pid).await;
    timeout(Duration::from_millis(PROCESS_REAP_GRACE_MS), child.wait())
        .await
        .map_err(|_| ProcessFailure::new("child_reap", io::ErrorKind::TimedOut))?
        .map_err(|error| ProcessFailure::from_io("child_reap", error))
}

async fn update_state(
    manager: &JobManager,
    id: &str,
    state: JobState,
    started: Option<u128>,
    finished: Option<u128>,
) {
    if let Some(job) = manager.jobs.lock().await.get_mut(id) {
        if job.snapshot.finished_at.is_some() {
            return;
        }
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
    if let Some(job) = manager.jobs.lock().await.get_mut(id) {
        if job.snapshot.finished_at.is_some() {
            return;
        }
        let final_state = match job.snapshot.state {
            JobState::Cancelled | JobState::TimedOut => job.snapshot.state,
            _ => state,
        };
        let result_override = (state == final_state).then_some(result_override).flatten();
        let result = result_override.or_else(|| match final_state {
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
                text: "terminal execution failed at job_wait: TimedOut".into(),
            }])),
            JobState::Cancelled => Some(ToolCallResult::error(vec![ToolResultContent {
                kind: "text",
                text: "execution cancelled".into(),
            }])),
            JobState::Queued | JobState::Running | JobState::Failed => None,
        });
        let finished_at = now_ms();
        job.snapshot.state = final_state;
        job.snapshot.finished_at = Some(finished_at);
        job.snapshot.execution_duration_ms = execution_duration_ms;
        job.snapshot.last_updated_at = finished_at;
        job.snapshot.exit_code = Some(exit_code);
        job.snapshot.stdout = stdout;
        job.snapshot.stderr = stderr;
        job.snapshot.omitted_bytes = omitted;
        job.snapshot.result = result;
        job.child_pid = None;
        tracing::info!(
            event = "relay.process.stage",
            stage = "job_finish",
            outcome = match final_state {
                JobState::Completed => "completed",
                JobState::Failed => "failed",
                JobState::TimedOut => "timed_out",
                JobState::Cancelled => "cancelled",
                JobState::Queued | JobState::Running => "invalid",
            },
            duration_ms = execution_duration_ms.unwrap_or_default(),
        );
    }
}
