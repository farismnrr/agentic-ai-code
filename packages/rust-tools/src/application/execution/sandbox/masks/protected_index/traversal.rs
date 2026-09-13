use super::{
    control_check, DirectoryRecord, DirectorySignature, DirectoryWatcher, ProtectedPathIndex,
    CONTROL_CHECK_INTERVAL, MAX_PROTECTED_SCAN_ENTRIES,
};
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

pub(super) fn scan(
    root: &Path,
    control: Option<&super::super::super::SpawnControl<'_>>,
) -> io::Result<ProtectedPathIndex> {
    control_check(control)?;
    let mut watcher = match DirectoryWatcher::new() {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_watch",
                outcome = "unavailable",
                error_kind = ?error.kind(),
            );
            None
        }
    };
    let root_directory = open_directory(root)?;
    install_directory_watch(&mut watcher, &root_directory);
    let root_signature = DirectorySignature::read(&root_directory)?;
    let root_stream = DirectoryStream::open(&root_directory)?;
    let mut stack = vec![ScanFrame {
        path: root.to_path_buf(),
        directory: root_directory,
        signature: root_signature,
        stream: root_stream,
        child_directories: Vec::new(),
    }];
    let mut directories = HashMap::new();
    let mut protected_paths = BTreeSet::new();
    let mut scanned_entries = 0usize;

    while !stack.is_empty() {
        let (path, directory) = {
            let frame = stack.last().expect("scan stack is non-empty");
            (frame.path.clone(), frame.directory.as_raw_fd())
        };
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
            directories.insert(
                frame.path,
                DirectoryRecord {
                    signature: frame.signature,
                    child_directories: frame.child_directories,
                },
            );
            continue;
        };

        scanned_entries = scanned_entries.saturating_add(1);
        if scanned_entries > MAX_PROTECTED_SCAN_ENTRIES {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_discovery",
                outcome = "failed",
                error_kind = ?io::ErrorKind::InvalidData,
                scanned_entries,
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "protected-path scan exceeds bounded workspace maximum",
            ));
        }
        if scanned_entries.is_multiple_of(CONTROL_CHECK_INTERVAL) {
            if let Err(error) = control_check(control) {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_discovery",
                    outcome = "failed",
                    error_kind = ?error.kind(),
                    scanned_entries,
                );
                return Err(error);
            }
        }

        let may_be_protected = crate::core::protected_paths::may_be_protected_entry(&entry.name);
        if may_be_protected
            && crate::core::protected_paths::is_protected_relative(&entry_path_relative(
                root,
                &path,
                &entry.name,
            )?)
        {
            protected_paths.insert(path.join(&entry.name));
            continue;
        }
        if entry.socket {
            protected_paths.insert(path.join(&entry.name));
            continue;
        }
        if !entry.directory {
            continue;
        }

        let parent = stack.last().expect("scan stack is non-empty");
        if parent.directory.as_raw_fd() != directory {
            return Err(io::Error::other("protected-path traversal lost its parent"));
        }
        let child = open_child_directory(&parent.directory, &entry.name)?;
        install_directory_watch(&mut watcher, &child);
        let signature = DirectorySignature::read(&child)?;
        if (entry.device != 0 || entry.inode != 0)
            && (signature.device != entry.device || signature.inode != entry.inode)
        {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed during protected-path discovery",
            ));
        }
        let stream = DirectoryStream::open(&child)?;
        let entry_path = path.join(&entry.name);
        stack
            .last_mut()
            .expect("scan stack is non-empty")
            .child_directories
            .push(entry.name);
        stack.push(ScanFrame {
            path: entry_path,
            directory: child,
            signature,
            stream,
            child_directories: Vec::new(),
        });
    }

    if let Some(watcher) = watcher.as_mut() {
        match watcher.has_events() {
            Ok(false) => {}
            Ok(true) => {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace changed during protected-path discovery",
                ));
            }
            Err(error) => {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_watch",
                    outcome = "failed",
                    error_kind = ?error.kind(),
                );
                return Err(error);
            }
        }
    }

    Ok(ProtectedPathIndex {
        directories,
        protected_paths,
        scanned_entries,
        watcher: watcher.map(std::sync::Mutex::new),
    })
}

fn install_directory_watch(watcher: &mut Option<DirectoryWatcher>, directory: &File) {
    let error = watcher
        .as_mut()
        .and_then(|watcher| watcher.watch_directory(directory).err());
    if let Some(error) = error {
        tracing::warn!(
            event = "relay.sandbox.stage",
            stage = "protected_path_watch",
            outcome = "unavailable",
            error_kind = ?error.kind(),
        );
        *watcher = None;
    }
}

fn entry_path_relative(root: &Path, directory: &Path, name: &OsStr) -> io::Result<PathBuf> {
    let mut relative = directory
        .strip_prefix(root)
        .map_err(|_| io::Error::other("protected-path scan escaped workspace"))?
        .to_path_buf();
    relative.push(name);
    Ok(relative)
}

struct ValidationFrame {
    path: PathBuf,
    directory: File,
    signature: DirectorySignature,
    child_directories: Vec<OsString>,
    next_child: usize,
}

pub(super) fn validate(
    root: &Path,
    index: &ProtectedPathIndex,
    control: Option<&super::super::super::SpawnControl<'_>>,
) -> io::Result<bool> {
    if let Err(error) = control_check(control) {
        tracing::warn!(
            event = "relay.sandbox.stage",
            stage = "protected_path_index_validation",
            outcome = "failed",
            error_kind = ?error.kind(),
            scanned_entries = index.scanned_entries,
        );
        return Err(error);
    }
    if index.watcher.is_some() {
        let fresh = index.is_fresh()?;
        if !fresh {
            tracing::info!(
                event = "relay.sandbox.stage",
                stage = "protected_path_index_validation",
                outcome = "invalidated",
                reason = "filesystem_event",
                scanned_entries = index.scanned_entries,
            );
        }
        return Ok(fresh);
    }
    let root_directory = match open_directory(root) {
        Ok(directory) => directory,
        Err(_) => return Ok(false),
    };
    let Some(root_record) = index.directories.get(root) else {
        return Ok(false);
    };
    let root_signature = DirectorySignature::read(&root_directory)?;
    if root_signature != root_record.signature {
        return Ok(false);
    }
    let mut stack = vec![ValidationFrame {
        path: root.to_path_buf(),
        directory: root_directory,
        signature: root_signature,
        child_directories: root_record.child_directories.clone(),
        next_child: 0,
    }];
    let mut steps = 0usize;
    let mut visited = 0usize;

    while !stack.is_empty() {
        steps = steps.saturating_add(1);
        if steps.is_multiple_of(CONTROL_CHECK_INTERVAL) {
            if let Err(error) = control_check(control) {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_index_validation",
                    outcome = "failed",
                    error_kind = ?error.kind(),
                    scanned_entries = index.scanned_entries,
                    checked_directories = visited,
                );
                return Err(error);
            }
        }
        let child = {
            let frame = stack.last_mut().expect("validation stack is non-empty");
            if frame.next_child < frame.child_directories.len() {
                let name = frame.child_directories[frame.next_child].clone();
                frame.next_child += 1;
                Some((frame.path.join(&name), name, frame.directory.as_raw_fd()))
            } else {
                None
            }
        };

        if let Some((path, name, parent_fd)) = child {
            let parent = stack.last().expect("validation stack is non-empty");
            if parent.directory.as_raw_fd() != parent_fd {
                return Ok(false);
            }
            let child_directory = match open_child_directory(&parent.directory, &name) {
                Ok(directory) => directory,
                Err(_) => return Ok(false),
            };
            let Some(record) = index.directories.get(&path) else {
                return Ok(false);
            };
            let signature = DirectorySignature::read(&child_directory)?;
            if signature != record.signature {
                return Ok(false);
            }
            stack.push(ValidationFrame {
                path,
                directory: child_directory,
                signature,
                child_directories: record.child_directories.clone(),
                next_child: 0,
            });
            continue;
        }

        let frame = stack.pop().expect("validation stack is non-empty");
        if DirectorySignature::read(&frame.directory)? != frame.signature {
            return Ok(false);
        }
        visited = visited.saturating_add(1);
    }

    Ok(visited == index.directories.len())
}
