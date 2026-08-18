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
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncWriteExt;
use tokio::time::{timeout, Duration};

mod payload;
mod policy;
use payload::{bounded_context, bounded_payload, bounded_string};
pub use policy::effect_classes;
use policy::{canonical_repository_root, contained_config_path, repository_identity};

pub const MAX_CONFIG_BYTES: usize = 64 * 1024;
pub const MAX_HANDLERS: usize = 32;
pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024;
pub const MAX_CONTEXT_BYTES: usize = 8 * 1024;
const MAX_TRACKED_SESSIONS: usize = 256;
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
    #[serde(alias = "effect")]
    pub effect_class: Option<String>,
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
    pub context: Option<Value>,
}

#[derive(Clone)]
pub struct HookManager {
    config: Arc<ServerConfig>,
    root: PathBuf,
    handlers: Arc<Vec<HookHandler>>,
    session_started: Arc<tokio::sync::Mutex<HashSet<String>>>,
    stop_attempts: Arc<tokio::sync::Mutex<HashMap<String, u8>>>,
    approval_tokens: Arc<tokio::sync::Mutex<HashMap<String, (String, String)>>>,
}

impl HookManager {
    pub fn disabled(config: Arc<ServerConfig>) -> Arc<Self> {
        Arc::new(Self {
            root: PathBuf::new(),
            config,
            handlers: Arc::new(Vec::new()),
            session_started: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            stop_attempts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            approval_tokens: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        })
    }

    pub fn load(config: Arc<ServerConfig>) -> Result<Arc<Self>, McpError> {
        if !config.enable_agent_hooks {
            return Ok(Self::disabled(config));
        }
        let execution_root = config
            .resolved_execution_root()
            .map_err(|_| McpError::InvalidRequest("hook repository is unavailable".into()))?;
        let dir = config
            .resolved_dir()
            .map_err(|_| McpError::InvalidRequest("hook repository is unavailable".into()))?;
        let root = canonical_repository_root(&dir, &execution_root)?;
        let relative = config
            .agent_hooks_config
            .as_deref()
            .unwrap_or(".agents/hooks.json");
        let path = contained_config_path(&root, relative)?;
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
            session_started: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            stop_attempts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            approval_tokens: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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

    pub async fn issue_approval(&self, agent_session: &str, tool_id: &str) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.approval_tokens.lock().await.insert(
            token.clone(),
            (
                bounded_string(agent_session, 128),
                bounded_string(tool_id, 128),
            ),
        );
        token
    }

    pub async fn consume_approval(&self, token: &str, agent_session: &str, tool_id: &str) -> bool {
        let mut tokens = self.approval_tokens.lock().await;
        let Some((session, tool)) = tokens.remove(token) else {
            return false;
        };
        session == bounded_string(agent_session, 128) && tool == bounded_string(tool_id, 128)
    }

    pub async fn started_session_count(&self) -> usize {
        self.session_started.lock().await.len()
    }

    pub async fn start_session(
        &self,
        agent_session: &str,
        repository_identity: &str,
    ) -> Option<Value> {
        let mut started = self.session_started.lock().await;
        if started.contains(agent_session) || started.len() >= MAX_TRACKED_SESSIONS {
            return None;
        }
        started.insert(agent_session.to_owned());
        drop(started);
        let result = self
            .invoke(
                HookEvent::SessionStart,
                json!({
                    "hook_event": "session_start",
                    "agentSession": bounded_string(agent_session, 128),
                    "repository_identity": bounded_string(repository_identity, 512),
                    "context": { "repository_identity": bounded_string(repository_identity, 512) },
                }),
            )
            .await;
        result.context
    }

    /// Evaluate one stop boundary. A blocked first attempt gives the agent a
    /// real continuation opportunity; the next boundary is forced through so
    /// a hook cannot create an immediate self-loop or an unbounded stop loop.
    pub async fn pre_agent_stop(&self, agent_session: &str) -> bool {
        let mut attempts = self.stop_attempts.lock().await;
        let attempt = attempts.entry(agent_session.to_owned()).or_insert(0);
        *attempt = attempt.saturating_add(1);
        let current_attempt = *attempt;
        drop(attempts);
        if current_attempt > 2 {
            return true;
        }
        let result = self
            .invoke(
                HookEvent::PreAgentStop,
                json!({
                    "hook_event": "pre_agent_stop",
                    "agentSession": bounded_string(agent_session, 128),
                    "attempt": current_attempt,
                }),
            )
            .await;
        if result.decision == HookDecision::Continue || current_attempt == 2 {
            self.stop_attempts.lock().await.remove(agent_session);
            if result.decision != HookDecision::Continue {
                tracing::warn!(
                    event = "relay.hook",
                    hook_event = "pre_agent_stop",
                    outcome = "proceed_after_bounded_retry"
                );
            }
            return true;
        }
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
                && handler.effect_class.as_deref().is_none_or(|effect| {
                    payload
                        .get("effect_classes")
                        .and_then(Value::as_array)
                        .is_some_and(|effects| {
                            effects.iter().any(|value| value.as_str() == Some(effect))
                        })
                })
        });
        let mut context = None;
        for handler in matching {
            let result = self.run(handler, &payload).await;
            if result.context.is_some() {
                context = result.context.clone();
            }
            if result.decision != HookDecision::Continue {
                return HookResult {
                    duration_ms: started.elapsed().as_millis() as u64,
                    context: result.context,
                    ..result
                };
            }
            if result.reason == "hook_failure" && handler.class == HookClass::Security {
                return HookResult {
                    decision: HookDecision::Block,
                    reason: "security_hook_failure",
                    duration_ms: started.elapsed().as_millis() as u64,
                    context: None,
                };
            }
        }
        HookResult {
            decision: HookDecision::Continue,
            reason: "no_block",
            duration_ms: started.elapsed().as_millis() as u64,
            context,
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
            hook_workspace_access(payload),
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
        let stdout = if let Some(task) = stdout_task {
            task.await.unwrap_or_default()
        } else {
            Vec::new()
        };
        if let Some(task) = stderr_task {
            let _ = task.await;
        }
        let decision = match status.code() {
            Some(0) => HookDecision::Continue,
            Some(BLOCK_EXIT) => HookDecision::Block,
            Some(APPROVAL_EXIT) => HookDecision::RequestApproval,
            _ => return failed_result(started),
        };
        let context = (handler.event == HookEvent::SessionStart)
            .then(|| bounded_context(&stdout))
            .flatten();
        tracing::info!(event = "relay.hook", hook_event = handler.event.name(), decision = ?decision, duration_ms = started.elapsed().as_millis() as u64, reason = "handler_result");
        HookResult {
            decision,
            reason: if decision == HookDecision::Continue {
                "continued"
            } else {
                "handler_decision"
            },
            duration_ms: started.elapsed().as_millis() as u64,
            context,
        }
    }
}

fn hook_workspace_access(payload: &Value) -> sandbox::WorkspaceAccess {
    let writable = payload
        .get("effect_classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|effect| {
            matches!(
                effect,
                "workspace_write" | "workspace_delete" | "external_mutation"
            )
        });
    if writable {
        sandbox::WorkspaceAccess::Writable
    } else {
        sandbox::WorkspaceAccess::ReadOnly
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
        context: None,
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
            .effect_class
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

async fn drain_output<R: AsyncRead + Unpin>(mut stream: R) -> Vec<u8> {
    let mut retained = 0usize;
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match tokio::io::AsyncReadExt::read(&mut stream, &mut buffer).await {
            Ok(0) | Err(_) => return output,
            Ok(bytes) => {
                let remaining = MAX_CONTEXT_BYTES.saturating_sub(retained);
                output.extend_from_slice(&buffer[..bytes.min(remaining)]);
                retained = retained.saturating_add(bytes).min(MAX_CONTEXT_BYTES);
                if retained >= MAX_CONTEXT_BYTES {
                    return output;
                }
            }
        }
    }
}
