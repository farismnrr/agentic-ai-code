#[path = "creative/contracts.rs"]
mod contracts;
#[path = "creative/graph.rs"]
mod graph;
#[path = "creative/ingest.rs"]
mod ingest;

use ai_tools::application::creative::{dispatch_tool, CreativeTrack, CREATIVE_SCHEMA_VERSION};
use ai_tools::core::config::{ServerConfig, ToolProfile};
use ai_tools::interfaces::mcp::{
    retained_tool_catalog, runtime_tool_catalog, validate_tool_arguments,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use uuid::Uuid;

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "creative-platform-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("creative test workspace");
        Self(root)
    }

    fn config(&self) -> ServerConfig {
        ServerConfig {
            dir: Some(self.0.to_string_lossy().into_owned()),
            execution_root: Some(self.0.to_string_lossy().into_owned()),
            enable_creative: true,
            ..ServerConfig::default()
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn dispatch_sync(
    config: &ServerConfig,
    name: &str,
    arguments: &Value,
) -> Result<Option<ai_tools::interfaces::mcp::ToolCallResult>, ai_tools::core::error::McpError> {
    tokio::runtime::Runtime::new()
        .expect("creative test runtime")
        .block_on(dispatch_tool(name, arguments, config, "local"))
}

fn call(config: &ServerConfig, name: &str, arguments: Value) -> Value {
    let result = dispatch_sync(config, name, &arguments)
        .expect("creative dispatch")
        .expect("creative tool result");
    assert!(
        !result.is_error,
        "creative tool returned error: {:?}",
        result.content
    );
    serde_json::from_str(&result.content[0].text).expect("creative JSON result")
}

fn create_project(config: &ServerConfig, project_id: &str) {
    let result = call(
        config,
        "creative_project",
        json!({
            "action": "create",
            "project_id": project_id,
            "title": "Creative fixture",
            "intent": "Prove contained reusable production state",
            "tracks": [CreativeTrack::Scene, CreativeTrack::Anime, CreativeTrack::Game]
        }),
    );
    assert_eq!(result["project"]["schema_version"], CREATIVE_SCHEMA_VERSION);
    assert_eq!(result["project"]["project_id"], project_id);
    assert_eq!(
        result["layout"]["state_root"],
        format!(".masihawam/creative/projects/{project_id}")
    );
    assert_eq!(
        result["layout"]["production_root"],
        format!("creative/{project_id}")
    );
    assert_eq!(
        result["layout"]["assets_root"],
        format!("creative/{project_id}/assets")
    );
    assert_eq!(
        result["layout"]["exports_root"],
        format!("creative/{project_id}/exports")
    );
}

#[test]
fn runtime_catalog_composes_creative_tools_without_changing_retained_base() {
    let retained = retained_tool_catalog();
    assert_eq!(retained.len(), 52);
    assert!(!retained
        .iter()
        .any(|tool| tool.name.starts_with("creative_")));

    let disabled = runtime_tool_catalog(ToolProfile::Full, false);
    assert_eq!(disabled.len(), retained.len() + 1);
    assert!(disabled.iter().any(|tool| tool.name == "creative_status"));
    assert!(!disabled.iter().any(|tool| tool.name == "creative_project"));

    let enabled = runtime_tool_catalog(ToolProfile::Full, true);
    for name in [
        "creative_status",
        "creative_catalog",
        "creative_project",
        "creative_element",
        "creative_asset",
        "creative_graph",
        "creative_job",
    ] {
        assert!(
            enabled.iter().any(|tool| tool.name == name),
            "missing {name}"
        );
    }

    let primary = runtime_tool_catalog(ToolProfile::Primary, true);
    assert!(primary.iter().any(|tool| tool.name == "creative_status"));
    assert!(!primary.iter().any(|tool| tool.name == "creative_project"));
}

#[test]
fn creative_action_schemas_require_action_specific_inputs() {
    let tools = runtime_tool_catalog(ToolProfile::Full, true);
    let tool = |name: &str| {
        tools
            .iter()
            .find(|tool| tool.name == name)
            .expect("creative tool")
    };

    for (name, arguments) in [
        ("creative_project", json!({"action": "create"})),
        ("creative_project", json!({"action": "get"})),
        (
            "creative_element",
            json!({"action": "create_revision", "project_id": "project_schema"}),
        ),
        (
            "creative_element",
            json!({
                "action": "create_revision",
                "project_id": "project_schema",
                "authority": "authoritative"
            }),
        ),
        ("creative_graph", json!({"action": "execute"})),
        (
            "creative_job",
            json!({"action": "get", "project_id": "project_schema"}),
        ),
    ] {
        assert!(
            validate_tool_arguments(tool(name), &arguments).is_err(),
            "{name} accepted incomplete action arguments"
        );
    }
}

#[test]
fn contained_asset_and_element_lineage_round_trip_without_forged_provenance() {
    let workspace = TempWorkspace::new();
    let config = workspace.config();
    create_project(&config, "project_lineage");

    fs::create_dir_all(workspace.path("assets")).unwrap();
    fs::write(
        workspace.path("assets/reference.png"),
        b"fixture-image-bytes",
    )
    .unwrap();

    let asset_tool = runtime_tool_catalog(ToolProfile::Full, true)
        .into_iter()
        .find(|tool| tool.name == "creative_asset")
        .expect("creative asset tool");
    assert!(validate_tool_arguments(
        &asset_tool,
        &json!({
            "action": "register",
            "project_id": "project_lineage",
            "path": "assets/reference.png",
            "media_type": "image/png",
            "role": "character_reference",
            "source": "generated_asset"
        })
    )
    .is_err());

    assert!(validate_tool_arguments(
        &asset_tool,
        &json!({
            "action": "register",
            "project_id": "project_lineage",
            "path": "assets/reference.png",
            "media_type": "image/png",
            "role": "character_reference",
            "metadata": {"api_key": "must-not-persist"}
        })
    )
    .is_err());

    let registered = call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_lineage",
            "path": "assets/reference.png",
            "media_type": "image/png",
            "role": "character_reference",
            "metadata": {"width": 512, "height": 512}
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap().to_owned();
    assert_eq!(registered["source"], "manual_import");
    assert_eq!(registered["source_surface"], "manual_import");
    assert_eq!(registered["state"], "candidate");

    let revision = call(
        &config,
        "creative_element",
        json!({
            "action": "create_revision",
            "project_id": "project_lineage",
            "kind": "character",
            "name": "Hero",
            "authority": "authoritative",
            "reference_asset_ids": [asset_id],
            "spec": {"silhouette": "locked", "palette": "approved"}
        }),
    );
    let element_id = revision["element_id"].as_str().unwrap().to_owned();
    let revision_id = revision["revision_id"].as_str().unwrap().to_owned();

    call(
        &config,
        "creative_element",
        json!({
            "action": "promote",
            "project_id": "project_lineage",
            "element_id": element_id,
            "revision_id": revision_id
        }),
    );
    call(
        &config,
        "creative_asset",
        json!({
            "action": "promote",
            "project_id": "project_lineage",
            "asset_id": asset_id
        }),
    );

    let project = call(
        &config,
        "creative_project",
        json!({"action": "get", "project_id": "project_lineage"}),
    );
    assert_eq!(
        project["project"]["elements"][0]["selected_revision_id"],
        revision_id
    );
    assert_eq!(project["project"]["assets"][0]["state"], "accepted");
    assert_eq!(project["project"]["assets"][0]["source"], "manual_import");
    assert_eq!(
        project["project"]["assets"][0]["source_surface"],
        "manual_import"
    );
    let accepted = call(
        &config,
        "creative_asset",
        json!({
            "action": "search",
            "project_id": "project_lineage",
            "source": "manual_import",
            "source_surface": "manual_import",
            "state": "accepted",
            "limit": 10
        }),
    );
    assert_eq!(accepted["assets"].as_array().map(Vec::len), Some(1));
    assert_eq!(accepted["assets"][0]["asset_id"], asset_id);
    assert_eq!(
        project["project"]["assets"][0]["relative_path"],
        "assets/reference.png"
    );
    assert_eq!(
        project["project"]["assets"][0]["checksum_sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );

    let state_registration = dispatch_sync(
        &config,
        "creative_asset",
        &json!({
            "action": "register",
            "project_id": "project_lineage",
            "path": ".masihawam/creative/projects/project_lineage/project.json",
            "media_type": "application/json",
            "role": "forbidden_state"
        }),
    );
    assert!(state_registration.is_err());
}
