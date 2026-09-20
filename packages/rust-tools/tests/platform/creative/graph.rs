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

#[test]
fn partial_rerun_invalidates_only_descendants_and_reuses_independent_outputs() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_partial_rerun");

    let graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_parallel".into(),
        project_id: "project_partial_rerun".into(),
        revision: 1,
        nodes: vec![
            GraphNode {
                node_id: "left".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text":"left-v1"}),
            },
            GraphNode {
                node_id: "right".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text":"right-v1"}),
            },
            GraphNode {
                node_id: "left_review".into(),
                kind: GraphNodeKind::ExternalReviewGate,
                execution_binding_id: None,
                inputs: json!({"approved":true}),
            },
            GraphNode {
                node_id: "right_review".into(),
                kind: GraphNodeKind::ExternalReviewGate,
                execution_binding_id: None,
                inputs: json!({"approved":true}),
            },
        ],
        edges: vec![
            GraphEdge {
                from_node: "left".into(),
                from_port: "output".into(),
                to_node: "left_review".into(),
                to_port: "input".into(),
            },
            GraphEdge {
                from_node: "right".into(),
                from_port: "output".into(),
                to_node: "right_review".into(),
                to_port: "input".into(),
            },
        ],
    };

    let first = call(
        &config,
        "creative_graph",
        json!({"action":"execute","graph":graph}),
    );
    let first_job_id = first["job"]["job_id"].as_str().unwrap().to_owned();
    let runs = first["job"]["node_runs"].as_array().unwrap();
    assert_eq!(
        runs.iter()
            .filter(|run| run["execution_batch"] == 0)
            .count(),
        2
    );
    assert_eq!(
        runs.iter()
            .filter(|run| run["execution_batch"] == 1)
            .count(),
        2
    );

    let rerun = call(
        &config,
        "creative_graph",
        json!({
            "action":"partial_rerun",
            "project_id":"project_partial_rerun",
            "graph_id":"graph_parallel",
            "previous_job_id":first_job_id,
            "changed_node_ids":["left"]
        }),
    );
    assert_eq!(rerun["job"]["status"], "completed");
    assert_eq!(
        rerun["invalidated_node_ids"],
        json!(["left", "left_review"])
    );
    assert_eq!(rerun["reused_node_ids"], json!(["right", "right_review"]));
    let rerun_runs = rerun["job"]["node_runs"].as_array().unwrap();
    assert!(rerun_runs
        .iter()
        .find(|run| run["node_id"] == "right")
        .is_some_and(|run| run["reused"] == true));
    assert!(rerun_runs
        .iter()
        .find(|run| run["node_id"] == "left")
        .is_some_and(|run| run["reused"] == false));
}

#[test]
fn graph_templates_are_immutable_project_scoped_snapshots_with_declared_io() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_template");

    let graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_template_source".into(),
        project_id: "project_template".into(),
        revision: 1,
        nodes: vec![
            GraphNode {
                node_id: "brief".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text":"template brief"}),
            },
            GraphNode {
                node_id: "review".into(),
                kind: GraphNodeKind::ExternalReviewGate,
                execution_binding_id: None,
                inputs: json!({"approved":true}),
            },
        ],
        edges: vec![GraphEdge {
            from_node: "brief".into(),
            from_port: "output".into(),
            to_node: "review".into(),
            to_port: "input".into(),
        }],
    };

    let saved = call(
        &config,
        "creative_graph",
        json!({
            "action":"template_save",
            "template_id":"template_reviewed_brief",
            "description":"Reusable reviewed brief graph",
            "graph":graph,
            "input_node_ids":["brief"],
            "output_node_ids":["review"]
        }),
    );
    assert_eq!(saved["template"]["template_id"], "template_reviewed_brief");
    assert_eq!(saved["template"]["version"], 1);

    let loaded = call(
        &config,
        "creative_graph",
        json!({
            "action":"template_get",
            "project_id":"project_template",
            "template_id":"template_reviewed_brief"
        }),
    );
    assert_eq!(loaded["template"]["graph"]["revision"], 1);
    assert_eq!(loaded["template"]["input_node_ids"], json!(["brief"]));
    assert_eq!(loaded["template"]["output_node_ids"], json!(["review"]));

    let listed = call(
        &config,
        "creative_graph",
        json!({"action":"template_list","project_id":"project_template"}),
    );
    assert_eq!(listed["templates"].as_array().map(Vec::len), Some(1));

    assert!(dispatch_sync(
        &config,
        "creative_graph",
        &json!({
            "action":"template_save",
            "template_id":"template_reviewed_brief",
            "graph":graph,
            "input_node_ids":["brief"],
            "output_node_ids":["review"]
        }),
    )
    .is_err());
}

#[test]
fn one_headless_graph_runtime_expresses_scene_anime_and_game_evidence_branches() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_cross_track_graph");

    let graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_cross_track".into(),
        project_id: "project_cross_track_graph".into(),
        revision: 1,
        nodes: vec![
            GraphNode {
                node_id: "brief".into(),
                kind: GraphNodeKind::InputText,
                execution_binding_id: None,
                inputs: json!({"text":"shared production brief"}),
            },
            GraphNode {
                node_id: "scene_evidence".into(),
                kind: GraphNodeKind::VisualEvidence,
                execution_binding_id: None,
                inputs: json!({}),
            },
            GraphNode {
                node_id: "anime_evidence".into(),
                kind: GraphNodeKind::TemporalEvidence,
                execution_binding_id: None,
                inputs: json!({}),
            },
            GraphNode {
                node_id: "game_evidence".into(),
                kind: GraphNodeKind::GameQa,
                execution_binding_id: None,
                inputs: json!({}),
            },
        ],
        edges: ["scene_evidence", "anime_evidence", "game_evidence"]
            .into_iter()
            .map(|target| GraphEdge {
                from_node: "brief".into(),
                from_port: "output".into(),
                to_node: target.into(),
                to_port: "input".into(),
            })
            .collect(),
    };

    let executed = call(
        &config,
        "creative_graph",
        json!({"action":"execute","graph":graph}),
    );
    assert_eq!(executed["job"]["status"], "completed");
    let runs = executed["job"]["node_runs"].as_array().unwrap();
    assert_eq!(
        runs.iter()
            .filter(|run| run["execution_batch"] == 0)
            .count(),
        1
    );
    assert_eq!(
        runs.iter()
            .filter(|run| run["execution_batch"] == 1)
            .count(),
        3
    );
    for node_id in ["scene_evidence", "anime_evidence", "game_evidence"] {
        assert!(runs.iter().any(|run| run["node_id"] == node_id));
    }
}
