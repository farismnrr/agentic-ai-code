use super::{call, create_project, TempWorkspace};
use serde_json::{json, Value};
use std::path::Path;

fn media_binding_descriptor() -> String {
    json!({
        "binding_id": "binding_local_raster",
        "binding_version": "local-raster-v1",
        "capabilities": [
            "image.generate",
            "image.reference_generate",
            "image.edit",
            "image.inpaint",
            "image.upscale",
            "image.remove_background",
            "image.outpaint"
        ],
        "media_roles": ["image", "reference_image", "mask"],
        "extension_schema": {
            "type": "object",
            "maxProperties": 8,
            "additionalProperties": false
        },
        "constraints": {
            "max_width": 4096,
            "max_height": 4096,
            "estimate": {
                "base_compute_units": 10,
                "base_output_bytes": 4096,
                "compute_units_per_megapixel": 5,
                "output_bytes_per_megapixel": 65536,
                "compute_units_per_item": 1,
                "output_bytes_per_item": 1024,
                "cost_micros_per_compute_unit": 0
            }
        },
        "license_notes": "local deterministic raster conformance backend",
        "estimate_available": true,
        "availability": "available"
    })
    .to_string()
}

fn configured_workspace() -> (TempWorkspace, ai_tools::core::config::ServerConfig) {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.creative_binding_descriptors = vec![media_binding_descriptor()];
    config.creative_binding_backends = vec!["binding_local_raster=local_raster".into()];
    config.creative_approval_compute_units = 10_000;
    config.creative_job_hard_compute_units = 100_000;
    config.creative_project_hard_compute_units = 1_000_000;
    config.creative_max_job_output_bytes = 16 * 1024 * 1024;
    (workspace, config)
}

fn run_capability(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    capability_id: &str,
    parameters: Value,
) -> Value {
    let submitted = call(
        config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":project_id,
            "capability_id":capability_id,
            "execution_binding_id":"binding_local_raster",
            "parameters":parameters,
            "approved":true
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":project_id,
            "job_id":job_id
        }),
    )
}

#[test]
fn local_raster_binding_executes_all_image_capabilities_with_durable_lineage() {
    let (workspace, config) = configured_workspace();
    create_project(&config, "project_media");

    let generated = run_capability(
        &config,
        "project_media",
        "image.generate",
        json!({"prompt":"caller authored procedural conformance prompt","width":64,"height":48}),
    );
    assert_eq!(generated["job"]["status"], "completed");
    assert!(generated["job"]["actual_output_bytes"].as_u64().unwrap() > 0);
    let generated_id = generated["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();

    let generated_asset = call(
        &config,
        "creative_asset",
        json!({"action":"get","project_id":"project_media","asset_id":generated_id}),
    );
    let asset = &generated_asset["asset"];
    assert_eq!(asset["source"], "generated_asset");
    assert_eq!(asset["source_surface"], "mcp");
    assert_eq!(asset["state"], "candidate");
    assert_eq!(asset["media_type"], "image/png");
    assert_eq!(asset["metadata"]["width"], 64);
    assert_eq!(asset["metadata"]["height"], 48);
    let generated_path = asset["relative_path"].as_str().unwrap();
    assert!(Path::new(generated_path).starts_with("creative/project_media/assets/generated"));
    let decoded = image::open(workspace.path(generated_path)).expect("generated PNG decodes");
    assert_eq!((decoded.width(), decoded.height()), (64, 48));

    let referenced = run_capability(
        &config,
        "project_media",
        "image.reference_generate",
        json!({
            "prompt":"caller authored reference transform",
            "reference_asset_ids":[generated_id],
            "width":80,
            "height":60
        }),
    );
    let referenced_id = referenced["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let referenced_asset = call(
        &config,
        "creative_asset",
        json!({"action":"get","project_id":"project_media","asset_id":referenced_id}),
    );
    assert_eq!(referenced_asset["asset"]["parent_asset_id"], generated_id);
    assert_eq!(referenced_asset["asset"]["metadata"]["width"], 80);

    let edited = run_capability(
        &config,
        "project_media",
        "image.edit",
        json!({"asset_id":referenced_id}),
    );
    let edited_id = edited["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(edited["job"]["status"], "completed");

    let inpainted = run_capability(
        &config,
        "project_media",
        "image.inpaint",
        json!({"asset_id":edited_id,"mask_asset_id":generated_id,"prompt":"fill region"}),
    );
    assert_eq!(inpainted["job"]["status"], "completed");

    let upscaled = run_capability(
        &config,
        "project_media",
        "image.upscale",
        json!({"asset_id":edited_id,"width":160,"height":120}),
    );
    let upscaled_id = upscaled["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let upscaled_asset = call(
        &config,
        "creative_asset",
        json!({"action":"get","project_id":"project_media","asset_id":upscaled_id}),
    );
    assert_eq!(upscaled_asset["asset"]["metadata"]["width"], 160);
    assert_eq!(upscaled_asset["asset"]["metadata"]["height"], 120);

    let removed = run_capability(
        &config,
        "project_media",
        "image.remove_background",
        json!({"asset_id":upscaled_id}),
    );
    assert_eq!(removed["job"]["status"], "completed");

    let outpainted = run_capability(
        &config,
        "project_media",
        "image.outpaint",
        json!({"asset_id":upscaled_id,"width":192,"height":144,"prompt":"extend canvas"}),
    );
    assert_eq!(outpainted["job"]["status"], "completed");
    let outpainted_id = outpainted["job"]["output_asset_ids"][0].as_str().unwrap();
    let outpainted_asset = call(
        &config,
        "creative_asset",
        json!({"action":"get","project_id":"project_media","asset_id":outpainted_id}),
    );
    assert_eq!(outpainted_asset["asset"]["metadata"]["width"], 192);
    assert_eq!(outpainted_asset["asset"]["metadata"]["height"], 144);
}

#[test]
fn local_raster_backend_is_operator_mapped_and_fails_closed_without_mapping() {
    let (_workspace, mut config) = configured_workspace();
    create_project(&config, "project_media_mapping");
    config.creative_binding_backends.clear();

    let submitted = call(
        &config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":"project_media_mapping",
            "capability_id":"image.generate",
            "execution_binding_id":"binding_local_raster",
            "parameters":{"prompt":"no hidden backend default","width":32,"height":32},
            "approved":true
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap();
    let waited = call(
        &config,
        "creative_job",
        json!({"action":"wait","project_id":"project_media_mapping","job_id":job_id}),
    );
    assert_eq!(waited["job"]["status"], "failed");
    assert_eq!(waited["job"]["failure_code"], "execution_not_implemented");
}
