use super::super::contracts::{AssetMetadata, AssetSource, AssetState, AssetSurface};
use super::super::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) fn execute(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("delivery workflow id is required".into()))?;
    let (role, artifact_kind, payload, parent_asset_id) = match workflow_id {
        "export_profile" => export_payload(cwd, config, &job)?,
        "project_handoff" => handoff_payload(cwd, config, &job)?,
        "promo_pack" | "key_visual_bundle" | "portfolio_delivery" => {
            packaged_payload(cwd, config, &job, workflow_id)?
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "unsupported local delivery workflow".into(),
            ))
        }
    };
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|_| McpError::Internal("delivery manifest encoding failed".into()))?;
    if bytes.len() as u64 > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.actual_output_bytes = Some(bytes.len() as u64);
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    let relative_path = format!(
        "creative/{}/exports/{}_{}.json",
        job.project_id, workflow_id, job.job_id
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
        AssetRegistrationInput {
            path: relative_path,
            media_type: "application/json".into(),
            role: role.into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id,
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some(artifact_kind.into()),
                artifact_version: Some("v1".into()),
                ..AssetMetadata::default()
            },
        },
    )?;
    job.status = CreativeJobStatus::Completed;
    job.failure_code = None;
    job.output_asset_ids = vec![asset_id.clone()];
    job.actual_output_bytes = Some(bytes.len() as u64);
    job.node_runs = vec![NodeRunRecord {
        node_id: workflow_id.into(),
        status: CreativeJobStatus::Completed,
        output: Some(json!({
            "asset_id":asset_id,
            "artifact_kind":artifact_kind,
            "deploy_performed":false,
            "publish_performed":false
        })),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn export_payload(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<(&'static str, &'static str, Value, Option<String>), McpError> {
    let profile = required_str(&job.execution_parameters, "profile")?;
    let file_name = required_str(&job.execution_parameters, "file_name")?;
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(McpError::InvalidRequest(
            "export profile file_name must be a bounded leaf name".into(),
        ));
    }
    let ids = asset_ids(&job.execution_parameters, "asset_ids", 1, 128)?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut assets = Vec::with_capacity(ids.len());
    let mut materialized_outputs = Vec::with_capacity(ids.len());
    for (index, id) in ids.iter().enumerate() {
        let asset = project
            .asset(id)
            .ok_or_else(|| McpError::InvalidRequest("export references an unknown Asset".into()))?;
        if asset.state != AssetState::Accepted {
            return Err(McpError::InvalidRequest(
                "final export profiles require accepted Assets".into(),
            ));
        }
        let source = store::resolve_registered_asset_path(cwd, config, &job.project_id, id)?;
        let bytes = fs::read(&source)
            .map_err(|_| McpError::InvalidRequest("accepted export Asset cannot be read".into()))?;
        let leaf = if ids.len() == 1 {
            file_name.to_owned()
        } else {
            let source_leaf = Path::new(&asset.relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    McpError::InvalidRequest("export Asset path has no valid leaf name".into())
                })?;
            format!("{:03}_{}", index + 1, source_leaf)
        };
        let output_path = format!(
            "creative/{}/exports/{}/files/{leaf}",
            job.project_id, job.job_id
        );
        crate::application::workspace::write_contained_bytes(
            &output_path,
            cwd,
            &bytes,
            true,
            false,
            config,
        )?;
        materialized_outputs.push(json!({
            "asset_id":asset.asset_id,
            "output_path":output_path,
            "checksum_sha256":asset.checksum_sha256,
            "bytes":asset.bytes
        }));
        assets.push(json!({
            "asset_id":asset.asset_id,
            "media_type":asset.media_type,
            "role":asset.role,
            "relative_path":asset.relative_path,
            "checksum_sha256":asset.checksum_sha256,
            "bytes":asset.bytes
        }));
    }
    let payload = json!({
        "schema":"creative-export-profile-v1",
        "project_id":job.project_id,
        "profile":profile,
        "requested_file_name":file_name,
        "assets":assets,
        "materialized_outputs":materialized_outputs,
        "deploy_performed":false,
        "publish_performed":false
    });
    Ok((
        "creative_export_manifest",
        "creative_export_profile",
        payload,
        ids.first().cloned(),
    ))
}

fn handoff_payload(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<(&'static str, &'static str, Value, Option<String>), McpError> {
    let label = required_str(&job.execution_parameters, "label")?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let selected_elements = project
        .elements
        .iter()
        .map(|element| {
            json!({
                "element_id":element.element_id,
                "kind":element.kind,
                "name":element.name,
                "selected_revision_id":element.selected_revision_id
            })
        })
        .collect::<Vec<_>>();
    let accepted_assets = project
        .assets
        .iter()
        .filter(|asset| asset.state == AssetState::Accepted)
        .map(|asset| {
            json!({
                "asset_id":asset.asset_id,
                "role":asset.role,
                "relative_path":asset.relative_path,
                "checksum_sha256":asset.checksum_sha256,
                "source":asset.source,
                "source_surface":asset.source_surface,
                "element_id":asset.element_id
            })
        })
        .collect::<Vec<_>>();
    let mut qa_summary = BTreeMap::new();
    for finding in &project.qa_findings {
        let key = format!("{:?}", finding.severity).to_ascii_lowercase();
        *qa_summary.entry(key).or_insert(0usize) += 1;
    }
    let payload = json!({
        "schema":"creative-project-handoff-v1",
        "label":label,
        "project_id":project.project_id,
        "title":project.title,
        "intent":project.intent,
        "tracks":project.tracks,
        "target":project.target,
        "selected_elements":selected_elements,
        "accepted_assets":accepted_assets,
        "scene_boards":project.scene_boards,
        "scenes":project.scenes,
        "games":project.games,
        "audio_plans":project.audio_plans,
        "graph_ids":project.graph_ids,
        "job_ids":project.job_ids,
        "qa_summary":qa_summary,
        "qa_findings":project.qa_findings,
        "blender_root":"blender/",
        "state_root":format!(".masihawam/creative/projects/{}", job.project_id),
        "deploy_performed":false,
        "publish_performed":false
    });
    Ok((
        "creative_project_handoff",
        "creative_project_handoff",
        payload,
        None,
    ))
}

fn packaged_payload(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
    workflow_id: &str,
) -> Result<(&'static str, &'static str, Value, Option<String>), McpError> {
    let ids = asset_ids(&job.execution_parameters, "asset_ids", 1, 64)?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut assets = Vec::with_capacity(ids.len());
    for id in &ids {
        let asset = project.asset(id).ok_or_else(|| {
            McpError::InvalidRequest("delivery template references an unknown Asset".into())
        })?;
        if asset.state != AssetState::Accepted {
            return Err(McpError::InvalidRequest(
                "delivery templates require accepted Assets".into(),
            ));
        }
        assets.push(json!({
            "asset_id":asset.asset_id,
            "role":asset.role,
            "media_type":asset.media_type,
            "checksum_sha256":asset.checksum_sha256,
            "relative_path":asset.relative_path
        }));
    }
    let template = match workflow_id {
        "promo_pack" => "anime_game_promo_pack",
        "key_visual_bundle" => "key_visual_cover_thumbnail_bundle",
        "portfolio_delivery" => "editable_portfolio_delivery",
        _ => return Err(McpError::InvalidRequest("unknown delivery template".into())),
    };
    let payload = json!({
        "schema":"creative-packaged-delivery-v1",
        "template":template,
        "project_id":job.project_id,
        "assets":assets,
        "element_ids":job.execution_parameters.get("element_ids").cloned().unwrap_or_else(|| json!([])),
        "output_profile":job.execution_parameters.get("output_profile").cloned().unwrap_or_else(|| json!("contained")),
        "editable_source_required":workflow_id == "portfolio_delivery",
        "build_test_required":workflow_id == "portfolio_delivery",
        "deploy_performed":false,
        "publish_performed":false,
        "publish_requires_distinct_action":true
    });
    Ok((
        "creative_delivery_template",
        "creative_packaged_delivery",
        payload,
        ids.first().cloned(),
    ))
}

fn asset_ids(value: &Value, field: &str, min: usize, max: usize) -> Result<Vec<String>, McpError> {
    let values = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest(format!("{field} is required")))?;
    if values.len() < min || values.len() > max {
        return Err(McpError::InvalidRequest(format!(
            "{field} count is outside allowed bounds"
        )));
    }
    values
        .iter()
        .map(|value| {
            let id = value
                .as_str()
                .ok_or_else(|| McpError::InvalidRequest(format!("{field} is invalid")))?;
            super::super::contracts::validate_id(id, field)?;
            Ok(id.to_owned())
        })
        .collect()
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, McpError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| McpError::InvalidRequest(format!("{field} is required")))
}
