//! One protected credential-path policy shared by native tools and sandboxes.
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub const PROTECTED_DIRECTORIES: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".config/gcloud",
    ".config/gh",
    ".docker",
    ".kube",
    ".password-store",
    ".config/op",
    ".config/1Password",
    ".local/share/keyrings",
    ".local/share/kwalletd",
    ".mozilla",
    ".config/chromium",
    ".config/google-chrome",
    ".config/BraveSoftware",
    ".local/state/ai-tools",
];
pub const PROTECTED_FILES: &[&str] = &[
    ".npmrc",
    ".netrc",
    ".pypirc",
    ".git-credentials",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".env",
    ".codex/auth.json",
    ".claude/.credentials.json",
];

/// Traversal has already rejected protected ancestors. Avoid reconstructing
/// their component vectors for ordinary build/dependency entries.
pub fn may_be_protected_entry(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    bytes == b".env"
        || (bytes.starts_with(b".env.") && bytes != b".env.example")
        || PROTECTED_DIRECTORIES
            .iter()
            .chain(PROTECTED_FILES.iter())
            .any(|entry| {
                entry
                    .rsplit('/')
                    .next()
                    .is_some_and(|last| name == OsStr::new(last))
            })
}

pub fn is_protected_relative(path: &Path) -> bool {
    let components: Vec<&OsStr> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect();

    PROTECTED_DIRECTORIES
        .iter()
        .chain(PROTECTED_FILES.iter())
        .any(|entry| {
            components
                .windows(entry.split('/').count())
                .any(|window| window.iter().copied().eq(entry.split('/').map(OsStr::new)))
        })
        || components.iter().any(|name| {
            let bytes = name.as_encoded_bytes();
            bytes == b".env" || (bytes.starts_with(b".env.") && bytes != b".env.example")
        })
}

/// Fail-closed detector for Git metadata lines that may quote or prefix paths.
/// It intentionally accepts mild over-blocking on metadata, but `.env.example`
/// remains an explicit non-secret exception.
pub fn contains_protected_path_reference(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    normalized
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '"' | '\'' | ',' | ';' | ':' | '=' | '(' | ')' | '[' | ']' | '{' | '}'
                )
        })
        .any(|token| is_protected_relative(Path::new(token)))
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
