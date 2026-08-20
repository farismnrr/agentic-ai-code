use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use ring::digest::{Context, SHA256};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

// Metadata-only snapshots avoid reading workspace contents or credentials. On
// Linux, unprivileged writes cannot restore ctime, so unchanged fingerprints
// are strong evidence that a failed provider left the selected workspace as-is.
const MAX_SNAPSHOT_ENTRIES: usize = 200_000;

pub(super) struct WorkspaceSnapshot {
    pub(super) fingerprint: String,
    pub(super) safe_to_compare: bool,
}

pub(super) fn workspace_snapshot(cwd: &Path, config: &ServerConfig) -> WorkspaceSnapshot {
    let cwd_text = cwd.to_string_lossy();
    let root = crate::git::resolve_git_workspace(Some(cwd_text.as_ref()), config).or_else(|_| {
        std::fs::canonicalize(cwd)
            .map_err(|_| McpError::InvalidRequest("workspace snapshot root is inaccessible".into()))
    });
    let Ok(root) = root else {
        return WorkspaceSnapshot {
            fingerprint: String::new(),
            safe_to_compare: false,
        };
    };
    match fingerprint_workspace_metadata(&root) {
        Ok(fingerprint) => WorkspaceSnapshot {
            fingerprint,
            safe_to_compare: true,
        },
        Err(_) => WorkspaceSnapshot {
            fingerprint: String::new(),
            safe_to_compare: false,
        },
    }
}

fn fingerprint_workspace_metadata(root: &Path) -> Result<String, std::io::Error> {
    let mut digest = Context::new(&SHA256);
    let mut stack = vec![root.to_path_buf()];
    let mut scanned = 0usize;
    while let Some(directory) = stack.pop() {
        let mut entries = std::fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| std::io::Error::other("workspace snapshot escaped root"))?;
            if relay_core::protected_paths::is_protected_relative(relative) {
                continue;
            }
            scanned = scanned.saturating_add(1);
            if scanned > MAX_SNAPSHOT_ENTRIES {
                return Err(std::io::Error::other(
                    "workspace snapshot exceeds entry maximum",
                ));
            }
            let metadata = std::fs::symlink_metadata(&path)?;
            digest.update(relative.as_os_str().as_bytes());
            digest.update(&[0]);
            for value in [
                metadata.dev(),
                metadata.ino(),
                metadata.mode() as u64,
                metadata.len(),
                metadata.mtime() as u64,
                metadata.mtime_nsec() as u64,
                metadata.ctime() as u64,
                metadata.ctime_nsec() as u64,
            ] {
                digest.update(&value.to_le_bytes());
            }
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                digest.update(b"d");
                stack.push(path);
            } else if file_type.is_file() {
                digest.update(b"f");
            } else if file_type.is_symlink() {
                digest.update(b"l");
                digest.update(std::fs::read_link(&path)?.as_os_str().as_bytes());
            } else {
                return Err(std::io::Error::other(
                    "workspace snapshot encountered unsupported entry type",
                ));
            }
        }
    }
    let value = digest.finish();
    Ok(value
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
