use super::index::{ProtectedPathIndex, ProtectedPathInventory};
use super::traversal::{open_child_directory, open_directory, scan_directory_into};
use super::watcher::{IndexChange, WatchChanges};
use super::IndexScanBudget;
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const MAX_RECONCILIATION_PASSES: usize = 32;

pub(super) fn reconcile(
    root: &Path,
    index: &ProtectedPathIndex,
    mut pending: Vec<IndexChange>,
    budget: &mut IndexScanBudget,
) -> io::Result<()> {
    let mut inventory = index
        .inventory
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut passes = 0usize;
    loop {
        budget.check()?;
        if passes >= MAX_RECONCILIATION_PASSES {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path event queue did not quiesce",
            ));
        }
        passes += 1;
        for change in pending {
            apply_change(root, &mut inventory, change, budget)?;
        }
        let watcher = inventory.watcher.as_mut().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                "snapshot-only protected-path index cannot reconcile watcher events",
            )
        })?;
        pending = match watcher.drain_changes(root)? {
            WatchChanges::Clean => return Ok(()),
            WatchChanges::Changes(changes) => changes,
            WatchChanges::FullScan(reason) => {
                return Err(io::Error::new(io::ErrorKind::Interrupted, reason));
            }
        };
    }
}

fn apply_change(
    root: &Path,
    inventory: &mut ProtectedPathInventory,
    change: IndexChange,
    budget: &mut IndexScanBudget,
) -> io::Result<()> {
    budget.check()?;
    if !change.path.starts_with(root) || change.path == root {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "protected-path event is outside the indexed root",
        ));
    }
    let (parent, name) = open_parent_beneath(root, &change.path)?;
    let Some(stat) = stat_child(&parent, &name)? else {
        prune_prefix(inventory, &change.path)?;
        update_parent_record(inventory, &parent, &change.path, false)?;
        return Ok(());
    };

    let relative = change
        .path
        .strip_prefix(root)
        .map_err(|_| io::Error::other("protected-path event escaped indexed root"))?;
    let protected = crate::core::protected_paths::may_be_protected_entry(&name)
        && crate::core::protected_paths::is_protected_relative(relative);
    let kind = stat.st_mode & libc::S_IFMT;
    if kind == libc::S_IFDIR {
        if protected {
            prune_prefix(inventory, &change.path)?;
            inventory.protected_paths.insert(change.path.clone());
            update_parent_record(inventory, &parent, &change.path, false)?;
            return Ok(());
        }
        if crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES
            .iter()
            .any(|directory| name == OsStr::new(directory))
        {
            prune_prefix(inventory, &change.path)?;
            update_parent_record(inventory, &parent, &change.path, false)?;
            return Ok(());
        }
        prune_prefix(inventory, &change.path)?;
        let child = open_child_directory(&parent, &name)?;
        if child_identity(&child)? != (stat.st_dev, stat.st_ino) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "directory changed during protected-path reconciliation",
            ));
        }
        scan_directory_into(root, &change.path, child, inventory, budget, false)?;
        update_parent_record(inventory, &parent, &change.path, true)?;
        return Ok(());
    }

    prune_prefix(inventory, &change.path)?;
    if protected || kind == libc::S_IFSOCK {
        inventory.protected_paths.insert(change.path.clone());
    }
    update_parent_record(inventory, &parent, &change.path, false)?;
    Ok(())
}

fn open_directory_beneath(root: &Path, directory: &Path) -> io::Result<File> {
    let relative = directory
        .strip_prefix(root)
        .map_err(|_| io::Error::other("protected-path directory escaped indexed root"))?;
    let mut current = open_directory(root)?;
    for component in relative.components() {
        match component {
            std::path::Component::Normal(name) => {
                current = open_child_directory(&current, name)?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "protected-path event contains an unsafe component",
                ));
            }
        }
    }
    Ok(current)
}

fn open_parent_beneath(root: &Path, path: &Path) -> io::Result<(File, OsString)> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("protected-path event has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("protected-path event has no name"))?
        .to_os_string();
    Ok((open_directory_beneath(root, parent)?, name))
}

fn stat_child(parent: &File, name: &OsStr) -> io::Result<Option<libc::stat>> {
    let name = CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid event name"))?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::zeroed();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(error);
    }
    Ok(Some(unsafe { stat.assume_init() }))
}

fn child_identity(directory: &File) -> io::Result<(u64, u64)> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::zeroed();
    if unsafe { libc::fstat(directory.as_raw_fd(), stat.as_mut_ptr()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let stat = unsafe { stat.assume_init() };
    if stat.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "protected-path index entry changed type",
        ));
    }
    Ok((stat.st_dev, stat.st_ino))
}

fn prune_prefix(inventory: &mut ProtectedPathInventory, prefix: &Path) -> io::Result<()> {
    if let Some(watcher) = inventory.watcher.as_mut() {
        watcher.remove_watches_under(prefix)?;
    }
    inventory
        .directories
        .retain(|path, _| !path.starts_with(prefix));
    inventory
        .protected_paths
        .retain(|path| !path.starts_with(prefix));
    Ok(())
}

fn update_parent_record(
    inventory: &mut ProtectedPathInventory,
    parent: &File,
    path: &Path,
    include_directory: bool,
) -> io::Result<()> {
    let parent_path = path
        .parent()
        .ok_or_else(|| io::Error::other("protected-path event has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("protected-path event has no name"))?
        .to_os_string();
    let record = inventory.directories.get_mut(parent_path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Interrupted,
            "protected-path parent inventory is missing",
        )
    })?;
    if include_directory {
        if !record.child_directories.iter().any(|child| child == &name) {
            record.child_directories.push(name);
        }
    } else {
        record.child_directories.retain(|child| child != &name);
    }
    let _ = parent;
    Ok(())
}
