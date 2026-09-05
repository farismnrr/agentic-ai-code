//! Complete bounded masking for every exposed user tree; never prune visible caches.
use std::path::{Path, PathBuf};
const MAX_PROTECTED_SCAN_ENTRIES: usize = 500_000;

pub(super) fn mask_executables(
    args: &mut Vec<String>,
    config: &crate::core::config::ServerConfig,
    names: &[&str],
) -> Result<(), std::io::Error> {
    let mut masked = std::collections::BTreeSet::new();
    for directory in super::safe_path_entries(config) {
        for name in names {
            let candidate = directory.join(name);
            match std::fs::symlink_metadata(&candidate) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    return Err(std::io::Error::other(
                        "forbidden executable metadata is unavailable",
                    ))
                }
                Ok(_) => {}
            }
            let canonical = std::fs::canonicalize(&candidate)
                .map_err(|_| std::io::Error::other("forbidden executable target is unsafe"))?;
            if masked.insert(canonical.clone()) {
                mask_protected_file(args, &canonical)?;
            }
            // /bin and /usr/bin can be separate bind mounts even on a merged-
            // usr host. Mask each visible spelling, not just its host inode.
            if masked.insert(candidate.clone()) {
                args.extend([
                    "--ro-bind".into(),
                    "/dev/null".into(),
                    candidate.to_string_lossy().into_owned(),
                ]);
            }
        }
    }
    Ok(())
}

pub(super) fn mask_protected_file(
    args: &mut Vec<String>,
    path: &Path,
) -> Result<(), std::io::Error> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            return Err(std::io::Error::other(
                "protected path metadata is unavailable",
            ))
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "protected toolchain credential path is a symbolic link",
        ));
    }
    if metadata.is_file() {
        args.extend([
            "--ro-bind".into(),
            "/dev/null".into(),
            path.to_string_lossy().into_owned(),
        ]);
    }
    Ok(())
}

pub(super) fn add_optional_socket(
    args: &mut Vec<String>,
    enabled: bool,
    configured_path: &str,
    name: &str,
) -> Result<(), std::io::Error> {
    if !enabled {
        return Ok(());
    }
    let socket = Path::new(configured_path);
    if !socket.exists() {
        return Err(std::io::Error::other(format!(
            "{name} access enabled but socket '{}' is unavailable",
            socket.display()
        )));
    }
    let value = socket.to_string_lossy().into_owned();
    args.extend(["--bind".into(), value.clone(), value]);
    Ok(())
}

pub(super) fn add_protected_paths(
    args: &mut Vec<String>,
    execution_root: &Path,
    recursive: bool,
    skip: Option<&Path>,
) -> Result<(), std::io::Error> {
    let paths = if recursive {
        discover_protected_paths(execution_root).map_err(|_| {
            std::io::Error::other("protected-path discovery could not complete safely")
        })?
    } else {
        crate::core::protected_paths::protected_paths(execution_root).collect()
    };
    for path in paths.into_iter().filter(|p| skip != Some(p.as_path())) {
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if !recursive && error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(std::io::Error::other(
                    "protected path metadata is unavailable",
                ))
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(std::io::Error::other(
                "protected sandbox path is a symbolic link",
            ));
        }
        if metadata.is_dir() {
            args.extend(["--tmpfs".into(), path.to_string_lossy().into_owned()]);
        } else {
            args.extend([
                "--ro-bind".into(),
                "/dev/null".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
    }
    Ok(())
}

pub(super) fn mask_state(
    args: &mut Vec<String>,
    config: &crate::core::config::ServerConfig,
    root: &Path,
) -> Result<(), std::io::Error> {
    let state = config.activity.resolved_state_dir()?;
    match std::fs::symlink_metadata(&state) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(std::io::Error::other("state root metadata unavailable")),
        Ok(_) => {}
    }
    let canonical = std::fs::canonicalize(&state)?;
    if canonical != state {
        return Err(std::io::Error::other("state root is not canonical"));
    }
    if root.starts_with(&state) {
        return Err(std::io::Error::other("workspace overlaps protected state"));
    }
    if state.starts_with(root) {
        args.extend(["--tmpfs".into(), state.to_string_lossy().into_owned()]);
    }
    Ok(())
}

fn discover_protected_paths(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut protected = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut scanned = 0usize;
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            scanned = scanned.saturating_add(1);
            if scanned > MAX_PROTECTED_SCAN_ENTRIES {
                return Err(std::io::Error::other(
                    "protected-path scan exceeds bounded workspace maximum",
                ));
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| std::io::Error::other("protected-path scan escaped workspace"))?;
            let kind = entry.file_type()?;
            if (crate::core::protected_paths::may_be_protected_entry(&entry.file_name())
                && crate::core::protected_paths::is_protected_relative(relative))
                || is_socket(&kind)
            {
                protected.push(path);
            } else if kind.is_dir() {
                stack.push(path);
            }
        }
    }
    Ok(protected)
}

fn is_socket(kind: &std::fs::FileType) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        kind.is_socket()
    }
    #[cfg(not(unix))]
    {
        let _ = kind;
        false
    }
}
