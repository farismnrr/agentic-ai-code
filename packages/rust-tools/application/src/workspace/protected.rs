//! Shared native-workspace protection for owner credential stores.
use relay_core::error::McpError;
use std::path::Path;

pub(super) fn reject_protected_path(root: &Path, target: &Path) -> Result<(), McpError> {
    let Ok(relative) = target.strip_prefix(root) else {
        return Err(McpError::InvalidRequest(
            "path is outside execution root".into(),
        ));
    };
    if relay_core::protected_paths::is_protected_relative(relative) {
        return Err(McpError::InvalidRequest(
            "path is protected by workspace policy".into(),
        ));
    }
    Ok(())
}
