use super::super::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::super::store;
use crate::application::blender::run_character_workflow;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};

pub(super) async fn execute(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    mut job: CreativeJobRecord,
) -> Result<CreativeJobRecord, McpError> {
    if !config.enable_creative {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("blender_capability_disabled".into());
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    let workflow_id = job
        .workflow_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("character workflow id is required".into()))?;
    let result = run_character_workflow(
        cwd,
        config,
        owner,
        &job.project_id,
        workflow_id,
        &job.execution_parameters,
    )
    .await?;
    let mut output_asset_ids = Vec::new();
    collect_asset_id(&result, "/export/asset_id", &mut output_asset_ids);
    collect_asset_id(&result, "/checkpoint/asset_id", &mut output_asset_ids);
    collect_asset_id(
        &result,
        "/checkpoint/checkpoint_asset_id",
        &mut output_asset_ids,
    );
    output_asset_ids.sort();
    output_asset_ids.dedup();
    let project = store::load_project(cwd, config, &job.project_id)?;
    let actual_output_bytes = output_asset_ids
        .iter()
        .filter_map(|asset_id| project.asset(asset_id).map(|asset| asset.bytes))
        .fold(0u64, u64::saturating_add);
    if actual_output_bytes > config.creative_max_job_output_bytes {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("output_hard_limit_exceeded".into());
        job.actual_output_bytes = Some(actual_output_bytes);
        job.updated_at_ms = store::now_ms();
        return Ok(job);
    }
    job.status = CreativeJobStatus::Completed;
    job.failure_code = None;
    job.output_asset_ids = output_asset_ids;
    job.actual_output_bytes = Some(actual_output_bytes);
    job.node_runs = vec![NodeRunRecord {
        node_id: workflow_id.to_owned(),
        status: CreativeJobStatus::Completed,
        output: Some(json!({
            "workflow":workflow_id,
            "evidence":result,
            "subjective_review":"not_inspected"
        })),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    job.updated_at_ms = store::now_ms();
    Ok(job)
}

fn collect_asset_id(value: &Value, pointer: &str, output: &mut Vec<String>) {
    if let Some(asset_id) = value.pointer(pointer).and_then(Value::as_str) {
        output.push(asset_id.to_owned());
    }
}
