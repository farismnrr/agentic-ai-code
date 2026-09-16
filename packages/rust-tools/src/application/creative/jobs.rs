use super::contracts::{validate_id, validate_spec, SceneManifest, CREATIVE_SCHEMA_VERSION};
use super::graph::{
    self, CreativeEstimate, CreativeJobKind, CreativeJobRecord, CreativeJobStatus,
    ExternalNodeExecutor, GraphExecutionContext, GraphNode, GraphNodeKind,
};
use super::{registry, store};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

mod bootstrap;
mod character;
mod delivery;
mod estimates;
mod game;
mod graph_exec;
mod scene;
mod support;
#[cfg(all(feature = "test-creative-binding", debug_assertions))]
mod test_binding;
pub use estimates::estimate_request;
pub(super) use graph_exec::{
    execute_graph_direct, execute_graph_job, execute_graph_partial_direct, PartialGraphExecution,
};
use support::{enforce_owner, owned_jobs, validate_owner};

const MAX_JOB_TIMEOUT_MS: u64 = 86_400_000;

#[derive(Debug, Clone)]
pub struct SubmitRequest {
    pub graph_id: Option<String>,
    pub capability_id: Option<String>,
    pub workflow_id: Option<String>,
    pub execution_binding_id: Option<String>,
    pub parameters: Value,
    pub semantic_spec: Option<Value>,
    pub changed_fields: Vec<String>,
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

#[derive(Debug, Clone, Default)]
pub struct CompilationLineage {
    pub compiler_version: Option<String>,
    pub execution_binding_version: Option<String>,
    pub changed_fields: Vec<String>,
}

pub fn prepare_request(
    config: &ServerConfig,
    request: &mut SubmitRequest,
) -> Result<CompilationLineage, McpError> {
    let Some(spec) = request.semantic_spec.take() else {
        if !request.changed_fields.is_empty() {
            return Err(McpError::InvalidRequest(
                "creative changed_fields require semantic_spec".into(),
            ));
        }
        return Ok(CompilationLineage::default());
    };
    if request
        .parameters
        .as_object()
        .is_none_or(|object| !object.is_empty())
    {
        return Err(McpError::InvalidRequest(
            "creative semantic_spec and raw parameters are mutually exclusive".into(),
        ));
    }
    if request.graph_id.is_some() || request.workflow_id.is_some() {
        return Err(McpError::InvalidRequest(
            "creative semantic compiler currently targets one capability job".into(),
        ));
    }
    let capability_id = request
        .capability_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("semantic_spec requires capability_id".into()))?;
    let binding_id = request.execution_binding_id.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("semantic_spec requires execution_binding_id".into())
    })?;
    let compiled = super::compiler::compile(
        config,
        capability_id,
        binding_id,
        &spec,
        &request.changed_fields,
    )?;
    request.parameters = compiled.parameters;
    request.changed_fields = compiled.changed_fields.clone();
    Ok(CompilationLineage {
        compiler_version: Some(compiled.compiler_version),
        execution_binding_version: Some(compiled.execution_binding_version),
        changed_fields: compiled.changed_fields,
    })
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
    mut request: SubmitRequest,
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
    let mut lineage = prepare_request(config, &mut request)?;
    validate_spec(&request.parameters)?;
    if lineage.execution_binding_version.is_none() {
        if let Some(binding_id) = request.execution_binding_id.as_deref() {
            lineage.execution_binding_version =
                registry::binding(config, binding_id)?.map(|binding| binding.binding_version);
        }
    }
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
        compiler_version: lineage.compiler_version,
        execution_binding_version: lineage.execution_binding_version,
        changed_fields: lineage.changed_fields,
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

pub async fn wait(
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
        CreativeJobKind::Graph => execute_graph_job(cwd, config, owner, &job).await?,
        CreativeJobKind::Capability | CreativeJobKind::Workflow => {
            execute_bound_job(cwd, config, owner, job).await?
        }
    };
    store::store_job(cwd, config, &terminal)?;
    Ok(terminal)
}

pub(super) async fn execute_bound_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    match job.workflow_id.as_deref() {
        Some("image_to_3d_bootstrap") => bootstrap::execute(cwd, config, job).await,
        Some(
            "character_mesh_production"
            | "character_rig_production"
            | "character_action"
            | "character_facial_performance"
            | "character_secondary_motion",
        ) => character::execute(cwd, config, owner, job).await,
        Some(
            "export_profile" | "project_handoff" | "promo_pack" | "key_visual_bundle"
            | "portfolio_delivery",
        ) => delivery::execute(cwd, config, job),
        Some("game_source_scaffold" | "game_build_playtest" | "game_iteration") => {
            game::execute_local(cwd, config, job)
        }
        Some(
            "scene_continuity_review"
            | "visual_qa_evidence"
            | "temporal_qa_evidence"
            | "scoped_revision",
        ) => scene::execute_local(cwd, config, job),
        _ => execute_leaf_bound_job(cwd, config, job),
    }
}

pub(super) fn execute_leaf_bound_job(
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
            return test_binding::execute(cwd, config, job);
        }
    }
    if let Some(executed) = super::deployment::execute_local_static_game(cwd, config, &job)? {
        return Ok(executed);
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
