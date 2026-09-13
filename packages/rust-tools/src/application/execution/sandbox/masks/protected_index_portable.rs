//! Portable fail-closed discovery used only where Bubblewrap execution is unavailable.
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_PROTECTED_SCAN_ENTRIES: usize = 500_000;
const CONTROL_CHECK_INTERVAL: usize = 64;

pub(super) struct ProtectedPathIndex {
    pub(super) protected_paths: BTreeSet<PathBuf>,
    pub(super) scanned_entries: usize,
}

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
) -> io::Result<(PathBuf, Arc<ProtectedPathIndex>, bool)> {
    let root = std::fs::canonicalize(root)?;
    let mut pending = vec![root.clone()];
    let mut protected = BTreeSet::new();
    let mut scanned_entries = 0usize;
    while let Some(directory) = pending.pop() {
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
        Arc::new(ProtectedPathIndex {
            protected_paths: protected,
            scanned_entries,
        }),
        false,
    ))
}

pub(super) fn prime(root: &Path) -> io::Result<usize> {
    let (_, index, _) = discover(root, None)?;
    Ok(index.scanned_entries)
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
