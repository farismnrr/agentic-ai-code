use super::support::TestFixture;
use ai_tools::application::execution::{
    dispatch_tool_call, start_terminal_job, JobSnapshot, JobState,
};
use ai_tools::application::hooks::HookManager;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::interfaces::mcp::retained_tool_catalog;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

async fn run_terminal_after_index_warmup(
    fixture: &TestFixture,
    command: &str,
    timeout_ms: u64,
) -> JobSnapshot {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let id = start_terminal_job(
            &json!({
                "command": command,
                "cwd": fixture.root,
                "timeout_ms": timeout_ms
            }),
            &fixture.config,
            &fixture.manager,
        )
        .await
        .expect("failed to start protected-index test command");
        let snapshot = fixture
            .manager
            .wait(&id)
            .await
            .expect("protected-index test command wait");
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
            matches!(snapshot.state, JobState::Failed | JobState::Cancelled),
            "unexpected protected-index test state {:?}: {diagnostic}",
            snapshot.state
        );
        assert!(
            diagnostic.contains("protected_path_discovery"),
            "unexpected protected-index test failure: {diagnostic}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "protected-path index did not become ready within the test budget"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn large_workspace_reindexes_new_protected_path_before_sync_deadline() {
    let fixture = TestFixture::on_project_filesystem().await;
    let large_directory = fixture.root.join("large");
    fs::create_dir_all(&large_directory).expect("create large workspace subtree");
    for index in 0..12_000 {
        fs::create_dir(large_directory.join(format!("entry-{index:05}")))
            .expect("create nested workspace directory");
    }

    fixture.manager.prepare_for_serving().await;
    let warm_snapshot = run_terminal_after_index_warmup(&fixture, "true", 5000).await;
    assert_eq!(warm_snapshot.state, JobState::Completed);

    for index in 0..256 {
        fs::write(
            large_directory.join(format!("entry-{index:05}/artifact-{index:05}.rmeta")),
            b"ordinary build output",
        )
        .expect("create ordinary build artifact");
    }
    let churn_started = std::time::Instant::now();
    let churn_snapshot = run_terminal_after_index_warmup(&fixture, "true", 1000).await;
    assert_eq!(churn_snapshot.state, JobState::Completed);
    assert!(
        churn_started.elapsed() < Duration::from_secs(2),
        "ordinary build churn invalidated the warm protected-path index: {:?}",
        churn_started.elapsed()
    );

    let protected_path = large_directory.join("entry-11999/.env.local");
    fs::write(&protected_path, "PRIVATE_SENTINEL")
        .expect("create nested protected path after index warmup");
    let refresh_started = std::time::Instant::now();
    let snapshot =
        run_terminal_after_index_warmup(&fixture, "cat large/entry-11999/.env.local", 5000).await;
    assert_eq!(snapshot.state, JobState::Completed);
    assert!(!snapshot.stdout.contains("PRIVATE_SENTINEL"));
    assert!(
        refresh_started.elapsed() < Duration::from_secs(8),
        "protected-path index refresh exceeded the sync execution budget: {:?}",
        refresh_started.elapsed()
    );
}

#[tokio::test]
async fn incremental_workspace_changes_keep_new_protected_paths_masked() {
    let fixture = TestFixture::new().await;
    let existing = fixture.root.join("existing/deep");
    fs::create_dir_all(&existing).expect("create existing nested directory");
    fs::write(existing.join("public.txt"), "ordinary-marker\n")
        .expect("write ordinary file in existing directory");
    let indexed = run_terminal_after_index_warmup(&fixture, "true", 5000).await;
    assert_eq!(indexed.state, JobState::Completed);

    let exact_secret = existing.join(".env.local");
    fs::write(&exact_secret, "EXACT_PATH_PRIVATE_SENTINEL")
        .expect("create protected file in an existing directory");
    let exact = run_terminal_after_index_warmup(
        &fixture,
        "sh -c 'cat existing/deep/.env.local 2>/dev/null; cat existing/deep/public.txt'",
        5000,
    )
    .await;
    assert_eq!(exact.state, JobState::Completed);
    assert_eq!(exact.stdout, "ordinary-marker\n");
    assert!(!exact.stdout.contains("EXACT_PATH_PRIVATE_SENTINEL"));

    fs::remove_file(&exact_secret).expect("remove protected file from existing directory");
    let removed = run_terminal_after_index_warmup(
        &fixture,
        "sh -c 'test ! -e existing/deep/.env.local && cat existing/deep/public.txt'",
        5000,
    )
    .await;
    assert_eq!(removed.state, JobState::Completed);
    assert_eq!(removed.stdout, "ordinary-marker\n");

    let created_secret = fixture.root.join("generated/cache/private/.env.production");
    fs::create_dir_all(created_secret.parent().expect("new protected file parent"))
        .expect("create new subtree");
    fs::write(&created_secret, "NEW_SUBTREE_PRIVATE_SENTINEL")
        .expect("write protected file in new subtree");
    fs::write(
        fixture.root.join("generated/public.txt"),
        "generated-marker\n",
    )
    .expect("write ordinary file in new subtree");
    let created = run_terminal_after_index_warmup(
        &fixture,
        "sh -c 'cat generated/cache/private/.env.production 2>/dev/null; cat generated/public.txt'",
        5000,
    )
    .await;
    assert_eq!(created.state, JobState::Completed);
    assert_eq!(created.stdout, "generated-marker\n");
    assert!(!created.stdout.contains("NEW_SUBTREE_PRIVATE_SENTINEL"));

    let incoming =
        std::env::temp_dir().join(format!("terminal-index-incoming-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(incoming.join("old"))
        .expect("create incoming directory tree outside workspace");
    fs::write(incoming.join("old/.env.local"), "MOVED_PRIVATE_SENTINEL")
        .expect("write incoming protected file");
    fs::write(incoming.join("public.txt"), "moved-marker\n").expect("write incoming ordinary file");
    let moved = fixture.root.join("moved");
    fs::rename(&incoming, &moved).expect("move directory into workspace");
    let moved_in = run_terminal_after_index_warmup(
        &fixture,
        "sh -c 'cat moved/old/.env.local 2>/dev/null; cat moved/public.txt'",
        5000,
    )
    .await;
    assert_eq!(moved_in.state, JobState::Completed);
    assert_eq!(moved_in.stdout, "moved-marker\n");
    assert!(!moved_in.stdout.contains("MOVED_PRIVATE_SENTINEL"));

    fs::remove_dir_all(&moved).expect("remove moved-in subtree");
    fs::create_dir_all(moved.join("new")).expect("recreate moved path with a new subtree");
    fs::write(
        moved.join("new/.env.production"),
        "REPLACEMENT_PRIVATE_SENTINEL",
    )
    .expect("write replacement protected file");
    fs::write(moved.join("public.txt"), "replacement-marker\n")
        .expect("write replacement ordinary file");
    let replaced = run_terminal_after_index_warmup(
        &fixture,
        "sh -c 'cat moved/old/.env.local moved/new/.env.production 2>/dev/null; cat moved/public.txt'",
        5000,
    )
    .await;
    assert_eq!(replaced.state, JobState::Completed);
    assert_eq!(replaced.stdout, "replacement-marker\n");
    assert!(!replaced.stdout.contains("MOVED_PRIVATE_SENTINEL"));
    assert!(!replaced.stdout.contains("REPLACEMENT_PRIVATE_SENTINEL"));
}

#[cfg(unix)]
#[tokio::test]
async fn sandbox_failure_returns_sanitized_stage_diagnostic() {
    use std::os::unix::fs::symlink;

    let fixture = TestFixture::new().await;
    symlink("/etc/shadow", fixture.root.join(".env.local"))
        .expect("create protected symbolic link fixture");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let snapshot = loop {
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
        let message = snapshot
            .result
            .as_ref()
            .and_then(|result| result.content.first())
            .map(|content| content.text.as_str())
            .unwrap_or_default();
        if message.contains("protected_path_discovery: Other") {
            break snapshot;
        }
        assert!(
            matches!(snapshot.state, JobState::Failed | JobState::Cancelled)
                && message.contains("protected_path_discovery"),
            "unexpected diagnostic job result: {message}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "protected-path index did not refresh for symlink diagnostic"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
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
    let fixture = TestFixture::with_config(|config| config.max_running_jobs = 1).await;
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
            "local",
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
