use super::{bounded_string, HookEvent, HookManager, SessionStartOutcome};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::Notify;

const SESSION_TTL: StdDuration = StdDuration::from_secs(30 * 60);

#[derive(Debug, Clone)]
pub(crate) enum SessionState {
    Pending(Arc<SessionInitialization>),
    Started(Instant),
}

#[derive(Debug)]
pub(crate) struct SessionInitialization {
    result: tokio::sync::Mutex<Option<SessionStartOutcome>>,
    completed: Notify,
}

impl SessionInitialization {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            result: tokio::sync::Mutex::new(None),
            completed: Notify::new(),
        })
    }

    async fn complete(&self, result: SessionStartOutcome) {
        *self.result.lock().await = Some(result);
        self.completed.notify_waiters();
    }

    async fn wait(&self) -> SessionStartOutcome {
        loop {
            let notified = self.completed.notified();
            if let Some(result) = self.result.lock().await.clone() {
                return result;
            }
            notified.await;
        }
    }
}

pub(crate) async fn start(
    manager: &HookManager,
    agent_session: &str,
    repository_identity: &str,
) -> SessionStartOutcome {
    let initialization = {
        let mut sessions = manager.session_started.lock().await;
        let now = Instant::now();
        sessions.retain(|_, state| {
            matches!(state, SessionState::Pending(_))
                || matches!(state, SessionState::Started(seen) if now.duration_since(*seen) <= SESSION_TTL)
        });
        let at_capacity = sessions.len() >= super::MAX_TRACKED_SESSIONS;
        match sessions.get(agent_session) {
            Some(SessionState::Started(_)) => {
                sessions.insert(agent_session.to_owned(), SessionState::Started(now));
                return SessionStartOutcome::AlreadyStarted;
            }
            Some(SessionState::Pending(initialization)) => initialization.clone(),
            None if at_capacity => return SessionStartOutcome::CapacityExhausted,
            None => {
                let initialization = SessionInitialization::new();
                sessions.insert(
                    agent_session.to_owned(),
                    SessionState::Pending(initialization.clone()),
                );
                let manager = manager.clone();
                let agent_session = agent_session.to_owned();
                let repository_identity = repository_identity.to_owned();
                let task_initialization = initialization.clone();
                tokio::spawn(async move {
                    complete(
                        &manager,
                        &agent_session,
                        &repository_identity,
                        task_initialization,
                    )
                    .await;
                });
                initialization
            }
        }
    };
    initialization.wait().await
}

async fn complete(
    manager: &HookManager,
    agent_session: &str,
    repository_identity: &str,
    initialization: Arc<SessionInitialization>,
) {
    let result = match tokio::spawn({
        let manager = manager.clone();
        let agent_session = agent_session.to_owned();
        let repository_identity = repository_identity.to_owned();
        async move {
            manager
                .session_start_invocations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            manager
                .invoke(
                    HookEvent::SessionStart,
                    json!({
                        "hook_event": "session_start",
                        "agentSession": bounded_string(&agent_session, 128),
                        "repository_identity": bounded_string(&repository_identity, 512),
                        "context": { "repository_identity": bounded_string(&repository_identity, 512) },
                    }),
                )
                .await
        }
    })
    .await
    {
        Ok(result) if result.decision == super::HookDecision::Continue => {
            SessionStartOutcome::Started {
                context: result.context,
            }
        }
        Ok(result) if result.reason == "security_hook_failure" => {
            SessionStartOutcome::SecurityFailure
        }
        Ok(_) | Err(_) => SessionStartOutcome::Blocked,
    };
    let mut sessions = manager.session_started.lock().await;
    if matches!(sessions.get(agent_session), Some(SessionState::Pending(current)) if Arc::ptr_eq(current, &initialization))
    {
        if matches!(result, SessionStartOutcome::Started { .. }) {
            sessions.insert(
                agent_session.to_owned(),
                SessionState::Started(Instant::now()),
            );
        } else {
            sessions.remove(agent_session);
        }
    }
    drop(sessions);
    initialization.complete(result).await;
}
