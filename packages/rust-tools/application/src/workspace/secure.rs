//! Linux no-follow directory traversal and atomic mutation primitives.

use relay_core::error::McpError;
use std::ffi::{CStr, CString, OsStr};
use std::io::Write;
use std::path::Path;

fn inaccessible_directory_error() -> McpError {
    McpError::InvalidRequest("directory is inaccessible".into())
}

#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct SecureDirectory {
    fd: OwnedFd,
    root: std::path::PathBuf,
    path: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct DirectoryEntry {
    pub(super) name: std::ffi::OsString,
    pub(super) file_type: EntryType,
    identity: FileIdentity,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug)]
pub(super) struct EntryType {
    symlink: bool,
    directory: bool,
    regular: bool,
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
impl SecureDirectory {
    pub(super) fn open_relative(root: &Path, target: &Path) -> Result<Self, McpError> {
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
            root: root.to_path_buf(),
            path: root.to_path_buf(),
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

    pub(super) fn open_child(&self, child: &DirectoryEntry) -> Result<Self, McpError> {
        self.open_child_name(&child.name, child.identity)
    }

    pub(super) fn open_child_name(
        &self,
        name: &OsStr,
        expected: FileIdentity,
    ) -> Result<Self, McpError> {
        let child_path = self.path.join(name);
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
            root: self.root.clone(),
            path: child_path,
        };
        directory.verify_identity(expected)?;
        Ok(directory)
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn path_for_child(&self, name: &OsStr) -> std::path::PathBuf {
        self.path.join(name)
    }

    pub(super) fn verify_identity(&self, expected: FileIdentity) -> Result<(), McpError> {
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

    pub(super) fn open_regular_file(
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

    pub(super) fn verify_regular_entry(
        &self,
        name: &OsStr,
        expected: FileIdentity,
    ) -> Result<(), McpError> {
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

    pub(super) fn create_temp_file(&self, content: &[u8], mode: u32) -> Result<CString, McpError> {
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
        let target = CString::new(name.as_bytes()).map_err(|_| {
            (
                McpError::InvalidRequest("file name is invalid".into()),
                false,
            )
        })?;
        let temp = self
            .create_temp_file(content, mode)
            .map_err(|error| (error, false))?;
        if let Err(error) = self.verify_regular_entry(name, expected) {
            unsafe { libc::unlinkat(self.fd.as_raw_fd(), temp.as_ptr(), 0) };
            return Err((error, false));
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
            return Err((
                McpError::InvalidRequest("file replacement could not be committed".into()),
                false,
            ));
        }
        self.sync_directory().map_err(|error| (error, true))
    }

    pub(super) fn atomic_create_regular_file(
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

    pub(super) fn sync_directory(&self) -> Result<(), McpError> {
        if unsafe { libc::fsync(self.fd.as_raw_fd()) } < 0 {
            return Err(McpError::InvalidRequest(
                "directory durability sync failed".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn open_or_create_child(
        &self,
        name: &OsStr,
        create: bool,
    ) -> Result<Self, McpError> {
        let child_path = self.path.join(name);
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
            root: self.root.clone(),
            path: child_path,
        })
    }

    pub(super) fn entry_type(&self, name: &OsStr) -> Result<Option<EntryType>, McpError> {
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

    pub(super) fn read_entries(
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
