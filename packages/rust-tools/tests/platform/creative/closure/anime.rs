use super::*;

#[test]
fn fresh_anime_contract_benchmark_preserves_character_scene_and_handoff() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    let project_id = "closure_anime";
    create_project(&config, &workspace, project_id, &["anime"]);
    let character = accepted_element(
        &config,
        &workspace,
        project_id,
        "character",
        "Kiko",
        json!({"appearance":"blue jacket and silver bob","archetype":"optimistic mechanic"}),
    );
    let style = accepted_element(
        &config,
        &workspace,
        project_id,
        "style",
        "Cel Dawn",
        json!({"render_intent":"two-tone cel shading","line_weight":"clean"}),
    );
    let videos = [
        generated_video(
            &config,
            &workspace,
            project_id,
            4_000,
            "anime workshop establishing",
        ),
        generated_video(
            &config,
            &workspace,
            project_id,
            4_000,
            "anime mechanic action",
        ),
        generated_video(
            &config,
            &workspace,
            project_id,
            4_000,
            "anime reaction closeup",
        ),
    ];
    call(
        &config,
        "creative_project",
        json!({
            "action":"scene_put","cwd":workspace.0.to_string_lossy(),"project_id":project_id,
            "scene":{
                "scene_id":"anime_workshop","title":"Workshop Spark","revision_id":"anime_scene_rev_1","planning_mode":"manual",
                "global_direction":{"genre_look":"bright mechanical anime","lighting":"dawn window light","color_palette":["sky_blue","warm_orange"],"hero_frame_first":true},
                "cast_element_ids":[character],"style_element_id":style,"target_duration_ms":12000,"selected_hero_frame_asset_id":videos[1],
                "shots":[
                    {"shot_id":"anime_1","order":1,"duration_ms":4000,"element_ids":[character],"reference_asset_ids":[videos[0]],"engine":"mixed","action":"Workshop reveal.","state_in":{"tool":"bench"},"state_out":{"tool":"bench"},"continuity":{"outfit":"blue_jacket"}},
                    {"shot_id":"anime_2","order":2,"duration_ms":4000,"element_ids":[character],"reference_asset_ids":[videos[1]],"engine":"mixed","action":"Kiko starts the engine.","state_in":{"tool":"bench"},"state_out":{"engine":"running"},"continuity":{"outfit":"blue_jacket"}},
                    {"shot_id":"anime_3","order":3,"duration_ms":4000,"element_ids":[character],"reference_asset_ids":[videos[2]],"engine":"mixed","action":"Kiko smiles.","state_in":{"engine":"running"},"state_out":{"engine":"running"},"continuity":{"outfit":"blue_jacket"}}
                ]
            }
        }),
    );
    let continuity = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "scene_continuity_review"),
        json!({"scene_id":"anime_workshop"}),
        None,
    );
    assert_eq!(continuity["status"], "completed");
    let assembled = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "sequence_assemble"),
        json!({"video_asset_ids":videos,"fps":24,"width":1280,"height":720}),
        Some("test_closure_media"),
    );
    assert_eq!(assembled["status"], "completed");
    let final_asset = assembled["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    promote_asset(&config, &workspace, project_id, &final_asset);
    let handoff = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "project_handoff"),
        json!({"label":"anime-workshop-handoff"}),
        None,
    );
    assert_eq!(handoff["status"], "completed");
    assert_eq!(
        handoff["node_runs"][0]["output"]["publish_performed"],
        false
    );
    let project = call(
        &config,
        "creative_project",
        json!({"action":"get","cwd":workspace.0.to_string_lossy(),"project_id":project_id}),
    );
    assert_eq!(
        project["project"]["scenes"][0]["target_duration_ms"],
        12_000
    );
    assert_eq!(project["project"]["elements"].as_array().unwrap().len(), 2);
}
