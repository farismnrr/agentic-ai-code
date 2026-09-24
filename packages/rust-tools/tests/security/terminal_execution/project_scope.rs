#![cfg(target_os = "linux")]

use ai_tools::application::execution::dispatch_tool_call;
use ai_tools::application::execution::{start_terminal_job, JobManager, JobState};
use ai_tools::application::hooks::HookManager;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::retained_tool_catalog;
use serde_json::json;
use std::{fs, process::Command, sync::Arc, time::Duration};

async fn run_terminal_until_completed(
    config: &ServerConfig,
    manager: &Arc<JobManager>,
    arguments: &serde_json::Value,
) -> ai_tools::application::execution::JobSnapshot {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let id = start_terminal_job(arguments, config, manager)
            .await
            .expect("start terminal job");
        let snapshot = manager.wait(&id).await.expect("wait for terminal job");
        if snapshot.state == JobState::Completed {
            return snapshot;
        }
        let diagnostic = snapshot
            .result
            .as_ref()
            .and_then(|result| result.content.first())
            .map(|content| content.text.as_str())
            .unwrap_or_default();
        assert!(
            diagnostic.contains("protected_path_discovery"),
            "unexpected terminal warmup failure: {diagnostic}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "selected-root protected-path index did not become ready"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

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

#[tokio::test]
async fn disjoint_authorized_root_is_not_visible_to_terminal_exec_by_default() {
    let boundary = std::env::temp_dir().join(format!(
        "terminal-disjoint-authority-{}",
        uuid::Uuid::new_v4()
    ));
    let root_a = boundary.join("RootA");
    let root_b = boundary.join("RootB");
    fs::create_dir_all(&root_a).expect("create Root A");
    fs::create_dir_all(&root_b).expect("create Root B");
    fs::write(root_a.join(".env"), "root-a-secret").expect("write Root A secret");
    fs::write(root_b.join("disjoint-marker.txt"), "root-b-marker").expect("write Root B marker");

    let config = ServerConfig {
        dir: Some(root_a.to_string_lossy().into_owned()),
        execution_root: Some(boundary.to_string_lossy().into_owned()),
        default_terminal_timeout_ms: 5_000,
        ..ServerConfig::default()
    };
    config
        .ensure_workspaces_initialized()
        .expect("initialize Root A authority");
    config
        .workspaces
        .write()
        .expect("workspace authority lock")
        .add(&root_b)
        .expect("authorize disjoint Root B");
    let manager = JobManager::new(config.clone());
    let snapshot = run_terminal_until_completed(
        &config,
        &manager,
        &json!({
            "command": "test",
            "args": ["!", "-e", "../RootB/disjoint-marker.txt"],
            "cwd": root_a,
            "timeout_ms": 5000
        }),
    )
    .await;

    assert_eq!(
        snapshot.stdout, "",
        "stderr={:?} result={:?}",
        snapshot.stderr, snapshot.result
    );
    assert!(!snapshot.stdout.contains("root-b-marker"));
    assert!(!snapshot.stdout.contains("root-a-secret"));
    manager.shutdown().await;
    fs::remove_dir_all(boundary).expect("remove disjoint authority fixture");
}

#[tokio::test]
async fn disjoint_authorized_root_is_not_searched_by_text_search_by_default() {
    let boundary = std::env::temp_dir().join(format!(
        "text-search-disjoint-authority-{}",
        uuid::Uuid::new_v4()
    ));
    let root_a = boundary.join("RootA");
    let root_b = boundary.join("RootB");
    fs::create_dir_all(&root_a).expect("create Root A");
    fs::create_dir_all(&root_b).expect("create Root B");
    fs::write(root_a.join(".env"), "root-a-secret-marker").expect("write Root A secret");
    fs::write(root_a.join("visible.txt"), "root-a-visible-marker").expect("write Root A file");
    fs::write(root_b.join("disjoint.txt"), "root-b-disjoint-marker").expect("write Root B file");

    let config = ServerConfig {
        dir: Some(root_a.to_string_lossy().into_owned()),
        execution_root: Some(boundary.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    config
        .ensure_workspaces_initialized()
        .expect("initialize Root A authority");
    config
        .workspaces
        .write()
        .expect("workspace authority lock")
        .add(&root_b)
        .expect("authorize disjoint Root B");

    let tool = retained_tool_catalog()
        .into_iter()
        .find(|tool| tool.name == "text_search")
        .expect("text_search tool");
    let manager = JobManager::new(config.clone());
    let lsp = Arc::new(LspSessionManager::new(config.clone()).expect("LSP manager"));
    let hooks = Arc::new(HookManager::load(Arc::new(config.clone())).expect("hook manager"));
    let result = dispatch_tool_call(
        &tool,
        &json!({
            "query": "root-b-disjoint-marker",
            "cwd": root_a,
            "max_results": 10
        }),
        &config,
        &manager,
        &lsp,
        &hooks,
        "local",
    )
    .await
    .expect("text search result");
    assert!(!result.is_error);
    let matches = result
        .structured_content
        .as_ref()
        .and_then(|value| value.get("matches"))
        .and_then(serde_json::Value::as_array)
        .expect("text search matches");
    assert!(
        matches.is_empty(),
        "Root B leaked into text search: {matches:?}"
    );

    let protected_result = dispatch_tool_call(
        &tool,
        &json!({
            "query": "root-a-secret-marker",
            "cwd": boundary.join("RootA"),
            "max_results": 10
        }),
        &config,
        &manager,
        &lsp,
        &hooks,
        "local",
    )
    .await
    .expect("protected text search result");
    let protected_matches = protected_result
        .structured_content
        .as_ref()
        .and_then(|value| value.get("matches"))
        .and_then(serde_json::Value::as_array)
        .expect("protected text search matches");
    assert!(protected_matches.is_empty(), "protected path was searched");

    manager.shutdown().await;
    fs::remove_dir_all(boundary).expect("remove text search authority fixture");
}
