//! Contained filesystem path resolution for native workspace operations.
//!
//! Path contract:
//! - `cwd = None` selects the canonical execution root; relative `cwd` values
//!   resolve from that root and must name an existing contained directory.
//! - Relative operation paths resolve from the selected `cwd`; absolute paths
//!   are accepted only when their canonical target (or canonical write parent)
//!   remains beneath the execution root.
//! - `.` and `..` are permitted when canonical resolution remains contained.
//! - Existing-path resolution follows symlinks, then rejects any target outside
//!   the root. Contained symlinks are therefore valid for read-style access.
//! - Write-target resolution permits a missing final component only when its
//!   existing parent canonicalizes beneath the root. Existing final symlinks are
//!   rejected, including contained symlinks, so callers never mutate through a
//!   final symlink accidentally.
//!
//! These helpers validate containment for the filesystem state observed during
//! resolution. They intentionally do not claim race-free mutation: edit/write
//! callers must pair this foundation with operation-time no-follow/revalidation
//! and atomic mutation semantics appropriate to their phase.

use crate::error::McpError;
use std::fs;
use std::path::{Path, PathBuf};

/// The entry type an operation expects after resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Accept any existing filesystem entry.
    Any,
    /// Accept regular files only.
    File,
    /// Accept directories only.
    Directory,
}

/// Resolve an existing directory beneath `execution_root`.
pub fn resolve_contained_cwd(
    execution_root: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    resolve_existing_path(execution_root, cwd, ".", EntryKind::Directory)
}

/// Resolve an existing path beneath `execution_root` and enforce its type.
pub fn resolve_existing_path(
    execution_root: &Path,
    cwd: Option<&str>,
    path: &str,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let root = canonical_root(execution_root)?;
    let base = resolve_cwd_from_root(&root, cwd)?;
    let requested = resolve_requested_path(&base, path)?;
    resolve_existing_from_root(&root, &requested, expected)
}

/// Resolve an existing native filesystem path beneath `execution_root`.
///
/// This variant is for paths discovered from the filesystem itself, whose
/// components are not required to be valid UTF-8. User-supplied MCP paths
/// still enter through [`resolve_existing_path`].
pub fn resolve_existing_native_path(
    execution_root: &Path,
    path: &Path,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let root = canonical_root(execution_root)?;
    resolve_existing_from_root(&root, path, expected)
}

/// Resolve a file target whose final component may not exist.
///
/// Every parent component must already exist and resolve beneath the root.
/// An existing final target is canonicalized and type-checked; a missing final
/// target is returned with its validated, canonical parent. The caller must
/// still use an appropriately safe mutation strategy for the final operation.
pub fn resolve_write_target(
    execution_root: &Path,
    cwd: Option<&str>,
    path: &str,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let root = canonical_root(execution_root)?;
    let base = resolve_cwd_from_root(&root, cwd)?;
    let requested = resolve_requested_path(&base, path)?;

    if requested.as_os_str().is_empty()
        || requested.file_name().is_none()
        || requested.file_name() == Some(std::ffi::OsStr::new("."))
        || requested.file_name() == Some(std::ffi::OsStr::new(".."))
    {
        return Err(McpError::InvalidRequest("write target is invalid".into()));
    }

    match fs::symlink_metadata(&requested) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(McpError::InvalidRequest(
                    "write target has an unsupported entry type".into(),
                ));
            }
            let canonical = fs::canonicalize(&requested).map_err(|_| inaccessible_path_error())?;
            ensure_contained(&root, &canonical)?;
            ensure_kind(&canonical, expected)?;
            Ok(canonical)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = requested.parent().ok_or_else(invalid_write_parent_error)?;
            let canonical_parent =
                fs::canonicalize(parent).map_err(|_| invalid_write_parent_error())?;
            ensure_contained(&root, &canonical_parent)?;
            if !canonical_parent.is_dir() {
                return Err(invalid_write_parent_error());
            }
            Ok(canonical_parent.join(
                requested
                    .file_name()
                    .ok_or_else(|| McpError::InvalidRequest("write target is invalid".into()))?,
            ))
        }
        Err(_) => Err(inaccessible_path_error()),
    }
}

fn resolve_existing_from_root(
    root: &Path,
    requested: &Path,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let canonical = fs::canonicalize(requested).map_err(|_| missing_path_error())?;
    ensure_contained(root, &canonical)?;
    ensure_kind(&canonical, expected)?;
    Ok(canonical)
}

fn canonical_root(execution_root: &Path) -> Result<PathBuf, McpError> {
    let root = fs::canonicalize(execution_root)
        .map_err(|_| McpError::InvalidRequest("execution root is inaccessible".into()))?;
    if !root.is_dir() {
        return Err(McpError::InvalidRequest(
            "execution root is not a directory".into(),
        ));
    }
    Ok(root)
}

fn resolve_cwd_from_root(root: &Path, cwd: Option<&str>) -> Result<PathBuf, McpError> {
    let requested = match cwd {
        Some(value) => resolve_requested_path(root, value)?,
        None => root.to_path_buf(),
    };
    let canonical = fs::canonicalize(requested).map_err(|_| missing_cwd_error())?;
    ensure_contained(root, &canonical)?;
    if !canonical.is_dir() {
        return Err(McpError::InvalidRequest("cwd is not a directory".into()));
    }
    Ok(canonical)
}

fn resolve_requested_path(base: &Path, value: &str) -> Result<PathBuf, McpError> {
    let path = Path::new(value);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(base.join(path))
    }
}

fn ensure_contained(root: &Path, path: &Path) -> Result<(), McpError> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "path is outside the execution root".into(),
        ))
    }
}

fn ensure_kind(path: &Path, expected: EntryKind) -> Result<(), McpError> {
    let metadata = fs::metadata(path).map_err(|_| inaccessible_path_error())?;
    let valid = match expected {
        EntryKind::Any => true,
        EntryKind::File => metadata.is_file(),
        EntryKind::Directory => metadata.is_dir(),
    };
    if valid {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "path has an unsupported entry type".into(),
        ))
    }
}

fn missing_cwd_error() -> McpError {
    McpError::InvalidRequest("cwd path does not exist or is inaccessible".into())
}

fn missing_path_error() -> McpError {
    McpError::InvalidRequest("path does not exist or is inaccessible".into())
}

fn inaccessible_path_error() -> McpError {
    McpError::InvalidRequest("path is inaccessible".into())
}

fn invalid_write_parent_error() -> McpError {
    McpError::InvalidRequest("write target parent does not exist or is inaccessible".into())
}
