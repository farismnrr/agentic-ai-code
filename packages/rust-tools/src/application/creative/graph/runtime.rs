use super::{
    validate_graph, CreativeGraph, CreativeJobRecord, CreativeJobStatus, GraphNode, GraphNodeKind,
    NodeRunRecord,
};
use crate::application::creative::contracts::{CreativeProject, CREATIVE_SCHEMA_VERSION};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use uuid::Uuid;

const MAX_PARALLEL_GRAPH_NODES: usize = 16;

pub fn execute_graph(
    graph: &CreativeGraph,
    project: &CreativeProject,
    config: &ServerConfig,
    owner: &str,
    now_ms: u128,
) -> Result<CreativeJobRecord, McpError> {
    execute_graph_internal(graph, project, config, owner, now_ms, None, None)
}

pub fn execute_graph_partial(
    graph: &CreativeGraph,
    project: &CreativeProject,
    config: &ServerConfig,
    owner: &str,
    previous: &CreativeJobRecord,
    changed_node_ids: &[String],
    now_ms: u128,
) -> Result<CreativeJobRecord, McpError> {
    if previous.status != CreativeJobStatus::Completed
        || previous.graph_id.as_deref() != Some(graph.graph_id.as_str())
        || previous.project_id != graph.project_id
    {
        return Err(McpError::InvalidRequest(
            "creative partial rerun requires a completed job for the same stored graph".into(),
        ));
    }
    let dirty = dirty_descendants(graph, changed_node_ids)?;
    execute_graph_internal(
        graph,
        project,
        config,
        owner,
        now_ms,
        Some(previous),
        Some(&dirty),
    )
}

pub fn dirty_descendants(
    graph: &CreativeGraph,
    changed_node_ids: &[String],
) -> Result<HashSet<String>, McpError> {
    if changed_node_ids.is_empty() {
        return Err(McpError::InvalidRequest(
            "creative partial rerun requires at least one changed node".into(),
        ));
    }
    let known = graph
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<HashSet<_>>();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.from_node.as_str())
            .or_default()
            .push(edge.to_node.as_str());
    }
    let mut dirty = HashSet::new();
    let mut queue = VecDeque::new();
    for node_id in changed_node_ids {
        if !known.contains(node_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative partial rerun references an unknown changed node".into(),
            ));
        }
        if dirty.insert(node_id.clone()) {
            queue.push_back(node_id.clone());
        }
    }
    while let Some(node_id) = queue.pop_front() {
        for child in outgoing.get(node_id.as_str()).into_iter().flatten() {
            if dirty.insert((*child).to_owned()) {
                queue.push_back((*child).to_owned());
            }
        }
    }
    Ok(dirty)
}

fn execute_graph_internal(
    graph: &CreativeGraph,
    project: &CreativeProject,
    config: &ServerConfig,
    owner: &str,
    now_ms: u128,
    previous: Option<&CreativeJobRecord>,
    dirty: Option<&HashSet<String>>,
) -> Result<CreativeJobRecord, McpError> {
    let validation = validate_graph(graph, project, config)?;
    let mut job = CreativeJobRecord {
        schema_version: CREATIVE_SCHEMA_VERSION,
        job_id: format!("job_{}", Uuid::new_v4().simple()),
        project_id: project.project_id.clone(),
        owner: owner.to_owned(),
        kind: super::CreativeJobKind::Graph,
        graph_id: Some(graph.graph_id.clone()),
        capability_id: None,
        workflow_id: None,
        execution_binding_id: None,
        execution_parameters: Value::Object(Default::default()),
        compiler_version: None,
        execution_binding_version: None,
        changed_fields: Vec::new(),
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
        actual_output_bytes: None,
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
    let batches = execution_batches(graph, &validation.topological_order);
    let previous_runs = previous
        .map(|record| {
            record
                .node_runs
                .iter()
                .map(|run| (run.node_id.as_str(), run))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut outputs = HashMap::<String, Value>::new();

    for (batch, node_ids) in batches {
        let mut execute = Vec::new();
        for node_id in node_ids {
            let node = nodes
                .get(node_id.as_str())
                .expect("validated scheduled node exists");
            let should_reuse = dirty.is_some_and(|set| !set.contains(&node.node_id));
            if should_reuse {
                if let Some(run) = previous_runs
                    .get(node.node_id.as_str())
                    .filter(|run| run.status == CreativeJobStatus::Completed)
                {
                    if let Some(output) = run.output.clone() {
                        outputs.insert(node.node_id.clone(), output.clone());
                        job.node_runs.push(NodeRunRecord {
                            node_id: node.node_id.clone(),
                            status: CreativeJobStatus::Completed,
                            output: Some(output),
                            failure_code: None,
                            reused: true,
                            execution_batch: batch,
                        });
                        continue;
                    }
                }
            }
            let upstream = incoming
                .get(node.node_id.as_str())
                .into_iter()
                .flatten()
                .filter_map(|source| outputs.get(*source).cloned())
                .collect::<Vec<_>>();
            execute.push((*node, upstream));
        }

        let mut results = Vec::with_capacity(execute.len());
        for wave in execute.chunks(MAX_PARALLEL_GRAPH_NODES) {
            let mut wave_results = std::thread::scope(|scope| {
                let handles = wave
                    .iter()
                    .map(|(node, upstream)| {
                        let upstream = upstream.clone();
                        scope.spawn(move || {
                            (node.node_id.clone(), execute_node(node, project, upstream))
                        })
                    })
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("creative graph node thread panicked"))
                    .collect::<Vec<_>>()
            });
            results.append(&mut wave_results);
        }
        results.sort_by(|left, right| left.0.cmp(&right.0));
        for (node_id, result) in results {
            match result {
                Ok(output) => {
                    outputs.insert(node_id.clone(), output.clone());
                    job.node_runs.push(NodeRunRecord {
                        node_id,
                        status: CreativeJobStatus::Completed,
                        output: Some(output),
                        failure_code: None,
                        reused: false,
                        execution_batch: batch,
                    });
                }
                Err(code) => {
                    job.node_runs.push(NodeRunRecord {
                        node_id,
                        status: CreativeJobStatus::Failed,
                        output: None,
                        failure_code: Some(code.clone()),
                        reused: false,
                        execution_batch: batch,
                    });
                    job.status = CreativeJobStatus::Failed;
                    job.failure_code = Some(code);
                    job.updated_at_ms = now_ms;
                    job.node_runs.sort_by(|left, right| {
                        left.execution_batch
                            .cmp(&right.execution_batch)
                            .then_with(|| left.node_id.cmp(&right.node_id))
                    });
                    return Ok(job);
                }
            }
        }
    }

    job.node_runs.sort_by(|left, right| {
        left.execution_batch
            .cmp(&right.execution_batch)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    job.status = CreativeJobStatus::Completed;
    job.updated_at_ms = now_ms;
    Ok(job)
}

fn execution_batches(
    graph: &CreativeGraph,
    topological_order: &[String],
) -> BTreeMap<u32, Vec<String>> {
    let incoming = incoming_edges(graph);
    let mut level_by_node = HashMap::<String, u32>::new();
    let mut batches = BTreeMap::<u32, Vec<String>>::new();
    for node_id in topological_order {
        let level = incoming
            .get(node_id.as_str())
            .into_iter()
            .flatten()
            .filter_map(|source| level_by_node.get(*source).copied())
            .max()
            .map_or(0, |level| level.saturating_add(1));
        level_by_node.insert(node_id.clone(), level);
        batches.entry(level).or_default().push(node_id.clone());
    }
    for nodes in batches.values_mut() {
        nodes.sort();
    }
    batches
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
