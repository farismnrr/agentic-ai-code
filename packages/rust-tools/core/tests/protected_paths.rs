use relay_core::protected_paths::{
    git_clean_exclusion_patterns, git_exclusion_pathspecs, git_mutation_exclusion_pathspecs,
    is_protected_relative,
};
use std::path::Path;

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
