use super::jobs::{configured_workspace, test_binding_descriptor};
use super::{call, create_project, dispatch_sync};
use serde_json::{json, Value};

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
