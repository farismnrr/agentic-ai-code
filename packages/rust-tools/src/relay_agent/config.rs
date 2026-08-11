//! CLI contract and server configuration for `relay-agent`.
//!
//! The flag set here matches the legacy Node CLI (`packages/relay-agent/bin/cli.mjs`)
//! exactly, per the frozen audit in `.agents/plans/028-phase0-contract-audit.md`:
//! `--dir`/`-d`, `--port`/`-p` (default `47821`), `--origin`/`-o` (env fallback
//! `RELAY_AGENT_ORIGIN`), and a `stop --port <port>` subcommand.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use super::error::RelayError;

pub const DEFAULT_PORT: u16 = 47_821;

/// Top-level CLI, matching the legacy `relay-agent [--port] [--dir] [--origin]`
/// and `relay-agent stop --port <port>` invocations.
#[derive(Parser, Debug)]
#[command(name = "relay-agent", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Port to run the server on.
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// Explicit security mode: local (loopback only) or remote (OAuth required).
    #[arg(long, value_enum, env = "RELAY_AGENT_MODE", default_value = "local")]
    pub mode: SecurityMode,

    /// Default working directory configuration, not a filesystem sandbox (falls back to the OS home directory).
    #[arg(short, long)]
    pub dir: Option<String>,

    /// Allowed Nuxt/browser origin for MCP requests.
    #[arg(short, long, env = "RELAY_AGENT_ORIGIN")]
    pub origin: Option<String>,

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
    pub oauth_secret: Option<String>,
    pub oauth_issuer: Option<String>,
    pub oauth_audience: Option<String>,
    pub oauth_owner_subject: Option<String>,
    pub execution_root: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            mode: SecurityMode::Local,
            dir: None,
            origin: None,
            oauth_secret: None,
            oauth_issuer: None,
            oauth_audience: None,
            oauth_owner_subject: None,
            execution_root: None,
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
        // Also reject shallow roots (depth < 3 components e.g. /home/user is allowed, / is not).
        // This catches unnamed top-level dirs that aren't in the explicit list above.
        let depth = canonical.components().count();
        if depth < 3 {
            return Err(RelayError::InvalidConfig(format!(
                "execution root '{}' is too shallow (depth {}). \
                 Use a directory at least 2 levels deep (e.g. /home/user/project).",
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
            dir: cli.dir.clone(),
            origin: cli.origin.clone(),
            oauth_secret: cli.oauth_secret.clone(),
            oauth_issuer: cli.oauth_issuer.clone(),
            oauth_audience: cli.oauth_audience.clone(),
            oauth_owner_subject: cli.oauth_owner_subject.clone(),
            execution_root: cli.execution_root.clone(),
        }
    }
}
