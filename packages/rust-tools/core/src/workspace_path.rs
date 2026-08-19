//! Contained filesystem path resolution for native workspace operations.
//!
//! Path contract:
//! - `cwd = None` selects the canonical primary workspace root; relative `cwd` values
//!   resolve from that root and must name an existing contained directory.
//! - Relative operation paths resolve from the selected `cwd`; absolute paths
//!   are accepted only when their canonical target (or canonical write parent)
//!   remains beneath an authorized workspace root.
//! - `.` and `..` are permitted when canonical resolution remains contained.
//! - Existing-path resolution follows symlinks, then rejects any target outside
//!   authorized workspace roots. Contained symlinks are valid for read-style access.
//! - Write-target resolution permits a missing final component only when its
//!   existing parent canonicalizes beneath an authorized root. Existing final symlinks are
//!   rejected, including contained symlinks, so callers never mutate through a
//!   final symlink accidentally.
use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
/// Maximum number of dynamically authorized workspaces.
pub const MAX_AUTHORIZED_WORKSPACES: usize = 32;
/// A single workspace entry describing a primary or dynamically authorized root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceEntry {
    pub path: String,
    pub canonical_path: PathBuf,
    pub is_primary: bool,
}
/// Thread-safe registry of authorized workspace roots.
#[derive(Debug, Clone)]
pub struct WorkspaceAllowlist {
    boundary_root: PathBuf,
    primary_root: PathBuf,
    authorized_roots: Vec<PathBuf>,
}
impl Default for WorkspaceAllowlist {
    fn default() -> Self {
        Self {
            boundary_root: PathBuf::from("/nonexistent"),
            primary_root: PathBuf::from("/nonexistent"),
            authorized_roots: Vec::new(),
        }
    }
}
impl WorkspaceAllowlist {
    /// Create a new workspace allowlist with an explicit primary root.
    pub fn new(primary_root: PathBuf) -> Result<Self, McpError> {
        let canonical = canonical_root(&primary_root)?;
        validate_workspace_root_path(&canonical)?;
        Ok(Self {
            boundary_root: canonical.clone(),
            primary_root: canonical,
            authorized_roots: Vec::new(),
        })
    }
    pub fn primary_root(&self) -> &Path {
        &self.primary_root
    }
    pub fn set_roots(
        &mut self,
        boundary_root: PathBuf,
        primary_root: PathBuf,
    ) -> Result<(), McpError> {
        let boundary = canonical_root(&boundary_root)?;
        validate_workspace_root_path(&boundary)?;
        let primary = canonical_root(&primary_root)?;
        validate_workspace_root_path(&primary)?;
        if !primary.starts_with(&boundary) {
            return Err(McpError::InvalidRequest(
                "primary workspace root is outside the execution boundary".into(),
            ));
        }
        self.boundary_root = boundary;
        self.primary_root = primary;
        self.authorized_roots.clear();
        Ok(())
    }
    pub fn authorized_roots(&self) -> &[PathBuf] {
        &self.authorized_roots
    }
    pub fn all_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::with_capacity(self.authorized_roots.len() + 1);
        if self.primary_root.exists() {
            roots.push(self.primary_root.clone());
        }
        roots.extend(self.authorized_roots.clone());
        roots
    }
    pub fn is_contained(&self, path: &Path) -> bool {
        if self.primary_root.exists() && path.starts_with(&self.primary_root) {
            return true;
        }
        self.authorized_roots
            .iter()
            .any(|root| path.starts_with(root))
    }

    pub fn containing_root<'a>(&'a self, path: &Path) -> Option<&'a Path> {
        if self.primary_root.exists() && path.starts_with(&self.primary_root) {
            return Some(&self.primary_root);
        }
        self.authorized_roots
            .iter()
            .find(|root| path.starts_with(root))
            .map(PathBuf::as_path)
    }

    pub fn add(&mut self, path: &Path) -> Result<PathBuf, McpError> {
        let canonical = fs::canonicalize(path).map_err(|_| inaccessible_path_error())?;
        if !canonical.is_dir() {
            return Err(McpError::InvalidRequest(
                "workspace root must be an existing directory".into(),
            ));
        }
        validate_workspace_root_path(&canonical)?;
        if !canonical.starts_with(&self.boundary_root) {
            return Err(McpError::InvalidRequest(
                "workspace root is outside the configured execution boundary".into(),
            ));
        }

        if self.is_contained(&canonical) {
            return Ok(canonical);
        }

        if self.authorized_roots.len() >= MAX_AUTHORIZED_WORKSPACES {
            return Err(McpError::InvalidRequest(
                "authorized workspaces capacity exhausted (maximum 32)".into(),
            ));
        }

        self.authorized_roots.push(canonical.clone());
        Ok(canonical)
    }

    pub fn remove(&mut self, path: &Path) -> Result<bool, McpError> {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if canonical == self.primary_root {
            return Err(McpError::InvalidRequest(
                "cannot remove primary workspace root".into(),
            ));
        }
        let len_before = self.authorized_roots.len();
        self.authorized_roots
            .retain(|r| *r != canonical && *r != path);
        Ok(self.authorized_roots.len() < len_before)
    }

    pub fn list(&self) -> Vec<WorkspaceEntry> {
        let mut entries = Vec::new();
        if self.primary_root.exists() {
            entries.push(WorkspaceEntry {
                path: self.primary_root.to_string_lossy().into_owned(),
                canonical_path: self.primary_root.clone(),
                is_primary: true,
            });
        }
        for root in &self.authorized_roots {
            entries.push(WorkspaceEntry {
                path: root.to_string_lossy().into_owned(),
                canonical_path: root.clone(),
                is_primary: false,
            });
        }
        entries
    }
}

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

/// Resolve an existing directory beneath an authorized workspace root.
pub fn resolve_contained_cwd(
    execution_root: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    resolve_existing_path(execution_root, cwd, ".", EntryKind::Directory)
}

/// Resolve an existing path beneath an authorized workspace root and enforce its type.
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

/// Resolve an existing native filesystem path beneath an authorized workspace root.
pub fn resolve_existing_native_path(
    execution_root: &Path,
    path: &Path,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let root = canonical_root(execution_root)?;
    resolve_existing_from_root(&root, path, expected)
}

/// Resolve a file target whose final component may not exist.
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

/// Multi-root resolution variants using a `WorkspaceAllowlist`.
pub fn resolve_contained_cwd_in_allowlist(
    allowlist: &WorkspaceAllowlist,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    match cwd {
        Some(value) => {
            let path = Path::new(value);
            let target = if path.is_absolute() {
                path.to_path_buf()
            } else {
                allowlist.primary_root().join(path)
            };
            let canonical = fs::canonicalize(&target).map_err(|_| missing_cwd_error())?;
            if !allowlist.is_contained(&canonical) {
                return Err(McpError::InvalidRequest(
                    "cwd is outside authorized workspace roots".into(),
                ));
            }
            if !canonical.is_dir() {
                return Err(McpError::InvalidRequest("cwd is not a directory".into()));
            }
            Ok(canonical)
        }
        None => {
            let primary = allowlist.primary_root();
            if !primary.is_dir() {
                return Err(McpError::InvalidRequest(
                    "primary workspace root is inaccessible".into(),
                ));
            }
            Ok(primary.to_path_buf())
        }
    }
}

pub fn resolve_existing_path_in_allowlist(
    allowlist: &WorkspaceAllowlist,
    cwd: Option<&str>,
    path: &str,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let base = resolve_contained_cwd_in_allowlist(allowlist, cwd)?;
    let requested = resolve_requested_path(&base, path)?;
    let canonical = fs::canonicalize(&requested).map_err(|_| missing_path_error())?;
    if !allowlist.is_contained(&canonical) {
        return Err(McpError::InvalidRequest(
            "path is outside authorized workspace roots".into(),
        ));
    }
    ensure_kind(&canonical, expected)?;
    Ok(canonical)
}

pub fn resolve_write_target_in_allowlist(
    allowlist: &WorkspaceAllowlist,
    cwd: Option<&str>,
    path: &str,
    expected: EntryKind,
) -> Result<PathBuf, McpError> {
    let base = resolve_contained_cwd_in_allowlist(allowlist, cwd)?;
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
            if !allowlist.is_contained(&canonical) {
                return Err(McpError::InvalidRequest(
                    "write target is outside authorized workspace roots".into(),
                ));
            }
            ensure_kind(&canonical, expected)?;
            Ok(canonical)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = requested.parent().ok_or_else(invalid_write_parent_error)?;
            let canonical_parent =
                fs::canonicalize(parent).map_err(|_| invalid_write_parent_error())?;
            if !allowlist.is_contained(&canonical_parent) {
                return Err(McpError::InvalidRequest(
                    "write target parent is outside authorized workspace roots".into(),
                ));
            }
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

pub fn validate_workspace_root_path(canonical: &Path) -> Result<(), McpError> {
    let forbidden_roots: &[&Path] = &[
        Path::new("/"),
        Path::new("/tmp"),
        Path::new("/etc"),
        Path::new("/proc"),
        Path::new("/sys"),
        Path::new("/dev"),
        Path::new("/root"),
        Path::new("/var"),
        Path::new("/usr"),
        Path::new("/bin"),
        Path::new("/sbin"),
        Path::new("/lib"),
        Path::new("/lib64"),
        Path::new("/boot"),
        Path::new("/run"),
        Path::new("/opt"),
        Path::new("/srv"),
    ];
    for bad in forbidden_roots {
        if canonical == *bad {
            return Err(McpError::InvalidRequest(format!(
                "workspace root '{}' is a forbidden system path and cannot be used as a filesystem boundary",
                canonical.display()
            )));
        }
    }
    if crate::protected_paths::is_protected_relative(canonical) {
        return Err(McpError::InvalidRequest(
            "workspace root targets a protected credential path".into(),
        ));
    }
    let depth = canonical.components().count();
    if depth < 3 {
        return Err(McpError::InvalidRequest(format!(
            "workspace root '{}' is too shallow (depth {}). Use a canonical non-root owner home or project directory",
            canonical.display(),
            depth
        )));
    }
    Ok(())
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
