//! One protected credential-path policy shared by native tools and sandboxes.
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub const PROTECTED_DIRECTORIES: [&str; 6] = [
    ".ssh",
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
            Some(".ssh" | ".aws" | ".docker" | ".kube")
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

/// Git whole-repository operations must exclude every protected path without
/// relying on output filtering alone. `.env.*` is intentionally excluded as a
/// broad family here, including `.env.example`; callers may still allow an
/// explicit `.env.example` path after `is_protected_relative` validation.
pub fn git_exclusion_pathspecs() -> Vec<String> {
    let mut pathspecs = Vec::with_capacity(PROTECTED_DIRECTORIES.len() + PROTECTED_FILES.len() + 1);
    for directory in PROTECTED_DIRECTORIES {
        pathspecs.push(format!(":(glob,exclude)**/{directory}/**"));
    }
    for file in PROTECTED_FILES {
        pathspecs.push(format!(":(glob,exclude)**/{file}"));
    }
    pathspecs.push(":(glob,exclude)**/.env.*".into());
    pathspecs
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
    fn git_exclusions_cover_dynamic_env_variants() {
        let exclusions = git_exclusion_pathspecs();
        assert!(exclusions
            .iter()
            .any(|item| item == ":(glob,exclude)**/.env"));
        assert!(exclusions
            .iter()
            .any(|item| item == ":(glob,exclude)**/.env.*"));
    }
}
