use ai_tools::core::protected_paths::{
    contains_protected_path_reference, git_clean_exclusion_patterns, git_exclusion_pathspecs,
    git_mutation_exclusion_pathspecs, is_protected_relative,
};
use std::path::Path;

#[test]
fn traversal_prefilter_covers_every_canonical_protected_family() {
    use ai_tools::core::protected_paths::{
        may_be_protected_entry, PROTECTED_DIRECTORIES, PROTECTED_FILES,
    };
    for path in PROTECTED_DIRECTORIES
        .iter()
        .chain(PROTECTED_FILES.iter())
        .chain([".env.local", ".env.production"].iter())
    {
        assert!(is_protected_relative(Path::new(path)));
        assert!(may_be_protected_entry(Path::new(path).file_name().unwrap()));
    }
    assert!(!may_be_protected_entry(std::ffi::OsStr::new("ordinary.rs")));
    assert!(!may_be_protected_entry(std::ffi::OsStr::new(
        ".env.example"
    )));
}

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
fn metadata_path_detector_handles_git_punctuation() {
    assert!(contains_protected_path_reference(
        "rename to .config/gh/hosts.yml,"
    ));
    assert!(contains_protected_path_reference(
        "--- nested/.env.production:1"
    ));
    assert!(contains_protected_path_reference("copy from .ssh/key)"));
    assert!(!contains_protected_path_reference("+++ .env.example"));
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
