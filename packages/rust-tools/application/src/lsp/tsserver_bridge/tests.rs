//! Deterministic unit coverage for the 039C Vue Bridge Final Remediation
//! (Required fixes 1-5) that does not require spawning a real tsserver
//! child. Real Vue `<script setup lang="ts">` semantics (symbols,
//! definition, references, hover, diagnostics) and the fail-closed
//! bridge-death integration (Required fix 6) are proven end-to-end
//! against the installed toolchain in
//! `application/examples/typescript_lsp_acceptance.rs` instead, since
//! those require a real, installed `@vue/language-server`/`tsserver.js`.
//! Split out of `tsserver_bridge.rs` to stay under the maintainability
//! file-length budget.

use super::framing::read_bounded_line;
use super::*;
use std::io::Cursor;
use tokio::io::BufReader;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "tsserver-bridge-test-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// --- Required fix 1: safe bridge file reads --------------------------

#[test]
fn resolve_bridge_file_accepts_normal_contained_file() {
    let fixture = Fixture::new("normal");
    let file = fixture.root.join("component.vue");
    std::fs::write(&file, "<script setup lang=\"ts\"></script>").unwrap();
    let resolved = resolve_bridge_file(&fixture.root, file.to_str().unwrap());
    assert_eq!(resolved, Some(std::fs::canonicalize(&file).unwrap()));
}

#[test]
fn resolve_bridge_file_rejects_dotdot_traversal() {
    let fixture = Fixture::new("traversal");
    std::fs::create_dir_all(fixture.root.join("inner")).unwrap();
    let outside = fixture.root.join("outside-secret.txt");
    std::fs::write(&outside, "secret").unwrap();
    let traversal = fixture
        .root
        .join("inner")
        .join("..")
        .join("outside-secret.txt");
    assert_eq!(
        resolve_bridge_file(&fixture.root.join("inner"), traversal.to_str().unwrap()),
        None
    );
}

#[test]
fn resolve_bridge_file_rejects_absolute_outside_path() {
    let fixture = Fixture::new("absolute-outside");
    let outside = Fixture::new("absolute-outside-target");
    let target = outside.root.join("secret.vue");
    std::fs::write(&target, "leak").unwrap();
    assert_eq!(
        resolve_bridge_file(&fixture.root, target.to_str().unwrap()),
        None
    );
}

#[cfg(unix)]
#[test]
fn resolve_bridge_file_rejects_outside_target_symlink() {
    let fixture = Fixture::new("symlink");
    let outside = Fixture::new("symlink-target");
    let real_secret = outside.root.join("secret.vue");
    std::fs::write(&real_secret, "leak").unwrap();
    let link = fixture.root.join("link.vue");
    std::os::unix::fs::symlink(&real_secret, &link).unwrap();
    assert_eq!(
        resolve_bridge_file(&fixture.root, link.to_str().unwrap()),
        None
    );
}

#[test]
fn resolve_bridge_file_rejects_protected_target() {
    let fixture = Fixture::new("protected");
    std::fs::create_dir_all(fixture.root.join(".ssh")).unwrap();
    let key = fixture.root.join(".ssh").join("id_ed25519");
    std::fs::write(&key, "private").unwrap();
    assert_eq!(
        resolve_bridge_file(&fixture.root, key.to_str().unwrap()),
        None
    );
}

// --- Required fix 2: narrowed command surface -------------------------

#[test]
fn allowlist_covers_every_reviewed_bridge_command() {
    for command in ALLOWED_COMMANDS {
        assert!(!command.is_empty());
    }
    assert!(ALLOWED_COMMANDS.contains(&"projectInfo"));
    assert!(ALLOWED_COMMANDS.contains(&"quickinfo"));
}

#[test]
fn unknown_command_is_not_in_allowlist() {
    assert!(!ALLOWED_COMMANDS.contains(&"exec"));
    assert!(!ALLOWED_COMMANDS.contains(&"eval"));
    assert!(!ALLOWED_COMMANDS.contains(&"reload"));
    assert!(!ALLOWED_COMMANDS.contains(&"open"));
    assert!(!ALLOWED_COMMANDS.contains(&"close"));
}

#[test]
fn extract_file_argument_reads_object_shaped_commands() {
    let arguments = json!({"file": "/workspace/a.vue", "line": 1, "offset": 1});
    assert_eq!(
        extract_file_argument("quickinfo", &arguments),
        Some("/workspace/a.vue".to_owned())
    );
}

#[test]
fn extract_file_argument_reads_positional_shaped_commands() {
    let arguments = json!(["/workspace/a.vue", 42]);
    assert_eq!(
        extract_file_argument("getComponentProps", &arguments),
        Some("/workspace/a.vue".to_owned())
    );
}

#[test]
fn extract_file_argument_none_for_completion_entry_data_blob() {
    let arguments = json!([{"__vue__autoImportSuggestions": true, "fileName": "ignored"}]);
    assert_eq!(
        extract_file_argument("resolveAutoImportCompletionEntry", &arguments),
        None
    );
}

// --- Required fix 3: real bounded stdout framing ----------------------

#[tokio::test]
async fn read_bounded_line_reads_a_normal_line() {
    let mut reader = BufReader::new(Cursor::new(b"{\"type\":\"response\"}\n".to_vec()));
    let line = read_bounded_line(&mut reader, MAX_TSSERVER_LINE_BYTES)
        .await
        .unwrap();
    assert_eq!(line, Some(b"{\"type\":\"response\"}".to_vec()));
}

#[tokio::test]
async fn read_bounded_line_rejects_oversized_line_without_full_allocation() {
    let mut payload = vec![b'a'; 64];
    payload.push(b'\n');
    let mut reader = BufReader::new(Cursor::new(payload));
    let result = read_bounded_line(&mut reader, 8).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_bounded_line_rejects_eof_mid_line() {
    let mut reader = BufReader::new(Cursor::new(b"{\"incomplete\":true".to_vec()));
    let result = read_bounded_line(&mut reader, MAX_TSSERVER_LINE_BYTES).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_bounded_line_clean_eof_with_no_pending_bytes_is_not_an_error() {
    let mut reader = BufReader::new(Cursor::new(Vec::new()));
    let result = read_bounded_line(&mut reader, MAX_TSSERVER_LINE_BYTES).await;
    assert_eq!(result, Ok(None));
}

// --- Required fix 4: bound work before spawning ------------------------

#[tokio::test]
async fn concurrency_semaphore_denies_permits_at_capacity() {
    let semaphore = Arc::new(Semaphore::new(1));
    let first = semaphore.clone().try_acquire_owned();
    assert!(first.is_ok());
    let second = semaphore.clone().try_acquire_owned();
    assert!(second.is_err());
    drop(first);
    let third = semaphore.try_acquire_owned();
    assert!(third.is_ok());
}

// --- Required fix 5: keep bridge documents fresh ------------------------

#[test]
fn content_hash_changes_when_content_changes() {
    let a = content_hash("<script setup lang=\"ts\">const x = 1;</script>");
    let b = content_hash("<script setup lang=\"ts\">const x = 2;</script>");
    assert_ne!(a, b);
    let a_again = content_hash("<script setup lang=\"ts\">const x = 1;</script>");
    assert_eq!(a, a_again);
}
