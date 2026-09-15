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
