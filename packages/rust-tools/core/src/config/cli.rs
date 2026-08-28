//! Relay CLI declaration and command-line security-mode contract.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 47_821;

/// Top-level CLI, matching the legacy `relay-agent [--port] [--dir] [--origin]`
/// and `relay-agent stop --port <port>` invocations, with explicit bind-host
/// control for deployments that need a non-loopback listener.
#[derive(Parser, Debug)]
#[command(name = "ai-tools relay-agent", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Port to run the server on.
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// IP address on which to listen. Defaults to loopback; local mode only
    /// permits loopback, while remote non-loopback binds require OAuth and an
    /// explicit browser Origin.
    #[arg(long, env = "RELAY_AGENT_BIND_HOST", default_value = "127.0.0.1")]
    pub bind_host: String,

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

    /// Allow terminal subprocesses to use the host network namespace.
    /// Disabled by default; dedicated HTTP/search tools remain separate capabilities.
    #[arg(long, env = "RELAY_ALLOW_TERMINAL_NETWORK", default_value_t = false)]
    pub allow_terminal_network: bool,

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

    /// Operator-approved language-server executable mappings. Values are
    /// `language=executable`; executable resolution is restricted to the relay
    /// safe PATH and no repository file can supply command arguments.
    #[arg(long = "lsp-server", env = "RELAY_LSP_SERVER", value_delimiter = ',')]
    pub lsp_servers: Vec<String>,

    /// MCP tool exposure profile. Full is the canonical superset; Primary is the fast-path subset.
    #[arg(long, value_enum, env = "RELAY_TOOL_PROFILE", default_value = "full")]
    pub tool_profile: ToolProfile,

    /// Explicitly enable the repository-owned deterministic lifecycle hooks.
    /// Hook configuration is never trusted merely because it exists.
    #[arg(long, env = "RELAY_ENABLE_AGENT_HOOKS", default_value_t = false)]
    pub enable_agent_hooks: bool,

    /// Optional contained path to the repository-owned hook configuration.
    #[arg(long, env = "RELAY_AGENT_HOOKS_CONFIG")]
    pub agent_hooks_config: Option<String>,

    /// Workspace activity capture mode. Off preserves the pre-activity
    /// compatibility behavior; required durably admits workspace operations
    /// before execution.
    #[arg(long, env = "RELAY_ACTIVITY_MODE", default_value = "off")]
    pub activity_mode: ActivityMode,

    /// Owner-only relay activity state directory. When omitted, the relay
    /// uses the operator's XDG state directory outside the workspace.
    #[arg(long, env = "RELAY_ACTIVITY_STATE_DIR")]
    pub activity_state_dir: Option<String>,

    /// HTTPS Nuxt activity ingestion endpoint. The exporter is asynchronous.
    #[arg(long, env = "RELAY_ACTIVITY_SINK_URL")]
    pub activity_sink_url: Option<String>,

    /// Scoped activity source bearer credential. It is never presented in
    /// diagnostics or activity metadata.
    #[arg(long, env = "RELAY_ACTIVITY_SOURCE_TOKEN")]
    pub activity_source_token: Option<String>,

    /// Maximum encrypted journal bytes, including SQLite WAL overhead.
    #[arg(long, env = "RELAY_ACTIVITY_SPOOL_QUOTA_BYTES", default_value_t = 64 * 1024 * 1024)]
    pub activity_spool_quota_bytes: u64,

    /// How long acknowledged local rows remain available for recovery.
    #[arg(
        long,
        env = "RELAY_ACTIVITY_ACK_RETENTION_MS",
        default_value_t = 86_400_000
    )]
    pub activity_ack_retention_ms: u64,

    /// Enable the fixed-recipient task completion Telegram notifier. The
    /// bot token and channel are imported from Hermes' owner-only .env and
    /// stored in the relay's encrypted local database.
    #[arg(long, env = "RELAY_TELEGRAM_ENABLED", default_value_t = false)]
    pub telegram_enabled: bool,

    /// Optional Hermes .env path. Defaults to `$HOME/.hermes/.env`.
    #[arg(long, env = "RELAY_TELEGRAM_HERMES_ENV")]
    pub telegram_hermes_env: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolProfile {
    Full,
    Primary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]
pub enum SecurityMode {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityMode {
    Off,
    Required,
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
