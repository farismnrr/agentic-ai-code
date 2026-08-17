//! One Bubblewrap construction path for every application process.

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

pub(super) fn safe_path_entries(config: &ServerConfig) -> Vec<PathBuf> {
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

pub(super) fn resolve_safe_executable(
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
    let root = execution_root.to_string_lossy().into_owned();
    let root_bind = match workspace_access {
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
    // Keep the restored pre-refactor sandbox surfaces after centralization:
    // general execution may read /opt and the relay binary directory, while
    // read-only text search receives neither mount. This distinction prevents
    // the shared Bubblewrap builder from broadening the search tool's readable
    // host surface.
    if matches!(workspace_access, WorkspaceAccess::Writable) {
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
    }
    if matches!(workspace_access, WorkspaceAccess::Writable) {
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
    add_protected_paths(&mut args, &execution_root);
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
    let mut command = Command::new(bwrap);
    command
        .args(args)
        .env_clear()
        .env("HOME", root)
        .env("PATH", safe_path)
        .env("LANG", "C.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
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
    for relative in [".ssh", ".aws", ".config/gcloud", ".docker", ".kube"] {
        let path = execution_root.join(relative);
        if path.exists() {
            args.extend(["--tmpfs".into(), path.to_string_lossy().into_owned()]);
        }
    }
    for relative in [
        ".npmrc",
        ".netrc",
        ".pypirc",
        ".cargo/credentials",
        ".cargo/credentials.toml",
    ] {
        let path = execution_root.join(relative);
        if path.exists() {
            args.extend([
                "--ro-bind".into(),
                "/dev/null".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
    }
}
