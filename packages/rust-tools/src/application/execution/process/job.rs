use super::{
    finish, run_process, update_state, JobManager, JobState, OutputBuffer, ProcessFailure,
    ProcessOutput, ProcessTimeouts,
};
use crate::application::execution::jobs::JobKind;
use crate::application::execution::now_ms;
use crate::core::config::ServerConfig;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use std::io;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{watch, Mutex};
use tokio::time::{timeout, Duration};

pub(in crate::application::execution) async fn run_job(
    manager: Arc<JobManager>,
    id: String,
    job: JobKind,
    mut cancel: watch::Receiver<bool>,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
) {
    let job_started = Instant::now();
    let timeout_ms = match &job {
        JobKind::Process(inv) => effective_timeout(&manager.config, inv.timeout_ms),
    };
    let preparation_deadline =
        Some(job_started + Duration::from_millis(super::PROCESS_PREPARATION_TIMEOUT_MS));
    let semaphore = manager.semaphore.clone();
    let semaphore_started = Instant::now();
    let permit = tokio::select! {
        biased;
        result = async {
            match preparation_deadline {
                Some(deadline) => timeout(
                    deadline.saturating_duration_since(Instant::now()),
                    semaphore.acquire_owned(),
                )
                .await
                .map_err(|_| ProcessFailure::new("semaphore_acquire", io::ErrorKind::TimedOut))?
                .map_err(|_| ProcessFailure::new("semaphore_acquire", io::ErrorKind::BrokenPipe)),
                None => semaphore
                    .acquire_owned()
                    .await
                    .map_err(|_| ProcessFailure::new("semaphore_acquire", io::ErrorKind::BrokenPipe)),
            }
        }
        => result,
        _ = cancel.changed() => {
            finish_failure(
                &manager,
                &id,
                JobState::Cancelled,
                ProcessFailure::new("semaphore_acquire", io::ErrorKind::Interrupted),
                job_started.elapsed().as_millis() as u64,
            )
            .await;
            return;
        }
    };
    tracing::info!(
        event = "relay.process.stage",
        stage = "semaphore_acquire",
        outcome = if permit.is_ok() {
            "completed"
        } else {
            "failed"
        },
        error_kind = ?permit.as_ref().err().map(|failure| failure.kind),
        duration_ms = semaphore_started.elapsed().as_millis() as u64,
    );
    let _permit = match permit {
        Ok(permit) => permit,
        Err(failure) => {
            let state = if failure.kind == io::ErrorKind::TimedOut {
                JobState::TimedOut
            } else {
                JobState::Failed
            };
            finish_failure(
                &manager,
                &id,
                state,
                failure,
                job_started.elapsed().as_millis() as u64,
            )
            .await;
            return;
        }
    };
    if *cancel.borrow() {
        finish_failure(
            &manager,
            &id,
            JobState::Cancelled,
            ProcessFailure::new("semaphore_acquire", io::ErrorKind::Interrupted),
            job_started.elapsed().as_millis() as u64,
        )
        .await;
        return;
    }
    update_state(&manager, &id, JobState::Running, Some(now_ms()), None).await;
    let result = match job {
        JobKind::Process(invocation) => {
            run_process(
                &manager,
                &id,
                &manager.config,
                &invocation,
                &mut cancel,
                ProcessOutput { stdout, stderr },
                ProcessTimeouts {
                    preparation_deadline,
                    command_timeout_ms: timeout_ms,
                },
            )
            .await
        }
    };
    let execution_duration_ms = job_started.elapsed().as_millis() as u64;
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
                event = "relay.process.failed",
                stage = error.stage,
                error_kind = ?error.kind,
                duration_ms = execution_duration_ms,
            );
            let state = match error.kind {
                io::ErrorKind::Interrupted => JobState::Cancelled,
                io::ErrorKind::TimedOut => JobState::TimedOut,
                _ => JobState::Failed,
            };
            finish_failure(&manager, &id, state, error, execution_duration_ms).await;
        }
    }
}

async fn finish_failure(
    manager: &JobManager,
    id: &str,
    state: JobState,
    failure: ProcessFailure,
    execution_duration_ms: u64,
) {
    let diagnostic = failure.diagnostic();
    finish(
        manager,
        id,
        state,
        -1,
        (String::new(), String::new(), 0),
        Some(execution_duration_ms),
        Some(ToolCallResult::error(vec![ToolResultContent {
            kind: "text",
            text: diagnostic,
        }])),
    )
    .await;
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
