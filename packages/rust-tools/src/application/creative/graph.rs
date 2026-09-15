use super::contracts::{validate_id, validate_spec, CreativeProject, CREATIVE_SCHEMA_VERSION};
use super::registry;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};

mod runtime;
pub use runtime::{dirty_descendants, execute_graph, execute_graph_partial};

const MAX_GRAPH_NODES: usize = 256;
const MAX_GRAPH_EDGES: usize = 1_024;
const MAX_PORT_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    InputText,
    InputAsset,
    ElementRef,
    SelectRevision,
    ExternalReviewGate,
    GenerateImage,
    GenerateVideo,
    GenerateAudio,
    Generate3d,
    StoryboardStore,
    SceneManifestValidate,
    DccExecute,
    DccRender,
    GameBuild,
    GamePlaytest,
    GameDeploy,
    VisualEvidence,
    TemporalEvidence,
    GameQa,
    AssembleSequence,
    ExportArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub node_id: String,
    pub kind: GraphNodeKind,
    #[serde(default)]
    pub execution_binding_id: Option<String>,
    #[serde(default)]
    pub inputs: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreativeGraph {
    pub schema_version: u32,
    pub graph_id: String,
    pub project_id: String,
    pub revision: u32,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreativeGraphTemplate {
    pub schema_version: u32,
    pub template_id: String,
    pub project_id: String,
    pub version: u32,
    pub description: String,
    pub graph: CreativeGraph,
    #[serde(default)]
    pub input_node_ids: Vec<String>,
    #[serde(default)]
    pub output_node_ids: Vec<String>,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphValidation {
    pub valid: bool,
    #[serde(default)]
    pub topological_order: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<GraphDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreativeJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRunRecord {
    pub node_id: String,
    pub status: CreativeJobStatus,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub failure_code: Option<String>,
    #[serde(default)]
    pub reused: bool,
    #[serde(default)]
    pub execution_batch: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreativeJobKind {
    #[default]
    Graph,
    Capability,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreativeEstimate {
    pub compute_units: u64,
    pub output_bytes: u64,
    #[serde(default)]
    pub estimated_cost_micros: Option<u64>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreativeJobRecord {
    pub schema_version: u32,
    pub job_id: String,
    pub project_id: String,
    #[serde(default = "default_job_owner")]
    pub owner: String,
    #[serde(default)]
    pub kind: CreativeJobKind,
    #[serde(default)]
    pub graph_id: Option<String>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub execution_binding_id: Option<String>,
    #[serde(default)]
    pub execution_parameters: Value,
    pub status: CreativeJobStatus,
    #[serde(default)]
    pub estimate: Option<CreativeEstimate>,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub timeout_ms: u64,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
    #[serde(default)]
    pub node_runs: Vec<NodeRunRecord>,
    #[serde(default)]
    pub output_asset_ids: Vec<String>,
    #[serde(default)]
    pub failure_code: Option<String>,
}

fn default_job_owner() -> String {
    "legacy".into()
}

pub fn validate_graph(
    graph: &CreativeGraph,
    project: &CreativeProject,
    config: &ServerConfig,
) -> Result<GraphValidation, McpError> {
    if graph.schema_version != CREATIVE_SCHEMA_VERSION {
        return Err(McpError::InvalidRequest(
            "creative graph schema version is unsupported".into(),
        ));
    }
    validate_id(&graph.graph_id, "graph_id")?;
    validate_id(&graph.project_id, "project_id")?;
    if graph.project_id != project.project_id {
        return Err(McpError::InvalidRequest(
            "creative graph belongs to a different project".into(),
        ));
    }
    if graph.revision == 0
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_GRAPH_NODES
        || graph.edges.len() > MAX_GRAPH_EDGES
    {
        return Err(McpError::InvalidRequest(
            "creative graph size or revision exceeds allowed bounds".into(),
        ));
    }

    let mut diagnostics = Vec::new();
    let mut node_ids = HashSet::new();
    for node in &graph.nodes {
        validate_id(&node.node_id, "node_id")?;
        validate_spec(&node.inputs)?;
        validate_graph_inputs(&node.inputs, 0)?;
        if !node_ids.insert(node.node_id.as_str()) {
            diagnostics.push(diagnostic(
                "duplicate_node_id",
                "graph contains duplicate node identity",
                Some(&node.node_id),
            ));
        }
        validate_node(node, project, config, &mut diagnostics)?;
    }

    let mut indegree = graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        validate_port(&edge.from_port)?;
        validate_port(&edge.to_port)?;
        if edge.from_port != "output" || edge.to_port != "input" {
            diagnostics.push(diagnostic(
                "unsupported_edge_port",
                "minimal creative graph runtime currently supports output -> input edges only",
                None,
            ));
            continue;
        }
        if edge.from_node == edge.to_node {
            diagnostics.push(diagnostic(
                "self_edge",
                "graph edge cannot target the same node",
                Some(&edge.from_node),
            ));
            continue;
        }
        if !node_ids.contains(edge.from_node.as_str()) || !node_ids.contains(edge.to_node.as_str())
        {
            diagnostics.push(diagnostic(
                "unknown_edge_node",
                "graph edge references an unknown node",
                None,
            ));
            continue;
        }
        *indegree
            .get_mut(edge.to_node.as_str())
            .expect("validated graph node is present") += 1;
        outgoing
            .entry(edge.from_node.as_str())
            .or_default()
            .push(edge.to_node.as_str());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(*node))
        .collect::<Vec<_>>();
    ready.sort_unstable();
    let mut queue = VecDeque::from(ready);
    let mut order = Vec::with_capacity(graph.nodes.len());
    while let Some(node) = queue.pop_front() {
        order.push(node.to_owned());
        let mut next = outgoing.get(node).cloned().unwrap_or_default();
        next.sort_unstable();
        for target in next {
            if let Some(degree) = indegree.get_mut(target) {
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    queue.push_back(target);
                }
            }
        }
    }
    if order.len() != graph.nodes.len() {
        diagnostics.push(diagnostic(
            "graph_cycle",
            "creative graph must be acyclic",
            None,
        ));
        order.clear();
    }

    Ok(GraphValidation {
        valid: diagnostics.is_empty(),
        topological_order: order,
        diagnostics,
    })
}

pub fn validate_graph_template(
    template: &CreativeGraphTemplate,
    project: &CreativeProject,
    config: &ServerConfig,
) -> Result<(), McpError> {
    if template.schema_version != CREATIVE_SCHEMA_VERSION {
        return Err(McpError::InvalidRequest(
            "creative graph template schema version is unsupported".into(),
        ));
    }
    validate_id(&template.template_id, "template_id")?;
    validate_id(&template.project_id, "project_id")?;
    if template.project_id != project.project_id || template.graph.project_id != project.project_id
    {
        return Err(McpError::InvalidRequest(
            "creative graph template belongs to a different project".into(),
        ));
    }
    if template.version == 0
        || template.description.len() > 1024
        || template.description.chars().any(char::is_control)
        || template.input_node_ids.len() > MAX_GRAPH_NODES
        || template.output_node_ids.is_empty()
        || template.output_node_ids.len() > MAX_GRAPH_NODES
    {
        return Err(McpError::InvalidRequest(
            "creative graph template exceeds allowed bounds".into(),
        ));
    }
    let validation = validate_graph(&template.graph, project, config)?;
    if !validation.valid {
        return Err(McpError::InvalidRequest(
            "creative graph template contains an invalid graph".into(),
        ));
    }
    let nodes = template
        .graph
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    for node_id in &template.input_node_ids {
        validate_id(node_id, "template input node id")?;
        if !seen.insert(node_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative graph template contains duplicate input node ids".into(),
            ));
        }
        let node = nodes
            .get(node_id.as_str())
            .ok_or_else(|| McpError::InvalidRequest("unknown template input node".into()))?;
        if !matches!(
            node.kind,
            GraphNodeKind::InputText | GraphNodeKind::InputAsset | GraphNodeKind::ElementRef
        ) {
            return Err(McpError::InvalidRequest(
                "creative graph template inputs must reference declared input nodes".into(),
            ));
        }
    }
    seen.clear();
    for node_id in &template.output_node_ids {
        validate_id(node_id, "template output node id")?;
        if !seen.insert(node_id.as_str()) || !nodes.contains_key(node_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "creative graph template output node is invalid or duplicated".into(),
            ));
        }
    }
    Ok(())
}

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

pub fn capability_for_node(kind: &GraphNodeKind) -> Option<&'static str> {
    match kind {
        GraphNodeKind::GenerateImage => Some("image.generate"),
        GraphNodeKind::GenerateVideo => Some("video.generate"),
        GraphNodeKind::GenerateAudio => Some("audio.voice"),
        GraphNodeKind::Generate3d => Some("3d.image_to_mesh"),
        GraphNodeKind::StoryboardStore => Some("storyboard.store"),
        GraphNodeKind::SceneManifestValidate => Some("scene.manifest_validate"),
        GraphNodeKind::DccExecute => Some("dcc.execute"),
        GraphNodeKind::DccRender => Some("dcc.render"),
        GraphNodeKind::GameBuild => Some("game.build"),
        GraphNodeKind::GamePlaytest => Some("game.playtest"),
        GraphNodeKind::GameDeploy => Some("game.deploy"),
        _ => None,
    }
}

fn validate_node(
    node: &GraphNode,
    project: &CreativeProject,
    config: &ServerConfig,
    diagnostics: &mut Vec<GraphDiagnostic>,
) -> Result<(), McpError> {
    match node.kind {
        GraphNodeKind::InputText => {
            if node.inputs.get("text").and_then(Value::as_str).is_none() {
                diagnostics.push(diagnostic(
                    "input_text_required",
                    "InputText requires inputs.text",
                    Some(&node.node_id),
                ));
            }
        }
        GraphNodeKind::InputAsset => {
            let asset_id = node.inputs.get("asset_id").and_then(Value::as_str);
            if asset_id.is_none_or(|id| project.asset(id).is_none()) {
                diagnostics.push(diagnostic(
                    "asset_reference_invalid",
                    "InputAsset requires a project asset_id",
                    Some(&node.node_id),
                ));
            }
        }
        GraphNodeKind::ElementRef => {
            let element_id = node.inputs.get("element_id").and_then(Value::as_str);
            if element_id.is_none_or(|id| project.element(id).is_none()) {
                diagnostics.push(diagnostic(
                    "element_reference_invalid",
                    "ElementRef requires a project element_id",
                    Some(&node.node_id),
                ));
            }
        }
        GraphNodeKind::SelectRevision => {
            let element_id = node.inputs.get("element_id").and_then(Value::as_str);
            let revision_id = node.inputs.get("revision_id").and_then(Value::as_str);
            let exists = element_id
                .and_then(|id| project.element(id))
                .zip(revision_id)
                .is_some_and(|(element, revision)| {
                    element
                        .revisions
                        .iter()
                        .any(|value| value.revision_id == revision)
                });
            if !exists {
                diagnostics.push(diagnostic(
                    "revision_reference_invalid",
                    "SelectRevision requires an existing element revision",
                    Some(&node.node_id),
                ));
            }
        }
        _ => {}
    }

    if let Some(capability_id) = capability_for_node(&node.kind) {
        if registry::capability(capability_id).is_none() {
            diagnostics.push(diagnostic(
                "capability_unavailable",
                "graph node references an unavailable capability",
                Some(&node.node_id),
            ));
        } else if registry::capability(capability_id)
            .is_some_and(|descriptor| descriptor.requires_execution_binding)
        {
            match node.execution_binding_id.as_deref() {
                None => diagnostics.push(diagnostic(
                    "execution_binding_required",
                    "pluggable graph node requires caller-selected execution_binding_id",
                    Some(&node.node_id),
                )),
                Some(binding_id) => match registry::binding(config, binding_id)? {
                    None => diagnostics.push(diagnostic(
                        "execution_binding_unavailable",
                        "selected execution binding is unavailable",
                        Some(&node.node_id),
                    )),
                    Some(binding)
                        if binding.availability != registry::BindingAvailability::Available =>
                    {
                        diagnostics.push(diagnostic(
                            "execution_binding_unavailable",
                            "selected execution binding is unavailable",
                            Some(&node.node_id),
                        ));
                    }
                    Some(binding)
                        if !binding
                            .capabilities
                            .iter()
                            .any(|value| value == capability_id) =>
                    {
                        diagnostics.push(diagnostic(
                            "execution_binding_incompatible",
                            "selected execution binding does not support this capability",
                            Some(&node.node_id),
                        ));
                    }
                    Some(_) => {}
                },
            }
        }
    }
    Ok(())
}

fn validate_graph_inputs(value: &Value, depth: usize) -> Result<(), McpError> {
    if depth > 8 {
        return Err(McpError::InvalidRequest(
            "creative graph input nesting exceeds maximum".into(),
        ));
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let normalized = key.to_ascii_lowercase().replace(['-', '.'], "_");
                if [
                    "endpoint",
                    "url",
                    "api_key",
                    "apikey",
                    "token",
                    "secret",
                    "password",
                    "authorization",
                    "credential",
                    "credentials",
                    "headers",
                    "command",
                    "code",
                    "script",
                    "executable",
                    "environment",
                    "env",
                    "socket",
                ]
                .iter()
                .any(|forbidden| {
                    normalized == *forbidden
                        || normalized.starts_with(&format!("{forbidden}_"))
                        || normalized.ends_with(&format!("_{forbidden}"))
                }) {
                    return Err(McpError::InvalidRequest(
                        "creative graph inputs cannot encode endpoints, credentials, or executable payloads"
                            .into(),
                    ));
                }
                validate_graph_inputs(child, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                validate_graph_inputs(child, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_port(value: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.len() > MAX_PORT_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(McpError::InvalidRequest(
            "graph port identity exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}

fn diagnostic(code: &str, message: &str, node_id: Option<&str>) -> GraphDiagnostic {
    GraphDiagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        node_id: node_id.map(str::to_owned),
    }
}
