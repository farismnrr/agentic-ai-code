use super::{call, create_project, dispatch_sync, TempWorkspace};
use ai_tools::core::config::ToolProfile;
use ai_tools::interfaces::mcp::{runtime_tool_catalog, validate_tool_arguments};
use serde_json::json;
use std::collections::HashSet;

#[test]
fn plan069_mcp_contract_parity_matrix_is_present_and_fail_closed() {
    let workspace = TempWorkspace::new();
    let mut config = workspace.config();
    config.creative_binding_descriptors =
        vec![super::jobs::binding_descriptor_for_workflow_tests()];
    create_project(&config, "project_parity");

    let workflows = call(&config, "creative_catalog", json!({"catalog":"workflows"}));
    let workflow_ids = workflows["items"]
        .as_array()
        .expect("workflow catalog")
        .iter()
        .filter_map(|item| item["workflow_id"].as_str())
        .collect::<HashSet<_>>();
    for required in [
        "character_turnaround",
        "image_to_3d_bootstrap",
        "character_mesh_production",
        "character_rig_production",
        "character_action",
        "character_facial_performance",
        "character_secondary_motion",
        "storyboard_auto",
        "storyboard_manual",
        "shot_render_preview",
        "scene_continuity_review",
        "visual_qa_evidence",
        "temporal_qa_evidence",
        "scoped_revision",
        "video_reframe",
        "video_upscale",
        "video_remove_background",
        "video_motion_control",
        "video_clip_extract",
        "voice_clone",
        "voice_change",
        "video_dub",
        "export_profile",
        "project_handoff",
        "game_source_scaffold",
        "game_build_playtest",
        "game_iteration",
        "game_deploy",
        "promo_pack",
        "key_visual_bundle",
        "portfolio_delivery",
    ] {
        assert!(
            workflow_ids.contains(required),
            "missing parity workflow {required}"
        );
    }

    let catalog = runtime_tool_catalog(ToolProfile::Full, true);
    let graph = catalog
        .iter()
        .find(|tool| tool.name == "creative_graph")
        .expect("creative_graph");
    let project = catalog
        .iter()
        .find(|tool| tool.name == "creative_project")
        .expect("creative_project");
    let graph_actions = graph
        .input_schema
        .pointer("/properties/action/enum")
        .and_then(|v| v.as_array())
        .expect("graph action enum");
    for action in ["validate", "get", "template_save", "template_instantiate"] {
        assert!(
            graph_actions
                .iter()
                .any(|value| value.as_str() == Some(action)),
            "missing bounded graph action {action}"
        );
    }
    for action in [
        "execute",
        "partial_rerun",
        "rerun_selected",
        "rerun_subgraph",
        "rerun_all_dirty",
    ] {
        assert!(
            !graph_actions
                .iter()
                .any(|value| value.as_str() == Some(action)),
            "heavy graph action {action} must be operator CLI only"
        );
    }
    for action in [
        "scene_put",
        "game_put",
        "audio_put",
        "qa_add",
        "room_create",
        "room_join",
        "room_update",
        "room_get",
        "room_leave",
    ] {
        let enum_values = project
            .input_schema
            .pointer("/properties/action/enum")
            .and_then(|v| v.as_array())
            .expect("project action enum");
        assert!(
            enum_values
                .iter()
                .any(|value| value.as_str() == Some(action)),
            "missing project action {action}"
        );
    }

    let injection = validate_tool_arguments(
        graph,
        &json!({
            "action":"template_instantiate",
            "project_id":"project_parity",
            "template_id":"template_fixture",
            "new_graph_id":"graph_fixture",
            "binding_overrides":{"node_a":"binding_mock_image"},
            "input_overrides":{"node_a":{"endpoint":"https://forbidden.example"}}
        }),
    );
    assert!(
        injection.is_err(),
        "template input authority injection must fail closed"
    );

    let missing_binding = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"cost_estimate",
            "project_id":"project_parity",
            "capability_id":"video.generate",
            "parameters":{"duration_ms":1000}
        }),
    );
    assert!(
        missing_binding.is_err(),
        "pluggable execution must never auto-select a binding"
    );

    let deploy_publish = call(
        &config,
        "creative_catalog",
        json!({"catalog":"workflows","id":"game_deploy"}),
    );
    let text = deploy_publish.to_string();
    assert!(
        !text.contains("game.publish"),
        "deploy contract must not smuggle publish authority"
    );
}
