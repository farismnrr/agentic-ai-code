#![cfg(target_os = "linux")]

use ai_tools::application::execution::{start_terminal_job, JobManager, JobState};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::{fs, process::Command};

#[tokio::test]
async fn selected_git_repository_narrows_terminal_mount_inside_projects_root() {
    let root =
        std::env::temp_dir().join(format!("terminal-project-scope-{}", uuid::Uuid::new_v4()));
    let repository = root.join("MasihAwam/ai-code");
    let sibling_secret = root.join("Sensio/.env");
    fs::create_dir_all(&repository).expect("create selected repository");
    fs::create_dir_all(sibling_secret.parent().expect("sibling parent"))
        .expect("create sibling repository");
    fs::write(&sibling_secret, "sibling-secret-canary").expect("write sibling canary");

    let initialized = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repository)
        .status()
        .expect("run git init");
    assert!(initialized.success(), "git init must succeed");

    let config = ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        default_terminal_timeout_ms: 5_000,
        ..ServerConfig::default()
    };
    config
        .ensure_workspaces_initialized()
        .expect("initialize broad authorized Projects root");
    let manager = JobManager::new(config.clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let snapshot = loop {
        let id = start_terminal_job(
            &json!({
                "command": "sh",
                "args": ["-c", "test ! -e ../Sensio/.env && printf selected-repo-only"],
                "cwd": repository,
                "timeout_ms": 5000
            }),
            &config,
            &manager,
        )
        .await
        .expect("start terminal job in selected child repository");
        let snapshot = manager.wait(&id).await.expect("wait for terminal job");
        if snapshot.state == JobState::Completed {
            break snapshot;
        }
        let diagnostic = snapshot
            .result
            .as_ref()
            .and_then(|result| result.content.first())
            .map(|content| content.text.as_str())
            .unwrap_or_default();
        assert!(
            matches!(snapshot.state, JobState::Failed | JobState::Cancelled)
                && diagnostic.contains("protected_path_discovery"),
            "unexpected selected-repository warmup result: {diagnostic}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "selected repository protected-path index did not become ready"
        );
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    };
    manager.shutdown().await;

    assert_eq!(snapshot.state, JobState::Completed, "{:?}", snapshot.result);
    assert_eq!(snapshot.stdout, "selected-repo-only");
    assert!(!snapshot.stdout.contains("sibling-secret-canary"));
    drop(manager);
    fs::remove_dir_all(root).expect("remove terminal scope fixture");
}
