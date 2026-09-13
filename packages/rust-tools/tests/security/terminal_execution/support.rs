use ai_tools::application::execution::JobManager;
use ai_tools::core::config::ServerConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) struct TestFixture {
    pub(super) root: PathBuf,
    pub(super) config: ServerConfig,
    pub(super) manager: Arc<JobManager>,
}

impl TestFixture {
    pub(super) fn new() -> Self {
        Self::with_config(|_| {})
    }

    pub(super) fn with_config<F: FnOnce(&mut ServerConfig)>(customize: F) -> Self {
        let root =
            std::env::temp_dir().join(format!("terminal-exec-test-{}", uuid::Uuid::new_v4()));
        Self::at_root(root, customize)
    }

    pub(super) fn on_project_filesystem() -> Self {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".terminal-exec-test-{}", uuid::Uuid::new_v4()));
        Self::at_root(root, |_| {})
    }

    fn at_root<F: FnOnce(&mut ServerConfig)>(root: PathBuf, customize: F) -> Self {
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
        Self {
            root,
            config,
            manager,
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
