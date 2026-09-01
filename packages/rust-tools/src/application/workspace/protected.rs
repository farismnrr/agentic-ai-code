//! Shared native-workspace protection for owner credential stores.
use crate::core::error::McpError;
use std::path::Path;

pub(super) fn reject_protected_path(root: &Path, target: &Path) -> Result<(), McpError> {
    if !target.starts_with(root) {
        return Err(McpError::InvalidRequest(
            "path is outside execution root".into(),
        ));
    }
    if crate::core::protected_paths::is_protected_path(root, target) {
        return Err(McpError::InvalidRequest(
            "path is protected by workspace policy".into(),
        ));
    }
    Ok(())
}

pub(super) fn is_protected_discovered_path(root: &Path, target: &Path) -> bool {
    if crate::core::protected_paths::is_protected_path(root, target) {
        return true;
    }
    std::fs::canonicalize(target)
        .map(|canonical| crate::core::protected_paths::is_protected_path(root, &canonical))
        .unwrap_or(false)
}
