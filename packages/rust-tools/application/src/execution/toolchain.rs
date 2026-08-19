//! Reviewed toolchain-root detection shared by executable resolution and sandbox mounts.

use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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
    let mut entries: Vec<PathBuf> = DEFAULT_PATHS.iter().map(PathBuf::from).collect();
    if let Ok(home) = super::sandbox::runtime_home() {
        for sub in [".cargo/bin", ".local/bin"] {
            let dir = home.join(sub);
            if dir.is_dir() {
                entries.push(dir);
            }
        }
    }
    for p in &config.toolchain_paths {
        entries.push(std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)));
    }
    entries
}

pub(crate) fn resolve_safe_executable(
    config: &ServerConfig,
    binary: &str,
) -> Result<PathBuf, McpError> {
    relay_core::terminal_policy::validate_executable(binary, config.allow_docker)?;
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
