//! Shared native-workspace protection for owner credential stores.
use relay_core::error::McpError;
use std::path::Path;

pub(super) fn reject_protected_path(root: &Path, target: &Path) -> Result<(), McpError> {
    let Ok(relative) = target.strip_prefix(root) else {
        return Err(McpError::InvalidRequest(
            "path is outside execution root".into(),
        ));
    };
    const DIRS: [&str; 5] = [".ssh", ".aws", ".config/gcloud", ".docker", ".kube"];
    const FILES: [&str; 5] = [
        ".npmrc",
        ".netrc",
        ".pypirc",
        ".cargo/credentials",
        ".cargo/credentials.toml",
    ];
    if DIRS.iter().any(|p| relative.starts_with(p))
        || FILES.iter().any(|p| relative == Path::new(p))
    {
        return Err(McpError::InvalidRequest(
            "path is protected by workspace policy".into(),
        ));
    }
    Ok(())
}
