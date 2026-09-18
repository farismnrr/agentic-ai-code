use ai_tools::application::execution::{start_terminal_job, JobManager, JobState};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) struct TestFixture {
    pub(super) root: PathBuf,
    pub(super) config: ServerConfig,
    pub(super) manager: Arc<JobManager>,
}

impl TestFixture {
    pub(super) async fn new() -> Self {
        Self::with_config(|_| {}).await
    }

    pub(super) async fn with_config<F: FnOnce(&mut ServerConfig)>(customize: F) -> Self {
        let root =
            std::env::temp_dir().join(format!("terminal-exec-test-{}", uuid::Uuid::new_v4()));
        Self::at_root(root, customize).await
    }

    pub(super) async fn on_project_filesystem() -> Self {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".terminal-exec-test-{}", uuid::Uuid::new_v4()));
        Self::at_root(root, |_| {}).await
    }

    async fn at_root<F: FnOnce(&mut ServerConfig)>(root: PathBuf, customize: F) -> Self {
        fs::create_dir_all(&root).expect("failed to create fixture root");
        let mut config = ServerConfig {
            dir: Some(root.to_string_lossy().into()),
            execution_root: Some(root.to_string_lossy().into()),
            default_terminal_timeout_ms: 10_000,
            ..ServerConfig::default()
        };
        customize(&mut config);
        let _ = config.ensure_workspaces_initialized();
        let manager = JobManager::new(config.clone());
        let fixture = Self {
            root,
            config,
            manager,
        };
        fixture.wait_for_protected_index().await;
        fixture
    }

    async fn wait_for_protected_index(&self) {
        // Protected-path indexing has its own lifecycle budget and must not
        // inherit deliberately tiny terminal deadlines from individual tests.
        let mut warmup_config = self.config.clone();
        warmup_config.default_terminal_timeout_ms =
            warmup_config.default_terminal_timeout_ms.max(5_000);
        warmup_config.max_terminal_timeout_ms = warmup_config.max_terminal_timeout_ms.max(5_000);
        let warmup_manager = JobManager::new(warmup_config.clone());
        warmup_manager.prepare_for_serving().await;

        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let id = start_terminal_job(
                &json!({
                    "command": "true",
                    "cwd": self.root,
                    "timeout_ms": 5000
                }),
                &warmup_config,
                &warmup_manager,
            )
            .await
            .expect("failed to start protected-index warmup command");
            let snapshot = warmup_manager
                .wait(&id)
                .await
                .expect("protected-index warmup wait");
            if snapshot.state == JobState::Completed {
                warmup_manager.shutdown().await;
                return;
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
                "unexpected protected-index warmup failure: {diagnostic}"
            );
            assert!(
                Instant::now() < deadline,
                "protected-path index prewarm did not complete within 30 seconds"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
