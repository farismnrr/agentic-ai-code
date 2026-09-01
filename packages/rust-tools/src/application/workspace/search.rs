//! Bounded filename search over the shared secure traversal foundation.

use super::protected::{is_protected_discovered_path, reject_protected_path};
use super::secure::SecureDirectory;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::EntryKind;
use serde::Serialize;
use serde_json::Value;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
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
    let _ = config.ensure_workspaces_initialized();
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let search_root_path = crate::core::workspace_path::resolve_existing_path_in_allowlist(
        &guard,
        cwd,
        ".",
        EntryKind::Directory,
    )?;
    let execution_root = guard.containing_root(&search_root_path).ok_or_else(|| {
        McpError::InvalidRequest("directory is outside authorized workspace roots".into())
    })?;
    reject_protected_path(execution_root, &search_root_path)?;
    let search_root = SecureDirectory::open_relative(execution_root, &search_root_path)?;
    let path_pattern = pattern.contains('/');
    let mut state = FileSearchState {
        pattern,
        path_pattern,
        max_results: crate::application::continuation::MAX_TOTAL_ENTRIES,
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
    let scope = search_root.root().to_string_lossy().into_owned();
    let (matches, continuation) = crate::application::continuation::paginate(
        arguments,
        matches,
        max_results.min(crate::application::continuation::MAX_TOTAL_ENTRIES),
        config,
        "file_search",
        &scope,
        None,
    )?;
    let count = matches.len();
    Ok(FileSearchResult {
        pattern: pattern.to_owned(),
        matches,
        count,
        truncated: continuation.is_some() || truncated,
        continuation,
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
        let child_path = directory.path_for_child(&child.name);
        if is_protected_discovered_path(directory.root(), &child_path) {
            continue;
        }
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
        continuation: None,
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
