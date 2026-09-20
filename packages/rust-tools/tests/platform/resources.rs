use ai_tools::application::{
    creative::{dispatch_tool, CreativeTrack},
    resources::{list, read, RESOURCE_NAMES},
};
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::{
    retained_tool_catalog, tool_for_wire, DiscoverResult, SERVER_INSTRUCTIONS,
    TOOL_DESCRIPTION_REPORTING_SUFFIX,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

// Git hooks export these repository-local variables to child processes. The
// fixture deliberately creates an independent repository, so preserve the same
// isolation that Git documents for a command invoked outside the current repo.
const GIT_LOCAL_ENV_VARS: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_PARAMETERS",
    "GIT_DIR",
    "GIT_GRAFT_FILE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_INTERNAL_SUPER_PREFIX",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_REPLACE_REF_BASE",
    "GIT_SHALLOW_FILE",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
];

struct TempRepo(PathBuf);

impl TempRepo {
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(format!(
                "resources-test-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ))
            .join("ai-code");
        fs::create_dir_all(&root).unwrap();
        let mut command = Command::new("git");
        command.args(["init", "--quiet"]).current_dir(&root);
        for variable in GIT_LOCAL_ENV_VARS {
            command.env_remove(variable);
        }
        let status = command.status().unwrap();
        assert!(status.success(), "resource fixture git init must succeed");
        Self(root)
    }

    fn config(&self) -> ServerConfig {
        ServerConfig {
            dir: Some(self.0.to_string_lossy().into_owned()),
            execution_root: Some(self.0.to_string_lossy().into_owned()),
            ..ServerConfig::default()
        }
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        if let Some(parent) = self.0.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

fn creative_call(config: &ServerConfig, name: &str, arguments: Value) -> Value {
    let result = tokio::runtime::Runtime::new()
        .expect("creative resource test runtime")
        .block_on(dispatch_tool(name, &arguments, config, "local"))
        .expect("creative resource dispatch")
        .expect("creative resource tool result");
    assert!(
        !result.is_error,
        "creative tool error: {:?}",
        result.content
    );
    serde_json::from_str(&result.content[0].text).expect("creative JSON result")
}

fn create_creative_project(config: &ServerConfig, project_id: &str) {
    creative_call(
        config,
        "creative_project",
        json!({
            "action": "create",
            "project_id": project_id,
            "title": "Resource fixture",
            "intent": "Verify native contained media resources",
            "tracks": [CreativeTrack::Scene]
        }),
    );
}

#[test]
fn lists_the_server_owned_resources() {
    let repository = TempRepo::new();
    let resources = list(&repository.config()).unwrap();
    let names = resources
        .iter()
        .map(|resource| resource.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, RESOURCE_NAMES);
}

#[test]
fn discover_advertises_the_portable_bootstrap_contract() {
    let discover = DiscoverResult::current();
    assert_eq!(discover.instructions, SERVER_INSTRUCTIONS);
    assert!(discover.instructions.contains("Task Execution Report"));
    assert!(discover.instructions.contains("agent-guidance"));
    assert!(discover.instructions.contains("Restart / Operator Action"));
}

#[test]
fn agent_guidance_includes_global_bootstrap_before_repository_guidance() {
    let repository = TempRepo::new();
    fs::create_dir_all(repository.0.join("ai-self")).unwrap();
    fs::create_dir_all(repository.0.join(".agents/knowledge")).unwrap();
    fs::write(
        repository.0.join("ai-self/BOOTSTRAP.md"),
        "GLOBAL BOOTSTRAP\n### Task Execution Report\n",
    )
    .unwrap();
    fs::write(
        repository.0.join("AGENTS.md"),
        "REPOSITORY AGENT GUIDANCE\n",
    )
    .unwrap();
    fs::write(
        repository.0.join(".agents/knowledge/resources.md"),
        "RESOURCE INDEX GUIDANCE\n",
    )
    .unwrap();

    let content = read(&repository.config(), "workspace://ai-code/agent-guidance").unwrap();
    let text = content.text.as_deref().expect("text resource content");
    let bootstrap = text.find("GLOBAL BOOTSTRAP").unwrap();
    let agents = text.find("REPOSITORY AGENT GUIDANCE").unwrap();
    let resources = text.find("RESOURCE INDEX GUIDANCE").unwrap();
    assert!(bootstrap < agents);
    assert!(agents < resources);
}

#[test]
fn tool_wire_description_appends_reporting_contract_without_mutating_catalog() {
    let tool = retained_tool_catalog()
        .into_iter()
        .find(|tool| tool.name == "terminal_exec")
        .expect("terminal_exec retained tool");
    let original = tool.description;
    assert!(!original.contains(TOOL_DESCRIPTION_REPORTING_SUFFIX));

    let wire = tool_for_wire(&tool);
    let description = wire["description"].as_str().unwrap();
    assert!(description.starts_with(original));
    assert!(description.ends_with(TOOL_DESCRIPTION_REPORTING_SUFFIX));
    assert_eq!(
        description
            .matches(TOOL_DESCRIPTION_REPORTING_SUFFIX)
            .count(),
        1
    );
    assert_eq!(tool.description, original);
}

#[test]
fn reads_the_server_owned_manifest() {
    let repository = TempRepo::new();
    let content = read(&repository.config(), "workspace://ai-code/manifest").unwrap();
    assert!(content
        .text
        .as_deref()
        .expect("text resource content")
        .contains("\"repository\":\"ai-code\""));
    assert!(content.blob.is_none());
    assert_eq!(content.uri, "workspace://ai-code/manifest");
}

#[test]
fn blender_resource_is_exposed_only_with_the_creative_master_flag() {
    let repository = TempRepo::new();
    let disabled = repository.config();
    assert!(read(&disabled, "workspace://ai-code/blender-capability").is_err());

    let enabled = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    let resources = list(&enabled).unwrap();
    assert!(resources
        .iter()
        .any(|resource| resource.name == "blender-capability"));

    let content = read(&enabled, "workspace://ai-code/blender-capability").unwrap();
    let value: serde_json::Value =
        serde_json::from_str(content.text.as_deref().expect("text resource content")).unwrap();
    assert_eq!(value["capability"], "blender");
    let tools = value["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5);
    assert_eq!(
        tools,
        &[
            "blender_session",
            "blender_inspect",
            "blender_python_api_docs",
            "blender_screenshot",
            "blender_animation_preview",
        ]
        .map(serde_json::Value::from)
    );
    assert_eq!(value["routing"]["default"], "bounded_mcp_control_plane");
    assert_eq!(
        value["routing"]["heavy_execution"],
        "foreground_operator_cli"
    );
    assert_eq!(
        value["routing"]["raw_python"],
        "foreground_operator_cli_only"
    );
    assert_eq!(value["authority"]["caller_executable_override"], false);
    assert_eq!(
        value["authority"]["production_artifacts_project_contained"],
        true
    );

    let manifest = read(&enabled, "workspace://ai-code/manifest").unwrap();
    let manifest_text = manifest.text.as_deref().expect("text resource content");
    assert!(manifest_text.contains("blender-capability"));
    assert!(manifest_text.contains("\"blender\""));
}

#[test]
fn reads_a_registered_contained_png_as_blob_resource() {
    let repository = TempRepo::new();
    let config = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    create_creative_project(&config, "project_media_resource");

    let asset_path = repository.0.join("assets").join("review.png");
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]))
        .save(&asset_path)
        .unwrap();
    let expected = fs::read(&asset_path).unwrap();

    let registered = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_media_resource",
            "path": "assets/review.png",
            "media_type": "image/png",
            "role": "review"
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap();
    let asset = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "get",
            "project_id": "project_media_resource",
            "asset_id": asset_id
        }),
    );
    let resource_uri = asset["resource_uri"].as_str().unwrap();

    let content = read(&config, resource_uri).unwrap();
    assert_eq!(content.mime_type, "image/png");
    assert!(content.text.is_none());
    assert_eq!(
        STANDARD.decode(content.blob.as_deref().unwrap()).unwrap(),
        expected
    );
}

#[test]
fn rejects_creative_image_resource_with_oversized_dimensions() {
    let repository = TempRepo::new();
    let config = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    create_creative_project(&config, "project_large_image_resource");

    let asset_path = repository.0.join("assets").join("wide.png");
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(4097, 1, image::Rgba([20, 40, 60, 255]))
        .save(&asset_path)
        .unwrap();

    let registered = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_large_image_resource",
            "path": "assets/wide.png",
            "media_type": "image/png",
            "role": "review"
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap();
    let asset = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "get",
            "project_id": "project_large_image_resource",
            "asset_id": asset_id
        }),
    );
    assert!(read(&config, asset["resource_uri"].as_str().unwrap()).is_err());
}

#[test]
fn rejects_creative_resource_when_registered_bytes_change() {
    let repository = TempRepo::new();
    let config = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    create_creative_project(&config, "project_changed_resource");

    let asset_path = repository.0.join("assets").join("changed.png");
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]))
        .save(&asset_path)
        .unwrap();

    let registered = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_changed_resource",
            "path": "assets/changed.png",
            "media_type": "image/png",
            "role": "review"
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap();
    let asset = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "get",
            "project_id": "project_changed_resource",
            "asset_id": asset_id
        }),
    );
    image::RgbaImage::from_pixel(2, 2, image::Rgba([99, 88, 77, 255]))
        .save(&asset_path)
        .unwrap();
    assert!(read(&config, asset["resource_uri"].as_str().unwrap()).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_creative_resource_after_symlink_substitution() {
    use std::os::unix::fs::symlink;

    let repository = TempRepo::new();
    let config = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    create_creative_project(&config, "project_symlink_resource");

    let asset_path = repository.0.join("assets").join("linked.png");
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]))
        .save(&asset_path)
        .unwrap();

    let registered = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_symlink_resource",
            "path": "assets/linked.png",
            "media_type": "image/png",
            "role": "review"
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap();
    let asset = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "get",
            "project_id": "project_symlink_resource",
            "asset_id": asset_id
        }),
    );

    let outside = repository.0.parent().unwrap().join("outside.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]))
        .save(&outside)
        .unwrap();
    fs::remove_file(&asset_path).unwrap();
    symlink(&outside, &asset_path).unwrap();

    assert!(read(&config, asset["resource_uri"].as_str().unwrap()).is_err());
}

#[test]
fn rejects_non_image_creative_resource_content() {
    let repository = TempRepo::new();
    let config = ServerConfig {
        enable_creative: true,
        ..repository.config()
    };
    create_creative_project(&config, "project_text_resource");

    fs::create_dir_all(repository.0.join("assets")).unwrap();
    fs::write(
        repository.0.join("assets").join("notes.txt"),
        b"not an image",
    )
    .unwrap();

    let registered = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "register",
            "project_id": "project_text_resource",
            "path": "assets/notes.txt",
            "media_type": "application/json",
            "role": "notes"
        }),
    );
    let asset_id = registered["asset_id"].as_str().unwrap();
    let asset = creative_call(
        &config,
        "creative_asset",
        json!({
            "action": "get",
            "project_id": "project_text_resource",
            "asset_id": asset_id
        }),
    );
    assert!(read(&config, asset["resource_uri"].as_str().unwrap()).is_err());
}
