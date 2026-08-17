//! One Bubblewrap construction path for application-owned subprocesses.

use super::{InvocationProgram, ToolInvocation};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Clone, Copy)]
pub(super) enum WorkspaceAccess {
    ReadOnly,
    Writable,
}

#[derive(Clone, Copy)]
enum NetworkAccess {
    Host,
    Isolated,
}

struct SandboxProfile<'a> {
    workspace_access: WorkspaceAccess,
    network_access: NetworkAccess,
    expose_optional_sockets: bool,
    expose_runtime_extras: bool,
    home: &'a str,
    workspace_root: Option<&'a Path>,
}

pub(crate) fn safe_path_entries(config: &ServerConfig) -> Vec<PathBuf> {
    let mut entries = [
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    entries.extend(config.toolchain_paths.iter().map(PathBuf::from));
    entries
}

pub(crate) fn resolve_safe_executable(
    config: &ServerConfig,
    binary: &str,
) -> Result<PathBuf, McpError> {
    relay_core::terminal_policy::validate_executable(binary, config.allow_docker)?;
    safe_path_entries(config)
        .into_iter()
        .map(|directory| directory.join(binary))
        .find(|candidate| candidate.is_file() && is_executable(candidate))
        .ok_or_else(|| {
            McpError::InvalidRequest("command is not available in the configured safe PATH".into())
        })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

pub(super) fn spawn(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    workspace_access: WorkspaceAccess,
) -> Result<Child, std::io::Error> {
    spawn_with_profile(
        config,
        invocation,
        SandboxProfile {
            workspace_access,
            network_access: if config.allow_terminal_network {
                NetworkAccess::Host
            } else {
                NetworkAccess::Isolated
            },
            expose_optional_sockets: matches!(workspace_access, WorkspaceAccess::Writable),
            expose_runtime_extras: matches!(workspace_access, WorkspaceAccess::Writable),
            home: "execution_root",
            workspace_root: None,
        },
    )
}

/// Spawn an approved language server with a stricter profile than ordinary
/// terminal execution: read-only workspace, isolated network namespace, no
/// Docker/Tailscale sockets, a temporary HOME, cleared environment, and only
/// the relay safe PATH/toolchain mounts.
pub(crate) fn spawn_lsp(
    config: &ServerConfig,
    executable: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
) -> Result<Child, std::io::Error> {
    let invocation = ToolInvocation {
        program: InvocationProgram::Direct(executable),
        args,
        cwd: Some(cwd.clone()),
        timeout_ms: 0,
    };
    spawn_with_profile(
        config,
        &invocation,
        SandboxProfile {
            workspace_access: WorkspaceAccess::ReadOnly,
            network_access: NetworkAccess::Isolated,
            expose_optional_sockets: false,
            expose_runtime_extras: false,
            home: "/tmp/lsp-home",
            workspace_root: Some(&cwd),
        },
    )
}

fn spawn_with_profile(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    profile: SandboxProfile<'_>,
) -> Result<Child, std::io::Error> {
    let current_exe = env::current_exe()?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| std::io::Error::other("missing binary directory"))?
        .to_path_buf();
    let program_path = match &invocation.program {
        InvocationProgram::SelfBinary => current_exe,
        InvocationProgram::Direct(path) => path.clone(),
    };
    if !program_path.exists() {
        return Err(std::io::Error::other("tool binary unavailable"));
    }
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| std::io::Error::other("invalid execution root"))?;
    let bwrap = resolve_safe_executable(config, "bwrap")
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let sandbox_root = profile.workspace_root.unwrap_or(&execution_root);
    let root = sandbox_root.to_string_lossy().into_owned();
    let root_bind = match profile.workspace_access {
        WorkspaceAccess::ReadOnly => "--ro-bind",
        WorkspaceAccess::Writable => "--bind",
    };
    let mut args = vec![
        "--ro-bind",
        "/usr",
        "/usr",
        "--ro-bind",
        "/lib",
        "/lib",
        "--ro-bind-try",
        "/lib64",
        "/lib64",
        "--ro-bind-try",
        "/etc",
        "/etc",
        "--ro-bind-try",
        "/bin",
        "/bin",
        "--ro-bind-try",
        "/sbin",
        "/sbin",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        root_bind,
        root.as_str(),
        root.as_str(),
        "--unshare-pid",
        "--die-with-parent",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    if matches!(profile.network_access, NetworkAccess::Isolated) {
        args.push("--unshare-net".into());
    }
    if profile.home != "execution_root" {
        args.extend(["--dir".into(), profile.home.into()]);
    }
    if profile.expose_runtime_extras {
        args.extend([
            "--ro-bind-try".into(),
            "/opt".into(),
            "/opt".into(),
            "--ro-bind".into(),
            bin_dir.to_string_lossy().into_owned(),
            bin_dir.to_string_lossy().into_owned(),
        ]);
    }
    for path in &config.toolchain_paths {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| std::io::Error::other("invalid toolchain path"))?;
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
        if canonical.file_name() == Some(std::ffi::OsStr::new("bin")) {
            if let Some(toolchain_root) = canonical.parent() {
                if toolchain_root.join("lib/rustlib").is_dir() {
                    let value = toolchain_root.to_string_lossy().into_owned();
                    args.extend(["--ro-bind".into(), value.clone(), value]);
                }
            }
        }
    }
    if profile.expose_optional_sockets {
        add_optional_socket(
            &mut args,
            config.allow_docker,
            &config.docker_socket,
            "Docker",
        )?;
        add_optional_socket(
            &mut args,
            config.allow_tailscale,
            &config.tailscale_socket,
            "Tailscale",
        )?;
    }
    add_protected_paths(&mut args, sandbox_root);
    if sandbox_root != execution_root {
        add_protected_paths(&mut args, &execution_root);
    }
    if let Some(cwd) = &invocation.cwd {
        args.extend(["--chdir".into(), cwd.to_string_lossy().into_owned()]);
    }
    args.push(program_path.to_string_lossy().into_owned());
    args.extend(invocation.args.clone());
    let safe_path = safe_path_entries(config)
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");
    let home = if profile.home == "execution_root" {
        root
    } else {
        profile.home.into()
    };
    let mut command = Command::new(bwrap);
    command
        .args(args)
        .env_clear()
        .env("HOME", home)
        .env("PATH", safe_path)
        .env("LANG", "C.UTF-8")
        .env("TMPDIR", "/tmp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if profile.workspace_root.is_some() {
        command
            .env("CARGO_HOME", "/tmp/lsp-home/.cargo")
            .env("CARGO_TARGET_DIR", "/tmp/lsp-target");
    }
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
}

fn add_optional_socket(
    args: &mut Vec<String>,
    enabled: bool,
    configured_path: &str,
    name: &str,
) -> Result<(), std::io::Error> {
    if !enabled {
        return Ok(());
    }
    let socket = Path::new(configured_path);
    if !socket.exists() {
        return Err(std::io::Error::other(format!(
            "{name} access enabled but socket '{}' is unavailable",
            socket.display()
        )));
    }
    let value = socket.to_string_lossy().into_owned();
    args.extend(["--bind".into(), value.clone(), value]);
    Ok(())
}

fn add_protected_paths(args: &mut Vec<String>, execution_root: &Path) {
    for path in relay_core::protected_paths::protected_paths(execution_root) {
        if path.exists() {
            let is_directory = path.is_dir();
            if is_directory {
                args.extend(["--tmpfs".into(), path.to_string_lossy().into_owned()]);
            } else {
                args.extend([
                    "--ro-bind".into(),
                    "/dev/null".into(),
                    path.to_string_lossy().into_owned(),
                ]);
            }
        }
    }
}
