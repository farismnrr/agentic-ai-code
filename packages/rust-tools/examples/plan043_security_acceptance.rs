//! Plan 043 negative/security and advanced-Git acceptance.

use ai_tools::application::execution::{dispatch_tool_call, JobManager};
use ai_tools::application::hooks::effect_classes;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::core::config::ServerConfig;
use ai_tools::core::workspace_path::MAX_AUTHORIZED_WORKSPACES;
use ai_tools::interfaces::mcp::{find_tool, tool_catalog};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("plan043-sec-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "plan043@example.test"]);
    git(root, &["config", "user.name", "Plan 043"]);
    fs::write(root.join("tracked.txt"), "base\n").unwrap();
    git(root, &["add", "tracked.txt"]);
    git(root, &["commit", "-q", "-m", "base"]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let boundary = TempDir::new("boundary");
    let outside = TempDir::new("outside");
    let repo = boundary.0.join("repo");
    fs::create_dir_all(&repo)?;
    let repo = fs::canonicalize(repo)?;
    init_repo(&repo);
    let config = ServerConfig {
        dir: Some(repo.to_string_lossy().into_owned()),
        execution_root: Some(boundary.0.to_string_lossy().into_owned()),
        ..Default::default()
    };
    config.ensure_workspaces_initialized()?;
    let jobs = JobManager::new(config.clone());
    let lsp = Arc::new(LspSessionManager::new(config.clone())?);
    let hooks = ai_tools::application::hooks::HookManager::load(Arc::new(config.clone()))?;

    assert_eq!(
        tool_catalog().len(),
        77,
        "Plan 043 must extend all 50 v7 tools without removals"
    );
    assert!(find_tool("git_push").is_some());
    assert!(find_tool("git_commit_amend").is_some());
    for tool in [
        "git_commit_amend",
        "git_worktree_remove",
        "git_stash_pop",
        "git_tag_delete",
        "git_restore",
        "git_clean",
        "git_reset",
        "git_remote_set_url",
    ] {
        let effects = effect_classes(tool, true, false);
        assert!(effects.contains(&"workspace_write"), "{tool}: {effects:?}");
        assert_ne!(effects, vec!["git_read"]);
    }

    let workspace_add = find_tool("workspace_add").unwrap();
    assert!(dispatch_tool_call(
        &workspace_add,
        &json!({"path": outside.0.to_string_lossy()}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());
    let ssh = boundary.0.join(".ssh");
    fs::create_dir_all(&ssh)?;
    assert!(dispatch_tool_call(
        &workspace_add,
        &json!({"path": ssh.to_string_lossy()}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());
    for index in 0..MAX_AUTHORIZED_WORKSPACES {
        let path = boundary.0.join(format!("authorized-{index}"));
        fs::create_dir_all(&path)?;
        let result = dispatch_tool_call(
            &workspace_add,
            &json!({"path": path.to_string_lossy()}),
            &config,
            &jobs,
            &lsp,
            &hooks,
        )
        .await?;
        assert!(!result.is_error);
    }
    let overflow = boundary.0.join("authorized-overflow");
    fs::create_dir_all(&overflow)?;
    assert!(dispatch_tool_call(
        &workspace_add,
        &json!({"path": overflow.to_string_lossy()}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());

    let worktree_add = find_tool("git_worktree_add").unwrap();
    assert!(dispatch_tool_call(
        &worktree_add,
        &json!({"cwd": repo.to_string_lossy(), "path": outside.0.join("wt").to_string_lossy(), "create_branch": "outside-wt"}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());

    fs::write(repo.join("tracked.txt"), "amended\n")?;
    let stage = find_tool("git_stage").unwrap();
    dispatch_tool_call(
        &stage,
        &json!({"cwd": repo.to_string_lossy(), "paths": ["tracked.txt"]}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    let amend = find_tool("git_commit_amend").unwrap();
    let result = dispatch_tool_call(
        &amend,
        &json!({"cwd": repo.to_string_lossy(), "message": "amended base"}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!result.is_error);
    assert_eq!(git(&repo, &["log", "-1", "--format=%s"]), "amended base");

    fs::write(repo.join("pick.txt"), "pick\n")?;
    git(&repo, &["add", "pick.txt"]);
    git(&repo, &["commit", "-q", "-m", "pick source"]);
    let pick_sha = git(&repo, &["rev-parse", "HEAD"]);
    let parent_sha = git(&repo, &["rev-parse", "HEAD^"]);
    git(&repo, &["reset", "--hard", "-q", &parent_sha]);
    let cherry = find_tool("git_cherry_pick").unwrap();
    dispatch_tool_call(
        &cherry,
        &json!({"cwd": repo.to_string_lossy(), "commit": pick_sha}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(repo.join("pick.txt").exists());
    let picked_head = git(&repo, &["rev-parse", "HEAD"]);
    let revert = find_tool("git_revert").unwrap();
    dispatch_tool_call(
        &revert,
        &json!({"cwd": repo.to_string_lossy(), "commit": picked_head}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!repo.join("pick.txt").exists());

    fs::write(repo.join("tracked.txt"), "dirty\n")?;
    let restore = find_tool("git_restore").unwrap();
    dispatch_tool_call(
        &restore,
        &json!({"cwd": repo.to_string_lossy(), "paths": ["tracked.txt"]}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert_eq!(fs::read_to_string(repo.join("tracked.txt"))?, "amended\n");

    fs::write(repo.join("scratch.tmp"), "scratch\n")?;
    fs::write(repo.join(".env.local"), "SECRET=keep\n")?;
    let clean = find_tool("git_clean").unwrap();
    dispatch_tool_call(
        &clean,
        &json!({"cwd": repo.to_string_lossy(), "dry_run": false}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!repo.join("scratch.tmp").exists());
    assert!(
        repo.join(".env.local").exists(),
        "git_clean must preserve protected files"
    );
    fs::remove_file(repo.join(".env.local"))?;

    fs::write(repo.join("tracked.txt"), "stashed\n")?;
    fs::write(repo.join(".env.local"), "SECRET=not-stashed\n")?;
    let stash_push = find_tool("git_stash_push").unwrap();
    dispatch_tool_call(
        &stash_push,
        &json!({"cwd": repo.to_string_lossy(), "include_untracked": true}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(repo.join(".env.local").exists());
    let stash_apply = find_tool("git_stash_apply").unwrap();
    dispatch_tool_call(
        &stash_apply,
        &json!({"cwd": repo.to_string_lossy(), "index": 0}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert_eq!(fs::read_to_string(repo.join("tracked.txt"))?, "stashed\n");
    dispatch_tool_call(
        &restore,
        &json!({"cwd": repo.to_string_lossy(), "paths": ["tracked.txt"]}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    let stash_drop = find_tool("git_stash_drop").unwrap();
    dispatch_tool_call(
        &stash_drop,
        &json!({"cwd": repo.to_string_lossy(), "index": 0}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    fs::remove_file(repo.join(".env.local"))?;

    let remote_add = find_tool("git_remote_add").unwrap();
    assert!(dispatch_tool_call(
        &remote_add,
        &json!({"cwd": repo.to_string_lossy(), "name": "bad", "url": "https://user:secret@github.com/example/repo.git"}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());
    assert!(dispatch_tool_call(
        &remote_add,
        &json!({"cwd": repo.to_string_lossy(), "name": "bad", "url": "http://github.com/example/repo.git"}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await
    .is_err());

    let before_reset = git(&repo, &["rev-parse", "HEAD^"]);
    let reset = find_tool("git_reset").unwrap();
    let result = dispatch_tool_call(
        &reset,
        &json!({"cwd": repo.to_string_lossy(), "target": before_reset, "mode": "soft"}),
        &config,
        &jobs,
        &lsp,
        &hooks,
    )
    .await?;
    assert!(!result.is_error);

    println!("Plan 043 security/history acceptance: PASS");
    Ok(())
}
