//! One Bubblewrap construction path for application-owned subprocesses.

use super::{InvocationProgram, InvocationSecurity, ToolInvocation};
use crate::core::config::ServerConfig;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use tokio::process::{Child, Command};
use tokio::sync::watch;

mod masks;
mod paths;
use masks::{add_optional_socket, add_protected_paths};
mod managed_process;
mod protected_paths;
mod ssh_material;
mod toolchain_mounts;

pub(super) use managed_process::spawn;
pub(crate) use managed_process::{spawn_hook, spawn_lsp};
pub(crate) use protected_paths::prime_protected_path_indexes;

#[derive(Clone, Copy)]
pub(crate) enum WorkspaceAccess {
    ReadOnly,
    Writable,
}

pub(crate) struct SpawnControl<'a> {
    deadline: Option<Instant>,
    cancel: &'a watch::Receiver<bool>,
}

impl SpawnControl<'_> {
    pub(crate) fn check(&self) -> Result<(), io::Error> {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "terminal execution deadline elapsed",
            ));
        }
        if *self.cancel.borrow() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "terminal execution was cancelled",
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct SandboxError {
    pub(crate) stage: &'static str,
    source: io::Error,
}

impl SandboxError {
    fn at(stage: &'static str, source: io::Error) -> Self {
        Self { stage, source }
    }

    pub(crate) fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }

    fn into_io_error(self) -> io::Error {
        self.source
    }
}

impl From<io::Error> for SandboxError {
    fn from(source: io::Error) -> Self {
        Self::at("sandbox_profile", source)
    }
}

#[derive(Clone, Copy)]
enum NetworkAccess {
    Host,
    Isolated,
}

#[derive(Clone, Copy)]
enum WorkspaceMount<'a> {
    Authorized,
    Fixed(&'a Path),
    Hidden,
}

struct SandboxProfile<'a> {
    workspace_access: WorkspaceAccess,
    network_access: NetworkAccess,
    expose_optional_sockets: bool,
    expose_runtime_extras: bool,
    workspace_mount: WorkspaceMount<'a>,
}

pub(crate) use super::toolchain::{resolve_safe_executable, safe_path_entries};

pub(crate) fn runtime_home() -> Result<PathBuf, std::io::Error> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is unavailable"))?;
    if !home.is_absolute() {
        return Err(std::io::Error::other("HOME must be an absolute path"));
    }
    std::fs::canonicalize(home).map_err(|_| std::io::Error::other("HOME is unavailable"))
}

fn spawn_with_profile(
    config: &ServerConfig,
    invocation: &ToolInvocation,
    profile: SandboxProfile<'_>,
    control: Option<&SpawnControl<'_>>,
) -> Result<Child, SandboxError> {
    let resolution_started = Instant::now();
    let current_exe =
        env::current_exe().map_err(|error| SandboxError::at("executable_resolution", error))?;
    let program_path = match &invocation.program {
        InvocationProgram::SelfBinary => current_exe,
        InvocationProgram::Direct(path) => path.clone(),
    };
    if !program_path.exists() {
        return Err(SandboxError::at(
            "executable_resolution",
            io::Error::new(io::ErrorKind::NotFound, "tool binary unavailable"),
        ));
    }
    let workspace_hidden = matches!(profile.workspace_mount, WorkspaceMount::Hidden);
    let execution_root = if workspace_hidden {
        None
    } else {
        Some(
            config
                .resolved_execution_root()
                .map_err(|_| std::io::Error::other("invalid execution root"))?,
        )
    };
    let host_home =
        runtime_home().map_err(|error| SandboxError::at("executable_resolution", error))?;
    let sandbox_home = if workspace_hidden {
        "/tmp/home".to_owned()
    } else {
        host_home.to_string_lossy().into_owned()
    };
    let bwrap = resolve_safe_executable(config, "bwrap").map_err(|_| {
        SandboxError::at(
            "executable_resolution",
            io::Error::new(io::ErrorKind::NotFound, "sandbox executable unavailable"),
        )
    })?;
    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "executable_resolution",
        outcome = "completed",
        duration_ms = resolution_started.elapsed().as_millis() as u64,
    );
    let profile_started = Instant::now();
    if let Some(control) = control {
        control.check()?;
    }
    let discovered_repository = if matches!(profile.workspace_mount, WorkspaceMount::Authorized) {
        let cwd_arg = invocation.cwd.as_ref().and_then(|path| path.to_str());
        crate::application::git::resolve_git_workspace(cwd_arg, config).ok()
    } else {
        None
    };
    let discovered_workspace = if matches!(profile.workspace_mount, WorkspaceMount::Authorized) {
        discovered_repository
            .clone()
            .or_else(|| invocation.cwd.clone())
    } else {
        None
    };
    let configured_workspace = if workspace_hidden {
        None
    } else {
        Some(
            std::fs::canonicalize(config.resolved_dir().unwrap_or_default())
                .map_err(|_| std::io::Error::other("invalid workspace directory"))?,
        )
    };
    // The containing allowlist root remains the authorization boundary. A
    // repository discovered beneath it may only narrow the terminal mount;
    // it can never grant access outside that containing root.
    if !workspace_hidden {
        config
            .ensure_workspaces_initialized()
            .map_err(|_| std::io::Error::other("workspace authority is unavailable"))?;
    }
    let authorized_root = if matches!(profile.workspace_mount, WorkspaceMount::Authorized)
        && invocation.expose_authorized_siblings
        && !matches!(invocation.security, InvocationSecurity::Ssh { .. })
    {
        let configured_workspace = configured_workspace
            .as_deref()
            .ok_or_else(|| std::io::Error::other("workspace authority is unavailable"))?;
        let guard = config
            .workspaces
            .read()
            .map_err(|_| std::io::Error::other("workspace authority is unavailable"))?;
        let containing_root = guard
            .containing_root(invocation.cwd.as_deref().unwrap_or(configured_workspace))
            .ok_or_else(|| std::io::Error::other("sandbox root is unauthorized"))?;
        let selected_root = discovered_repository
            .as_deref()
            .filter(|repository| {
                *repository != containing_root && repository.starts_with(containing_root)
            })
            .unwrap_or(containing_root);
        Some(selected_root.to_path_buf())
    } else {
        None
    };
    let sandbox_root = match profile.workspace_mount {
        WorkspaceMount::Authorized => authorized_root
            .as_deref()
            .or(discovered_workspace.as_deref())
            .or(configured_workspace.as_deref()),
        WorkspaceMount::Fixed(root) => Some(root),
        WorkspaceMount::Hidden => None,
    };
    if let Some(sandbox_root) = sandbox_root {
        let execution_root = execution_root
            .as_deref()
            .ok_or_else(|| std::io::Error::other("invalid execution root"))?;
        if sandbox_root == Path::new("/")
            || !sandbox_root.starts_with(execution_root)
            || !config.is_path_contained(sandbox_root)
        {
            return Err(std::io::Error::other(
                "sandbox root is outside authorized workspace authority",
            )
            .into());
        }
        if crate::core::protected_paths::is_protected_path(execution_root, sandbox_root) {
            return Err(std::io::Error::other(
                "sandbox workspace is protected by credential policy",
            )
            .into());
        }
    }
    let root_bind = match profile.workspace_access {
        WorkspaceAccess::ReadOnly => "--ro-bind",
        WorkspaceAccess::Writable => "--bind",
    };
    let root = sandbox_root.map(|root| root.to_string_lossy().into_owned());
    let workspace = root.as_deref().map(|root| (root_bind, root));
    let mut args = paths::base_bwrap_args(&sandbox_home, workspace);
    if matches!(profile.network_access, NetworkAccess::Isolated) {
        args.push("--unshare-net".into());
    }
    if profile.expose_runtime_extras && !workspace_hidden {
        args.extend(["--ro-bind-try".into(), "/opt".into(), "/opt".into()]);
        if matches!(invocation.program, InvocationProgram::SelfBinary)
            && sandbox_root.is_some_and(|root| !program_path.starts_with(root))
        {
            let bin = program_path.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), bin.clone(), bin]);
        }
    }
    let sandbox_program_path = if workspace_hidden {
        let sandbox_path = "/tmp/ai-tools-network";
        args.extend([
            "--ro-bind".into(),
            program_path.to_string_lossy().into_owned(),
            sandbox_path.into(),
        ]);
        PathBuf::from(sandbox_path)
    } else {
        program_path.clone()
    };
    let (cargo_home, rustup_home, mut protected_path_freshness_checks) = if workspace_hidden {
        (None, None, Vec::new())
    } else {
        let sandbox_root =
            sandbox_root.ok_or_else(|| std::io::Error::other("workspace mount is unavailable"))?;
        let toolchain_mounts = toolchain_mounts::mount_toolchains(
            &mut args,
            config,
            sandbox_root,
            &host_home,
            control,
        )?;
        (
            toolchain_mounts.cargo_home,
            toolchain_mounts.rustup_home,
            toolchain_mounts.freshness_checks,
        )
    };
    let ssh_root = if matches!(&invocation.security, InvocationSecurity::Ssh { .. }) {
        Some(config.resolved_ssh_root().map_err(std::io::Error::other)?)
    } else {
        None
    };
    // A mounted HOME must hide the entire SSH store even for dedicated SSH;
    // only its exact reviewed material is restored below.
    if let Some(sandbox_root) = sandbox_root {
        add_protected_paths(
            &mut args,
            sandbox_root,
            true,
            None,
            "workspace",
            control,
            &mut protected_path_freshness_checks,
        )
        .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
        masks::mask_state(&mut args, config, sandbox_root)?;
        if execution_root
            .as_deref()
            .is_some_and(|execution_root| sandbox_root != execution_root)
        {
            let execution_root = execution_root
                .as_deref()
                .ok_or_else(|| std::io::Error::other("invalid execution root"))?;
            add_protected_paths(
                &mut args,
                execution_root,
                false,
                ssh_root.as_deref(),
                "execution_root_mask",
                control,
                &mut protected_path_freshness_checks,
            )
            .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
        }
    }
    if let InvocationSecurity::Ssh {
        identity_file,
        known_hosts_file,
    } = &invocation.security
    {
        ssh_material::add_material(
            &mut args,
            config,
            &host_home,
            identity_file,
            known_hosts_file,
        )?;
    }
    if !workspace_hidden && invocation.expose_authorized_siblings {
        let sandbox_root =
            sandbox_root.ok_or_else(|| std::io::Error::other("workspace mount is unavailable"))?;
        let execution_root = execution_root
            .as_deref()
            .ok_or_else(|| std::io::Error::other("invalid execution root"))?;
        if let Ok(guard) = config.workspaces.read() {
            for ws in guard.all_roots() {
                if let Some(control) = control {
                    control.check()?;
                }
                if ws != sandbox_root
                    && !ws.starts_with(sandbox_root)
                    && !sandbox_root.starts_with(&ws)
                    && !crate::core::protected_paths::is_protected_path(execution_root, &ws)
                {
                    let val = ws.to_string_lossy().into_owned();
                    args.extend([root_bind.into(), val.clone(), val]);
                    add_protected_paths(
                        &mut args,
                        &ws,
                        true,
                        None,
                        "authorized_sibling",
                        control,
                        &mut protected_path_freshness_checks,
                    )
                    .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
                    masks::mask_state(&mut args, config, &ws)?;
                }
            }
        }
    }
    let executable_directories = if workspace_hidden {
        [
            "/usr/local/sbin",
            "/usr/local/bin",
            "/usr/sbin",
            "/usr/bin",
            "/sbin",
            "/bin",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect()
    } else {
        safe_path_entries(config)
    };
    if ssh_root.is_none() {
        masks::mask_executables_from(
            &mut args,
            executable_directories.clone(),
            crate::core::terminal_policy::GENERIC_SSH_CLIENTS,
            control,
        )?;
    }
    masks::mask_executables_from(
        &mut args,
        executable_directories,
        crate::core::terminal_policy::PRIVILEGE_BROKERS,
        control,
    )?;
    // Masks run before opt-ins, so a configured socket under HOME cannot bypass
    // default denial yet can still be exposed by its separate operator grant.
    if !workspace_hidden && profile.expose_optional_sockets {
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
    let freshness_started = Instant::now();
    for index in &protected_path_freshness_checks {
        if let Some(control) = control {
            control.check()?;
        }
        let fresh = index
            .is_fresh()
            .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
        if !fresh {
            return Err(SandboxError::at(
                "protected_path_discovery",
                io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace changed before sandbox spawn",
                ),
            ));
        }
    }
    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "protected_path_final_check",
        outcome = "completed",
        index_count = protected_path_freshness_checks.len(),
        duration_ms = freshness_started.elapsed().as_millis() as u64,
    );
    if let Some(control) = control {
        control.check()?;
    }
    if workspace_hidden {
        args.extend(["--chdir".into(), "/tmp".into()]);
    } else if let Some(cwd) = &invocation.cwd {
        args.extend(["--chdir".into(), cwd.to_string_lossy().into_owned()]);
    }
    args.push(sandbox_program_path.to_string_lossy().into_owned());
    args.extend(invocation.args.clone());
    let safe_path = if workspace_hidden {
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_owned()
    } else {
        safe_path_entries(config)
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(":")
    };
    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "profile_construction",
        outcome = "completed",
        bubblewrap_argument_count = args.len(),
        duration_ms = profile_started.elapsed().as_millis() as u64,
    );
    let mut command = Command::new(bwrap);
    command
        .args(args)
        .env_clear()
        .env("HOME", sandbox_home)
        .env("PATH", safe_path)
        .env("LANG", "C.UTF-8")
        .env("TMPDIR", "/tmp")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if matches!(invocation.security, InvocationSecurity::Ssh { .. }) {
        command.stdin(Stdio::null());
    } else {
        command.stdin(Stdio::piped());
    }
    if let Some(rustup_home) = rustup_home {
        command.env("RUSTUP_HOME", rustup_home);
    }
    match (
        matches!(profile.workspace_mount, WorkspaceMount::Fixed(_)),
        cargo_home,
    ) {
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
    #[cfg(target_os = "linux")]
    // SAFETY: prctl is async-signal-safe here and touches no allocator/locks.
    // Inherited across exec: copied/renamed setuid helpers cannot gain privilege.
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let spawn_started = Instant::now();
    let child = command
        .spawn()
        .map_err(|error| SandboxError::at("bwrap_spawn", error))?;
    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "bwrap_spawn",
        outcome = "completed",
        duration_ms = spawn_started.elapsed().as_millis() as u64,
    );
    Ok(child)
}
