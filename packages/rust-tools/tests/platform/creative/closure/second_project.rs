use super::*;

#[test]
fn materially_different_second_project_uses_the_same_contracts_without_special_cases() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    create_project(&config, &workspace, "closure_second", &["scene", "game"]);
    let style = accepted_element(
        &config,
        &workspace,
        "closure_second",
        "style",
        "Woodblock Folklore",
        json!({"render_intent":"inked woodblock folklore","palette":["indigo","paper"]}),
    );
    call(
        &config,
        "creative_project",
        json!({
            "action":"scene_put","cwd":workspace.0.to_string_lossy(),"project_id":"closure_second",
            "scene":{"scene_id":"quiet_shrine","title":"Quiet Shrine","planning_mode":"auto","global_direction":{"genre_look":"folklore contemplative","hero_frame_first":true},"style_element_id":style,"target_duration_ms":10000,"shots":[{"shot_id":"shrine_1","order":1,"duration_ms":10000,"engine":"generated_video","action":"Wind moves prayer ribbons.","continuity":{"weather":"still_cold"}}]}
        }),
    );
    call(
        &config,
        "creative_project",
        json!({
            "action":"game_put","cwd":workspace.0.to_string_lossy(),"project_id":"closure_second",
            "game":{"game_id":"moss_puzzle","title":"Moss Puzzle","production_intent":"design_only","genre":"puzzle","perspective":"side_view","core_loop":"Rotate stones to guide water.","win_condition":"Water reaches the shrine.","lose_condition":"No legal moves remain.","restart_behavior":"Reset the board.","player_mode":"solo","target_devices":["desktop"],"verbs":["rotate","reset"],"inputs":["mouse"],"style_element_id":style,"asset_roles":[]}
        }),
    );
    let project = call(
        &config,
        "creative_project",
        json!({"action":"get","cwd":workspace.0.to_string_lossy(),"project_id":"closure_second"}),
    );
    assert_eq!(
        project["project"]["scenes"][0]["global_direction"]["genre_look"],
        "folklore contemplative"
    );
    assert_eq!(project["project"]["games"][0]["genre"], "puzzle");
    assert_eq!(project["project"]["games"][0]["inputs"][0], "mouse");
}
