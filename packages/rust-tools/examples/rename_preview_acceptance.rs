//! Plan 039C PHASE-06 acceptance: proves `code_rename_preview` normalizes
//! both common `WorkspaceEdit` shapes into a bounded, non-mutating preview,
//! and fails closed on every adversarial shape a real language server could
//! plausibly send: an unsupported resource operation, overlapping edits,
//! and a target outside the contained workspace.
//!
//! Uses a small deterministic fake LSP server (same fixture pattern as
//! `lsp_foundation_acceptance`) that advertises `renameProvider: true` and
//! returns a scripted `WorkspaceEdit` selected by the `new_name` argument,
//! since crafting these exact adversarial `WorkspaceEdit` shapes from a
//! real language server is not practical to make deterministic.

use ai_tools::application::code::dispatch_code_tool;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("RENAME_PREVIEW_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("RENAME_PREVIEW_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = fixture_root();
    fs::create_dir_all(root.join("bin")).map_err(io_error)?;
    fs::create_dir_all(root.join("src")).map_err(io_error)?;
    fs::write(root.join("src/a.rs"), "pub fn a() {}\n").map_err(io_error)?;
    fs::write(root.join("src/b.rs"), "pub fn b() {}\n").map_err(io_error)?;
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .map_err(io_error)?;
    require(status.success(), "fixture git init")?;
    install_fake_server(&root)?;

    let mut config = ServerConfig {
        execution_root: Some(path_text(Path::new("/home/farismnrr"))?),
        toolchain_paths: vec![path_text(&root.join("bin"))?],
        lsp_servers: vec!["rust=fake-lsp-rename".into()],
        ..ServerConfig::default()
    };
    config.dir = Some(path_text(&root)?);
    config.validate().map_err(|error| error.to_string())?;
    let manager = LspSessionManager::new(config.clone()).map_err(|error| error.to_string())?;
    let cwd = path_text(&root)?;

    // ---- valid multi-file preview (`changes` form), no mutation ----
    let before_a = fs::read_to_string(root.join("src/a.rs")).map_err(io_error)?;
    let before_b = fs::read_to_string(root.join("src/b.rs")).map_err(io_error)?;
    let multi = call(
        "code_rename_preview",
        json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-multi-file"}),
        &config,
        &manager,
    )
    .await?;
    require(
        multi["applied"] == json!(false),
        "multi-file preview reports applied=false",
    )?;
    let files = multi["files"].as_array().cloned().unwrap_or_default();
    require(files.len() == 2, "multi-file preview covers both files")?;
    require(
        files.iter().any(|f| f["path"] == "src/a.rs")
            && files.iter().any(|f| f["path"] == "src/b.rs"),
        "multi-file preview names both workspace-relative files",
    )?;
    require(
        fs::read_to_string(root.join("src/a.rs")).map_err(io_error)? == before_a
            && fs::read_to_string(root.join("src/b.rs")).map_err(io_error)? == before_b,
        "multi-file preview performed no mutation",
    )?;

    // ---- valid `documentChanges` text-edit form ----
    let doc_changes = call(
        "code_rename_preview",
        json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-doc-changes"}),
        &config,
        &manager,
    )
    .await?;
    let files = doc_changes["files"].as_array().cloned().unwrap_or_default();
    require(
        files.len() == 1 && files[0]["path"] == "src/a.rs",
        "documentChanges text-edit form is normalized into the same preview shape",
    )?;

    // ---- unsupported resource operation: fail closed, whole preview rejected ----
    let resource_op = dispatch_code_tool(
        "code_rename_preview",
        &json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-resource-op"}),
        &config,
        &manager,
    )
    .await;
    require(
        resource_op.is_err(),
        "a documentChanges resource operation (rename/create/delete) is rejected, not silently dropped",
    )?;

    // ---- overlapping edits within one file: rejected ----
    let overlap = dispatch_code_tool(
        "code_rename_preview",
        &json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-overlap"}),
        &config,
        &manager,
    )
    .await;
    require(
        overlap.is_err(),
        "overlapping edits in one file are rejected as ambiguous",
    )?;

    // ---- target outside the contained workspace: rejected ----
    let outside_root = dispatch_code_tool(
        "code_rename_preview",
        &json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-outside-root"}),
        &config,
        &manager,
    )
    .await;
    require(
        outside_root.is_err(),
        "a WorkspaceEdit targeting a path outside the contained workspace is rejected",
    )?;

    // ---- mixed `changes` + `documentChanges`: rejected as ambiguous, no
    // partial preview, hidden resource op in the ignored half never drops
    // silently ----
    let mixed = dispatch_code_tool(
        "code_rename_preview",
        &json!({"cwd": cwd, "path": "src/a.rs", "line": 0, "column": 7, "new_name": "scenario-mixed-both"}),
        &config,
        &manager,
    )
    .await;
    require(
        mixed.is_err(),
        "a WorkspaceEdit carrying both changes and documentChanges is rejected outright",
    )?;

    // ---- confirm no mutation occurred across every rejected/valid scenario ----
    require(
        fs::read_to_string(root.join("src/a.rs")).map_err(io_error)? == before_a
            && fs::read_to_string(root.join("src/b.rs")).map_err(io_error)? == before_b,
        "no scenario mutated any file on disk",
    )?;

    // ---- patch interoperability: the preview itself never mutates, but its
    // edits must translate without semantic loss into the Plan-039B
    // apply_patch unified-diff mutation path. ----
    let edit = &doc_changes["files"][0]["edits"][0];
    let start_char = edit["range"]["start"]["character"].as_u64().unwrap() as usize;
    let end_char = edit["range"]["end"]["character"].as_u64().unwrap() as usize;
    let new_text = edit["new_text"].as_str().unwrap();
    let expected_after = format!(
        "{}{}{}\n",
        &before_a.trim_end_matches('\n')[..start_char],
        new_text,
        &before_a.trim_end_matches('\n')[end_char..],
    );
    let patch = format!(
        "--- a/src/a.rs\n+++ a/src/a.rs\n@@ -1,1 +1,1 @@\n-{}\n+{}\n",
        before_a.trim_end_matches('\n'),
        expected_after.trim_end_matches('\n'),
    );
    let dry_run = ai_tools::application::workspace::apply_patch(
        &json!({"patch": patch, "cwd": cwd, "dry_run": true}),
        &config,
    )
    .map_err(|error| {
        format!("apply_patch dry_run rejected the translated rename preview: {error}")
    })?;
    require(
        dry_run.dry_run,
        "translated patch dry-run reports dry_run=true",
    )?;
    let committed = ai_tools::application::workspace::apply_patch(
        &json!({"patch": patch, "cwd": cwd, "dry_run": false}),
        &config,
    )
    .map_err(|error| {
        format!("apply_patch commit rejected the translated rename preview: {error}")
    })?;
    require(!committed.dry_run, "translated patch commits")?;
    require(
        fs::read_to_string(root.join("src/a.rs")).map_err(io_error)? == expected_after,
        "the preview's edit, translated into a unified diff and applied through apply_patch, \
         produces exactly the renamed content with no semantic loss",
    )?;

    manager.shutdown_all().await;
    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

async fn call(
    name: &str,
    arguments: Value,
    config: &ServerConfig,
    manager: &std::sync::Arc<LspSessionManager>,
) -> Result<Value, String> {
    let result = dispatch_code_tool(name, &arguments, config, manager)
        .await
        .map_err(|error| format!("{name}: {error}"))?
        .ok_or_else(|| format!("{name}: not handled"))?;
    if result.is_error {
        return Err(format!("{name} unexpectedly failed"));
    }
    serde_json::from_str(&result.content[0].text).map_err(|error| error.to_string())
}

fn install_fake_server(root: &Path) -> Result<(), String> {
    let path = root.join("bin/fake-lsp-rename");
    fs::write(&path, FAKE_SERVER).map_err(io_error)?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&path).map_err(io_error)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).map_err(io_error)?;
    }
    Ok(())
}

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    PathBuf::from(format!("/home/farismnrr/.cache/ai-code-rename-lsp-{nonce}"))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "fixture path is not UTF-8".to_string())
}
fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
fn require(value: bool, label: &str) -> Result<(), String> {
    if value {
        Ok(())
    } else {
        Err(format!("assertion failed: {label}"))
    }
}

const FAKE_SERVER: &str = r#"#!/usr/bin/python3
import json, sys

def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line == b"\r\n":
            break
        name, value = line.decode("ascii").split(":", 1)
        headers[name.lower()] = value.strip()
    length = int(headers["content-length"])
    return json.loads(sys.stdin.buffer.read(length).decode("utf-8"))

def send(value):
    payload = json.dumps(value, separators=(",", ":")).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(payload)}\r\n\r\n".encode() + payload)
    sys.stdout.buffer.flush()

def range_at(line, start, end):
    return {"start": {"line": line, "character": start}, "end": {"line": line, "character": end}}

while True:
    message = read_message()
    if message is None:
        sys.exit(0)
    method = message.get("method")
    if method == "initialize":
        root_uri = message["params"]["rootUri"]
        send({"jsonrpc": "2.0", "id": message["id"], "result": {"capabilities": {
            "renameProvider": True,
            "textDocumentSync": {"openClose": True, "change": 1},
        }}})
        send({"jsonrpc": "2.0", "method": "experimental/serverStatus",
              "params": {"quiescent": True, "health": "ok"}})
    elif method == "shutdown":
        send({"jsonrpc": "2.0", "id": message["id"], "result": None})
    elif method == "exit":
        sys.exit(0)
    elif method in ("initialized", "textDocument/didOpen", "textDocument/didChange", "workspace/didChangeConfiguration"):
        pass
    elif method == "textDocument/rename":
        params = message["params"]
        new_name = params["newName"]
        uri = params["textDocument"]["uri"]
        a_uri = uri
        b_uri = uri.replace("src/a.rs", "src/b.rs")
        if new_name == "scenario-multi-file":
            result = {"changes": {
                a_uri: [{"range": range_at(0, 7, 8), "newText": "renamed_a"}],
                b_uri: [{"range": range_at(0, 7, 8), "newText": "renamed_b"}],
            }}
        elif new_name == "scenario-doc-changes":
            result = {"documentChanges": [{
                "textDocument": {"uri": a_uri, "version": 1},
                "edits": [{"range": range_at(0, 7, 8), "newText": "renamed_a"}],
            }]}
        elif new_name == "scenario-resource-op":
            result = {"documentChanges": [{
                "kind": "rename", "oldUri": a_uri, "newUri": a_uri + ".renamed",
            }]}
        elif new_name == "scenario-overlap":
            result = {"changes": {
                a_uri: [
                    {"range": range_at(0, 0, 5), "newText": "x"},
                    {"range": range_at(0, 3, 8), "newText": "y"},
                ]
            }}
        elif new_name == "scenario-mixed-both":
            result = {
                "changes": {a_uri: [{"range": range_at(0, 7, 8), "newText": "visible_only"}]},
                "documentChanges": [{
                    "kind": "rename", "oldUri": b_uri, "newUri": b_uri + ".hidden-op",
                }],
            }
        elif new_name == "scenario-outside-root":
            result = {"changes": {"file:///etc/passwd": [{"range": range_at(0, 0, 1), "newText": "x"}]}}
        else:
            result = {"changes": {a_uri: [{"range": range_at(0, 7, 8), "newText": new_name}]}}
        send({"jsonrpc": "2.0", "id": message["id"], "result": result})
    elif "id" in message:
        send({"jsonrpc": "2.0", "id": message["id"], "error": {"code": -32601, "message": "unknown"}})
"#;
