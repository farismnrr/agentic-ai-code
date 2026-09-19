use ai_tools::application::resources::{list, read, RESOURCE_NAMES};
use ai_tools::core::config::ServerConfig;
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
fn reads_the_server_owned_manifest() {
    let repository = TempRepo::new();
    let content = read(&repository.config(), "workspace://ai-code/manifest").unwrap();
    assert!(content.text.contains("\"repository\":\"ai-code\""));
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
    let value: serde_json::Value = serde_json::from_str(&content.text).unwrap();
    assert_eq!(value["capability"], "blender");
    let tools = value["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 4);
    assert_eq!(
        tools,
        &[
            "blender_session",
            "blender_inspect",
            "blender_python_api_docs",
            "blender_screenshot",
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
    assert!(manifest.text.contains("blender-capability"));
    assert!(manifest.text.contains("\"blender\""));
}
