use super::super::contracts::{
    AssetMetadata, AssetSource, AssetState, AssetSurface, QaFinding, QaSeverity,
};
use super::super::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use uuid::Uuid;

pub(super) fn execute_local(
    cwd: Option<&str>,
    config: &ServerConfig,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("scene workflow id is required".into()))?;
    let output = match workflow_id {
        "scene_continuity_review" => continuity(cwd, config, &job)?,
        "visual_qa_evidence" => qa_evidence(cwd, config, &job, false)?,
        "temporal_qa_evidence" => qa_evidence(cwd, config, &job, true)?,
        "scoped_revision" => scoped_revision(cwd, config, &job)?,
        _ => {
            return Err(McpError::InvalidRequest(
                "unsupported local scene workflow".into(),
            ))
        }
    };
    let asset_id = output
        .get("asset_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    job.status = CreativeJobStatus::Completed;
    job.failure_code = None;
    job.output_asset_ids = asset_id.clone().into_iter().collect();
    job.actual_output_bytes = asset_id.as_deref().and_then(|id| {
        store::load_project(cwd, config, &job.project_id)
            .ok()
            .and_then(|project| project.asset(id).map(|asset| asset.bytes))
    });
    job.node_runs = vec![NodeRunRecord {
        node_id: workflow_id.into(),
        status: CreativeJobStatus::Completed,
        output: Some(output),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn continuity(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Value, McpError> {
    let scene_id = required_str(&job.execution_parameters, "scene_id")?;
    let mut project = store::load_project(cwd, config, &job.project_id)?;
    let scene = project
        .scenes
        .iter()
        .find(|scene| scene.scene_id == scene_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown scene manifest".into()))?;
    let mut shots = scene.shots.clone();
    shots.sort_by_key(|shot| shot.order);
    let mut findings = Vec::new();
    for pair in shots.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        if previous.state_out != Value::Null
            && next.state_in != Value::Null
            && previous.state_out != next.state_in
        {
            findings.push(json!({"code":"state_handoff_mismatch","from_shot":previous.shot_id,"to_shot":next.shot_id}));
        }
        if previous.location_variant_id != next.location_variant_id
            && scene.location_element_id.is_some()
        {
            findings.push(json!({"code":"location_variant_changed","from_shot":previous.shot_id,"to_shot":next.shot_id}));
        }
        let previous_direction = previous.continuity.get("screen_direction");
        let next_direction = next.continuity.get("screen_direction");
        if previous_direction.is_some()
            && next_direction.is_some()
            && previous_direction != next_direction
        {
            findings.push(json!({"code":"screen_direction_changed","from_shot":previous.shot_id,"to_shot":next.shot_id}));
        }
    }
    if findings.is_empty() {
        findings.push(json!({"code":"continuity_not_visually_inspected"}));
    }
    let receipt = json!({
        "schema":"scene-continuity-evidence-v1",
        "scene_id":scene_id,
        "scene_revision_id":scene.revision_id,
        "selected_hero_frame_asset_id":scene.selected_hero_frame_asset_id,
        "findings":findings,
        "visual_inspection":"not_inspected"
    });
    let asset_id = write_receipt(
        cwd,
        config,
        job,
        "continuity",
        "scene_continuity_evidence",
        &receipt,
        None,
    )?;
    project = store::load_project(cwd, config, &job.project_id)?;
    let finding = QaFinding {
        finding_id: format!("qa_{}", Uuid::new_v4().simple()),
        domain: "scene_continuity".into(),
        severity: QaSeverity::NotInspected,
        subject_id: scene_id.to_owned(),
        message:
            "Deterministic continuity evidence generated; visual continuity remains not inspected."
                .into(),
        source_revision_id: scene.revision_id,
        asset_id: Some(asset_id.clone()),
        evaluator_binding_id: None,
        evidence: receipt.clone(),
        created_at_ms: store::now_ms(),
    };
    let _ = project;
    store::add_qa_finding(cwd, config, &job.project_id, finding)?;
    Ok(json!({"asset_id":asset_id,"evidence":receipt}))
}

fn qa_evidence(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
    temporal: bool,
) -> Result<Value, McpError> {
    let subject_id = required_str(&job.execution_parameters, "subject_id")?;
    let source_revision_id = job
        .execution_parameters
        .get("source_revision_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let source_asset_id = job
        .execution_parameters
        .get("asset_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let evaluator_binding_id = job
        .execution_parameters
        .get("evaluator_binding_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let deterministic = if temporal {
        json!({
            "duration_ms":job.execution_parameters.get("duration_ms").cloned(),
            "sample_frames":job.execution_parameters.get("sample_frames").cloned().unwrap_or_else(|| json!([])),
            "timing_inspection":"structural_only"
        })
    } else {
        json!({
            "reference_asset_ids":job.execution_parameters.get("reference_asset_ids").cloned().unwrap_or_else(|| json!([])),
            "preview_asset_ids":job.execution_parameters.get("preview_asset_ids").cloned().unwrap_or_else(|| json!([])),
            "visual_inspection":"structural_only"
        })
    };
    let semantic_inspection = if evaluator_binding_id.is_some() {
        "evaluator_requested"
    } else {
        "not_inspected"
    };
    let receipt = json!({
        "schema":if temporal {"temporal-qa-evidence-v1"} else {"visual-qa-evidence-v1"},
        "subject_id":subject_id,
        "source_revision_id":source_revision_id,
        "source_asset_id":source_asset_id,
        "deterministic":deterministic,
        "semantic_inspection":semantic_inspection,
        "evaluator_binding_id":evaluator_binding_id
    });
    let domain = if temporal { "temporal_qa" } else { "visual_qa" };
    let asset_id = write_receipt(
        cwd,
        config,
        job,
        domain,
        domain,
        &receipt,
        source_asset_id.as_deref(),
    )?;
    store::add_qa_finding(
        cwd,
        config,
        &job.project_id,
        QaFinding {
            finding_id: format!("qa_{}", Uuid::new_v4().simple()),
            domain: domain.into(),
            severity: QaSeverity::NotInspected,
            subject_id: subject_id.into(),
            message: "Deterministic QA evidence generated; semantic judgment requires explicit review/evaluator.".into(),
            source_revision_id,
            asset_id: Some(asset_id.clone()),
            evaluator_binding_id,
            evidence: receipt.clone(),
            created_at_ms: store::now_ms(),
        },
    )?;
    Ok(json!({"asset_id":asset_id,"evidence":receipt}))
}

fn scoped_revision(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Value, McpError> {
    let parent_asset_id = required_str(&job.execution_parameters, "parent_asset_id")?;
    let changed_scope = required_str(&job.execution_parameters, "changed_scope")?;
    let locked_fields = job
        .execution_parameters
        .get("locked_fields")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let requested_change = job
        .execution_parameters
        .get("requested_change")
        .cloned()
        .unwrap_or(Value::Null);
    let project = store::load_project(cwd, config, &job.project_id)?;
    let parent = project.asset(parent_asset_id).ok_or_else(|| {
        McpError::InvalidRequest("scoped revision parent Asset is unknown".into())
    })?;
    let receipt = json!({
        "schema":"scoped-revision-v1",
        "parent_asset_id":parent_asset_id,
        "parent_checksum_sha256":parent.checksum_sha256,
        "changed_scope":changed_scope,
        "locked_fields":locked_fields,
        "requested_change":requested_change,
        "unrelated_state_preserved":true
    });
    let asset_id = write_receipt(
        cwd,
        config,
        job,
        "revisions",
        "scoped_revision_receipt",
        &receipt,
        Some(parent_asset_id),
    )?;
    Ok(
        json!({"asset_id":asset_id,"parent_asset_id":parent_asset_id,"changed_scope":changed_scope,"locked_fields":locked_fields}),
    )
}

fn write_receipt(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
    folder: &str,
    role: &str,
    value: &Value,
    parent_asset_id: Option<&str>,
) -> Result<String, McpError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| McpError::Internal("creative evidence encoding failed".into()))?;
    let relative_path = format!(
        "creative/{}/qa/{folder}/{}.json",
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
        AssetRegistrationInput {
            path: relative_path,
            media_type: "application/json".into(),
            role: role.into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: parent_asset_id.map(str::to_owned),
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some(role.into()),
                artifact_version: Some("1".into()),
                ..AssetMetadata::default()
            },
        },
    )?;
    Ok(asset_id)
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, McpError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))
}
