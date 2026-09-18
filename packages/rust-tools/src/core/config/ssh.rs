use super::{RelayError, ServerConfig};

impl ServerConfig {
    /// Resolve the approved SSH credential root. The default is ~/.ssh, while
    /// an explicit root is allowed for a dedicated diagnostic identity. The
    /// credential root must remain outside every model-writable execution root,
    /// except for the execution root's own protected `.ssh` directory. In that
    /// case the sandbox still exposes only exact reviewed files read-only.
    pub fn resolved_ssh_root(&self) -> Result<std::path::PathBuf, RelayError> {
        let configured = match self.ssh_root.as_deref() {
            Some(path) => {
                let path = std::path::PathBuf::from(path);
                if !path.is_absolute() {
                    return Err(RelayError::InvalidConfig(
                        "ssh-root must be an absolute path".into(),
                    ));
                }
                path
            }
            None => dirs_home()
                .ok_or_else(|| {
                    RelayError::InvalidConfig(
                        "HOME is required when SSH diagnostics are enabled".into(),
                    )
                })?
                .join(".ssh"),
        };
        let root = std::fs::canonicalize(&configured)
            .map_err(|_| RelayError::InvalidConfig("SSH credential root is unavailable".into()))?;
        if !root.is_dir() {
            return Err(RelayError::InvalidConfig(
                "SSH credential root must be a directory".into(),
            ));
        }
        validate_ssh_owner_permissions(&root, true)?;
        if let Ok(execution_root) = self.resolved_execution_root() {
            let protected_ssh_root = execution_root.join(".ssh");
            if root.starts_with(&execution_root) && root != protected_ssh_root {
                return Err(RelayError::InvalidConfig(
                    "SSH credential root must be outside the execution root".into(),
                ));
            }
        }
        Ok(root)
    }

    /// Resolve the operator SSH config and prove it remains beneath the
    /// approved SSH credential root.
    pub fn resolved_ssh_config(&self) -> Result<std::path::PathBuf, RelayError> {
        let root = self.resolved_ssh_root()?;
        let configured = self
            .ssh_config
            .as_deref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("config"));
        let canonical = std::fs::canonicalize(&configured).map_err(|_| {
            RelayError::InvalidConfig("configured SSH config is unavailable".into())
        })?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(RelayError::InvalidConfig(
                "configured SSH config must be a regular file beneath the SSH credential root"
                    .into(),
            ));
        }
        validate_ssh_owner_permissions(&canonical, false)?;
        Ok(canonical)
    }

    pub fn resolved_ssh_redis_password_file(
        &self,
    ) -> Result<Option<std::path::PathBuf>, RelayError> {
        let Some(configured) = self.ssh_readonly_redis_password_file.as_deref() else {
            return Ok(None);
        };
        let root = self.resolved_ssh_root()?;
        let configured = std::path::PathBuf::from(configured);
        if !configured.is_absolute() {
            return Err(RelayError::InvalidConfig(
                "SSH Redis password file must be an absolute path".into(),
            ));
        }
        let canonical = std::fs::canonicalize(&configured).map_err(|_| {
            RelayError::InvalidConfig("SSH Redis password file is unavailable".into())
        })?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(RelayError::InvalidConfig(
                "SSH Redis password file must be a regular file beneath the SSH credential root"
                    .into(),
            ));
        }
        validate_ssh_secret_file(&canonical)?;
        Ok(Some(canonical))
    }
}

fn validate_ssh_secret_file(path: &std::path::Path) -> Result<(), RelayError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| RelayError::InvalidConfig("SSH secret metadata is unavailable".into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RelayError::InvalidConfig(
            "SSH secret path must be a regular non-symlink file".into(),
        ));
    }
    if metadata.len() == 0 || metadata.len() > 4096 {
        return Err(RelayError::InvalidConfig(
            "SSH secret file must contain between 1 and 4096 bytes".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let effective_uid = unsafe { libc::geteuid() };
        if metadata.uid() != effective_uid {
            return Err(RelayError::InvalidConfig(
                "SSH secret file must be owned by the relay operator".into(),
            ));
        }
        if metadata.mode() & 0o077 != 0 {
            return Err(RelayError::InvalidConfig(
                "SSH secret file must not be accessible by group or world".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn valid_diagnostic_principal(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.'))
}

fn validate_ssh_owner_permissions(
    path: &std::path::Path,
    directory: bool,
) -> Result<(), RelayError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| RelayError::InvalidConfig("SSH credential metadata is unavailable".into()))?;
    if metadata.file_type().is_symlink() {
        return Err(RelayError::InvalidConfig(
            "SSH credential paths must not be symbolic links".into(),
        ));
    }
    if directory != metadata.is_dir() {
        return Err(RelayError::InvalidConfig(
            "SSH credential path has the wrong filesystem type".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let effective_uid = unsafe { libc::geteuid() };
        if metadata.uid() != effective_uid {
            return Err(RelayError::InvalidConfig(
                "SSH credential paths must be owned by the relay operator".into(),
            ));
        }
        if metadata.mode() & 0o022 != 0 {
            return Err(RelayError::InvalidConfig(
                "SSH credential paths must not be group/world writable".into(),
            ));
        }
    }
    Ok(())
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
}
