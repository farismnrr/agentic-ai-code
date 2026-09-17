//! Portable fail-closed discovery used only where Bubblewrap execution is unavailable.
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_PROTECTED_SCAN_ENTRIES: usize = 500_000;
const CONTROL_CHECK_INTERVAL: usize = 64;

pub(in crate::application::execution::sandbox) struct ProtectedPathFreshness {
    protected_paths: Arc<[PathBuf]>,
    scanned_entries: usize,
}

impl ProtectedPathFreshness {
    pub(super) fn protected_paths(&self) -> &[PathBuf] {
        &self.protected_paths
    }

    pub(super) fn scanned_entries(&self) -> usize {
        self.scanned_entries
    }

    pub(super) fn watcher_enabled(&self) -> bool {
        false
    }

    fn is_fresh(&self) -> io::Result<bool> {
        Ok(true)
    }
}

pub(super) struct PreSpawnFreshnessGuard;

impl ProtectedPathIndex {
    pub(super) fn watcher_enabled(&self) -> bool {
        false
    }

    pub(super) fn is_fresh(&self) -> io::Result<bool> {
        Ok(true)
    }
}

pub(super) fn discover(
    root: &Path,
    control: Option<&super::super::SpawnControl<'_>>,
) -> io::Result<(PathBuf, ProtectedPathFreshness, bool)> {
    discover_with_budget(root, control, Duration::from_secs(60))
}

fn discover_with_budget(
    root: &Path,
    control: Option<&super::super::SpawnControl<'_>>,
    timeout: Duration,
) -> io::Result<(PathBuf, ProtectedPathFreshness, bool)> {
    let root = std::fs::canonicalize(root)?;
    let deadline = Instant::now() + timeout;
    let mut pending = vec![root.clone()];
    let mut protected = BTreeSet::new();
    let mut scanned_entries = 0usize;
    while let Some(directory) = pending.pop() {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path index initialization deadline elapsed",
            ));
        }
        if let Some(control) = control {
            control.check()?;
        }
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            scanned_entries = scanned_entries.saturating_add(1);
            if scanned_entries > MAX_PROTECTED_SCAN_ENTRIES {
                return Err(io::Error::other(
                    "protected-path scan exceeds bounded workspace maximum",
                ));
            }
            if scanned_entries % CONTROL_CHECK_INTERVAL == 0 {
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "protected-path index initialization deadline elapsed",
                    ));
                }
                if let Some(control) = control {
                    control.check()?;
                }
            }
            let name = entry.file_name();
            let kind = entry.file_type()?;
            let path = directory.join(&name);
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| io::Error::other("protected-path scan escaped workspace"))?;
            let may_be_protected = crate::core::protected_paths::may_be_protected_entry(&name);
            let socket = is_socket(&kind);
            if (may_be_protected && crate::core::protected_paths::is_protected_relative(relative))
                || socket
            {
                protected.insert(path);
                continue;
            }
            if kind.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok((
        root,
        ProtectedPathFreshness {
            protected_paths: Arc::from(protected.into_iter().collect::<Vec<_>>()),
            scanned_entries,
        },
        false,
    ))
}

pub(super) fn prime(root: &Path, budget: Duration) -> io::Result<usize> {
    let (_, index, _) = discover_with_budget(root, None, budget)?;
    Ok(index.scanned_entries())
}

pub(super) fn schedule_initialization(_root: &Path, _budget: Duration) -> io::Result<()> {
    Ok(())
}

pub(super) fn lock_and_validate_freshness(
    checks: &[ProtectedPathFreshness],
) -> io::Result<PreSpawnFreshnessGuard> {
    for check in checks {
        if !check.is_fresh()? {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed before sandbox spawn",
            ));
        }
    }
    Ok(PreSpawnFreshnessGuard)
}

fn is_socket(kind: &std::fs::FileType) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        kind.is_socket()
    }
    #[cfg(not(unix))]
    {
        let _ = kind;
        false
    }
}
