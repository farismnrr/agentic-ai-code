use relay_application::git::dispatch_git_tool;
use relay_core::config::ServerConfig;
use serde_json::json;
use std::{fs, path::Path, process::Command};

fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success());
}

#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044a-issues-{}", std::process::id()));
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    run(&repo, &["init", "-q", "-b", "main"]);
    run(&repo, &["config", "user.email", "fixture@example.test"]);
    run(&repo, &["config", "user.name", "fixture"]);
    fs::write(repo.join("README.md"), "test\n").unwrap();
    run(&repo, &["add", "README.md"]);
    run(&repo, &["commit", "-qm", "init"]);
    run(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git",
        ],
    );

    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };

    // 1. Invalid state for issue_list
    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "state": "evil"}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue state is invalid"),
        "expected invalid state error, got {err:?}"
    );

    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "state": "merged"}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue state is invalid"),
        "expected invalid state error for merged, got {err:?}"
    );

    // 2. Invalid labels (> 10 labels)
    let too_many_labels: Vec<String> = (0..11).map(|i| format!("label-{i}")).collect();
    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": too_many_labels}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue labels exceed maximum"),
        "expected labels exceed max error, got {err:?}"
    );

    // 3. Invalid label content (empty string, newline, NUL, oversized)
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo, "labels": [""]}), &config)
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue label is invalid"),
        "expected invalid label error for empty string, got {err:?}"
    );

    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": ["has\nnewline"]}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue label is invalid"),
        "expected invalid label error for newline, got {err:?}"
    );

    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": ["has\0nul"]}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue label is invalid"),
        "expected invalid label error for NUL byte, got {err:?}"
    );

    let oversized_label = "a".repeat(129);
    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": [oversized_label]}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue label is invalid"),
        "expected invalid label error for oversized label, got {err:?}"
    );

    // 4. Missing / invalid issue number for issue_get
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue number is required"),
        "expected missing number error, got {err:?}"
    );

    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 0}), &config)
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("issue number is required"),
        "expected number 0 error, got {err:?}"
    );

    // 5. Offline forge invocation fails closed
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("forge operation failed")
            || format!("{err:?}").contains("invalid_git_output"),
        "expected forge operation failure, got {err:?}"
    );

    let _ = fs::remove_dir_all(base);
    println!("plan044a issue reads acceptance: PASS");
}
