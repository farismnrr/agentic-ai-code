//! Cached, fail-closed protected-path discovery for mounted workspace trees.
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_PROTECTED_SCAN_ENTRIES: usize = 500_000;
const CONTROL_CHECK_INTERVAL: usize = 64;
const MAX_CACHE_ROOTS: usize = 64;
const MAX_CACHED_INDEX_ENTRIES: usize = 1_000_000;
const FAILED_INDEX_RETRY: Duration = Duration::from_secs(30);

mod traversal;
use traversal::{scan, validate};

fn control_check(control: Option<&super::super::SpawnControl<'_>>) -> io::Result<()> {
    if let Some(control) = control {
        control.check()?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectorySignature {
    device: u64,
    inode: u64,
    mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    length: u64,
}

impl DirectorySignature {
    fn read(directory: &File) -> io::Result<Self> {
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
        Ok(Self {
            device: stat.st_dev,
            inode: stat.st_ino,
            mode: stat.st_mode,
            modified_seconds: stat.st_mtime,
            modified_nanoseconds: stat.st_mtime_nsec,
            changed_seconds: stat.st_ctime,
            changed_nanoseconds: stat.st_ctime_nsec,
            length: stat.st_size.max(0) as u64,
        })
    }
}

#[derive(Clone)]
struct DirectoryRecord {
    signature: DirectorySignature,
    child_directories: Vec<OsString>,
}

pub(super) struct ProtectedPathIndex {
    directories: HashMap<PathBuf, DirectoryRecord>,
    pub(super) protected_paths: BTreeSet<PathBuf>,
    pub(super) scanned_entries: usize,
}

enum CachedPathIndex {
    Ready(std::sync::Arc<ProtectedPathIndex>),
    Failed {
        kind: io::ErrorKind,
        retry_after: Instant,
    },
}

static PROTECTED_PATH_INDEX: OnceLock<Mutex<HashMap<PathBuf, CachedPathIndex>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<PathBuf, CachedPathIndex>> {
    PROTECTED_PATH_INDEX.get_or_init(|| Mutex::new(HashMap::new()))
}

fn make_cache_room(
    entries: &mut HashMap<PathBuf, CachedPathIndex>,
    current_root: &Path,
    new_entry_count: usize,
) {
    loop {
        let cached_entry_count = entries
            .values()
            .map(|entry| match entry {
                CachedPathIndex::Ready(index) => index.scanned_entries,
                CachedPathIndex::Failed { .. } => 0,
            })
            .sum::<usize>();
        if entries.len() < MAX_CACHE_ROOTS
            && cached_entry_count.saturating_add(new_entry_count) <= MAX_CACHED_INDEX_ENTRIES
        {
            return;
        }
        let Some(evict) = entries
            .keys()
            .find(|root| root.as_path() != current_root)
            .cloned()
        else {
            return;
        };
        entries.remove(&evict);
    }
}

fn lock_cache(
    control: Option<&super::super::SpawnControl<'_>>,
) -> io::Result<std::sync::MutexGuard<'static, HashMap<PathBuf, CachedPathIndex>>> {
    loop {
        match cache().try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => {
                control_check(control)?;
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

pub(super) fn discover(
    root: &Path,
    control: Option<&super::super::SpawnControl<'_>>,
) -> io::Result<(PathBuf, std::sync::Arc<ProtectedPathIndex>, bool)> {
    control_check(control)?;
    let root = std::fs::canonicalize(root)?;
    let mut entries = lock_cache(control)?;

    if let Some(CachedPathIndex::Failed { kind, retry_after }) = entries.get(&root) {
        if Instant::now() < *retry_after {
            return Err(io::Error::new(
                *kind,
                "protected-path discovery is unavailable",
            ));
        }
    }
    if let Some(CachedPathIndex::Ready(index)) = entries.get(&root) {
        match validate(&root, index, control) {
            Ok(true) => return Ok((root, index.clone(), true)),
            Ok(false) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(error)
            }
            Err(_) => {}
        }
    }

    entries.remove(&root);
    match scan(&root, control) {
        Ok(index) => {
            let index = std::sync::Arc::new(index);
            make_cache_room(&mut entries, &root, index.scanned_entries);
            entries.insert(root.clone(), CachedPathIndex::Ready(index.clone()));
            Ok((root, index, false))
        }
        Err(error) => {
            // Only the deterministic entry-count ceiling is memoized. Filesystem
            // permission and I/O errors may be transient and must be retried so
            // repaired workspaces do not stay blocked for the cache TTL.
            if error.kind() == io::ErrorKind::InvalidData {
                make_cache_room(&mut entries, &root, 0);
                entries.insert(
                    root,
                    CachedPathIndex::Failed {
                        kind: error.kind(),
                        retry_after: Instant::now() + FAILED_INDEX_RETRY,
                    },
                );
            }
            Err(error)
        }
    }
}

pub(super) fn prime(root: &Path) -> io::Result<usize> {
    let (_, index, _) = discover(root, None)?;
    Ok(index.scanned_entries)
}
