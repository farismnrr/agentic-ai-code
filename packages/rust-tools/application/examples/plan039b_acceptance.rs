use relay_application::{git::dispatch_git_tool, workspace::apply_patch};
use relay_core::config::ServerConfig;
use serde_json::json;
use std::{fs, process::Command};
fn run(dir: &std::path::Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success())
}
#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-039b-{}", std::process::id()));
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    run(&repo, &["init", "-q"]);
    run(&repo, &["config", "user.email", "fixture@example.test"]);
    run(&repo, &["config", "user.name", "fixture"]);
    fs::write(repo.join("sample.txt"), "one\ntwo\nthree\n").unwrap();
    run(&repo, &["add", "sample.txt"]);
    run(&repo, &["commit", "-qm", "init"]);
    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    let status = dispatch_git_tool("git_status", &json!({"cwd":repo}), &config)
        .await
        .unwrap()
        .unwrap();
    assert!(!status.is_error);
    let patch = "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n";
    let dry = apply_patch(&json!({"cwd":repo,"patch":patch,"dry_run":true}), &config).unwrap();
    assert!(dry.dry_run);
    assert_eq!(
        fs::read_to_string(repo.join("sample.txt")).unwrap(),
        "one\ntwo\nthree\n"
    );
    let applied = apply_patch(&json!({"cwd":repo,"patch":patch}), &config).unwrap();
    assert!(!applied.dry_run);
    assert_eq!(
        fs::read_to_string(repo.join("sample.txt")).unwrap(),
        "one\nTWO\nthree\n"
    );
    assert!(apply_patch(&json!({"cwd":repo,"patch":patch}), &config).is_err());
    let traversal = "--- a/../escape\n+++ b/../escape\n@@ -1 +1 @@\n-x\n+y\n";
    assert!(apply_patch(&json!({"cwd":repo,"patch":traversal}), &config).is_err());
    let _ = fs::remove_dir_all(base);
    println!("plan039b native acceptance: PASS");
}
