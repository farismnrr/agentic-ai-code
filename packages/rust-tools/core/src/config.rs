//! CLI contract and server configuration for `relay-agent`.
//!
//! The flag set here matches the legacy Node CLI (`packages/relay-agent/bin/cli.mjs`)
//! exactly, per the frozen audit in `.agents/plans/028-phase0-contract-audit.md`:
//! `--dir`/`-d`, `--port`/`-p` (default `47821`), `--origin`/`-o` (env fallback
//! `RELAY_AGENT_ORIGIN`), `--bind-host` (env fallback
//! `RELAY_AGENT_BIND_HOST`), and a `stop --port <port>` subcommand.
use crate::error::RelayError;
use serde::{Deserialize, Serialize};
mod activity;
mod cli;
mod lsp;
pub use activity::ActivityConfig;
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
    pub allow_docker: bool,
    pub docker_socket: String,
    pub allow_tailscale: bool,
    pub tailscale_socket: String,
    pub toolchain_paths: Vec<String>,
    /// Operator-approved LSP executable mappings (`language=executable`).
    pub lsp_servers: Vec<String>,
    pub enable_agent_hooks: bool,
    pub agent_hooks_config: Option<String>,
    pub tool_profile: ToolProfile,
    pub activity: ActivityConfig,
    #[serde(skip, default = "default_workspaces")]
    pub workspaces: std::sync::Arc<std::sync::RwLock<crate::workspace_path::WorkspaceAllowlist>>,
}
fn default_workspaces(
) -> std::sync::Arc<std::sync::RwLock<crate::workspace_path::WorkspaceAllowlist>> {
    std::sync::Arc::new(std::sync::RwLock::new(
        crate::workspace_path::WorkspaceAllowlist::default(),
    ))
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            mode: SecurityMode::Local,
            dir: None,
            origin: None,
            allowed_hosts: Vec::new(),
            oauth_secret: None,
            oauth_issuer: None,
            oauth_audience: None,
            oauth_owner_subject: None,
            execution_root: None,
            bind_host: "127.0.0.1".into(),
            trusted_proxy: false,
            trusted_proxy_cidr: None,
            default_terminal_timeout_ms: 30_000,
            max_terminal_timeout_ms: 0,
            completed_job_ttl_ms: 3_600_000,
            max_retained_output_bytes: 1_048_576,
            max_running_jobs: 16,
            allow_terminal_network: false,
            allow_docker: false,
            docker_socket: "/var/run/docker.sock".into(),
            allow_tailscale: false,
            tailscale_socket: "/var/run/tailscale/tailscaled.sock".into(),
            toolchain_paths: Vec::new(),
            lsp_servers: Vec::new(),
            enable_agent_hooks: false,
            agent_hooks_config: None,
            tool_profile: ToolProfile::Full,
            activity: ActivityConfig::default(),
            workspaces: default_workspaces(),
        }
    }
}
impl ServerConfig {
    /// Resolve the effective working directory: the configured `dir`, or the
    /// OS home directory if unset. Does not touch the filesystem.
    pub fn resolved_dir(&self) -> Result<std::path::PathBuf, RelayError> {
        match &self.dir {
            Some(d) => Ok(std::path::PathBuf::from(d)),
            None => dirs_home().ok_or_else(|| {
                RelayError::InvalidConfig(
                    "no --dir given and the OS home directory could not be determined".into(),
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

    /// Validate configuration before binding. Never broadens trust (e.g.
    /// never rewrites an origin into a wildcard or a laxer form).
    pub fn validate(&self) -> Result<(), RelayError> {
        if self.port == 0 {
            return Err(RelayError::InvalidConfig(
                "port must be non-zero".to_string(),
            ));
        }
        if let Some(origin) = &self.origin {
            if origin == "*" {
                return Err(RelayError::InvalidConfig(
                    "wildcard origin is not permitted".to_string(),
                ));
            }
            if origin.trim().is_empty() {
                return Err(RelayError::InvalidConfig(
                    "origin must not be blank".to_string(),
                ));
            }
        }
        for host in &self.allowed_hosts {
            if parse_host_authority(host).is_none() {
                return Err(RelayError::InvalidConfig(
                    "allowed-host must be a hostname or IP address with an optional numeric port; wildcards and URL syntax are not permitted".to_string(),
                ));
            }
        }
        let bind_ip = self.bind_host.parse::<std::net::IpAddr>().map_err(|_| {
            RelayError::InvalidConfig("bind-host must be a valid IPv4 or IPv6 address".to_string())
        })?;
        if self.mode == SecurityMode::Remote
            && (self.oauth_issuer.is_none()
                || self.oauth_audience.is_none()
                || self.oauth_owner_subject.is_none())
        {
            return Err(RelayError::InvalidConfig(
                "oauth_issuer, oauth_audience, and oauth_owner_subject are required in remote mode"
                    .to_string(),
            ));
        }
        if self.mode == SecurityMode::Remote {
            let Some(issuer) = self.oauth_issuer.as_deref() else {
                return Err(RelayError::InvalidConfig(
                    "oauth_issuer is required in remote mode".into(),
                ));
            };
            let parsed = url::Url::parse(issuer).map_err(|_| {
                RelayError::InvalidConfig(
                    "oauth_issuer must be a canonical absolute HTTPS URI".into(),
                )
            })?;
            let fixture_override = cfg!(debug_assertions)
                && std::env::var("RELAY_AGENT_ALLOW_INSECURE_OAUTH_ISSUER_FIXTURE").as_deref()
                    == Ok("1");
            if !fixture_override
                && (parsed.as_str() != issuer
                    || parsed.scheme() != "https"
                    || parsed.host_str().is_none()
                    || parsed.cannot_be_a_base()
                    || !parsed.has_authority()
                    || parsed.username() != ""
                    || parsed.password().is_some()
                    || parsed.query().is_some()
                    || parsed.fragment().is_some())
            {
                return Err(RelayError::InvalidConfig(
                    "oauth_issuer must be a canonical absolute HTTPS URI without credentials, query, or fragment".into(),
                ));
            }
            // The only plaintext exception is a debug-only local JWKS fixture
            // used by deterministic black-box conformance. Release builds
            // cannot enable this path.
            let Some(audience) = self.oauth_audience.as_deref() else {
                return Err(RelayError::InvalidConfig(
                    "oauth_audience is required in remote mode".into(),
                ));
            };
            let parsed = url::Url::parse(audience).map_err(|_| {
                RelayError::InvalidConfig(
                    "oauth_audience must be a canonical absolute HTTPS URI".into(),
                )
            })?;
            if parsed.as_str() != audience
                || parsed.scheme() != "https"
                || parsed.host_str().is_none()
                || parsed.cannot_be_a_base()
                || !parsed.has_authority()
                || parsed.username() != ""
                || parsed.password().is_some()
                || parsed.query().is_some()
                || parsed.fragment().is_some()
            {
                return Err(RelayError::InvalidConfig(
                    "oauth_audience must be a canonical absolute HTTPS URI without credentials, query, or fragment".into(),
                ));
            }
        }
        if self.mode == SecurityMode::Local
            && self.bind_host != "127.0.0.1"
            && self.bind_host != "::1"
        {
            return Err(RelayError::InvalidConfig(
                "local mode must bind to loopback".into(),
            ));
        }
        if self.mode == SecurityMode::Remote {
            if self.bind_host.trim().is_empty() {
                return Err(RelayError::InvalidConfig(
                    "remote bind host must not be blank".into(),
                ));
            }
            if !bind_ip.is_loopback() && self.origin.is_none() {
                return Err(RelayError::InvalidConfig(
                    "non-loopback remote binds require an explicit browser Origin".into(),
                ));
            }
        }
        if self.trusted_proxy && self.mode != SecurityMode::Remote {
            return Err(RelayError::InvalidConfig(
                "--trusted-proxy is only valid in remote mode".into(),
            ));
        }
        let trusted_proxy_cidr = if self.trusted_proxy {
            let Some(cidr) = self.trusted_proxy_cidr.as_deref() else {
                return Err(RelayError::InvalidConfig(
                    "--trusted-proxy requires --trusted-proxy-cidr to identify the edge peer"
                        .into(),
                ));
            };
            Some(cidr.parse::<ipnet::IpNet>().map_err(|_| {
                RelayError::InvalidConfig("--trusted-proxy-cidr must be a valid IP CIDR".into())
            })?)
        } else if self.trusted_proxy_cidr.is_some() {
            return Err(RelayError::InvalidConfig(
                "--trusted-proxy-cidr requires --trusted-proxy".into(),
            ));
        } else {
            None
        };
        if !bind_ip.is_loopback()
            && self.trusted_proxy
            && !trusted_proxy_cidr
                .expect("trusted proxy CIDR is validated above")
                .contains(&std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
        {
            return Err(RelayError::InvalidConfig(
                "non-loopback binds with trusted proxy require a CIDR containing 127.0.0.1".into(),
            ));
        }
        if self.max_running_jobs == 0 {
            return Err(RelayError::InvalidConfig(
                "max_running_jobs must be non-zero".into(),
            ));
        }
        if self.max_retained_output_bytes == 0 {
            return Err(RelayError::InvalidConfig(
                "max_retained_output_bytes must be non-zero".into(),
            ));
        }
        activity::validate(&self.activity)?;
        if self.allow_docker {
            let socket = std::path::Path::new(&self.docker_socket);
            if !socket.is_absolute() {
                return Err(RelayError::InvalidConfig(
                    "docker-socket must be an absolute path".into(),
                ));
            }
        }
        if self.allow_tailscale {
            let socket = std::path::Path::new(&self.tailscale_socket);
            if !socket.is_absolute() {
                return Err(RelayError::InvalidConfig(
                    "tailscale-socket must be an absolute path".into(),
                ));
            }
        }
        lsp::validate_entries(&self.lsp_servers)?;
        let execution_root = self.resolved_execution_root()?;
        let workspace = std::fs::canonicalize(self.resolved_dir()?).map_err(|_| {
            RelayError::InvalidConfig("workspace directory cannot be resolved".into())
        })?;
        if !workspace.is_dir() || !workspace.starts_with(&execution_root) {
            return Err(RelayError::InvalidConfig(
                "workspace directory must be contained by execution root".into(),
            ));
        }
        if let Some(path) = &self.agent_hooks_config {
            if !self.enable_agent_hooks {
                return Err(RelayError::InvalidConfig(
                    "agent-hooks-config requires --enable-agent-hooks".into(),
                ));
            }
            if path.contains('\0') || !std::path::Path::new(path).is_relative() {
                return Err(RelayError::InvalidConfig(
                    "agent-hooks-config must be a relative repository path".into(),
                ));
            }
        }
        for path in &self.toolchain_paths {
            let candidate = std::fs::canonicalize(path).map_err(|_| {
                RelayError::InvalidConfig(
                    "toolchain-path must resolve to an existing directory".into(),
                )
            })?;
            if !candidate.is_dir() || !candidate.starts_with(&execution_root) {
                return Err(RelayError::InvalidConfig(
                    "toolchain-path must be a directory beneath execution root".into(),
                ));
            }
        }
        Ok(())
    }
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
}

impl From<&Cli> for ServerConfig {
    fn from(cli: &Cli) -> Self {
        Self {
            port: cli.port,
            mode: cli.mode,
            trusted_proxy: cli.trusted_proxy,
            trusted_proxy_cidr: cli.trusted_proxy_cidr.clone(),
            dir: cli.dir.clone(),
            origin: cli.origin.clone(),
            allowed_hosts: cli.allowed_hosts.clone(),
            oauth_secret: cli.oauth_secret.clone(),
            oauth_issuer: cli.oauth_issuer.clone(),
            oauth_audience: cli.oauth_audience.clone(),
            oauth_owner_subject: cli.oauth_owner_subject.clone(),
            execution_root: cli.execution_root.clone(),
            bind_host: cli.bind_host.clone(),
            default_terminal_timeout_ms: cli.default_terminal_timeout_ms,
            max_terminal_timeout_ms: cli.max_terminal_timeout_ms,
            completed_job_ttl_ms: cli.completed_job_ttl_ms,
            max_retained_output_bytes: cli.max_retained_output_bytes,
            max_running_jobs: cli.max_running_jobs,
            allow_terminal_network: cli.allow_terminal_network,
            allow_docker: cli.allow_docker,
            docker_socket: cli.docker_socket.clone(),
            allow_tailscale: cli.allow_tailscale,
            tailscale_socket: cli.tailscale_socket.clone(),
            toolchain_paths: cli.toolchain_paths.clone(),
            lsp_servers: cli.lsp_servers.clone(),
            enable_agent_hooks: cli.enable_agent_hooks,
            agent_hooks_config: cli.agent_hooks_config.clone(),
            tool_profile: cli.tool_profile,
            activity: ActivityConfig {
                mode: cli.activity_mode,
                state_dir: cli.activity_state_dir.clone(),
                sink_url: cli.activity_sink_url.clone(),
                source_token: cli.activity_source_token.clone(),
                spool_quota_bytes: cli.activity_spool_quota_bytes,
                acknowledged_retention_ms: cli.activity_ack_retention_ms,
            },
            workspaces: default_workspaces(),
        }
    }
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
