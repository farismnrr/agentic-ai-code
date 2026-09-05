//! Real Bubblewrap execution against disposable HOME fixtures, never owner secrets.
#![cfg(target_os = "linux")]
use ai_tools::application::execution::{
    start_terminal_job, start_terminal_job_for, JobManager, JobSnapshot,
};
use ai_tools::core::config::{ActivityConfig, ServerConfig};
use serde_json::json;
use std::{fs, path::Path, process::Command};

#[test]
fn broad_home_sandbox() {
    let root = std::env::temp_dir().join(format!("terminal-home-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let rust = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(rust.status.success());
    let rust_bin = Path::new(std::str::from_utf8(&rust.stdout).unwrap().trim()).join("bin");
    let node = Command::new("node")
        .args(["-p", "require('path').dirname(process.execPath)"])
        .output()
        .unwrap();
    assert!(node.status.success());
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "terminal_sandbox::home_fixture_child",
            "--nocapture",
        ])
        .env("HOME", &root)
        .env("XDG_STATE_HOME", root.join(".local/state"))
        .env("TERMINAL_RUST_FIXTURE_BIN", rust_bin)
        .env(
            "TERMINAL_NODE_FIXTURE_BIN",
            std::str::from_utf8(&node.stdout).unwrap().trim(),
        )
        .env("TERMINAL_HOME_FIXTURE", "1")
        .env("PARENT_SECRET_CANARY", "fixture-secret-only")
        .env("SSH_AUTH_SOCK", root.join("agent.sock"))
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/fixture/bus")
        .output()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn shell(config: &ServerConfig, cwd: &Path, script: &str) -> JobSnapshot {
    let manager = JobManager::new(config.clone());
    let id = start_terminal_job(
        &json!({"command":"sh", "args":["-c", script], "cwd":cwd, "timeout_ms":10000}),
        config,
        &manager,
    )
    .await
    .unwrap();
    let result = manager.wait(&id).await.unwrap();
    manager.shutdown().await;
    result
}

#[tokio::test]
async fn home_fixture_child() {
    if std::env::var("TERMINAL_HOME_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    let root = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap();
    let mut config = ServerConfig {
        dir: Some(root.to_string_lossy().into()),
        execution_root: Some(root.to_string_lossy().into()),
        activity: ActivityConfig {
            state_dir: Some(root.join("relay-state").to_string_lossy().into()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    };
    for dir in ["project-a", "project-b", "Downloads"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    let protected = [
        ".ssh/key",
        ".gnupg/key",
        ".aws/credentials",
        ".config/gcloud/token",
        ".config/gh/hosts.yml",
        ".docker/config.json",
        ".kube/config",
        ".npmrc",
        ".netrc",
        ".pypirc",
        ".git-credentials",
        ".cargo/credentials",
        ".cargo/credentials.toml",
        "project-a/.env",
        "project-b/.env.production",
        "project-a/node_modules/dependency/.env.local",
        ".local/share/keyrings/login.keyring",
        ".config/chromium/Default/Cookies",
        ".codex/auth.json",
        ".local/state/ai-tools/payload.key",
    ];
    for path in protected {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "fixture-protected-canary").unwrap();
    }
    fs::create_dir(root.join("relay-state")).unwrap();
    fs::write(
        root.join("relay-state/payload.key"),
        "fixture-protected-canary",
    )
    .unwrap();
    fs::write(root.join("project-a/.env.example"), "public-example").unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.join("agent.sock")).unwrap();
    for dir in ["project-a", "project-b", "Downloads"] {
        let result = shell(
            &config,
            &root.join(dir),
            "printf ordinary > ordinary.txt; cat ordinary.txt",
        )
        .await;
        assert_eq!(
            result.exit_code,
            Some(0),
            "ordinary execution failed: {}",
            result.stderr
        );
        assert_eq!(result.stdout, "ordinary");
    }
    let result = shell(&config, &root.join("project-a"), "cat ../project-b/ordinary.txt; cat .env.example; cat .env ../.ssh/key ../.cargo/credentials node_modules/dependency/.env.local 2>/dev/null; test ! -S ../agent.sock").await;
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert!(result.stdout.contains("public-example"));
    assert!(!result.stdout.contains("fixture-protected-canary"));
    for path in protected.into_iter().chain(["relay-state/payload.key"]) {
        let result = shell(
            &config,
            &root,
            &format!("cat {path} 2>/dev/null; printf changed > {path} 2>/dev/null"),
        )
        .await;
        assert!(!result.stdout.contains("fixture-protected-canary"));
        assert_eq!(
            fs::read_to_string(root.join(path)).unwrap(),
            "fixture-protected-canary"
        );
    }
    let result = shell(&config, &root, "test -z \"$PARENT_SECRET_CANARY$SSH_AUTH_SOCK$DBUS_SESSION_BUS_ADDRESS\"; test ! -e /run/user; test ! -S /var/run/docker.sock; test ! -S /var/run/tailscale/tailscaled.sock; cat /proc/self/status").await;
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert!(result.stdout.contains("NoNewPrivs:\t1"));
    assert!(result.stdout.contains("CapEff:\t0000000000000000"));
    for binary in [
        "sudo", "su", "doas", "pkexec", "runas", "ssh", "scp", "sftp",
    ] {
        let manager = JobManager::new(config.clone());
        assert!(
            start_terminal_job(&json!({"command":binary}), &config, &manager)
                .await
                .is_err()
        );
        let result = shell(&config, &root, &format!("{binary} --help")).await;
        assert_ne!(
            result.exit_code,
            Some(0),
            "wrapped broker executed: {binary}"
        );
        for prefix in ["/bin", "/usr/bin"] {
            let result = shell(&config, &root, &format!("{prefix}/{binary} --help")).await;
            assert_ne!(
                result.exit_code,
                Some(0),
                "absolute broker executed: {prefix}/{binary}"
            );
        }
    }
    let narrow = ServerConfig {
        dir: Some(root.join("project-a").to_string_lossy().into()),
        execution_root: Some(root.join("project-a").to_string_lossy().into()),
        ..ServerConfig::default()
    };
    let manager = JobManager::new(narrow.clone());
    assert!(start_terminal_job(
        &json!({"command":"pwd", "cwd":root.join("project-b")}),
        &narrow,
        &manager
    )
    .await
    .is_err());
    // Independent operator socket grants stay opt-in, including sockets in HOME.
    config.docker_socket = root.join("agent.sock").to_string_lossy().into();
    config.tailscale_socket = config.docker_socket.clone();
    assert_ne!(
        shell(&config, &root, "test -S agent.sock").await.exit_code,
        Some(0)
    );
    config.allow_docker = true;
    assert_eq!(
        shell(&config, &root, "test -S agent.sock").await.exit_code,
        Some(0)
    );
    config.allow_docker = false;
    config.allow_tailscale = true;
    assert_eq!(
        shell(&config, &root, "test -S agent.sock").await.exit_code,
        Some(0)
    );
    config.allow_tailscale = false;
    // Interpreter and an uncovered Git operation remain legitimate fallback.
    assert_eq!(
        shell(&config, &root, "python3 -c 'print(6 * 7)' ; git --version")
            .await
            .exit_code,
        Some(0)
    );
    // Complete scans include dependency/build/cache trees. A modest synthetic
    // home has thousands of entries; none may be skipped because of its name.
    let cache = root.join("project-b/target");
    fs::create_dir(&cache).unwrap();
    for i in 0..10_000 {
        fs::write(cache.join(format!("entry-{i}")), "ordinary").unwrap();
    }
    fs::write(cache.join(".env.hidden"), "fixture-protected-canary").unwrap();
    let result = shell(
        &config,
        &root,
        "cat project-b/target/.env.hidden 2>/dev/null; cat project-b/target/entry-9999",
    )
    .await;
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert_eq!(result.stdout, "ordinary", "{}", result.stderr);
    let redacted = shell(
        &config,
        &root,
        "printf 'Authorization: Bearer fixture-secret-token-0123456789'",
    )
    .await;
    assert!(!redacted.stdout.contains("fixture-secret-token-0123456789"));
    assert!(redacted.stdout.contains("Bearer [REDACTED]"));
    assert!(!redacted
        .job_json()
        .to_string()
        .contains("fixture-secret-token-0123456789"));
    assert!(!redacted
        .output_text()
        .contains("fixture-secret-token-0123456789"));
    // Failure to traverse any visible directory fails before the command runs.
    use std::os::unix::fs::PermissionsExt;
    let inaccessible = root.join("inaccessible");
    fs::create_dir(&inaccessible).unwrap();
    fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o0)).unwrap();
    let result = shell(&config, &root, "printf must-not-run").await;
    fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o700)).unwrap();
    assert_ne!(result.exit_code, Some(0));
    assert!(!result.stdout.contains("must-not-run"));
    // Reviewed Rust and a symlink-based Node installation are available without
    // inheriting the parent PATH, auth environment, or toolchain credentials.
    let node_link = root.join("node-bin");
    std::os::unix::fs::symlink(
        std::env::var_os("TERMINAL_NODE_FIXTURE_BIN").unwrap(),
        &node_link,
    )
    .unwrap();
    config.toolchain_paths = vec![
        std::env::var("TERMINAL_RUST_FIXTURE_BIN").unwrap(),
        node_link.to_string_lossy().into(),
    ];
    fs::write(
        root.join("project-a/Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("project-a/src")).unwrap();
    fs::write(
        root.join("project-a/src/main.rs"),
        "fn main() { println!(\"fixture-build-ok\"); }\n",
    )
    .unwrap();
    let result = shell(&config, &root.join("project-a"), "cargo build --offline && ./target/debug/fixture && node -e 'console.log(42)' && npm --version").await;
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert!(result.stdout.contains("fixture-build-ok"));
    assert!(result.stdout.contains("42"));
    config.toolchain_paths.clear();
    let manager = JobManager::new(config.clone());
    assert!(start_terminal_job(
        &json!({"command":"pwd", "cwd":root.join(".ssh")}),
        &config,
        &manager
    )
    .await
    .is_err());
    std::os::unix::fs::symlink(
        root.join("project-a/ordinary.txt"),
        root.join("project-b/.env.local"),
    )
    .unwrap();
    let result = shell(&config, &root, "printf must-not-run").await;
    assert_ne!(result.exit_code, Some(0));
    assert!(!result.stdout.contains("must-not-run"));
}

#[tokio::test]
async fn background_jobs_are_scoped_to_owner_and_session() {
    let root = std::env::temp_dir().join(format!("terminal-owner-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let config = ServerConfig {
        dir: Some(root.to_string_lossy().into()),
        execution_root: Some(root.to_string_lossy().into()),
        activity: ActivityConfig {
            state_dir: Some(root.join("state").to_string_lossy().into()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    };
    fs::create_dir_all(root.join("state")).unwrap();
    let manager = JobManager::new(config.clone());
    let id = start_terminal_job_for(
        &json!({"command":"true", "cwd":root, "timeout_ms":10000}),
        &config,
        &manager,
        "owner-a",
        Some("session-a"),
    )
    .await
    .unwrap();
    assert!(manager
        .get_for(&id, "owner-a", Some("session-a"))
        .await
        .is_some());
    assert!(manager
        .get_for(&id, "owner-b", Some("session-a"))
        .await
        .is_none());
    assert!(manager
        .get_for(&id, "owner-a", Some("session-b"))
        .await
        .is_none());
    assert!(manager
        .cancel_for(&id, "owner-b", Some("session-a"))
        .await
        .is_err());
    let _ = manager.wait(&id).await.unwrap();
    manager.shutdown().await;
    fs::remove_dir_all(root).unwrap();
}
