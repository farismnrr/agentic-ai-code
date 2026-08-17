//! Deterministic, vendor-neutral lifecycle hooks.
//!
//! Hooks are subordinate to the relay policy: they can block or request
//! approval, but there is no hook result that grants a capability. Repository
//! configuration is inert until the operator enables it and its identity
//! matches the canonical contained repository.

use crate::execution::sandbox;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncWriteExt;
use tokio::time::{timeout, Duration};

pub const MAX_CONFIG_BYTES: usize = 64 * 1024;
pub const MAX_HANDLERS: usize = 32;
pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024;
pub const MAX_CONTEXT_BYTES: usize = 8 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 30_000;
const BLOCK_EXIT: i32 = 10;
const APPROVAL_EXIT: i32 = 11;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    PreToolUse,
    PostToolUse,
    ToolError,
    AfterFileChange,
    PreAgentStop,
}

impl HookEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::ToolError => "tool_error",
            Self::AfterFileChange => "after_file_change",
            Self::PreAgentStop => "pre_agent_stop",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookClass {
    Security,
    Cosmetic,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HookHandler {
    pub event: HookEvent,
    pub command: Vec<String>,
    #[serde(default = "default_class")]
    pub class: HookClass,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HookConfig {
    pub repository_identity: String,
    pub handlers: Vec<HookHandler>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookDecision {
    Continue,
    Block,
    RequestApproval,
}

#[derive(Debug, Clone)]
pub struct HookResult {
    pub decision: HookDecision,
    pub reason: &'static str,
    pub duration_ms: u64,
}

#[derive(Clone)]
pub struct HookManager {
    config: Arc<ServerConfig>,
    root: PathBuf,
    handlers: Arc<Vec<HookHandler>>,
    session_started: Arc<AtomicBool>,
}

impl HookManager {
    pub fn disabled(config: Arc<ServerConfig>) -> Arc<Self> {
        Arc::new(Self {
            root: PathBuf::new(),
            config,
            handlers: Arc::new(Vec::new()),
            session_started: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn load(config: Arc<ServerConfig>) -> Result<Arc<Self>, McpError> {
        if !config.enable_agent_hooks {
            return Ok(Self::disabled(config));
        }
        let root = config
            .resolved_execution_root()
            .map_err(|_| McpError::InvalidRequest("hook repository is unavailable".into()))?;
        let relative = config
            .agent_hooks_config
            .as_deref()
            .unwrap_or(".agents/hooks.json");
        let path = root.join(relative);
        if !path.starts_with(root.join(".agents")) {
            return Err(McpError::InvalidRequest(
                "hook configuration must be beneath .agents".into(),
            ));
        }
        let bytes = std::fs::read(&path)
            .map_err(|_| McpError::InvalidRequest("hook configuration is unavailable".into()))?;
        if bytes.len() > MAX_CONFIG_BYTES {
            return Err(McpError::InvalidRequest(
                "hook configuration is too large".into(),
            ));
        }
        let parsed: HookConfig = serde_json::from_slice(&bytes)
            .map_err(|_| McpError::InvalidRequest("hook configuration is invalid".into()))?;
        let identity = repository_identity(&root)?;
        if parsed.repository_identity != identity || parsed.handlers.len() > MAX_HANDLERS {
            return Err(McpError::InvalidRequest(
                "hook repository identity is not trusted".into(),
            ));
        }
        for handler in &parsed.handlers {
            validate_handler(handler, &root)?;
        }
        Ok(Arc::new(Self {
            config,
            root,
            handlers: Arc::new(parsed.handlers),
            session_started: Arc::new(AtomicBool::new(false)),
        }))
    }

    pub fn is_enabled(&self) -> bool {
        !self.handlers.is_empty()
    }

    pub fn repository_identity(&self) -> Option<String> {
        self.is_enabled()
            .then(|| repository_identity(&self.root).ok())
            .flatten()
    }

    pub async fn start_session(&self, session_id: &str, repository_identity: &str) {
        if self.session_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = self
            .invoke(
                HookEvent::SessionStart,
                json!({
                    "hook_event": "session_start",
                    "session_id": bounded_string(session_id, 128),
                    "repository_identity": bounded_string(repository_identity, 512),
                }),
            )
            .await;
    }

    /// One bounded stop-gate retry. A stop request never loops indefinitely;
    /// callers may invoke this at most twice before proceeding with shutdown.
    pub async fn pre_agent_stop(&self, session_id: &str) -> bool {
        for attempt in 0..2 {
            let result = self
                .invoke(
                    HookEvent::PreAgentStop,
                    json!({
                        "hook_event": "pre_agent_stop",
                        "session_id": bounded_string(session_id, 128),
                        "attempt": attempt + 1,
                    }),
                )
                .await;
            if result.decision == HookDecision::Continue {
                return true;
            }
        }
        tracing::warn!(
            event = "relay.hook",
            hook_event = "pre_agent_stop",
            outcome = "proceed_after_bounded_retry"
        );
        false
    }

    pub async fn invoke(&self, event: HookEvent, payload: Value) -> HookResult {
        let started = Instant::now();
        let payload = bounded_payload(payload);
        let matching = self.handlers.iter().filter(|handler| {
            handler.event == event
                && handler
                    .tool
                    .as_deref()
                    .is_none_or(|tool| payload.get("tool_id").and_then(Value::as_str) == Some(tool))
                && handler.effect.as_deref().is_none_or(|effect| {
                    payload.get("effect_class").and_then(Value::as_str) == Some(effect)
                })
        });
        for handler in matching {
            let result = self.run(handler, &payload).await;
            if result.decision != HookDecision::Continue {
                return HookResult {
                    duration_ms: started.elapsed().as_millis() as u64,
                    ..result
                };
            }
            if result.reason == "hook_failure" && handler.class == HookClass::Security {
                return HookResult {
                    decision: HookDecision::Block,
                    reason: "security_hook_failure",
                    duration_ms: started.elapsed().as_millis() as u64,
                };
            }
        }
        HookResult {
            decision: HookDecision::Continue,
            reason: "no_block",
            duration_ms: started.elapsed().as_millis() as u64,
        }
    }

    async fn run(&self, handler: &HookHandler, payload: &Value) -> HookResult {
        let started = Instant::now();
        let executable = match sandbox::resolve_safe_executable(&self.config, &handler.command[0]) {
            Ok(path) => path,
            Err(_) => return failed_result(started),
        };
        let mut child = match sandbox::spawn_hook(
            &self.config,
            executable,
            handler.command[1..].to_vec(),
            self.root.clone(),
        ) {
            Ok(child) => child,
            Err(_) => return failed_result(started),
        };
        let stdout_task = child
            .stdout
            .take()
            .map(|stream| tokio::spawn(drain_output(stream)));
        let stderr_task = child
            .stderr
            .take()
            .map(|stream| tokio::spawn(drain_output(stream)));
        let input = serde_json::to_vec(payload).unwrap_or_default();
        if input.len() > MAX_PAYLOAD_BYTES {
            return failed_result(started);
        }
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&input).await;
            let _ = stdin.shutdown().await;
        }
        let deadline = if handler.timeout_ms == 0 {
            DEFAULT_TIMEOUT_MS
        } else {
            handler.timeout_ms.clamp(1, MAX_TIMEOUT_MS)
        };
        let status = match timeout(Duration::from_millis(deadline), child.wait()).await {
            Ok(Ok(status)) => status,
            _ => {
                crate::execution::kill_process_group(&mut child).await;
                if let Some(task) = stdout_task {
                    let _ = task.await;
                }
                if let Some(task) = stderr_task {
                    let _ = task.await;
                }
                return failed_result(started);
            }
        };
        if let Some(task) = stdout_task {
            let _ = task.await;
        }
        if let Some(task) = stderr_task {
            let _ = task.await;
        }
        let decision = match status.code() {
            Some(0) => HookDecision::Continue,
            Some(BLOCK_EXIT) => HookDecision::Block,
            Some(APPROVAL_EXIT) => HookDecision::RequestApproval,
            _ => return failed_result(started),
        };
        tracing::info!(event = "relay.hook", hook_event = handler.event.name(), decision = ?decision, duration_ms = started.elapsed().as_millis() as u64, reason = "handler_result");
        HookResult {
            decision,
            reason: if decision == HookDecision::Continue {
                "continued"
            } else {
                "handler_decision"
            },
            duration_ms: started.elapsed().as_millis() as u64,
        }
    }
}

fn default_class() -> HookClass {
    HookClass::Cosmetic
}

fn failed_result(started: Instant) -> HookResult {
    HookResult {
        decision: HookDecision::Continue,
        reason: "hook_failure",
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn validate_handler(handler: &HookHandler, root: &Path) -> Result<(), McpError> {
    if handler.command.is_empty()
        || handler.command.len() > 32
        || handler.command.iter().any(|arg| arg.len() > 4096)
    {
        return Err(McpError::InvalidRequest("hook command is invalid".into()));
    }
    if handler.command[0].contains('/')
        || handler.command[0].contains('\\')
        || handler.command[0].is_empty()
        || [
            "sh",
            "bash",
            "dash",
            "zsh",
            "fish",
            "cmd",
            "powershell",
            "pwsh",
        ]
        .contains(&handler.command[0].as_str())
        || handler.command[1..]
            .iter()
            .any(|arg| ["-c", "-lc", "/c", "-command"].contains(&arg.as_str()))
    {
        return Err(McpError::InvalidRequest(
            "hook command must use a safe PATH executable".into(),
        ));
    }
    if handler.timeout_ms > MAX_TIMEOUT_MS
        || handler
            .tool
            .as_deref()
            .is_some_and(|value| value.len() > 128)
        || handler
            .effect
            .as_deref()
            .is_some_and(|value| value.len() > 64)
    {
        return Err(McpError::InvalidRequest(
            "hook handler exceeds bounds".into(),
        ));
    }
    if !root.join(".agents").is_dir() {
        return Err(McpError::InvalidRequest(
            "hook repository metadata is unavailable".into(),
        ));
    }
    Ok(())
}

async fn drain_output<R: AsyncRead + Unpin>(mut stream: R) {
    let mut retained = 0usize;
    let mut buffer = [0u8; 4096];
    loop {
        match tokio::io::AsyncReadExt::read(&mut stream, &mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(bytes) => {
                retained = retained.saturating_add(bytes);
                if retained >= MAX_CONTEXT_BYTES {
                    return;
                }
            }
        }
    }
}

fn bounded_payload(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.remove("raw_output");
        object.remove("content");
        object.remove("prompt");
        object.remove("secrets");
        object.remove("environment");
        object.remove("command_output");
    }
    let encoded = serde_json::to_vec(&payload).unwrap_or_default();
    if encoded.len() <= MAX_PAYLOAD_BYTES {
        payload
    } else {
        json!({ "hook_payload_truncated": true })
    }
}

fn bounded_string(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn repository_identity(root: &Path) -> Result<String, McpError> {
    let git = root.join(".git");
    let git_identity = std::fs::canonicalize(&git)
        .map_err(|_| McpError::InvalidRequest("repository identity is unavailable".into()))?;
    Ok(format!("{}|{}", root.display(), git_identity.display()))
}
