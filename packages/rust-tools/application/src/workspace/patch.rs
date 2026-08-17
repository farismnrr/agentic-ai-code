//! Constrained multi-file patch preflight and commit.
use super::protected::reject_protected_path;
use super::secure::{FileIdentity, SecureDirectory};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_contained_cwd, resolve_write_target, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

const MAX_PATCH_BYTES: usize = 512 * 1024;
const MAX_PATCH_FILES: usize = 20;
const MAX_PATCH_HUNKS: usize = 100;
const MAX_PATCH_FILE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct ApplyPatchResult {
    pub dry_run: bool,
    changed_paths: Vec<String>,
    hunks: usize,
    files: Vec<PatchFileResult>,
}
#[derive(Debug, Serialize)]
struct PatchFileResult {
    path: String,
    before_hash: String,
    after_hash: String,
    bytes: usize,
}
struct PlannedFile {
    path: String,
    name: std::ffi::OsString,
    identity: FileIdentity,
    mode: u32,
    before: Vec<u8>,
    after: Vec<u8>,
    directory: SecureDirectory,
}
struct FilePatch {
    path: String,
    hunks: Vec<Hunk>,
}
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_count: usize,
    lines: Vec<(char, String)>,
}

pub fn apply_patch(arguments: &Value, config: &ServerConfig) -> Result<ApplyPatchResult, McpError> {
    let patch = arguments
        .get("patch")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("patch is required".into()))?;
    if patch.is_empty() || patch.len() > MAX_PATCH_BYTES {
        return Err(McpError::InvalidRequest(
            "patch exceeds allowed bounds".into(),
        ));
    }
    let dry_run = arguments
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd = resolve_contained_cwd(&root, arguments.get("cwd").and_then(Value::as_str))?;
    let parsed = parse_patch(patch)?;
    let mut planned = Vec::new();
    let mut total_hunks = 0;
    for file_patch in parsed {
        total_hunks += file_patch.hunks.len();
        let target = resolve_write_target(
            &root,
            Some(cwd.to_string_lossy().as_ref()),
            &file_patch.path,
            EntryKind::File,
        )?;
        reject_protected_path(&root, &target)?;
        let parent = target
            .parent()
            .ok_or_else(|| McpError::InvalidRequest("patch target is invalid".into()))?
            .to_path_buf();
        let name = target
            .file_name()
            .ok_or_else(|| McpError::InvalidRequest("patch target is invalid".into()))?
            .to_os_string();
        let directory = SecureDirectory::open_relative(&root, &parent)?;
        let (mut file, identity, mode) = directory.open_regular_file(&name)?;
        let mut before = Vec::new();
        Read::by_ref(&mut file)
            .take((MAX_PATCH_FILE_BYTES + 1) as u64)
            .read_to_end(&mut before)
            .map_err(|_| McpError::InvalidRequest("patch target is inaccessible".into()))?;
        if before.len() > MAX_PATCH_FILE_BYTES {
            return Err(McpError::InvalidRequest(
                "patch target exceeds maximum".into(),
            ));
        }
        let source = String::from_utf8(before.clone())
            .map_err(|_| McpError::InvalidRequest("patch target is not valid UTF-8 text".into()))?;
        let after = apply_hunks(&source, &file_patch.hunks)?.into_bytes();
        if after.len() > MAX_PATCH_FILE_BYTES {
            return Err(McpError::InvalidRequest(
                "patched file exceeds maximum".into(),
            ));
        }
        planned.push(PlannedFile {
            path: file_patch.path,
            name,
            identity,
            mode,
            before,
            after,
            directory,
        });
    }
    let files = planned
        .iter()
        .map(|p| PatchFileResult {
            path: p.path.clone(),
            before_hash: stable_hash(&p.before),
            after_hash: stable_hash(&p.after),
            bytes: p.after.len(),
        })
        .collect();
    if !dry_run {
        for p in &planned {
            p.directory.verify_regular_entry(&p.name, p.identity)?;
        }
        let mut committed: Vec<&PlannedFile> = Vec::new();
        for p in &planned {
            if let Err(err) = p
                .directory
                .atomic_replace_regular_file(&p.name, p.identity, &p.after, p.mode)
            {
                let mut rollback_incomplete = false;
                for done in committed.into_iter().rev() {
                    match done.directory.open_regular_file(&done.name) {
                        Ok((_f, current, mode)) => {
                            if done
                                .directory
                                .atomic_replace_regular_file(
                                    &done.name,
                                    current,
                                    &done.before,
                                    mode,
                                )
                                .is_err()
                            {
                                rollback_incomplete = true;
                            }
                        }
                        Err(_) => rollback_incomplete = true,
                    }
                }
                if rollback_incomplete {
                    return Err(McpError::InvalidRequest(
                        "patch commit failed and rollback was incomplete; workspace may be partially modified".into(),
                    ));
                }
                return Err(err);
            }
            committed.push(p);
        }
    }
    Ok(ApplyPatchResult {
        dry_run,
        changed_paths: planned.iter().map(|p| p.path.clone()).collect(),
        hunks: total_hunks,
        files,
    })
}

fn parse_patch(input: &str) -> Result<Vec<FilePatch>, McpError> {
    let lines = input.lines().collect::<Vec<_>>();
    let mut i = 0;
    let mut files = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    let mut hunks_total = 0;
    while i < lines.len() {
        if !lines[i].starts_with("--- ") {
            return Err(bad_patch());
        }
        let old = normalize_header(&lines[i][4..])?;
        i += 1;
        if i >= lines.len() || !lines[i].starts_with("+++ ") {
            return Err(bad_patch());
        }
        let new = normalize_header(&lines[i][4..])?;
        if old != new {
            return Err(McpError::InvalidRequest(
                "patch rename/add/delete is unsupported".into(),
            ));
        }
        i += 1;
        let mut hunks = Vec::new();
        while i < lines.len() && !lines[i].starts_with("--- ") {
            if !lines[i].starts_with("@@ ") {
                return Err(bad_patch());
            }
            let (old_start, old_count, new_count) = parse_hunk_header(lines[i])?;
            i += 1;
            let mut body = Vec::new();
            while i < lines.len() && !lines[i].starts_with("@@ ") && !lines[i].starts_with("--- ") {
                let line = lines[i];
                let kind = line.chars().next().ok_or_else(bad_patch)?;
                if !matches!(kind, ' ' | '+' | '-') {
                    return Err(bad_patch());
                }
                body.push((kind, line[1..].to_owned()));
                i += 1;
            }
            hunks.push(Hunk {
                old_start,
                old_count,
                new_count,
                lines: body,
            });
            hunks_total += 1;
            if hunks_total > MAX_PATCH_HUNKS {
                return Err(McpError::InvalidRequest(
                    "patch hunk count exceeds maximum".into(),
                ));
            }
        }
        if !seen_paths.insert(new.clone()) {
            return Err(McpError::InvalidRequest(
                "patch contains duplicate target path".into(),
            ));
        }
        files.push(FilePatch { path: new, hunks });
        if files.len() > MAX_PATCH_FILES {
            return Err(McpError::InvalidRequest(
                "patch file count exceeds maximum".into(),
            ));
        }
    }
    if files.is_empty() {
        return Err(bad_patch());
    }
    Ok(files)
}
fn normalize_header(value: &str) -> Result<String, McpError> {
    let raw = value.split_whitespace().next().unwrap_or("");
    let raw = raw
        .strip_prefix("a/")
        .or_else(|| raw.strip_prefix("b/"))
        .unwrap_or(raw);
    if raw == "/dev/null"
        || raw.is_empty()
        || Path::new(raw).is_absolute()
        || raw.split('/').any(|p| p == ".." || p.is_empty())
    {
        return Err(McpError::InvalidRequest("patch path is invalid".into()));
    }
    Ok(raw.to_owned())
}
fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize), McpError> {
    let end = line[3..].find(" @@").map(|n| n + 3).ok_or_else(bad_patch)?;
    let mut parts = line[3..end].split_whitespace();
    let old = parse_range(parts.next().ok_or_else(bad_patch)?, '-')?;
    let new = parse_range(parts.next().ok_or_else(bad_patch)?, '+')?;
    Ok((old.0, old.1, new.1))
}
fn parse_range(value: &str, prefix: char) -> Result<(usize, usize), McpError> {
    let v = value.strip_prefix(prefix).ok_or_else(bad_patch)?;
    let mut p = v.split(',');
    let start = p
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(bad_patch)?;
    let count = p
        .next()
        .map(str::parse)
        .transpose()
        .map_err(|_| bad_patch())?
        .unwrap_or(1);
    Ok((start, count))
}
fn apply_hunks(source: &str, hunks: &[Hunk]) -> Result<String, McpError> {
    let newline = source.ends_with('\n');
    let src = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for h in hunks {
        let start = h.old_start.saturating_sub(1);
        if start < cursor || start > src.len() {
            return Err(stale_patch());
        }
        out.extend_from_slice(&src[cursor..start]);
        let mut pos = start;
        let mut old_seen = 0;
        let mut new_seen = 0;
        for (kind, text) in &h.lines {
            match kind {
                ' ' => {
                    if src.get(pos) != Some(text) {
                        return Err(stale_patch());
                    }
                    out.push(text.clone());
                    pos += 1;
                    old_seen += 1;
                    new_seen += 1
                }
                '-' => {
                    if src.get(pos) != Some(text) {
                        return Err(stale_patch());
                    }
                    pos += 1;
                    old_seen += 1
                }
                '+' => {
                    out.push(text.clone());
                    new_seen += 1
                }
                _ => unreachable!(),
            }
        }
        if old_seen != h.old_count || new_seen != h.new_count {
            return Err(bad_patch());
        }
        cursor = pos;
    }
    out.extend_from_slice(&src[cursor..]);
    let mut result = out.join("\n");
    if newline {
        result.push('\n')
    }
    Ok(result)
}
fn stable_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3)
    }
    format!("fnv1a64:{h:016x}")
}
fn bad_patch() -> McpError {
    McpError::InvalidRequest("patch format is invalid".into())
}
fn stale_patch() -> McpError {
    McpError::InvalidRequest("patch context is stale or ambiguous".into())
}
