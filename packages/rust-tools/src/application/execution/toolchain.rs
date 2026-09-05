//! Reviewed toolchain-root detection shared by executable resolution and sandbox mounts.

use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const MAX_DISCOVERED_ENVIRONMENTS: usize = 32;

pub(super) fn reviewed_root(bin_dir: &Path) -> Option<&Path> {
    if bin_dir.file_name() != Some(OsStr::new("bin")) {
        return None;
    }
    let root = bin_dir.parent()?;
    let recognized = root.join("lib/rustlib").is_dir()
        || (root.join("lib/node_modules").is_dir() && root.join("bin/node").is_file());
    recognized.then_some(root)
}

pub(crate) fn safe_path_entries(config: &ServerConfig) -> Vec<PathBuf> {
    const DEFAULT_PATHS: &[&str] = &[
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ];
    let mut entries = Vec::new();
    for path in config.toolchain_paths.iter().map(PathBuf::from) {
        push_safe_directory(&mut entries, path, false);
    }
    for path in DEFAULT_PATHS.iter().map(PathBuf::from) {
        push_safe_directory(&mut entries, path, true);
    }
    if let Ok(home) = super::sandbox::runtime_home() {
        for sub in [
            ".cargo/bin",
            ".local/bin",
            ".local/share/fnm",
            ".volta/bin",
            ".asdf/shims",
            ".bun/bin",
            ".npm-global/bin",
            ".local/share/pnpm",
            ".nvm/current/bin",
            ".conda/bin",
            "miniconda3/bin",
            "anaconda3/bin",
            ".local/share/mamba/bin",
        ] {
            push_safe_directory(&mut entries, home.join(sub), false);
        }
        for env_root in [
            home.join(".conda/envs"),
            home.join("miniconda3/envs"),
            home.join("anaconda3/envs"),
            home.join(".local/share/mamba/envs"),
        ] {
            discover_conda_bins(&mut entries, &env_root);
        }
    }
    #[cfg(target_os = "macos")]
    for path in [
        "/opt/homebrew/bin",
        "/usr/local/homebrew/bin",
        "/home/linuxbrew/.linuxbrew/bin",
    ] {
        push_safe_directory(&mut entries, PathBuf::from(path), true);
    }
    #[cfg(target_os = "windows")]
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let profile = PathBuf::from(profile);
        for sub in ["scoop/shims", "AppData/Local/Microsoft/WinGet/Links"] {
            push_safe_directory(&mut entries, profile.join(sub), false);
        }
    }
    entries
}

fn discover_conda_bins(entries: &mut Vec<PathBuf>, env_root: &Path) {
    let Ok(read_dir) = std::fs::read_dir(env_root) else {
        return;
    };
    let mut count = 0;
    for entry in read_dir.flatten() {
        if count >= MAX_DISCOVERED_ENVIRONMENTS {
            break;
        }
        let path = entry.path().join("bin");
        let before = entries.len();
        push_safe_directory(entries, path, false);
        if entries.len() > before {
            count += 1;
        }
    }
}

fn push_safe_directory(entries: &mut Vec<PathBuf>, path: PathBuf, allow_system: bool) {
    if std::fs::symlink_metadata(&path).is_err() {
        return;
    }
    let Ok(metadata) = std::fs::metadata(&path) else {
        return;
    };
    if !metadata.is_dir() || !is_safe_directory(&metadata, allow_system) {
        return;
    }
    let Ok(canonical) = std::fs::canonicalize(&path) else {
        return;
    };
    if !canonical.is_dir() || entries.iter().any(|existing| existing == &path) {
        return;
    }
    entries.push(path);
}

fn is_safe_directory(metadata: &std::fs::Metadata, allow_system: bool) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let mode = metadata.permissions().mode();
        let writable_by_group_or_other = mode & 0o022 != 0;
        if writable_by_group_or_other {
            return false;
        }
        if allow_system {
            return true;
        }
        metadata.uid() == unsafe { libc::geteuid() }
    }
    #[cfg(not(unix))]
    {
        let _ = allow_system;
        true
    }
}

pub(crate) fn resolve_safe_executable(
    config: &ServerConfig,
    binary: &str,
) -> Result<PathBuf, McpError> {
    crate::core::terminal_policy::validate_executable(binary, config.allow_docker)?;
    let safe_entries = safe_path_entries(config);
    let mut canonical_safe_entries = safe_entries
        .iter()
        .filter_map(|directory| std::fs::canonicalize(directory).ok())
        .collect::<Vec<_>>();
    for path in &config.toolchain_paths {
        if let Ok(canonical) = std::fs::canonicalize(path) {
            if let Some(root) = reviewed_root(&canonical) {
                canonical_safe_entries.push(root.to_path_buf());
            }
        }
    }

    for directory in safe_entries {
        let candidate = directory.join(binary);
        if !candidate.is_file() || !is_executable(&candidate) {
            continue;
        }
        let canonical_target = std::fs::canonicalize(&candidate).map_err(|_| {
            McpError::InvalidRequest("configured executable target is unavailable".into())
        })?;
        if !canonical_safe_entries
            .iter()
            .any(|safe_root| canonical_target.starts_with(safe_root))
        {
            return Err(McpError::InvalidRequest(
                "configured executable symlink escapes the reviewed safe PATH roots".into(),
            ));
        }
        // Preserve the reviewed executable path instead of replacing it with
        // the canonical target. Multi-call shims such as rustup dispatch from
        // argv[0] (`cargo`, `rustc`, ...); canonicalizing the final symlink to
        // `rustup` changes program semantics even though the target is safe.
        return Ok(candidate);
    }
    Err(McpError::InvalidRequest(
        "command is not available in the configured safe PATH".into(),
    ))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}
