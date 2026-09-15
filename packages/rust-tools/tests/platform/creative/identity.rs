use super::media::{configured_workspace, run_capability};
use super::{call, create_project, dispatch_sync};
use serde_json::json;

fn identity_binding_descriptor(version: &str) -> String {
    json!({
        "binding_id":"test_identity_prepare",
        "binding_version":version,
        "capabilities":["identity.prepare"],
        "media_roles":["reference_image","identity_artifact"],
        "extension_schema":{"type":"object","additionalProperties":false},
        "constraints":{"estimate":{"base_compute_units":25,"base_output_bytes":4096}},
        "license_notes":"test identity artifact may be replaced; Character Pack remains authoritative",
        "estimate_available":true,
        "availability":"available"
    })
    .to_string()
}

fn create_character(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    reference_asset_id: &str,
) -> (String, String) {
    let created = call(
        config,
        "creative_element",
        json!({
            "action":"create_revision",
            "project_id":project_id,
            "kind":"character",
            "name":"Identity Character",
            "authority":"authoritative",
            "reference_asset_ids":[reference_asset_id],
            "spec":{"identity_contract":"reference_pack_authority"}
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

fn prepare_identity(
    config: &ai_tools::core::config::ServerConfig,
    project_id: &str,
    element_id: &str,
    reference_asset_id: &str,
    artifact_version: &str,
) -> serde_json::Value {
    let submitted = call(
        config,
        "creative_job",
        json!({
            "action":"submit",
            "project_id":project_id,
            "capability_id":"identity.prepare",
            "execution_binding_id":"test_identity_prepare",
            "parameters":{
                "element_id":element_id,
                "reference_asset_ids":[reference_asset_id],
                "subject_kind":"fictional",
                "artifact_kind":"embedding",
                "artifact_version":artifact_version
            },
            "approved":true
        }),
    );
    let job_id = submitted["job"]["job_id"].as_str().unwrap().to_owned();
    call(
        config,
        "creative_job",
        json!({"action":"wait","project_id":project_id,"job_id":job_id}),
    )
}

#[test]
fn identity_preparation_is_explicit_optional_traceable_and_replaceable() {
    let (_workspace, mut config) = configured_workspace();
    config
        .creative_binding_descriptors
        .push(identity_binding_descriptor("identity-test-v1"));
    create_project(&config, "project_identity_optional");

    let baseline = run_capability(
        &config,
        "project_identity_optional",
        "image.generate",
        json!({"prompt":"reference-only baseline","width":48,"height":48}),
    );
    let reference_asset_id = baseline["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let (element_id, selected_revision_id) =
        create_character(&config, "project_identity_optional", &reference_asset_id);

    let baseline_character = call(
        &config,
        "creative_element",
        json!({"action":"get","project_id":"project_identity_optional","element_id":element_id}),
    );
    assert_eq!(
        baseline_character["element"]["selected_revision_id"],
        selected_revision_id
    );

    let prepared = prepare_identity(
        &config,
        "project_identity_optional",
        &element_id,
        &reference_asset_id,
        "artifact-v1",
    );
    assert_eq!(prepared["job"]["status"], "completed");
    assert_eq!(
        prepared["job"]["execution_binding_version"],
        "identity-test-v1"
    );
    let artifact_id = prepared["job"]["output_asset_ids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let artifact = call(
        &config,
        "creative_asset",
        json!({"action":"get","project_id":"project_identity_optional","asset_id":artifact_id}),
    );
    assert_eq!(artifact["asset"]["role"], "identity_binding_artifact");
    assert_eq!(artifact["asset"]["element_id"], element_id);
    assert_eq!(artifact["asset"]["parent_asset_id"], reference_asset_id);
    assert_eq!(artifact["asset"]["metadata"]["artifact_kind"], "embedding");
    assert_eq!(
        artifact["asset"]["metadata"]["artifact_version"],
        "artifact-v1"
    );
    assert!(artifact["asset"]["metadata"]["license_notes"]
        .as_str()
        .unwrap()
        .contains("Character Pack remains authoritative"));

    call(
        &config,
        "creative_asset",
        json!({"action":"reject","project_id":"project_identity_optional","asset_id":artifact_id}),
    );
    config
        .creative_binding_descriptors
        .retain(|raw| !raw.contains("test_identity_prepare"));
    config
        .creative_binding_descriptors
        .push(identity_binding_descriptor("identity-test-v2"));
    let replacement = prepare_identity(
        &config,
        "project_identity_optional",
        &element_id,
        &reference_asset_id,
        "artifact-v2",
    );
    assert_eq!(
        replacement["job"]["execution_binding_version"],
        "identity-test-v2"
    );

    let character_after_replacement = call(
        &config,
        "creative_element",
        json!({"action":"get","project_id":"project_identity_optional","element_id":element_id}),
    );
    assert_eq!(
        character_after_replacement["element"]["selected_revision_id"],
        selected_revision_id
    );
    let selected = character_after_replacement["element"]["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| revision["revision_id"] == selected_revision_id)
        .unwrap();
    assert_eq!(selected["reference_asset_ids"], json!([reference_asset_id]));
    assert_eq!(
        selected["spec"]["identity_contract"],
        "reference_pack_authority"
    );
}

#[test]
fn real_person_identity_preparation_requires_explicit_authorization_attestation() {
    let (_workspace, mut config) = configured_workspace();
    config
        .creative_binding_descriptors
        .push(identity_binding_descriptor("identity-test-v1"));
    create_project(&config, "project_identity_auth");

    let denied = dispatch_sync(
        &config,
        "creative_job",
        &json!({
            "action":"cost_estimate",
            "project_id":"project_identity_auth",
            "capability_id":"identity.prepare",
            "execution_binding_id":"test_identity_prepare",
            "parameters":{
                "element_id":"element_example",
                "reference_asset_ids":["asset_example"],
                "subject_kind":"real_person",
                "artifact_kind":"embedding",
                "artifact_version":"v1"
            }
        }),
    );
    assert!(denied.is_err());
}
