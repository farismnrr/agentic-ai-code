use super::index::{ProtectedPathIndex, ProtectedPathInventory};
use super::watcher::{
    DirectoryRecord, DirectorySignature, DirectoryWatcher, IndexChange, WatchChanges,
};
use super::IndexScanBudget;
use std::collections::{BTreeSet, HashMap};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

struct DirectoryStream(*mut libc::DIR);

impl DirectoryStream {
    fn open(directory: &File) -> io::Result<Self> {
        let duplicate = unsafe { libc::dup(directory.as_raw_fd()) };
        if duplicate < 0 {
            return Err(io::Error::last_os_error());
        }
        let stream = unsafe { libc::fdopendir(duplicate) };
        if stream.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::close(duplicate) };
            return Err(error);
        }
        Ok(Self(stream))
    }

    fn next(&mut self, directory: &File) -> io::Result<Option<ScannedEntry>> {
        loop {
            unsafe { *libc::__errno_location() = 0 };
            let entry = unsafe { libc::readdir(self.0) };
            if entry.is_null() {
                let error = unsafe { *libc::__errno_location() };
                return if error == 0 {
                    Ok(None)
                } else {
                    Err(io::Error::from_raw_os_error(error))
                };
            }

            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            let name = OsString::from_vec(name.to_bytes().to_vec());
            let entry_type = unsafe { (*entry).d_type };
            if entry_type == libc::DT_SOCK {
                return Ok(Some(ScannedEntry {
                    name,
                    directory: false,
                    socket: true,
                    device: 0,
                    inode: 0,
                }));
            }
            if entry_type != libc::DT_DIR && entry_type != libc::DT_UNKNOWN {
                return Ok(Some(ScannedEntry {
                    name,
                    directory: false,
                    socket: false,
                    device: 0,
                    inode: 0,
                }));
            }

            // Most entries in developer workspaces are regular files or
            // symlinks. Their dirent type is sufficient: the parent
            // directory signature is checked after traversal, so concurrent
            // replacement invalidates this scan. Stat only directories (to
            // preserve the openat identity check) and filesystems that report
            // DT_UNKNOWN.
            let name_c = CString::new(name.as_bytes()).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid directory entry")
            })?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::zeroed();
            if unsafe {
                libc::fstatat(
                    directory.as_raw_fd(),
                    name_c.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } < 0
            {
                return Err(io::Error::last_os_error());
            }
            let stat = unsafe { stat.assume_init() };
            let kind = stat.st_mode & libc::S_IFMT;
            return Ok(Some(ScannedEntry {
                name,
                directory: kind == libc::S_IFDIR,
                socket: kind == libc::S_IFSOCK,
                device: stat.st_dev,
                inode: stat.st_ino,
            }));
        }
    }
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { libc::closedir(self.0) };
        }
    }
}

struct ScannedEntry {
    name: OsString,
    directory: bool,
    socket: bool,
    device: u64,
    inode: u64,
}

struct ScanFrame {
    path: PathBuf,
    directory: File,
    signature: DirectorySignature,
    stream: DirectoryStream,
    child_directories: Vec<OsString>,
}

fn open_directory(path: &Path) -> io::Result<File> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid directory path"))?;
    let descriptor = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn open_child_directory(parent: &File, name: &OsStr) -> io::Result<File> {
    let name = CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid directory entry"))?;
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn scan(root: &Path, budget: &mut IndexScanBudget) -> io::Result<ProtectedPathIndex> {
    budget.check()?;
    let watcher = DirectoryWatcher::new().inspect_err(|error| {
        tracing::warn!(
            event = "relay.sandbox.stage",
            stage = "protected_path_watch",
            outcome = "unavailable",
            error_kind = ?error.kind(),
        );
    })?;
    let root_directory = open_directory(root)?;
    let mut inventory = ProtectedPathInventory {
        directories: HashMap::new(),
        protected_paths: BTreeSet::new(),
        scanned_entries: 0,
        watcher,
    };
    scan_directory_into(root, root, root_directory, &mut inventory, budget)?;
    inventory.scanned_entries = budget.scanned_entries();

    match inventory.watcher.drain_changes(root)? {
        WatchChanges::Clean => {}
        WatchChanges::Changes(_) | WatchChanges::FullScan(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed during protected-path discovery",
            ));
        }
    }

    Ok(ProtectedPathIndex {
        inventory: std::sync::Mutex::new(inventory),
    })
}

fn install_directory_watch(
    watcher: &mut DirectoryWatcher,
    directory: &File,
    path: &Path,
) -> io::Result<()> {
    watcher
        .watch_directory(directory, path)
        .inspect_err(|error| {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_watch",
                outcome = "unavailable",
                error_kind = ?error.kind(),
            );
        })
}

fn scan_directory_into(
    root: &Path,
    path: &Path,
    directory: File,
    inventory: &mut ProtectedPathInventory,
    budget: &mut IndexScanBudget,
) -> io::Result<()> {
    install_directory_watch(&mut inventory.watcher, &directory, path)?;
    let signature = DirectorySignature::read(&directory)?;
    let stream = DirectoryStream::open(&directory)?;
    let mut stack = vec![ScanFrame {
        path: path.to_path_buf(),
        directory,
        signature,
        stream,
        child_directories: Vec::new(),
    }];

    while !stack.is_empty() {
        let parent_path = stack.last().expect("scan stack is non-empty").path.clone();
        let next = {
            let frame = stack.last_mut().expect("scan stack is non-empty");
            frame.stream.next(&frame.directory)?
        };
        let Some(entry) = next else {
            let frame = stack.pop().expect("scan stack is non-empty");
            if DirectorySignature::read(&frame.directory)? != frame.signature {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace changed during protected-path discovery",
                ));
            }
            inventory.directories.insert(
                frame.path,
                DirectoryRecord {
                    signature: frame.signature,
                    child_directories: frame.child_directories,
                },
            );
            continue;
        };

        budget.consume_entry()?;
        let path = parent_path.join(&entry.name);
        let relative = path
            .strip_prefix(root)
            .map_err(|_| io::Error::other("protected-path traversal escaped workspace"))?;
        let may_be_protected = crate::core::protected_paths::may_be_protected_entry(&entry.name);
        if may_be_protected && crate::core::protected_paths::is_protected_relative(relative) {
            inventory.protected_paths.insert(path);
            continue;
        }
        if entry.socket {
            inventory.protected_paths.insert(path);
            continue;
        }
        if !entry.directory {
            continue;
        }

        let parent = stack.last().expect("scan stack is non-empty");
        let child = open_child_directory(&parent.directory, &entry.name)?;
        let child_signature = DirectorySignature::read(&child)?;
        if (entry.device != 0 || entry.inode != 0)
            && (child_signature.device != entry.device || child_signature.inode != entry.inode)
        {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed during protected-path discovery",
            ));
        }
        // Watch every traversed directory before opening its entry stream, so
        // protected files created deeper in the tree invalidate this index.
        install_directory_watch(&mut inventory.watcher, &child, &path)?;
        let stream = DirectoryStream::open(&child)?;
        stack
            .last_mut()
            .expect("scan stack is non-empty")
            .child_directories
            .push(entry.name);
        stack.push(ScanFrame {
            path,
            directory: child,
            signature: child_signature,
            stream,
            child_directories: Vec::new(),
        });
    }
    Ok(())
}

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
        pending = match inventory.watcher.drain_changes(root)? {
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
        prune_prefix(inventory, &change.path)?;
        let child = open_child_directory(&parent, &name)?;
        if child_identity(&child)? != (stat.st_dev, stat.st_ino) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "directory changed during protected-path reconciliation",
            ));
        }
        scan_directory_into(root, &change.path, child, inventory, budget)?;
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
    let signature = DirectorySignature::read(directory)?;
    Ok((signature.device, signature.inode))
}

fn prune_prefix(inventory: &mut ProtectedPathInventory, prefix: &Path) -> io::Result<()> {
    inventory.watcher.remove_watches_under(prefix)?;
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
    record.signature = DirectorySignature::read(parent)?;
    Ok(())
}
