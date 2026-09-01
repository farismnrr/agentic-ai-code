use super::{invalid_git_output, MAX_GIT_OUTPUT_BYTES};
use crate::core::error::McpError;
use std::path::Path;

use super::process::run_git;

pub(super) fn is_protected_git_metadata_line(line: &str) -> bool {
    const PATH_METADATA_PREFIXES: [&str; 8] = [
        "diff --git ",
        "--- ",
        "+++ ",
        "rename from ",
        "rename to ",
        "copy from ",
        "copy to ",
        "Binary files ",
    ];
    PATH_METADATA_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
        && crate::core::protected_paths::contains_protected_path_reference(line)
}

pub(super) fn reject_protected_diff_changes(
    root: &Path,
    mode: &str,
    presentation_args: &[String],
) -> Result<(), McpError> {
    let mut args = vec![
        "diff".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        "-M".into(),
        "-C".into(),
        "--find-copies-harder".into(),
        "--name-status".into(),
        "-z".into(),
    ];
    match mode {
        "working" => {}
        "staged" => args.push("--cached".into()),
        "refs" => args.extend(
            presentation_args
                .iter()
                .filter(|arg| arg.len() == 40 && arg.bytes().all(|byte| byte.is_ascii_hexdigit()))
                .take(2)
                .cloned(),
        ),
        _ => return Err(McpError::InvalidRequest("git diff mode is invalid".into())),
    }
    reject_protected_name_status(root, &args, false)
}

pub(super) fn reject_protected_diff_renames(
    root: &Path,
    mode: &str,
    presentation_args: &[String],
) -> Result<(), McpError> {
    let mut args = vec![
        "diff".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        "-M".into(),
        "-C".into(),
        "--find-copies-harder".into(),
        "--name-status".into(),
        "-z".into(),
    ];
    match mode {
        "working" => {}
        "staged" => args.push("--cached".into()),
        "refs" => args.extend(
            presentation_args
                .iter()
                .filter(|arg| arg.len() == 40 && arg.bytes().all(|byte| byte.is_ascii_hexdigit()))
                .take(2)
                .cloned(),
        ),
        _ => return Err(McpError::InvalidRequest("git diff mode is invalid".into())),
    }
    reject_protected_name_status(root, &args, true)
}

pub(super) fn reject_protected_commit_changes(root: &Path, commit: &str) -> Result<(), McpError> {
    reject_protected_name_status(root, &commit_name_status_args(commit), false)
}

pub(super) fn reject_protected_commit_renames(root: &Path, commit: &str) -> Result<(), McpError> {
    reject_protected_name_status(root, &commit_name_status_args(commit), true)
}

pub(super) fn protected_staged_rename_copy_paths(
    root: &Path,
) -> Result<std::collections::HashSet<String>, McpError> {
    let args = [
        "diff",
        "--cached",
        "--no-ext-diff",
        "--no-textconv",
        "-M",
        "-C",
        "--find-copies-harder",
        "--name-status",
        "-z",
    ];
    let output = run_git(root, &args, MAX_GIT_OUTPUT_BYTES)?;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut hidden = std::collections::HashSet::new();
    while let Some(status) = fields.next() {
        let status = std::str::from_utf8(status).map_err(|_| invalid_git_output())?;
        let is_rename_or_copy = status.starts_with('R') || status.starts_with('C');
        let path_count = usize::from(is_rename_or_copy) + 1;
        let mut paths = Vec::with_capacity(path_count);
        let mut protected = false;
        for _ in 0..path_count {
            let path = fields.next().ok_or_else(invalid_git_output)?;
            let path = std::str::from_utf8(path).map_err(|_| invalid_git_output())?;
            protected |= is_protected_git_path(root, path);
            paths.push(path.to_owned());
        }
        if is_rename_or_copy && protected {
            hidden.extend(paths);
        }
    }
    Ok(hidden)
}

fn commit_name_status_args(commit: &str) -> Vec<String> {
    vec![
        "diff-tree".into(),
        "--root".into(),
        "-m".into(),
        "-r".into(),
        "-M".into(),
        "-C".into(),
        "--find-copies-harder".into(),
        "--no-commit-id".into(),
        "--name-status".into(),
        "-z".into(),
        commit.into(),
    ]
}

fn reject_protected_name_status(
    root: &Path,
    args: &[String],
    renames_only: bool,
) -> Result<(), McpError> {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_git(root, &refs, MAX_GIT_OUTPUT_BYTES)?;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(status) = fields.next() {
        let status = std::str::from_utf8(status).map_err(|_| invalid_git_output())?;
        let is_rename_or_copy = status.starts_with('R') || status.starts_with('C');
        let path_count = usize::from(is_rename_or_copy) + 1;
        let mut protected = false;
        for _ in 0..path_count {
            let path = fields.next().ok_or_else(invalid_git_output)?;
            let path = std::str::from_utf8(path).map_err(|_| invalid_git_output())?;
            protected |= is_protected_git_path(root, path);
        }
        if protected && (!renames_only || is_rename_or_copy) {
            return Err(McpError::InvalidRequest(
                "git change references a protected path".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_git_metadata_paths(
    cwd: &Path,
    execution_root: &Path,
) -> Result<(), McpError> {
    let out = run_git(
        cwd,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ],
        16 * 1024,
    )?;
    let text = std::str::from_utf8(&out).map_err(|_| invalid_git_output())?;
    let paths = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if paths.len() != 2 {
        return Err(McpError::InvalidRequest(
            "git metadata location is invalid".into(),
        ));
    }
    let mut canonical_paths = Vec::with_capacity(2);
    for value in paths {
        let canonical = std::fs::canonicalize(value)
            .map_err(|_| McpError::InvalidRequest("git metadata is inaccessible".into()))?;
        if !canonical.starts_with(execution_root)
            || crate::core::protected_paths::is_protected_path(execution_root, &canonical)
            || !canonical.is_dir()
        {
            return Err(McpError::InvalidRequest(
                "git metadata is outside the allowed workspace boundary".into(),
            ));
        }
        canonical_paths.push(canonical);
    }

    // Git may otherwise escape an in-root `.git` directory through a symlinked
    // object database or `objects/info/alternates`. The relay does not need
    // shared/external object stores, so keep object resolution inside the
    // canonical common Git directory and reject alternates fail-closed.
    let common_dir = &canonical_paths[1];
    let objects = std::fs::canonicalize(common_dir.join("objects"))
        .map_err(|_| McpError::InvalidRequest("git object database is inaccessible".into()))?;
    if !objects.starts_with(common_dir) || !objects.is_dir() {
        return Err(McpError::InvalidRequest(
            "git object database is outside the allowed metadata boundary".into(),
        ));
    }
    validate_git_object_store(&objects)?;
    Ok(())
}

fn validate_git_object_store(objects: &Path) -> Result<(), McpError> {
    const MAX_GIT_OBJECT_ENTRIES: usize = 200_000;
    let mut pending = vec![objects.to_path_buf()];
    let mut seen = 0usize;
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|_| McpError::InvalidRequest("git object database is inaccessible".into()))?;
        for entry in entries {
            let entry = entry.map_err(|_| {
                McpError::InvalidRequest("git object database is inaccessible".into())
            })?;
            seen += 1;
            if seen > MAX_GIT_OBJECT_ENTRIES {
                return Err(McpError::InvalidRequest(
                    "git object database exceeds validation limit".into(),
                ));
            }
            let file_type = entry.file_type().map_err(|_| {
                McpError::InvalidRequest("git object database is inaccessible".into())
            })?;
            if file_type.is_symlink() {
                return Err(McpError::InvalidRequest(
                    "git object database contains a symlink".into(),
                ));
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    let alternates = objects.join("info/alternates");
    if std::fs::metadata(&alternates).is_ok()
        && std::fs::read(&alternates)
            .map_err(|_| {
                McpError::InvalidRequest("git alternate object database is inaccessible".into())
            })?
            .iter()
            .any(|byte| !byte.is_ascii_whitespace())
    {
        return Err(McpError::InvalidRequest(
            "git alternate object databases are not allowed".into(),
        ));
    }
    Ok(())
}

pub(super) fn is_protected_git_path(root: &Path, path: &str) -> bool {
    let target = root.join(path);
    crate::core::protected_paths::is_protected_path(root, &target)
        || std::fs::canonicalize(&target)
            .map(|canonical| crate::core::protected_paths::is_protected_path(root, &canonical))
            .unwrap_or(false)
}
