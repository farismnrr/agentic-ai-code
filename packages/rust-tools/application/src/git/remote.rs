use super::*;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use url::Url;

use super::remote_process::run_remote_git;

const MAX_REMOTE_NAME_BYTES: usize = 64;
const MAX_REMOTE_CONFIG_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Serialize)]
pub(super) struct GitRemoteIdentity {
    pub(super) name: String,
    pub(super) provider: &'static str,
    pub(super) owner: String,
    pub(super) repository: String,
    pub(super) canonical_url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct GitRemoteListResult {
    repository_root: String,
    remotes: Vec<GitRemoteIdentity>,
}

#[derive(Debug, Serialize)]
pub(super) struct GitRemoteBranchResult {
    repository_root: String,
    remote: GitRemoteIdentity,
    branch: String,
    exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct GitRemoteMutationResult {
    repository_root: String,
    operation: &'static str,
    remote: GitRemoteIdentity,
    branch: String,
    head: String,
    verified: bool,
}

pub(super) fn git_remote_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitRemoteListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_remote_config(&repo.root)?;
    let names = configured_remote_names(&repo.root)?;
    let mut remotes = Vec::with_capacity(names.len());
    for name in names {
        remotes.push(resolve_remote_identity(&repo.root, &name)?);
    }
    Ok(GitRemoteListResult {
        repository_root: repo.relative_root,
        remotes,
    })
}

pub(super) async fn git_remote_branch_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitRemoteBranchResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_remote_config(&repo.root)?;
    let remote = requested_remote(&repo.root, arguments)?;
    let branch = requested_branch(arguments)?;
    let head = remote_branch_head(&repo.root, &remote, &branch).await?;
    Ok(GitRemoteBranchResult {
        repository_root: repo.relative_root,
        remote,
        branch,
        exists: head.is_some(),
        head,
    })
}

pub(super) async fn git_fetch(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitRemoteMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_remote_config(&repo.root)?;
    let remote = requested_remote(&repo.root, arguments)?;
    let branch = requested_branch(arguments)?;
    let remote_ref = format!("refs/heads/{branch}");
    let tracking_ref = format!("refs/remotes/{}/{branch}", remote.name);
    let refspec = format!("{remote_ref}:{tracking_ref}");
    run_remote_git(
        &repo.root,
        &[
            "fetch",
            "--no-tags",
            "--no-recurse-submodules",
            &remote.canonical_url,
            &refspec,
        ],
    )
    .await?;
    let observed = resolve_commit_ref(&repo.root, &tracking_ref)?;
    let remote_head = remote_branch_head(&repo.root, &remote, &branch)
        .await?
        .ok_or_else(|| McpError::InvalidRequest("remote branch disappeared after fetch".into()))?;
    if observed != remote_head {
        return Err(McpError::InvalidRequest(
            "remote branch verification did not match fetched ref".into(),
        ));
    }
    Ok(GitRemoteMutationResult {
        repository_root: repo.relative_root,
        operation: "fetch",
        remote,
        branch,
        head: observed,
        verified: true,
    })
}

pub(super) async fn git_push(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitRemoteMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_remote_config(&repo.root)?;
    let remote = requested_remote(&repo.root, arguments)?;
    let branch = requested_branch(arguments)?;
    let local_ref = format!("refs/heads/{branch}");
    let local_head = resolve_commit_ref(&repo.root, &local_ref)?;
    let remote_ref = format!("refs/heads/{branch}");
    let refspec = format!("{local_ref}:{remote_ref}");
    run_remote_git(
        &repo.root,
        &[
            "push",
            "--porcelain",
            "--no-verify",
            &remote.canonical_url,
            &refspec,
        ],
    )
    .await?;
    let observed = remote_branch_head(&repo.root, &remote, &branch)
        .await?
        .ok_or_else(|| McpError::InvalidRequest("remote branch is absent after push".into()))?;
    if observed != local_head {
        return Err(McpError::InvalidRequest(
            "remote branch verification did not match pushed commit".into(),
        ));
    }
    if arguments
        .get("set_upstream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let tracking_ref = format!("refs/remotes/{}/{branch}", remote.name);
        run_git(
            &repo.root,
            &["update-ref", &tracking_ref, &local_head],
            8192,
        )?;
        let upstream = format!("{}/{branch}", remote.name);
        run_git(
            &repo.root,
            &["branch", "--set-upstream-to", &upstream, &branch],
            8192,
        )?;
    }
    Ok(GitRemoteMutationResult {
        repository_root: repo.relative_root,
        operation: "push",
        remote,
        branch,
        head: local_head,
        verified: true,
    })
}

pub(super) async fn git_remote_branch_delete(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitRemoteMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_remote_config(&repo.root)?;
    let remote = requested_remote(&repo.root, arguments)?;
    let branch = requested_branch(arguments)?;
    let expected = arguments
        .get("expected_sha")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("expected_sha is required".into()))?;
    validate_sha(expected)?;
    let default_branch = remote_default_branch(&repo.root, &remote).await?;
    if default_branch == branch {
        return Err(McpError::InvalidRequest(
            "remote default branch cannot be deleted".into(),
        ));
    }
    let current = remote_branch_head(&repo.root, &remote, &branch)
        .await?
        .ok_or_else(|| McpError::InvalidRequest("remote branch does not exist".into()))?;
    if current != expected {
        return Err(McpError::InvalidRequest(
            "remote branch changed since expected_sha was observed".into(),
        ));
    }
    let remote_ref = format!("refs/heads/{branch}");
    let refspec = format!(":{remote_ref}");
    run_remote_git(
        &repo.root,
        &[
            "push",
            "--porcelain",
            "--no-verify",
            &remote.canonical_url,
            &refspec,
        ],
    )
    .await?;
    if remote_branch_head(&repo.root, &remote, &branch)
        .await?
        .is_some()
    {
        return Err(McpError::InvalidRequest(
            "remote branch still exists after deletion".into(),
        ));
    }
    Ok(GitRemoteMutationResult {
        repository_root: repo.relative_root,
        operation: "delete_remote_branch",
        remote,
        branch,
        head: expected.to_owned(),
        verified: true,
    })
}

pub(super) fn requested_remote(
    root: &Path,
    arguments: &Value,
) -> Result<GitRemoteIdentity, McpError> {
    let name = arguments
        .get("remote")
        .and_then(Value::as_str)
        .unwrap_or("origin");
    validate_remote_name(name)?;
    resolve_remote_identity(root, name)
}

fn requested_branch(arguments: &Value) -> Result<String, McpError> {
    branch::validate_branch_name(
        arguments
            .get("branch")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest("branch is required".into()))?,
    )
}

fn configured_remote_names(root: &Path) -> Result<Vec<String>, McpError> {
    let keys = local_config_keys(root)?;
    let mut names = Vec::new();
    for key in keys {
        let Some(rest) = key.strip_prefix("remote.") else {
            continue;
        };
        let Some(name) = rest.strip_suffix(".url") else {
            continue;
        };
        validate_remote_name(name)?;
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    if names.len() > 16 {
        return Err(McpError::InvalidRequest(
            "remote count exceeds maximum".into(),
        ));
    }
    Ok(names)
}

fn resolve_remote_identity(root: &Path, name: &str) -> Result<GitRemoteIdentity, McpError> {
    validate_remote_name(name)?;
    let key = format!("remote.{name}.url");
    let output = git_command(root)
        .args(["config", "--local", "--no-includes", "--get-all", &key])
        .output()
        .map_err(|_| McpError::Internal("failed to inspect git remote".into()))?;
    if !output.status.success() || output.stdout.len() > 8192 {
        return Err(McpError::InvalidRequest("git remote is unavailable".into()));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| invalid_git_output())?;
    let values = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if values.len() != 1 {
        return Err(McpError::InvalidRequest(
            "git remote URL is ambiguous".into(),
        ));
    }
    let (owner, repository) = parse_github_repository(values[0])?;
    Ok(GitRemoteIdentity {
        name: name.to_owned(),
        provider: "github",
        canonical_url: format!("https://github.com/{owner}/{repository}.git"),
        owner,
        repository,
    })
}

fn parse_github_repository(value: &str) -> Result<(String, String), McpError> {
    let path = if let Some(rest) = value.strip_prefix("git@github.com:") {
        rest.to_owned()
    } else {
        let parsed = Url::parse(value)
            .map_err(|_| McpError::InvalidRequest("git remote URL is unsupported".into()))?;
        let supported = (parsed.scheme() == "https" && parsed.username().is_empty())
            || (parsed.scheme() == "ssh" && parsed.username() == "git");
        if !supported
            || parsed.host_str() != Some("github.com")
            || parsed.password().is_some()
            || parsed.port().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(McpError::InvalidRequest(
                "git remote URL is unsupported".into(),
            ));
        }
        parsed.path().trim_start_matches('/').to_owned()
    };
    let path = path.strip_suffix(".git").unwrap_or(&path);
    let mut segments = path.split('/');
    let owner = segments.next().unwrap_or_default();
    let repository = segments.next().unwrap_or_default();
    if segments.next().is_some() || !valid_repo_segment(owner) || !valid_repo_segment(repository) {
        return Err(McpError::InvalidRequest(
            "GitHub repository identity is invalid".into(),
        ));
    }
    Ok((owner.to_owned(), repository.to_owned()))
}

fn valid_repo_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && !value.starts_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn validate_remote_name(value: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.len() > MAX_REMOTE_NAME_BYTES
        || value.starts_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(McpError::InvalidRequest(
            "git remote name is invalid".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_sha(value: &str) -> Result<(), McpError> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(McpError::InvalidRequest("expected_sha is invalid".into()));
    }
    Ok(())
}

fn local_config_keys(root: &Path) -> Result<Vec<String>, McpError> {
    let output = git_command(root)
        .args([
            "config",
            "--local",
            "--no-includes",
            "--name-only",
            "--list",
        ])
        .output()
        .map_err(|_| McpError::Internal("failed to inspect git remote configuration".into()))?;
    if !output.status.success() || output.stdout.len() > MAX_REMOTE_CONFIG_BYTES {
        return Err(McpError::InvalidRequest(
            "git remote configuration could not be verified".into(),
        ));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| invalid_git_output())?;
    Ok(text
        .lines()
        .map(|line| line.trim().to_ascii_lowercase())
        .collect())
}

fn validate_remote_config(root: &Path) -> Result<(), McpError> {
    for key in local_config_keys(root)? {
        let dangerous = key.starts_with("include.")
            || key.starts_with("includeif.")
            || key.starts_with("url.")
            || key.starts_with("credential.")
            || key.starts_with("http.")
            || key == "core.askpass"
            || key == "core.sshcommand"
            || key == "core.gitproxy"
            || key.starts_with("protocol.")
            || (key.starts_with("remote.") && !(key.ends_with(".url") || key.ends_with(".fetch")));
        if dangerous {
            return Err(McpError::InvalidRequest(
                "repository Git configuration contains unsafe remote transport overrides".into(),
            ));
        }
    }
    Ok(())
}

pub(super) async fn remote_branch_head(
    root: &Path,
    remote: &GitRemoteIdentity,
    branch: &str,
) -> Result<Option<String>, McpError> {
    let target = format!("refs/heads/{branch}");
    let output = run_remote_git(
        root,
        &["ls-remote", "--heads", &remote.canonical_url, &target],
    )
    .await?;
    if output.is_empty() {
        return Ok(None);
    }
    let text = std::str::from_utf8(&output).map_err(|_| invalid_git_output())?;
    let mut lines = text.lines();
    let line = lines.next().ok_or_else(invalid_git_output)?;
    if lines.next().is_some() {
        return Err(invalid_git_output());
    }
    let mut fields = line.split('\t');
    let sha = fields.next().unwrap_or_default();
    let reference = fields.next().unwrap_or_default();
    validate_sha(sha)?;
    if reference != target {
        return Err(invalid_git_output());
    }
    Ok(Some(sha.to_owned()))
}

async fn remote_default_branch(
    root: &Path,
    remote: &GitRemoteIdentity,
) -> Result<String, McpError> {
    let output = run_remote_git(
        root,
        &["ls-remote", "--symref", &remote.canonical_url, "HEAD"],
    )
    .await?;
    let text = std::str::from_utf8(&output).map_err(|_| invalid_git_output())?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("ref: refs/heads/") {
            let branch = rest.strip_suffix("\tHEAD").ok_or_else(invalid_git_output)?;
            return branch::validate_branch_name(branch);
        }
    }
    Err(McpError::InvalidRequest(
        "remote default branch could not be verified".into(),
    ))
}
