use ai_tools::application::blender::{
    artifact_relative_path, bounded_mcp_config, bridge_address, project_layout,
    public_mcp_timeout_ms, unbounded_operator_config, validate_blender_project_root_path,
    validate_blender_relative_path, BlenderArtifactScope, BlenderSessionOwnership,
    BlenderSessionState, BlenderSessionStatus, BLENDER_LAB_PROTOCOL, DEFAULT_BLENDER_LAB_PORT,
    MAX_BLENDER_PYTHON_BYTES, MAX_BLENDER_REQUEST_BYTES, MAX_BLENDER_RESPONSE_BYTES,
    REVIEWED_BLENDER_PATH_NAMES,
};
use ai_tools::application::hooks::effect_classes_for_call;
use ai_tools::core::config::{
    ServerConfig, BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS, BLENDER_MCP_DEFAULT_TIMEOUT_MS,
    BLENDER_MCP_SCREENSHOT_TIMEOUT_MS, BLENDER_MCP_SESSION_TIMEOUT_MS,
};
use ai_tools::interfaces::mcp::{blender_tool_catalog, validate_tool_arguments};
use serde_json::json;
use std::net::Ipv4Addr;

#[test]
fn blender_operator_config_is_creative_gated_loopback_only_and_bounded() {
    let default = ServerConfig::default();
    assert!(!default.enable_creative);
    assert_eq!(default.blender_bridge_port, DEFAULT_BLENDER_LAB_PORT);
    assert_eq!(bridge_address(&default).ip(), &Ipv4Addr::LOCALHOST);
    assert_eq!(BLENDER_LAB_PROTOCOL, "blender_lab_json_nul_v1");
    assert_eq!(MAX_BLENDER_REQUEST_BYTES, 1024 * 1024);
    assert_eq!(MAX_BLENDER_RESPONSE_BYTES, 8 * 1024 * 1024);
    assert_eq!(MAX_BLENDER_PYTHON_BYTES, 256 * 1024);
    assert_eq!(BLENDER_MCP_DEFAULT_TIMEOUT_MS, 60_000);
    assert_eq!(BLENDER_MCP_SESSION_TIMEOUT_MS, 120_000);
    assert_eq!(BLENDER_MCP_SCREENSHOT_TIMEOUT_MS, 120_000);
    assert_eq!(BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS, 180_000);
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
        blender_executable: Some("/opt/blender/blender".into()),
        ..ServerConfig::default()
    };
    enabled.validate().expect("reviewed Blender config");

    let mut privileged_port = enabled.clone();
    privileged_port.blender_bridge_port = 80;
    assert!(privileged_port.validate().is_err());

    let mut operator_timeout = enabled.clone();
    operator_timeout.blender_bridge_timeout_ms = 120_000;
    operator_timeout
        .validate()
        .expect("operator foreground timeout may exceed MCP ceiling");
    assert_eq!(
        bounded_mcp_config(&operator_timeout, "blender_session").blender_bridge_timeout_ms,
        BLENDER_MCP_SESSION_TIMEOUT_MS
    );
    assert_eq!(
        bounded_mcp_config(&operator_timeout, "blender_inspect").blender_bridge_timeout_ms,
        BLENDER_MCP_DEFAULT_TIMEOUT_MS
    );
    assert_eq!(
        bounded_mcp_config(&operator_timeout, "blender_screenshot").blender_bridge_timeout_ms,
        BLENDER_MCP_SCREENSHOT_TIMEOUT_MS
    );
    assert_eq!(
        bounded_mcp_config(&operator_timeout, "blender_animation_preview")
            .blender_bridge_timeout_ms,
        BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS
    );
    assert_eq!(
        unbounded_operator_config(&operator_timeout).blender_bridge_timeout_ms,
        0
    );

    let mut unbounded_timeout = enabled.clone();
    unbounded_timeout.blender_bridge_timeout_ms = 120_001;
    assert!(unbounded_timeout.validate().is_err());

    let mut bad_executable = enabled;
    bad_executable.blender_executable = Some("blender\n--evil".into());
    assert!(bad_executable.validate().is_err());
}

#[test]
fn blender_project_root_requires_canonical_blender_parent() {
    use std::path::Path;

    validate_blender_project_root_path(Path::new("/tmp/Projects/Blender/ari-final"))
        .expect("canonical Blender project root");
    for rejected in [
        "/tmp/Projects/MasihAwam/ai-code",
        "/tmp/Projects/Blender",
        "/tmp/Projects/Creative/ari-final",
    ] {
        assert!(
            validate_blender_project_root_path(Path::new(rejected)).is_err(),
            "{rejected}"
        );
    }
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
fn blender_public_contract_exposes_only_bounded_agent_tools() {
    let tools = blender_tool_catalog();
    let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "blender_session",
            "blender_inspect",
            "blender_python_api_docs",
            "blender_screenshot",
            "blender_animation_preview",
        ]
    );

    for removed in [
        "blender_execute_python",
        "blender_render",
        "blender_asset_import",
        "blender_asset_export",
        "blender_checkpoint_create",
        "blender_checkpoint_restore",
    ] {
        assert!(
            !names.contains(&removed),
            "{removed} must be operator CLI only"
        );
    }

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

    for (name, expected_timeout_ms) in [
        ("blender_session", BLENDER_MCP_SESSION_TIMEOUT_MS),
        ("blender_inspect", BLENDER_MCP_DEFAULT_TIMEOUT_MS),
        ("blender_python_api_docs", BLENDER_MCP_DEFAULT_TIMEOUT_MS),
        ("blender_screenshot", BLENDER_MCP_SCREENSHOT_TIMEOUT_MS),
        (
            "blender_animation_preview",
            BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS,
        ),
    ] {
        let tool = tools.iter().find(|tool| tool.name == name).unwrap();
        assert_eq!(
            tool.execution
                .as_ref()
                .and_then(|execution| execution.get("timeoutMs"))
                .and_then(|value| value.as_u64()),
            Some(expected_timeout_ms),
            "{name} execution timeout contract"
        );
        assert_eq!(public_mcp_timeout_ms(name), Some(expected_timeout_ms));
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
