//! Portable workspace traversal for non-Linux CLI targets.
//!
//! Linux uses descriptor-relative no-follow syscalls in `secure.rs`. This
//! module keeps macOS and Windows builds usable with standard-library APIs,
//! rejecting symlinked components and re-checking metadata around operations.
//! It does not claim Linux's descriptor-level race resistance.

use crate::core::error::McpError;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};

fn inaccessible_directory_error() -> McpError {
    McpError::InvalidRequest("directory is inaccessible".into())
}

#[derive(Debug)]
pub(super) struct SecureDirectory {
    root: PathBuf,
    path: PathBuf,
}

#[derive(Debug)]
pub(super) struct DirectoryEntry {
    pub(super) name: OsString,
    pub(super) file_type: EntryType,
    identity: FileIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    len: u64,
    modified_nanos: u128,
    directory: bool,
    regular: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct EntryType {
    symlink: bool,
    directory: bool,
    regular: bool,
}

impl EntryType {
    pub(super) fn is_symlink(self) -> bool {
        self.symlink
    }

    pub(super) fn is_dir(self) -> bool {
        self.directory
    }

    pub(super) fn is_file(self) -> bool {
        self.regular
    }
}

impl SecureDirectory {
    pub(super) fn open_relative(root: &Path, target: &Path) -> Result<Self, McpError> {
        let relative = target
            .strip_prefix(root)
            .map_err(|_| McpError::InvalidRequest("path is outside execution root".into()))?;
        let root_identity = directory_identity(root)?;
        let directory = Self {
            root: root.to_path_buf(),
            path: root.to_path_buf(),
        };
        directory.verify_identity(root_identity)?;

        let mut current = directory;
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                return Err(McpError::InvalidRequest(
                    "path is not a relative directory".into(),
                ));
            };
            let child = current.path.join(component);
            let expected = directory_identity(&child)?;
            current = current.open_child_name(component, expected)?;
        }
        current.verify_identity(directory_identity(target)?)?;
        Ok(current)
    }

    pub(super) fn open_child(&self, child: &DirectoryEntry) -> Result<Self, McpError> {
        self.open_child_name(&child.name, child.identity)
    }

    pub(super) fn open_child_name(
        &self,
        name: &OsStr,
        expected: FileIdentity,
    ) -> Result<Self, McpError> {
        let child_path = self.path.join(name);
        let actual = directory_identity(&child_path)?;
        if actual != expected {
            return Err(McpError::InvalidRequest(
                "directory changed during traversal".into(),
            ));
        }
        Ok(Self {
            root: self.root.clone(),
            path: child_path,
        })
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn path_for_child(&self, name: &OsStr) -> PathBuf {
        self.path.join(name)
    }

    pub(super) fn verify_identity(&self, expected: FileIdentity) -> Result<(), McpError> {
        if directory_identity(&self.path)? != expected {
            return Err(McpError::InvalidRequest(
                "directory changed during traversal".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn open_regular_file(
        &self,
        name: &OsStr,
    ) -> Result<(std::fs::File, FileIdentity, u32), McpError> {
        let path = self.path.join(name);
        let metadata = regular_metadata(&path)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
        let current = regular_metadata(&path)?;
        if metadata_identity(&current) != metadata_identity(&metadata) {
            return Err(McpError::InvalidRequest("file changed during edit".into()));
        }
        Ok((file, metadata_identity(&metadata), file_mode(&metadata)))
    }

    pub(super) fn verify_regular_entry(
        &self,
        name: &OsStr,
        expected: FileIdentity,
    ) -> Result<(), McpError> {
        let metadata = regular_metadata(&self.path.join(name))?;
        if metadata_identity(&metadata) != expected {
            return Err(McpError::InvalidRequest("file changed during edit".into()));
        }
        Ok(())
    }

    pub(super) fn create_temp_file(&self, content: &[u8], mode: u32) -> Result<OsString, McpError> {
        let temp_name = OsString::from(format!(".relay-write-{}.tmp", uuid::Uuid::new_v4()));
        let temp_path = self.path.join(&temp_name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| McpError::InvalidRequest("temporary file could not be created".into()))?;
        let write_result = (|| -> std::io::Result<()> {
            file.write_all(content)?;
            file.flush()?;
            set_file_mode(&temp_path, mode)?;
            file.sync_all()?;
            Ok(())
        })();
        drop(file);
        if write_result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
            return Err(McpError::InvalidRequest(
                "file content could not be written".into(),
            ));
        }
        Ok(temp_name)
    }

    pub(super) fn atomic_replace_regular_file(
        &self,
        name: &OsStr,
        expected: FileIdentity,
        content: &[u8],
        mode: u32,
    ) -> Result<(), McpError> {
        self.atomic_replace_regular_file_state(name, expected, content, mode)
            .map_err(|(error, _committed)| error)
    }

    pub(super) fn atomic_replace_regular_file_state(
        &self,
        name: &OsStr,
        expected: FileIdentity,
        content: &[u8],
        mode: u32,
    ) -> Result<(), (McpError, bool)> {
        self.verify_regular_entry(name, expected)
            .map_err(|error| (error, false))?;
        let temp_name = self
            .create_temp_file(content, mode)
            .map_err(|error| (error, false))?;
        if let Err(error) = self.verify_regular_entry(name, expected) {
            let _ = std::fs::remove_file(self.path.join(&temp_name));
            return Err((error, false));
        }
        std::fs::rename(self.path.join(&temp_name), self.path.join(name)).map_err(|_| {
            (
                McpError::InvalidRequest("file replacement could not be committed".into()),
                false,
            )
        })?;
        self.sync_directory().map_err(|error| (error, true))
    }

    pub(super) fn atomic_create_regular_file(
        &self,
        name: &OsStr,
        content: &[u8],
        mode: u32,
    ) -> Result<(), McpError> {
        let target = self.path.join(name);
        if std::fs::symlink_metadata(&target).is_ok() {
            return Err(McpError::InvalidRequest(
                "file already exists or could not be created".into(),
            ));
        }
        let temp_name = self.create_temp_file(content, mode)?;
        let temp = self.path.join(&temp_name);
        if std::fs::hard_link(&temp, &target).is_err() {
            let _ = std::fs::remove_file(&temp);
            return Err(McpError::InvalidRequest(
                "file already exists or could not be created".into(),
            ));
        }
        let _ = std::fs::remove_file(temp);
        self.sync_directory()
    }

    pub(super) fn sync_directory(&self) -> Result<(), McpError> {
        // std::fs::File::sync_all above durably writes the file. Portable
        // directory handles are not exposed by the standard library, so this
        // is intentionally a no-op outside Linux rather than a false claim of
        // directory-level fsync semantics.
        Ok(())
    }

    pub(super) fn open_or_create_child(
        &self,
        name: &OsStr,
        create: bool,
    ) -> Result<Self, McpError> {
        let child_path = self.path.join(name);
        match std::fs::symlink_metadata(&child_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(McpError::InvalidRequest(
                "write target parent is inaccessible".into(),
            )),
            Ok(metadata) if metadata.is_dir() => Ok(Self {
                root: self.root.clone(),
                path: child_path,
            }),
            Ok(_) => Err(McpError::InvalidRequest(
                "write target parent is inaccessible".into(),
            )),
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&child_path).map_err(|_| {
                    McpError::InvalidRequest("parent directory could not be created".into())
                })?;
                Ok(Self {
                    root: self.root.clone(),
                    path: child_path,
                })
            }
            Err(_) => Err(McpError::InvalidRequest(
                "write target parent is inaccessible".into(),
            )),
        }
    }

    pub(super) fn entry_type(&self, name: &OsStr) -> Result<Option<EntryType>, McpError> {
        let path = self.path.join(name);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(McpError::InvalidRequest(
                    "write target is inaccessible".into(),
                ))
            }
        };
        Ok(Some(entry_type(&metadata)))
    }

    pub(super) fn read_entries(
        &self,
        limit: usize,
        limit_message: &str,
    ) -> Result<Vec<DirectoryEntry>, McpError> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&self.path).map_err(|_| inaccessible_directory_error())? {
            let entry = entry.map_err(|_| inaccessible_directory_error())?;
            if entries.len() >= limit {
                return Err(McpError::InvalidRequest(limit_message.into()));
            }
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|_| inaccessible_directory_error())?;
            entries.push(DirectoryEntry {
                name: entry.file_name(),
                file_type: entry_type(&metadata),
                identity: metadata_identity(&metadata),
            });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }
}

fn directory_identity(path: &Path) -> Result<FileIdentity, McpError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| inaccessible_directory_error())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(inaccessible_directory_error());
    }
    Ok(metadata_identity(&metadata))
}

fn regular_metadata(path: &Path) -> Result<std::fs::Metadata, McpError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| McpError::InvalidRequest("file is inaccessible".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(McpError::InvalidRequest(
            "file has an unsupported entry type".into(),
        ));
    }
    Ok(metadata)
}

fn metadata_identity(metadata: &std::fs::Metadata) -> FileIdentity {
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or(u128::MAX);
    FileIdentity {
        len: metadata.len(),
        modified_nanos,
        directory: metadata.is_dir(),
        regular: metadata.is_file(),
    }
}

fn entry_type(metadata: &std::fs::Metadata) -> EntryType {
    let file_type = metadata.file_type();
    EntryType {
        symlink: file_type.is_symlink(),
        directory: metadata.is_dir(),
        regular: metadata.is_file(),
    }
}

fn file_mode(metadata: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return metadata.permissions().mode() & 0o7777;
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o644
    }
}

fn set_file_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}
