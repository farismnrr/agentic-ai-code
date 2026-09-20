use super::{
    activity, blender, creative, lsp, parse_host_authority, ssh, SecurityMode, ServerConfig,
};
use crate::core::error::RelayError;

impl ServerConfig {
    /// Validate configuration before binding. Never broadens trust (e.g.
    /// never rewrites an origin into a wildcard or a laxer form).
    pub fn validate(&self) -> Result<(), RelayError> {
        if self.port == 0 {
            return Err(RelayError::InvalidConfig(
                "port must be non-zero".to_string(),
            ));
        }
        if !(1..=60_000).contains(&self.default_terminal_timeout_ms) {
            return Err(RelayError::InvalidConfig(
                "default terminal timeout must be between 1 and 60000 ms".into(),
            ));
        }
        if !(1..=60_000).contains(&self.max_terminal_timeout_ms) {
            return Err(RelayError::InvalidConfig(
                "maximum terminal timeout must be between 1 and 60000 ms".into(),
            ));
        }
        if self.default_terminal_timeout_ms > self.max_terminal_timeout_ms {
            return Err(RelayError::InvalidConfig(
                "default terminal timeout must not exceed maximum terminal timeout".into(),
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
        creative::validate(self)?;
        blender::validate(self)?;
        activity::validate(&self.activity)?;
        if self.allow_ssh {
            if self
                .ssh_config
                .as_deref()
                .is_some_and(|path| !std::path::Path::new(path).is_absolute())
            {
                return Err(RelayError::InvalidConfig(
                    "ssh-config must be an absolute path beneath the approved SSH credential root"
                        .into(),
                ));
            }
            let _ = self.resolved_ssh_root()?;
            let _ = self.resolved_ssh_config()?;
            for (label, value) in [
                ("ssh-readonly-db-user", self.ssh_readonly_db_user.as_deref()),
                (
                    "ssh-readonly-redis-user",
                    self.ssh_readonly_redis_user.as_deref(),
                ),
            ] {
                if let Some(value) = value {
                    if !ssh::valid_diagnostic_principal(value) {
                        return Err(RelayError::InvalidConfig(format!(
                            "{label} must be a bounded database identity name"
                        )));
                    }
                }
            }
            if self.ssh_readonly_redis_user.is_some()
                != self.ssh_readonly_redis_password_file.is_some()
            {
                return Err(RelayError::InvalidConfig(
                    "ssh-readonly-redis-user and ssh-readonly-redis-password-file must be configured together"
                        .into(),
                ));
            }
            let _ = self.resolved_ssh_redis_password_file()?;
        } else if self.ssh_root.is_some()
            || self.ssh_config.is_some()
            || self.ssh_readonly_db_user.is_some()
            || self.ssh_readonly_redis_user.is_some()
            || self.ssh_readonly_redis_password_file.is_some()
        {
            return Err(RelayError::InvalidConfig(
                "SSH-specific configuration requires --allow-ssh".into(),
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
        for (kind, paths) in [
            ("toolchain-path", &self.toolchain_paths),
            ("toolchain-state-path", &self.toolchain_state_paths),
        ] {
            for path in paths {
                let candidate = std::fs::canonicalize(path).map_err(|_| {
                    RelayError::InvalidConfig(format!(
                        "{kind} must resolve to an existing directory"
                    ))
                })?;
                // Explicit toolchain paths and state directories are mounted
                // read-only by the sandbox and are operator-approved exceptions
                // to the workspace boundary.
                let metadata = candidate.metadata().map_err(|_| {
                    RelayError::InvalidConfig(format!("{kind} metadata is unavailable"))
                })?;
                if !metadata.is_dir() {
                    return Err(RelayError::InvalidConfig(format!(
                        "{kind} must be an existing directory"
                    )));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::{MetadataExt, PermissionsExt};
                    if metadata.uid() != unsafe { libc::geteuid() }
                        || metadata.permissions().mode() & 0o022 != 0
                    {
                        return Err(RelayError::InvalidConfig(format!(
                            "{kind} must be owner-controlled and not writable by group/other"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}
