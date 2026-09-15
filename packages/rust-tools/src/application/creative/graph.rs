use serde::{Deserialize, Serialize};
use serde_json::Value;

mod job_validation;
mod runtime;
mod validation;
pub use job_validation::validate_job_record;
pub use runtime::{dirty_descendants, execute_graph, execute_graph_partial};
pub use validation::{capability_for_node, validate_graph, validate_graph_template};

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
    pub actual_output_bytes: Option<u64>,
    #[serde(default)]
    pub failure_code: Option<String>,
}

fn default_job_owner() -> String {
    "legacy".into()
}
