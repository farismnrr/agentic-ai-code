use super::support::TestFixture;
use ai_tools::application::execution::{dispatch_tool_call, start_terminal_job, JobState};
use ai_tools::application::hooks::HookManager;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::interfaces::mcp::retained_tool_catalog;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn large_workspace_reindexes_new_protected_path_before_sync_deadline() {
    let fixture = TestFixture::on_project_filesystem();
    let large_directory = fixture.root.join("large");
    fs::create_dir_all(&large_directory).expect("create large workspace subtree");
    for index in 0..12_000 {
        fs::create_dir(large_directory.join(format!("entry-{index:05}")))
            .expect("create nested workspace directory");
    }

    fixture.manager.prepare_for_serving().await;
    let warm_id = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start cache-warming command");
    assert_eq!(
        fixture
            .manager
            .wait(&warm_id)
            .await
            .expect("warm wait")
            .state,
        JobState::Completed
    );

    let protected_path = large_directory.join("entry-11999/.env.local");
    fs::write(&protected_path, "PRIVATE_SENTINEL")
        .expect("create nested protected path after index warmup");
    let refresh_started = std::time::Instant::now();
    let read_id = start_terminal_job(
        &json!({
            "command": "cat large/entry-11999/.env.local",
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start protected-path check");
    let snapshot = fixture
        .manager
        .wait(&read_id)
        .await
        .expect("protected-path wait");
    assert_eq!(snapshot.state, JobState::Completed);
    assert!(!snapshot.stdout.contains("PRIVATE_SENTINEL"));
    assert!(
        refresh_started.elapsed() < Duration::from_secs(8),
        "protected-path index refresh exceeded the sync execution budget: {:?}",
        refresh_started.elapsed()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn sandbox_failure_returns_sanitized_stage_diagnostic() {
    use std::os::unix::fs::symlink;

    let fixture = TestFixture::new();
    symlink("/etc/shadow", fixture.root.join(".env.local"))
        .expect("create protected symbolic link fixture");
    let id = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start diagnostic job");
    let snapshot = fixture.manager.wait(&id).await.expect("diagnostic wait");
    assert_eq!(snapshot.state, JobState::Failed);
    let result = snapshot.result.as_ref().expect("safe execution diagnostic");
    assert!(result.is_error);
    let message = &result.content[0].text;
    assert!(message.contains("terminal execution failed at protected_path_discovery: Other"));
    assert!(!message.contains(&fixture.root.to_string_lossy().to_string()));
    assert!(!message.contains("/etc/shadow"));
    assert!(snapshot
        .task_json(0)
        .to_string()
        .contains("protected_path_discovery"));
}

#[tokio::test]
async fn dropping_sync_request_cancels_job_and_releases_semaphore() {
    let fixture = TestFixture::with_config(|config| config.max_running_jobs = 1);
    let config = fixture.config.clone();
    let manager = fixture.manager.clone();
    let lsp = Arc::new(LspSessionManager::new(config.clone()).expect("LSP manager"));
    let hooks = Arc::new(HookManager::load(Arc::new(config.clone())).expect("hook manager"));
    let request = tokio::spawn(async move {
        let catalog = retained_tool_catalog();
        let tool = catalog
            .iter()
            .find(|tool| tool.name == "terminal_exec")
            .expect("terminal_exec tool");
        dispatch_tool_call(
            tool,
            &json!({
                "command": "sleep 30",
                "cwd": config.dir.clone(),
                "timeout_ms": 30_000
            }),
            &config,
            &manager,
            &lsp,
            &hooks,
        )
        .await
    });

    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.manager.active_jobs_count().await == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("sync request did not start its job");
    request.abort();
    let _ = request.await;

    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.manager.active_jobs_count().await != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropped request left its job active");

    let next_id = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("job could not be started after request cancellation");
    assert_eq!(
        fixture
            .manager
            .wait(&next_id)
            .await
            .expect("next job wait")
            .state,
        JobState::Completed
    );
}
