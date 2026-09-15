use super::*;

#[test]
fn fresh_game_contract_benchmark_builds_deploys_without_publish_and_preserves_iteration() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    let project_id = "closure_game";
    create_project(&config, &workspace, project_id, &["game"]);
    let style = accepted_element(
        &config,
        &workspace,
        project_id,
        "style",
        "Arcade Paper",
        json!({"render_intent":"flat graphic arcade","contrast":"high"}),
    );
    call(
        &config,
        "creative_project",
        json!({
            "action":"game_put","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game":{
                "game_id":"parcel_dash","title":"Parcel Dash","revision_id":"game_rev_1","production_intent":"build",
                "genre":"arcade","perspective":"top_down","core_loop":"Collect parcels while avoiding hazards.",
                "win_condition":"Collect ten parcels.","lose_condition":"Lose all three lives.","restart_behavior":"Press R or Enter to restart.",
                "progression":"Score increases per parcel.","camera":"fixed top-down","language":"en",
                "physics_timing":{"fixed_step_ms":16},"style_formula":{"shape":"paper cutout","palette":"limited"},
                "placeholder_policy":"No missing required runtime assets.","runtime_budget":{"target_fps":60,"max_asset_bytes":10485760,"max_initial_load_ms":3000},
                "player_mode":"solo","target_devices":["desktop","mobile"],"verbs":["move","collect","restart"],"inputs":["keyboard","touch"],
                "style_element_id":style,"asset_roles":[]
            }
        }),
    );
    let source = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "game_source_scaffold"),
        json!({"game_id":"parcel_dash","template_family":"2d_canvas"}),
        None,
    );
    assert_eq!(source["status"], "completed");
    let build = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "game_build_playtest"),
        json!({"game_id":"parcel_dash"}),
        None,
    );
    assert_eq!(build["status"], "completed");
    assert_eq!(build["node_runs"][0]["output"]["hard_fail"], false);
    assert_eq!(
        build["node_runs"][0]["output"]["evidence"]["browser_runtime_inspection"],
        "not_inspected"
    );
    let build_revision = build["node_runs"][0]["output"]["build_revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let deploy = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "game_deploy"),
        json!({"game_id":"parcel_dash","build_revision":build_revision}),
        Some("test_closure_media"),
    );
    assert_eq!(deploy["status"], "completed");
    assert_eq!(deploy["node_runs"][0]["output"]["published"], false);
    assert!(deploy["node_runs"][0]["output"]["deployment_url"]
        .as_str()
        .unwrap()
        .starts_with("https://example.invalid/"));

    let iteration = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "game_iteration"),
        json!({"game_id":"parcel_dash","changed_fields":["movement_speed"],"gameplay_impacting":true}),
        None,
    );
    assert_eq!(iteration["status"], "completed");
    assert_eq!(
        iteration["node_runs"][0]["output"]["receipt"]["source_preserved"],
        true
    );
    let game = call(
        &config,
        "creative_project",
        json!({"action":"game_get","cwd":workspace.0.to_string_lossy(),"project_id":project_id,"game_id":"parcel_dash"}),
    );
    assert!(game["game"]["build_state"]["source_revision"].is_string());
    assert!(game["game"]["build_state"]["accepted_build_revision"].is_null());
    assert_eq!(game["game"]["build_state"]["published"], false);
}

#[test]
fn multiplayer_rooms_are_owner_bound_revision_safe_and_isolated() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    let project_id = "closure_multiplayer";
    create_project(&config, &workspace, project_id, &["game"]);
    call(
        &config,
        "creative_project",
        json!({
            "action":"game_put","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game":{
                "game_id":"online_duel","title":"Online Duel","production_intent":"build","genre":"arena","perspective":"top_down",
                "core_loop":"Move and tag the rival.","win_condition":"Reach five tags.","lose_condition":"Rival reaches five tags.",
                "restart_behavior":"Start a new round.","player_mode":"online_multiplayer","target_devices":["desktop"],
                "verbs":["move","tag"],"inputs":["keyboard"],"asset_roles":[]
            }
        }),
    );
    let created = call_as_owner(
        &config,
        "owner_a",
        "creative_project",
        json!({
            "action":"room_create","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","member_id":"member_a","shared_state":{"score_a":0,"score_b":0}
        }),
    );
    let room_id = created["room"]["room_id"].as_str().unwrap().to_owned();
    assert_eq!(created["room"]["revision"], 1);

    let cross_owner = dispatch_as_owner(
        &config,
        "owner_b",
        "creative_project",
        &json!({
            "action":"room_get","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","room_id":room_id
        }),
    );
    assert!(cross_owner.is_err());

    let stale = dispatch_as_owner(
        &config,
        "owner_a",
        "creative_project",
        &json!({
            "action":"room_update","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","room_id":room_id,"expected_revision":0,
            "shared_state":{"score_a":1,"score_b":0}
        }),
    );
    assert!(stale.is_err());

    let updated = call_as_owner(
        &config,
        "owner_a",
        "creative_project",
        json!({
            "action":"room_update","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","room_id":room_id,"expected_revision":1,
            "shared_state":{"score_a":1,"score_b":0}
        }),
    );
    assert_eq!(updated["room"]["revision"], 2);
    assert_eq!(updated["room"]["shared_state"]["score_a"], 1);

    let second = call_as_owner(
        &config,
        "owner_a",
        "creative_project",
        json!({
            "action":"room_create","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","member_id":"member_x","shared_state":{"round":2}
        }),
    );
    assert_ne!(second["room"]["room_id"], room_id);

    let closed = call_as_owner(
        &config,
        "owner_a",
        "creative_project",
        json!({
            "action":"room_leave","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "game_id":"online_duel","room_id":room_id,"member_id":"member_a"
        }),
    );
    assert_eq!(closed["room"]["closed"], true);
    assert_eq!(closed["room"]["member_ids"].as_array().unwrap().len(), 0);
}
