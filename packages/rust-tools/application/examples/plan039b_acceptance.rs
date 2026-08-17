use relay_application::{git::dispatch_git_tool, workspace::apply_patch};
use relay_core::config::ServerConfig;
use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};

fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success());
}

async fn git_call(name: &str, arguments: Value, config: &ServerConfig) -> Value {
    let result = dispatch_git_tool(name, &arguments, config)
        .await
        .unwrap_or_else(|error| panic!("{name}: {error:?}"))
        .unwrap();
    assert!(!result.is_error, "{name} unexpectedly failed");
    serde_json::from_str(&result.content[0].text).unwrap()
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
    fs::write(repo.join("space name.txt"), "space\n").unwrap();
    run(&repo, &["add", "sample.txt", "space name.txt"]);
    run(&repo, &["commit", "-qm", "init"]);
    fs::write(repo.join("second.txt"), "second\n").unwrap();
    run(&repo, &["add", "second.txt"]);
    run(&repo, &["commit", "-qm", "second"]);

    let marker = base.join("git-helper-executed");
    let helper = format!("sh -c 'touch {}'", marker.display());
    run(&repo, &["config", "core.fsmonitor", &helper]);
    run(&repo, &["config", "core.pager", &helper]);
    run(&repo, &["config", "diff.evil.command", &helper]);
    run(&repo, &["config", "diff.evil.textconv", &helper]);
    fs::write(repo.join(".gitattributes"), "*.txt diff=evil\n").unwrap();
    fs::write(repo.join("sample.txt"), "one\ntwo changed\nthree\n").unwrap();
    fs::write(repo.join("space name.txt"), "space changed\n").unwrap();

    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    let status = git_call("git_status", json!({"cwd":repo}), &config).await;
    assert!(status["unstaged"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p == "space name.txt"));
    let diff = git_call("git_diff", json!({"cwd":repo,"path":"sample.txt"}), &config).await;
    assert!(diff["text"].as_str().unwrap().contains("two changed"));
    let log = git_call("git_log", json!({"cwd":repo,"max_results":2}), &config).await;
    assert_eq!(log["commits"].as_array().unwrap().len(), 2);
    let head = log["commits"][1]["sha"].as_str().unwrap();
    let shown = git_call(
        "git_show",
        json!({"cwd":repo,"ref":head,"path":"sample.txt"}),
        &config,
    )
    .await;
    assert!(shown["text"].as_str().unwrap().contains("sample.txt"));
    let blame = git_call(
        "git_blame",
        json!({"cwd":repo,"path":"sample.txt","start_line":1,"end_line":2}),
        &config,
    )
    .await;
    assert_eq!(blame["lines"].as_array().unwrap().len(), 2);
    assert!(
        !marker.exists(),
        "repository Git config executed an external helper"
    );

    fs::write(repo.join("sample.txt"), "one\ntwo\nthree\n").unwrap();
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
