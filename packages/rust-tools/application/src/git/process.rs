use super::*;
use std::io::Read;
use std::process::Stdio;

pub(crate) fn run_git(cwd: &Path, args: &[&str], max: usize) -> Result<Vec<u8>, McpError> {
    let (output, truncated) = run_git_bytes_bounded(cwd, args, max)?;
    if truncated {
        return Err(McpError::InvalidRequest(
            "git output exceeds maximum".into(),
        ));
    }
    Ok(output)
}

pub(super) fn run_git_text_bounded(
    cwd: &Path,
    args: &[&str],
    max: usize,
) -> Result<(String, bool), McpError> {
    let (mut output, truncated) = run_git_bytes_bounded(cwd, args, max)?;
    if truncated {
        if let Err(error) = std::str::from_utf8(&output) {
            if error.error_len().is_none() {
                output.truncate(error.valid_up_to());
            }
        }
    }
    let text = std::str::from_utf8(&output).map_err(|_| invalid_git_output())?;
    if text.lines().any(|line| {
        [".ssh/", ".aws/", ".config/gcloud/", ".docker/", ".kube/"]
            .iter()
            .any(|prefix| line.contains(prefix))
            || [".npmrc", ".netrc", ".pypirc"].iter().any(|name| {
                line.split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_')
                    .any(|token| token == *name)
            })
    }) {
        return Err(McpError::InvalidRequest(
            "git output contains a protected path".into(),
        ));
    }
    Ok((text.to_owned(), truncated))
}

fn run_git_bytes_bounded(
    cwd: &Path,
    args: &[&str],
    max: usize,
) -> Result<(Vec<u8>, bool), McpError> {
    let mut child = git_command(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| McpError::Internal("failed to start git".into()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Internal("failed to capture git output".into()))?;
    let mut output = Vec::with_capacity(max.min(64 * 1024));
    let mut limited = (&mut stdout).take(max.saturating_add(1) as u64);
    limited
        .read_to_end(&mut output)
        .map_err(|_| McpError::InvalidRequest("git output could not be read".into()))?;
    let truncated = output.len() > max;
    if truncated {
        output.truncate(max);
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|_| McpError::Internal("failed to wait for git".into()))?;
    if !truncated && !status.success() {
        return Err(McpError::InvalidRequest("git command failed".into()));
    }
    Ok((output, truncated))
}

fn git_command(cwd: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("HOME", "/nonexistent")
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .arg("--no-pager");
    for kv in [
        "core.pager=cat",
        "core.fsmonitor=false",
        "core.hooksPath=/dev/null",
        "diff.external=",
        "diff.trustExitCode=false",
        "color.ui=false",
        "interactive.diffFilter=",
    ] {
        c.arg("-c").arg(kv);
    }
    c
}

pub(super) fn bounded_bytes(arguments: &Value) -> usize {
    arguments
        .get("max_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(64 * 1024)
        .min(MAX_GIT_OUTPUT_BYTES as u64) as usize
}

pub(super) fn bounded_results(arguments: &Value) -> usize {
    arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(DEFAULT_GIT_RESULTS)
        .clamp(1, MAX_GIT_RESULTS)
}

pub(super) fn validate_ref(value: &str) -> Result<String, McpError> {
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && !segment.starts_with('.')
            && !segment.ends_with('.')
            && !segment.contains("..")
    };
    let valid_chars = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let valid_shape =
        !value.starts_with('/') && !value.ends_with('/') && value.split('/').all(valid_segment);
    if value.is_empty()
        || value.len() > MAX_GIT_REF_BYTES
        || value.starts_with('-')
        || !valid_chars
        || !valid_shape
        || value.contains(['\0', '\n', '\r'])
    {
        Err(McpError::InvalidRequest("git ref is invalid".into()))
    } else {
        Ok(value.to_owned())
    }
}

pub(super) fn resolve_commit_ref(cwd: &Path, value: &str) -> Result<String, McpError> {
    let reference = validate_ref(value)?;
    let peel = format!("{reference}^{{commit}}");
    let output = run_git(
        cwd,
        &["rev-parse", "--verify", "--end-of-options", &peel],
        128,
    )?;
    let commit = std::str::from_utf8(&output)
        .map_err(|_| invalid_git_output())?
        .trim();
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(McpError::InvalidRequest(
            "git ref is not a commit reference".into(),
        ));
    }
    Ok(commit.to_owned())
}

pub(super) fn validated_ref(arguments: &Value, key: &str) -> Result<String, McpError> {
    validate_ref(
        arguments
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))?,
    )
}

pub(super) fn validated_optional_path(
    arguments: &Value,
    repo: &RepoContext,
    key: &str,
) -> Result<Option<String>, McpError> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|_| validated_required_path(arguments, repo, key))
        .transpose()
}

pub(super) fn validated_required_path(
    arguments: &Value,
    repo: &RepoContext,
    key: &str,
) -> Result<String, McpError> {
    let value = arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))?;
    if value.is_empty() || value.len() > MAX_GIT_PATH_BYTES {
        return Err(McpError::InvalidRequest(
            "git path exceeds allowed bounds".into(),
        ));
    }
    let root = repo.root.to_string_lossy();
    let resolved = resolve_existing_path(
        &repo.execution_root,
        Some(root.as_ref()),
        value,
        EntryKind::File,
    )?;
    reject_protected_target(&repo.execution_root, &resolved)?;
    let relative = resolved
        .strip_prefix(&repo.root)
        .map_err(|_| McpError::InvalidRequest("git path is outside repository".into()))?;
    relative
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| McpError::InvalidRequest("git path is not valid UTF-8".into()))
}

pub(super) fn invalid_git_output() -> McpError {
    McpError::InvalidRequest("git output is invalid".into())
}
