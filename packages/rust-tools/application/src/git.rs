//! Bounded, read-only Git intelligence with a fail-closed process contract.

use crate::workspace::reject_protected_target;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_contained_cwd, resolve_existing_path, EntryKind};
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_GIT_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_GIT_PATH_BYTES: usize = 4096;
const MAX_GIT_REF_BYTES: usize = 512;
const MAX_GIT_RESULTS: usize = 100;
const DEFAULT_GIT_RESULTS: usize = 50;
const MAX_BLAME_LINES: usize = 500;
const MAX_DIFF_CONTEXT: u64 = 20;

#[derive(Debug, Serialize)]
struct GitStatusResult {
    repository_root: String,
    branch: Option<String>,
    detached: bool,
    upstream: Option<String>,
    ahead: u64,
    behind: u64,
    staged: Vec<String>,
    unstaged: Vec<String>,
    untracked: Vec<String>,
    conflicts: Vec<String>,
    truncated: bool,
}
#[derive(Debug, Serialize)]
struct GitTextResult {
    repository_root: String,
    text: String,
    truncated: bool,
}
#[derive(Debug, Serialize)]
struct GitLogResult {
    repository_root: String,
    commits: Vec<GitCommit>,
    truncated: bool,
}
#[derive(Debug, Serialize)]
struct GitCommit {
    sha: String,
    parents: Vec<String>,
    timestamp: i64,
    subject: String,
}
#[derive(Debug, Serialize)]
struct GitBlameResult {
    repository_root: String,
    lines: Vec<GitBlameLine>,
    truncated: bool,
}
#[derive(Debug, Serialize)]
struct GitBlameLine {
    line: u64,
    sha: String,
}

pub async fn dispatch_git_tool(
    name: &str,
    arguments: &Value,
    config: &ServerConfig,
) -> Result<Option<ToolCallResult>, McpError> {
    let value = match name {
        "git_status" => serde_json::to_value(git_status(arguments, config)?),
        "git_diff" => serde_json::to_value(git_diff(arguments, config)?),
        "git_log" => serde_json::to_value(git_log(arguments, config)?),
        "git_show" => serde_json::to_value(git_show(arguments, config)?),
        "git_blame" => serde_json::to_value(git_blame(arguments, config)?),
        _ => return Ok(None),
    }
    .map_err(|_| McpError::Internal("failed to serialize git result".into()))?;
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("failed to serialize git result".into()))?;
    if text.len() > MAX_GIT_OUTPUT_BYTES + 32 * 1024 {
        return Err(McpError::InvalidRequest(
            "git result exceeds output maximum".into(),
        ));
    }
    Ok(Some(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }])))
}

fn git_status(arguments: &Value, config: &ServerConfig) -> Result<GitStatusResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let include_untracked = arguments
        .get("include_untracked")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut args = vec!["status", "--porcelain=v2", "--branch", "-z"];
    if !include_untracked {
        args.push("--untracked-files=no");
    }
    let output = run_git(&repo.root, &args, MAX_GIT_OUTPUT_BYTES)?;
    let mut result = GitStatusResult {
        repository_root: repo.relative_root,
        branch: None,
        detached: false,
        upstream: None,
        ahead: 0,
        behind: 0,
        staged: vec![],
        unstaged: vec![],
        untracked: vec![],
        conflicts: vec![],
        truncated: false,
    };
    for record in output.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = std::str::from_utf8(record).map_err(|_| invalid_git_output())?;
        if let Some(v) = text.strip_prefix("# branch.head ") {
            if v == "(detached)" {
                result.detached = true
            } else {
                result.branch = Some(v.to_owned())
            }
        } else if let Some(v) = text.strip_prefix("# branch.upstream ") {
            result.upstream = Some(v.to_owned());
        } else if let Some(v) = text.strip_prefix("# branch.ab ") {
            for part in v.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    result.ahead = n.parse().unwrap_or(0)
                } else if let Some(n) = part.strip_prefix('-') {
                    result.behind = n.parse().unwrap_or(0)
                }
            }
        } else if let Some(path) = text.strip_prefix("? ") {
            if !is_protected_git_path(&repo.root, path) {
                push_bounded(
                    &mut result.untracked,
                    path.to_owned(),
                    &mut result.truncated,
                );
            }
        } else if text.starts_with("u ") {
            if let Some(path) =
                status_path(text).filter(|path| !is_protected_git_path(&repo.root, path))
            {
                push_bounded(&mut result.conflicts, path, &mut result.truncated);
            }
        } else if text.starts_with("1 ") || text.starts_with("2 ") {
            let bytes = text.as_bytes();
            if bytes.len() > 4 {
                let x = bytes[2] as char;
                let y = bytes[3] as char;
                if let Some(path) =
                    status_path(text).filter(|path| !is_protected_git_path(&repo.root, path))
                {
                    if x != '.' {
                        push_bounded(&mut result.staged, path.clone(), &mut result.truncated);
                    }
                    if y != '.' {
                        push_bounded(&mut result.unstaged, path, &mut result.truncated);
                    }
                }
            }
        }
    }
    Ok(result)
}

fn status_path(record: &str) -> Option<String> {
    let fields = if record.starts_with("1 ") {
        9
    } else if record.starts_with("2 ") {
        10
    } else if record.starts_with("u ") {
        11
    } else {
        return None;
    };
    record
        .splitn(fields, ' ')
        .nth(fields - 1)
        .map(str::to_owned)
}
fn push_bounded(target: &mut Vec<String>, value: String, truncated: &mut bool) {
    if target.len() < MAX_GIT_RESULTS {
        target.push(value)
    } else {
        *truncated = true
    }
}

fn git_diff(arguments: &Value, config: &ServerConfig) -> Result<GitTextResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("working");
    let context = arguments
        .get("context_lines")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .min(MAX_DIFF_CONTEXT);
    let max = bounded_bytes(arguments);
    let mut owned = vec![
        "diff".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        format!("--unified={context}"),
    ];
    match mode {
        "working" => {}
        "staged" => owned.push("--cached".into()),
        "refs" => {
            let base = validated_ref(arguments, "base_ref")?;
            let head = validated_ref(arguments, "head_ref")?;
            owned.push(base);
            owned.push(head);
        }
        _ => return Err(McpError::InvalidRequest("git diff mode is invalid".into())),
    }
    if let Some(path) = validated_optional_path(arguments, &repo, "path")? {
        owned.push("--".into());
        owned.push(path);
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let (text, truncated) = run_git_text_bounded(&repo.root, &refs, max)?;
    Ok(GitTextResult {
        repository_root: repo.relative_root,
        text,
        truncated,
    })
}

fn git_log(arguments: &Value, config: &ServerConfig) -> Result<GitLogResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let max = bounded_results(arguments);
    let mut owned = vec![
        "log".to_string(),
        "--no-show-signature".into(),
        "--format=%H%x1f%P%x1f%ct%x1f%s%x1e".into(),
        format!("--max-count={}", max + 1),
    ];
    if let Some(r) = arguments.get("ref").and_then(Value::as_str) {
        owned.push(validate_ref(r)?)
    }
    if let Some(path) = validated_optional_path(arguments, &repo, "path")? {
        owned.push("--".into());
        owned.push(path)
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let out = run_git(&repo.root, &refs, MAX_GIT_OUTPUT_BYTES)?;
    let text = std::str::from_utf8(&out).map_err(|_| invalid_git_output())?;
    let mut commits = Vec::new();
    for rec in text
        .split('\x1e')
        .filter(|r| !r.trim().is_empty())
        .take(max + 1)
    {
        let mut f = rec.trim_start_matches('\n').splitn(4, '\x1f');
        commits.push(GitCommit {
            sha: f.next().unwrap_or("").to_owned(),
            parents: f
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            timestamp: f.next().unwrap_or("0").parse().unwrap_or(0),
            subject: f.next().unwrap_or("").trim_end().to_owned(),
        });
    }
    let truncated = commits.len() > max;
    commits.truncate(max);
    Ok(GitLogResult {
        repository_root: repo.relative_root,
        commits,
        truncated,
    })
}

fn git_show(arguments: &Value, config: &ServerConfig) -> Result<GitTextResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let reference = validated_ref(arguments, "ref")?;
    let include_patch = arguments
        .get("include_patch")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max = bounded_bytes(arguments);
    let mut owned = vec![
        "show".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        "--no-show-signature".into(),
        "--format=fuller".into(),
    ];
    if !include_patch {
        owned.push("--no-patch".into())
    }
    owned.push(reference);
    if let Some(path) = validated_optional_path(arguments, &repo, "path")? {
        owned.push("--".into());
        owned.push(path)
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let (text, truncated) = run_git_text_bounded(&repo.root, &refs, max)?;
    Ok(GitTextResult {
        repository_root: repo.relative_root,
        text,
        truncated,
    })
}

fn git_blame(arguments: &Value, config: &ServerConfig) -> Result<GitBlameResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let path = validated_required_path(arguments, &repo, "path")?;
    let start = arguments
        .get("start_line")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let end = arguments
        .get("end_line")
        .and_then(Value::as_u64)
        .unwrap_or(start.saturating_add(199));
    if end < start || end.saturating_sub(start) as usize >= MAX_BLAME_LINES {
        return Err(McpError::InvalidRequest(
            "git blame line range exceeds maximum".into(),
        ));
    }
    let range = format!("{start},{end}");
    let out = run_git(
        &repo.root,
        &[
            "blame",
            "--no-textconv",
            "--line-porcelain",
            "-L",
            &range,
            "--",
            &path,
        ],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let text = std::str::from_utf8(&out).map_err(|_| invalid_git_output())?;
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut p = line.split_whitespace();
        let sha = p.next().unwrap_or("");
        let _orig = p.next();
        let final_line = p.next();
        if sha.len() >= 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            if let Ok(n) = final_line.unwrap_or("0").parse() {
                lines.push(GitBlameLine {
                    line: n,
                    sha: sha.to_owned(),
                })
            }
        }
    }
    Ok(GitBlameResult {
        repository_root: repo.relative_root,
        lines,
        truncated: false,
    })
}

/// Resolve a canonical Git workspace identity for non-MCP application
/// services such as LSP. The result is always contained by execution_root and
/// is obtained through the same hardened Git process path used by git_* tools.
pub(crate) fn resolve_git_workspace(
    cwd_arg: Option<&str>,
    config: &ServerConfig,
) -> Result<PathBuf, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    if cwd_arg.is_some_and(|value| value.len() > MAX_GIT_PATH_BYTES) {
        return Err(McpError::InvalidRequest(
            "workspace cwd exceeds maximum".into(),
        ));
    }
    let cwd = resolve_contained_cwd(&execution_root, cwd_arg)?;
    let out = run_git(&cwd, &["rev-parse", "--show-toplevel"], 8192)?;
    let root_text = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    let root = std::fs::canonicalize(root_text)
        .map_err(|_| McpError::InvalidRequest("workspace root is inaccessible".into()))?;
    if !root.starts_with(&execution_root) {
        return Err(McpError::InvalidRequest(
            "workspace root is outside execution root".into(),
        ));
    }
    Ok(root)
}

struct RepoContext {
    root: PathBuf,
    relative_root: String,
    execution_root: PathBuf,
}
fn resolve_repo(arguments: &Value, config: &ServerConfig) -> Result<RepoContext, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd_arg = arguments.get("cwd").and_then(Value::as_str);
    if cwd_arg.is_some_and(|v| v.len() > MAX_GIT_PATH_BYTES) {
        return Err(McpError::InvalidRequest("git cwd exceeds maximum".into()));
    }
    let cwd = resolve_contained_cwd(&execution_root, cwd_arg)?;
    let out = run_git(&cwd, &["rev-parse", "--show-toplevel"], 8192)?;
    let root_text = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    let root = std::fs::canonicalize(root_text)
        .map_err(|_| McpError::InvalidRequest("git repository root is inaccessible".into()))?;
    if !root.starts_with(&execution_root) {
        return Err(McpError::InvalidRequest(
            "git repository is outside execution root".into(),
        ));
    }
    let relative_root = root
        .strip_prefix(&execution_root)
        .ok()
        .and_then(Path::to_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_owned();
    Ok(RepoContext {
        root,
        relative_root,
        execution_root,
    })
}

mod process;
use process::*;

fn append_protected_exclusions(args: &mut Vec<String>) {
    args.push("--".into());
    for path in [
        ".ssh/**",
        ".aws/**",
        ".config/gcloud/**",
        ".docker/**",
        ".kube/**",
        ".npmrc",
        ".netrc",
        ".pypirc",
        ".cargo/credentials",
        ".cargo/credentials.toml",
    ] {
        args.push(format!(":(exclude){path}"));
    }
}

fn is_protected_git_path(root: &Path, path: &str) -> bool {
    let target = root.join(path);
    relay_core::protected_paths::is_protected_path(root, &target)
        || std::fs::canonicalize(&target)
            .map(|canonical| relay_core::protected_paths::is_protected_path(root, &canonical))
            .unwrap_or(false)
}
