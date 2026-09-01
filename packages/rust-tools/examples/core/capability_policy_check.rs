use ai_tools::core::config::ServerConfig;
use ai_tools::core::protected_paths::is_protected_relative;
use std::path::Path;

fn main() {
    for path in [
        ".ssh",
        ".ssh/config",
        ".config/gcloud/credentials.db",
        "Projects/app/.config/gcloud/credentials.db",
        ".config/gh/hosts.yml",
        "Projects/app/.config/gh/hosts.yml",
        ".cargo/credentials",
        ".git-credentials",
        "Projects/app/.git-credentials",
        "Projects/app/.npmrc",
        ".env",
        "Projects/app/.env",
        "Projects/app/.env.local",
    ] {
        assert!(
            is_protected_relative(Path::new(path)),
            "{path} must be protected"
        );
    }
    for path in [
        ".ssh-cache",
        ".npmrc.bak",
        ".env.example",
        "Projects/app/.env.example",
        ".envoy",
        "src/main.rs",
    ] {
        assert!(
            !is_protected_relative(Path::new(path)),
            "{path} must not be overblocked"
        );
    }
    for line in [
        "diff --git a/.env.local b/.env.local",
        "+++ b/nested/.config/gh/hosts.yml",
        "rename to project/.git-credentials",
    ] {
        assert!(
            ai_tools::core::protected_paths::contains_protected_path_reference(line),
            "{line} must be detected in Git metadata"
        );
    }
    assert!(
        !ai_tools::core::protected_paths::contains_protected_path_reference(
            "diff --git a/.env.example b/.env.example"
        )
    );
    assert!(!ServerConfig::default().allow_terminal_network);
    println!("capability policy boundary: PASS");
}
