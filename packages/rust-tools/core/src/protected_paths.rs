//! One protected credential-path policy shared by native tools and sandboxes.
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub const PROTECTED_DIRECTORIES: [&str; 7] = [
    ".ssh",
    ".gnupg",
    ".aws",
    ".config/gcloud",
    ".config/gh",
    ".docker",
    ".kube",
];
pub const PROTECTED_FILES: [&str; 7] = [
    ".npmrc",
    ".netrc",
    ".pypirc",
    ".git-credentials",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".env",
];

pub fn is_protected_relative(path: &Path) -> bool {
    let components: Vec<&OsStr> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect();

    components.iter().any(|component| {
        matches!(
            component.to_str(),
            Some(".ssh" | ".gnupg" | ".aws" | ".docker" | ".kube")
        )
    }) || components.windows(2).any(|pair| {
        pair[0] == OsStr::new(".config") && matches!(pair[1].to_str(), Some("gcloud" | "gh"))
    }) || components.iter().any(|component| {
        matches!(
            component.to_str(),
            Some(".npmrc" | ".netrc" | ".pypirc" | ".git-credentials")
        )
    }) || components.windows(2).any(|pair| {
        pair[0] == OsStr::new(".cargo")
            && matches!(pair[1].to_str(), Some("credentials" | "credentials.toml"))
    }) || components.iter().any(|name| {
        let bytes = name.as_encoded_bytes();
        bytes == b".env" || (bytes.starts_with(b".env.") && bytes != b".env.example")
    })
}

/// Fail-closed detector for Git metadata lines that may quote or prefix paths.
/// It intentionally accepts mild over-blocking on metadata, but `.env.example`
/// remains an explicit non-secret exception.
pub fn contains_protected_path_reference(value: &str) -> bool {
    let bytes = value.as_bytes();
    for (index, _) in value.match_indices('.') {
        let start_ok =
            index == 0 || matches!(bytes[index - 1], b'/' | b'\\' | b' ' | b'\t' | b'"' | b'\'');
        if !start_ok {
            continue;
        }
        let tail = &value[index..];
        let end = tail
            .find(['/', '\\', ' ', '\t', '"', '\''])
            .unwrap_or(tail.len());
        let component = &tail[..end];
        if matches!(
            component,
            ".ssh"
                | ".gnupg"
                | ".aws"
                | ".docker"
                | ".kube"
                | ".npmrc"
                | ".netrc"
                | ".pypirc"
                | ".git-credentials"
        ) || component == ".env"
            || (component.starts_with(".env.") && component != ".env.example")
        {
            return true;
        }
        if component == ".config" {
            let rest = tail.get(end + 1..).unwrap_or("");
            if rest.starts_with("gcloud/")
                || rest == "gcloud"
                || rest.starts_with("gh/")
                || rest == "gh"
            {
                return true;
            }
        }
        if component == ".cargo" {
            let rest = tail.get(end + 1..).unwrap_or("");
            if rest.starts_with("credentials/")
                || rest == "credentials"
                || rest.starts_with("credentials.toml/")
                || rest == "credentials.toml"
            {
                return true;
            }
        }
    }
    false
}

pub fn is_protected_path(root: &Path, target: &Path) -> bool {
    let Ok(relative) = target.strip_prefix(root) else {
        return false;
    };
    is_protected_relative(relative)
}

pub fn protected_paths(root: &Path) -> impl Iterator<Item = PathBuf> + '_ {
    PROTECTED_DIRECTORIES
        .iter()
        .chain(PROTECTED_FILES.iter())
        .map(|entry| root.join(entry))
}

/// Git whole-repository operations exclude exact static protected roots.
/// Dynamic `.env.*` changes are rejected by Git change/rename preflight;
/// omitting the broad family here preserves safe `.env.example` visibility.
pub fn git_exclusion_pathspecs() -> Vec<String> {
    let mut pathspecs = Vec::with_capacity(PROTECTED_DIRECTORIES.len() + PROTECTED_FILES.len());
    for directory in PROTECTED_DIRECTORIES {
        pathspecs.push(format!(":(glob,exclude)**/{directory}/**"));
    }
    for file in PROTECTED_FILES {
        pathspecs.push(format!(":(glob,exclude)**/{file}"));
    }
    pathspecs
}

/// Mutation pathspec exclusions are intentionally stricter than read presentation:
/// broad mutations also skip every `.env.*` variant, including `.env.example`.
pub fn git_mutation_exclusion_pathspecs() -> Vec<String> {
    let mut pathspecs = git_exclusion_pathspecs();
    pathspecs.push(":(glob,exclude)**/.env.*".into());
    pathspecs
}

/// `git clean -e` patterns protecting credential-shaped paths from broad cleanup.
pub fn git_clean_exclusion_patterns() -> Vec<String> {
    let mut patterns = Vec::with_capacity(PROTECTED_DIRECTORIES.len() + PROTECTED_FILES.len() + 1);
    for directory in PROTECTED_DIRECTORIES {
        patterns.push(format!("**/{directory}/**"));
    }
    for file in PROTECTED_FILES {
        patterns.push(format!("**/{file}"));
    }
    patterns.push("**/.env.*".into());
    patterns
}

/// Ripgrep receives exact protected roots/files from the canonical policy.
/// Dynamic `.env.*` variants are additionally protected by Bubblewrap masking
/// and parsed-result path validation, preserving `.env.example` searchability.
pub fn ripgrep_exclusion_globs() -> Vec<String> {
    let mut globs = Vec::with_capacity(PROTECTED_DIRECTORIES.len() + PROTECTED_FILES.len());
    for directory in PROTECTED_DIRECTORIES {
        globs.push(format!("!**/{directory}/**"));
    }
    for file in PROTECTED_FILES {
        globs.push(format!("!**/{file}"));
    }
    globs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_policy_keeps_only_example_public() {
        assert!(is_protected_relative(Path::new(".env")));
        assert!(is_protected_relative(Path::new("nested/.env.local")));
        assert!(is_protected_relative(Path::new("nested/.env.production")));
        assert!(is_protected_relative(Path::new(".env.production/secret")));
        assert!(!is_protected_relative(Path::new(".env.example")));
        assert!(!is_protected_relative(Path::new("nested/.env.example")));
    }

    #[test]
    fn git_exclusions_cover_static_protected_files() {
        let exclusions = git_exclusion_pathspecs();
        assert!(exclusions
            .iter()
            .any(|item| item == ":(glob,exclude)**/.env"));
        assert!(exclusions
            .iter()
            .any(|item| item == ":(glob,exclude)**/.gnupg/**"));
        assert!(!exclusions
            .iter()
            .any(|item| item == ":(glob,exclude)**/.env.*"));
        assert!(git_mutation_exclusion_pathspecs()
            .iter()
            .any(|item| item == ":(glob,exclude)**/.env.*"));
        assert!(git_clean_exclusion_patterns()
            .iter()
            .any(|item| item == "**/.env.*"));
    }
}
