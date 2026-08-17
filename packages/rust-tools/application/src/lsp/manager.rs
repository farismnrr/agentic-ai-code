use super::{
    ApprovedServerSpec, LspError, LspSession, WorkspaceIdentity, LSP_IDLE_TIMEOUT,
    LSP_RESTART_WINDOW, MAX_LSP_PROJECTS, MAX_LSP_RESTARTS_PER_WINDOW, MAX_LSP_SESSIONS,
};
use crate::execution::sandbox;
use relay_core::config::ServerConfig;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionKey {
    root: PathBuf,
    language: String,
}

pub struct LspSessionManager {
    config: ServerConfig,
    specs: HashMap<String, ApprovedServerSpec>,
    sessions: Mutex<HashMap<SessionKey, Arc<LspSession>>>,
    restarts: Mutex<HashMap<SessionKey, VecDeque<Instant>>>,
    lifecycle: Mutex<()>,
}

impl LspSessionManager {
    pub fn new(config: ServerConfig) -> Result<Arc<Self>, LspError> {
        let mut specs = HashMap::new();
        for entry in &config.lsp_servers {
            let (language, executable) = entry.split_once('=').ok_or(LspError::Internal)?;
            let path = sandbox::resolve_safe_executable(&config, executable)
                .map_err(|_| LspError::ServerUnavailable)?;
            specs.insert(
                language.to_ascii_lowercase(),
                ApprovedServerSpec {
                    language: language.to_owned(),
                    executable: path,
                    args: Vec::new(),
                },
            );
        }
        let manager = Arc::new(Self {
            config,
            specs,
            sessions: Mutex::new(HashMap::new()),
            restarts: Mutex::new(HashMap::new()),
            lifecycle: Mutex::new(()),
        });
        let weak = Arc::downgrade(&manager);
        tokio::runtime::Handle::try_current()
            .map_err(|_| LspError::Internal)?
            .spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let Some(manager) = weak.upgrade() else {
                        return;
                    };
                    let _guard = manager.lifecycle.lock().await;
                    manager.evict_idle().await;
                }
            });
        Ok(manager)
    }

    pub async fn session_for(
        self: &Arc<Self>,
        cwd: Option<&str>,
        language: &str,
    ) -> Result<Arc<LspSession>, LspError> {
        let _guard = self.lifecycle.lock().await;
        self.evict_idle().await;
        let spec = self
            .specs
            .get(&language.to_ascii_lowercase())
            .cloned()
            .ok_or(LspError::UnsupportedLanguage)?;
        let root = crate::git::resolve_git_workspace(cwd, &self.config)
            .map_err(|_| LspError::ContainedLocationRejected)?;
        let execution_root = self
            .config
            .resolved_execution_root()
            .map_err(|_| LspError::Internal)?;
        crate::workspace::reject_protected_target(&execution_root, &root)
            .map_err(|_| LspError::ContainedLocationRejected)?;
        let key = SessionKey {
            root: root.clone(),
            language: spec.language.to_ascii_lowercase(),
        };
        if let Some(session) = self.sessions.lock().await.get(&key).cloned() {
            if !session.is_faulted() {
                return Ok(session);
            }
            self.sessions.lock().await.remove(&key);
            self.record_restart(&key).await?;
        }
        self.ensure_capacity(&root).await?;
        self.check_restart_budget(&key).await?;
        let session = match LspSession::start(&self.config, &spec, WorkspaceIdentity { root }).await
        {
            Ok(session) => session,
            Err(error) => {
                self.record_restart(&key).await?;
                return Err(
                    if matches!(
                        error,
                        LspError::StartupFailed
                            | LspError::Timeout
                            | LspError::Crashed
                            | LspError::Internal
                    ) {
                        LspError::StartupFailed
                    } else {
                        error
                    },
                );
            }
        };
        self.sessions.lock().await.insert(key, session.clone());
        Ok(session)
    }

    pub async fn shutdown_all(&self) {
        let _guard = self.lifecycle.lock().await;
        let sessions = {
            let mut sessions = self.sessions.lock().await;
            sessions
                .drain()
                .map(|(_, session)| session)
                .collect::<Vec<_>>()
        };
        for session in sessions {
            session.shutdown().await;
        }
    }

    pub async fn active_session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    async fn ensure_capacity(&self, root: &PathBuf) -> Result<(), LspError> {
        let sessions = self.sessions.lock().await;
        if sessions.len() >= MAX_LSP_SESSIONS {
            return Err(LspError::CapacityReached);
        }
        let mut projects = sessions.keys().map(|key| &key.root).collect::<Vec<_>>();
        projects.sort();
        projects.dedup();
        if !projects.contains(&root) && projects.len() >= MAX_LSP_PROJECTS {
            return Err(LspError::CapacityReached);
        }
        Ok(())
    }

    async fn evict_idle(&self) {
        let snapshot = self
            .sessions
            .lock()
            .await
            .iter()
            .map(|(key, session)| (key.clone(), session.clone()))
            .collect::<Vec<_>>();
        let mut remove = Vec::new();
        for (key, session) in snapshot {
            if !session.is_faulted() && session.idle_for().await >= LSP_IDLE_TIMEOUT {
                remove.push((key, session));
            }
        }
        if remove.is_empty() {
            return;
        }
        {
            let mut sessions = self.sessions.lock().await;
            for (key, _) in &remove {
                sessions.remove(key);
            }
        }
        for (_, session) in remove {
            session.shutdown().await;
        }
    }

    async fn check_restart_budget(&self, key: &SessionKey) -> Result<(), LspError> {
        let now = Instant::now();
        let mut restarts = self.restarts.lock().await;
        let history = restarts.entry(key.clone()).or_default();
        while history
            .front()
            .is_some_and(|instant| now.duration_since(*instant) > LSP_RESTART_WINDOW)
        {
            history.pop_front();
        }
        if history.len() >= MAX_LSP_RESTARTS_PER_WINDOW {
            return Err(LspError::Crashed);
        }
        Ok(())
    }

    async fn record_restart(&self, key: &SessionKey) -> Result<(), LspError> {
        self.check_restart_budget(key).await?;
        self.restarts
            .lock()
            .await
            .entry(key.clone())
            .or_default()
            .push_back(Instant::now());
        Ok(())
    }
}
