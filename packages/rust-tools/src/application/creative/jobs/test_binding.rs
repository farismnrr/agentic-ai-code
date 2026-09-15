use super::super::contracts::{AssetMetadata, AssetSource, AssetState, AssetSurface};
use super::super::graph::{CreativeJobRecord, CreativeJobStatus};
use super::super::{registry, store};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};

pub(super) fn execute(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    if job.capability_id.as_deref() == Some("identity.prepare") {
        return execute_identity_prepare(cwd, config, job);
    }

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

fn execute_identity_prepare(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let params = &job.execution_parameters;
    let element_id = required_str(params, "element_id")?;
    let reference_asset_ids = params
        .get("reference_asset_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("identity references are required".into()))?;
    let parent_asset_id = reference_asset_ids
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("identity reference is invalid".into()))?;
    let artifact_kind = required_str(params, "artifact_kind")?;
    let artifact_version = required_str(params, "artifact_version")?;
    let subject_kind = required_str(params, "subject_kind")?;
    let binding_id = job.execution_binding_id.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("identity preparation requires a binding".into())
    })?;
    let binding = registry::binding(config, binding_id)?
        .ok_or_else(|| McpError::InvalidRequest("identity binding is unavailable".into()))?;

    let bytes = serde_json::to_vec(&json!({
        "schema":"identity-artifact-test-v1",
        "subject_kind":subject_kind,
        "artifact_kind":artifact_kind,
        "artifact_version":artifact_version,
        "binding_version":binding.binding_version,
        "reference_count":reference_asset_ids.len()
    }))
    .map_err(|_| McpError::Internal("identity artifact encoding failed".into()))?;
    if bytes.len() as u64 > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.actual_output_bytes = Some(bytes.len() as u64);
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }

    let relative_path = format!(
        "creative/{}/assets/identity/{}.bin",
        job.project_id, job.job_id
    );
    crate::application::workspace::write_contained_bytes(
        &relative_path,
        cwd,
        &bytes,
        true,
        false,
        config,
    )?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        &job.project_id,
        store::AssetRegistrationInput {
            path: relative_path,
            media_type: "application/octet-stream".into(),
            role: "identity_binding_artifact".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Anime,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: Some(parent_asset_id.to_owned()),
            element_id: Some(element_id.to_owned()),
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some(artifact_kind.to_owned()),
                artifact_version: Some(artifact_version.to_owned()),
                license_notes: binding.license_notes,
                ..AssetMetadata::default()
            },
        },
    )?;
    job.status = CreativeJobStatus::Completed;
    job.output_asset_ids = vec![asset_id];
    job.actual_output_bytes = Some(bytes.len() as u64);
    job.failure_code = None;
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, McpError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))
}
