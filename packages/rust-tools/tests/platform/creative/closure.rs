#![cfg(feature = "test-creative-binding")]

use super::{call, TempWorkspace};
use ai_tools::application::creative;
use ai_tools::core::config::ServerConfig;
use ai_tools::core::error::McpError;
use ai_tools::interfaces::mcp::ToolCallResult;
use serde_json::{json, Value};

fn closure_binding_descriptor() -> String {
    json!({
        "binding_id":"test_closure_media",
        "binding_version":"closure-v1",
        "capabilities":[
            "video.generate",
            "video.sequence_assemble",
            "game.deploy"
        ],
        "media_roles":["video","video_clip","deployment"],
        "extension_schema":{"type":"object","additionalProperties":true},
        "constraints":{
            "estimate":{
                "base_compute_units":5,
                "base_output_bytes":1024,
                "cost_micros_per_compute_unit":0
            }
        },
        "license_notes":"test-only deterministic Plan 069 closure binding",
        "estimate_available":true,
        "availability":"available"
    })
    .to_string()
}

fn config(workspace: &TempWorkspace) -> ServerConfig {
    let mut config = workspace.config();
    config.creative_binding_descriptors = vec![closure_binding_descriptor()];
    config.creative_approval_compute_units = 10_000;
    config.creative_job_hard_compute_units = 10_000;
    config.creative_project_hard_compute_units = 1_000_000;
    config.creative_max_job_output_bytes = 16 * 1024 * 1024;
    config.creative_max_concurrent_jobs = 16;
    config
}

fn create_project(
    config: &ServerConfig,
    workspace: &TempWorkspace,
    project_id: &str,
    tracks: &[&str],
) {
    call(
        config,
        "creative_project",
        json!({
            "action":"create",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "title":format!("Closure fixture {project_id}"),
            "intent":"Fresh Plan 069 closure acceptance fixture",
            "tracks":tracks
        }),
    );
}

fn accepted_element(
    config: &ServerConfig,
    workspace: &TempWorkspace,
    project_id: &str,
    kind: &str,
    name: &str,
    spec: Value,
) -> String {
    let created = call(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "kind":kind,
            "name":name,
            "authority":"authoritative",
            "reference_asset_ids":[],
            "spec":spec
        }),
    );
    let element_id = created["element_id"].as_str().unwrap().to_owned();
    let revision_id = created["revision_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_element",
        json!({
            "action":"promote",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "element_id":element_id,
            "revision_id":revision_id
        }),
    );
    element_id
}

fn submit_wait(
    config: &ServerConfig,
    workspace: &TempWorkspace,
    project_id: &str,
    target: (&str, &str),
    parameters: Value,
    binding_id: Option<&str>,
) -> Value {
    let mut request = json!({
        "action":"submit",
        "cwd":workspace.0.to_string_lossy(),
        "project_id":project_id,
        "parameters":parameters,
        "approved":true
    });
    request[target.0] = json!(target.1);
    if let Some(binding_id) = binding_id {
        request["execution_binding_id"] = json!(binding_id);
    }
    let submitted = call(config, "creative_job", request);
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({
            "action":"wait",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "job_id":job_id
        }),
    )["job"]
        .clone()
}

fn promote_asset(
    config: &ServerConfig,
    workspace: &TempWorkspace,
    project_id: &str,
    asset_id: &str,
) {
    call(
        config,
        "creative_asset",
        json!({
            "action":"promote",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "asset_id":asset_id
        }),
    );
}

fn dispatch_as_owner(
    config: &ServerConfig,
    owner: &str,
    name: &str,
    arguments: &Value,
) -> Result<Option<ToolCallResult>, McpError> {
    tokio::runtime::Runtime::new()
        .expect("closure acceptance runtime")
        .block_on(creative::dispatch_tool(name, arguments, config, owner))
}

fn call_as_owner(config: &ServerConfig, owner: &str, name: &str, arguments: Value) -> Value {
    let result = dispatch_as_owner(config, owner, name, &arguments)
        .expect("owner-scoped creative dispatch")
        .expect("owner-scoped creative result");
    assert!(
        !result.is_error,
        "owner-scoped tool error: {:?}",
        result.content
    );
    serde_json::from_str(&result.content[0].text).expect("owner-scoped creative JSON")
}

fn generated_video(
    config: &ServerConfig,
    workspace: &TempWorkspace,
    project_id: &str,
    duration_ms: u64,
    prompt: &str,
) -> String {
    let job = submit_wait(
        config,
        workspace,
        project_id,
        ("capability_id", "video.generate"),
        json!({"duration_ms":duration_ms,"prompt":prompt}),
        Some("test_closure_media"),
    );
    assert_eq!(job["status"], "completed");
    let asset_id = job["output_asset_ids"][0].as_str().unwrap().to_owned();
    promote_asset(config, workspace, project_id, &asset_id);
    asset_id
}

#[path = "closure/anime.rs"]
mod anime;
#[path = "closure/game.rs"]
mod game;
#[path = "closure/scene.rs"]
mod scene;
#[path = "closure/second_project.rs"]
mod second_project;
