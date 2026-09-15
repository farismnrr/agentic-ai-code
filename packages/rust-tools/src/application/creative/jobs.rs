use super::contracts::{validate_id, validate_spec, CREATIVE_SCHEMA_VERSION};
use super::graph::{self, CreativeEstimate, CreativeJobKind, CreativeJobRecord, CreativeJobStatus};
use super::store;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

mod estimates;
pub use estimates::estimate_request;

const MAX_JOB_TIMEOUT_MS: u64 = 86_400_000;

#[derive(Debug, Clone)]
pub struct SubmitRequest {
    pub graph_id: Option<String>,
    pub capability_id: Option<String>,
    pub workflow_id: Option<String>,
    pub execution_binding_id: Option<String>,
    pub parameters: Value,
    pub approved: bool,
    pub max_retries: Option<u32>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BudgetStatus {
    pub approval_compute_units: u64,
    pub job_hard_compute_units: u64,
    pub project_hard_compute_units: u64,
    pub project_admitted_compute_units: u64,
    pub project_remaining_compute_units: u64,
    pub max_job_output_bytes: u64,
    pub max_concurrent_jobs: usize,
    pub running_jobs: usize,
    pub max_retries: u32,
    pub provider_quota: &'static str,
    pub local_compute_quota: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDecision {
    Allowed,
    ApprovalRequired,
    JobHardLimitExceeded,
    ProjectHardLimitExceeded,
    OutputHardLimitExceeded,
    ConcurrencyLimitExceeded,
}

pub fn budget_status(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<BudgetStatus, McpError> {
    validate_owner(owner)?;
    store::load_project(cwd, config, project_id)?;
    let jobs = owned_jobs(cwd, config, owner, project_id)?;
    let admitted = admitted_compute_units(&jobs);
    let running_jobs = jobs
        .iter()
        .filter(|job| job.status == CreativeJobStatus::Running)
        .count();
    Ok(BudgetStatus {
        approval_compute_units: config.creative_approval_compute_units,
        job_hard_compute_units: config.creative_job_hard_compute_units,
        project_hard_compute_units: config.creative_project_hard_compute_units,
        project_admitted_compute_units: admitted,
        project_remaining_compute_units: config
            .creative_project_hard_compute_units
            .saturating_sub(admitted),
        max_job_output_bytes: config.creative_max_job_output_bytes,
        max_concurrent_jobs: config.creative_max_concurrent_jobs,
        running_jobs,
        max_retries: config.creative_max_retries,
        provider_quota: "binding_owned_not_reported",
        local_compute_quota: "bounded_by_operator_limits",
    })
}

pub fn submit(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    request: SubmitRequest,
) -> Result<
    (
        AdmissionDecision,
        Option<CreativeJobRecord>,
        CreativeEstimate,
    ),
    McpError,
> {
    validate_owner(owner)?;
    let project = store::load_project(cwd, config, project_id)?;
    validate_submit_shape(&request)?;
    validate_spec(&request.parameters)?;
    let estimate = estimate_request(cwd, config, project_id, &request)?;
    let status = budget_status(cwd, config, owner, project_id)?;
    let decision = admission_decision(config, &status, &estimate, request.approved);
    if decision != AdmissionDecision::Allowed {
        return Ok((decision, None, estimate));
    }
    let max_retries = request.max_retries.unwrap_or(config.creative_max_retries);
    if max_retries > config.creative_max_retries {
        return Err(McpError::InvalidRequest(
            "creative job retry request exceeds operator maximum".into(),
        ));
    }
    let timeout_ms = request.timeout_ms.unwrap_or(0);
    if timeout_ms > MAX_JOB_TIMEOUT_MS {
        return Err(McpError::InvalidRequest(
            "creative job timeout exceeds maximum".into(),
        ));
    }
    let (kind, graph_id, capability_id, workflow_id) = if let Some(graph_id) = request.graph_id {
        (CreativeJobKind::Graph, Some(graph_id), None, None)
    } else if let Some(capability_id) = request.capability_id {
        (CreativeJobKind::Capability, None, Some(capability_id), None)
    } else {
        (CreativeJobKind::Workflow, None, None, request.workflow_id)
    };
    let now = store::now_ms();
    let job = CreativeJobRecord {
        schema_version: CREATIVE_SCHEMA_VERSION,
        job_id: format!("job_{}", Uuid::new_v4().simple()),
        project_id: project.project_id,
        owner: owner.to_owned(),
        kind,
        graph_id,
        capability_id,
        workflow_id,
        execution_binding_id: request.execution_binding_id,
        execution_parameters: request.parameters,
        status: CreativeJobStatus::Queued,
        estimate: Some(estimate.clone()),
        approved: request.approved,
        retry_count: 0,
        max_retries,
        timeout_ms,
        created_at_ms: now,
        updated_at_ms: now,
        node_runs: Vec::new(),
        output_asset_ids: Vec::new(),
        actual_output_bytes: None,
        failure_code: None,
    };
    store::store_job(cwd, config, &job)?;
    Ok((AdmissionDecision::Allowed, Some(job), estimate))
}

pub fn get(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    job_id: &str,
) -> Result<CreativeJobRecord, McpError> {
    validate_owner(owner)?;
    let job = store::load_job(cwd, config, project_id, job_id)?;
    enforce_owner(&job, owner)?;
    Ok(job)
}

pub fn list(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<Vec<CreativeJobRecord>, McpError> {
    validate_owner(owner)?;
    owned_jobs(cwd, config, owner, project_id)
}

pub fn cancel(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    job_id: &str,
) -> Result<CreativeJobRecord, McpError> {
    let mut job = get(cwd, config, owner, project_id, job_id)?;
    match job.status {
        CreativeJobStatus::Queued | CreativeJobStatus::Running => {
            job.status = CreativeJobStatus::Cancelled;
            job.updated_at_ms = store::now_ms();
            store::store_job(cwd, config, &job)?;
            Ok(job)
        }
        _ => Err(McpError::InvalidRequest(
            "creative job is already terminal".into(),
        )),
    }
}

pub fn wait(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    job_id: &str,
    retry: bool,
) -> Result<CreativeJobRecord, McpError> {
    let mut job = get(cwd, config, owner, project_id, job_id)?;
    if retry && job.status == CreativeJobStatus::Failed {
        if job.retry_count >= job.max_retries {
            return Err(McpError::InvalidRequest(
                "creative job retry limit reached".into(),
            ));
        }
        job.retry_count = job.retry_count.saturating_add(1);
        job.status = CreativeJobStatus::Queued;
        job.failure_code = None;
        job.node_runs.clear();
        job.updated_at_ms = store::now_ms();
        store::store_job(cwd, config, &job)?;
    }
    match job.status {
        CreativeJobStatus::Completed | CreativeJobStatus::Failed | CreativeJobStatus::Cancelled => {
            return Ok(job)
        }
        CreativeJobStatus::Running => return Ok(job),
        CreativeJobStatus::Queued => {}
    }
    let running = owned_jobs(cwd, config, owner, project_id)?
        .into_iter()
        .filter(|candidate| candidate.status == CreativeJobStatus::Running)
        .count();
    if running >= config.creative_max_concurrent_jobs {
        return Err(McpError::InvalidRequest(
            "creative job concurrency limit reached".into(),
        ));
    }
    job.status = CreativeJobStatus::Running;
    job.updated_at_ms = store::now_ms();
    store::store_job(cwd, config, &job)?;

    let terminal = match job.kind {
        CreativeJobKind::Graph => execute_graph_job(cwd, config, owner, &job)?,
        CreativeJobKind::Capability | CreativeJobKind::Workflow => {
            execute_bound_job(cwd, config, job)?
        }
    };
    store::store_job(cwd, config, &terminal)?;
    Ok(terminal)
}

fn execute_bound_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    #[cfg(all(feature = "test-creative-binding", debug_assertions))]
    {
        if job
            .execution_binding_id
            .as_deref()
            .is_some_and(|binding_id| binding_id.starts_with("test_"))
        {
            return execute_test_binding(config, job);
        }
    }
    if let Some(executed) = super::media::execute_media_job(cwd, config, &job)? {
        return Ok(executed);
    }
    let mut failed = job;
    failed.status = CreativeJobStatus::Failed;
    failed.failure_code = Some("execution_not_implemented".into());
    failed.updated_at_ms = store::now_ms();
    Ok(failed)
}

#[cfg(all(feature = "test-creative-binding", debug_assertions))]
fn execute_test_binding(
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let behavior = job
        .execution_parameters
        .get("test_behavior")
        .and_then(Value::as_str)
        .unwrap_or("complete");
    let simulated_duration_ms = job
        .execution_parameters
        .get("test_duration_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if job.timeout_ms > 0 && simulated_duration_ms > job.timeout_ms {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("execution_timeout".into());
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    let actual_output_bytes = job
        .execution_parameters
        .get("test_actual_output_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if actual_output_bytes > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    match behavior {
        "running" => {
            job.status = CreativeJobStatus::Running;
            job.failure_code = None;
        }
        "complete" => {
            job.status = CreativeJobStatus::Completed;
            job.failure_code = None;
        }
        "fail" => {
            job.status = CreativeJobStatus::Failed;
            job.failure_code = Some("test_execution_failed".into());
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "test creative binding behavior is unsupported".into(),
            ));
        }
    }
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn execute_graph_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    queued: &CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let graph_id = queued
        .graph_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("graph creative job requires graph_id".into()))?;
    let graph = store::load_graph(cwd, config, &queued.project_id, graph_id)?;
    let project = store::load_project(cwd, config, &queued.project_id)?;
    let executed = graph::execute_graph(&graph, &project, config, owner, store::now_ms())?;
    let mut terminal = queued.clone();
    terminal.status = executed.status;
    terminal.node_runs = executed.node_runs;
    terminal.output_asset_ids = executed.output_asset_ids;
    terminal.failure_code = executed.failure_code;
    terminal.updated_at_ms = store::now_ms();
    Ok(terminal)
}

fn admission_decision(
    config: &ServerConfig,
    status: &BudgetStatus,
    estimate: &CreativeEstimate,
    approved: bool,
) -> AdmissionDecision {
    if estimate.compute_units > config.creative_job_hard_compute_units {
        return AdmissionDecision::JobHardLimitExceeded;
    }
    if estimate.output_bytes > config.creative_max_job_output_bytes {
        return AdmissionDecision::OutputHardLimitExceeded;
    }
    if status
        .project_admitted_compute_units
        .saturating_add(estimate.compute_units)
        > config.creative_project_hard_compute_units
    {
        return AdmissionDecision::ProjectHardLimitExceeded;
    }
    if status.running_jobs >= config.creative_max_concurrent_jobs {
        return AdmissionDecision::ConcurrencyLimitExceeded;
    }
    if estimate.compute_units > config.creative_approval_compute_units && !approved {
        return AdmissionDecision::ApprovalRequired;
    }
    AdmissionDecision::Allowed
}

fn admitted_compute_units(jobs: &[CreativeJobRecord]) -> u64 {
    jobs.iter()
        .filter(|job| job.status != CreativeJobStatus::Cancelled)
        .filter_map(|job| job.estimate.as_ref().map(|estimate| estimate.compute_units))
        .fold(0u64, u64::saturating_add)
}

fn owned_jobs(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<Vec<CreativeJobRecord>, McpError> {
    Ok(store::list_jobs(cwd, config, project_id)?
        .into_iter()
        .filter(|job| job.owner == owner)
        .collect())
}

fn enforce_owner(job: &CreativeJobRecord, owner: &str) -> Result<(), McpError> {
    if job.owner != owner {
        return Err(McpError::InvalidRequest(
            "creative job belongs to a different owner".into(),
        ));
    }
    Ok(())
}

fn validate_owner(owner: &str) -> Result<(), McpError> {
    if owner.is_empty() || owner.len() > 512 || owner.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative job owner is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_submit_shape(request: &SubmitRequest) -> Result<(), McpError> {
    let targets = usize::from(request.graph_id.is_some())
        + usize::from(request.capability_id.is_some())
        + usize::from(request.workflow_id.is_some());
    if targets != 1 {
        return Err(McpError::InvalidRequest(
            "creative job submit requires exactly one graph_id, capability_id, or workflow_id"
                .into(),
        ));
    }
    for (value, label) in [
        (request.graph_id.as_deref(), "graph_id"),
        (
            request.execution_binding_id.as_deref(),
            "execution_binding_id",
        ),
    ] {
        if let Some(value) = value {
            validate_id(value, label)?;
        }
    }
    Ok(())
}
