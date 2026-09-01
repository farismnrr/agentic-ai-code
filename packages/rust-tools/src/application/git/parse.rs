use super::context::resolve_repo;
use super::process::{invalid_git_output, run_git, validated_required_path};
use super::{MAX_BLAME_LINES, MAX_GIT_OUTPUT_BYTES, MAX_GIT_RESULTS};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct GitBlameResult {
    pub(super) repository_root: String,
    pub(super) lines: Vec<GitBlameLine>,
    pub(super) truncated: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct GitBlameLine {
    pub(super) line: u64,
    pub(super) sha: String,
}

pub(super) fn status_path(record: &str) -> Option<String> {
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

pub(super) fn push_bounded(target: &mut Vec<String>, value: String, truncated: &mut bool) {
    if target.len() < MAX_GIT_RESULTS {
        target.push(value)
    } else {
        *truncated = true
    }
}

pub(super) fn git_blame(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitBlameResult, McpError> {
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
