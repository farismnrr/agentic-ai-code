use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::collections::BinaryHeap;
use std::path::Path;

#[cfg(target_os = "linux")]
use std::ffi::{CStr, CString, OsStr};
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

pub const DEFAULT_DIRECTORY_DEPTH: usize = 2;
pub const MAX_DIRECTORY_DEPTH: usize = 4;
pub const DEFAULT_DIRECTORY_ENTRIES: usize = 100;
pub const MAX_DIRECTORY_ENTRIES: usize = 100;
pub const MAX_DIRECTORY_SCAN_ENTRIES: usize = 4_096;
pub const MAX_DIRECTORY_RESULT_BYTES: usize = 256 * 1024;

#[derive(Debug, Serialize)]
pub struct DirectoryListResult {
    path: String,
    entries: Vec<DirectoryListEntry>,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct DirectoryListEntry {
    path: String,
    #[serde(rename = "type")]
    kind: &'static str,
}

pub fn directory_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<DirectoryListResult, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let requested_path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
    let depth = arguments
        .get("depth")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_DIRECTORY_DEPTH)
        .min(MAX_DIRECTORY_DEPTH);
    let max_entries = arguments
        .get("max_entries")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_DIRECTORY_ENTRIES)
        .clamp(1, MAX_DIRECTORY_ENTRIES);

    let directory =
        resolve_existing_path(&execution_root, cwd, requested_path, EntryKind::Directory)?;
    let directory = SecureDirectory::open_relative(&execution_root, &directory)?;
    let mut state = TraversalState {
        entries: Vec::new(),
        max_entries,
        truncated: false,
    };
    visit_directory(&directory, Path::new(""), depth, &mut state)?;

    Ok(DirectoryListResult {
        path: requested_path.to_owned(),
        entries: state.entries,
        truncated: state.truncated,
    })
}

struct TraversalState {
    entries: Vec<DirectoryListEntry>,
    max_entries: usize,
    truncated: bool,
}

fn visit_directory(
    directory: &SecureDirectory,
    relative: &Path,
    remaining_depth: usize,
    state: &mut TraversalState,
) -> Result<(), McpError> {
    if remaining_depth == 0 || state.truncated {
        return Ok(());
    }

    let children =
        directory.read_entries(MAX_DIRECTORY_SCAN_ENTRIES, "directory scan exceeds maximum")?;

    for child in children {
        if state.entries.len() >= state.max_entries {
            state.truncated = true;
            break;
        }

        // MCP paths are UTF-8 strings. Native entries that cannot be represented
        // exactly are omitted, including any descendants below such a directory;
        // never collapse distinct native names through lossy conversion.
        if child.name.to_str().is_none() {
            continue;
        }
        let child_relative = relative.join(&child.name);
        let file_type = child.file_type;
        let kind = if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_dir() {
            "directory"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };
        state.entries.push(DirectoryListEntry {
            path: display_relative_path(&child_relative),
            kind,
        });

        if file_type.is_dir() && remaining_depth > 1 {
            let child_directory = directory.open_child(&child.name)?;
            visit_directory(
                &child_directory,
                &child_relative,
                remaining_depth - 1,
                state,
            )?;
            if state.truncated {
                break;
            }
        }
    }

    Ok(())
}

fn display_relative_path(path: &Path) -> String {
    path.to_str()
        .expect("directory traversal filters non-UTF-8 components")
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn inaccessible_directory_error() -> McpError {
    McpError::InvalidRequest("directory is inaccessible".into())
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct SecureDirectory {
    fd: OwnedFd,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct DirectoryEntry {
    name: std::ffi::OsString,
    file_type: EntryType,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug)]
struct EntryType {
    symlink: bool,
    directory: bool,
    regular: bool,
}

#[cfg(target_os = "linux")]
impl EntryType {
    fn is_symlink(self) -> bool {
        self.symlink
    }
    fn is_dir(self) -> bool {
        self.directory
    }
    fn is_file(self) -> bool {
        self.regular
    }
}

#[cfg(target_os = "linux")]
impl SecureDirectory {
    fn open_relative(root: &Path, target: &Path) -> Result<Self, McpError> {
        let relative = target
            .strip_prefix(root)
            .map_err(|_| McpError::InvalidRequest("path is outside execution root".into()))?;
        let root_c = CString::new(root.as_os_str().as_bytes())
            .map_err(|_| McpError::InvalidRequest("execution root is invalid".into()))?;
        let root_fd = unsafe {
            libc::open(
                root_c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if root_fd < 0 {
            return Err(inaccessible_directory_error());
        }
        let mut directory = Self {
            fd: unsafe { OwnedFd::from_raw_fd(root_fd) },
        };
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                return Err(McpError::InvalidRequest(
                    "path is not a relative directory".into(),
                ));
            };
            directory = directory.open_child(component)?;
        }
        Ok(directory)
    }

    fn open_child(&self, name: &OsStr) -> Result<Self, McpError> {
        let name = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("directory entry is invalid".into()))?;
        let fd = unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(inaccessible_directory_error());
        }
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    fn read_entries(
        &self,
        limit: usize,
        limit_message: &str,
    ) -> Result<Vec<DirectoryEntry>, McpError> {
        let duplicate = unsafe { libc::dup(self.fd.as_raw_fd()) };
        if duplicate < 0 {
            return Err(inaccessible_directory_error());
        }
        let directory = unsafe { libc::fdopendir(duplicate) };
        if directory.is_null() {
            unsafe { libc::close(duplicate) };
            return Err(inaccessible_directory_error());
        }
        let mut names = Vec::new();
        loop {
            unsafe { *libc::__errno_location() = 0 };
            let entry = unsafe { libc::readdir(directory) };
            if entry.is_null() {
                let error = unsafe { *libc::__errno_location() };
                unsafe { libc::closedir(directory) };
                if error != 0 {
                    return Err(inaccessible_directory_error());
                }
                break;
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            if names.len() >= limit {
                unsafe { libc::closedir(directory) };
                return Err(McpError::InvalidRequest(limit_message.into()));
            }
            names.push(std::ffi::OsString::from_vec(name.to_bytes().to_vec()));
        }
        names.sort();
        names
            .into_iter()
            .map(|name| {
                let c_name =
                    CString::new(name.as_bytes()).map_err(|_| inaccessible_directory_error())?;
                let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
                if unsafe {
                    libc::fstatat(
                        self.fd.as_raw_fd(),
                        c_name.as_ptr(),
                        &mut stat,
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                } < 0
                {
                    return Err(inaccessible_directory_error());
                }
                let kind = stat.st_mode & libc::S_IFMT;
                Ok(DirectoryEntry {
                    name,
                    file_type: EntryType {
                        symlink: kind == libc::S_IFLNK,
                        directory: kind == libc::S_IFDIR,
                        regular: kind == libc::S_IFREG,
                    },
                })
            })
            .collect()
    }
}
