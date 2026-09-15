use super::{call, create_project, dispatch_sync, TempWorkspace};
use ai_tools::application::creative::{
    CreativeGraph, GraphNode, GraphNodeKind, CREATIVE_SCHEMA_VERSION,
};
use serde_json::{json, Value};

pub(super) fn binding_descriptor_for_workflow_tests() -> String {
    json!({
        "binding_id": "binding_mock_image",
        "binding_version": "1",
        "capabilities": ["image.generate", "image.upscale"],
        "media_roles": ["image", "reference_image"],
        "extension_schema": {
            "type": "object",
            "maxProperties": 8,
            "additionalProperties": false
        },
        "constraints": {
            "max_width": 4096,
            "max_height": 4096,
            "estimate": {
                "base_compute_units": 120,
                "base_output_bytes": 1024,
                "compute_units_per_megapixel": 10,
                "output_bytes_per_megapixel": 2048,
                "compute_units_per_item": 5,
                "output_bytes_per_item": 512,
                "cost_micros_per_compute_unit": 3
            }
        },
        "license_notes": "deterministic conformance fixture",
        "estimate_available": true,
        "availability": "available"
    })
    .to_string()
}

fn test_binding_descriptor() -> String {
    json!({
        "binding_id": "test_mock_image",
        "binding_version": "1",
        "capabilities": ["image.generate", "image.upscale"],
        "media_roles": ["image", "reference_image"],
        "extension_schema": {
            "type": "object",
            "maxProperties": 16,
            "additionalProperties": true
        },
        "constraints": {
            "estimate": {
                "base_compute_units": 10,
                "base_output_bytes": 1024,
                "compute_units_per_megapixel": 1,
                "output_bytes_per_megapixel": 128,
                "compute_units_per_item": 1,
                "output_bytes_per_item": 64,
                "cost_micros_per_compute_unit": 1
            }
        },
        "license_notes": "test-only deterministic lifecycle binding",
        "estimate_available": true,
        "availability": "available"
    })
    .to_string()
}

fn configured_workspace() -> (TempWorkspace, ai_tools::core::config::ServerConfig) {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.creative_binding_descriptors = vec![binding_descriptor_for_workflow_tests()];
    config.creative_approval_compute_units = 100;
    config.creative_job_hard_compute_units = 1_000;
    config.creative_project_hard_compute_units = 260;
    config.creative_max_job_output_bytes = 64 * 1024;
    config.creative_max_concurrent_jobs = 2;
    config.creative_max_retries = 1;
    (workspace, config)
}

#[test]
fn configured_binding_discovery_and_cost_preflight_are_explicit_and_bounded() {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_job_estimate");

    let catalog = call(
        &config,
        "creative_catalog",
        json!({"catalog":"execution_bindings"}),
    );
    assert_eq!(catalog["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(catalog["items"][0]["binding_id"], "binding_mock_image");
    assert_eq!(catalog["items"][0]["availability"], "available");

    let estimate = call(
        &config,
        "creative_job",
        json!({
            "action":"cost_estimate",
            "project_id":"project_job_estimate",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1000,"height":1000,"batch_count":2}
        }),
    );
    assert_eq!(estimate["estimate"]["compute_units"], 140);
    assert_eq!(estimate["estimate"]["output_bytes"], 4096);
    assert_eq!(estimate["estimate"]["estimated_cost_micros"], 420);
    assert_eq!(estimate["estimate"]["source"], "binding:binding_mock_image");

    let missing_binding = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"cost_estimate",
            "project_id":"project_job_estimate",
            "capability_id":"image.generate",
            "parameters":{}
        }),
    );
    assert!(missing_binding.is_err());
}

#[test]
fn job_submit_enforces_approval_hard_budgets_owner_and_retry_lifecycle() {
    let (_workspace, mut config) = configured_workspace();
    create_project(&config, "project_job_lifecycle");

    let soft_denied = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"submit",
            "project_id":"project_job_lifecycle",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1000,"height":1000}
        }),
    )
    .expect("soft approval response")
    .expect("creative job result");
    assert!(soft_denied.is_error);
    let soft_json: Value = serde_json::from_str(&soft_denied.content[0].text).unwrap();
    assert_eq!(soft_json["code"], "creative_job_approval_required");

    let submitted = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_job_lifecycle",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1000,"height":1000},
            "approved":true,
            "max_retries":1
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    assert_eq!(submitted["job"]["status"], "queued");
    assert_eq!(submitted["job"]["owner"], "local");

    let budget = call(
        &config,
        "creative_job",
        json!({"action":"budget_status","project_id":"project_job_lifecycle"}),
    );
    assert_eq!(budget["budget"]["project_admitted_compute_units"], 135);
    assert_eq!(budget["budget"]["project_remaining_compute_units"], 125);

    let owner_isolation = tokio::runtime::Runtime::new().unwrap().block_on(
        ai_tools::application::creative::dispatch_tool(
            "creative_job",
            &json!({
                "action":"get",
                "project_id":"project_job_lifecycle",
                "job_id":job_id
            }),
            &config,
            "different-owner",
        ),
    );
    assert!(owner_isolation.is_err());

    let failed = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_job_lifecycle",
            "job_id":job_id
        }),
    );
    assert_eq!(failed["job"]["status"], "failed");
    assert_eq!(failed["job"]["failure_code"], "execution_not_implemented");

    let retried = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_job_lifecycle",
            "job_id":job_id,
            "retry":true
        }),
    );
    assert_eq!(retried["job"]["status"], "failed");
    assert_eq!(retried["job"]["retry_count"], 1);
    assert!(dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"wait",
            "project_id":"project_job_lifecycle",
            "job_id":job_id,
            "retry":true
        }),
    )
    .is_err());

    let second_denied = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"submit",
            "project_id":"project_job_lifecycle",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1000,"height":1000},
            "approved":true
        }),
    )
    .expect("hard budget response")
    .expect("creative job result");
    assert!(second_denied.is_error);
    let hard_json: Value = serde_json::from_str(&second_denied.content[0].text).unwrap();
    assert_eq!(hard_json["code"], "creative_project_hard_limit_exceeded");

    config.creative_project_hard_compute_units = 1_000;
    let cancellable = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_job_lifecycle",
            "capability_id":"image.upscale",
            "execution_binding_id":"binding_mock_image",
            "parameters":{},
            "approved":true
        }),
    );
    let cancellable_id = cancellable["job"]["job_id"].as_str().unwrap().to_owned();
    let cancelled = call(
        &config,
        "creative_job",
        json!({
            "action":"cancel",
            "project_id":"project_job_lifecycle",
            "job_id":cancellable_id
        }),
    );
    assert_eq!(cancelled["job"]["status"], "cancelled");

    let listed = call(
        &config,
        "creative_job",
        json!({"action":"list","project_id":"project_job_lifecycle"}),
    );
    assert!(listed["jobs"].as_array().unwrap().len() >= 2);
}

#[test]
fn queued_graph_job_can_be_retrieved_and_waited_to_completion() {
    let (_workspace, mut config) = configured_workspace();
    config.creative_approval_compute_units = 1_000;
    config.creative_project_hard_compute_units = 10_000;
    create_project(&config, "project_graph_job");

    let graph = CreativeGraph {
        schema_version: CREATIVE_SCHEMA_VERSION,
        graph_id: "graph_job_control".into(),
        project_id: "project_graph_job".into(),
        revision: 1,
        nodes: vec![GraphNode {
            node_id: "brief".into(),
            kind: GraphNodeKind::InputText,
            execution_binding_id: None,
            inputs: json!({"text":"persisted graph job"}),
        }],
        edges: vec![],
    };
    let first = call(
        &config,
        "creative_graph",
        json!({"action":"execute","graph":graph}),
    );
    assert_eq!(first["job"]["status"], "completed");

    let submitted = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_graph_job",
            "graph_id":"graph_job_control"
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    assert_eq!(submitted["job"]["status"], "queued");

    let reloaded = call(
        &config,
        "creative_job",
        json!({
            "action":"get",
            "project_id":"project_graph_job",
            "job_id":job_id
        }),
    );
    assert_eq!(reloaded["job"]["status"], "queued");

    let completed = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_graph_job",
            "job_id":job_id
        }),
    );
    assert_eq!(completed["job"]["status"], "completed");
    assert_eq!(
        completed["job"]["node_runs"].as_array().map(Vec::len),
        Some(1)
    );
}

#[test]
fn test_binding_proves_running_timeout_actual_output_and_terminal_lifecycle() {
    let (_workspace, mut config) = configured_workspace();
    config
        .creative_binding_descriptors
        .push(test_binding_descriptor());
    config.creative_approval_compute_units = 1_000;
    config.creative_project_hard_compute_units = 100_000;
    config.creative_max_job_output_bytes = 4_096;
    create_project(&config, "project_test_binding");

    let running = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_test_binding",
            "capability_id":"image.generate",
            "execution_binding_id":"test_mock_image",
            "parameters":{"test_behavior":"running"},
            "approved":true
        }),
    );
    let running_id = running["job"]["job_id"].as_str().unwrap().to_owned();
    let running_wait = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":running_id
        }),
    );
    assert_eq!(running_wait["job"]["status"], "running");
    let cancelled = call(
        &config,
        "creative_job",
        json!({
            "action":"cancel",
            "project_id":"project_test_binding",
            "job_id":running_id
        }),
    );
    assert_eq!(cancelled["job"]["status"], "cancelled");

    let timeout = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_test_binding",
            "capability_id":"image.generate",
            "execution_binding_id":"test_mock_image",
            "parameters":{"test_behavior":"complete","test_duration_ms":100},
            "timeout_ms":10,
            "approved":true
        }),
    );
    let timeout_id = timeout["job"]["job_id"].as_str().unwrap().to_owned();
    let timed_out = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":timeout_id
        }),
    );
    assert_eq!(timed_out["job"]["status"], "failed");
    assert_eq!(timed_out["job"]["failure_code"], "execution_timeout");

    let oversized = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_test_binding",
            "capability_id":"image.generate",
            "execution_binding_id":"test_mock_image",
            "parameters":{"test_behavior":"complete","test_actual_output_bytes":8192},
            "approved":true
        }),
    );
    let oversized_id = oversized["job"]["job_id"].as_str().unwrap().to_owned();
    let oversized_result = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":oversized_id
        }),
    );
    assert_eq!(oversized_result["job"]["status"], "failed");
    assert_eq!(
        oversized_result["job"]["failure_code"],
        "output_hard_limit_exceeded"
    );

    let completed = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_test_binding",
            "capability_id":"image.generate",
            "execution_binding_id":"test_mock_image",
            "parameters":{"test_behavior":"complete"},
            "approved":true
        }),
    );
    let completed_id = completed["job"]["job_id"].as_str().unwrap().to_owned();
    let completed_result = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":completed_id
        }),
    );
    assert_eq!(completed_result["job"]["status"], "completed");

    let failed = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_test_binding",
            "capability_id":"image.generate",
            "execution_binding_id":"test_mock_image",
            "parameters":{"test_behavior":"fail"},
            "approved":true,
            "max_retries":1
        }),
    );
    let failed_id = failed["job"]["job_id"].as_str().unwrap().to_owned();
    let failed_result = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":failed_id
        }),
    );
    assert_eq!(
        failed_result["job"]["failure_code"],
        "test_execution_failed"
    );
    let retried = call(
        &config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":"project_test_binding",
            "job_id":failed_id,
            "retry":true
        }),
    );
    assert_eq!(retried["job"]["retry_count"], 1);
    assert_eq!(retried["job"]["failure_code"], "test_execution_failed");
}

#[test]
fn output_hard_limit_blocks_submit_before_job_creation() {
    let (_workspace, mut config) = configured_workspace();
    config.creative_approval_compute_units = 1_000;
    config.creative_project_hard_compute_units = 10_000;
    config.creative_max_job_output_bytes = 2_000;
    create_project(&config, "project_output_budget");

    let denied = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"submit",
            "project_id":"project_output_budget",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1000,"height":1000},
            "approved":true
        }),
    )
    .expect("output budget response")
    .expect("creative job result");
    assert!(denied.is_error);
    let value: Value = serde_json::from_str(&denied.content[0].text).unwrap();
    assert_eq!(value["code"], "creative_job_output_limit_exceeded");

    let listed = call(
        &config,
        "creative_job",
        json!({"action":"list","project_id":"project_output_budget"}),
    );
    assert_eq!(listed["jobs"].as_array().map(Vec::len), Some(0));
}
