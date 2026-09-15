use super::{call, create_project, dispatch_sync, TempWorkspace};
use serde_json::json;

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
