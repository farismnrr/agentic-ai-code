use super::super::toolchain;
use super::{masks, runtime_home, safe_path_entries};
use crate::core::config::ServerConfig;
use std::path::Path;
use std::time::Instant;

pub(crate) fn prime_protected_path_indexes(config: &ServerConfig) {
    let _ = config.ensure_workspaces_initialized();
    let workspace_roots = config
        .workspaces
        .read()
        .map(|workspaces| workspaces.all_roots())
        .unwrap_or_default();
    // Workspace roots may represent a broad Projects tree. Their protected
    // paths are indexed per selected sandbox before spawn, not eagerly scanned
    // here before the relay can accept requests.
    let mut roots = std::collections::BTreeSet::new();
    let home = runtime_home().ok();
    let canonical_home_cargo_bin = home
        .as_ref()
        .and_then(|home| std::fs::canonicalize(home.join(".cargo/bin")).ok());
    let mut toolchain_roots = std::collections::BTreeSet::new();

    for configured in &config.toolchain_paths {
        if let Ok(canonical) = std::fs::canonicalize(configured) {
            toolchain_roots.insert(canonical.clone());
            if let Some(root) = toolchain::reviewed_root(&canonical) {
                toolchain_roots.insert(root.to_path_buf());
            }
            if canonical_home_cargo_bin.as_ref() == Some(&canonical) {
                if let Some(home) = &home {
                    for subdirectory in [".cargo", ".rustup"] {
                        let candidate = home.join(subdirectory);
                        if candidate.is_dir() {
                            toolchain_roots.insert(candidate);
                        }
                    }
                }
            }
        }
    }
    for discovered in safe_path_entries(config) {
        let Ok(canonical) = std::fs::canonicalize(&discovered) else {
            continue;
        };
        if canonical.starts_with(Path::new("/usr"))
            || canonical.starts_with(Path::new("/bin"))
            || canonical.starts_with(Path::new("/sbin"))
            || canonical.starts_with(Path::new("/lib"))
            || canonical.starts_with(Path::new("/opt"))
            || workspace_roots
                .iter()
                .any(|workspace| canonical.starts_with(workspace))
        {
            continue;
        }
        toolchain_roots.insert(canonical);
    }
    for root in &toolchain_roots {
        if workspace_roots
            .iter()
            .any(|workspace| root.starts_with(workspace))
        {
            continue;
        }
        if !toolchain_roots
            .iter()
            .any(|other| other != root && root.starts_with(other))
        {
            roots.insert(root.clone());
        }
    }

    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "protected_path_cache_prime",
        workspace_root_count = workspace_roots.len(),
        toolchain_root_count = roots.len(),
    );
    for root in roots {
        let started = Instant::now();
        match masks::prime_protected_path_index(&root) {
            Ok(scanned_entries) => tracing::info!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "completed",
                scanned_entries,
                duration_ms = started.elapsed().as_millis() as u64,
            ),
            Err(error) => tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "failed",
                error_kind = ?error.kind(),
                duration_ms = started.elapsed().as_millis() as u64,
            ),
        }
    }
}
