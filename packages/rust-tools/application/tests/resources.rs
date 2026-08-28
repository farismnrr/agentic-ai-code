use relay_application::resources::{list, read, RESOURCE_NAMES};
use relay_core::config::ServerConfig;
use std::{fs, path::PathBuf, process::Command};

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
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .status()
            .unwrap();
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
