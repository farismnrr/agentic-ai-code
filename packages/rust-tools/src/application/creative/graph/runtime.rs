use super::{
    validate_graph, CreativeGraph, CreativeJobRecord, CreativeJobStatus, GraphNode, GraphNodeKind,
    NodeRunRecord,
};
use crate::application::creative::contracts::{CreativeProject, CREATIVE_SCHEMA_VERSION};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

pub fn execute_graph(
    graph: &CreativeGraph,
    project: &CreativeProject,
    config: &ServerConfig,
    owner: &str,
    now_ms: u128,
) -> Result<CreativeJobRecord, McpError> {
    let validation = validate_graph(graph, project, config)?;
    let job_id = format!("job_{}", Uuid::new_v4().simple());
    let mut job = CreativeJobRecord {
        schema_version: CREATIVE_SCHEMA_VERSION,
        job_id,
        project_id: project.project_id.clone(),
        owner: owner.to_owned(),
        kind: super::CreativeJobKind::Graph,
        graph_id: Some(graph.graph_id.clone()),
        capability_id: None,
        workflow_id: None,
        execution_binding_id: None,
        execution_parameters: Value::Object(Default::default()),
        status: CreativeJobStatus::Running,
        estimate: None,
        approved: true,
        retry_count: 0,
        max_retries: 0,
        timeout_ms: 0,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        node_runs: Vec::new(),
        output_asset_ids: Vec::new(),
        failure_code: None,
    };
    if !validation.valid {
        job.status = CreativeJobStatus::Failed;
        job.failure_code = Some("graph_validation_failed".into());
        return Ok(job);
    }

    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let incoming = incoming_edges(graph);
    let mut outputs = HashMap::<String, Value>::new();

    for node_id in validation.topological_order {
        let node = nodes
            .get(node_id.as_str())
            .expect("validated topological node exists");
        let upstream = incoming
            .get(node.node_id.as_str())
            .into_iter()
            .flatten()
            .filter_map(|source| outputs.get(*source).cloned())
            .collect::<Vec<_>>();
        match execute_node(node, project, upstream) {
            Ok(output) => {
                outputs.insert(node.node_id.clone(), output.clone());
                job.node_runs.push(NodeRunRecord {
                    node_id: node.node_id.clone(),
                    status: CreativeJobStatus::Completed,
                    output: Some(output),
                    failure_code: None,
                });
            }
            Err(code) => {
                job.node_runs.push(NodeRunRecord {
                    node_id: node.node_id.clone(),
                    status: CreativeJobStatus::Failed,
                    output: None,
                    failure_code: Some(code.clone()),
                });
                job.status = CreativeJobStatus::Failed;
                job.failure_code = Some(code);
                return Ok(job);
            }
        }
    }

    job.status = CreativeJobStatus::Completed;
    job.updated_at_ms = now_ms;
    Ok(job)
}

fn execute_node(
    node: &GraphNode,
    project: &CreativeProject,
    upstream: Vec<Value>,
) -> Result<Value, String> {
    match node.kind {
        GraphNodeKind::InputText => Ok(json!({
            "text": node.inputs.get("text").cloned().unwrap_or(Value::Null)
        })),
        GraphNodeKind::InputAsset => {
            let asset_id = node
                .inputs
                .get("asset_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "asset_reference_invalid".to_owned())?;
            let asset = project
                .asset(asset_id)
                .ok_or_else(|| "asset_reference_invalid".to_owned())?;
            Ok(json!({"asset_id": asset.asset_id, "role": asset.role}))
        }
        GraphNodeKind::ElementRef => {
            let element_id = node
                .inputs
                .get("element_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "element_reference_invalid".to_owned())?;
            let element = project
                .element(element_id)
                .ok_or_else(|| "element_reference_invalid".to_owned())?;
            Ok(json!({
                "element_id": element.element_id,
                "selected_revision_id": element.selected_revision_id
            }))
        }
        GraphNodeKind::SelectRevision => Ok(json!({
            "element_id": node.inputs.get("element_id").cloned().unwrap_or(Value::Null),
            "revision_id": node.inputs.get("revision_id").cloned().unwrap_or(Value::Null)
        })),
        GraphNodeKind::ExternalReviewGate => {
            if node.inputs.get("approved").and_then(Value::as_bool) == Some(true) {
                Ok(json!({"approved": true, "upstream": upstream}))
            } else {
                Err("external_review_required".into())
            }
        }
        GraphNodeKind::VisualEvidence | GraphNodeKind::TemporalEvidence | GraphNodeKind::GameQa => {
            Ok(json!({
                "inspection": "not_inspected",
                "upstream": upstream
            }))
        }
        GraphNodeKind::GenerateImage
        | GraphNodeKind::GenerateVideo
        | GraphNodeKind::GenerateAudio
        | GraphNodeKind::Generate3d
        | GraphNodeKind::StoryboardStore
        | GraphNodeKind::SceneManifestValidate
        | GraphNodeKind::DccExecute
        | GraphNodeKind::DccRender
        | GraphNodeKind::GameBuild
        | GraphNodeKind::GamePlaytest
        | GraphNodeKind::GameDeploy
        | GraphNodeKind::AssembleSequence
        | GraphNodeKind::ExportArtifact => Err("execution_not_implemented".into()),
    }
}

fn incoming_edges(graph: &CreativeGraph) -> HashMap<&str, Vec<&str>> {
    let mut result: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        result
            .entry(edge.to_node.as_str())
            .or_default()
            .push(edge.from_node.as_str());
    }
    result
}
