use super::{
    bridge, BlenderSessionOwnership, BlenderSessionState, BlenderSessionStatus,
    BLENDER_LAB_PROTOCOL,
};
use crate::application::execution::{kill_process_group, resolve_safe_executable};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

const READY_POLL_MS: u64 = 100;
const PROCESS_REAP_TIMEOUT_MS: u64 = 5_000;

struct OwnedSession {
    child: Child,
    project_root: PathBuf,
    project_id: String,
    owner: String,
}

#[derive(Default)]
struct SessionState {
    owned: Option<OwnedSession>,
}

fn state() -> &'static Mutex<SessionState> {
    static STATE: OnceLock<Mutex<SessionState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SessionState::default()))
}

pub async fn status(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<BlenderSessionStatus, McpError> {
    ensure_enabled(config)?;
    let project_root = project_root(cwd, config)?;
    let mut guard = state().lock().await;
    reap_finished(&mut guard).await?;
    match bridge::probe(config).await {
        Ok(_) => Ok(session_status(
            BlenderSessionState::Ready,
            Some(ownership_for(&guard, owner, project_id, &project_root)),
            config,
            Some(&project_root),
        )),
        Err(error) if is_unavailable(&error) => Ok(session_status(
            BlenderSessionState::Closed,
            None,
            config,
            Some(&project_root),
        )),
        Err(_) => Ok(session_status(
            BlenderSessionState::Incompatible,
            guard
                .owned
                .as_ref()
                .map(|_| ownership_for(&guard, owner, project_id, &project_root)),
            config,
            Some(&project_root),
        )),
    }
}

pub async fn start(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<BlenderSessionStatus, McpError> {
    ensure_enabled(config)?;
    let deadline = (config.blender_bridge_timeout_ms > 0)
        .then(|| Instant::now() + Duration::from_millis(config.blender_bridge_timeout_ms));
    let project_root = project_root(cwd, config)?;
    ensure_project_layout(cwd, config)?;

    let mut guard = state().lock().await;
    reap_finished(&mut guard).await?;
    let initial_probe = match deadline {
        Some(deadline) => match config_for_deadline(config, deadline) {
            Some(probe_config) => bridge::probe(&probe_config).await,
            None => Err(McpError::InvalidRequest(
                "Blender bridge did not become ready before timeout".into(),
            )),
        },
        None => bridge::probe(config).await,
    };
    match initial_probe {
        Ok(_) => {
            let ownership = ownership_for(&guard, owner, project_id, &project_root);
            return Ok(session_status(
                BlenderSessionState::Ready,
                Some(ownership),
                config,
                Some(&project_root),
            ));
        }
        Err(error) if is_unavailable(&error) => {}
        Err(_) => {
            return Ok(session_status(
                BlenderSessionState::Incompatible,
                guard
                    .owned
                    .as_ref()
                    .map(|_| ownership_for(&guard, owner, project_id, &project_root)),
                config,
                Some(&project_root),
            ));
        }
    }

    if let Some(owned) = guard.owned.as_ref() {
        if !matches_identity(owned, owner, project_id, &project_root) {
            return Err(McpError::InvalidRequest(
                "relay-owned Blender session belongs to a different owner or project".into(),
            ));
        }
        return Err(McpError::InvalidRequest(
            "relay-owned Blender process exists but its bridge is unavailable".into(),
        ));
    }

    let executable = resolve_blender_executable(config)?;
    let blender_root = project_root.join("blender");
    let tmp_root = blender_root.join("tmp");
    let runtime_home = runtime_home(config)?;
    if !runtime_home.is_dir() {
        return Err(McpError::InvalidRequest(
            "Blender runtime profile is not prepared; install the official Blender Lab MCP extension into the relay execution root .masihawam/blender-runtime-home before starting a relay-owned session"
                .into(),
        ));
    }
    let mut command = Command::new(executable);
    command
        .arg("--background")
        .arg("--online-mode")
        .arg("--command")
        .arg("blender_mcp")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(config.blender_bridge_port.to_string())
        .current_dir(&blender_root)
        .env("HOME", &runtime_home)
        .env("TMPDIR", &tmp_root)
        .env("TMP", &tmp_root)
        .env("TEMP", &tmp_root)
        .kill_on_drop(false);
    #[cfg(unix)]
    command.process_group(0);
    let child = command.spawn().map_err(|_| {
        McpError::InvalidRequest("failed to launch reviewed Blender executable".into())
    })?;
    guard.owned = Some(OwnedSession {
        child,
        project_root: project_root.clone(),
        project_id: project_id.to_owned(),
        owner: owner.to_owned(),
    });
    drop(guard);

    loop {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            stop_owned_process(owner, project_id, &project_root).await?;
            return Err(McpError::InvalidRequest(
                "Blender bridge did not become ready before timeout".into(),
            ));
        }
        let probe_ready = match deadline {
            Some(deadline) => match config_for_deadline(config, deadline) {
                Some(probe_config) => bridge::probe(&probe_config).await.is_ok(),
                None => false,
            },
            None => bridge::probe(config).await.is_ok(),
        };
        if probe_ready {
            return Ok(session_status(
                BlenderSessionState::Ready,
                Some(BlenderSessionOwnership::RelayOwned),
                config,
                Some(&project_root),
            ));
        }
        {
            let mut guard = state().lock().await;
            if let Some(owned) = guard.owned.as_mut() {
                if owned
                    .child
                    .try_wait()
                    .map_err(|_| McpError::Internal("failed to inspect Blender process".into()))?
                    .is_some()
                {
                    guard.owned = None;
                    return Err(McpError::InvalidRequest(
                        "Blender exited before its loopback bridge became ready; verify the official Blender Lab MCP extension is installed and enabled for the relay service user"
                            .into(),
                    ));
                }
            }
        }
        sleep(Duration::from_millis(READY_POLL_MS)).await;
    }
}

pub async fn stop(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<BlenderSessionStatus, McpError> {
    ensure_enabled(config)?;
    let project_root = project_root(cwd, config)?;
    let mut guard = state().lock().await;
    reap_finished(&mut guard).await?;
    let Some(owned) = guard.owned.as_ref() else {
        if bridge::probe(config).await.is_ok() {
            return Err(McpError::InvalidRequest(
                "refusing to stop externally owned Blender session".into(),
            ));
        }
        return Ok(session_status(
            BlenderSessionState::Closed,
            None,
            config,
            Some(&project_root),
        ));
    };
    if !matches_identity(owned, owner, project_id, &project_root) {
        return Err(McpError::InvalidRequest(
            "refusing to stop Blender session owned by a different owner or project".into(),
        ));
    }
    let mut owned = guard.owned.take().expect("owned session checked above");
    drop(guard);
    terminate_owned(&mut owned.child).await;
    Ok(session_status(
        BlenderSessionState::Closed,
        None,
        config,
        Some(&project_root),
    ))
}

async fn stop_owned_process(
    owner: &str,
    project_id: &str,
    project_root: &Path,
) -> Result<(), McpError> {
    let mut guard = state().lock().await;
    let Some(owned) = guard.owned.as_ref() else {
        return Ok(());
    };
    if !matches_identity(owned, owner, project_id, project_root) {
        return Err(McpError::InvalidRequest(
            "refusing to clean up Blender session owned by a different owner or project".into(),
        ));
    }
    let mut owned = guard.owned.take().expect("owned session checked above");
    drop(guard);
    terminate_owned(&mut owned.child).await;
    Ok(())
}

async fn terminate_owned(child: &mut Child) {
    #[cfg(unix)]
    kill_process_group(child).await;
    let _ = child.kill().await;
    let _ = timeout(Duration::from_millis(PROCESS_REAP_TIMEOUT_MS), child.wait()).await;
}

async fn reap_finished(state: &mut SessionState) -> Result<(), McpError> {
    if let Some(owned) = state.owned.as_mut() {
        if owned
            .child
            .try_wait()
            .map_err(|_| McpError::Internal("failed to inspect Blender process".into()))?
            .is_some()
        {
            state.owned = None;
        }
    }
    Ok(())
}

fn ownership_for(
    state: &SessionState,
    owner: &str,
    project_id: &str,
    project_root: &Path,
) -> BlenderSessionOwnership {
    if state
        .owned
        .as_ref()
        .is_some_and(|owned| matches_identity(owned, owner, project_id, project_root))
    {
        BlenderSessionOwnership::RelayOwned
    } else {
        BlenderSessionOwnership::External
    }
}

fn matches_identity(
    owned: &OwnedSession,
    owner: &str,
    project_id: &str,
    project_root: &Path,
) -> bool {
    owned.owner == owner && owned.project_id == project_id && owned.project_root == project_root
}

fn ensure_enabled(config: &ServerConfig) -> Result<(), McpError> {
    if !config.enable_creative {
        return Err(McpError::InvalidRequest(
            "Creative capability is disabled by operator configuration".into(),
        ));
    }
    Ok(())
}

fn project_root(cwd: Option<&str>, config: &ServerConfig) -> Result<PathBuf, McpError> {
    super::resolve_project_root(cwd, config)
}

fn runtime_home(config: &ServerConfig) -> Result<PathBuf, McpError> {
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
    Ok(root.join(".masihawam/blender-runtime-home"))
}

pub(super) fn ensure_project_layout(
    cwd: Option<&str>,
    config: &ServerConfig,
) -> Result<(), McpError> {
    for path in [
        "blender/scenes/.relay-layout",
        "blender/assets/.relay-layout",
        "blender/references/.relay-layout",
        "blender/renders/preview/.relay-layout",
        "blender/renders/final/.relay-layout",
        "blender/animations/.relay-layout",
        "blender/exports/.relay-layout",
        "blender/checkpoints/.relay-layout",
        "blender/tmp/.relay-layout",
    ] {
        crate::application::workspace::write_contained_bytes(
            path, cwd, b"{}\n", true, true, config,
        )?;
    }
    Ok(())
}

fn resolve_blender_executable(config: &ServerConfig) -> Result<PathBuf, McpError> {
    if let Some(path) = config.blender_executable.as_deref() {
        return validate_operator_executable(Path::new(path));
    }
    if let Ok(path) = resolve_safe_executable(config, "blender") {
        return Ok(path);
    }
    #[cfg(target_os = "macos")]
    {
        for path in [
            "/Applications/Blender.app/Contents/MacOS/Blender",
            "/Applications/Blender 5.1.app/Contents/MacOS/Blender",
        ] {
            if let Ok(path) = validate_operator_executable(Path::new(path)) {
                return Ok(path);
            }
        }
    }
    Err(McpError::InvalidRequest(
        "no reviewed Blender executable is configured or discoverable".into(),
    ))
}

fn validate_operator_executable(path: &Path) -> Result<PathBuf, McpError> {
    if !path.is_absolute() {
        return Err(McpError::InvalidRequest(
            "operator Blender executable must be an absolute path".into(),
        ));
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| {
        McpError::InvalidRequest("operator Blender executable is unavailable".into())
    })?;
    let metadata = canonical.metadata().map_err(|_| {
        McpError::InvalidRequest("operator Blender executable is unavailable".into())
    })?;
    if !metadata.is_file() {
        return Err(McpError::InvalidRequest(
            "operator Blender executable must be a regular file".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(McpError::InvalidRequest(
                "operator Blender executable is not executable".into(),
            ));
        }
    }
    Ok(canonical)
}

fn session_status(
    state: BlenderSessionState,
    ownership: Option<BlenderSessionOwnership>,
    config: &ServerConfig,
    project_root: Option<&Path>,
) -> BlenderSessionStatus {
    BlenderSessionStatus {
        state,
        ownership,
        protocol: BLENDER_LAB_PROTOCOL.into(),
        loopback_port: config.blender_bridge_port,
        project_root: project_root.map(|path| path.to_string_lossy().into_owned()),
    }
}

fn config_for_deadline(config: &ServerConfig, deadline: Instant) -> Option<ServerConfig> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return None;
    }
    let remaining_ms = remaining.as_millis().min(u128::from(u64::MAX)) as u64;
    let mut scoped = config.clone();
    scoped.blender_bridge_timeout_ms = scoped.blender_bridge_timeout_ms.min(remaining_ms.max(1));
    Some(scoped)
}

fn is_unavailable(error: &McpError) -> bool {
    matches!(error, McpError::InvalidRequest(message) if message == "Blender bridge is unavailable" || message == "Blender bridge connection timed out")
}
