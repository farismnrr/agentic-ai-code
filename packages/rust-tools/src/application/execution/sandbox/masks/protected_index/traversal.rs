use super::index::{ProtectedPathIndex, ProtectedPathInventory};
pub(super) use super::reconciliation::reconcile;
use super::watcher::{DirectoryRecord, DirectoryWatcher};
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
                }));
            }
            if entry_type == libc::DT_DIR {
                return Ok(Some(ScannedEntry {
                    name,
                    directory: true,
                    socket: false,
                }));
            }
            if entry_type != libc::DT_UNKNOWN {
                return Ok(Some(ScannedEntry {
                    name,
                    directory: false,
                    socket: false,
                }));
            }

            // Filesystems that report DT_UNKNOWN need one no-follow stat to
            // classify the entry. Normal Linux developer filesystems provide
            // d_type, so ordinary directories avoid an extra fstatat syscall.
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
}

struct ScanFrame {
    path: PathBuf,
    directory: File,
    stream: DirectoryStream,
    child_directories: Vec<OsString>,
}

pub(super) fn open_directory(path: &Path) -> io::Result<File> {
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

pub(super) fn open_child_directory(parent: &File, name: &OsStr) -> io::Result<File> {
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
    let watcher = match DirectoryWatcher::new() {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_watch",
                outcome = "unavailable",
                error_kind = ?error.kind(),
            );
            return super::external_scan::build_external_inventory(root, budget)?.ok_or(error);
        }
    };
    scan_with_walker(root, budget, watcher)
}

fn scan_with_walker(
    root: &Path,
    budget: &mut IndexScanBudget,
    watcher: Option<DirectoryWatcher>,
) -> io::Result<ProtectedPathIndex> {
    budget.check()?;
    let root_directory = open_directory(root)?;
    let mut inventory = ProtectedPathInventory {
        directories: HashMap::new(),
        protected_paths: BTreeSet::new(),
        scanned_entries: 0,
        watcher,
    };
    scan_directory_into(root, root, root_directory, &mut inventory, budget, true)?;
    inventory.scanned_entries = budget.scanned_entries();

    if let Some(watcher) = inventory.watcher.as_mut() {
        match watcher.drain_changes(root)? {
            super::watcher::WatchChanges::Clean => {}
            super::watcher::WatchChanges::Changes(_)
            | super::watcher::WatchChanges::FullScan(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace changed during protected-path discovery",
                ));
            }
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

pub(super) fn scan_directory_into(
    root: &Path,
    path: &Path,
    directory: File,
    inventory: &mut ProtectedPathInventory,
    budget: &mut IndexScanBudget,
    skip_generated: bool,
) -> io::Result<()> {
    budget.consume_directory()?;
    if let Some(watcher) = inventory.watcher.as_mut() {
        install_directory_watch(watcher, &directory, path)?;
    }
    let stream = DirectoryStream::open(&directory)?;
    let mut stack = vec![ScanFrame {
        path: path.to_path_buf(),
        directory,
        stream,
        child_directories: Vec::new(),
    }];

    while !stack.is_empty() {
        let next = {
            let frame = stack.last_mut().expect("scan stack is non-empty");
            frame.stream.next(&frame.directory)?
        };
        let Some(entry) = next else {
            let frame = stack.pop().expect("scan stack is non-empty");
            if inventory.watcher.is_some() {
                inventory.directories.insert(
                    frame.path,
                    DirectoryRecord {
                        child_directories: frame.child_directories,
                    },
                );
            }
            continue;
        };

        budget.consume_entry()?;
        let may_be_protected = crate::core::protected_paths::may_be_protected_entry(&entry.name);
        if !entry.directory && !entry.socket && !may_be_protected {
            continue;
        }

        let path = stack
            .last()
            .expect("scan stack is non-empty")
            .path
            .join(&entry.name);
        if may_be_protected {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| io::Error::other("protected-path traversal escaped workspace"))?;
            if crate::core::protected_paths::is_protected_relative(relative) {
                inventory.protected_paths.insert(path);
                continue;
            }
        }
        if entry.socket {
            inventory.protected_paths.insert(path);
            continue;
        }
        if !entry.directory {
            continue;
        }
        if skip_generated
            && crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES
                .iter()
                .any(|directory| entry.name == OsStr::new(directory))
        {
            continue;
        }

        let parent = stack.last().expect("scan stack is non-empty");
        let child = open_child_directory(&parent.directory, &entry.name)?;
        budget.consume_directory()?;
        // The parent is already watched before its stream is read, so a
        // concurrent child replacement queues an invalidating event. Watch the
        // child before opening its stream so deeper changes are covered too.
        if let Some(watcher) = inventory.watcher.as_mut() {
            install_directory_watch(watcher, &child, &path)?;
        }
        let stream = DirectoryStream::open(&child)?;
        if inventory.watcher.is_some() {
            stack
                .last_mut()
                .expect("scan stack is non-empty")
                .child_directories
                .push(entry.name);
        }
        stack.push(ScanFrame {
            path,
            directory: child,
            stream,
            child_directories: Vec::new(),
        });
    }
    Ok(())
}
