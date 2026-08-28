//! Bounded, vendor-neutral activity facts. This module deliberately contains no
//! transport, persistence, or provider types.

use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod presentation;
mod validation;
use presentation::{action_for_tool, target_for_tool};
pub use validation::transition_allowed;

pub const CONTRACT_VERSION: &str = "activity.event.v1";
pub const MAX_TEXT: usize = 256;
pub const MAX_PATH: usize = 4096;
pub const MAX_LIST: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityError {
    AdmissionFailed,
    StorageUnavailable,
    InvalidEvent,
}

impl fmt::Display for ActivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AdmissionFailed => "activity could not be durably admitted",
            Self::StorageUnavailable => "activity storage is unavailable",
            Self::InvalidEvent => "activity event is invalid",
        })
    }
}

impl std::error::Error for ActivityError {}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Started,
    Running,
    Ok,
    Error,
    Denied,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Filesystem,
    Search,
    Terminal,
    Git,
    Code,
    Delegated,
    Network,
    Workspace,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    WorkspaceRead,
    WorkspaceWrite,
    WorkspaceDelete,
    ProcessExec,
    NetworkRead,
    NetworkWrite,
    GitRead,
    ExternalMutation,
    PrivilegedBridge,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    Exact,
    Summary,
    Unavailable,
    NotApplicable,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    #[serde(default = "external_actor")]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
}

/// Client-reported display metadata. It is intentionally separate from the
/// truthful actor/source facts and never participates in permission decisions.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

fn external_actor() -> String {
    "External MCP client".into()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub summary: Option<String>,
    pub result_class: Option<String>,
    pub evidence: Evidence,
    pub payload_reference: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActivityEvent {
    pub contract_version: String,
    pub activity_id: String,
    pub source_id: String,
    pub source_sequence: u64,
    pub status: Status,
    pub tool_id: String,
    pub category: Category,
    pub effects: Vec<Effect>,
    pub workspace_root_fingerprint: Option<String>,
    pub actor: Actor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,
    pub occurred_at_ms: i64,
    #[serde(skip_serializing)]
    pub ingested_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub presentation: Presentation,
}

/// Build the relay-boundary event from validated tool identity and the
/// canonical workspace allowlist. The arguments are inspected only to derive
/// a bounded display target; they are never included in the event.
pub fn event_for_tool(
    config: &relay_core::config::ServerConfig,
    tool_id: &str,
    effects: &[&str],
    arguments: &Value,
    client_info: Option<(&str, &str)>,
) -> ActivityEvent {
    let root = workspace_root(config, arguments.get("cwd").and_then(Value::as_str));
    let client_info = client_info.and_then(|(name, version)| {
        let name = bounded_display(name, MAX_TEXT)?;
        let version = bounded_display(version, 64)?;
        Some(ClientInfo { name, version })
    });
    ActivityEvent {
        contract_version: CONTRACT_VERSION.into(),
        activity_id: Uuid::new_v4().to_string(),
        source_id: String::new(),
        source_sequence: 0,
        status: Status::Started,
        tool_id: bounded_display(tool_id, MAX_TEXT).unwrap_or_else(|| "unknown_tool".into()),
        category: category_for_tool(tool_id),
        effects: effects
            .iter()
            .filter_map(|effect| effect_for_name(effect))
            .collect(),
        workspace_root_fingerprint: root
            .as_ref()
            .map(|root| workspace_root_fingerprint(root.to_string_lossy().as_bytes())),
        actor: actor_or_external(None),
        client_info,
        occurred_at_ms: now_ms(),
        ingested_at_ms: None,
        duration_ms: None,
        presentation: Presentation {
            target: target_for_tool(tool_id, arguments, root.as_deref()),
            action: action_for_tool(tool_id, arguments, root.as_deref()),
            summary: Some("operation admitted".into()),
            result_class: Some("started".into()),
            evidence: Evidence::NotApplicable,
            payload_reference: None,
            complete: false,
        },
    }
}

pub fn complete_event(
    start: &ActivityEvent,
    status: Status,
    duration_ms: u64,
    summary: &str,
    evidence: Evidence,
    payload_reference: Option<String>,
) -> ActivityEvent {
    let mut event = start.with_status(status, Some(duration_ms));
    event.occurred_at_ms = now_ms();
    event.presentation.summary = Some(summary.chars().take(MAX_TEXT).collect());
    event.presentation.result_class = Some(status_name(status).into());
    event.presentation.evidence = evidence;
    event.presentation.payload_reference = payload_reference;
    event.presentation.complete = matches!(
        status,
        Status::Ok | Status::Error | Status::Denied | Status::Cancelled | Status::Interrupted
    );
    event
}

fn workspace_root(config: &relay_core::config::ServerConfig, cwd: Option<&str>) -> Option<PathBuf> {
    let _ = config.ensure_workspaces_initialized();
    let guard = config.workspaces.read().ok()?;
    let cwd = relay_core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd).ok()?;
    guard.containing_root(&cwd).map(Path::to_path_buf)
}

fn category_for_tool(tool_id: &str) -> Category {
    if matches!(
        tool_id,
        "file_read"
            | "file_write"
            | "file_edit"
            | "apply_patch"
            | "directory_list"
            | "workspace_add"
            | "workspace_remove"
    ) {
        Category::Filesystem
    } else if matches!(tool_id, "file_search" | "text_search" | "web_search") {
        Category::Search
    } else if tool_id.starts_with("terminal_") {
        Category::Terminal
    } else if tool_id.starts_with("git_") {
        Category::Git
    } else if tool_id.starts_with("code_") {
        Category::Code
    } else if matches!(tool_id, "http_fetch") {
        Category::Network
    } else {
        Category::Other
    }
}

fn effect_for_name(effect: &str) -> Option<Effect> {
    Some(match effect {
        "workspace_read" => Effect::WorkspaceRead,
        "workspace_write" => Effect::WorkspaceWrite,
        "workspace_delete" => Effect::WorkspaceDelete,
        "process_exec" => Effect::ProcessExec,
        "network_read" => Effect::NetworkRead,
        "network_write" => Effect::NetworkWrite,
        "git_read" => Effect::GitRead,
        "external_mutation" => Effect::ExternalMutation,
        "privileged_bridge" => Effect::PrivilegedBridge,
        _ => return None,
    })
}

fn bounded_display(value: &str, max: usize) -> Option<String> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return None;
    }
    Some(value.chars().take(max).collect())
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Started => "started",
        Status::Running => "running",
        Status::Ok => "ok",
        Status::Error => "error",
        Status::Denied => "denied",
        Status::Cancelled => "cancelled",
        Status::Interrupted => "interrupted",
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

/// Application-facing activity persistence boundary. Concrete journal,
/// encryption, and exporter implementations stay in infrastructure.
pub trait ActivityRecorder: Send + Sync {
    fn required(&self) -> bool;
    fn record_start(
        &self,
        event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<ActivityEvent, ActivityError>;
    fn record_outcome(
        &self,
        event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<(), ActivityError>;
}

pub type SharedActivityRecorder = Arc<dyn ActivityRecorder>;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopActivityRecorder;

impl ActivityRecorder for NoopActivityRecorder {
    fn required(&self) -> bool {
        false
    }

    fn record_start(
        &self,
        event: ActivityEvent,
        _payload: Option<Vec<u8>>,
    ) -> Result<ActivityEvent, ActivityError> {
        Ok(event)
    }

    fn record_outcome(
        &self,
        _event: ActivityEvent,
        _payload: Option<Vec<u8>>,
    ) -> Result<(), ActivityError> {
        Ok(())
    }
}

pub fn workspace_root_fingerprint(canonical_root: &[u8]) -> String {
    digest(&SHA256, canonical_root)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn actor_or_external(actor: Option<Actor>) -> Actor {
    actor.unwrap_or_else(|| Actor {
        label: external_actor(),
        source: None,
        channel: None,
    })
}
