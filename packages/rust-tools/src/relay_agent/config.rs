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

    /// Default working directory configuration, not a filesystem sandbox (falls back to the OS home directory).
    #[arg(short, long)]
    pub dir: Option<String>,

    /// Allowed Nuxt/browser origin for MCP requests.
    #[arg(short, long, env = "RELAY_AGENT_ORIGIN")]
    pub origin: Option<String>,

    /// OAuth symmetric secret for validating JWTs
    #[arg(long, env = "OAUTH_SECRET")]
    pub oauth_secret: Option<String>,

    /// OAuth issuer expected in JWT 'iss' claim
    #[arg(long, env = "OAUTH_ISSUER")]
    pub oauth_issuer: Option<String>,

    /// OAuth audience expected in JWT 'aud' claim
    #[arg(long, env = "OAUTH_AUDIENCE")]
    pub oauth_audience: Option<String>,
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
    pub dir: Option<String>,
    pub origin: Option<String>,
    pub oauth_secret: Option<String>,
    pub oauth_issuer: Option<String>,
    pub oauth_audience: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            dir: None,
            origin: None,
            oauth_secret: None,
            oauth_issuer: None,
            oauth_audience: None,
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
        if self.oauth_secret.is_some()
            && (self.oauth_issuer.is_none() || self.oauth_audience.is_none())
        {
            return Err(RelayError::InvalidConfig(
                "oauth_issuer and oauth_audience are required when oauth_secret is set".to_string(),
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
            dir: cli.dir.clone(),
            origin: cli.origin.clone(),
            oauth_secret: cli.oauth_secret.clone(),
            oauth_issuer: cli.oauth_issuer.clone(),
            oauth_audience: cli.oauth_audience.clone(),
        }
    }
}
