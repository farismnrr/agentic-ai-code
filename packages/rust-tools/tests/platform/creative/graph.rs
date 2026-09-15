use super::{call, create_project, dispatch_sync, TempWorkspace};
use ai_tools::application::creative::{
    CreativeGraph, GraphEdge, GraphNode, GraphNodeKind, CREATIVE_SCHEMA_VERSION,
};
use serde_json::json;

#[test]
fn graph_runtime_persists_control_flow_jobs_and_requires_explicit_bindings() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_graph");

    let graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_control".into(),
        project_id: "project_graph".into(),
        revision: 1,
        nodes: vec![
            GraphNode {
                node_id: "brief".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text": "approved caller-authored brief"}),
            },
            GraphNode {
                node_id: "review".into(),
                kind: GraphNodeKind::ExternalReviewGate,
                execution_binding_id: None,
                inputs: json!({"approved": true}),
            },
            GraphNode {
                node_id: "qa".into(),
                kind: GraphNodeKind::VisualEvidence,
                execution_binding_id: None,
                inputs: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                from_node: "brief".into(),
                from_port: "output".into(),
                to_node: "review".into(),
                to_port: "input".into(),
            },
            GraphEdge {
                from_node: "review".into(),
                from_port: "output".into(),
                to_node: "qa".into(),
                to_port: "input".into(),
            },
        ],
    };

    let validation = call(
        &config,
        "creative_graph",
        json!({"action": "validate", "graph": graph}),
    );
    assert_eq!(validation["validation"]["valid"], true);

    let executed = call(
        &config,
        "creative_graph",
        json!({"action": "execute", "graph": graph}),
    );
    let job_id = executed["job"]["job_id"].as_str().unwrap().to_owned();
    assert_eq!(executed["job"]["status"], "completed");
    assert_eq!(
        executed["job"]["node_runs"].as_array().map(Vec::len),
        Some(3)
    );

    let stored = call(
        &config,
        "creative_job",
        json!({
            "action": "get",
            "project_id": "project_graph",
            "job_id": job_id
        }),
    );
    assert_eq!(stored["job"]["status"], "completed");

    let generated_graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_generate".into(),
        project_id: "project_graph".into(),
        revision: 1,
        nodes: vec![GraphNode {
            node_id: "generate".into(),
            kind: GraphNodeKind::GenerateImage,
            execution_binding_id: None,
            inputs: json!({"prompt": "caller authored"}),
        }],
        edges: vec![],
    };
    let missing_binding = call(
        &config,
        "creative_graph",
        json!({"action": "validate", "graph": generated_graph}),
    );
    assert_eq!(missing_binding["validation"]["valid"], false);
    assert!(missing_binding["validation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "execution_binding_required"));

    let unavailable_graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_unavailable_binding".into(),
        project_id: "project_graph".into(),
        revision: 1,
        nodes: vec![GraphNode {
            node_id: "generate".into(),
            kind: GraphNodeKind::GenerateImage,
            execution_binding_id: Some("missing_binding".into()),
            inputs: json!({"prompt": "caller authored"}),
        }],
        edges: vec![],
    };
    let unavailable_binding = call(
        &config,
        "creative_graph",
        json!({"action": "validate", "graph": unavailable_graph}),
    );
    assert_eq!(unavailable_binding["validation"]["valid"], false);
    assert!(unavailable_binding["validation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "execution_binding_unavailable"));

    let cyclic_graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_cycle".into(),
        project_id: "project_graph".into(),
        revision: 1,
        nodes: vec![
            GraphNode {
                node_id: "a".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text": "a"}),
            },
            GraphNode {
                node_id: "b".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text": "b"}),
            },
        ],
        edges: vec![
            GraphEdge {
                from_node: "a".into(),
                from_port: "output".into(),
                to_node: "b".into(),
                to_port: "input".into(),
            },
            GraphEdge {
                from_node: "b".into(),
                from_port: "output".into(),
                to_node: "a".into(),
                to_port: "input".into(),
            },
        ],
    };
    let cyclic = call(
        &config,
        "creative_graph",
        json!({"action": "validate", "graph": cyclic_graph}),
    );
    assert_eq!(cyclic["validation"]["valid"], false);
    assert!(cyclic["validation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "graph_cycle"));

    let unsafe_graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_unsafe".into(),
        project_id: "project_graph".into(),
        revision: 1,
        nodes: vec![GraphNode {
            node_id: "unsafe".into(),
            kind: GraphNodeKind::GenerateImage,
            execution_binding_id: None,
            inputs: json!({
                "prompt": "caller authored",
                "endpoint": "https://untrusted.example",
                "token": "must-not-enter-graph-state"
            }),
        }],
        edges: vec![],
    };
    assert!(dispatch_sync(
        &config,
        "creative_graph",
        &json!({"action": "validate", "graph": unsafe_graph}),
    )
    .is_err());
}
