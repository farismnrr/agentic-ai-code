use relay_application::resources::{list, read, RESOURCE_NAMES};
use relay_core::config::ServerConfig;
use std::path::PathBuf;

fn repository_config() -> ServerConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let root = root.canonicalize().unwrap();
    ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    }
}

#[test]
fn lists_the_server_owned_resources() {
    let resources = list(&repository_config()).unwrap();
    let names = resources
        .iter()
        .map(|resource| resource.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, RESOURCE_NAMES);
}

#[test]
fn reads_the_server_owned_manifest() {
    let content = read(&repository_config(), "workspace://ai-code/manifest").unwrap();
    assert!(content.text.contains("\"repository\":\"ai-code\""));
    assert_eq!(content.uri, "workspace://ai-code/manifest");
}
