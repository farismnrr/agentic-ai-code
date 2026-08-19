//! Reviewed toolchain-root detection shared by executable resolution and sandbox mounts.

use std::ffi::OsStr;
use std::path::Path;

pub(super) fn reviewed_root(bin_dir: &Path) -> Option<&Path> {
    if bin_dir.file_name() != Some(OsStr::new("bin")) {
        return None;
    }
    let root = bin_dir.parent()?;
    let recognized = root.join("lib/rustlib").is_dir()
        || (root.join("lib/node_modules").is_dir() && root.join("bin/node").is_file());
    recognized.then_some(root)
}
