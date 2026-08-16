use relay_core::workspace_path::{
    resolve_contained_cwd, resolve_existing_path, resolve_write_target, EntryKind,
};
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn expect_ok<T>(label: &str, result: Result<T, relay_core::error::McpError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{label}: expected success, got {error}"),
    }
}

fn expect_err<T>(label: &str, result: Result<T, relay_core::error::McpError>) {
    let _ = expect_error(label, result);
}

fn expect_error<T>(
    label: &str,
    result: Result<T, relay_core::error::McpError>,
) -> relay_core::error::McpError {
    match result {
        Ok(_) => panic!("{label}: expected rejection"),
        Err(error) => error,
    }
}

fn main() {
    #[cfg(not(unix))]
    panic!("workspace path acceptance requires Unix symlink semantics");

    #[cfg(unix)]
    run();
}

#[cfg(unix)]
fn run() {
    let fixture = Fixture::new();
    let root = &fixture.root;
    let external = &fixture.external;

    fs::create_dir_all(root.join("src/nested")).expect("create contained directories");
    fs::write(root.join("src/lib.rs"), b"contained\n").expect("create contained file");
    fs::create_dir_all(external.join("dir")).expect("create external directory");
    fs::write(external.join("secret.txt"), b"external\n").expect("create external file");

    symlink(external.join("secret.txt"), root.join("external-file-link"))
        .expect("create external file symlink");
    symlink(external.join("dir"), root.join("external-dir-link"))
        .expect("create external directory symlink");
    symlink("loop-b", root.join("loop-a")).expect("create loop a");
    symlink("loop-a", root.join("loop-b")).expect("create loop b");
    symlink(root.join("src/lib.rs"), root.join("contained-file-link"))
        .expect("create contained file symlink");
    symlink(root.join("src"), root.join("contained-dir-link"))
        .expect("create contained directory symlink");

    let canonical_root = fs::canonicalize(root).expect("canonical root");
    let canonical_file = fs::canonicalize(root.join("src/lib.rs")).expect("canonical file");

    let cwd = expect_ok("default cwd", resolve_contained_cwd(root, None));
    assert_eq!(cwd, canonical_root);

    let nested_cwd = expect_ok(
        "relative cwd",
        resolve_contained_cwd(root, Some("src/nested")),
    );
    assert!(nested_cwd.starts_with(&canonical_root));

    let absolute_cwd = expect_ok(
        "absolute contained cwd",
        resolve_contained_cwd(root, Some(root.join("src").to_str().expect("utf8 fixture"))),
    );
    assert_eq!(absolute_cwd, canonical_root.join("src"));

    let dotted_cwd = expect_ok(
        "contained dotdot cwd",
        resolve_contained_cwd(root, Some("src/nested/..")),
    );
    assert_eq!(dotted_cwd, canonical_root.join("src"));

    let relative = expect_ok(
        "relative file",
        resolve_existing_path(root, Some("src/nested"), "../lib.rs", EntryKind::File),
    );
    assert_eq!(relative, canonical_file);

    let dotted = expect_ok(
        "dot path",
        resolve_existing_path(root, None, "./src/./lib.rs", EntryKind::File),
    );
    assert_eq!(dotted, canonical_file);

    let absolute_inside = expect_ok(
        "absolute contained file",
        resolve_existing_path(
            root,
            None,
            root.join("src/lib.rs").to_str().expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    assert_eq!(absolute_inside, canonical_file);

    expect_err(
        "nested traversal escape",
        resolve_existing_path(root, Some("src/nested"), "../../../outside", EntryKind::Any),
    );
    expect_err(
        "absolute external file",
        resolve_existing_path(
            root,
            None,
            external.join("secret.txt").to_str().expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    expect_err(
        "external file symlink",
        resolve_existing_path(root, None, "external-file-link", EntryKind::File),
    );
    expect_err(
        "external directory symlink",
        resolve_existing_path(root, None, "external-dir-link", EntryKind::Directory),
    );
    expect_err(
        "external symlink cwd",
        resolve_contained_cwd(root, Some("external-dir-link")),
    );
    expect_err(
        "missing cwd",
        resolve_contained_cwd(root, Some("missing-cwd")),
    );
    expect_err("file cwd", resolve_contained_cwd(root, Some("src/lib.rs")));

    let contained_symlink_read = expect_ok(
        "contained file symlink read",
        resolve_existing_path(root, None, "contained-file-link", EntryKind::File),
    );
    assert_eq!(contained_symlink_read, canonical_file);

    let contained_symlink_cwd = expect_ok(
        "contained directory symlink cwd",
        resolve_contained_cwd(root, Some("contained-dir-link")),
    );
    assert_eq!(contained_symlink_cwd, canonical_root.join("src"));
    expect_err(
        "symlink loop",
        resolve_existing_path(root, None, "loop-a", EntryKind::Any),
    );
    expect_err(
        "missing read",
        resolve_existing_path(root, None, "src/missing.rs", EntryKind::File),
    );
    expect_err(
        "absolute missing read",
        resolve_existing_path(
            root,
            None,
            root.join("src/missing-absolute.rs")
                .to_str()
                .expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    expect_err(
        "file as directory",
        resolve_existing_path(root, None, "src/lib.rs", EntryKind::Directory),
    );
    expect_err(
        "directory as file",
        resolve_existing_path(root, None, "src", EntryKind::File),
    );

    let new_target = expect_ok(
        "new write target",
        resolve_write_target(root, Some("src"), "created.rs", EntryKind::File),
    );
    assert_eq!(new_target, canonical_root.join("src/created.rs"));
    assert!(!new_target.exists());

    let contained_traversal = expect_ok(
        "contained write traversal",
        resolve_write_target(root, Some("src/nested"), "../new.rs", EntryKind::File),
    );
    assert_eq!(contained_traversal, canonical_root.join("src/new.rs"));

    let absolute_new_target = expect_ok(
        "absolute contained write",
        resolve_write_target(
            root,
            None,
            root.join("src/absolute-new.rs")
                .to_str()
                .expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    assert_eq!(
        absolute_new_target,
        canonical_root.join("src/absolute-new.rs")
    );

    let contained_symlink_parent = expect_ok(
        "contained symlink write parent",
        resolve_write_target(
            root,
            None,
            "contained-dir-link/through-parent.rs",
            EntryKind::File,
        ),
    );
    assert_eq!(
        contained_symlink_parent,
        canonical_root.join("src/through-parent.rs")
    );

    expect_err(
        "escaping write traversal",
        resolve_write_target(root, Some("src"), "../../external/new.txt", EntryKind::File),
    );
    expect_err(
        "missing write parent",
        resolve_write_target(root, None, "missing-parent/new.rs", EntryKind::File),
    );
    expect_err(
        "external absolute write",
        resolve_write_target(
            root,
            None,
            external.join("new.txt").to_str().expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    expect_err(
        "write through external symlink parent",
        resolve_write_target(root, None, "external-dir-link/new.txt", EntryKind::File),
    );
    expect_err(
        "write to external symlink final",
        resolve_write_target(root, None, "external-file-link", EntryKind::File),
    );
    expect_err(
        "write to contained symlink final",
        resolve_write_target(root, None, "contained-file-link", EntryKind::File),
    );
    expect_err(
        "existing directory as write file",
        resolve_write_target(root, None, "src", EntryKind::File),
    );

    let existing_file = expect_ok(
        "existing write file",
        resolve_write_target(root, None, "src/lib.rs", EntryKind::File),
    );
    assert_eq!(existing_file, canonical_file);

    let traversal_error = expect_error(
        "error confidentiality",
        resolve_existing_path(
            root,
            None,
            external
                .join("CANARY-PATH-038.txt")
                .to_str()
                .expect("utf8 fixture"),
            EntryKind::File,
        ),
    );
    assert!(!traversal_error.to_string().contains("CANARY-PATH-038"));
    assert_eq!(traversal_error.message(), "Invalid request");
    assert!(traversal_error.data().is_none());

    let terminal_relative = expect_ok(
        "terminal relative cwd compatibility",
        relay_core::terminal_policy::resolve_contained_cwd(root, Some("src")),
    );
    assert_eq!(terminal_relative, canonical_root.join("src"));
    let terminal_escape = expect_error(
        "terminal traversal compatibility",
        relay_core::terminal_policy::resolve_contained_cwd(root, Some("../external")),
    );
    assert_eq!(
        terminal_escape.to_string(),
        "Invalid Request: path traversal outside execution root is forbidden"
    );

    assert_eq!(
        fs::read(external.join("secret.txt")).expect("external sentinel remains readable"),
        b"external\n"
    );
    assert!(!external.join("dir/new.txt").exists());

    println!("workspace path security acceptance: PASS");
}

#[cfg(unix)]
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    external: PathBuf,
}

#[cfg(unix)]
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "relay-workspace-path-check-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let root = base.join("root");
        let external = base.join("external");
        fs::create_dir_all(&root).expect("create root fixture");
        fs::create_dir_all(&external).expect("create external fixture");
        Self {
            base,
            root,
            external,
        }
    }
}

#[cfg(unix)]
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[cfg(unix)]
fn unique_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos()
}
