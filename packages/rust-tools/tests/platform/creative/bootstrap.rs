#![cfg(all(unix, feature = "test-creative-binding"))]

use super::TempWorkspace;
use ai_tools::application::creative;
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::ToolCallResult;
use image::{ImageBuffer, Rgba};
use serde_json::{json, Value};
use std::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn call(config: &ServerConfig, tool: &str, arguments: Value) -> Value {
    let result = creative::dispatch_tool(tool, &arguments, config, "owner_bootstrap")
        .await
        .expect("creative bootstrap dispatch")
        .expect("creative bootstrap result");
    result_json(result)
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error, "tool error: {:?}", result.content);
    serde_json::from_str(&result.content[0].text).expect("creative bootstrap JSON")
}

fn mesh_binding_descriptor() -> String {
    json!({
        "binding_id":"test_mesh_bootstrap",
        "binding_version":"mesh-bootstrap-v1",
        "capabilities":["3d.image_to_mesh"],
        "media_roles":["reference_image","mesh"],
        "extension_schema":{"type":"object","additionalProperties":false},
        "constraints":{
            "estimate":{
                "base_compute_units":25,
                "base_output_bytes":4096,
                "cost_micros_per_compute_unit":0
            }
        },
        "license_notes":"test-only deterministic mesh fixture",
        "estimate_available":true,
        "availability":"available"
    })
    .to_string()
}

async fn prepare_project(
    workspace: &TempWorkspace,
    config: &ServerConfig,
) -> (String, Vec<String>) {
    call(
        config,
        "creative_project",
        json!({
            "action":"create",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap",
            "title":"Character bootstrap",
            "intent":"reproducible 2D to Blender fixture",
            "tracks":["anime"]
        }),
    )
    .await;

    let mut asset_ids = Vec::new();
    for (index, role) in ["front", "side", "back"].into_iter().enumerate() {
        let relative = format!("refs/{role}.png");
        let path = workspace.0.join(&relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        ImageBuffer::<Rgba<u8>, Vec<u8>>::from_pixel(
            16,
            16,
            Rgba([40 + index as u8, 80, 120, 255]),
        )
        .save(&path)
        .unwrap();
        let registered = call(
            config,
            "creative_asset",
            json!({
                "action":"register",
                "cwd":workspace.0.to_string_lossy(),
                "project_id":"project_bootstrap",
                "path":relative,
                "media_type":"image/png",
                "role":format!("character_{role}_reference")
            }),
        )
        .await;
        asset_ids.push(registered["asset_id"].as_str().unwrap().to_owned());
    }

    let created = call(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap",
            "kind":"character",
            "name":"Bootstrap Hero",
            "authority":"authoritative",
            "reference_asset_ids":asset_ids,
            "spec":{"identity_notes":"accepted turnaround source"}
        }),
    )
    .await;
    let element_id = created["element_id"].as_str().unwrap().to_owned();
    let revision_id = created["revision_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_element",
        json!({
            "action":"promote",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap",
            "element_id":element_id,
            "revision_id":revision_id
        }),
    )
    .await;
    (element_id, asset_ids)
}

async fn run_bootstrap(
    workspace: &TempWorkspace,
    config: &ServerConfig,
    route: &str,
    binding_id: Option<&str>,
    element_id: &str,
    refs: &[String],
) -> Value {
    let mut submit = json!({
        "action":"submit",
        "cwd":workspace.0.to_string_lossy(),
        "project_id":"project_bootstrap",
        "workflow_id":"image_to_3d_bootstrap",
        "parameters":{
            "route":route,
            "element_id":element_id,
            "reference_asset_ids":refs,
            "alignment":{
                "front_asset_id":refs[0],
                "side_asset_id":refs[1],
                "back_asset_id":refs[2],
                "reference_scale":2.5
            }
        },
        "approved":true
    });
    if let Some(binding_id) = binding_id {
        submit["execution_binding_id"] = json!(binding_id);
    }
    let submitted = call(config, "creative_job", submit).await;
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({
            "action":"wait",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap",
            "job_id":job_id
        }),
    )
    .await
}

async fn start_bridge(connections: usize) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let mut reference_index = 0usize;
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            let code = request["code"].as_str().unwrap();
            let result = if code.contains("# MASIHAWAM_ASSET_IMPORT_REFERENCE") {
                let name = ["RefFront", "RefSide", "RefBack"][reference_index % 3];
                reference_index += 1;
                json!({"imported":[name],"kind":"reference_image"})
            } else if code.contains("# MASIHAWAM_ASSET_IMPORT") {
                json!({"imported":["GeneratedBootstrapMesh"],"kind":"asset"})
            } else if code.contains("# MASIHAWAM_CHARACTER_BOOTSTRAP") {
                let scene_path = extract_assignment_string(code, "_scene_path = ").unwrap();
                fs::write(&scene_path, b"BLENDER-bootstrap-fixture").unwrap();
                if code.contains("_route = \"manual\"") {
                    json!({
                        "route":"manual",
                        "bootstrap_objects":["MA_Blockout_Torso","MA_Blockout_Head"],
                        "silhouette_extent_evidence":{"front":[1.1,2.8],"side":[0.64,2.8],"back":[1.1,2.8]},
                        "visual_inspection":"not_inspected"
                    })
                } else {
                    json!({
                        "route":"binding",
                        "bootstrap_objects":["GeneratedBootstrapMesh"],
                        "silhouette_extent_evidence":{"front":[1.0,1.0],"side":[1.0,1.0],"back":[1.0,1.0]},
                        "visual_inspection":"not_inspected"
                    })
                }
            } else if code.contains("# MASIHAWAM_INSPECT:character") {
                json!({
                    "scope":"character",
                    "root":"GeneratedBootstrapMesh",
                    "armature":null,
                    "meshes":["GeneratedBootstrapMesh"],
                    "shape_keys":{},
                    "materials":{}
                })
            } else {
                panic!("unexpected Blender bootstrap code: {code}");
            };
            let mut response = serde_json::to_vec(&json!({
                "status":"ok",
                "result":result,
                "stdout":""
            }))
            .unwrap();
            response.push(0);
            stream.write_all(&response).await.unwrap();
        }
    });
    (port, handle)
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> Value {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0);
        if let Some(position) = chunk[..read].iter().position(|byte| *byte == 0) {
            bytes.extend_from_slice(&chunk[..position]);
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    serde_json::from_slice(&bytes).unwrap()
}

fn extract_assignment_string(code: &str, prefix: &str) -> Option<String> {
    code.lines().find_map(|line| {
        line.strip_prefix(prefix)
            .and_then(|encoded| serde_json::from_str::<String>(encoded).ok())
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_character_references_reproduce_manual_and_binding_blender_bootstraps() {
    let workspace = TempWorkspace::new();
    let (port, bridge) = start_bridge(11).await;
    let mut config = workspace.config();
    config.enable_blender = true;
    config.blender_bridge_port = port;
    config.blender_bridge_timeout_ms = 1_000;
    config.creative_binding_descriptors = vec![mesh_binding_descriptor()];
    let (element_id, refs) = prepare_project(&workspace, &config).await;

    let manual = run_bootstrap(&workspace, &config, "manual", None, &element_id, &refs).await;
    assert_eq!(manual["job"]["status"], "completed");
    assert_eq!(
        manual["job"]["output_asset_ids"].as_array().unwrap().len(),
        1
    );
    let manual_evidence = &manual["job"]["node_runs"][0]["output"];
    assert_eq!(manual_evidence["route"], "manual");
    assert_eq!(
        manual_evidence["reference_alignment"]["front_asset_id"],
        refs[0]
    );
    assert_eq!(manual_evidence["visual_inspection"], "not_inspected");
    assert!(manual_evidence["silhouette_extent_evidence"]["front"].is_array());

    let binding = run_bootstrap(
        &workspace,
        &config,
        "binding",
        Some("test_mesh_bootstrap"),
        &element_id,
        &refs,
    )
    .await;
    assert_eq!(binding["job"]["status"], "completed");
    assert_eq!(
        binding["job"]["execution_binding_id"],
        "test_mesh_bootstrap"
    );
    assert_eq!(
        binding["job"]["output_asset_ids"].as_array().unwrap().len(),
        2
    );
    let binding_evidence = &binding["job"]["node_runs"][0]["output"];
    assert_eq!(binding_evidence["route"], "binding");
    assert!(binding_evidence["generated_mesh_asset_id"].is_string());
    assert!(binding_evidence["scene_asset_id"].is_string());

    let scene_asset_id = binding_evidence["scene_asset_id"].as_str().unwrap();
    let scene_asset = call(
        &config,
        "creative_asset",
        json!({
            "action":"get",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap",
            "asset_id":scene_asset_id
        }),
    )
    .await;
    assert_eq!(scene_asset["asset"]["source_surface"], "blender");
    assert_eq!(scene_asset["asset"]["element_id"], element_id);
    assert_eq!(
        scene_asset["asset"]["metadata"]["artifact_kind"],
        "blender_character_bootstrap_scene"
    );

    let project = call(
        &config,
        "creative_project",
        json!({
            "action":"get",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_bootstrap"
        }),
    )
    .await;
    let character = project["project"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["element_id"] == element_id)
        .unwrap();
    let selected_id = character["selected_revision_id"].as_str().unwrap();
    let selected = character["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| revision["revision_id"] == selected_id)
        .unwrap();
    assert_eq!(selected["state"], "accepted");
    assert_eq!(selected["reference_asset_ids"], json!(refs));

    bridge.await.unwrap();
}

#[tokio::test]
async fn binding_and_hybrid_routes_require_an_explicit_compatible_binding() {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.enable_blender = true;
    config.creative_binding_descriptors = vec![mesh_binding_descriptor()];
    let (element_id, refs) = prepare_project(&workspace, &config).await;
    for route in ["binding", "hybrid"] {
        let error = creative::dispatch_tool(
            "creative_job",
            &json!({
                "action":"cost_estimate",
                "cwd":workspace.0.to_string_lossy(),
                "project_id":"project_bootstrap",
                "workflow_id":"image_to_3d_bootstrap",
                "parameters":{
                    "route":route,
                    "element_id":element_id,
                    "reference_asset_ids":refs,
                    "alignment":{
                        "front_asset_id":refs[0],
                        "side_asset_id":refs[1],
                        "back_asset_id":refs[2]
                    }
                }
            }),
            &config,
            "owner_bootstrap",
        )
        .await
        .expect_err("binding-backed bootstrap must not auto-select a binding");
        assert!(format!("{error:?}").contains("execution_binding_required"));
    }
}
