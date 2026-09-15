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
            "action":"create_revision", "project_id":project_id, "kind":kind, "name":name,
            "authority":authority, "reference_asset_ids":references, "spec":spec
        }),
    );
    let element_id = created["element_id"].as_str().unwrap().to_owned();
    let revision_id = created["revision_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_element",
        json!({
            "action":"promote", "project_id":project_id, "element_id":element_id, "revision_id":revision_id
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
            "action":"submit", "project_id":project_id, "workflow_id":workflow_id,
            "execution_binding_id":"binding_local_raster", "parameters":parameters, "approved":true
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({
            "action":"wait", "project_id":project_id, "job_id":job_id
        }),
    )
}

#[test]
fn style_revision_marks_only_declared_assets_and_scene_shots_for_review() {
    let (workspace, config) = configured_workspace();
    create_project(&config, "project_style_dependency");

    let (style_element_id, first_style_revision_id) = create_and_promote_element(
        &config,
        "project_style_dependency",
        "style",
        "Dependency Style",
        "authoritative",
        vec![],
        json!({"palette":"blue","line":"clean","material":"toon"}),
    );

    let unrelated = run_capability(
        &config,
        "project_style_dependency",
        "image.generate",
        json!({"prompt":"unrelated accepted reference","width":48,"height":48}),
    );
    let unrelated_asset_id = unrelated["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    call(
        &config,
        "creative_asset",
        json!({
            "action":"promote",
            "project_id":"project_style_dependency",
            "asset_id":unrelated_asset_id
        }),
    );

    let (character_element_id, _) = create_and_promote_element(
        &config,
        "project_style_dependency",
        "character",
        "Dependency Character",
        "authoritative",
        vec![unrelated_asset_id.clone()],
        json!({"identity":"stable"}),
    );
    let turnaround = run_workflow(
        &config,
        "project_style_dependency",
        "character_turnaround",
        json!({
            "element_id":character_element_id,
            "style_element_id":style_element_id,
            "reference_asset_ids":[unrelated_asset_id],
            "views":["front","side"]
        }),
    );
    let dependent_asset_ids = turnaround["job"]["output_asset_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    let project_path =
        workspace.path(".masihawam/creative/projects/project_style_dependency/project.json");
    let mut persisted: Value = serde_json::from_slice(
        &std::fs::read(&project_path).expect("read persisted creative project"),
    )
    .expect("parse persisted creative project");
    persisted["scenes"] = json!([{
        "scene_id":"scene_style",
        "title":"Style dependency fixture",
        "cast_element_ids":[character_element_id],
        "style_element_id":style_element_id,
        "target_duration_ms":1000,
        "shots":[{
            "shot_id":"shot_style_1",
            "order":1,
            "duration_ms":1000,
            "element_ids":[character_element_id],
            "camera":{},
            "action":"hold",
            "continuity":{}
        }]
    }]);
    std::fs::write(
        &project_path,
        serde_json::to_vec_pretty(&persisted).expect("serialize creative project fixture"),
    )
    .expect("write creative project fixture");

    let next_style = call(
        &config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":"project_style_dependency",
            "element_id":style_element_id,
            "authority":"authoritative",
            "reference_asset_ids":[],
            "spec":{"palette":"red","line":"clean","material":"toon"}
        }),
    );
    let next_style_revision_id = next_style["revision_id"].as_str().unwrap().to_owned();
    assert_ne!(next_style_revision_id, first_style_revision_id);
    let promoted = call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_style_dependency",
            "element_id":style_element_id,
            "revision_id":next_style_revision_id
        }),
    );
    assert_eq!(promoted["selected_revision_id"], next_style_revision_id);

    let project = call(
        &config,
        "creative_project",
        json!({"action":"get","project_id":"project_style_dependency"}),
    );
    let findings = project["project"]["qa_findings"].as_array().unwrap();
    let review_subjects = findings
        .iter()
        .filter(|finding| finding["domain"] == "style_dependency")
        .map(|finding| finding["subject_id"].as_str().unwrap().to_owned())
        .collect::<std::collections::HashSet<_>>();
    for asset_id in &dependent_asset_ids {
        assert!(review_subjects.contains(asset_id));
        let asset = call(
            &config,
            "creative_asset",
            json!({
                "action":"get",
                "project_id":"project_style_dependency",
                "asset_id":asset_id
            }),
        );
        assert_eq!(asset["asset"]["state"], "candidate");
    }
    assert!(review_subjects.contains("shot_style_1"));
    assert!(!review_subjects.contains(&unrelated_asset_id));
    assert!(findings
        .iter()
        .filter(|finding| finding["domain"] == "style_dependency")
        .all(|finding| finding["severity"] == "soft_finding"));

    let unrelated = call(
        &config,
        "creative_asset",
        json!({
            "action":"get",
            "project_id":"project_style_dependency",
            "asset_id":unrelated_asset_id
        }),
    );
    assert_eq!(unrelated["asset"]["state"], "accepted");

    let finding_count = findings.len();
    call(
        &config,
        "creative_element",
        json!({
            "action":"promote",
            "project_id":"project_style_dependency",
            "element_id":style_element_id,
            "revision_id":next_style_revision_id
        }),
    );
    let repeated = call(
        &config,
        "creative_project",
        json!({"action":"get","project_id":"project_style_dependency"}),
    );
    assert_eq!(
        repeated["project"]["qa_findings"].as_array().unwrap().len(),
        finding_count
    );
}
