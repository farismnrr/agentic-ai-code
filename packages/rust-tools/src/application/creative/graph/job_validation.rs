use super::{CreativeJobKind, CreativeJobRecord, MAX_GRAPH_NODES};
use crate::application::creative::contracts::{
    validate_id, validate_spec, CREATIVE_SCHEMA_VERSION,
};
use crate::application::creative::registry;
use crate::core::error::McpError;
use std::collections::HashSet;

pub fn validate_job_record(job: &CreativeJobRecord) -> Result<(), McpError> {
    if job.schema_version != CREATIVE_SCHEMA_VERSION {
        return Err(McpError::InvalidRequest(
            "creative job schema version is unsupported".into(),
        ));
    }
    validate_id(&job.job_id, "job_id")?;
    validate_id(&job.project_id, "project_id")?;
    if job.owner.is_empty() || job.owner.len() > 512 || job.owner.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative job owner is invalid".into(),
        ));
    }
    match job.kind {
        CreativeJobKind::Graph => {
            let graph_id = job.graph_id.as_deref().ok_or_else(|| {
                McpError::InvalidRequest("graph creative job requires graph_id".into())
            })?;
            validate_id(graph_id, "graph_id")?;
            if job.capability_id.is_some() || job.workflow_id.is_some() {
                return Err(McpError::InvalidRequest(
                    "graph creative job cannot also declare capability/workflow identity".into(),
                ));
            }
        }
        CreativeJobKind::Capability => {
            let capability_id = job.capability_id.as_deref().ok_or_else(|| {
                McpError::InvalidRequest("capability creative job requires capability_id".into())
            })?;
            if registry::capability(capability_id).is_none()
                || job.graph_id.is_some()
                || job.workflow_id.is_some()
            {
                return Err(McpError::InvalidRequest(
                    "capability creative job identity is invalid".into(),
                ));
            }
        }
        CreativeJobKind::Workflow => {
            let workflow_id = job.workflow_id.as_deref().ok_or_else(|| {
                McpError::InvalidRequest("workflow creative job requires workflow_id".into())
            })?;
            if registry::workflow(workflow_id).is_none()
                || job.graph_id.is_some()
                || job.capability_id.is_some()
            {
                return Err(McpError::InvalidRequest(
                    "workflow creative job identity is invalid".into(),
                ));
            }
        }
    }
    if let Some(binding_id) = job.execution_binding_id.as_deref() {
        validate_id(binding_id, "execution_binding_id")?;
    }
    for (value, label) in [
        (job.compiler_version.as_deref(), "compiler_version"),
        (
            job.execution_binding_version.as_deref(),
            "execution_binding_version",
        ),
    ] {
        if value.is_some_and(|value| {
            value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
        }) {
            return Err(McpError::InvalidRequest(format!(
                "creative job {label} is invalid"
            )));
        }
    }
    if job.compiler_version.is_some() != job.execution_binding_version.is_some()
        || job.changed_fields.len() > 32
    {
        return Err(McpError::InvalidRequest(
            "creative compiler lineage is inconsistent".into(),
        ));
    }
    let mut changed_fields = HashSet::new();
    for field in &job.changed_fields {
        validate_id(field, "changed_field")?;
        if !changed_fields.insert(field.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative compiler lineage contains duplicate changed fields".into(),
            ));
        }
    }
    if let Some(estimate) = &job.estimate {
        if estimate.compute_units == 0
            || estimate.output_bytes == 0
            || estimate.source.is_empty()
            || estimate.source.len() > 128
            || estimate.source.chars().any(char::is_control)
        {
            return Err(McpError::InvalidRequest(
                "creative job estimate is invalid".into(),
            ));
        }
    }
    if job.retry_count > job.max_retries || job.max_retries > 16 || job.timeout_ms > 86_400_000 {
        return Err(McpError::InvalidRequest(
            "creative job retry/timeout bounds are invalid".into(),
        ));
    }
    if job.output_asset_ids.len() > 256 {
        return Err(McpError::InvalidRequest(
            "creative job output asset count exceeds maximum".into(),
        ));
    }
    for asset_id in &job.output_asset_ids {
        validate_id(asset_id, "asset_id")?;
    }
    if job.node_runs.len() > MAX_GRAPH_NODES {
        return Err(McpError::InvalidRequest(
            "creative job node-run count exceeds maximum".into(),
        ));
    }
    let mut node_ids = HashSet::new();
    for run in &job.node_runs {
        validate_id(&run.node_id, "node_id")?;
        if !node_ids.insert(run.node_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative job contains duplicate node runs".into(),
            ));
        }
        if let Some(output) = &run.output {
            validate_spec(output)?;
        }
        if let Some(code) = run.failure_code.as_deref() {
            validate_id(code, "failure_code")?;
        }
    }
    validate_spec(&job.execution_parameters)?;
    if let Some(code) = job.failure_code.as_deref() {
        validate_id(code, "failure_code")?;
    }
    Ok(())
}
