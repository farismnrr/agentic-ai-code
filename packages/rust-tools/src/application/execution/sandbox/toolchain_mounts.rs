//! Add the reviewed runtime/toolchain mounts and mask their credential state.

use super::masks::{self, add_protected_paths, mask_protected_file, ProtectedPathFreshness};
use super::{paths, SandboxError, SpawnControl};
use crate::core::config::ServerConfig;
use std::path::{Path, PathBuf};

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
    let home_cargo_bin = host_home.join(".cargo/bin");
    let canonical_home_cargo_bin = std::fs::canonicalize(&home_cargo_bin).ok();

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
        if canonical_home_cargo_bin.as_ref() == Some(&canonical) {
            let candidate = host_home.join(".cargo");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                if !candidate.starts_with(sandbox_root) {
                    args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                    toolchain_roots.insert(candidate.clone());
                }
                for file in ["credentials", "credentials.toml"] {
                    mask_protected_file(args, &candidate.join(file))?;
                }
                cargo_home = Some(value);
            }
            let candidate = host_home.join(".rustup");
            if candidate.is_dir() {
                let value = candidate.to_string_lossy().into_owned();
                if !candidate.starts_with(sandbox_root) {
                    args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                    toolchain_roots.insert(candidate.clone());
                }
                rustup_home = Some(value);
            }
        }
    }

    let safe_path_entries = super::super::toolchain::safe_path_entries(config);

    // Automatic ~/.cargo/bin discovery must carry the rustup state that makes
    // rustup's cargo/rustc proxy shims functional. Keep the state read-only and
    // mask Cargo credential files exactly as for an explicit toolchain path.
    if cargo_home.is_none()
        && canonical_home_cargo_bin.as_ref().is_some_and(|cargo_bin| {
            safe_path_entries
                .iter()
                .filter_map(|path| std::fs::canonicalize(path).ok())
                .any(|path| &path == cargo_bin)
        })
    {
        let candidate = host_home.join(".cargo");
        if candidate.is_dir() {
            let value = candidate.to_string_lossy().into_owned();
            if !candidate.starts_with(sandbox_root) {
                args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                toolchain_roots.insert(candidate.clone());
            }
            for file in ["credentials", "credentials.toml"] {
                mask_protected_file(args, &candidate.join(file))?;
            }
            cargo_home = Some(value);
        }
        let candidate = host_home.join(".rustup");
        if candidate.is_dir() {
            let value = candidate.to_string_lossy().into_owned();
            if !candidate.starts_with(sandbox_root) {
                args.extend(["--ro-bind".into(), value.clone(), value.clone()]);
                toolchain_roots.insert(candidate.clone());
            }
            rustup_home = Some(value);
        }
    }

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
