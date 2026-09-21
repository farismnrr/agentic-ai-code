use super::support::TestFixture;
use ai_tools::application::execution::{
    dispatch_tool_call, start_terminal_job, start_terminal_job_for, JobState,
};
use ai_tools::application::hooks::HookManager;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::interfaces::mcp::retained_tool_catalog;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

// 1. terminal_exec("true") completes
#[tokio::test]
async fn test_terminal_exec_true_completes() {
    let fixture = TestFixture::new().await;
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
    .expect("failed to start true job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
}

// 2. terminal_exec("printf", ["ok"]) returns captured stdout
#[tokio::test]
async fn test_terminal_exec_printf_captures_stdout() {
    let fixture = TestFixture::new().await;
    let id = start_terminal_job(
        &json!({
            "command": "printf",
            "args": ["ok"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start printf job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
    assert_eq!(snapshot.stdout, "ok");
    assert!(snapshot.stderr.is_empty());
}

// 3. non-zero commands return without hanging
#[tokio::test]
async fn test_terminal_exec_nonzero_exit_without_hanging() {
    let fixture = TestFixture::new().await;
    let start = Instant::now();
    let id = start_terminal_job(
        &json!({
            "command": "false",
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start false job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(snapshot.state, JobState::Completed);
    assert_ne!(snapshot.exit_code, Some(0));
}

// 4. stderr is captured without hanging
#[tokio::test]
async fn test_terminal_exec_captures_stderr() {
    let fixture = TestFixture::new().await;
    let id = start_terminal_job(
        &json!({
            "command": "sh",
            "args": ["-c", "printf 'error output' >&2"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start stderr job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
    assert_eq!(snapshot.stderr, "error output");
}

// 5. timeout produces TimedOut
#[tokio::test]
async fn test_terminal_exec_timeout_produces_timed_out() {
    let fixture = TestFixture::new().await;
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
    .expect("failed to warm the selected workspace index");
    assert_eq!(
        fixture
            .manager
            .wait(&warm_id)
            .await
            .expect("wait for warm command")
            .state,
        JobState::Completed
    );

    let start = Instant::now();
    let id = start_terminal_job(
        &json!({
            "command": "sleep",
            "args": ["10"],
            "cwd": fixture.root,
            "timeout_ms": 200
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start sleep job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    let elapsed = start.elapsed();
    assert_eq!(
        snapshot.state,
        JobState::TimedOut,
        "stdout={:?} stderr={:?} result={:?} elapsed={elapsed:?}",
        snapshot.stdout,
        snapshot.stderr,
        snapshot.result
    );
    let result = snapshot.result.as_ref().expect("timed-out tool result");
    assert!(result.is_error);
    let timeout_text = &result.content[0].text;
    assert!(
        timeout_text == "terminal execution failed at child_wait: TimedOut"
            || timeout_text == "terminal execution failed at job_wait: TimedOut",
        "unexpected timeout stage: {timeout_text}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "timeout took too long: {elapsed:?}"
    );
}

// 6. process descendants cannot keep a completed request alive indefinitely
#[tokio::test]
async fn test_terminal_exec_descendants_do_not_hang() {
    let fixture = TestFixture::new().await;
    let start = Instant::now();
    // Primary shell process exits immediately, but background descendant inherits stdout pipe
    let id = start_terminal_job(
        &json!({
            "command": "sh",
            "args": ["-c", "(sleep 30 >&1) & exit 0"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("failed to start background child job");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    let elapsed = start.elapsed();
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
    // Must finish within drain grace period (< 3s), definitely not 30s!
    assert!(
        elapsed < Duration::from_secs(3),
        "descendant held execution for {elapsed:?}"
    );
}

// 8. semaphore capacity is restored after timeout/cancellation
#[tokio::test]
async fn test_terminal_exec_semaphore_capacity_restored() {
    // Only 1 running job allowed
    let fixture = TestFixture::with_config(|c| {
        c.max_running_jobs = 1;
    })
    .await;

    // 1st job: times out quickly
    let id1 = start_terminal_job(
        &json!({
            "command": "sleep",
            "args": ["10"],
            "cwd": fixture.root,
            "timeout_ms": 100
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("job1 start failed");

    let snap1 = fixture.manager.wait(&id1).await.expect("job1 wait failed");
    assert_eq!(snap1.state, JobState::TimedOut);

    // 2nd job: a normal synchronous call should acquire the released semaphore
    let id2 = start_terminal_job(
        &json!({
            "command": "printf",
            "args": ["second-complete"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("job2 start failed");

    let snap2 = fixture.manager.wait(&id2).await.expect("job2 wait failed");
    assert_eq!(snap2.state, JobState::Completed);
    assert_eq!(snap2.stdout, "second-complete");

    // 3rd job: should also acquire capacity immediately and complete
    let id3 = start_terminal_job(
        &json!({
            "command": "printf",
            "args": ["capacity-restored"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("job3 start failed");

    let snap3 = fixture.manager.wait(&id3).await.expect("job3 wait failed");
    assert_eq!(snap3.state, JobState::Completed);
    assert_eq!(snap3.stdout, "capacity-restored");
}

#[tokio::test]
async fn terminal_exec_semaphore_wait_uses_the_same_deadline_and_cannot_spawn_late() {
    let fixture = TestFixture::with_config(|config| config.max_running_jobs = 1).await;
    let manager = fixture.manager.clone();
    let config = fixture.config.clone();
    let first = tokio::spawn(async move {
        start_terminal_job(
            &json!({
                "command": "sleep",
                "args": ["30"],
                "cwd": config.dir.clone(),
                "timeout_ms": 5000
            }),
            &config,
            &manager,
        )
        .await
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        while fixture.manager.active_jobs_count().await == 0 {
            if first.is_finished() {
                panic!("first job finished before admission");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("first job did not acquire execution capacity");

    let started = Instant::now();
    let queued_id = start_terminal_job(
        &json!({
            "command": "printf",
            "args": ["must-not-spawn"],
            "cwd": fixture.root,
            "timeout_ms": 100
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("queued job failed to be admitted");
    let queued = fixture
        .manager
        .wait(&queued_id)
        .await
        .expect("queued job wait");
    assert_eq!(queued.state, JobState::TimedOut);
    assert!(
        queued.stdout.is_empty(),
        "queued job spawned after its deadline"
    );
    assert!(started.elapsed() < Duration::from_secs(2));

    first.abort();
    let _ = first.await;
    fixture.manager.shutdown().await;
}

#[tokio::test]
async fn terminal_exec_accepts_60000_ms_and_rejects_larger_public_timeout() {
    let fixture = TestFixture::new().await;
    let accepted_id = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": fixture.root,
            "timeout_ms": 60_000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("public 60000 ms timeout should be accepted");
    assert_eq!(
        fixture
            .manager
            .wait(&accepted_id)
            .await
            .expect("accepted timeout wait")
            .state,
        JobState::Completed
    );

    let rejected = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": fixture.root,
            "timeout_ms": 60_001
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect_err("timeout above the public hard maximum must be rejected");
    assert!(rejected.to_string().contains("terminal maximum"));
}

// 9. repeated terminal executions do not accumulate stuck jobs
#[tokio::test]
async fn test_terminal_exec_repeated_executions_do_not_accumulate() {
    let fixture = TestFixture::new().await;
    for i in 0..10 {
        let id = start_terminal_job(
            &json!({
                "command": "printf",
                "args": [format!("run-{i}")],
                "cwd": fixture.root,
                "timeout_ms": 5000
            }),
            &fixture.config,
            &fixture.manager,
        )
        .await
        .expect("repeated run failed to start");

        let snap = fixture.manager.wait(&id).await.expect("wait failed");
        assert_eq!(snap.state, JobState::Completed);
        assert_eq!(snap.stdout, format!("run-{i}"));
    }

    // Verify all 10 completed cleanly without stuck running jobs
    let running_count = fixture.manager.active_jobs_count().await;
    assert_eq!(running_count, 0, "found accumulated running jobs");
}

// 10. synchronous waiting itself has a bounded failure mode
#[tokio::test]
async fn test_synchronous_wait_has_bounded_watchdog() {
    let fixture = TestFixture::with_config(|c| {
        c.default_terminal_timeout_ms = 100;
        c.max_terminal_timeout_ms = 200;
    })
    .await;

    let start = Instant::now();
    let id = start_terminal_job(
        &json!({
            "command": "sleep",
            "args": ["30"],
            "cwd": fixture.root,
            "timeout_ms": 100
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("start failed");

    // wait should return deterministically bounded
    let snap = fixture.manager.wait(&id).await.expect("wait failed");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "wait did not bound failure mode"
    );
    assert!(matches!(
        snap.state,
        JobState::TimedOut | JobState::Completed | JobState::Cancelled
    ));
}

// 10. owner/session-aware synchronous execution reaches the same final result semantics
#[tokio::test]
async fn test_owner_aware_and_direct_sync_execution_parity() {
    let fixture = TestFixture::new().await;

    // Owner/session-aware synchronous compatibility path
    let task_id = start_terminal_job_for(
        &json!({
            "command": "printf",
            "args": ["parity-check"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
        "test-owner",
        Some("test-session"),
    )
    .await
    .expect("start task failed");

    let owner_snap = fixture
        .manager
        .wait(&task_id)
        .await
        .expect("owner-aware result missing");

    // Synchronous dispatch path
    let catalog = retained_tool_catalog();
    let tool = catalog
        .iter()
        .find(|t| t.name == "terminal_exec")
        .expect("terminal_exec tool missing");
    let lsp = Arc::new(LspSessionManager::new(fixture.config.clone()).unwrap());
    let hooks = Arc::new(HookManager::load(Arc::new(fixture.config.clone())).unwrap());

    let sync_res = dispatch_tool_call(
        tool,
        &json!({
            "command": "printf",
            "args": ["parity-check"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
        &lsp,
        &hooks,
        "local",
    )
    .await
    .expect("sync dispatch failed");

    assert_eq!(owner_snap.exit_code, Some(0));
    assert_eq!(owner_snap.stdout, "parity-check");
    assert!(!sync_res.is_error);
    assert!(sync_res.content[0].text.contains("Exit: 0"));
    assert!(sync_res.content[0].text.contains("Stdout: parity-check"));
}

// 11. stdout/stderr truncation still respects configured limits
#[tokio::test]
async fn test_output_truncation_respects_limits() {
    let fixture = TestFixture::with_config(|c| {
        c.max_retained_output_bytes = 100;
    })
    .await;

    let id = start_terminal_job(
        &json!({
            "command": "python3",
            "args": ["-c", "print('A' * 2000)"],
            "cwd": fixture.root,
            "timeout_ms": 5000
        }),
        &fixture.config,
        &fixture.manager,
    )
    .await
    .expect("start python job failed");

    let snapshot = fixture.manager.wait(&id).await.expect("wait failed");
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
    assert!(snapshot.omitted_bytes > 0);
    assert!(snapshot.stdout.len() <= 100);
}
