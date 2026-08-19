//! One Bubblewrap construction path for application-owned subprocesses.

use super::{InvocationProgram, ToolInvocation};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;

const MAX_PROTECTED_SCAN_ENTRIES: usize = 200_000;
use tokio::process::{Child, Command};

#[derive(Clone, Copy)]
pub(crate) enum WorkspaceAccess {
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
    workspace_root: Option<&'a Path>,
}

fn runtime_home() -> Result<PathBuf, std::io::Error> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is unavailable"))?;
    if !home.is_absolute() {
        return Err(std::io::Error::other("HOME must be an absolute path"));
    }
    std::fs::canonicalize(home).map_err(|_| std::io::Error::other("HOME is unavailable"))
}

pub(crate) fn safe_path_entries(config: &ServerConfig) -> Vec<PathBuf> {
    const DEFAULT_PATHS: &[&str] = &[
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ];
    let mut entries: Vec<PathBuf> = DEFAULT_PATHS.iter().map(PathBuf::from).collect();
    if let Ok(home) = runtime_home() {
        for sub in [".cargo/bin", ".local/bin"] {
            let dir = home.join(sub);
            if dir.is_dir() {
                entries.push(dir);
            }
        }
    }
    for p in &config.toolchain_paths {
        entries.push(std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)));
    }
    entries
}

pub(crate) fn resolve_safe_executable(
    config: &ServerConfig,
    binary: &str,
) -> Result<PathBuf, McpError> {
    relay_core::terminal_policy::validate_executable(binary, config.allow_docker)?;
    let safe_entries = safe_path_entries(config);
    let mut canonical_safe_entries = safe_entries
        .iter()
        .filter_map(|directory| std::fs::canonicalize(directory).ok())
        .collect::<Vec<_>>();
    for path in &config.toolchain_paths {
        if let Ok(canonical) = std::fs::canonicalize(path) {
            if let Some(root) = super::toolchain::reviewed_root(&canonical) {
                canonical_safe_entries.push(root.to_path_buf());
            }
        }
    }

    for directory in safe_entries {
        let candidate = directory.join(binary);
        if !candidate.is_file() || !is_executable(&candidate) {
            continue;
        }
        let canonical_target = std::fs::canonicalize(&candidate).map_err(|_| {
            McpError::InvalidRequest("configured executable target is unavailable".into())
        })?;
        if !canonical_safe_entries
            .iter()
            .any(|safe_root| canonical_target.starts_with(safe_root))
        {
            return Err(McpError::InvalidRequest(
                "configured executable symlink escapes the reviewed safe PATH roots".into(),
            ));
        }
        // Preserve the reviewed executable path instead of replacing it with
        // the canonical target. Multi-call shims such as rustup dispatch from
        // argv[0] (`cargo`, `rustc`, ...); canonicalizing the final symlink to
        // `rustup` changes program semantics even though the target is safe.
        return Ok(candidate);
    }
    Err(McpError::InvalidRequest(
        "command is not available in the configured safe PATH".into(),
    ))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

pub(super) fn spawn(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    workspace_access: WorkspaceAccess,
) -> Result<Child, std::io::Error> {
    let network_access = if invocation.allow_network || config.allow_terminal_network {
        NetworkAccess::Host
    } else {
        NetworkAccess::Isolated
    };
    let writable = matches!(workspace_access, WorkspaceAccess::Writable);
    spawn_with_profile(
        config,
        invocation,
        SandboxProfile {
            workspace_access,
            network_access,
            expose_optional_sockets: writable,
            expose_runtime_extras: writable,
            workspace_root: None,
        },
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
            cwd: Some(cwd.clone()),
            timeout_ms: 0,
            allow_network: false,
        },
        SandboxProfile {
            workspace_access: WorkspaceAccess::ReadOnly,
            network_access: NetworkAccess::Isolated,
            expose_optional_sockets: false,
            expose_runtime_extras: false,
            workspace_root: Some(&cwd),
        },
    )
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
            cwd: Some(cwd.clone()),
            timeout_ms: 0,
            allow_network: false,
        },
        SandboxProfile {
            workspace_access,
            network_access: NetworkAccess::Isolated,
            expose_optional_sockets: false,
            expose_runtime_extras: false,
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
    let host_home = runtime_home()?;
    let home = host_home.to_string_lossy().into_owned();
    let bwrap = resolve_safe_executable(config, "bwrap")
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let discovered_workspace = if profile.workspace_root.is_none() {
        let cwd_arg = invocation.cwd.as_ref().and_then(|path| path.to_str());
        crate::git::resolve_git_workspace(cwd_arg, config)
            .ok()
            .or_else(|| invocation.cwd.clone())
    } else {
        None
    };
    let configured_workspace = std::fs::canonicalize(config.resolved_dir().unwrap_or_default())
        .map_err(|_| std::io::Error::other("invalid workspace directory"))?;
    let sandbox_root = profile
        .workspace_root
        .or(discovered_workspace.as_deref())
        .unwrap_or(&configured_workspace);
    if sandbox_root == host_home {
        return Err(std::io::Error::other(
            "workspace directory must not be the runtime HOME",
        ));
    }
    if relay_core::protected_paths::is_protected_path(&execution_root, sandbox_root) {
        return Err(std::io::Error::other(
            "sandbox workspace is protected by credential policy",
        ));
    }
    let root = sandbox_root.to_string_lossy().into_owned();
    let root_bind = match profile.workspace_access {
        WorkspaceAccess::ReadOnly => "--ro-bind",
        WorkspaceAccess::Writable => "--bind",
    };
    let mut args = base_bwrap_args(&home, root_bind, &root);
    if matches!(profile.network_access, NetworkAccess::Isolated) {
        args.push("--unshare-net".into());
    }
    if profile.expose_runtime_extras {
        args.extend(["--ro-bind-try".into(), "/opt".into(), "/opt".into()]);
        if !bin_dir.starts_with(sandbox_root) {
            let bin = bin_dir.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), bin.clone(), bin]);
        }
    }
    let mut cargo_home = None;
    let mut rustup_home = None;
    let home_cargo_bin = host_home.join(".cargo/bin");
    let canonical_home_cargo_bin = std::fs::canonicalize(&home_cargo_bin).ok();
    for path in &config.toolchain_paths {
        let configured = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&configured)
            .map_err(|_| std::io::Error::other("invalid toolchain path"))?;
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
        if let Some(toolchain_root) = super::toolchain::reviewed_root(&canonical) {
            let value = toolchain_root.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), value.clone(), value]);
        }
        if canonical_home_cargo_bin.as_ref() == Some(&canonical) {
            let candidate = host_home.join(".cargo");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                for file in ["credentials", "credentials.toml"] {
                    mask_protected_file(&mut args, &candidate.join(file))?;
                }
                cargo_home = Some(value);
            }
            let candidate = host_home.join(".rustup");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                rustup_home = Some(value);
            }
        }
    }
    if profile.expose_optional_sockets {
        for (enabled, socket, name) in [
            (config.allow_docker, &config.docker_socket, "Docker"),
            (
                config.allow_tailscale,
                &config.tailscale_socket,
                "Tailscale",
            ),
        ] {
            add_optional_socket(&mut args, enabled, socket, name)?;
        }
    }
    add_protected_paths(&mut args, sandbox_root, true)?;
    if sandbox_root != execution_root {
        add_protected_paths(&mut args, &execution_root, false)?;
    }
    let _ = config.ensure_workspaces_initialized();
    if let Ok(guard) = config.workspaces.read() {
        for ws in guard.all_roots() {
            if ws != sandbox_root
                && ws != host_home
                && !relay_core::protected_paths::is_protected_path(&execution_root, &ws)
            {
                let val = ws.to_string_lossy().into_owned();
                args.extend([root_bind.into(), val.clone(), val]);
                add_protected_paths(&mut args, &ws, false)?;
            }
        }
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
    if let Some(rustup_home) = rustup_home {
        command.env("RUSTUP_HOME", rustup_home);
    }
    match (profile.workspace_root.is_some(), cargo_home) {
        (true, _) => {
            command
                .env("CARGO_HOME", "/tmp/lsp-home/.cargo")
                .env("CARGO_TARGET_DIR", "/tmp/lsp-target");
        }
        (false, Some(cargo_home)) => {
            command.env("CARGO_HOME", cargo_home);
        }
        _ => {}
    }
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
}

fn mask_protected_file(args: &mut Vec<String>, path: &Path) -> Result<(), std::io::Error> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "protected toolchain credential path is a symbolic link",
        ));
    }
    if metadata.is_file() {
        args.extend([
            "--ro-bind".into(),
            "/dev/null".into(),
            path.to_string_lossy().into_owned(),
        ]);
    }
    Ok(())
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

fn add_protected_paths(
    args: &mut Vec<String>,
    execution_root: &Path,
    recursive: bool,
) -> Result<(), std::io::Error> {
    let paths = if recursive {
        discover_protected_paths(execution_root)?
    } else {
        relay_core::protected_paths::protected_paths(execution_root).collect()
    };
    for path in paths {
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            return Err(std::io::Error::other(
                "protected sandbox path is a symbolic link",
            ));
        }
        if metadata.is_dir() {
            args.extend(["--tmpfs".into(), path.to_string_lossy().into_owned()]);
        } else {
            args.extend([
                "--ro-bind".into(),
                "/dev/null".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
    }
    Ok(())
}

fn discover_protected_paths(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut protected = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut scanned = 0usize;
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            scanned = scanned.saturating_add(1);
            if scanned > MAX_PROTECTED_SCAN_ENTRIES {
                return Err(std::io::Error::other(
                    "protected-path scan exceeds bounded workspace maximum",
                ));
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| std::io::Error::other("protected-path scan escaped workspace"))?;
            if relay_core::protected_paths::is_protected_relative(relative) {
                protected.push(path);
            } else if entry.file_type()?.is_dir() {
                stack.push(path);
            }
        }
    }
    Ok(protected)
}

fn base_bwrap_args(home: &str, root_bind: &str, root: &str) -> Vec<String> {
    let mut args = Vec::with_capacity(32);
    for p in ["/usr", "/lib"] {
        args.extend(["--ro-bind".into(), p.into(), p.into()]);
    }
    for p in ["/lib64", "/etc", "/bin", "/sbin"] {
        args.extend(["--ro-bind-try".into(), p.into(), p.into()]);
    }
    for (flag, p) in [("--dev", "/dev"), ("--proc", "/proc"), ("--tmpfs", "/tmp")] {
        args.extend([flag.into(), p.into()]);
    }
    args.extend([
        "--dir".into(),
        home.into(),
        root_bind.into(),
        root.into(),
        root.into(),
        "--unshare-pid".into(),
        "--die-with-parent".into(),
    ]);
    args
}
