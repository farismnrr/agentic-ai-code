//! Add the reviewed runtime/toolchain mounts and mask their credential state.

use super::masks::{self, add_protected_paths, mask_protected_file, ProtectedPathFreshness};
use super::{paths, SandboxError, SpawnControl};
use crate::core::config::ServerConfig;
use std::path::{Path, PathBuf};

fn runtime_tool_home(variable: &str, fallback: PathBuf) -> PathBuf {
    if let Some(value) = std::env::var_os(variable) {
        let path = PathBuf::from(value);
        if path.is_absolute() && path.is_dir() {
            return path;
        }
    }
    fallback
}

pub(super) struct ToolchainMounts {
    pub(super) cargo_home: Option<String>,
    pub(super) rustup_home: Option<String>,
    pub(super) freshness_checks: Vec<ProtectedPathFreshness>,
}

pub(super) fn mount_toolchains(
    args: &mut Vec<String>,
    config: &ServerConfig,
    sandbox_root: &Path,
    host_home: &Path,
    control: Option<&SpawnControl<'_>>,
) -> Result<ToolchainMounts, SandboxError> {
    let mut cargo_home = None;
    let mut rustup_home = None;
    let mut toolchain_roots = std::collections::BTreeSet::new();
    let mut freshness_checks: Vec<ProtectedPathFreshness> = Vec::new();
    let cargo_store = runtime_tool_home("CARGO_HOME", host_home.join(".cargo"));
    let rustup_store = runtime_tool_home("RUSTUP_HOME", host_home.join(".rustup"));
    // User-toolchain commands must see the same resolved Cargo/Rustup stores
    // that executable discovery used. Mount these homes eagerly instead of
    // depending on a later path-shape branch to rediscover them.
    if cargo_store.is_dir() {
        let value = cargo_store.to_string_lossy().into_owned();
        if !cargo_store.starts_with(sandbox_root) {
            args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
            toolchain_roots.insert(cargo_store.clone());
        }
        for file in ["credentials", "credentials.toml"] {
            mask_protected_file(args, &cargo_store.join(file))?;
        }
        cargo_home = Some(value);
    }
    if rustup_store.is_dir() {
        let value = rustup_store.to_string_lossy().into_owned();
        if !rustup_store.starts_with(sandbox_root) {
            args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
            toolchain_roots.insert(rustup_store.clone());
        }
        rustup_home = Some(value);
    }

    for path in &config.toolchain_paths {
        if let Some(control) = control {
            control.check()?;
        }
        let configured = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&configured)
            .map_err(|_| std::io::Error::other("invalid toolchain path"))?;
        let canonical_value = canonical.to_string_lossy().into_owned();
        toolchain_roots.insert(canonical.clone());
        args.extend([
            "--ro-bind".into(),
            canonical_value.clone(),
            canonical_value.clone(),
        ]);
        // Preserve configured paths so shims retain argv[0] semantics, while
        // also mounting their canonical targets into the Bubblewrap namespace.
        let configured_value = configured.to_string_lossy().into_owned();
        if configured != canonical && !configured.starts_with(sandbox_root) {
            if configured.starts_with(host_home) {
                paths::add_bwrap_parent_dirs(args, configured.parent(), host_home);
                args.extend([
                    "--symlink".into(),
                    canonical_value.clone(),
                    configured_value,
                ]);
            } else {
                args.extend([
                    "--ro-bind".into(),
                    canonical_value.clone(),
                    configured_value,
                ]);
            }
        }
        if let Some(toolchain_root) = super::super::toolchain::reviewed_root(&canonical) {
            toolchain_roots.insert(toolchain_root.to_path_buf());
            let value = toolchain_root.to_string_lossy().into_owned();
            args.extend(["--ro-bind".into(), value.clone(), value]);
        }
    }

    for path in &config.toolchain_state_paths {
        if let Some(control) = control {
            control.check()?;
        }
        let configured = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&configured)
            .map_err(|_| std::io::Error::other("invalid toolchain state path"))?;
        let source = canonical.to_string_lossy().into_owned();
        let destination = configured.to_string_lossy().into_owned();
        if configured.starts_with(host_home) {
            paths::add_bwrap_parent_dirs(args, configured.parent(), host_home);
        }
        args.extend(["--ro-bind".into(), source, destination]);
        toolchain_roots.insert(canonical);
    }

    let safe_path_entries = super::super::toolchain::safe_path_entries(config);

    // Expose only validated executable directories from user-managed runtimes,
    // never their surrounding profiles or credential stores.
    for discovered in safe_path_entries {
        if let Some(control) = control {
            control.check()?;
        }
        let Ok(canonical) = std::fs::canonicalize(&discovered) else {
            continue;
        };
        if canonical.starts_with(sandbox_root)
            || canonical.starts_with(Path::new("/usr"))
            || canonical.starts_with(Path::new("/bin"))
            || canonical.starts_with(Path::new("/sbin"))
            || canonical.starts_with(Path::new("/lib"))
            || canonical.starts_with(Path::new("/opt"))
            || toolchain_roots.contains(&canonical)
        {
            continue;
        }
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
        let configured_value = discovered.to_string_lossy().into_owned();
        if discovered != canonical {
            paths::add_bwrap_parent_dirs(args, discovered.parent(), host_home);
            args.extend([
                "--symlink".into(),
                canonical.to_string_lossy().into_owned(),
                configured_value,
            ]);
        }
        toolchain_roots.insert(canonical);
    }

    for toolchain_root in &toolchain_roots {
        if let Some(control) = control {
            control.check()?;
        }
        if !toolchain_root.starts_with(sandbox_root)
            && !toolchain_roots
                .iter()
                .any(|other| other != toolchain_root && toolchain_root.starts_with(other))
        {
            add_protected_paths(
                args,
                toolchain_root,
                true,
                None,
                "toolchain",
                control,
                &mut freshness_checks,
            )
            .map_err(|error| SandboxError::at("protected_path_discovery", error))?;
            masks::mask_state(args, config, toolchain_root)?;
        }
    }

    Ok(ToolchainMounts {
        cargo_home,
        rustup_home,
        freshness_checks,
    })
}
