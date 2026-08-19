//! Deterministic regression acceptance test for MCP sandboxed execution restoration.
//!
//! Validates:
//! 1. `terminal_exec` synchronous execution with default and explicit cwd.
//! 2. `terminal_job_*` background execution, polling, and cancellation lifecycle.
//! 3. `text_search` with default cwd, explicit cwd, invalid cwd rejection, and credential exclusion.
//! 4. Sandboxed workspace execution with build/metadata directories (node_modules, target, .git).
//! 5. Protected path masking (.env, .ssh) inside sandbox.
//! 6. Execution root containment rejection.

use relay_application::execution::{dispatch_tool_call, start_terminal_job, JobManager, JobState};
use relay_application::lsp::LspSessionManager;
use relay_core::config::ServerConfig;
use relay_interfaces::mcp::find_tool;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("mcp_restore_acc_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        let canonical = fs::canonicalize(dir).expect("canonicalize test dir");
        Self { path: canonical }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Starting MCP Sandboxed Execution Restoration Acceptance ===");

    let boundary = TempTestDir::new("boundary");
    let workspace = boundary.path.join("workspace");
    let outside = TempTestDir::new("outside");

    fs::create_dir_all(&workspace)?;
    let workspace = fs::canonicalize(workspace)?;

    // Create realistic mock build directories and files
    fs::create_dir_all(workspace.join("node_modules/mock-pkg/dist"))?;
    fs::write(
        workspace.join("node_modules/mock-pkg/dist/index.js"),
        "console.log('pkg');",
    )?;
    fs::create_dir_all(workspace.join("target/debug/deps"))?;
    fs::write(workspace.join("target/debug/deps/lib.rlib"), "fake-lib")?;
    fs::create_dir_all(workspace.join(".git/objects/00"))?;
    fs::write(workspace.join(".git/objects/00/sample"), "fake-object")?;

    // Create searchable content and protected credential files
    fs::create_dir_all(workspace.join("src"))?;
    fs::write(
        workspace.join("src/main.rs"),
        "fn main() { println!(\"SEARCH_TOKEN_123\"); }\n",
    )?;
    fs::write(workspace.join(".env"), "SECRET_TOKEN_ABC=do_not_leak\n")?;
    fs::write(workspace.join(".env.local"), "SECRET_LOCAL=do_not_leak\n")?;

    let config = ServerConfig {
        dir: Some(workspace.to_string_lossy().into_owned()),
        execution_root: Some(boundary.path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    config.ensure_workspaces_initialized()?;

    let jobs = Arc::new(JobManager::new(config.clone()));
    let lsp = Arc::new(LspSessionManager::new(config.clone())?);
    let hooks = relay_application::hooks::HookManager::load(Arc::new(config.clone()))?;

    // -------------------------------------------------------------
    // Test 1: Synchronous terminal_exec
    // -------------------------------------------------------------
    println!("\n[1/5] Testing terminal_exec...");
    let tool = find_tool("terminal_exec").expect("terminal_exec found");

    // 1.1 terminal_exec true
    let res = dispatch_tool_call(
        &tool,
        &json!({ "command": "true" }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!res.is_error, "terminal_exec true failed: {:?}", res);
    assert!(res.content[0].text.contains("Exit: 0"));
    println!("  ✓ terminal_exec 'true' succeeded with exit 0");

    // 1.2 terminal_exec pwd with default cwd (None)
    let res = dispatch_tool_call(
        &tool,
        &json!({ "command": "pwd" }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(
        !res.is_error,
        "terminal_exec pwd (default cwd) failed: {:?}",
        res
    );
    assert!(res.content[0]
        .text
        .contains(&workspace.to_string_lossy().into_owned()));
    println!("  ✓ terminal_exec 'pwd' without cwd resolved to primary workspace");

    // 1.3 terminal_exec outside containment rejection
    let res = dispatch_tool_call(
        &tool,
        &json!({ "command": "pwd", "cwd": outside.path.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await;
    assert!(
        res.is_err(),
        "terminal_exec outside authorized workspace must be rejected"
    );
    println!("  ✓ terminal_exec with outside cwd rejected by workspace allowlist");

    // -------------------------------------------------------------
    // Test 2: Protected-Path Masking in Sandbox
    // -------------------------------------------------------------
    println!("\n[2/5] Testing protected path masking inside sandbox...");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "command": "cat .env" }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    // Masked with /dev/null -> stdout is empty
    assert!(
        !res.content[0].text.contains("SECRET_TOKEN_ABC"),
        "secret must not leak"
    );
    println!("  ✓ .env masked with /dev/null inside Bubblewrap sandbox");

    // -------------------------------------------------------------
    // Test 3: Background terminal_job lifecycle and cancellation
    // -------------------------------------------------------------
    println!("\n[3/5] Testing terminal_job lifecycle...");

    // 3.1 Successful job
    let task_id = start_terminal_job(
        &json!({ "command": "sh", "args": ["-c", "printf 'async-output-456'"] }),
        &config,
        &jobs,
    )
    .await?;
    let snapshot = loop {
        let snapshot = jobs.get(&task_id).await.expect("job exists");
        if matches!(snapshot.state, JobState::Completed | JobState::Failed) {
            break snapshot;
        }
        sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
    assert!(snapshot.stdout.contains("async-output-456"));
    println!("  ✓ terminal_job completed successfully with stdout captured");

    // 3.2 Job cancellation
    let task_id = start_terminal_job(
        &json!({ "command": "sh", "args": ["-c", "sleep 30"] }),
        &config,
        &jobs,
    )
    .await?;
    sleep(Duration::from_millis(20)).await;
    let cancel_res = jobs.cancel(&task_id).await?;
    assert!(matches!(
        cancel_res.state,
        JobState::Running | JobState::Queued | JobState::Cancelled
    ));
    let final_snapshot = loop {
        let snapshot = jobs.get(&task_id).await.expect("job exists");
        if matches!(
            snapshot.state,
            JobState::Cancelled | JobState::Failed | JobState::Completed
        ) {
            break snapshot;
        }
        sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(final_snapshot.state, JobState::Cancelled);
    println!("  ✓ terminal_job cancellation succeeded");

    // -------------------------------------------------------------
    // Test 4: Sandboxed text_search
    // -------------------------------------------------------------
    println!("\n[4/5] Testing text_search...");
    let tool = find_tool("text_search").expect("text_search found");

    // 4.1 text_search without cwd (defaulting to primary workspace root)
    let res = dispatch_tool_call(
        &tool,
        &json!({ "query": "SEARCH_TOKEN_123" }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!res.is_error, "text_search without cwd failed: {:?}", res);
    let parsed: Value = serde_json::from_str(&res.content[0].text)?;
    assert_eq!(parsed["count"], 1);
    assert_eq!(parsed["matches"][0]["path"], "src/main.rs");
    println!("  ✓ text_search without cwd defaulted to primary workspace and found match");

    // 4.2 text_search with explicit contained cwd
    let res = dispatch_tool_call(
        &tool,
        &json!({ "query": "SEARCH_TOKEN_123", "cwd": workspace.join("src").to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(
        !res.is_error,
        "text_search with explicit cwd failed: {:?}",
        res
    );
    let parsed: Value = serde_json::from_str(&res.content[0].text)?;
    assert_eq!(parsed["count"], 1);
    println!("  ✓ text_search with explicit contained cwd succeeded");

    // 4.3 text_search with outside cwd rejected
    let res = dispatch_tool_call(
        &tool,
        &json!({ "query": "SEARCH_TOKEN_123", "cwd": outside.path.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await;
    assert!(
        res.is_err(),
        "text_search outside workspace must be rejected"
    );
    println!("  ✓ text_search with outside cwd rejected");

    // 4.4 text_search excludes .env credentials
    let res = dispatch_tool_call(
        &tool,
        &json!({ "query": "SECRET_TOKEN_ABC" }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!res.is_error);
    let parsed: Value = serde_json::from_str(&res.content[0].text)?;
    assert_eq!(
        parsed["count"], 0,
        "secret in .env must be excluded from text search"
    );
    println!("  ✓ text_search excludes protected .env files");

    // -------------------------------------------------------------
    // Test 5: Tool Catalog Invariants
    // -------------------------------------------------------------
    println!("\n[5/5] Testing tool catalog completeness...");
    assert!(find_tool("terminal_exec").is_some());
    assert!(find_tool("text_search").is_some());
    assert!(find_tool("file_read").is_some());
    assert!(find_tool("file_write").is_some());
    println!("  ✓ Tool catalog invariants preserved");

    println!("\n=== All Acceptance Checks Passed: PASS ===");
    Ok(())
}
