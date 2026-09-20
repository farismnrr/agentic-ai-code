use super::*;

#[test]
fn fresh_scene_studio_contract_benchmark_reaches_contained_export() {
    let workspace = TempWorkspace::new();
    let config = config(&workspace);
    let project_id = "closure_scene";
    create_project(&config, &workspace, project_id, &["scene"]);
    let character = accepted_element(
        &config,
        &workspace,
        project_id,
        "character",
        "Mira",
        json!({"appearance":"short dark hair","role":"courier"}),
    );
    let style = accepted_element(
        &config,
        &workspace,
        project_id,
        "style",
        "Rain Neon",
        json!({"palette":["cyan","amber"],"render_intent":"cinematic"}),
    );

    let videos = [
        generated_video(&config, &workspace, project_id, 4_000, "wide arrival"),
        generated_video(&config, &workspace, project_id, 4_000, "medium reveal"),
        generated_video(&config, &workspace, project_id, 4_000, "close reaction"),
    ];

    call(
        &config,
        "creative_project",
        json!({
            "action":"scene_put",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "scene":{
                "scene_id":"scene_rain_arrival",
                "title":"Rain Arrival",
                "revision_id":"scene_rev_1",
                "planning_mode":"manual",
                "global_direction":{
                    "genre_look":"grounded neon thriller",
                    "lighting":"rainy practicals",
                    "color_palette":["cyan","amber"],
                    "atmosphere":"wet night street",
                    "era_time":"near future night",
                    "spatial_constraints":{"screen_direction":"left_to_right"},
                    "hero_frame_first":true
                },
                "cast_element_ids":[character],
                "style_element_id":style,
                "target_duration_ms":12000,
                "selected_hero_frame_asset_id":videos[0],
                "shots":[
                    {
                        "shot_id":"shot_1","order":1,"duration_ms":4000,"revision_id":"shot_rev_1",
                        "element_ids":[character],"reference_asset_ids":[videos[0]],"hero_frame_asset_id":videos[0],
                        "engine":"generated_video","execution_binding_id":"test_closure_media",
                        "camera":{"shot_size":"wide","focal_length_mm":35.0,"movement":"dolly_in"},
                        "action":"Mira enters the rain.","state_in":{"position":"offscreen_left"},"state_out":{"position":"center"},
                        "continuity":{"screen_direction":"left_to_right"}
                    },
                    {
                        "shot_id":"shot_2","order":2,"duration_ms":4000,"revision_id":"shot_rev_2",
                        "element_ids":[character],"reference_asset_ids":[videos[1]],"engine":"generated_video",
                        "execution_binding_id":"test_closure_media","camera":{"shot_size":"medium","focal_length_mm":50.0},
                        "action":"She checks the parcel.","state_in":{"position":"center"},"state_out":{"parcel":"raised"},
                        "continuity":{"screen_direction":"left_to_right"}
                    },
                    {
                        "shot_id":"shot_3","order":3,"duration_ms":4000,"revision_id":"shot_rev_3",
                        "element_ids":[character],"reference_asset_ids":[videos[2]],"engine":"generated_video",
                        "execution_binding_id":"test_closure_media","camera":{"shot_size":"close_up","focal_length_mm":85.0},
                        "action":"She sees the warning light.","state_in":{"parcel":"raised"},"state_out":{"reaction":"alert"},
                        "continuity":{"screen_direction":"left_to_right"}
                    }
                ]
            }
        }),
    );
    call(
        &config,
        "creative_project",
        json!({
            "action":"scene_board_put",
            "cwd":workspace.0.to_string_lossy(),
            "project_id":project_id,
            "scene_board":{
                "board_id":"board_rain_arrival","scene_id":"scene_rain_arrival","revision_id":"board_rev_1","planning_mode":"manual",
                "frames":[
                    {"frame_id":"frame_1","order":1,"shot_id":"shot_1","revision_id":"frame_rev_1","element_ids":[character],"reference_asset_ids":[videos[0]],"hero_candidate":true,"direction":{"framing":"wide"}},
                    {"frame_id":"frame_2","order":2,"shot_id":"shot_2","revision_id":"frame_rev_2","element_ids":[character],"reference_asset_ids":[videos[1]],"direction":{"framing":"medium"}},
                    {"frame_id":"frame_3","order":3,"shot_id":"shot_3","revision_id":"frame_rev_3","element_ids":[character],"reference_asset_ids":[videos[2]],"direction":{"framing":"close"}}
                ]
            }
        }),
    );

    let continuity = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "scene_continuity_review"),
        json!({"scene_id":"scene_rain_arrival"}),
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
    let assembled_asset = assembled["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    promote_asset(&config, &workspace, project_id, &assembled_asset);

    let exported = submit_wait(
        &config,
        &workspace,
        project_id,
        ("workflow_id", "export_profile"),
        json!({"asset_ids":[assembled_asset],"profile":"video","file_name":"scene-rain-arrival.mp4"}),
        None,
    );
    assert_eq!(exported["status"], "completed");
    assert_eq!(
        exported["node_runs"][0]["output"]["deploy_performed"],
        false
    );
    assert_eq!(
        exported["node_runs"][0]["output"]["publish_performed"],
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
    assert_eq!(
        project["project"]["scene_boards"][0]["frames"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(project["project"]["qa_findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["domain"] == "scene_continuity"));
}
