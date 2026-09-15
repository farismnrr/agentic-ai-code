use super::{call, create_project, dispatch_sync, TempWorkspace};
use serde_json::{json, Value};

fn descriptor(binding_id: &str, version: &str, extension_schema: Value) -> String {
    json!({
        "binding_id": binding_id,
        "binding_version": version,
        "capabilities": ["image.generate"],
        "media_roles": ["image", "reference_image"],
        "extension_schema": extension_schema,
        "constraints": {
            "estimate": {
                "base_compute_units": 10,
                "base_output_bytes": 1024,
                "compute_units_per_megapixel": 1,
                "output_bytes_per_megapixel": 1024
            }
        },
        "license_notes": "compiler conformance fixture",
        "estimate_available": true,
        "availability": "available"
    })
    .to_string()
}

fn configured_workspace() -> (TempWorkspace, ai_tools::core::config::ServerConfig) {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.creative_binding_descriptors = vec![
        descriptor(
            "binding_compile_a",
            "alpha-v1",
            json!({
                "type":"object",
                "properties":{"strength":{"type":"number","minimum":0,"maximum":1}},
                "required":["strength"],
                "additionalProperties":false
            }),
        ),
        descriptor(
            "binding_compile_b",
            "beta-v2",
            json!({
                "type":"object",
                "properties":{"quality":{"type":"string","enum":["draft","final"]}},
                "required":["quality"],
                "additionalProperties":false
            }),
        ),
    ];
    (workspace, config)
}

fn semantic_spec() -> Value {
    json!({
        "schema_version": 1,
        "prompt": "caller-authored character reference prompt",
        "references": [
            {"asset_id":"asset_back","role":"back_reference","order":2},
            {"asset_id":"asset_front","role":"front_reference","order":1}
        ],
        "width": 640,
        "height": 768,
        "batch_count": 1,
        "binding_extensions": {
            "binding_compile_a": {"strength": 0.75},
            "binding_compile_b": {"quality": "final"}
        }
    })
}

#[test]
fn semantic_compiler_is_deterministic_binding_selected_and_provider_neutral() {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_compiler");

    let first = call(
        &config,
        "creative_job",
        json!({
            "action":"compile_spec",
            "project_id":"project_compiler",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec": semantic_spec(),
            "changed_fields":["width","prompt"]
        }),
    );
    let repeated = call(
        &config,
        "creative_job",
        json!({
            "action":"compile_spec",
            "project_id":"project_compiler",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec": semantic_spec(),
            "changed_fields":["width","prompt"]
        }),
    );
    assert_eq!(first, repeated);
    assert_eq!(
        first["compiled"]["compiler_version"],
        "creative.semantic.v1"
    );
    assert_eq!(
        first["compiled"]["execution_binding_id"],
        "binding_compile_a"
    );
    assert_eq!(first["compiled"]["execution_binding_version"], "alpha-v1");
    assert_eq!(
        first["compiled"]["parameters"]["reference_asset_ids"],
        json!(["asset_front", "asset_back"])
    );
    assert_eq!(
        first["compiled"]["parameters"]["binding_extensions"],
        json!({"strength":0.75})
    );
    assert_eq!(
        first["compiled"]["changed_fields"],
        json!(["prompt", "width"])
    );

    let second_binding = call(
        &config,
        "creative_job",
        json!({
            "action":"compile_spec",
            "project_id":"project_compiler",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_b",
            "semantic_spec": semantic_spec()
        }),
    );
    assert_eq!(
        second_binding["compiled"]["execution_binding_version"],
        "beta-v2"
    );
    assert_eq!(
        second_binding["compiled"]["parameters"]["binding_extensions"],
        json!({"quality":"final"})
    );
    let serialized = serde_json::to_string(&semantic_spec()).unwrap();
    assert!(!serialized.contains("provider"));
    assert!(!serialized.contains("model"));
}

#[test]
fn semantic_submit_persists_compiler_and_binding_lineage_cross_turn() {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_compiler_job");

    let submitted = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_compiler_job",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec": semantic_spec(),
            "changed_fields":["width","prompt"],
            "approved":true
        }),
    );
    let job = &submitted["job"];
    assert_eq!(job["status"], "queued");
    assert_eq!(job["compiler_version"], "creative.semantic.v1");
    assert_eq!(job["execution_binding_version"], "alpha-v1");
    assert_eq!(job["changed_fields"], json!(["prompt", "width"]));
    assert_eq!(
        job["execution_parameters"]["reference_asset_ids"],
        json!(["asset_front", "asset_back"])
    );

    let job_id = job["job_id"].as_str().unwrap();
    let reloaded = call(
        &config,
        "creative_job",
        json!({
            "action":"get",
            "project_id":"project_compiler_job",
            "job_id":job_id
        }),
    );
    assert_eq!(reloaded["job"]["compiler_version"], "creative.semantic.v1");
    assert_eq!(reloaded["job"]["execution_binding_version"], "alpha-v1");
    assert_eq!(
        reloaded["job"]["changed_fields"],
        json!(["prompt", "width"])
    );
}

#[test]
fn semantic_compiler_rejects_unknown_fields_raw_parameter_mix_and_bad_extensions() {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_compiler_reject");

    let unknown = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"compile_spec",
            "project_id":"project_compiler_reject",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec": {"schema_version":1,"prompt":"x","provider":"forbidden"}
        }),
    );
    assert!(unknown.is_err());

    let mixed = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"submit",
            "project_id":"project_compiler_reject",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec": semantic_spec(),
            "parameters":{"prompt":"raw"}
        }),
    );
    assert!(mixed.is_err());

    let mut bad = semantic_spec();
    bad["binding_extensions"]["binding_compile_a"] = json!({"strength":2.0});
    let bad_extension = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"compile_spec",
            "project_id":"project_compiler_reject",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_compile_a",
            "semantic_spec":bad
        }),
    );
    assert!(bad_extension.is_err());
}
