//! One protected credential-path policy shared by native tools and sandboxes.
use std::path::{Path, PathBuf};

pub const PROTECTED_DIRECTORIES: [&str; 5] = [".ssh", ".aws", ".config/gcloud", ".docker", ".kube"];
pub const PROTECTED_FILES: [&str; 5] = [
    ".npmrc",
    ".netrc",
    ".pypirc",
    ".cargo/credentials",
    ".cargo/credentials.toml",
];

pub fn is_protected_relative(path: &Path) -> bool {
    PROTECTED_DIRECTORIES
        .iter()
        .any(|entry| is_same_or_descendant(path, Path::new(entry)))
        || PROTECTED_FILES.iter().any(|entry| path == Path::new(entry))
}

fn is_same_or_descendant(path: &Path, ancestor: &Path) -> bool {
    path == ancestor || path.strip_prefix(ancestor).is_ok()
}

pub fn protected_paths(root: &Path) -> impl Iterator<Item = PathBuf> + '_ {
    PROTECTED_DIRECTORIES
        .iter()
        .chain(PROTECTED_FILES.iter())
        .map(|entry| root.join(entry))
}
