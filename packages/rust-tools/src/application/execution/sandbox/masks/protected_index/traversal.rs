use super::index::{ProtectedPathIndex, ProtectedPathInventory};
use super::watcher::{DirectoryRecord, DirectoryWatcher, IndexChange, WatchChanges};
use super::IndexScanBudget;
use std::collections::{BTreeSet, HashMap};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::FileTypeExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

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

struct ExternalOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn resolve_external_tool(name: &str) -> Option<PathBuf> {
    ["/usr/local/bin", "/usr/bin", "/bin"]
        .into_iter()
        .map(Path::new)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn run_external_scan(
    executable: &Path,
    args: &[OsString],
    root: &Path,
    budget: &IndexScanBudget,
) -> io::Result<ExternalOutput> {
    budget.check()?;
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(root)
        .env_remove("RIPGREP_CONFIG_PATH")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("protected-path scanner stdout unavailable"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("protected-path scanner stderr unavailable"))?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });

    let status = loop {
        if budget.remaining().is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path external scan deadline elapsed",
            ));
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(2));
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| io::Error::other("protected-path scanner stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| io::Error::other("protected-path scanner stderr reader panicked"))??;
    Ok(ExternalOutput {
        status,
        stdout,
        stderr,
    })
}

fn normalized_relative(raw: &[u8]) -> io::Result<PathBuf> {
    let relative = PathBuf::from(OsString::from_vec(raw.to_vec()));
    let relative = relative
        .strip_prefix(".")
        .unwrap_or(relative.as_path())
        .to_path_buf();
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "protected-path scanner returned an unsafe path",
        ));
    }
    Ok(relative)
}

fn outermost_protected_relative(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .filter(|ancestor| crate::core::protected_paths::is_protected_relative(ancestor))
        .last()
        .map(Path::to_path_buf)
}

fn build_external_inventory(
    root: &Path,
    budget: &mut IndexScanBudget,
) -> io::Result<Option<ProtectedPathIndex>> {
    #[cfg(feature = "test-protected-index")]
    if budget.has_entry_limit() {
        return Ok(None);
    }

    let Some(rg) = resolve_external_tool("rg") else {
        return Ok(None);
    };
    let Some(find) = resolve_external_tool("find") else {
        return Ok(None);
    };

    let mut rg_args = vec![
        OsString::from("--files"),
        OsString::from("--hidden"),
        OsString::from("--no-ignore"),
        OsString::from("--no-config"),
        OsString::from("--null"),
    ];
    for directory in crate::core::protected_paths::PROTECTED_DIRECTORIES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("**/{directory}/**")));
    }
    for file in crate::core::protected_paths::PROTECTED_FILES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("**/{file}")));
    }
    rg_args.push(OsString::from("--glob"));
    rg_args.push(OsString::from("**/.env.*"));
    rg_args.push(OsString::from("--glob"));
    rg_args.push(OsString::from("!**/.env.example"));
    for directory in crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("!**/{directory}/**")));
    }
    rg_args.push(OsString::from("--"));
    rg_args.push(OsString::from("."));

    let rg_output = run_external_scan(&rg, &rg_args, root, budget)?;
    let rg_code = rg_output.status.code();
    if !matches!(rg_code, Some(0) | Some(1)) {
        let detail = String::from_utf8_lossy(&rg_output.stderr);
        return Err(io::Error::other(format!(
            "ripgrep protected-path scan failed with status {:?}: {}",
            rg_code,
            detail.trim()
        )));
    }

    let mut protected_paths = BTreeSet::new();
    let mut reported_entries = 0usize;
    for raw in rg_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        budget.check()?;
        reported_entries = reported_entries.saturating_add(1);
        let relative = normalized_relative(raw)?;
        let protected = outermost_protected_relative(&relative).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ripgrep protected-path scan returned a non-protected match",
            )
        })?;
        protected_paths.insert(root.join(protected));
    }

    let mut find_args = vec![OsString::from(".")];
    find_args.extend([
        OsString::from("("),
        OsString::from("-type"),
        OsString::from("d"),
        OsString::from("("),
    ]);
    for (index, directory) in crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES
        .iter()
        .enumerate()
    {
        if index > 0 {
            find_args.push(OsString::from("-o"));
        }
        find_args.extend([OsString::from("-name"), OsString::from(directory)]);
    }
    find_args.extend([
        OsString::from(")"),
        OsString::from("-prune"),
        OsString::from(")"),
        OsString::from("-o"),
        OsString::from("("),
        OsString::from("-type"),
        OsString::from("s"),
        OsString::from("-o"),
        OsString::from("-type"),
        OsString::from("l"),
        OsString::from(")"),
        OsString::from("-print0"),
    ]);
    let find_output = run_external_scan(&find, &find_args, root, budget)?;
    if !find_output.status.success() {
        let detail = String::from_utf8_lossy(&find_output.stderr);
        return Err(io::Error::other(format!(
            "find protected-path special-file scan failed with status {:?}: {}",
            find_output.status.code(),
            detail.trim()
        )));
    }

    for raw in find_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        budget.check()?;
        reported_entries = reported_entries.saturating_add(1);
        let relative = normalized_relative(raw)?;
        let path = root.join(&relative);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let protected_ancestor = outermost_protected_relative(&relative);
        if metadata.file_type().is_symlink() {
            if let Some(protected) = protected_ancestor {
                protected_paths.insert(root.join(protected));
            }
            continue;
        }
        if metadata.file_type().is_socket() {
            protected_paths.insert(
                protected_ancestor
                    .map(|protected| root.join(protected))
                    .unwrap_or(path),
            );
        }
    }

    Ok(Some(ProtectedPathIndex {
        inventory: std::sync::Mutex::new(ProtectedPathInventory {
            directories: HashMap::new(),
            protected_paths,
            scanned_entries: reported_entries,
            watcher: None,
        }),
    }))
}

pub(super) fn scan(root: &Path, budget: &mut IndexScanBudget) -> io::Result<ProtectedPathIndex> {
    if let Some(index) = build_external_inventory(root, budget)? {
        return Ok(index);
    }
    scan_with_walker(root, budget)
}

fn scan_with_walker(root: &Path, budget: &mut IndexScanBudget) -> io::Result<ProtectedPathIndex> {
    budget.check()?;
    let watcher = None;
    let root_directory = open_directory(root)?;
    let mut inventory = ProtectedPathInventory {
        directories: HashMap::new(),
        protected_paths: BTreeSet::new(),
        scanned_entries: 0,
        watcher,
    };
    scan_directory_into(root, root, root_directory, &mut inventory, budget)?;
    inventory.scanned_entries = budget.scanned_entries();

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
        if crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES
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
