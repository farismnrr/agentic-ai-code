//! Plan 039C PHASE-07 acceptance: proves the post-edit diagnostic
//! integration contract — after a native workspace mutation (`file_write`,
//! same code path `file_edit`/`apply_patch` use), an already-active LSP
//! session observes the updated document and diagnostics without any
//! server/relay restart.
//!
//! This does not add a new hook framework: the existing document-sync
//! substrate (`lsp/document.rs`) already re-reads the file and sends a
//! versioned `didChange` on every semantic query
//! (`RustLanguageServer::diagnostics` -> `semantic::sync` ->
//! `LspSession::sync_document`). This acceptance proves that contract holds
//! end-to-end through the public `code_diagnostics` MCP tool, across a real
//! native mutation, on the same cached session.

use ai_tools::application::code::dispatch_code_tool;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::application::workspace::file_write;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("POST_EDIT_DIAGNOSTICS_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("POST_EDIT_DIAGNOSTICS_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = fixture_root();
    fs::create_dir_all(root.join("src")).map_err(io_error)?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"post-edit-proof\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .map_err(io_error)?;
    let clean_source = "pub fn value() -> i32 { 1 + 2 }\n";
    let broken_source = "pub fn value() -> i32 { \"not an integer\" }\n";
    fs::write(root.join("src/lib.rs"), clean_source).map_err(io_error)?;
    git(&root, &["init", "-q"])?;

    let toolchain = rust_toolchain_bin()?;
    let mut config = ServerConfig {
        execution_root: Some(path_text(Path::new("/home/farismnrr"))?),
        toolchain_paths: vec![path_text(&toolchain)?],
        lsp_servers: vec!["rust=rust-analyzer".into()],
        ..ServerConfig::default()
    };
    config.dir = Some(path_text(&root)?);
    config.validate().map_err(|error| error.to_string())?;
    let manager = LspSessionManager::new(config.clone()).map_err(|error| error.to_string())?;
    let cwd = path_text(&root)?;

    // (1) create/open the contained fixture, establish one active session.
    let session_before = manager
        .session_for(Some(&cwd), "rust")
        .await
        .map_err(|error| error.to_string())?;
    require(
        manager.active_session_count().await == 1,
        "exactly one active session before any query",
    )?;

    // (2) query diagnostics/semantic result on the clean fixture. Wait for
    // rust-analyzer's project load to settle first (symbols is a cheap,
    // deterministic readiness signal), then take one diagnostics reading as
    // the pre-edit baseline.
    symbols_until(&config, &manager, &cwd, |list| !list.is_empty()).await?;
    let before = diagnostics_until(&config, &manager, &cwd, |_| true).await?;
    require(
        !contains_mismatch(&before),
        "clean fixture reports no type-mismatch diagnostic before the edit",
    )?;

    // (3) mutate through an existing safe native workspace operation
    // (file_write — the same underlying file mutation file_edit/apply_patch
    // perform).
    file_write(
        &json!({"path": "src/lib.rs", "content": broken_source, "cwd": cwd, "overwrite": true}),
        &config,
    )
    .map_err(|error| format!("native file_write failed: {error}"))?;

    // (4)+(5) query diagnostics again through the same active LSP session
    // and observe the fresh, changed diagnostic — bounded polling, not an
    // unbounded wait, and never silently returning the pre-edit result as
    // current.
    let after = diagnostics_until(&config, &manager, &cwd, contains_mismatch).await?;
    require(
        contains_mismatch(&after),
        "post-edit query on the same session observes the new type-mismatch diagnostic",
    )?;
    require(
        before != after,
        "the stale pre-edit diagnostics were not silently returned as current",
    )?;

    // (7) no server/relay restart occurred: the exact same session object
    // (same process) served both the pre- and post-edit queries.
    let session_after = manager
        .session_for(Some(&cwd), "rust")
        .await
        .map_err(|error| error.to_string())?;
    require(
        Arc::ptr_eq(&session_before, &session_after),
        "the same LSP session/process instance served both queries; no restart occurred",
    )?;
    require(
        manager.active_session_count().await == 1,
        "still exactly one active session after the edit (no extra process spawned)",
    )?;

    // (6) restore/cleanup fixture.
    fs::write(root.join("src/lib.rs"), clean_source).map_err(io_error)?;
    manager.shutdown_all().await;
    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

fn contains_mismatch(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|d| {
        let message = d["message"].as_str().unwrap_or("").to_lowercase();
        message.contains("mismatched") || message.contains("expected")
    })
}

async fn symbols_until(
    config: &ServerConfig,
    manager: &Arc<LspSessionManager>,
    cwd: &str,
    ready: impl Fn(&[Value]) -> bool,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        let result = dispatch_code_tool(
            "code_symbols",
            &json!({"cwd": cwd, "path": "src/lib.rs"}),
            config,
            manager,
        )
        .await
        .map_err(|error| error.to_string())?
        .ok_or("code_symbols not handled")?;
        if !result.is_error {
            let value: Value =
                serde_json::from_str(&result.content[0].text).map_err(|error| error.to_string())?;
            let list = value["symbols"].as_array().cloned().unwrap_or_default();
            if ready(&list) {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
}

async fn diagnostics_until(
    config: &ServerConfig,
    manager: &Arc<LspSessionManager>,
    cwd: &str,
    ready: impl Fn(&[Value]) -> bool,
) -> Result<Vec<Value>, String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    let mut last_error = String::new();
    loop {
        let outcome = dispatch_code_tool(
            "code_diagnostics",
            &json!({"cwd": cwd, "path": "src/lib.rs"}),
            config,
            manager,
        )
        .await;
        match outcome {
            Ok(Some(result)) if !result.is_error => {
                let value: Value = serde_json::from_str(&result.content[0].text)
                    .map_err(|error| error.to_string())?;
                let list = value["diagnostics"].as_array().cloned().unwrap_or_default();
                if ready(&list) {
                    return Ok(list);
                }
            }
            Ok(Some(_)) => last_error = "code_diagnostics returned an error result".into(),
            Ok(None) => return Err("code_diagnostics not handled".into()),
            Err(error) => last_error = error.to_string(),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "code_diagnostics did not settle in time: {last_error}"
            ));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
}

fn rust_toolchain_bin() -> Result<PathBuf, String> {
    let candidates = [
        "/home/farismnrr/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin",
        "/home/farismnrr/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.join("rust-analyzer").is_file())
        .ok_or_else(|| "standalone rust-analyzer toolchain bin not found".into())
}

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(format!(
        "/home/farismnrr/.cache/ai-code-post-edit-lsp-{nonce}"
    ))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "non-utf8 fixture path".into())
}

fn git(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(io_error)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {:?} failed", args))
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
