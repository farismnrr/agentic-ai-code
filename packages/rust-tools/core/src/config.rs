//! CLI contract and server configuration for `relay-agent`.
//!
//! The flag set here matches the legacy Node CLI (`packages/relay-agent/bin/cli.mjs`)
//! exactly, per the frozen audit in `.agents/plans/028-phase0-contract-audit.md`:
//! `--dir`/`-d`, `--port`/`-p` (default `47821`), `--origin`/`-o` (env fallback
//! `RELAY_AGENT_ORIGIN`), and a `stop --port <port>` subcommand.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::error::RelayError;

pub const DEFAULT_PORT: u16 = 47_821;

/// Top-level CLI, matching the legacy `relay-agent [--port] [--dir] [--origin]`
/// and `relay-agent stop --port <port>` invocations.
#[derive(Parser, Debug)]
#[command(name = "ai-tools relay-agent", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Port to run the server on.
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// Explicit security mode: local (loopback only) or remote (OAuth required).
    #[arg(long, value_enum, env = "RELAY_AGENT_MODE", default_value = "local")]
    pub mode: SecurityMode,

    /// Trust X-Forwarded-Proto from a reverse proxy that is bound to this
    /// relay's loopback listener. Remote mode never enables this implicitly.
    #[arg(long, env = "RELAY_AGENT_TRUSTED_PROXY", default_value_t = false)]
    pub trusted_proxy: bool,

    /// CIDR containing the local HTTPS edge/tunnel peer. Required when
    /// --trusted-proxy is enabled; forwarded headers from other peers are ignored.
    #[arg(long, env = "RELAY_AGENT_TRUSTED_PROXY_CIDR")]
    pub trusted_proxy_cidr: Option<String>,

    /// Default working directory configuration, not a filesystem sandbox (falls back to the OS home directory).
    #[arg(short, long)]
    pub dir: Option<String>,

    /// Allowed Nuxt/browser origin for MCP requests.
    #[arg(short, long, env = "RELAY_AGENT_ORIGIN")]
    pub origin: Option<String>,

    /// Additional exact Host authorities allowed in local mode. Entries may
    /// optionally include a port; comma-separated and repeated values are
    /// supported.
    #[arg(
        long = "allowed-host",
        env = "RELAY_ALLOWED_HOSTS",
        value_delimiter = ','
    )]
    pub allowed_hosts: Vec<String>,

    /// Legacy OAuth symmetric secret retained for compatibility; the remote
    /// auth path validates JWTs with issuer/audience and JWKS instead.
    #[arg(long, env = "OAUTH_SECRET")]
    pub oauth_secret: Option<String>,

    /// OAuth issuer expected in JWT 'iss' claim
    #[arg(long, env = "OAUTH_ISSUER")]
    pub oauth_issuer: Option<String>,

    /// OAuth audience expected in JWT 'aud' claim
    #[arg(long, env = "OAUTH_AUDIENCE")]
    pub oauth_audience: Option<String>,

    /// Stable OAuth subject allowed to operate this single-owner coding agent.
    #[arg(long, env = "OAUTH_OWNER_SUBJECT")]
    pub oauth_owner_subject: Option<String>,

    /// Explicit execution root for filesystem containment.
    #[arg(long, env = "EXECUTION_ROOT")]
    pub execution_root: Option<String>,

    /// Default terminal deadline in milliseconds; zero means no deadline.
    #[arg(
        long,
        env = "RELAY_DEFAULT_TERMINAL_TIMEOUT_MS",
        default_value_t = 30_000
    )]
    pub default_terminal_timeout_ms: u64,

    /// Maximum terminal deadline in milliseconds; zero means no operator maximum.
    #[arg(long, env = "RELAY_MAX_TERMINAL_TIMEOUT_MS", default_value_t = 0)]
    pub max_terminal_timeout_ms: u64,

    /// Completed-job retention in milliseconds.
    #[arg(long, env = "RELAY_COMPLETED_JOB_TTL_MS", default_value_t = 3_600_000)]
    pub completed_job_ttl_ms: u64,

    /// Total retained stdout/stderr bytes per job.
    #[arg(
        long,
        env = "RELAY_MAX_RETAINED_OUTPUT_BYTES",
        default_value_t = 1_048_576
    )]
    pub max_retained_output_bytes: usize,

    /// Maximum simultaneously running jobs.
    #[arg(long, env = "RELAY_MAX_RUNNING_JOBS", default_value_t = 16)]
    pub max_running_jobs: usize,

    /// Explicit local-development access to a host Docker daemon socket.
    /// This is intentionally opt-in because Docker daemon access can escape the filesystem sandbox.
    #[arg(long, env = "RELAY_ALLOW_DOCKER", default_value_t = false)]
    pub allow_docker: bool,

    /// Host Docker socket to expose when --allow-docker is enabled.
    #[arg(
        long,
        env = "RELAY_DOCKER_SOCKET",
        default_value = "/var/run/docker.sock"
    )]
    pub docker_socket: String,

    /// Explicit local-development access to the host Tailscale daemon socket.
    #[arg(long, env = "RELAY_ALLOW_TAILSCALE", default_value_t = false)]
    pub allow_tailscale: bool,

    /// Host Tailscale socket to expose when --allow-tailscale is enabled.
    #[arg(
        long,
        env = "RELAY_TAILSCALE_SOCKET",
        default_value = "/var/run/tailscale/tailscaled.sock"
    )]
    pub tailscale_socket: String,

    /// Explicit user-owned toolchain directories added to the safe PATH.
    #[arg(
        long = "toolchain-path",
        env = "RELAY_TOOLCHAIN_PATH",
        value_delimiter = ','
    )]
    pub toolchain_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]
pub enum SecurityMode {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Stop the port-scoped local agent instance.
    Stop {
        /// Port of the running agent to stop.
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
}

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
    pub allow_docker: bool,
    pub docker_socket: String,
    pub allow_tailscale: bool,
    pub tailscale_socket: String,
    pub toolchain_paths: Vec<String>,
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
            allow_docker: false,
            docker_socket: "/var/run/docker.sock".into(),
            allow_tailscale: false,
            tailscale_socket: "/var/run/tailscale/tailscaled.sock".into(),
            toolchain_paths: Vec::new(),
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
            if self.bind_host != "127.0.0.1" && self.bind_host != "::1" {
                return Err(RelayError::InvalidConfig(
                    "remote mode must bind to loopback; terminate HTTPS at a trusted local edge \
                     or secure tunnel and opt in with --trusted-proxy"
                        .into(),
                ));
            }
        }
        if self.trusted_proxy && self.mode != SecurityMode::Remote {
            return Err(RelayError::InvalidConfig(
                "--trusted-proxy is only valid in remote mode".into(),
            ));
        }
        if self.trusted_proxy {
            let Some(cidr) = self.trusted_proxy_cidr.as_deref() else {
                return Err(RelayError::InvalidConfig(
                    "--trusted-proxy requires --trusted-proxy-cidr to identify the edge peer"
                        .into(),
                ));
            };
            cidr.parse::<ipnet::IpNet>().map_err(|_| {
                RelayError::InvalidConfig("--trusted-proxy-cidr must be a valid IP CIDR".into())
            })?;
        } else if self.trusted_proxy_cidr.is_some() {
            return Err(RelayError::InvalidConfig(
                "--trusted-proxy-cidr requires --trusted-proxy".into(),
            ));
        }
        if self.trusted_proxy && self.bind_host != "127.0.0.1" && self.bind_host != "::1" {
            return Err(RelayError::InvalidConfig(
                "trusted proxy mode requires a loopback relay bind; forwarded headers from \
                 public or arbitrary peers are never trusted"
                    .into(),
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
        for path in &self.toolchain_paths {
            let candidate = std::fs::canonicalize(path).map_err(|_| {
                RelayError::InvalidConfig(
                    "toolchain-path must resolve to an existing directory".into(),
                )
            })?;
            if !candidate.is_dir() || !candidate.starts_with(self.resolved_execution_root()?) {
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
            // Remote mode is loopback-first. A local HTTPS edge/tunnel may be
            // explicitly trusted with --trusted-proxy; remote mode never
            // exposes a public plaintext listener by default.
            bind_host: "127.0.0.1".into(),
            default_terminal_timeout_ms: cli.default_terminal_timeout_ms,
            max_terminal_timeout_ms: cli.max_terminal_timeout_ms,
            completed_job_ttl_ms: cli.completed_job_ttl_ms,
            max_retained_output_bytes: cli.max_retained_output_bytes,
            max_running_jobs: cli.max_running_jobs,
            allow_docker: cli.allow_docker,
            docker_socket: cli.docker_socket.clone(),
            allow_tailscale: cli.allow_tailscale,
            tailscale_socket: cli.tailscale_socket.clone(),
            toolchain_paths: cli.toolchain_paths.clone(),
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
