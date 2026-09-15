use super::super::contracts::{AssetMetadata, AssetSource, AssetState, AssetSurface};
use super::super::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::{registry, store};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};

pub(super) fn execute(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let injected_workflow_capability = if job.capability_id.is_none() {
        job.workflow_id
            .as_deref()
            .and_then(registry::workflow)
            .filter(|workflow| workflow.required_capabilities.len() == 1)
            .and_then(|workflow| workflow.required_capabilities.first().cloned())
    } else {
        None
    };
    if let Some(capability_id) = injected_workflow_capability.as_ref() {
        job.capability_id = Some(capability_id.clone());
    }
    let mut executed = if job.capability_id.as_deref() == Some("identity.prepare") {
        execute_identity_prepare(cwd, config, job)?
    } else if job.capability_id.as_deref() == Some("3d.image_to_mesh") {
        execute_image_to_mesh(cwd, config, job)?
    } else if job.capability_id.as_deref() == Some("game.deploy") {
        execute_game_deploy(cwd, config, job)?
    } else if job
        .capability_id
        .as_deref()
        .is_some_and(|id| id.starts_with("video.") || id.starts_with("audio."))
    {
        execute_media_conformance(cwd, config, job)?
    } else {
        execute_generic(cwd, config, job)?
    };
    if injected_workflow_capability.is_some() {
        executed.capability_id = None;
    }
    Ok(executed)
}

fn execute_generic(
    _cwd: Option<&str>,
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

fn execute_image_to_mesh(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let element_id = required_str(&job.execution_parameters, "element_id")?;
    let reference_asset_ids = job
        .execution_parameters
        .get("reference_asset_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("3D bootstrap references are required".into()))?;
    let parent_asset_id = reference_asset_ids
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("3D bootstrap reference is invalid".into()))?;
    let binding_id = job.execution_binding_id.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("3D bootstrap requires a selected binding".into())
    })?;
    let binding = registry::binding(config, binding_id)?
        .ok_or_else(|| McpError::InvalidRequest("3D bootstrap binding is unavailable".into()))?;
    let obj = b"o BootstrapCube\nv -0.5 -0.5 0.0\nv 0.5 -0.5 0.0\nv 0.5 0.5 0.0\nv -0.5 0.5 0.0\nv -0.5 -0.5 1.0\nv 0.5 -0.5 1.0\nv 0.5 0.5 1.0\nv -0.5 0.5 1.0\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n";
    if obj.len() as u64 > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.actual_output_bytes = Some(obj.len() as u64);
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    let relative_path = format!("creative/{}/assets/3d/{}.obj", job.project_id, job.job_id);
    crate::application::workspace::write_contained_bytes(
        &relative_path,
        cwd,
        obj,
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
            media_type: "model/obj".into(),
            role: "3d_bootstrap_candidate".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Anime,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: Some(parent_asset_id.to_owned()),
            element_id: Some(element_id.to_owned()),
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some("test_image_to_mesh".into()),
                artifact_version: Some(binding.binding_version),
                license_notes: binding.license_notes,
                ..AssetMetadata::default()
            },
        },
    )?;
    job.status = CreativeJobStatus::Completed;
    job.output_asset_ids = vec![asset_id];
    job.actual_output_bytes = Some(obj.len() as u64);
    job.failure_code = None;
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn execute_game_deploy(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let game_id = required_str(&job.execution_parameters, "game_id")?;
    let build_revision = required_str(&job.execution_parameters, "build_revision")?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut game = project
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown game manifest".into()))?;
    if game.build_state.accepted_build_revision.as_deref() != Some(build_revision) {
        return Err(McpError::InvalidRequest(
            "game deploy requires the accepted build revision".into(),
        ));
    }
    let deployment_id = format!("deployment_{}", uuid::Uuid::new_v4().simple());
    let deployment_url = format!(
        "https://example.invalid/creative/{}/{deployment_id}",
        job.project_id
    );
    game.build_state.deployment_id = Some(deployment_id.clone());
    game.build_state.deployment_url = Some(deployment_url.clone());
    game.build_state.published = false;
    game.revision_id = Some(format!("revision_{}", uuid::Uuid::new_v4().simple()));
    store::upsert_game(cwd, config, &job.project_id, game)?;

    let bytes = serde_json::to_vec(&json!({
        "schema":"game-deployment-conformance-v1",
        "game_id":game_id,
        "build_revision":build_revision,
        "deployment_id":deployment_id,
        "deployment_url":deployment_url,
        "published":false,
        "binding_id":job.execution_binding_id
    }))
    .map_err(|_| McpError::Internal("game deployment receipt encoding failed".into()))?;
    let relative_path = format!(
        "creative/{}/games/{}/deployments/{}.json",
        job.project_id, game_id, deployment_id
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
            media_type: "application/json".into(),
            role: "game_deployment_receipt".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Game,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: None,
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some("game_deployment_receipt".into()),
                artifact_version: Some("v1".into()),
                license_notes: Some(
                    "test-only deployment conformance; public publish remains separate".into(),
                ),
                ..AssetMetadata::default()
            },
        },
    )?;
    job.status = CreativeJobStatus::Completed;
    job.output_asset_ids = vec![asset_id];
    job.actual_output_bytes = Some(bytes.len() as u64);
    job.failure_code = None;
    job.node_runs = vec![NodeRunRecord {
        node_id: "game_deploy".into(),
        status: CreativeJobStatus::Completed,
        output: Some(json!({
            "deployment_id":deployment_id,
            "deployment_url":deployment_url,
            "game_id":game_id,
            "build_revision":build_revision,
            "published":false
        })),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn execute_media_conformance(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let capability_id = job
        .capability_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("test media capability is missing".into()))?;
    if capability_id == "audio.voice_clone"
        && job
            .execution_parameters
            .get("consent_asserted")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(McpError::InvalidRequest(
            "voice cloning requires explicit consent assertion".into(),
        ));
    }
    let (extension, media_type, role) = if capability_id == "audio.video_dub" {
        ("mp4", "video/mp4", "dubbed_video")
    } else if capability_id.starts_with("audio.") {
        ("wav", "audio/wav", "generated_audio")
    } else {
        ("mp4", "video/mp4", "generated_video")
    };
    let parent_asset_id = ["asset_id", "video_asset_id", "reference_asset_id"]
        .into_iter()
        .find_map(|field| {
            job.execution_parameters
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            job.execution_parameters
                .get("reference_asset_ids")
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let duration_ms = if capability_id == "video.clip_extract" {
        let start = job
            .execution_parameters
            .get("start_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let end = job
            .execution_parameters
            .get("end_ms")
            .and_then(Value::as_u64)
            .unwrap_or(start.saturating_add(1_000));
        if end <= start {
            return Err(McpError::InvalidRequest(
                "clip extraction end_ms must be greater than start_ms".into(),
            ));
        }
        Some(end - start)
    } else {
        job.execution_parameters
            .get("duration_ms")
            .and_then(Value::as_u64)
            .or_else(|| capability_id.starts_with("video.").then_some(15_000))
    };
    let bytes = serde_json::to_vec(&json!({
        "schema":"creative-media-conformance-v1",
        "capability_id":capability_id,
        "duration_ms":duration_ms,
        "parent_asset_id":parent_asset_id,
        "binding_id":job.execution_binding_id,
        "parameters_hash":"deterministic_fixture"
    }))
    .map_err(|_| McpError::Internal("media conformance artifact encoding failed".into()))?;
    if bytes.len() as u64 > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.actual_output_bytes = Some(bytes.len() as u64);
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    let relative_path = format!(
        "creative/{}/assets/conformance/{}/{}.{}",
        job.project_id,
        job.job_id,
        capability_id.replace('.', "_"),
        extension
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
            media_type: media_type.into(),
            role: role.into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id,
            element_id: job
                .execution_parameters
                .get("voice_element_id")
                .or_else(|| job.execution_parameters.get("element_id"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                duration_ms,
                language: job
                    .execution_parameters
                    .get("language")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                artifact_kind: Some(capability_id.to_owned()),
                artifact_version: Some("conformance-v1".into()),
                license_notes: Some("test-only conformance artifact".into()),
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
