use super::agent_policy::AgentProvider;
use super::sandbox;
use relay_core::config::ServerConfig;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const AUTH_PROBE_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Debug, Clone, Default)]
pub struct AgentCapabilities {
    providers: Vec<AgentProvider>,
}

impl AgentCapabilities {
    pub fn contains(&self, provider: AgentProvider) -> bool {
        self.providers.contains(&provider)
    }

    pub fn providers(&self) -> &[AgentProvider] {
        &self.providers
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.providers
            .iter()
            .map(|provider| provider.name())
            .collect()
    }
}

/// Discover only providers that can be invoked without an interactive login.
/// The check runs once while the relay router is built; a restart refreshes a
/// login/logout change and keeps request handling free of subprocess probes.
pub fn detect_agent_capabilities(config: &ServerConfig) -> AgentCapabilities {
    let providers = [
        AgentProvider::Codex,
        AgentProvider::Antigravity,
        AgentProvider::Claude,
    ]
    .into_iter()
    .filter(|provider| provider_is_available(config, *provider))
    .collect();
    AgentCapabilities { providers }
}

fn provider_is_available(config: &ServerConfig, provider: AgentProvider) -> bool {
    let Ok(program) = sandbox::resolve_safe_executable(config, provider.binary()) else {
        return false;
    };
    let environment = config.agent_environment_for(provider.name());
    if let Some(argv) = provider.auth_probe_argv() {
        if run_auth_probe(config, &program, argv, &environment) {
            return true;
        }
    }
    // This CLI has no documented side-effect-free auth-status command. Only
    // an explicit, narrow auth-root mapping can opt it into the catalog; an
    // environment variable alone must never make an unverified provider look
    // available.
    provider == AgentProvider::Antigravity && config.has_explicit_agent_auth_root(provider.name())
}

fn run_auth_probe(
    config: &ServerConfig,
    program: &std::path::Path,
    argv: &[&str],
    environment: &[(String, String)],
) -> bool {
    let Ok(home) = sandbox::runtime_home() else {
        return false;
    };
    let path = sandbox::safe_path_entries(config)
        .into_iter()
        .map(|entry| entry.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");
    let mut command = Command::new(program);
    command
        .args(argv)
        .env_clear()
        .env("HOME", home)
        .env("PATH", path)
        .env("LANG", "C.UTF-8")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (name, value) in environment {
        command.env(name, value);
    }
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    wait_bounded(&mut child, AUTH_PROBE_TIMEOUT)
}

fn wait_bounded(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}
