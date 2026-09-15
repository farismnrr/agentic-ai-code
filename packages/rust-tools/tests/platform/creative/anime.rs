use super::media::{configured_workspace, run_capability};
use super::{call, create_project};
use serde_json::{json, Value};

fn create_and_promote_element(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    kind: &str,
    name: &str,
    authority: &str,
    references: Vec<String>,
    spec: Value,
) -> (String, String) {
    let created = call(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":project_id,
            "kind":kind,
            "name":name,
            "authority":authority,
            "reference_asset_ids":references,
            "spec":spec
        }),
    );
    let element_id = created["element_id"].as_str().unwrap().to_owned();
    let revision_id = created["revision_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":project_id,
            "element_id":element_id,
            "revision_id":revision_id
        }),
    );
    (element_id, revision_id)
}

fn run_workflow(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    workflow_id: &str,
    parameters: Value,
) -> Value {
    let submitted = call(
        config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":project_id,
            "workflow_id":workflow_id,
            "execution_binding_id":"binding_local_raster",
            "parameters":parameters,
            "approved":true
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({
            "action":"wait",
            "project_id":project_id,
            "job_id":job_id
        }),
    )
}

#[test]
fn upper_layer_can_drive_character_style_turnaround_and_explicit_promotion_without_subjective_mcp_qa(
) {
    let (_workspace, config) = configured_workspace();
    create_project(&config, "project_anime_still");

    let (style_element_id, _) = create_and_promote_element(
        &config,
        "project_anime_still",
        "style",
        "Caller Approved Toon Style",
        "authoritative",
        vec![],
        json!({
            "palette":"caller-authored-blue-gold",
            "line_weight":"caller-authored-clean",
            "material_language":"caller-authored-toon"
        }),
    );

    let base = run_capability(
        &config,
        "project_anime_still",
        "image.generate",
        json!({
            "prompt":"caller authored neutral character reference",
            "width":64,
            "height":64
        }),
    );
    let base_asset_id = base["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();

    let (character_element_id, _) = create_and_promote_element(
        &config,
        "project_anime_still",
        "character",
        "Caller Approved Character",
        "authoritative",
        vec![base_asset_id.clone()],
        json!({
            "style_element_id":style_element_id,
            "identity_notes":"caller-authored identity constraints"
        }),
    );

    let requested_views = vec![
        "front",
        "side",
        "back",
        "three_quarter",
        "rear_three_quarter_hidden",
    ];
    let turnaround = run_workflow(
        &config,
        "project_anime_still",
        "character_turnaround",
        json!({
            "element_id":character_element_id,
            "reference_asset_ids":[base_asset_id],
            "views":requested_views,
            "prompt":"caller authored turnaround instruction"
        }),
    );
    assert_eq!(turnaround["job"]["status"], "completed");
    assert_eq!(
        turnaround["job"]["output_asset_ids"]
            .as_array()
            .map(Vec::len),
        Some(5)
    );
    assert_eq!(
        turnaround["job"]["node_runs"].as_array().map(Vec::len),
        Some(5)
    );
    assert!(turnaround["job"]["node_runs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|run| run["output"]["variant_label"] == "rear_three_quarter_hidden"));
    for run in turnaround["job"]["node_runs"].as_array().unwrap() {
        assert_eq!(run["status"], "completed");
        assert_eq!(run["output"]["reference_authority"], "interpreted");
        assert_eq!(run["output"]["inspection"], "not_inspected");
        assert_eq!(run["output"]["element_id"], character_element_id);
        assert_eq!(run["output"]["variant_kind"], "view");
    }

    let turnaround_ids = turnaround["job"]["output_asset_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    for asset_id in &turnaround_ids {
        let asset = call(
            &config,
            "creative_asset",
            json!({
                "action":"get",
                "project_id":"project_anime_still",
                "asset_id":asset_id
            }),
        );
        assert_eq!(asset["asset"]["source"], "generated_asset");
        assert_eq!(asset["asset"]["source_surface"], "anime");
        assert_eq!(asset["asset"]["state"], "candidate");
        assert_eq!(asset["asset"]["element_id"], character_element_id);
        assert_eq!(asset["asset"]["role"], "character_reference_interpreted");
    }

    let expressions = run_workflow(
        &config,
        "project_anime_still",
        "expression_sheet",
        json!({
            "element_id":character_element_id,
            "reference_asset_ids":turnaround_ids,
            "expressions":["neutral","smile","angry" ]
        }),
    );
    assert_eq!(expressions["job"]["status"], "completed");
    assert_eq!(
        expressions["job"]["output_asset_ids"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    for run in expressions["job"]["node_runs"].as_array().unwrap() {
        assert_eq!(run["output"]["variant_kind"], "expression");
        assert_eq!(run["output"]["reference_authority"], "interpreted");
        assert_eq!(run["output"]["inspection"], "not_inspected");
    }

    let rejected_expression_id = expressions["job"]["output_asset_ids"][1]
        .as_str()
        .unwrap()
        .to_owned();
    let rejected_asset = call(
        &config,
        "creative_asset",
        json!({
            "action":"reject",
            "project_id":"project_anime_still",
            "asset_id":rejected_expression_id
        }),
    );
    assert_eq!(rejected_asset["state"], "rejected");
    let rejected_asset = call(
        &config,
        "creative_asset",
        json!({
            "action":"get",
            "project_id":"project_anime_still",
            "asset_id":rejected_expression_id
        }),
    );
    assert_eq!(rejected_asset["asset"]["state"], "rejected");

    let rejected_revision = call(
        &config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":"project_anime_still",
            "element_id":character_element_id,
            "authority":"interpreted",
            "reference_asset_ids":[rejected_expression_id],
            "spec":{"review":"external_rejected_candidate"}
        }),
    );
    let rejected_revision_id = rejected_revision["revision_id"]
        .as_str()
        .unwrap()
        .to_owned();
    call(
        &config,
        "creative_element",
        json!({
            "action":"reject",
            "project_id":"project_anime_still",
            "element_id":character_element_id,
            "revision_id":rejected_revision_id
        }),
    );
    let rejected_element_state = call(
        &config,
        "creative_element",
        json!({
            "action":"get",
            "project_id":"project_anime_still",
            "element_id":character_element_id
        }),
    );
    let rejected_revision = rejected_element_state["element"]["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| revision["revision_id"] == rejected_revision_id)
        .unwrap();
    assert_eq!(rejected_revision["state"], "rejected");

    for asset_id in &turnaround_ids {
        call(
            &config,
            "creative_asset",
            json!({
                "action":"promote",
                "project_id":"project_anime_still",
                "asset_id":asset_id
            }),
        );
    }
    let revised = call(
        &config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":"project_anime_still",
            "element_id":character_element_id,
            "authority":"interpreted",
            "reference_asset_ids":turnaround_ids,
            "spec":{
                "style_element_id":style_element_id,
                "accepted_by":"external_upper_layer",
                "subjective_review":"external_not_mcp"
            }
        }),
    );
    let interpreted_revision_id = revised["revision_id"].as_str().unwrap().to_owned();
    call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_anime_still",
            "element_id":character_element_id,
            "revision_id":interpreted_revision_id
        }),
    );

    let character = call(
        &config,
        "creative_element",
        json!({
            "action":"get",
            "project_id":"project_anime_still",
            "element_id":character_element_id
        }),
    );
    assert_eq!(
        character["element"]["selected_revision_id"],
        interpreted_revision_id
    );
    let selected = character["element"]["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| revision["revision_id"] == interpreted_revision_id)
        .unwrap();
    assert_eq!(selected["authority"], "interpreted");
    assert_eq!(selected["state"], "accepted");
    assert_eq!(
        selected["reference_asset_ids"].as_array().map(Vec::len),
        Some(5)
    );

    let style = call(
        &config,
        "creative_element",
        json!({
            "action":"get",
            "project_id":"project_anime_still",
            "element_id":style_element_id
        }),
    );
    assert!(style["element"]["selected_revision_id"].as_str().is_some());
}
