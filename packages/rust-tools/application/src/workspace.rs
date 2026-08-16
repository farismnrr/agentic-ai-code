use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_path, resolve_write_target, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

#[cfg(target_os = "linux")]
use std::ffi::{CStr, CString, OsStr};
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

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
            let child_directory = directory.open_child(&child)?;
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
    identity: FileIdentity,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
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
        let root_identity = path_identity(root)?;
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
        directory.verify_identity(root_identity)?;

        let mut expected_path = root.to_path_buf();
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                return Err(McpError::InvalidRequest(
                    "path is not a relative directory".into(),
                ));
            };
            expected_path.push(component);
            let expected_identity = path_identity(&expected_path)?;
            directory = directory.open_child_name(component, expected_identity)?;
        }
        directory.verify_identity(path_identity(target)?)?;
        Ok(directory)
    }

    fn open_child(&self, child: &DirectoryEntry) -> Result<Self, McpError> {
        self.open_child_name(&child.name, child.identity)
    }

    fn open_child_name(&self, name: &OsStr, expected: FileIdentity) -> Result<Self, McpError> {
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
        let directory = Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        };
        directory.verify_identity(expected)?;
        Ok(directory)
    }

    fn verify_identity(&self, expected: FileIdentity) -> Result<(), McpError> {
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        if unsafe { libc::fstat(self.fd.as_raw_fd(), &mut stat) } < 0 {
            return Err(inaccessible_directory_error());
        }
        if file_identity(&stat) != expected {
            return Err(McpError::InvalidRequest(
                "directory changed during traversal".into(),
            ));
        }
        Ok(())
    }

    fn open_regular_file(
        &self,
        name: &OsStr,
    ) -> Result<(std::fs::File, FileIdentity, u32), McpError> {
        let name = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("file name is invalid".into()))?;
        let fd = unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(McpError::InvalidRequest("file is inaccessible".into()));
        }
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        if unsafe { libc::fstat(fd, &mut stat) } < 0 || stat.st_mode & libc::S_IFMT != libc::S_IFREG
        {
            unsafe { libc::close(fd) };
            return Err(McpError::InvalidRequest(
                "file has an unsupported entry type".into(),
            ));
        }
        Ok((
            unsafe { std::fs::File::from_raw_fd(fd) },
            file_identity(&stat),
            stat.st_mode & 0o7777,
        ))
    }

    fn verify_regular_entry(&self, name: &OsStr, expected: FileIdentity) -> Result<(), McpError> {
        let name = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("file name is invalid".into()))?;
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        if unsafe {
            libc::fstatat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
            || stat.st_mode & libc::S_IFMT != libc::S_IFREG
            || file_identity(&stat) != expected
        {
            return Err(McpError::InvalidRequest("file changed during edit".into()));
        }
        Ok(())
    }

    fn create_temp_file(&self, content: &[u8], mode: u32) -> Result<CString, McpError> {
        let temp_name = format!(".relay-write-{}.tmp", uuid::Uuid::new_v4());
        let temp = CString::new(temp_name.as_bytes())
            .map_err(|_| McpError::Internal("failed to create temporary file name".into()))?;
        let fd = unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(McpError::InvalidRequest(
                "temporary file could not be created".into(),
            ));
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let write_result = (|| -> std::io::Result<()> {
            file.write_all(content)?;
            file.flush()?;
            if unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } < 0 {
                return Err(std::io::Error::last_os_error());
            }
            file.sync_all()?;
            Ok(())
        })();
        drop(file);
        if write_result.is_err() {
            unsafe { libc::unlinkat(self.fd.as_raw_fd(), temp.as_ptr(), 0) };
            return Err(McpError::InvalidRequest(
                "file content could not be written".into(),
            ));
        }
        Ok(temp)
    }

    fn atomic_replace_regular_file(
        &self,
        name: &OsStr,
        expected: FileIdentity,
        content: &[u8],
        mode: u32,
    ) -> Result<(), McpError> {
        self.verify_regular_entry(name, expected)?;
        let target = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("file name is invalid".into()))?;
        let temp = self.create_temp_file(content, mode)?;
        if let Err(error) = self.verify_regular_entry(name, expected) {
            unsafe { libc::unlinkat(self.fd.as_raw_fd(), temp.as_ptr(), 0) };
            return Err(error);
        }
        if unsafe {
            libc::renameat(
                self.fd.as_raw_fd(),
                temp.as_ptr(),
                self.fd.as_raw_fd(),
                target.as_ptr(),
            )
        } < 0
        {
            unsafe { libc::unlinkat(self.fd.as_raw_fd(), temp.as_ptr(), 0) };
            return Err(McpError::InvalidRequest(
                "file replacement could not be committed".into(),
            ));
        }
        self.sync_directory()
    }

    fn atomic_create_regular_file(
        &self,
        name: &OsStr,
        content: &[u8],
        mode: u32,
    ) -> Result<(), McpError> {
        let target = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("file name is invalid".into()))?;
        let temp = self.create_temp_file(content, mode)?;
        #[cfg(target_os = "linux")]
        let renamed = unsafe {
            libc::renameat2(
                self.fd.as_raw_fd(),
                temp.as_ptr(),
                self.fd.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if renamed < 0 {
            unsafe { libc::unlinkat(self.fd.as_raw_fd(), temp.as_ptr(), 0) };
            return Err(McpError::InvalidRequest(
                "file already exists or could not be created".into(),
            ));
        }
        self.sync_directory()
    }

    fn sync_directory(&self) -> Result<(), McpError> {
        if unsafe { libc::fsync(self.fd.as_raw_fd()) } < 0 {
            return Err(McpError::InvalidRequest(
                "directory durability sync failed".into(),
            ));
        }
        Ok(())
    }

    fn open_or_create_child(&self, name: &OsStr, create: bool) -> Result<Self, McpError> {
        let name_c = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("directory entry is invalid".into()))?;
        let mut fd = unsafe {
            libc::openat(
                self.fd.as_raw_fd(),
                name_c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 && create {
            let errno = std::io::Error::last_os_error().raw_os_error();
            if errno == Some(libc::ENOENT) {
                if unsafe { libc::mkdirat(self.fd.as_raw_fd(), name_c.as_ptr(), 0o755) } < 0 {
                    return Err(McpError::InvalidRequest(
                        "parent directory could not be created".into(),
                    ));
                }
                fd = unsafe {
                    libc::openat(
                        self.fd.as_raw_fd(),
                        name_c.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
            }
        }
        if fd < 0 {
            return Err(McpError::InvalidRequest(
                "write target parent is inaccessible".into(),
            ));
        }
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    fn entry_type(&self, name: &OsStr) -> Result<Option<EntryType>, McpError> {
        let name = CString::new(name.as_bytes())
            .map_err(|_| McpError::InvalidRequest("file name is invalid".into()))?;
        let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
        if unsafe {
            libc::fstatat(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
        {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
                return Ok(None);
            }
            return Err(McpError::InvalidRequest(
                "write target is inaccessible".into(),
            ));
        }
        let kind = stat.st_mode & libc::S_IFMT;
        Ok(Some(EntryType {
            symlink: kind == libc::S_IFLNK,
            directory: kind == libc::S_IFDIR,
            regular: kind == libc::S_IFREG,
        }))
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
                    identity: file_identity(&stat),
                })
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
fn file_identity(stat: &libc::stat) -> FileIdentity {
    FileIdentity {
        device: stat.st_dev,
        inode: stat.st_ino,
    }
}

#[cfg(target_os = "linux")]
fn path_identity(path: &Path) -> Result<FileIdentity, McpError> {
    let metadata = std::fs::metadata(path).map_err(|_| inaccessible_directory_error())?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub const DEFAULT_FILE_SEARCH_RESULTS: usize = 100;
pub const MAX_FILE_SEARCH_RESULTS: usize = 100;
pub const MAX_FILE_SEARCH_DIRECTORY_ENTRIES: usize = 4_096;
pub const MAX_FILE_SEARCH_VISITED_ENTRIES: usize = 65_536;
pub const MAX_FILE_SEARCH_RESULT_BYTES: usize = 256 * 1024;
const MAX_FILE_SEARCH_PATTERN_BYTES: usize = 4_096;
const MAX_FILE_SEARCH_CWD_BYTES: usize = 4_096;
const MAX_FILE_SEARCH_PATTERN_SEGMENTS: usize = 128;
const MAX_FILE_SEARCH_SEGMENT_BYTES: usize = 255;
const MAX_FILE_SEARCH_PATH_BYTES: usize = 3_500;
const MAX_FILE_SEARCH_DEPTH: usize = 64;

const FILE_SEARCH_SKIPPED_DIRECTORIES: [&str; 5] =
    [".git", "node_modules", "target", ".nuxt", ".output"];

#[derive(Debug, Serialize)]
pub struct FileSearchResult {
    pattern: String,
    matches: Vec<String>,
    count: usize,
    truncated: bool,
}

pub fn file_search(arguments: &Value, config: &ServerConfig) -> Result<FileSearchResult, McpError> {
    let pattern = arguments
        .get("pattern")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file search pattern is required".into()))?;
    validate_file_search_pattern(pattern)?;
    if let Some(cwd) = arguments.get("cwd").and_then(Value::as_str) {
        if cwd.len() > MAX_FILE_SEARCH_CWD_BYTES {
            return Err(McpError::InvalidRequest(
                "file search cwd exceeds maximum".into(),
            ));
        }
    }

    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_FILE_SEARCH_RESULTS)
        .clamp(1, MAX_FILE_SEARCH_RESULTS);
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let search_root = resolve_existing_path(&execution_root, cwd, ".", EntryKind::Directory)?;
    let search_root = SecureDirectory::open_relative(&execution_root, &search_root)?;
    let path_pattern = pattern.contains('/');
    let mut state = FileSearchState {
        pattern,
        path_pattern,
        max_results,
        matches: Vec::new(),
        visited_entries: 0,
        truncated: false,
    };
    visit_file_search(&search_root, "", 0, &mut state)?;

    let mut matches = state.matches;
    matches.sort();
    let mut truncated = state.truncated;
    while !matches.is_empty()
        && file_search_result_size(pattern, &matches, truncated)? > MAX_FILE_SEARCH_RESULT_BYTES
    {
        matches.pop();
        truncated = true;
    }
    let count = matches.len();
    Ok(FileSearchResult {
        pattern: pattern.to_owned(),
        matches,
        count,
        truncated,
    })
}

struct FileSearchState<'a> {
    pattern: &'a str,
    path_pattern: bool,
    max_results: usize,
    matches: Vec<String>,
    visited_entries: usize,
    truncated: bool,
}

fn visit_file_search(
    directory: &SecureDirectory,
    relative_directory: &str,
    depth: usize,
    state: &mut FileSearchState<'_>,
) -> Result<(), McpError> {
    if state.truncated {
        return Ok(());
    }
    let children = directory.read_entries(
        MAX_FILE_SEARCH_DIRECTORY_ENTRIES,
        "file search directory scan exceeds maximum",
    )?;

    for child in children {
        state.visited_entries = state.visited_entries.saturating_add(1);
        if state.visited_entries > MAX_FILE_SEARCH_VISITED_ENTRIES {
            return Err(McpError::InvalidRequest(
                "file search traversal exceeds maximum".into(),
            ));
        }

        let Some(name) = child.name.to_str() else {
            continue;
        };
        if name.len() > MAX_FILE_SEARCH_SEGMENT_BYTES {
            return Err(McpError::InvalidRequest(
                "file search path segment exceeds maximum".into(),
            ));
        }
        let child_depth = depth.saturating_add(1);
        if child_depth > MAX_FILE_SEARCH_DEPTH {
            return Err(McpError::InvalidRequest(
                "file search depth exceeds maximum".into(),
            ));
        }
        let relative = append_search_path(relative_directory, name)?;
        let file_type = child.file_type;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if FILE_SEARCH_SKIPPED_DIRECTORIES.contains(&name) {
                continue;
            }
            let child_directory = directory.open_child(&child)?;
            visit_file_search(&child_directory, &relative, child_depth, state)?;
            if state.truncated {
                return Ok(());
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let matched = if state.path_pattern {
            glob_match_path(state.pattern, &relative)
        } else {
            glob_match_segment(state.pattern, name)
        };
        if matched {
            if state.matches.len() < state.max_results {
                state.matches.push(relative);
            } else {
                state.truncated = true;
                return Ok(());
            }
        }
    }
    Ok(())
}

fn validate_file_search_pattern(pattern: &str) -> Result<(), McpError> {
    if pattern.is_empty()
        || pattern.len() > MAX_FILE_SEARCH_PATTERN_BYTES
        || pattern.starts_with('/')
        || pattern.split('/').count() > MAX_FILE_SEARCH_PATTERN_SEGMENTS
    {
        return Err(McpError::InvalidRequest(
            "file search pattern must be a relative glob".into(),
        ));
    }
    for segment in pattern.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.len() > MAX_FILE_SEARCH_SEGMENT_BYTES
        {
            return Err(McpError::InvalidRequest(
                "file search pattern must be a bounded relative glob".into(),
            ));
        }
    }
    Ok(())
}

fn append_search_path(parent: &str, name: &str) -> Result<String, McpError> {
    let additional = name.len() + usize::from(!parent.is_empty());
    if parent.len().saturating_add(additional) > MAX_FILE_SEARCH_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file search path exceeds maximum".into(),
        ));
    }
    if parent.is_empty() {
        Ok(name.to_owned())
    } else {
        Ok(format!("{parent}/{name}"))
    }
}

fn file_search_result_size(
    pattern: &str,
    matches: &[String],
    truncated: bool,
) -> Result<usize, McpError> {
    let result = FileSearchResult {
        pattern: pattern.to_owned(),
        matches: matches.to_vec(),
        count: matches.len(),
        truncated,
    };
    serde_json::to_vec(&result)
        .map(|value| value.len())
        .map_err(|_| McpError::Internal("failed to serialize file search result".into()))
}

fn glob_match_path(pattern: &str, path: &str) -> bool {
    let pattern_segments = pattern.split('/').collect::<Vec<_>>();
    let path_segments = path.split('/').collect::<Vec<_>>();
    let mut pattern_index = 0usize;
    let mut path_index = 0usize;
    let mut star_pattern_index = None;
    let mut star_path_index = 0usize;

    while path_index < path_segments.len() {
        if pattern_index < pattern_segments.len()
            && pattern_segments[pattern_index] != "**"
            && glob_match_segment(pattern_segments[pattern_index], path_segments[path_index])
        {
            pattern_index += 1;
            path_index += 1;
        } else if pattern_index < pattern_segments.len() && pattern_segments[pattern_index] == "**"
        {
            star_pattern_index = Some(pattern_index);
            star_path_index = path_index;
            pattern_index += 1;
        } else if let Some(star) = star_pattern_index {
            pattern_index = star + 1;
            star_path_index += 1;
            path_index = star_path_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern_segments.len() && pattern_segments[pattern_index] == "**" {
        pattern_index += 1;
    }
    pattern_index == pattern_segments.len()
}

fn glob_match_segment(pattern: &str, text: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let text = text.chars().collect::<Vec<_>>();
    let mut pattern_index = 0usize;
    let mut text_index = 0usize;
    let mut star_pattern_index = None;
    let mut star_text_index = 0usize;

    while text_index < text.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == '?' || pattern[pattern_index] == text[text_index])
        {
            pattern_index += 1;
            text_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == '*' {
            star_pattern_index = Some(pattern_index);
            star_text_index = text_index;
            pattern_index += 1;
        } else if let Some(star) = star_pattern_index {
            pattern_index = star + 1;
            star_text_index += 1;
            text_index = star_text_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == '*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

pub const DEFAULT_FILE_READ_LINES: usize = 200;
pub const MAX_FILE_READ_LINES: usize = 1_000;
pub const MAX_FILE_READ_BYTES: usize = 256 * 1024;
const MAX_FILE_READ_LINE_BYTES: usize = 64 * 1024;
const MAX_FILE_READ_PATH_BYTES: usize = 4_096;
const MAX_FILE_READ_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileReadResult {
    path: String,
    start_line: u64,
    end_line: Option<u64>,
    content: String,
    truncated: bool,
}

fn read_bounded_line_sync<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, McpError> {
    let mut line = Vec::new();
    loop {
        let buffer = reader
            .fill_buf()
            .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let take = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(buffer.len(), |index| index + 1);
        if line.len().saturating_add(take) > max_bytes {
            return Err(McpError::InvalidRequest("file line exceeds maximum".into()));
        }
        line.extend_from_slice(&buffer[..take]);
        reader.consume(take);
        if line.last() == Some(&b'\n') {
            return Ok(Some(line));
        }
    }
}

pub fn file_read(arguments: &Value, config: &ServerConfig) -> Result<FileReadResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file read path is required".into()))?;
    if path.is_empty() || path.len() > MAX_FILE_READ_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file read path exceeds allowed bounds".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_READ_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file read cwd exceeds maximum".into(),
        ));
    }
    let offset_line = arguments
        .get("offset_line")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if offset_line == 0 {
        return Err(McpError::InvalidRequest(
            "offset_line must be at least 1".into(),
        ));
    }
    let limit_lines = arguments
        .get("limit_lines")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_FILE_READ_LINES)
        .clamp(1, MAX_FILE_READ_LINES);
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let target = resolve_existing_path(&root, cwd, path, EntryKind::File)?;
    let file = std::fs::File::open(&target)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    let mut reader = BufReader::new(file);
    let mut current = 1u64;
    while current < offset_line {
        if read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)?.is_none() {
            return Ok(FileReadResult {
                path: path.to_owned(),
                start_line: offset_line,
                end_line: None,
                content: String::new(),
                truncated: false,
            });
        }
        current += 1;
    }
    let mut content = Vec::new();
    let mut lines_read = 0usize;
    let mut end_line = None;
    let mut truncated = false;
    while lines_read < limit_lines {
        let Some(line) = read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)? else {
            break;
        };
        if std::str::from_utf8(&line).is_err() {
            return Err(McpError::InvalidRequest(
                "file is not valid UTF-8 text".into(),
            ));
        }
        if content.len().saturating_add(line.len()) > MAX_FILE_READ_BYTES {
            truncated = true;
            break;
        }
        content.extend_from_slice(&line);
        end_line = Some(offset_line + lines_read as u64);
        lines_read += 1;
    }
    if !truncated && lines_read == limit_lines {
        truncated = read_bounded_line_sync(&mut reader, MAX_FILE_READ_LINE_BYTES)?.is_some();
    }
    let content = String::from_utf8(content)
        .map_err(|_| McpError::InvalidRequest("file is not valid UTF-8 text".into()))?;
    Ok(FileReadResult {
        path: path.to_owned(),
        start_line: offset_line,
        end_line,
        content,
        truncated,
    })
}

pub const MAX_FILE_EDIT_BYTES: usize = 1024 * 1024;
const MAX_FILE_EDIT_TEXT_BYTES: usize = 256 * 1024;
const MAX_FILE_EDIT_PATH_BYTES: usize = 4_096;
const MAX_FILE_EDIT_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileEditResult {
    path: String,
    replacements: usize,
    changed: bool,
}

pub fn file_edit(arguments: &Value, config: &ServerConfig) -> Result<FileEditResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit path is required".into()))?;
    let old_text = arguments
        .get("old_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit old_text is required".into()))?;
    let new_text = arguments
        .get("new_text")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file edit new_text is required".into()))?;
    let replace_all = arguments
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if path.is_empty() || path.len() > MAX_FILE_EDIT_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit path exceeds allowed bounds".into(),
        ));
    }
    if old_text.is_empty()
        || old_text.len() > MAX_FILE_EDIT_TEXT_BYTES
        || new_text.len() > MAX_FILE_EDIT_TEXT_BYTES
    {
        return Err(McpError::InvalidRequest(
            "file edit text exceeds allowed bounds".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_EDIT_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file edit cwd exceeds maximum".into(),
        ));
    }
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let target = resolve_write_target(&root, cwd, path, EntryKind::File)?;
    let parent = target
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let name = target
        .file_name()
        .ok_or_else(|| McpError::InvalidRequest("file edit target is invalid".into()))?;
    let directory = SecureDirectory::open_relative(&root, parent)?;
    let (mut file, identity, mode) = directory.open_regular_file(name)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_FILE_EDIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| McpError::InvalidRequest("file edit target is inaccessible".into()))?;
    if bytes.len() > MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit target exceeds maximum".into(),
        ));
    }
    let source = String::from_utf8(bytes)
        .map_err(|_| McpError::InvalidRequest("file edit target is not valid UTF-8 text".into()))?;
    let matches = source.match_indices(old_text).count();
    if matches == 0 {
        return Err(McpError::InvalidRequest(
            "file edit text was not found".into(),
        ));
    }
    if !replace_all && matches != 1 {
        return Err(McpError::InvalidRequest(
            "file edit text is ambiguous".into(),
        ));
    }
    let updated = if replace_all {
        source.replace(old_text, new_text)
    } else {
        source.replacen(old_text, new_text, 1)
    };
    if updated.len() > MAX_FILE_EDIT_BYTES {
        return Err(McpError::InvalidRequest(
            "file edit result exceeds maximum".into(),
        ));
    }
    let changed = updated != source;
    if changed {
        directory.atomic_replace_regular_file(name, identity, updated.as_bytes(), mode)?;
    } else {
        directory.verify_regular_entry(name, identity)?;
    }
    Ok(FileEditResult {
        path: path.to_owned(),
        replacements: if replace_all { matches } else { 1 },
        changed,
    })
}

pub const MAX_FILE_WRITE_BYTES: usize = 1024 * 1024;
const MAX_FILE_WRITE_PATH_BYTES: usize = 4_096;
const MAX_FILE_WRITE_CWD_BYTES: usize = 4_096;

#[derive(Debug, Serialize)]
pub struct FileWriteResult {
    path: String,
    created: bool,
    overwritten: bool,
    bytes: usize,
}

fn normalize_write_path(
    root: &Path,
    cwd: &Path,
    value: &str,
) -> Result<std::path::PathBuf, McpError> {
    use std::path::Component;
    let requested = if Path::new(value).is_absolute() {
        Path::new(value).to_path_buf()
    } else {
        cwd.join(value)
    };
    let mut normalized = std::path::PathBuf::new();
    for component in requested.components() {
        match component {
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(McpError::InvalidRequest(
                        "write target escapes execution root".into(),
                    ));
                }
            }
            Component::Normal(value) => normalized.push(value),
            Component::Prefix(_) => {
                return Err(McpError::InvalidRequest("write target is invalid".into()))
            }
        }
    }
    if !normalized.starts_with(root) || normalized == root {
        return Err(McpError::InvalidRequest(
            "write target escapes execution root".into(),
        ));
    }
    Ok(normalized)
}

fn resolve_write_parent_directory(
    root: &Path,
    cwd: Option<&str>,
    path: &str,
    create_parents: bool,
) -> Result<(SecureDirectory, std::ffi::OsString), McpError> {
    let cwd = resolve_existing_path(root, cwd, ".", EntryKind::Directory)?;
    let normalized = normalize_write_path(root, &cwd, path)?;
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| McpError::InvalidRequest("write target escapes execution root".into()))?;
    let name = relative
        .file_name()
        .ok_or_else(|| McpError::InvalidRequest("write target is invalid".into()))?
        .to_os_string();
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut directory = SecureDirectory::open_relative(root, root)?;
    for component in parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(McpError::InvalidRequest(
                "write target parent is invalid".into(),
            ));
        };
        directory = directory.open_or_create_child(component, create_parents)?;
    }
    Ok((directory, name))
}

pub fn file_write(arguments: &Value, config: &ServerConfig) -> Result<FileWriteResult, McpError> {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file write path is required".into()))?;
    let content = arguments
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("file write content is required".into()))?;
    if path.is_empty() || path.len() > MAX_FILE_WRITE_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "file write path exceeds allowed bounds".into(),
        ));
    }
    if content.len() > MAX_FILE_WRITE_BYTES {
        return Err(McpError::InvalidRequest(
            "file write content exceeds maximum".into(),
        ));
    }
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    if cwd.is_some_and(|value| value.len() > MAX_FILE_WRITE_CWD_BYTES) {
        return Err(McpError::InvalidRequest(
            "file write cwd exceeds maximum".into(),
        ));
    }
    let create_parents = arguments
        .get("create_parents")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let overwrite = arguments
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let (directory, name) = resolve_write_parent_directory(&root, cwd, path, create_parents)?;
    match directory.entry_type(&name)? {
        Some(entry) if entry.is_symlink() || entry.is_dir() || !entry.is_file() => Err(
            McpError::InvalidRequest("write target has an unsupported entry type".into()),
        ),
        Some(_) if !overwrite => Err(McpError::InvalidRequest(
            "file already exists; overwrite is required".into(),
        )),
        Some(_) => {
            let (_file, identity, mode) = directory.open_regular_file(&name)?;
            directory.atomic_replace_regular_file(&name, identity, content.as_bytes(), mode)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: false,
                overwritten: true,
                bytes: content.len(),
            })
        }
        None => {
            directory.atomic_create_regular_file(&name, content.as_bytes(), 0o644)?;
            Ok(FileWriteResult {
                path: path.to_owned(),
                created: true,
                overwritten: false,
                bytes: content.len(),
            })
        }
    }
}
