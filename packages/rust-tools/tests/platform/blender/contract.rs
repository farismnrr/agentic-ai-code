use ai_tools::application::blender::{
    artifact_relative_path, bridge_address, project_layout, validate_blender_relative_path,
    BlenderArtifactScope, BlenderSessionOwnership, BlenderSessionState, BlenderSessionStatus,
    BLENDER_LAB_PROTOCOL, DEFAULT_BLENDER_LAB_PORT, MAX_BLENDER_PYTHON_BYTES,
    MAX_BLENDER_REQUEST_BYTES, MAX_BLENDER_RESPONSE_BYTES, REVIEWED_BLENDER_PATH_NAMES,
};
use ai_tools::application::hooks::effect_classes_for_call;
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::{blender_tool_catalog, validate_tool_arguments};
use serde_json::json;
use std::net::Ipv4Addr;

#[test]
fn blender_operator_config_is_disabled_loopback_only_and_bounded() {
    let default = ServerConfig::default();
    assert!(!default.enable_blender);
    assert_eq!(default.blender_bridge_port, DEFAULT_BLENDER_LAB_PORT);
    assert_eq!(bridge_address(&default).ip(), &Ipv4Addr::LOCALHOST);
    assert_eq!(BLENDER_LAB_PROTOCOL, "blender_lab_json_nul_v1");
    assert_eq!(MAX_BLENDER_REQUEST_BYTES, 1024 * 1024);
    assert_eq!(MAX_BLENDER_RESPONSE_BYTES, 8 * 1024 * 1024);
    assert_eq!(MAX_BLENDER_PYTHON_BYTES, 256 * 1024);
    assert_eq!(REVIEWED_BLENDER_PATH_NAMES, &["blender", "blender.exe"]);
    let external = BlenderSessionStatus {
        state: BlenderSessionState::Ready,
        ownership: Some(BlenderSessionOwnership::External),
        protocol: BLENDER_LAB_PROTOCOL.into(),
        loopback_port: DEFAULT_BLENDER_LAB_PORT,
        project_root: None,
    };
    assert_eq!(
        serde_json::to_value(external).unwrap()["ownership"],
        "external"
    );

    let enabled = ServerConfig {
        enable_creative: true,
        enable_blender: true,
        blender_executable: Some("/opt/blender/blender".into()),
        ..ServerConfig::default()
    };
    enabled.validate().expect("reviewed Blender config");

    let mut without_creative = enabled.clone();
    without_creative.enable_creative = false;
    assert!(without_creative.validate().is_err());

    let mut privileged_port = enabled.clone();
    privileged_port.blender_bridge_port = 80;
    assert!(privileged_port.validate().is_err());

    let mut unbounded_timeout = enabled.clone();
    unbounded_timeout.blender_bridge_timeout_ms = 120_001;
    assert!(unbounded_timeout.validate().is_err());

    let mut bad_executable = enabled;
    bad_executable.blender_executable = Some("blender\n--evil".into());
    assert!(bad_executable.validate().is_err());
}

#[test]
fn blender_project_layout_rejects_escape_and_noncanonical_destinations() {
    let layout = project_layout();
    assert_eq!(layout.root, "blender");
    assert_eq!(layout.scenes, "blender/scenes");
    assert_eq!(layout.assets, "blender/assets");
    assert_eq!(layout.references, "blender/references");
    assert_eq!(layout.renders_preview, "blender/renders/preview");
    assert_eq!(layout.renders_final, "blender/renders/final");
    assert_eq!(layout.animations, "blender/animations");
    assert_eq!(layout.exports, "blender/exports");
    assert_eq!(layout.checkpoints, "blender/checkpoints");
    assert_eq!(layout.tmp, "blender/tmp");

    let final_render = artifact_relative_path(BlenderArtifactScope::RenderFinal, "shot_010.png")
        .expect("contained render path");
    assert_eq!(final_render, "blender/renders/final/shot_010.png");
    validate_blender_relative_path(&final_render).expect("canonical final render");

    for rejected in [
        "/tmp/render.png",
        "../render.png",
        "creative/project/assets/render.png",
        "blender/other/render.png",
        "blender/renders/../escape.png",
    ] {
        assert!(
            validate_blender_relative_path(rejected).is_err(),
            "{rejected}"
        );
    }
    assert!(artifact_relative_path(BlenderArtifactScope::Scene, "../scene.blend").is_err());
    assert!(artifact_relative_path(BlenderArtifactScope::Export, "/tmp/model.glb").is_err());
}

#[test]
fn blender_v1_contract_has_exactly_eleven_bounded_tools_without_authority_injection() {
    let tools = blender_tool_catalog();
    let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "blender_session",
            "blender_inspect",
            "blender_python_api_docs",
            "blender_execute_python",
            "blender_screenshot",
            "blender_animation_preview",
            "blender_render",
            "blender_asset_import",
            "blender_asset_export",
            "blender_checkpoint_create",
            "blender_checkpoint_restore",
        ]
    );

    for tool in &tools {
        let schema_text = tool.input_schema.to_string();
        for forbidden in ["\"host\"", "\"port\"", "\"executable\"", "\"pid\""] {
            assert!(
                !schema_text.contains(forbidden),
                "{} exposes {forbidden}",
                tool.name
            );
        }
    }

    let session = tools
        .iter()
        .find(|tool| tool.name == "blender_session")
        .unwrap();
    validate_tool_arguments(
        session,
        &json!({"cwd":"/workspace/project","project_id":"project_demo","action":"status"}),
    )
    .expect("status schema");
    assert!(validate_tool_arguments(
        session,
        &json!({
            "cwd":"/workspace/project",
            "project_id":"project_demo",
            "action":"start",
            "host":"10.0.0.9"
        }),
    )
    .is_err());

    let render = tools
        .iter()
        .find(|tool| tool.name == "blender_render")
        .unwrap();
    validate_tool_arguments(
        render,
        &json!({
            "cwd":"/workspace/project",
            "project_id":"project_demo",
            "mode":"still",
            "output_scope":"final",
            "file_name":"hero_frame.png"
        }),
    )
    .expect("contained render schema");
    assert!(validate_tool_arguments(
        render,
        &json!({
            "cwd":"/workspace/project",
            "project_id":"project_demo",
            "mode":"still",
            "output_scope":"final",
            "file_name":"../escape.png"
        }),
    )
    .is_err());

    let import = tools
        .iter()
        .find(|tool| tool.name == "blender_asset_import")
        .unwrap();
    assert!(validate_tool_arguments(
        import,
        &json!({
            "cwd":"/workspace/project",
            "project_id":"project_demo",
            "asset_id":"asset_ref",
            "purpose":"reference",
            "url":"https://example.invalid/ref.png"
        }),
    )
    .is_err());
}

#[test]
fn blender_effect_policy_keeps_reads_start_stop_python_and_restore_distinct() {
    let status =
        effect_classes_for_call("blender_session", false, false, &json!({"action":"status"}));
    assert!(status.contains(&"privileged_bridge"));
    assert!(!status.contains(&"process_exec"));

    let start =
        effect_classes_for_call("blender_session", false, false, &json!({"action":"start"}));
    assert!(start.contains(&"process_exec"));
    assert!(start.contains(&"workspace_write"));

    let python = effect_classes_for_call("blender_execute_python", true, true, &json!({}));
    assert!(python.contains(&"process_exec"));
    assert!(python.contains(&"external_mutation"));
    assert!(python.contains(&"privileged_bridge"));

    let restore = effect_classes_for_call("blender_checkpoint_restore", true, false, &json!({}));
    assert!(restore.contains(&"workspace_delete"));
    assert!(restore.contains(&"privileged_bridge"));
}
