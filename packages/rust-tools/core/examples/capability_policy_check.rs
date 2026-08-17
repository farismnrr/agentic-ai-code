use relay_core::config::ServerConfig;
use relay_core::protected_paths::is_protected_relative;
use std::path::Path;

fn main() {
    for path in [
        ".ssh",
        ".ssh/config",
        ".config/gcloud/credentials.db",
        ".cargo/credentials",
    ] {
        assert!(
            is_protected_relative(Path::new(path)),
            "{path} must be protected"
        );
    }
    for path in [
        ".ssh-cache",
        ".npmrc.bak",
        ".env",
        ".env.example",
        "src/main.rs",
    ] {
        assert!(
            !is_protected_relative(Path::new(path)),
            "{path} must not be overblocked"
        );
    }
    assert!(!ServerConfig::default().allow_terminal_network);
    println!("capability policy boundary: PASS");
}
