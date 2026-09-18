use super::{
    spawn_with_profile, InvocationProgram, InvocationSecurity, NetworkAccess, SandboxError,
    SandboxProfile, SpawnControl, ToolInvocation, WorkspaceAccess, WorkspaceMount,
};
use crate::core::config::ServerConfig;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::watch;

pub(in crate::application::execution) fn spawn(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    workspace_access: WorkspaceAccess,
    deadline: Option<Instant>,
    cancel: &watch::Receiver<bool>,
) -> Result<Child, SandboxError> {
    // SSH is a distinct execution class: it gets host networking and exact
    // reviewed credential files, but no host-backed workspace or local privileged
    // sockets. Dedicated network tools use the same hidden-workspace boundary.
    let ssh = matches!(invocation.security, InvocationSecurity::Ssh { .. });
    let network_only = matches!(invocation.security, InvocationSecurity::NetworkOnly);
    let network_access = if ssh || invocation.allow_network {
        NetworkAccess::Host
    } else {
        NetworkAccess::Isolated
    };
    let effective_workspace_access = if ssh || network_only {
        WorkspaceAccess::ReadOnly
    } else {
        workspace_access
    };
    let writable = matches!(effective_workspace_access, WorkspaceAccess::Writable);
    spawn_with_profile(
        config,
        invocation,
        SandboxProfile {
            workspace_access: effective_workspace_access,
            network_access,
            expose_optional_sockets: !ssh
                && !network_only
                && writable
                && invocation.expose_optional_sockets,
            expose_runtime_extras: !ssh && !network_only && writable,
            workspace_mount: if ssh || network_only {
                WorkspaceMount::Hidden
            } else {
                WorkspaceMount::Authorized
            },
        },
        Some(&SpawnControl { deadline, cancel }),
    )
}

/// Spawn an approved language server with a stricter profile than ordinary
/// terminal execution: read-only workspace, isolated network namespace, no
/// Docker/Tailscale sockets, the runtime-resolved HOME path without a whole-home
/// bind, a cleared environment, and only the relay safe PATH/toolchain mounts.
pub(crate) fn spawn_lsp(
    config: &ServerConfig,
    executable: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
) -> Result<Child, std::io::Error> {
    spawn_with_profile(
        config,
        &ToolInvocation {
            program: InvocationProgram::Direct(executable),
            args,
            stdin_file: None,
            cwd: Some(cwd.clone()),
            timeout_ms: 0,
            allow_network: false,
            expose_optional_sockets: false,
            readonly_docker_socket: false,
            expose_authorized_siblings: false,
            security: InvocationSecurity::Standard,
        },
        SandboxProfile {
            workspace_access: WorkspaceAccess::ReadOnly,
            network_access: NetworkAccess::Isolated,
            expose_optional_sockets: false,
            expose_runtime_extras: false,
            workspace_mount: WorkspaceMount::Fixed(&cwd),
        },
        None,
    )
    .map_err(SandboxError::into_io_error)
}

/// Hook profile: contained repository cwd, with workspace authority capped by
/// the triggering operation. A read-only lifecycle event is never given a
/// writable bind merely because a hook is configured.
pub(crate) fn spawn_hook(
    config: &ServerConfig,
    executable: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    workspace_access: WorkspaceAccess,
) -> Result<Child, std::io::Error> {
    spawn_with_profile(
        config,
        &ToolInvocation {
            program: InvocationProgram::Direct(executable),
            args,
            stdin_file: None,
            cwd: Some(cwd.clone()),
            timeout_ms: 0,
            allow_network: false,
            expose_optional_sockets: false,
            readonly_docker_socket: false,
            expose_authorized_siblings: false,
            security: InvocationSecurity::Standard,
        },
        SandboxProfile {
            workspace_access,
            network_access: NetworkAccess::Isolated,
            expose_optional_sockets: false,
            expose_runtime_extras: false,
            workspace_mount: WorkspaceMount::Fixed(&cwd),
        },
        None,
    )
    .map_err(SandboxError::into_io_error)
}
