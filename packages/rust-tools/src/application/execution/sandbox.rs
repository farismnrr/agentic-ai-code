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
use masks::{
    add_optional_socket, add_protected_paths, mask_protected_file, ProtectedPathFreshness,
};
mod managed_process;
mod protected_paths;
mod ssh_material;

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

struct SandboxProfile<'a> {
    workspace_access: WorkspaceAccess,
    network_access: NetworkAccess,
    expose_optional_sockets: bool,
    expose_runtime_extras: bool,
    workspace_root: Option<&'a Path>,
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
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| std::io::Error::other("invalid execution root"))?;
    let host_home =
        runtime_home().map_err(|error| SandboxError::at("executable_resolution", error))?;
    let home = host_home.to_string_lossy().into_owned();
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
    let discovered_workspace = if profile.workspace_root.is_none() {
        let cwd_arg = invocation.cwd.as_ref().and_then(|path| path.to_str());
        crate::application::git::resolve_git_workspace(cwd_arg, config)
            .ok()
            .or_else(|| invocation.cwd.clone())
    } else {
        None
    };
    let configured_workspace = std::fs::canonicalize(config.resolved_dir().unwrap_or_default())
        .map_err(|_| std::io::Error::other("invalid workspace directory"))?;
    // Terminal authority is the containing authorized root, not a repository
    // guessed from cwd. The execution boundary alone never authorizes siblings.
    config
        .ensure_workspaces_initialized()
        .map_err(|_| std::io::Error::other("workspace authority is unavailable"))?;
    let authorized_root = if profile.workspace_root.is_none()
        && invocation.expose_authorized_siblings
        && !matches!(invocation.security, InvocationSecurity::Ssh { .. })
    {
        let guard = config
            .workspaces
            .read()
            .map_err(|_| std::io::Error::other("workspace authority is unavailable"))?;
        Some(
            guard
                .containing_root(invocation.cwd.as_deref().unwrap_or(&configured_workspace))
                .ok_or_else(|| std::io::Error::other("sandbox root is unauthorized"))?
                .to_path_buf(),
        )
    } else {
        None
    };
    let sandbox_root = profile
        .workspace_root
        .or(authorized_root.as_deref())
        .or(discovered_workspace.as_deref())
        .unwrap_or(&configured_workspace);
    if sandbox_root == Path::new("/")
        || !sandbox_root.starts_with(&execution_root)
        || !config.is_path_contained(sandbox_root)
    {
        return Err(std::io::Error::other(
            "sandbox root is outside authorized workspace authority",
        )
        .into());
    }
    if crate::core::protected_paths::is_protected_path(&execution_root, sandbox_root) {
        return Err(
            std::io::Error::other("sandbox workspace is protected by credential policy").into(),
        );
    }
    let root = sandbox_root.to_string_lossy().into_owned();
    let root_bind = match profile.workspace_access {
        WorkspaceAccess::ReadOnly => "--ro-bind",
        WorkspaceAccess::Writable => "--bind",
    };
    let mut args = paths::base_bwrap_args(&home, root_bind, &root);
    if matches!(profile.network_access, NetworkAccess::Isolated) {
        args.push("--unshare-net".into());
    }
    if profile.expose_runtime_extras {
        args.extend(["--ro-bind-try".into(), "/opt".into(), "/opt".into()]);
        if matches!(invocation.program, InvocationProgram::SelfBinary)
            && !program_path.starts_with(sandbox_root)
        {
            let bin = program_path.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), bin.clone(), bin]);
        }
    }
    let mut cargo_home = None;
    let mut rustup_home = None;
    let mut toolchain_roots = std::collections::BTreeSet::new();
    let mut protected_path_freshness_checks: Vec<ProtectedPathFreshness> = Vec::new();
    let home_cargo_bin = host_home.join(".cargo/bin");
    let canonical_home_cargo_bin = std::fs::canonicalize(&home_cargo_bin).ok();
    for path in &config.toolchain_paths {
        if let Some(control) = control {
            control.check()?;
        }
        let configured = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&configured)
            .map_err(|_| std::io::Error::other("invalid toolchain path"))?;
        let canonical_value = canonical.to_string_lossy().into_owned();
        toolchain_roots.insert(canonical.clone());
        args.extend([
            "--ro-bind".into(),
            canonical_value.clone(),
            canonical_value.clone(),
        ]);
        // Executable resolution preserves the configured path so shims keep
        // their argv[0] semantics. Mount the configured symlink directory at
        // that same path as well as its canonical target; otherwise a
        // provider such as an fnm-managed coding CLI resolves successfully during
        // capability discovery but is absent from the Bubblewrap namespace.
        let configured_value = configured.to_string_lossy().into_owned();
        if configured != canonical && !configured.starts_with(sandbox_root) {
            if configured.starts_with(&host_home) {
                paths::add_bwrap_parent_dirs(&mut args, configured.parent(), &host_home);
                args.extend([
                    "--symlink".into(),
                    canonical_value.clone(),
                    configured_value,
                ]);
            } else {
                args.extend([
                    "--ro-bind".into(),
                    canonical_value.clone(),
                    configured_value,
                ]);
            }
        }
        if let Some(toolchain_root) = super::toolchain::reviewed_root(&canonical) {
            toolchain_roots.insert(toolchain_root.to_path_buf());
            let value = toolchain_root.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), value.clone(), value]);
        }
        if canonical_home_cargo_bin.as_ref() == Some(&canonical) {
            let candidate = host_home.join(".cargo");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                if !candidate.starts_with(sandbox_root) {
                    args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                    toolchain_roots.insert(candidate.clone());
                }
                for file in ["credentials", "credentials.toml"] {
                    mask_protected_file(&mut args, &candidate.join(file))?;
                }
                cargo_home = Some(value);
            }
            let candidate = host_home.join(".rustup");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                if !candidate.starts_with(sandbox_root) {
                    args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                    toolchain_roots.insert(candidate.clone());
                }
                rustup_home = Some(value);
            }
        }
    }
    // Discovered user runtimes (Conda environments, nvm/fnm/Volta/asdf, npm
    // and pnpm bins) must be visible when the authorized workspace is narrower
    // than HOME. Mount only the validated executable directory, never its
    // surrounding profile or credential store.
    for discovered in super::toolchain::safe_path_entries(config) {
        if let Some(control) = control {
            control.check()?;
        }
        let Ok(canonical) = std::fs::canonicalize(&discovered) else {
            continue;
        };
        if canonical.starts_with(sandbox_root)
            || canonical.starts_with(Path::new("/usr"))
            || canonical.starts_with(Path::new("/bin"))
            || canonical.starts_with(Path::new("/sbin"))
            || canonical.starts_with(Path::new("/lib"))
            || canonical.starts_with(Path::new("/opt"))
            || toolchain_roots.contains(&canonical)
        {
            continue;
        }
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
        let configured_value = discovered.to_string_lossy().into_owned();
        if discovered != canonical {
            paths::add_bwrap_parent_dirs(&mut args, discovered.parent(), &host_home);
            args.extend([
                "--symlink".into(),
                canonical.to_string_lossy().into_owned(),
                configured_value,
            ]);
        }
        toolchain_roots.insert(canonical);
    }
    for toolchain_root in &toolchain_roots {
        if let Some(control) = control {
            control.check()?;
        }
        if !toolchain_root.starts_with(sandbox_root)
            && !toolchain_roots
                .iter()
                .any(|other| other != toolchain_root && toolchain_root.starts_with(other))
        {
            add_protected_paths(
                &mut args,
                toolchain_root,
                true,
                None,
                "toolchain",
                control,
                &mut protected_path_freshness_checks,
            )
            .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
            masks::mask_state(&mut args, config, toolchain_root)?;
        }
    }
    let ssh_root = if matches!(&invocation.security, InvocationSecurity::Ssh { .. }) {
        Some(config.resolved_ssh_root().map_err(std::io::Error::other)?)
    } else {
        None
    };
    // A mounted HOME must hide the entire SSH store even for dedicated SSH;
    // only its exact reviewed material is restored below.
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
    if sandbox_root != execution_root {
        add_protected_paths(
            &mut args,
            &execution_root,
            false,
            ssh_root.as_deref(),
            "execution_root_mask",
            control,
            &mut protected_path_freshness_checks,
        )
        .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
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
    let _ = config.ensure_workspaces_initialized();
    if invocation.expose_authorized_siblings {
        if let Ok(guard) = config.workspaces.read() {
            for ws in guard.all_roots() {
                if let Some(control) = control {
                    control.check()?;
                }
                if ws != sandbox_root
                    && !ws.starts_with(sandbox_root)
                    && !sandbox_root.starts_with(&ws)
                    && !crate::core::protected_paths::is_protected_path(&execution_root, &ws)
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
    if ssh_root.is_none() {
        masks::mask_executables(
            &mut args,
            config,
            crate::core::terminal_policy::GENERIC_SSH_CLIENTS,
            control,
        )?;
    }
    masks::mask_executables(
        &mut args,
        config,
        crate::core::terminal_policy::PRIVILEGE_BROKERS,
        control,
    )?;
    // Masks run before opt-ins, so a configured socket under HOME cannot bypass
    // default denial yet can still be exposed by its separate operator grant.
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
        .env("HOME", home)
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
