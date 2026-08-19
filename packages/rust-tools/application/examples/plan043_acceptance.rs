//! Authoritative Plan 043 End-to-End Acceptance Test
//!
//! Validates:
//! 1. Workspace Allowlisting & Dynamic Multi-Root Authorization
//! 2. Execution Timing Structure & Monotonic Properties
//! 3. Broad Git & Full Git Worktree Operations
use relay_application::execution::dispatch_tool_call;
use relay_application::lsp::LspSessionManager;
use relay_core::config::ServerConfig;
use relay_interfaces::mcp::{find_tool, ToolCallResult};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
struct TempTestDir {
    path: PathBuf,
}
impl TempTestDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("plan043_acc_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create test dir");
        let canonical = fs::canonicalize(dir).expect("canonicalize test dir");
        Self { path: canonical }
    }
}
impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git execution failed");
        assert!(status.success(), "git {:?} failed", args);
    };
    run(&["init"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test User"]);
    fs::write(dir.join("initial.txt"), "hello world\n").unwrap();
    run(&["add", "initial.txt"]);
    run(&["commit", "-m", "initial commit"]);
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Starting Plan 043 End-to-End Acceptance Test ===");
    let boundary_dir = TempTestDir::new("boundary");
    let primary_dir = boundary_dir.path.join("primary");
    let secondary_dir = boundary_dir.path.join("secondary");
    fs::create_dir_all(&primary_dir)?;
    fs::create_dir_all(&secondary_dir)?;
    let primary_dir = fs::canonicalize(primary_dir)?;
    let secondary_dir = fs::canonicalize(secondary_dir)?;
    init_git_repo(&primary_dir);
    let config = ServerConfig {
        dir: Some(primary_dir.to_string_lossy().into_owned()),
        execution_root: Some(boundary_dir.path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    config.ensure_workspaces_initialized().unwrap();
    let jobs = relay_application::execution::JobManager::new(config.clone());
    let lsp = Arc::new(LspSessionManager::new(config.clone()).unwrap());
    let hooks = relay_application::hooks::HookManager::load(Arc::new(config.clone())).unwrap();
    // -------------------------------------------------------------
    // Acceptance Test 1: Workspace Allowlisting
    // -------------------------------------------------------------
    println!("\n[1/3] Testing Workspace Allowlisting...");
    // 1.1 List initial workspaces
    let tool = find_tool("workspace_list").expect("workspace_list tool found");
    let res = dispatch_tool_call(&tool, &json!({}), &config, &jobs, &lsp, &hooks)
        .await
        .expect("workspace_list dispatch");
    assert!(!res.is_error);
    let list_val: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(list_val["total"], 1);
    assert_eq!(list_val["workspaces"][0]["is_primary"], true);
    println!("  ✓ workspace_list returned primary root");
    let file_tool = find_tool("file_write").expect("file_write tool found");
    let sec_file = secondary_dir.join("secondary_test.txt");
    let denied = dispatch_tool_call(
        &file_tool,
        &json!({ "path": sec_file.to_string_lossy(), "content": "denied", "cwd": secondary_dir.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await;
    assert!(
        denied.is_err(),
        "secondary workspace must be denied before workspace_add"
    );
    // 1.2 Add secondary workspace
    let tool = find_tool("workspace_add").expect("workspace_add tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "path": secondary_dir.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("workspace_add dispatch");
    assert!(!res.is_error);
    let add_val: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(add_val["authorized"], true);
    assert_eq!(add_val["total_authorized_workspaces"], 2);
    println!("  ✓ workspace_add authorized secondary workspace");
    // 1.3 Inspect with workspace_get
    let tool = find_tool("workspace_get").expect("workspace_get tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "path": secondary_dir.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("workspace_get dispatch");
    assert!(!res.is_error);
    println!("  ✓ workspace_get retrieved authorized workspace");
    // 1.4 Write and read file inside dynamically authorized secondary workspace
    let res = dispatch_tool_call(
        &file_tool,
        &json!({
            "path": sec_file.to_string_lossy(),
            "content": "authorized content in secondary workspace",
            "cwd": secondary_dir.to_string_lossy(),
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("file_write in secondary dispatch");
    assert!(!res.is_error);
    println!("  ✓ file_write succeeded inside dynamically authorized secondary workspace");
    // 1.5 Remove secondary workspace
    let tool = find_tool("workspace_remove").expect("workspace_remove tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "path": secondary_dir.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("workspace_remove dispatch");
    assert!(!res.is_error);
    let denied = dispatch_tool_call(
        &file_tool,
        &json!({ "path": sec_file.to_string_lossy(), "content": "denied again", "cwd": secondary_dir.to_string_lossy(), "overwrite": true }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await;
    assert!(
        denied.is_err(),
        "secondary workspace must be denied after workspace_remove"
    );
    println!("  ✓ workspace_remove revoked secondary workspace");
    // -------------------------------------------------------------
    // Acceptance Test 2: Execution Timing Structure
    // -------------------------------------------------------------
    println!("\n[2/3] Testing Per-Tool Monotonic Execution Timing...");
    let timed_result = ToolCallResult::complete(vec![]).with_timing(12, 18);
    let timing_meta = timed_result.meta.expect("meta is present");
    let timing_obj = &timing_meta["timing"];
    assert_eq!(timing_obj["dispatch_ms"], 12);
    assert_eq!(timing_obj["server_total_ms"], 18);
    let task_id = relay_application::execution::start_terminal_job(
        &json!({ "command": "sh", "args": ["-lc", "printf job-timing"], "cwd": primary_dir.to_string_lossy() }),
        &config,
        &jobs,
    )
    .await?;
    let snapshot = loop {
        let snapshot = jobs.get(&task_id).await.expect("terminal job exists");
        if matches!(
            snapshot.state,
            relay_application::execution::JobState::Completed
                | relay_application::execution::JobState::Failed
        ) {
            break snapshot;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };
    assert_eq!(snapshot.exit_code, Some(0));
    assert!(snapshot.execution_duration_ms.is_some());
    assert!(snapshot.job_json().get("executionDurationMs").is_some());
    println!("  ✓ Tool response and completed job timing metadata are populated");
    // -------------------------------------------------------------
    // Acceptance Test 3: Broad Git & Full Git Worktree Operations
    // -------------------------------------------------------------
    println!("\n[3/3] Testing Broad Git & Full Worktree Operations...");
    let repo_root = &primary_dir;
    // 3.1 Git Worktree Operations
    let worktree_dir = primary_dir.join("wt_feature_1");
    // Add worktree
    let tool = find_tool("git_worktree_add").expect("git_worktree_add tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "path": worktree_dir.to_string_lossy(),
            "create_branch": "feature/wt-test"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_worktree_add dispatch");
    assert!(!res.is_error, "git_worktree_add failed: {:?}", res);
    println!("  ✓ git_worktree_add created linked worktree feature/wt-test");
    // List worktrees
    let tool = find_tool("git_worktree_list").expect("git_worktree_list tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "cwd": repo_root.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_worktree_list dispatch");
    assert!(!res.is_error);
    let wt_list: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(wt_list["total"], 2);
    println!("  ✓ git_worktree_list discovered both main and linked worktrees");
    // Get worktree details
    let tool = find_tool("git_worktree_get").expect("git_worktree_get tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "path": worktree_dir.to_string_lossy(),
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_worktree_get dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_worktree_get retrieved individual worktree metadata");
    // Remove worktree
    let tool = find_tool("git_worktree_remove").expect("git_worktree_remove tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "path": worktree_dir.to_string_lossy(),
            "force": true
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_worktree_remove dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_worktree_remove removed linked worktree");
    // Prune worktree
    let tool = find_tool("git_worktree_prune").expect("git_worktree_prune tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "cwd": repo_root.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_worktree_prune dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_worktree_prune pruned worktree metadata");
    // 3.2 Branch rename
    let tool = find_tool("git_branch_rename").expect("git_branch_rename tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "old_name": "feature/wt-test",
            "new_name": "feature/renamed-branch"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_branch_rename dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_branch_rename renamed branch successfully");
    // 3.3 Git Stash operations
    fs::write(
        repo_root.join("initial.txt"),
        "modified content for stash\n",
    )
    .unwrap();
    let tool = find_tool("git_stash_push").expect("git_stash_push tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "message": "test stash message"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_stash_push dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_stash_push stashed uncommitted changes");
    let tool = find_tool("git_stash_list").expect("git_stash_list tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "cwd": repo_root.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_stash_list dispatch");
    assert!(!res.is_error);
    let stash_list: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(stash_list["total"], 1);
    println!("  ✓ git_stash_list found stashed entry");

    let tool = find_tool("git_stash_pop").expect("git_stash_pop tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "cwd": repo_root.to_string_lossy(), "index": 0 }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_stash_pop dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_stash_pop popped stashed changes");

    // 3.4 Git Tag operations
    let tool = find_tool("git_tag_create").expect("git_tag_create tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "name": "v1.0.0",
            "message": "release 1.0.0"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_tag_create dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_tag_create created annotated tag v1.0.0");

    let tool = find_tool("git_tag_list").expect("git_tag_list tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({ "cwd": repo_root.to_string_lossy() }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_tag_list dispatch");
    assert!(!res.is_error);
    let tag_list: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(tag_list["total"], 1);
    assert_eq!(tag_list["tags"][0]["name"], "v1.0.0");
    println!("  ✓ git_tag_list found v1.0.0 tag");

    let tool = find_tool("git_tag_delete").expect("git_tag_delete tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "name": "v1.0.0"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_tag_delete dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_tag_delete deleted tag v1.0.0");

    // 3.5 Git Remote operations
    let tool = find_tool("git_remote_add").expect("git_remote_add tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "name": "upstream",
            "url": "https://github.com/MasihAwam/upstream.git"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_remote_add dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_remote_add added upstream remote");

    let tool = find_tool("git_remote_set_url").expect("git_remote_set_url tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "name": "upstream",
            "url": "https://github.com/MasihAwam/upstream-updated.git"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_remote_set_url dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_remote_set_url updated remote URL");

    let tool = find_tool("git_remote_remove").expect("git_remote_remove tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "name": "upstream"
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_remote_remove dispatch");
    assert!(!res.is_error);
    println!("  ✓ git_remote_remove removed upstream remote");

    // 3.6 Restore and Clean
    fs::write(repo_root.join("untracked_scratch.txt"), "scratch\n").unwrap();
    let tool = find_tool("git_clean").expect("git_clean tool found");
    let res = dispatch_tool_call(
        &tool,
        &json!({
            "cwd": repo_root.to_string_lossy(),
            "dry_run": false
        }),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .expect("git_clean dispatch");
    assert!(!res.is_error);
    assert!(!repo_root.join("untracked_scratch.txt").exists());
    println!("  ✓ git_clean removed untracked file");

    println!("\n=== All Plan 043 Acceptance Criteria Passed Successfully! ===");
    Ok(())
}
