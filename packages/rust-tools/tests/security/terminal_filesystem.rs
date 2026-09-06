//! Behavior tests for filesystem boundaries, execution root ceiling, and read-only system mounts.
#![cfg(target_os = "linux")]

use super::terminal_sandbox::shell;
use ai_tools::application::execution::{start_terminal_job, JobManager};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::{fs, path::Path};

pub(super) async fn test_filesystem_boundaries(config: &ServerConfig, root: &Path) {
    // 1. Ordinary read/write in workspace and sibling project under HOME
    let ws_write = shell(
        config,
        &root.join("project-a"),
        "printf 'ws-write-content' > ws_write.txt && cat ws_write.txt",
    )
    .await;
    assert_eq!(ws_write.exit_code, Some(0), "{}", ws_write.stderr);
    assert_eq!(ws_write.stdout, "ws-write-content");

    let sibling_write = shell(
        config,
        &root.join("project-a"),
        "printf 'sibling-write-content' > ../project-b/sibling_write.txt && cat ../project-b/sibling_write.txt",
    )
    .await;
    assert_eq!(sibling_write.exit_code, Some(0), "{}", sibling_write.stderr);
    assert_eq!(sibling_write.stdout, "sibling-write-content");

    // 2. Write to /etc fails because it is mounted read-only
    let etc_write = shell(config, root, "touch /etc/ai_code_fs_test 2>&1").await;
    assert_ne!(etc_write.exit_code, Some(0), "writing to /etc must fail");
    assert!(
        etc_write.stdout.contains("Read-only file system")
            || etc_write.stderr.contains("Read-only file system")
    );

    // Reading /etc succeeds (runtime read-only access is expected)
    let etc_read = shell(config, root, "ls /etc >/dev/null && echo 'etc-readable'").await;
    assert_eq!(etc_read.exit_code, Some(0));
    assert!(etc_read.stdout.contains("etc-readable"));

    // 3. /tmp is isolated via Bubblewrap tmpfs
    let tmp_test = shell(
        config,
        root,
        "echo 'sandbox-tmp-canary' > /tmp/sandboxed_isolated_test.txt && cat /tmp/sandboxed_isolated_test.txt",
    )
    .await;
    assert_eq!(tmp_test.exit_code, Some(0));
    assert_eq!(tmp_test.stdout.trim(), "sandbox-tmp-canary");

    // The file must NOT exist on the host /tmp
    assert!(
        !Path::new("/tmp/sandboxed_isolated_test.txt").exists(),
        "sandbox tmp file must not leak to host /tmp"
    );

    // A subsequent sandbox process gets a fresh tmpfs
    let tmp_fresh = shell(
        config,
        root,
        "test ! -e /tmp/sandboxed_isolated_test.txt && echo 'tmp-fresh-ok'",
    )
    .await;
    assert_eq!(tmp_fresh.exit_code, Some(0));
    assert!(tmp_fresh.stdout.contains("tmp-fresh-ok"));

    // 4. Path outside execution root fails before spawn
    let manager = JobManager::new(config.clone());
    let outside_cwd = root.parent().unwrap();
    assert!(
        start_terminal_job(
            &json!({"command": "pwd", "cwd": outside_cwd}),
            config,
            &manager,
        )
        .await
        .is_err(),
        "cwd outside execution root must fail"
    );
    manager.shutdown().await;

    // 5. Symlink traversal cannot escape execution root to unmounted private paths
    let root_symlink = root.join("project-a/escape_to_root");
    let _ = fs::remove_file(&root_symlink);
    std::os::unix::fs::symlink("/root", &root_symlink).unwrap();
    let escape_read = shell(config, &root.join("project-a"), "cat escape_to_root/secret").await;
    assert_ne!(escape_read.exit_code, Some(0));
    let _ = fs::remove_file(&root_symlink);

    // Symlink pointing to /etc cannot be written through
    let etc_symlink = root.join("project-a/escape_to_etc");
    let _ = fs::remove_file(&etc_symlink);
    std::os::unix::fs::symlink("/etc/hosts", &etc_symlink).unwrap();
    let escape_write = shell(
        config,
        &root.join("project-a"),
        "echo 'pwned' >> escape_to_etc",
    )
    .await;
    assert_ne!(escape_write.exit_code, Some(0));
    let _ = fs::remove_file(&etc_symlink);

    // 6. Archive extraction cannot write outside execution root
    let tar_script = "python3 -c \"import tarfile, io; \
b = io.BytesIO(); \
t = tarfile.open(fileobj=b, mode='w'); \
ti = tarfile.TarInfo(name='../../escaped_target.txt'); \
ti.size = 7; \
t.addfile(ti, io.BytesIO(b'escaped')); \
t.close(); \
open('escape.tar', 'wb').write(b.getvalue())\" && tar -xf escape.tar 2>&1 || true";
    let _ = shell(config, &root.join("project-a"), tar_script).await;
    assert!(
        !root.parent().unwrap().join("escaped_target.txt").exists(),
        "archive extraction must not escape execution root"
    );
}
