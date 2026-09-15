use super::media::{configured_workspace, run_capability};
use super::{call, create_project, dispatch_sync};
use serde_json::json;

fn create_element(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    kind: &str,
    name: &str,
    spec: serde_json::Value,
) -> (String, String) {
    let created = call(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":project_id,
            "kind":kind,
            "name":name,
            "authority":"authoritative",
            "reference_asset_ids":[],
            "spec":spec
        }),
    );
    (
        created["element_id"].as_str().unwrap().to_owned(),
        created["revision_id"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn world_location_pack_is_typed_reusable_and_rejects_ambiguous_specs() {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_world");

    let (style_id, style_revision_id) = create_element(
        &config,
        "project_world",
        "style",
        "World Style",
        json!({"palette":"cyan-magenta","lighting":"high contrast"}),
    );
    call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_world",
            "element_id":style_id,
            "revision_id":style_revision_id
        }),
    );

    let (set_id, set_revision_id) = create_element(
        &config,
        "project_world",
        "asset3d",
        "Alley Set",
        json!({"role":"environment_set"}),
    );
    call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_world",
            "element_id":set_id,
            "revision_id":set_revision_id
        }),
    );

    let (location_id, location_revision_id) = create_element(
        &config,
        "project_world",
        "location",
        "Neon Alley",
        json!({
            "spec_type":"world_location_v1",
            "concept":"Narrow recurring neon alley used by multiple shots.",
            "scale":"pedestrian alley",
            "architecture_language":"brick, metal shutters, overhead utilities",
            "set_dressing_language":"ramen sign, vending machines, drain channels",
            "canonical_landmarks":["north_gate","ramen_sign"],
            "variants":[
                {"variant_id":"day_clear","time_of_day":"day","weather":"clear","lighting":"soft daylight"},
                {"variant_id":"rain_night","time_of_day":"night","weather":"rain","lighting":"neon wet reflections"}
            ],
            "style_element_id":style_id,
            "asset3d_element_ids":[set_id],
            "camera_landmarks":["north_gate_wide","ramen_sign_close"],
            "continuity_notes":"North gate stays opposite the ramen sign across variants."
        }),
    );
    call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_world",
            "element_id":location_id,
            "revision_id":location_revision_id
        }),
    );

    for (prompt, variant) in [
        ("caller-authored daytime location key art", "day_clear"),
        ("caller-authored rainy night location key art", "rain_night"),
    ] {
        let generated = run_capability(
            &config,
            "project_world",
            "image.generate",
            json!({
                "prompt":prompt,
                "element_id":location_id,
                "style_element_id":style_id,
                "width":48,
                "height":32
            }),
        );
        assert_eq!(generated["job"]["status"], "completed");
        let asset_id = generated["job"]["output_asset_ids"][0].as_str().unwrap();
        let asset = call(
            &config,
            "creative_asset",
            json!({
                "action":"get",
                "project_id":"project_world",
                "asset_id":asset_id
            }),
        );
        assert_eq!(asset["asset"]["element_id"], location_id);
        assert_eq!(asset["asset"]["dependency_element_ids"], json!([style_id]));
        assert!(matches!(variant, "day_clear" | "rain_night"));
    }

    let reloaded = call(
        &config,
        "creative_element",
        json!({
            "action":"get",
            "project_id":"project_world",
            "element_id":location_id
        }),
    );
    assert_eq!(reloaded["element"]["kind"], "location");
    assert_eq!(
        reloaded["element"]["selected_revision_id"],
        location_revision_id
    );
    let selected = reloaded["element"]["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| revision["revision_id"] == location_revision_id)
        .unwrap();
    assert_eq!(selected["spec"]["spec_type"], "world_location_v1");
    assert_eq!(selected["spec"]["style_element_id"], style_id);
    assert_eq!(selected["spec"]["asset3d_element_ids"], json!([set_id]));
    assert_eq!(
        selected["spec"]["variants"].as_array().map(Vec::len),
        Some(2)
    );

    let ambiguous = dispatch_sync(
        &config,
        "creative_element",
        &json!({
            "action":"create_revision",
            "project_id":"project_world",
            "kind":"location",
            "name":"Invalid Location",
            "authority":"authoritative",
            "reference_asset_ids":[],
            "spec":{
                "spec_type":"world_location_v1",
                "concept":"missing required typed world fields",
                "scale":"small",
                "architecture_language":"brick",
                "set_dressing_language":"signs",
                "provider_specific_prompt":"forbidden ambiguous extension"
            }
        }),
    );
    assert!(ambiguous.is_err());
}
