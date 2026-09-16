use super::{call, create_project, dispatch_sync, TempWorkspace};
use ai_tools::application::creative;
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::ToolCallResult;
use image::{ImageBuffer, Rgba};
use serde_json::{json, Value};
use std::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn curated_workflows_have_specific_schemas_and_cover_core_utility_surface() {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.creative_binding_descriptors =
        vec![super::jobs::binding_descriptor_for_workflow_tests()];
    create_project(&config, "project_workflows");

    let catalog = call(&config, "creative_catalog", json!({"catalog":"workflows"}));
    let ids = catalog["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["workflow_id"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for required in [
        "character_turnaround",
        "expression_sheet",
        "pose_sheet",
        "storyboard_auto",
        "storyboard_manual",
        "hero_frame",
        "image_to_3d_bootstrap",
        "character_rig_bootstrap",
        "image_to_video_preview",
        "lipsync_preview",
        "image_upscale",
        "video_upscale",
        "image_remove_background",
        "video_remove_background",
        "image_outpaint",
        "video_reframe",
        "video_motion_control",
        "video_clip_extract",
        "voice_clone",
        "voice_change",
        "video_dub",
        "shot_render_preview",
        "game_asset_batch",
        "game_build_playtest",
    ] {
        assert!(ids.contains(required), "missing workflow {required}");
    }

    let invalid = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"cost_estimate",
            "project_id":"project_workflows",
            "workflow_id":"image_upscale",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"width":1024,"height":1024,"unexpected":true}
        }),
    );
    assert!(invalid.is_err());

    let valid = call(
        &config,
        "creative_job",
        json!({
            "action":"cost_estimate",
            "project_id":"project_workflows",
            "workflow_id":"image_upscale",
            "execution_binding_id":"binding_mock_image",
            "parameters":{"asset_id":"asset_fixture","width":1024,"height":1024}
        }),
    );
    assert!(valid["estimate"]["compute_units"].as_u64().is_some());
}

async fn call_async(config: &ServerConfig, tool: &str, arguments: Value) -> Value {
    let result = creative::dispatch_tool(tool, &arguments, config, "owner_character_pipeline")
        .await
        .expect("character pipeline dispatch")
        .expect("character pipeline result");
    result_json(result)
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error, "tool error: {:?}", result.content);
    serde_json::from_str(&result.content[0].text).expect("character pipeline JSON")
}

async fn prepare_project(
    workspace: &TempWorkspace,
    config: &ServerConfig,
) -> (String, String, Vec<String>) {
    call_async(
        config,
        "creative_project",
        json!({
            "action":"create",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "title":"Character pipeline",
            "intent":"deterministic Blender character regression fixture",
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
            Rgba([70 + index as u8, 90, 130, 255]),
        )
        .save(path)
        .unwrap();
        let registered = call_async(
            config,
            "creative_asset",
            json!({
                "action":"register",
                "cwd":workspace.0.to_string_lossy(),
                "project_id":"project_character_pipeline",
                "path":relative,
                "media_type":"image/png",
                "role":format!("character_{role}_reference")
            }),
        )
        .await;
        asset_ids.push(registered["asset_id"].as_str().unwrap().to_owned());
    }

    let character = call_async(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "kind":"character",
            "name":"Pipeline Hero",
            "authority":"authoritative",
            "reference_asset_ids":asset_ids,
            "spec":{"identity_notes":"accepted regression fixture"}
        }),
    )
    .await;
    let character_id = character["element_id"].as_str().unwrap().to_owned();
    let character_revision = character["revision_id"].as_str().unwrap().to_owned();
    call_async(
        config,
        "creative_element",
        json!({
            "action":"promote",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "element_id":character_id,
            "revision_id":character_revision
        }),
    )
    .await;

    let style = call_async(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "kind":"style",
            "name":"Pipeline Style",
            "authority":"authoritative",
            "reference_asset_ids":[],
            "spec":{"style_notes":"deterministic fixture"}
        }),
    )
    .await;
    let style_id = style["element_id"].as_str().unwrap().to_owned();
    let style_revision = style["revision_id"].as_str().unwrap().to_owned();
    call_async(
        config,
        "creative_element",
        json!({
            "action":"promote",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "element_id":style_id,
            "revision_id":style_revision
        }),
    )
    .await;

    (character_id, style_id, asset_ids)
}

async fn run_job(
    workspace: &TempWorkspace,
    config: &ServerConfig,
    workflow_id: &str,
    parameters: Value,
) -> Value {
    let submitted = call_async(
        config,
        "creative_job",
        json!({
            "action":"submit",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "workflow_id":workflow_id,
            "parameters":parameters,
            "approved":true
        }),
    )
    .await;
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call_async(
        config,
        "creative_job",
        json!({
            "action":"wait",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":"project_character_pipeline",
            "job_id":job_id
        }),
    )
    .await
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

async fn start_bridge() -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let mut reference_index = 0usize;
        let mut generic_asset_import_index = 0usize;
        for _ in 0..24 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            let code = request["code"].as_str().unwrap();
            let result = if code.contains("# MASIHAWAM_ASSET_IMPORT_REFERENCE") {
                let name = ["RefFront", "RefSide", "RefBack"][reference_index % 3];
                reference_index += 1;
                json!({"imported":[name],"kind":"reference_image"})
            } else if code.contains("# MASIHAWAM_CHARACTER_BOOTSTRAP") {
                let scene_path = extract_assignment_string(code, "_scene_path = ").unwrap();
                fs::write(scene_path, b"BLENDER-bootstrap-fixture").unwrap();
                assert!(code.contains("obj['masihawam_character_bootstrap'] = True"));
                json!({
                    "route":"manual",
                    "bootstrap_objects":["MA_Blockout_Torso","MA_Blockout_Head"],
                    "silhouette_extent_evidence":{"front":[1.1,2.8],"side":[0.64,2.8],"back":[1.1,2.8]},
                    "visual_inspection":"not_inspected"
                })
            } else if code.contains("# MASIHAWAM_CHARACTER_MESH_PRODUCTION") {
                assert!(code.contains("masihawam_character_bootstrap"));
                assert!(code.contains("_tagged_meshes or _imported_meshes"));
                json!({
                    "meshes":["MA_Blockout_Torso","MA_Blockout_Head"],
                    "uv_ready":true,
                    "material":"MasihAwam_Toon",
                    "strategy":"manual_cleanup",
                    "vertex_counts":{"MA_Blockout_Torso":8,"MA_Blockout_Head":32},
                    "polygon_counts":{"MA_Blockout_Torso":6,"MA_Blockout_Head":24}
                })
            } else if code.contains("# MASIHAWAM_CHARACTER_RIG_PRODUCTION") {
                assert!(code.contains("bpy.ops.object.parent_set(type='ARMATURE_AUTO')"));
                assert!(code.contains("bounded_object_groups_fallback"));
                assert!(code.contains("'vertex_groups'"));
                json!({
                    "armature":"MasihAwam_Rig",
                    "bones":["root","spine","head","arm.L","arm.R","leg.L","leg.R"],
                    "meshes":["MA_Blockout_Torso","MA_Blockout_Head"],
                    "shape_keys":{},
                    "vertex_groups":{"MA_Blockout_Torso":["spine"],"MA_Blockout_Head":["head"]},
                    "weighting_strategy":"bounded_object_groups_fallback",
                    "deformation_inspection":"not_inspected"
                })
            } else if code.contains("# MASIHAWAM_CHARACTER_ACTION") {
                assert!(code.contains("hasattr(_action, 'fcurves')"));
                assert!(code.contains("getattr(_strip, 'channelbags', [])"));
                json!({
                    "armature":"MasihAwam_Rig",
                    "action":"FixtureAction",
                    "frame_start":1,
                    "frame_end":120,
                    "fcurves":6
                })
            } else if code.contains("# MASIHAWAM_ASSET_IMPORT_BLEND") {
                json!({"imported":["MA_Blockout_Torso","MA_Blockout_Head"],"kind":"blend"})
            } else if code.contains("# MASIHAWAM_ASSET_IMPORT") {
                generic_asset_import_index += 1;
                if generic_asset_import_index == 1 {
                    json!({"imported":["MA_Blockout_Torso","MA_Blockout_Head"],"kind":"asset"})
                } else {
                    json!({"imported":["MA_Blockout_Torso","MA_Blockout_Head","MasihAwam_Rig"],"kind":"asset"})
                }
            } else if code.contains("# MASIHAWAM_CHECKPOINT_CREATE") {
                let path = extract_assignment_string(code, "_path = ").unwrap();
                fs::write(path, b"BLENDER-checkpoint-fixture").unwrap();
                json!({"saved":true})
            } else if code.contains("# MASIHAWAM_ASSET_EXPORT") {
                let path = extract_assignment_string(code, "_path = ").unwrap();
                fs::write(path, b"fixture-export").unwrap();
                json!({"exported":true})
            } else if code.contains("# MASIHAWAM_INSPECT:") {
                json!({"fixture":true,"scope":"inspection"})
            } else {
                panic!("unexpected Blender character pipeline code: {code}");
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn character_pipeline_preserves_bootstrap_meshes_weights_and_blender_52_actions() {
    let workspace = TempWorkspace::new();
    let (port, bridge) = start_bridge().await;
    let mut config = workspace.config();
    config.blender_bridge_port = port;
    config.blender_bridge_timeout_ms = 1_000;
    let (element_id, style_element_id, refs) = prepare_project(&workspace, &config).await;

    let bootstrap = run_job(
        &workspace,
        &config,
        "image_to_3d_bootstrap",
        json!({
            "route":"manual",
            "element_id":element_id,
            "reference_asset_ids":refs,
            "alignment":{
                "front_asset_id":refs[0],
                "side_asset_id":refs[1],
                "back_asset_id":refs[2],
                "reference_scale":2.5
            }
        }),
    )
    .await;
    assert_eq!(bootstrap["job"]["status"], "completed");
    let scene_asset_id = bootstrap["job"]["node_runs"][0]["output"]["scene_asset_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mesh = run_job(
        &workspace,
        &config,
        "character_mesh_production",
        json!({
            "source_asset_id":scene_asset_id,
            "element_id":element_id,
            "style_element_id":style_element_id,
            "strategy":"manual_cleanup"
        }),
    )
    .await;
    assert_eq!(mesh["job"]["status"], "completed");
    let mesh_evidence = &mesh["job"]["node_runs"][0]["output"]["evidence"];
    assert_eq!(
        mesh_evidence["authored"]["meshes"],
        json!(["MA_Blockout_Torso", "MA_Blockout_Head"])
    );
    let mesh_asset_id = mesh_evidence["export"]["asset_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let rig = run_job(
        &workspace,
        &config,
        "character_rig_production",
        json!({"source_asset_id":mesh_asset_id,"element_id":element_id}),
    )
    .await;
    assert_eq!(rig["job"]["status"], "completed");
    let rig_evidence = &rig["job"]["node_runs"][0]["output"]["evidence"];
    assert_eq!(
        rig_evidence["authored"]["weighting_strategy"],
        "bounded_object_groups_fallback"
    );
    assert_eq!(
        rig_evidence["authored"]["vertex_groups"]["MA_Blockout_Head"],
        json!(["head"])
    );
    assert_eq!(
        rig_evidence["authored"]["vertex_groups"]["MA_Blockout_Torso"],
        json!(["spine"])
    );
    let rig_asset_id = rig_evidence["export"]["asset_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let action = run_job(
        &workspace,
        &config,
        "character_action",
        json!({
            "source_asset_id":rig_asset_id,
            "action_name":"FixtureAction",
            "start_frame":1,
            "end_frame":120
        }),
    )
    .await;
    assert_eq!(action["job"]["status"], "completed");
    let action_evidence = &action["job"]["node_runs"][0]["output"]["evidence"];
    assert_eq!(action_evidence["authored"]["fcurves"], 6);
    assert_eq!(action_evidence["authored"]["frame_end"], 120);
    assert!(action_evidence["export"]["asset_id"].is_string());

    bridge.await.unwrap();
}
