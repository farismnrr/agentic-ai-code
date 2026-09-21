//! Complete bounded masking for every exposed user tree; never prune visible caches.
use std::path::{Path, PathBuf};
use std::time::Instant;

#[cfg(target_os = "linux")]
mod protected_index;
#[cfg(not(target_os = "linux"))]
#[path = "masks/protected_index_portable.rs"]
mod protected_index;

pub(super) use protected_index::ProtectedPathFreshness;

pub(super) struct ProtectedPathFreshnessGuard<'a> {
    _guard: protected_index::PreSpawnFreshnessGuard<'a>,
}

pub(super) fn lock_protected_path_freshness<'a>(
    checks: &'a [ProtectedPathFreshness],
    control: Option<&super::SpawnControl<'_>>,
) -> Result<ProtectedPathFreshnessGuard<'a>, std::io::Error> {
    protected_index::lock_and_validate_freshness(checks, control)
        .map(|_guard| ProtectedPathFreshnessGuard { _guard })
}

pub(super) fn mask_executables_from(
    args: &mut Vec<String>,
    directories: impl IntoIterator<Item = PathBuf>,
    names: &[&str],
    control: Option<&super::SpawnControl<'_>>,
) -> Result<(), std::io::Error> {
    let mut masked = std::collections::BTreeSet::new();
    for directory in directories {
        for name in names {
            if let Some(control) = control {
                control.check()?;
            }
            let candidate = directory.join(name);
            match std::fs::symlink_metadata(&candidate) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    return Err(std::io::Error::other(
                        "forbidden executable metadata is unavailable",
                    ))
                }
                Ok(_) => {}
            }
            let canonical = std::fs::canonicalize(&candidate)
                .map_err(|_| std::io::Error::other("forbidden executable target is unsafe"))?;
            if masked.insert(canonical.clone()) {
                mask_protected_file(args, &canonical)?;
            }
            // /bin and /usr/bin can be separate bind mounts even on a merged-
            // usr host. Mask each visible spelling, not just its host inode.
            if masked.insert(candidate.clone()) {
                args.extend([
                    "--ro-bind".into(),
                    "/dev/null".into(),
                    candidate.to_string_lossy().into_owned(),
                ]);
            }
        }
    }
    Ok(())
}

pub(super) fn mask_protected_file(
    args: &mut Vec<String>,
    path: &Path,
) -> Result<(), std::io::Error> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            return Err(std::io::Error::other(
                "protected path metadata is unavailable",
            ))
        }
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

pub(super) fn add_optional_socket(
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

pub(super) fn add_protected_paths(
    args: &mut Vec<String>,
    execution_root: &Path,
    recursive: bool,
    skip: Option<&Path>,
    scope: &'static str,
    control: Option<&super::SpawnControl<'_>>,
    freshness_checks: &mut Vec<ProtectedPathFreshness>,
) -> Result<(), std::io::Error> {
    let scan_started = Instant::now();
    let (paths, scanned_entries, cache_hit, watch_enabled) = if recursive {
        let (canonical_root, index, cache_hit) = protected_index::discover(execution_root, control)
            .map_err(|error| {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_discovery",
                    scope,
                    outcome = "failed",
                    error_kind = ?error.kind(),
                    duration_ms = scan_started.elapsed().as_millis() as u64,
                );
                std::io::Error::new(
                    error.kind(),
                    "protected-path discovery could not complete safely",
                )
            })?;
        let paths = index
            .protected_paths()
            .iter()
            .filter(|path| path.strip_prefix(&canonical_root).is_ok())
            .cloned()
            .collect::<Vec<_>>();
        let scanned_entries = index.scanned_entries();
        let watch_enabled = index.watcher_enabled();
        freshness_checks.push(index);
        (paths, scanned_entries, cache_hit, watch_enabled)
    } else {
        (
            crate::core::protected_paths::protected_paths(execution_root).collect(),
            0,
            false,
            false,
        )
    };
    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "protected_path_discovery",
        scope,
        outcome = "completed",
        recursive,
        cache_hit,
        watch_enabled,
        protected_path_count = paths.len(),
        scanned_entries,
        duration_ms = scan_started.elapsed().as_millis() as u64,
    );
    for path in paths.into_iter().filter(|p| skip != Some(p.as_path())) {
        if let Some(control) = control {
            control.check()?;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if !recursive && error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(std::io::Error::other(format!(
                    "protected path metadata is unavailable: {:?}",
                    error.kind()
                )))
            }
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

pub(super) fn mask_state(
    args: &mut Vec<String>,
    config: &crate::core::config::ServerConfig,
    root: &Path,
) -> Result<(), std::io::Error> {
    let state = config.activity.resolved_state_dir()?;
    match std::fs::symlink_metadata(&state) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(std::io::Error::other("state root metadata unavailable")),
        Ok(_) => {}
    }
    let canonical = std::fs::canonicalize(&state)?;
    if canonical != state {
        return Err(std::io::Error::other("state root is not canonical"));
    }
    if root.starts_with(&state) {
        return Err(std::io::Error::other("workspace overlaps protected state"));
    }
    if state.starts_with(root) {
        args.extend(["--tmpfs".into(), state.to_string_lossy().into_owned()]);
    }
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
pub(super) fn test_prime_protected_path_index_with_entry_limit(
    root: &Path,
    budget: std::time::Duration,
    max_entries: usize,
) -> Result<usize, std::io::Error> {
    protected_index::prime_with_entry_limit(root, budget, max_entries)
}

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
pub(super) fn test_discover_protected_path_index(root: &Path) -> Result<(), std::io::Error> {
    protected_index::discover(root, None).map(|_| ())
}

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
pub(super) fn test_snapshot_rejects_new_protected_path(
    root: &Path,
    relative: &Path,
) -> Result<(), std::io::Error> {
    let (_, freshness, _) = protected_index::discover(root, None)?;
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "secret")?;
    let checks = [freshness];
    let _guard = protected_index::lock_and_validate_freshness(&checks, None)?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
pub(super) fn test_schedule_protected_path_index(
    root: &Path,
    budget: std::time::Duration,
) -> Result<(), std::io::Error> {
    protected_index::schedule_initialization(root, budget)
}

#[cfg(all(target_os = "linux", feature = "test-protected-index"))]
pub(super) fn test_is_permanent_index_error(kind: std::io::ErrorKind) -> bool {
    protected_index::is_permanent_index_error(kind)
}
