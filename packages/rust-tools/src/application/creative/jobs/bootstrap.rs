use super::super::contracts::{ElementKind, RevisionState};
use super::super::graph::{CreativeJobKind, CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::{registry, store};
use crate::application::blender::{run_character_bootstrap, CharacterBootstrapRequest};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::collections::HashSet;

pub(super) async fn execute(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    if !config.enable_blender {
        return Err(McpError::InvalidRequest(
            "image_to_3d_bootstrap requires the operator-enabled Blender capability".into(),
        ));
    }
    let params = job.execution_parameters.clone();
    let route = required_str(&params, "route")?.to_owned();
    let element_id = required_str(&params, "element_id")?.to_owned();
    let references = required_reference_ids(&params)?;
    let alignment = params
        .get("alignment")
        .and_then(Value::as_object)
        .ok_or_else(|| McpError::InvalidRequest("bootstrap alignment is required".into()))?;
    let front_asset_id = required_map_str(alignment, "front_asset_id")?.to_owned();
    let side_asset_id = required_map_str(alignment, "side_asset_id")?.to_owned();
    let back_asset_id = required_map_str(alignment, "back_asset_id")?.to_owned();
    let reference_scale = alignment
        .get("reference_scale")
        .and_then(Value::as_f64)
        .unwrap_or(2.0);

    validate_character_references(
        cwd,
        config,
        &job.project_id,
        &element_id,
        &references,
        [&front_asset_id, &side_asset_id, &back_asset_id],
    )?;

    let generated_mesh_asset_id = match route.as_str() {
        "manual" => {
            if job.execution_binding_id.is_some() {
                return Err(McpError::InvalidRequest(
                    "manual Blender bootstrap must not select an execution binding".into(),
                ));
            }
            None
        }
        "binding" | "hybrid" => {
            registry::validate_binding_selection(
                config,
                "3d.image_to_mesh",
                job.execution_binding_id.as_deref(),
            )?;
            let mut mesh_job = job.clone();
            mesh_job.kind = CreativeJobKind::Capability;
            mesh_job.capability_id = Some("3d.image_to_mesh".into());
            mesh_job.workflow_id = None;
            mesh_job.node_runs.clear();
            mesh_job.output_asset_ids.clear();
            mesh_job.failure_code = None;
            let mesh_job = super::execute_leaf_bound_job(cwd, config, mesh_job)?;
            if mesh_job.status != CreativeJobStatus::Completed {
                let mut failed = job;
                failed.status = CreativeJobStatus::Failed;
                failed.failure_code = mesh_job
                    .failure_code
                    .or_else(|| Some("bootstrap_mesh_generation_failed".into()));
                failed.updated_at_ms = store::now_ms();
                return Ok(failed);
            }
            let mesh_asset_id = mesh_job.output_asset_ids.first().cloned().ok_or_else(|| {
                McpError::InvalidRequest(
                    "selected 3D execution binding completed without a mesh Asset".into(),
                )
            })?;
            Some(mesh_asset_id)
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "bootstrap route is unsupported".into(),
            ))
        }
    };

    let setup = run_character_bootstrap(
        cwd,
        config,
        CharacterBootstrapRequest {
            project_id: &job.project_id,
            job_id: &job.job_id,
            element_id: &element_id,
            route: &route,
            front_asset_id: &front_asset_id,
            side_asset_id: &side_asset_id,
            back_asset_id: &back_asset_id,
            reference_scale,
            mesh_asset_id: generated_mesh_asset_id.as_deref(),
        },
    )
    .await?;
    let scene_asset_id = setup
        .get("scene_asset_id")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::Internal("Blender bootstrap returned no scene Asset ID".into()))?
        .to_owned();

    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut total_bytes = project
        .asset(&scene_asset_id)
        .map(|asset| asset.bytes)
        .unwrap_or(0);
    if let Some(mesh_asset_id) = generated_mesh_asset_id.as_deref() {
        total_bytes = total_bytes.saturating_add(
            project
                .asset(mesh_asset_id)
                .map(|asset| asset.bytes)
                .unwrap_or(0),
        );
    }
    if let Some(materialized_refs) = setup
        .get("materialized_reference_asset_ids")
        .and_then(Value::as_array)
    {
        for asset_id in materialized_refs.iter().filter_map(Value::as_str) {
            total_bytes = total_bytes.saturating_add(
                project
                    .asset(asset_id)
                    .map(|asset| asset.bytes)
                    .unwrap_or(0),
            );
        }
    }
    if let Some(materialized_mesh_id) = setup
        .get("materialized_mesh_asset_id")
        .and_then(Value::as_str)
    {
        total_bytes = total_bytes.saturating_add(
            project
                .asset(materialized_mesh_id)
                .map(|asset| asset.bytes)
                .unwrap_or(0),
        );
    }
    if total_bytes > config.creative_max_job_output_bytes {
        let mut failed = job;
        failed.status = CreativeJobStatus::Failed;
        failed.failure_code = Some("output_hard_limit_exceeded".into());
        failed.actual_output_bytes = Some(total_bytes);
        failed.updated_at_ms = store::now_ms();
        return Ok(failed);
    }

    let mut completed = job;
    completed.status = CreativeJobStatus::Completed;
    completed.failure_code = None;
    completed.actual_output_bytes = Some(total_bytes);
    completed.output_asset_ids = generated_mesh_asset_id
        .clone()
        .into_iter()
        .chain(std::iter::once(scene_asset_id.clone()))
        .collect();
    completed.node_runs = vec![NodeRunRecord {
        node_id: "blender_character_bootstrap".into(),
        status: CreativeJobStatus::Completed,
        output: Some(json!({
            "route":route,
            "character_element_id":element_id,
            "generated_mesh_asset_id":generated_mesh_asset_id,
            "scene_asset_id":scene_asset_id,
            "reference_asset_ids":references,
            "reference_alignment":setup.get("reference_alignment").cloned(),
            "materialized_reference_asset_ids":setup.get("materialized_reference_asset_ids").cloned(),
            "materialized_mesh_asset_id":setup.get("materialized_mesh_asset_id").cloned(),
            "silhouette_extent_evidence":setup.pointer("/bridge_result/silhouette_extent_evidence").cloned(),
            "visual_inspection":setup.pointer("/bridge_result/visual_inspection").cloned().unwrap_or_else(|| json!("not_inspected")),
            "character_inspection":setup.get("character_inspection").cloned()
        })),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    completed.updated_at_ms = store::now_ms();
    Ok(completed)
}

fn validate_character_references(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    element_id: &str,
    reference_asset_ids: &[String],
    required_alignment: [&str; 3],
) -> Result<(), McpError> {
    let project = store::load_project(cwd, config, project_id)?;
    let character = project
        .element(element_id)
        .ok_or_else(|| McpError::InvalidRequest("bootstrap Character Element is unknown".into()))?;
    if character.kind != ElementKind::Character {
        return Err(McpError::InvalidRequest(
            "image_to_3d_bootstrap element_id must reference a Character Element".into(),
        ));
    }
    let selected_id = character.selected_revision_id.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("bootstrap Character Element has no accepted revision".into())
    })?;
    let selected = character
        .revisions
        .iter()
        .find(|revision| revision.revision_id == selected_id)
        .ok_or_else(|| {
            McpError::InvalidRequest("selected Character revision is unavailable".into())
        })?;
    if selected.state != RevisionState::Accepted {
        return Err(McpError::InvalidRequest(
            "bootstrap Character revision must be accepted".into(),
        ));
    }

    let declared = reference_asset_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let selected_refs = selected
        .reference_asset_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let required = required_alignment.into_iter().collect::<HashSet<_>>();
    if required.len() != 3
        || !required.iter().all(|asset_id| declared.contains(asset_id))
        || !required
            .iter()
            .all(|asset_id| selected_refs.contains(asset_id))
    {
        return Err(McpError::InvalidRequest(
            "front/side/back bootstrap references must be distinct, declared, and belong to the accepted Character revision".into(),
        ));
    }
    for asset_id in reference_asset_ids {
        if !selected_refs.contains(asset_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "bootstrap references must come from the accepted Character revision".into(),
            ));
        }
        let asset = project.asset(asset_id).ok_or_else(|| {
            McpError::InvalidRequest("bootstrap references contain an unknown Asset".into())
        })?;
        if !asset.media_type.starts_with("image/") {
            return Err(McpError::InvalidRequest(
                "bootstrap references must be image Assets".into(),
            ));
        }
    }
    Ok(())
}

fn required_reference_ids(parameters: &Value) -> Result<Vec<String>, McpError> {
    let values = parameters
        .get("reference_asset_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("bootstrap references are required".into()))?;
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let asset_id = value.as_str().ok_or_else(|| {
            McpError::InvalidRequest("bootstrap reference Asset ID is invalid".into())
        })?;
        if !seen.insert(asset_id) {
            return Err(McpError::InvalidRequest(
                "bootstrap reference Asset IDs must be unique".into(),
            ));
        }
        result.push(asset_id.to_owned());
    }
    Ok(result)
}

fn required_str<'a>(parameters: &'a Value, field: &str) -> Result<&'a str, McpError> {
    parameters
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("bootstrap {field} is required")))
}

fn required_map_str<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a str, McpError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("bootstrap {field} is required")))
}
