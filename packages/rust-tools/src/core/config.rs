//! CLI contract and server configuration for `relay-agent`.
//!
//! The canonical filesystem root is `--workspace-root` / `RELAY_WORKSPACE_ROOT`,
//! with `--dir` retained as a CLI alias. It defaults to
//! `$HOME/Documents/Projects`; `--execution-root` is an optional CLI-only hard
//! ceiling override. The remaining flags retain the existing relay contract.
use crate::core::error::RelayError;
use serde::{Deserialize, Serialize};
mod activity;
mod blender;
mod cli;
mod conversion;
mod creative;
mod defaults;
mod lsp;
mod ssh;
mod validation;
pub use activity::ActivityConfig;
pub use blender::{
    BLENDER_MCP_ANIMATION_PREVIEW_TIMEOUT_MS, BLENDER_MCP_DEFAULT_TIMEOUT_MS,
    BLENDER_MCP_SCREENSHOT_TIMEOUT_MS, BLENDER_MCP_SESSION_TIMEOUT_MS,
};
pub use cli::{ActivityMode, Cli, Command, SecurityMode, ToolProfile, DEFAULT_PORT};
/// Validated server configuration, independent of how it was sourced (CLI,
/// tests, or otherwise). `ServerConfig::default()` is intentionally *not*
/// "production ready" — `origin: None` fails closed in the transport layer's
/// CORS policy, it is not a permissive default.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub mode: SecurityMode,
    pub dir: Option<String>,
    pub origin: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub oauth_secret: Option<String>,
    pub oauth_issuer: Option<String>,
    pub oauth_audience: Option<String>,
    pub oauth_owner_subject: Option<String>,
    pub execution_root: Option<String>,
    pub bind_host: String,
    pub trusted_proxy: bool,
    pub trusted_proxy_cidr: Option<String>,
    pub default_terminal_timeout_ms: u64,
    pub max_terminal_timeout_ms: u64,
    pub completed_job_ttl_ms: u64,
    pub max_retained_output_bytes: usize,
    pub max_running_jobs: usize,
    pub allow_terminal_network: bool,
    pub allow_ssh: bool,
    pub ssh_root: Option<String>,
    pub ssh_config: Option<String>,
    pub ssh_readonly_db_user: Option<String>,
    pub ssh_readonly_redis_user: Option<String>,
    pub ssh_readonly_redis_password_file: Option<String>,
    pub allow_docker: bool,
    pub docker_socket: String,
    pub allow_tailscale: bool,
    pub tailscale_socket: String,
    pub toolchain_paths: Vec<String>,
    /// Operator-approved read-only runtime/toolchain state directories. These
    /// are mounted into the sandbox but are never added to executable PATH.
    pub toolchain_state_paths: Vec<String>,
    /// Operator-approved LSP executable mappings (`language=executable`).
    pub lsp_servers: Vec<String>,
    /// Master switch for the complete first-party Creative production platform,
    /// including Scene, Anime/Blender, Game, graph, delivery, and related tools.
    /// Activation changes require a restart.
    pub enable_creative: bool,
    /// Optional operator-approved Blender executable path/name. Callers never control this value.
    pub blender_executable: Option<String>,
    /// Official Blender Lab loopback bridge port. No host override exists; v1 is loopback-only.
    pub blender_bridge_port: u16,
    /// Bounded Blender Lab request/response timeout.
    pub blender_bridge_timeout_ms: u64,
    /// Operator-supplied bounded JSON descriptors for discoverable Creative execution bindings.
    /// Descriptors never contain endpoints or credentials and never imply a default route.
    pub creative_binding_descriptors: Vec<String>,
    /// Operator-only execution backend implementations keyed by binding ID.
    /// This state is never exposed through Creative discovery responses.
    pub creative_binding_backends: Vec<String>,
    /// Compute units above which Creative submit requires explicit caller approval.
    pub creative_approval_compute_units: u64,
    /// Hard per-job compute ceiling. Caller approval cannot override this maximum.
    pub creative_job_hard_compute_units: u64,
    /// Hard cumulative project compute ceiling across persisted Creative jobs.
    pub creative_project_hard_compute_units: u64,
    /// Hard per-job retained/generated output ceiling.
    pub creative_max_job_output_bytes: u64,
    /// Maximum simultaneously running Creative jobs per project.
    pub creative_max_concurrent_jobs: usize,
    /// Maximum bounded retry count recorded/admitted for one Creative job.
    pub creative_max_retries: u32,
    pub enable_agent_hooks: bool,
    pub agent_hooks_config: Option<String>,
    pub tool_profile: ToolProfile,
    pub activity: ActivityConfig,
    /// Server-only Telegram delivery switch. It is skipped when configuration
    /// is serialized so diagnostics cannot include server state.
    #[serde(skip)]
    pub telegram_enabled: bool,
    #[serde(skip, default = "default_workspaces")]
    pub workspaces:
        std::sync::Arc<std::sync::RwLock<crate::core::workspace_path::WorkspaceAllowlist>>,
}
pub(super) fn default_workspaces(
) -> std::sync::Arc<std::sync::RwLock<crate::core::workspace_path::WorkspaceAllowlist>> {
    std::sync::Arc::new(std::sync::RwLock::new(
        crate::core::workspace_path::WorkspaceAllowlist::default(),
    ))
}
impl ServerConfig {
    /// Resolve the primary workspace root: the configured root, or
    /// `$HOME/Documents/Projects` if unset. Does not touch the filesystem.
    pub fn resolved_dir(&self) -> Result<std::path::PathBuf, RelayError> {
        match &self.dir {
            Some(d) => Ok(std::path::PathBuf::from(d)),
            None => dirs_home()
                .map(|home| home.join("Documents").join("Projects"))
                .ok_or_else(|| {
                    RelayError::InvalidConfig(
                        "no workspace root given and the OS home directory could not be determined"
                            .into(),
                    )
                }),
        }
    }
    /// Resolve the effective execution root, and reject unsafe system paths.
    pub fn resolved_execution_root(&self) -> Result<std::path::PathBuf, RelayError> {
        let root = match &self.execution_root {
            Some(d) => std::path::PathBuf::from(d),
            None => self.resolved_dir()?,
        };
        let canonical = std::fs::canonicalize(&root).map_err(|e| {
            RelayError::InvalidConfig(format!("execution root cannot be resolved: {}", e))
        })?;
        // P0-2: Reject system-level directories that would neutralize filesystem containment.
        // If execution_root is "/", every starts_with check passes — containment is void.
        let forbidden_roots: &[&std::path::Path] = &[
            std::path::Path::new("/"),
            std::path::Path::new("/tmp"),
            std::path::Path::new("/etc"),
            std::path::Path::new("/proc"),
            std::path::Path::new("/sys"),
            std::path::Path::new("/dev"),
            std::path::Path::new("/root"),
            std::path::Path::new("/var"),
            std::path::Path::new("/usr"),
            std::path::Path::new("/bin"),
            std::path::Path::new("/sbin"),
            std::path::Path::new("/lib"),
            std::path::Path::new("/lib64"),
            std::path::Path::new("/boot"),
            std::path::Path::new("/run"),
            std::path::Path::new("/opt"),
            std::path::Path::new("/srv"),
        ];
        for bad in forbidden_roots {
            if canonical.as_path() == *bad {
                return Err(RelayError::InvalidConfig(format!(
                    "execution root '{}' is a forbidden system path and cannot be used as a \
                     filesystem boundary. Configure a user-owned project directory instead \
                     (e.g. /home/user/project).",
                    canonical.display()
                )));
            }
        }
        // `/home/user` is the minimum supported owner-home boundary. This
        // still rejects `/home` and arbitrary top-level roots.
        let depth = canonical.components().count();
        if depth < 3 {
            return Err(RelayError::InvalidConfig(format!(
                "execution root '{}' is too shallow (depth {}). \
                Use a canonical non-root owner home (e.g. /home/user).",
                canonical.display(),
                depth
            )));
        }
        Ok(canonical)
    }
    /// Ensure that the primary workspace root is registered in the allowlist.
    pub fn ensure_workspaces_initialized(&self) -> Result<(), RelayError> {
        let mut guard = self
            .workspaces
            .write()
            .map_err(|_| RelayError::InvalidConfig("workspace allowlist lock poisoned".into()))?;
        if guard.primary_root() == std::path::Path::new("/nonexistent") {
            let boundary = self.resolved_execution_root()?;
            let primary = std::fs::canonicalize(self.resolved_dir()?).map_err(|_| {
                RelayError::InvalidConfig("workspace directory cannot be resolved".into())
            })?;
            guard
                .set_roots(boundary, primary)
                .map_err(|error| RelayError::InvalidConfig(error.to_string()))?;
        }
        Ok(())
    }
    /// Check if a path is contained within any authorized workspace root.
    pub fn is_path_contained(&self, path: &std::path::Path) -> bool {
        let _ = self.ensure_workspaces_initialized();
        if let Ok(guard) = self.workspaces.read() {
            guard.is_contained(path)
        } else if let Ok(root) = self.resolved_execution_root() {
            path.starts_with(&root)
        } else {
            false
        }
    }
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
}

/// Parse a Host header/configuration entry into a normalized host and exact
/// optional port. No default port is added: a missing port remains distinct
/// from every explicit port.
pub fn parse_host_authority(raw: &str) -> Option<(String, Option<u16>)> {
    if raw.is_empty()
        || raw
            .chars()
            .any(|ch| ch.is_ascii_whitespace() || ch.is_ascii_control())
        || raw.contains(['/', '?', '#', '@', '*'])
    {
        return None;
    }

    let parsed = url::Url::parse(&format!("http://{raw}")).ok()?;
    if parsed.username() != ""
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return None;
    }

    Some((parsed.host_str()?.to_ascii_lowercase(), parsed.port()))
}
