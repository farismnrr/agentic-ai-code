use super::{
    is_active, is_finished, now_ms, process, request_cancel, JobManager, JobSnapshot, JobState,
};
use crate::core::error::McpError;
use crate::interfaces::mcp::ToolCallResult;
use std::time::Instant;
use tokio::time::{timeout, Duration};

impl JobManager {
    /// Protected-path indexes are initialized on demand from the exact trees
    /// selected for an invocation. Serving startup intentionally performs no
    /// filesystem priming so unrelated workspace/toolchain roots cannot
    /// contend with the first terminal request.
    pub async fn prepare_for_serving(&self) {}

    pub async fn wait(&self, id: &str) -> Result<JobSnapshot, McpError> {
        let (timeout_ms, deadline) = {
            let jobs = self.jobs.lock().await;
            jobs.get(id)
                .map(|j| (j.timeout_ms, j.deadline))
                .unwrap_or((0, None))
        };
        let effective = if timeout_ms > 0 {
            timeout_ms
        } else {
            self.config.max_terminal_timeout_ms
        };
        let wait_start = Instant::now();
        let deadline = deadline
            .or_else(|| (effective > 0).then(|| wait_start + Duration::from_millis(effective)));

        loop {
            let snapshot = self
                .get_internal(id)
                .await
                .ok_or_else(|| McpError::Internal("execution job disappeared".into()))?;
            if is_finished(&snapshot) {
                tracing::info!(
                    event = "relay.process.stage",
                    stage = "job_wait",
                    outcome = "completed",
                    state = ?snapshot.state,
                    duration_ms = wait_start.elapsed().as_millis() as u64,
                );
                return Ok(snapshot);
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                tracing::warn!(
                    event = "relay.process.stage",
                    stage = "job_wait",
                    outcome = "deadline_elapsed",
                    duration_ms = wait_start.elapsed().as_millis() as u64,
                );
                self.request_timeout_internal(id).await;
                let cleanup_deadline =
                    Instant::now() + Duration::from_millis(process::PROCESS_CLEANUP_GRACE_MS);
                loop {
                    let snapshot = self
                        .get_internal(id)
                        .await
                        .ok_or_else(|| McpError::Internal("execution job disappeared".into()))?;
                    if is_finished(&snapshot) {
                        tracing::info!(
                            event = "relay.process.stage",
                            stage = "job_wait",
                            outcome = "completed_after_cancel",
                            state = ?snapshot.state,
                            duration_ms = wait_start.elapsed().as_millis() as u64,
                        );
                        return Ok(snapshot);
                    }
                    if Instant::now() >= cleanup_deadline {
                        tracing::error!(
                            event = "relay.process.stage",
                            stage = "job_wait",
                            outcome = "cleanup_grace_exhausted",
                            duration_ms = wait_start.elapsed().as_millis() as u64,
                        );
                        return Ok(self.abort_stalled_job(id).await.unwrap_or(snapshot));
                    }
                    let sleep_for = cleanup_deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(10));
                    if !sleep_for.is_zero() {
                        tokio::time::sleep(sleep_for).await;
                    }
                }
            }
            let sleep_for = deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|| Duration::from_millis(10))
                .min(Duration::from_millis(10));
            if !sleep_for.is_zero() {
                tokio::time::sleep(sleep_for).await;
            }
        }
    }

    pub async fn active_jobs_count(&self) -> usize {
        let jobs = self.jobs.lock().await;
        jobs.values().filter(|j| is_active(&j.snapshot)).count()
    }

    pub(crate) async fn request_cancel_internal(&self, id: &str) {
        if let Some(job) = self.jobs.lock().await.get_mut(id) {
            request_cancel(job);
            tracing::info!(
                event = "relay.process.stage",
                stage = "request_cancel",
                outcome = if job.snapshot.state == JobState::Cancelled {
                    "requested"
                } else {
                    "already_finished"
                },
            );
        }
    }

    async fn request_timeout_internal(&self, id: &str) {
        if let Some(job) = self.jobs.lock().await.get_mut(id) {
            if matches!(job.snapshot.state, JobState::Queued | JobState::Running) {
                let _ = job.cancel.send(true);
                job.snapshot.state = JobState::TimedOut;
                job.snapshot.last_updated_at = now_ms();
                tracing::warn!(
                    event = "relay.process.stage",
                    stage = "job_timeout",
                    outcome = "requested",
                    error_kind = ?std::io::ErrorKind::TimedOut,
                );
            }
        }
    }

    pub(in crate::application::execution) async fn register_child_pid(
        &self,
        id: &str,
        pid: Option<u32>,
    ) {
        if let Some(job) = self.jobs.lock().await.get_mut(id) {
            job.child_pid = pid;
        }
    }

    async fn abort_stalled_job(&self, id: &str) -> Option<JobSnapshot> {
        let (snapshot, abort_handle, child_pid) = {
            let mut jobs = self.jobs.lock().await;
            let job = jobs.get_mut(id)?;
            if job.snapshot.finished_at.is_some() {
                return Some(job.snapshot.clone());
            }
            let _ = job.cancel.send(true);
            let now = now_ms();
            job.snapshot.state = JobState::TimedOut;
            job.snapshot.finished_at = Some(now);
            job.snapshot.last_updated_at = now;
            job.snapshot.exit_code = Some(-1);
            job.snapshot.result = Some(ToolCallResult::error(vec![
                crate::interfaces::mcp::ToolResultContent {
                    kind: "text",
                    text: "terminal execution failed at job_wait: TimedOut".into(),
                },
            ]));
            (job.snapshot.clone(), job.task_abort.clone(), job.child_pid)
        };
        tracing::error!(
            event = "relay.process.failed",
            stage = "job_wait",
            error_kind = ?std::io::ErrorKind::TimedOut,
        );
        process::kill_process_group_by_pid(child_pid).await;
        if let Some(abort_handle) = abort_handle {
            abort_handle.abort();
        }
        tokio::task::yield_now().await;
        Some(snapshot)
    }

    pub async fn shutdown(&self) {
        let active_ids = {
            let jobs = self.jobs.lock().await;
            jobs.iter()
                .filter_map(|(id, job)| {
                    if is_active(&job.snapshot) {
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
}
