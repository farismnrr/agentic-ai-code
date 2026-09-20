use std::collections::{HashMap, HashSet};
use std::ffi::{CString, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(super) struct DirectoryRecord {
    pub(super) child_directories: Vec<OsString>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum IndexChangeKind {
    ExactPath,
    Subtree,
}

#[derive(Clone)]
pub(super) struct IndexChange {
    pub(super) path: PathBuf,
    pub(super) kind: IndexChangeKind,
}

pub(super) enum WatchChanges {
    Clean,
    Changes(Vec<IndexChange>),
    FullScan(&'static str),
}

struct RawInotifyEvent {
    watch_descriptor: i32,
    mask: u32,
    name: OsString,
}

pub(super) struct DirectoryWatcher {
    fd: File,
    watch_paths: HashMap<i32, PathBuf>,
    expected_ignored: HashSet<i32>,
}

impl DirectoryWatcher {
    pub(super) fn new() -> io::Result<Self> {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            fd: unsafe { File::from_raw_fd(fd) },
            watch_paths: HashMap::new(),
            expected_ignored: HashSet::new(),
        })
    }

    pub(super) fn watch_directory(&mut self, directory: &File, path: &Path) -> io::Result<()> {
        let proc_fd_path = CString::new(format!("/proc/self/fd/{}", directory.as_raw_fd()))
            .expect("proc fd path contains no NUL bytes");
        let mask = libc::IN_ATTRIB
            | libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_DELETE_SELF
            | libc::IN_MOVE_SELF
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO
            | libc::IN_UNMOUNT
            | libc::IN_ONLYDIR;
        let watch =
            unsafe { libc::inotify_add_watch(self.fd.as_raw_fd(), proc_fd_path.as_ptr(), mask) };
        if watch < 0 {
            return Err(io::Error::last_os_error());
        }
        let watch = watch as i32;
        self.expected_ignored.remove(&watch);
        if let Some(existing) = self.watch_paths.get(&watch) {
            if existing != path {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path watch descriptor was reused ambiguously",
                ));
            }
        } else {
            self.watch_paths.insert(watch, path.to_path_buf());
        }
        Ok(())
    }

    pub(super) fn remove_watches_under(&mut self, prefix: &Path) -> io::Result<()> {
        let watches = self
            .watch_paths
            .iter()
            .filter_map(|(descriptor, path)| path.starts_with(prefix).then_some(*descriptor))
            .collect::<Vec<_>>();
        for descriptor in watches {
            self.expected_ignored.insert(descriptor);
            let result = unsafe { libc::inotify_rm_watch(self.fd.as_raw_fd(), descriptor) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if !matches!(
                    error.raw_os_error(),
                    Some(libc::EINVAL) | Some(libc::ENOENT)
                ) {
                    self.expected_ignored.remove(&descriptor);
                    return Err(error);
                }
            }
            self.watch_paths.remove(&descriptor);
        }
        Ok(())
    }

    pub(super) fn drain_changes(&mut self, root: &Path) -> io::Result<WatchChanges> {
        let events = self.read_pending_events()?;
        if events.is_empty() {
            return Ok(WatchChanges::Clean);
        }

        let removed_directories = events
            .iter()
            .filter(|event| {
                event.mask & (libc::IN_DELETE | libc::IN_MOVED_FROM) != 0
                    && event.mask & libc::IN_ISDIR != 0
            })
            .filter_map(|event| self.event_path(event))
            .collect::<Vec<_>>();
        let mut changes = Vec::new();
        for event in events {
            if event.mask & (libc::IN_Q_OVERFLOW | libc::IN_UNMOUNT) != 0 {
                return Ok(WatchChanges::FullScan("watch overflow or unmount"));
            }
            if event.mask & libc::IN_IGNORED != 0 {
                if self.expected_ignored.remove(&event.watch_descriptor) {
                    continue;
                }
                let Some(watched_path) = self.watch_paths.get(&event.watch_descriptor).cloned()
                else {
                    return Ok(WatchChanges::FullScan("unknown ignored watch"));
                };
                if removed_directories
                    .iter()
                    .any(|prefix| watched_path.starts_with(prefix))
                {
                    self.watch_paths.remove(&event.watch_descriptor);
                    continue;
                }
                return Ok(WatchChanges::FullScan("unexpected watch removal"));
            }
            if event.mask & (libc::IN_DELETE_SELF | libc::IN_MOVE_SELF) != 0 {
                let Some(watched_path) = self.watch_paths.get(&event.watch_descriptor).cloned()
                else {
                    return Ok(WatchChanges::FullScan("unknown self-watch event"));
                };
                if removed_directories
                    .iter()
                    .any(|prefix| watched_path.starts_with(prefix))
                {
                    self.remove_watches_under(&watched_path)?;
                    continue;
                }
                return Ok(WatchChanges::FullScan("root or watched directory moved"));
            }
            if event.name.is_empty() {
                return Ok(WatchChanges::FullScan("watch event omitted its path"));
            }
            let Some(parent) = self.watch_paths.get(&event.watch_descriptor) else {
                return Ok(WatchChanges::FullScan("event came from an unknown watch"));
            };
            let path = parent.join(&event.name);
            if !path.starts_with(root) {
                return Ok(WatchChanges::FullScan("watch event escaped indexed root"));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| io::Error::other("watch event escaped indexed root"))?;
            let protected_name = crate::core::protected_paths::may_be_protected_entry(&event.name)
                && crate::core::protected_paths::is_protected_relative(relative);
            if protected_name {
                changes.push(IndexChange {
                    path,
                    kind: IndexChangeKind::ExactPath,
                });
            } else if event.mask & libc::IN_ISDIR != 0 {
                changes.push(IndexChange {
                    path,
                    kind: IndexChangeKind::Subtree,
                });
            }
        }

        Ok(if changes.is_empty() {
            WatchChanges::Clean
        } else {
            WatchChanges::Changes(coalesce_changes(changes))
        })
    }

    fn read_pending_events(&mut self) -> io::Result<Vec<RawInotifyEvent>> {
        let mut events = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let count = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if count > 0 {
                let count = count as usize;
                let header_len = std::mem::size_of::<libc::inotify_event>();
                let mut offset = 0usize;
                while offset < count {
                    if count.saturating_sub(offset) < header_len {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "truncated protected-path watch event",
                        ));
                    }
                    let event = unsafe {
                        std::ptr::read_unaligned(
                            buffer.as_ptr().add(offset).cast::<libc::inotify_event>(),
                        )
                    };
                    let record_len = header_len.saturating_add(event.len as usize);
                    if record_len < header_len || record_len > count.saturating_sub(offset) {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "malformed protected-path watch event",
                        ));
                    }
                    let name = if event.len == 0 {
                        OsString::new()
                    } else {
                        let raw = &buffer[offset + header_len..offset + record_len];
                        let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
                        OsString::from_vec(raw[..end].to_vec())
                    };
                    events.push(RawInotifyEvent {
                        watch_descriptor: event.wd,
                        mask: event.mask,
                        name,
                    });
                    offset = offset.saturating_add(record_len);
                }
                continue;
            }
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "protected-path watch descriptor closed",
                ));
            }
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(events);
            }
            return Err(error);
        }
    }

    fn event_path(&self, event: &RawInotifyEvent) -> Option<PathBuf> {
        let parent = self.watch_paths.get(&event.watch_descriptor)?;
        if event.name.is_empty() {
            Some(parent.clone())
        } else {
            Some(parent.join(&event.name))
        }
    }
}

fn coalesce_changes(mut changes: Vec<IndexChange>) -> Vec<IndexChange> {
    changes.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| match (left.kind, right.kind) {
                (IndexChangeKind::Subtree, IndexChangeKind::ExactPath) => std::cmp::Ordering::Less,
                (IndexChangeKind::ExactPath, IndexChangeKind::Subtree) => {
                    std::cmp::Ordering::Greater
                }
                _ => std::cmp::Ordering::Equal,
            })
    });
    changes.dedup_by(|left, right| left.path == right.path && left.kind == right.kind);
    let mut coalesced: Vec<IndexChange> = Vec::new();
    for change in changes {
        if coalesced.iter().any(|existing| {
            matches!(existing.kind, IndexChangeKind::Subtree)
                && change.path.starts_with(&existing.path)
        }) {
            continue;
        }
        if matches!(change.kind, IndexChangeKind::Subtree) {
            coalesced.retain(|existing| !existing.path.starts_with(&change.path));
        }
        coalesced.push(change);
    }
    coalesced
}
