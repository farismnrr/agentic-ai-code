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

    let mut stack = vec![(search_root, String::new(), 0usize)];
    let mut visited_entries = 0usize;
    let mut match_count = 0usize;
    let mut smallest_matches = BinaryHeap::new();

    while let Some((directory, relative_directory, depth)) = stack.pop() {
        let children = directory.read_entries(
            MAX_FILE_SEARCH_DIRECTORY_ENTRIES,
            "file search directory scan exceeds maximum",
        )?;

        let mut next_directories = Vec::new();
        for child in children {
            visited_entries = visited_entries.saturating_add(1);
            if visited_entries > MAX_FILE_SEARCH_VISITED_ENTRIES {
                return Err(McpError::InvalidRequest(
                    "file search traversal exceeds maximum".into(),
                ));
            }

            // Results are UTF-8 JSON paths. An entry whose native name cannot
            // be represented as UTF-8 is deterministically omitted, including
            // any directory below it; it is never lossily renamed or followed.
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
            let relative = append_search_path(&relative_directory, name)?;
            let file_type = child.file_type;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if FILE_SEARCH_SKIPPED_DIRECTORIES.contains(&name) {
                    continue;
                }
                let child_directory = directory.open_child(&child.name)?;
                next_directories.push((child_directory, relative, child_depth));
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let matched = if path_pattern {
                glob_match_path(pattern, &relative)
            } else {
                glob_match_segment(pattern, name)
            };
            if matched {
                match_count = match_count.saturating_add(1);
                smallest_matches.push(relative);
                if smallest_matches.len() > max_results.saturating_add(1) {
                    smallest_matches.pop();
                }
            }
        }

        // LIFO stack: reverse the sorted directory list so the next directory
        // visited is lexically smallest. Final matches are sorted independently.
        for item in next_directories.into_iter().rev() {
            stack.push(item);
        }
    }

    let mut matches = smallest_matches.into_vec();
    matches.sort();
    let mut truncated = match_count > max_results;
    if matches.len() > max_results {
        matches.truncate(max_results);
    }
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
