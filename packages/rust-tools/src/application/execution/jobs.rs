use super::{process, ToolInvocation};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::redaction::redact_credentials;
use crate::interfaces::mcp::ToolCallResult;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::{watch, Mutex, Semaphore};
use uuid::Uuid;

mod presentation;
mod wait;
const MAX_RETAINED_JOBS: usize = 64;
/// Per-owner admission guard keeps one authenticated client from consuming the
/// whole relay semaphore. The global operator limit remains authoritative too.
const MAX_RUNNING_JOBS_PER_OWNER: usize = 8;
pub(crate) enum JobKind {
    Process(ToolInvocation),
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
pub(crate) struct JobRecord {
    pub(crate) snapshot: JobSnapshot,
    pub(crate) owner: String,
    pub(crate) session: Option<String>,
    cancel: watch::Sender<bool>,
    stdout: Arc<Mutex<process::OutputBuffer>>,
    stderr: Arc<Mutex<process::OutputBuffer>>,
    timeout_ms: u64,
    task_abort: Option<tokio::task::AbortHandle>,
    pub(crate) child_pid: Option<u32>,
}
pub struct JobManager {
    pub(crate) jobs: Mutex<HashMap<String, JobRecord>>,
    idempotency: Mutex<HashMap<String, (String, String)>>,
    pub(super) semaphore: Arc<Semaphore>,
    pub(super) config: ServerConfig,
}

impl JobManager {
    pub fn new(config: ServerConfig) -> Arc<Self> {
        // Workspace protected-path indexes are demand-driven from the exact
        // sandbox root selected for an invocation. Do not prewarm broad
        // authorization roots here: a Projects-style root can monopolize the
        // single index worker and delay a much smaller repository request.
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            idempotency: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(config.max_running_jobs)),
            config,
        })
    }

    pub(super) async fn start(self: &Arc<Self>, job: JobKind) -> Result<String, McpError> {
        self.start_for(job, "local", None).await
    }

    pub(super) async fn start_for(
        self: &Arc<Self>,
        job: JobKind,
        owner: &str,
        session: Option<&str>,
    ) -> Result<String, McpError> {
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
                ) && job.snapshot.finished_at.is_some()
                {
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
        let owner_running = jobs
            .values()
            .filter(|record| record.owner == owner && is_active(&record.snapshot))
            .count();
        if owner_running >= MAX_RUNNING_JOBS_PER_OWNER {
            return Err(McpError::InvalidRequest(
                "execution owner concurrency limit reached".into(),
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
        let timeout_ms = match &job {
            JobKind::Process(inv) => inv.timeout_ms,
        };
        jobs.insert(
            id.clone(),
            JobRecord {
                snapshot,
                owner: owner.to_owned(),
                session: session.map(str::to_owned),
                cancel: cancel.clone(),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                timeout_ms,
                task_abort: None,
                child_pid: None,
            },
        );
        drop(jobs);
        let manager = Arc::clone(self);
        let job_id = id.clone();
        let task = tokio::spawn(async move {
            process::run_job(manager, job_id, job, receiver, stdout, stderr).await;
        });
        let abort_handle = task.abort_handle();
        drop(task);
        if let Some(record) = self.jobs.lock().await.get_mut(&id) {
            record.task_abort = Some(abort_handle);
        }
        Ok(id)
    }

    pub async fn existing_idempotency_key(
        &self,
        key: &str,
        fingerprint: &str,
    ) -> Result<Option<String>, McpError> {
        self.existing_idempotency_key_for(key, fingerprint, "local", None)
            .await
    }

    pub async fn existing_idempotency_key_for(
        &self,
        key: &str,
        fingerprint: &str,
        owner: &str,
        session: Option<&str>,
    ) -> Result<Option<String>, McpError> {
        let mut identities = self.idempotency.lock().await;
        let Some((job_id, original_fingerprint)) = identities.get(key).cloned() else {
            return Ok(None);
        };
        if original_fingerprint != fingerprint {
            return Err(McpError::InvalidRequest(
                "idempotency key was reused for different execution arguments".into(),
            ));
        }
        if self.jobs.lock().await.get(&job_id).is_some_and(|job| {
            job.owner == owner && session_matches(job.session.as_deref(), session)
        }) {
            Ok(Some(job_id))
        } else {
            identities.remove(key);
            Ok(None)
        }
    }

    pub async fn get(&self, id: &str) -> Option<JobSnapshot> {
        self.get_for(id, "local", None).await
    }

    pub async fn get_for(
        &self,
        id: &str,
        owner: &str,
        session: Option<&str>,
    ) -> Option<JobSnapshot> {
        self.expire_completed().await;
        let (mut snapshot, stdout, stderr) = {
            let jobs = self.jobs.lock().await;
            let job = jobs.get(id)?;
            if job.owner != owner || !session_matches(job.session.as_deref(), session) {
                return None;
            }
            (job.snapshot.clone(), job.stdout.clone(), job.stderr.clone())
        };
        let out = stdout.lock().await;
        let err = stderr.lock().await;
        snapshot.stdout = redact_credentials(&String::from_utf8_lossy(&out.bytes));
        snapshot.stderr = redact_credentials(&String::from_utf8_lossy(&err.bytes));
        snapshot.omitted_bytes = out.omitted + err.omitted;
        snapshot.last_updated_at = snapshot
            .last_updated_at
            .max(out.updated_at)
            .max(err.updated_at);
        Some(snapshot)
    }

    /// Internal observers may read a job without presenting its state to a
    /// caller. Transport handlers must use `get_for` for owner/session checks.
    pub(crate) async fn get_internal(&self, id: &str) -> Option<JobSnapshot> {
        self.expire_completed().await;
        let (mut snapshot, stdout, stderr) = {
            let jobs = self.jobs.lock().await;
            let job = jobs.get(id)?;
            (job.snapshot.clone(), job.stdout.clone(), job.stderr.clone())
        };
        let out = stdout.lock().await;
        let err = stderr.lock().await;
        snapshot.stdout = redact_credentials(&String::from_utf8_lossy(&out.bytes));
        snapshot.stderr = redact_credentials(&String::from_utf8_lossy(&err.bytes));
        snapshot.omitted_bytes = out.omitted + err.omitted;
        snapshot.last_updated_at = snapshot
            .last_updated_at
            .max(out.updated_at)
            .max(err.updated_at);
        Some(snapshot)
    }

    pub async fn cancel(&self, id: &str) -> Result<JobSnapshot, McpError> {
        self.cancel_for(id, "local", None).await
    }

    pub async fn cancel_for(
        &self,
        id: &str,
        owner: &str,
        session: Option<&str>,
    ) -> Result<JobSnapshot, McpError> {
        let mut jobs = self.jobs.lock().await;
        let job = jobs
            .get_mut(id)
            .ok_or_else(|| McpError::InvalidParams("unknown task".into()))?;
        if job.owner != owner || !session_matches(job.session.as_deref(), session) {
            return Err(McpError::InvalidParams("unknown task".into()));
        }
        request_cancel(job);
        Ok(job.snapshot.clone())
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

fn is_active(snapshot: &JobSnapshot) -> bool {
    matches!(snapshot.state, JobState::Queued | JobState::Running)
        || (snapshot.state == JobState::Cancelled && snapshot.finished_at.is_none())
        || (snapshot.state == JobState::TimedOut && snapshot.finished_at.is_none())
}

fn is_finished(snapshot: &JobSnapshot) -> bool {
    matches!(
        snapshot.state,
        JobState::Completed | JobState::Failed | JobState::TimedOut | JobState::Cancelled
    ) && snapshot.finished_at.is_some()
}

fn request_cancel(job: &mut JobRecord) {
    if is_active(&job.snapshot) {
        let _ = job.cancel.send(true);
        job.snapshot.state = JobState::Cancelled;
        job.snapshot.last_updated_at = now_ms();
    }
}

fn session_matches(recorded: Option<&str>, requested: Option<&str>) -> bool {
    match (recorded, requested) {
        (None, _) => true,
        (Some(expected), Some(actual)) => expected == actual,
        (Some(_), None) => false,
    }
}

pub fn render_output(exit_code: i32, stdout: &str, stderr: &str, omitted: u64) -> String {
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

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
