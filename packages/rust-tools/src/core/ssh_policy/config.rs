use super::policy_error;
use crate::core::error::McpError;
use std::fs;
use std::path::{Path, PathBuf};

mod args;
pub use args::{openssh_args, openssh_args_for_chain, openssh_args_for_chain_with_program};

const MAX_ALIAS_BYTES: usize = 255;
const MAX_INCLUDE_DEPTH: usize = 4;
const MAX_INCLUDE_FILES: usize = 64;
const MAX_PROXY_HOPS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConnectionSpec {
    pub alias: String,
    pub hostname: String,
    pub user: Option<String>,
    pub port: u16,
    pub identity_file: PathBuf,
    pub known_hosts_file: PathBuf,
    pub proxy_jump: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostBlock {
    patterns: Vec<String>,
    directives: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct PartialSpec {
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<String>,
    known_hosts_file: Option<String>,
    proxy_jump: Option<Vec<String>>,
}

pub fn validate_alias(alias: &str) -> Result<(), McpError> {
    if alias.is_empty()
        || alias.len() > MAX_ALIAS_BYTES
        || alias.starts_with('-')
        || alias.chars().any(|ch| {
            ch.is_ascii_control()
                || ch.is_ascii_whitespace()
                || !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-')
        })
    {
        return Err(policy_error("SSH host alias is invalid"));
    }
    Ok(())
}

pub fn resolve_connection_spec(
    ssh_root: &Path,
    config_path: &Path,
    alias: &str,
) -> Result<SshConnectionSpec, McpError> {
    validate_alias(alias)?;
    let root = fs::canonicalize(ssh_root)
        .map_err(|_| policy_error("SSH credential root is unavailable"))?;
    if !root.is_dir() {
        return Err(policy_error("SSH credential root must be a directory"));
    }
    let config = canonical_file_within(&root, config_path, "SSH config")?;
    let lines = load_config_lines(&root, &config, 0, &mut 0)?;
    let blocks = parse_config(&lines)?;
    let mut spec = PartialSpec::default();

    // OpenSSH applies the first obtained value for each parameter. We preserve
    // that useful precedence property while only recognizing a small safe
    // connectivity subset.
    for block in blocks {
        if !host_block_matches(&block.patterns, alias)? {
            continue;
        }
        for (key, value) in block.directives {
            match key.as_str() {
                "hostname" if spec.hostname.is_none() => {
                    validate_hostname(&value)?;
                    spec.hostname = Some(value);
                }
                "user" if spec.user.is_none() => {
                    validate_user(&value)?;
                    spec.user = Some(value);
                }
                "port" if spec.port.is_none() => {
                    let port = value
                        .parse::<u16>()
                        .ok()
                        .filter(|port| *port != 0)
                        .ok_or_else(|| policy_error("SSH config contains an invalid port"))?;
                    spec.port = Some(port);
                }
                "identityfile" if spec.identity_file.is_none() => {
                    spec.identity_file = Some(value);
                }
                "userknownhostsfile" if spec.known_hosts_file.is_none() => {
                    if value.split_whitespace().count() != 1 {
                        return Err(policy_error(
                            "SSH config must use exactly one known-hosts file",
                        ));
                    }
                    spec.known_hosts_file = Some(value);
                }
                "proxyjump" if spec.proxy_jump.is_none() => {
                    spec.proxy_jump = Some(parse_proxy_jump(&value)?);
                }
                _ => {}
            }
        }
    }

    let hostname = spec.hostname.unwrap_or_else(|| alias.to_owned());
    validate_hostname(&hostname)?;
    let identity_raw = spec
        .identity_file
        .ok_or_else(|| policy_error("SSH alias must configure an explicit IdentityFile"))?;
    let identity_file = resolve_ssh_path(&root, &identity_raw, "SSH identity file")?;
    let known_hosts_raw = spec
        .known_hosts_file
        .unwrap_or_else(|| "~/.ssh/known_hosts".to_owned());
    let known_hosts_file = resolve_ssh_path(&root, &known_hosts_raw, "SSH known-hosts file")?;

    Ok(SshConnectionSpec {
        alias: alias.to_owned(),
        hostname,
        user: spec.user,
        port: spec.port.unwrap_or(22),
        identity_file,
        known_hosts_file,
        proxy_jump: spec.proxy_jump.unwrap_or_default(),
    })
}

pub fn resolve_connection_chain(
    ssh_root: &Path,
    config_path: &Path,
    alias: &str,
    explicit_via: &[String],
) -> Result<Vec<SshConnectionSpec>, McpError> {
    let target = resolve_connection_spec(ssh_root, config_path, alias)?;
    let hops = if explicit_via.is_empty() {
        target.proxy_jump.clone()
    } else {
        explicit_via.to_vec()
    };
    if hops.len() > MAX_PROXY_HOPS {
        return Err(policy_error("SSH jump chain exceeds allowed bounds"));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut chain = Vec::with_capacity(hops.len() + 1);
    for hop in hops {
        validate_alias(&hop)?;
        if hop == alias || !seen.insert(hop.clone()) {
            return Err(policy_error(
                "SSH jump chain contains a cycle or duplicate alias",
            ));
        }
        chain.push(resolve_connection_spec(ssh_root, config_path, &hop)?);
    }
    chain.push(target);
    Ok(chain)
}

fn parse_config(lines: &[String]) -> Result<Vec<HostBlock>, McpError> {
    let mut blocks = Vec::<HostBlock>::new();
    let mut current = HostBlock {
        patterns: vec!["*".into()],
        directives: Vec::new(),
    };
    let mut current_is_global = true;
    let mut in_match = false;
    for raw_line in lines {
        let line = strip_config_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = split_directive(line)?;
        let key_lower = key.to_ascii_lowercase();
        if key_lower == "include" {
            return Err(policy_error("SSH Include must be expanded before parsing"));
        }
        if key_lower == "match" {
            if !current.directives.is_empty() || !current_is_global {
                blocks.push(current);
            }
            current = HostBlock {
                patterns: vec!["*".into()],
                directives: Vec::new(),
            };
            current_is_global = true;
            in_match = true;
            continue;
        }
        if key_lower == "host" {
            if !current.directives.is_empty() || !current_is_global {
                blocks.push(current);
            }
            let patterns = shell_words::split(value)
                .map_err(|_| policy_error("SSH Host pattern could not be parsed"))?;
            if patterns.is_empty() {
                return Err(policy_error("SSH Host pattern must not be empty"));
            }
            current = HostBlock {
                patterns,
                directives: Vec::new(),
            };
            current_is_global = false;
            in_match = false;
            continue;
        }
        if in_match {
            continue;
        }
        // Only the reviewed connectivity subset is consumed later. Other
        // directives are intentionally inert because raw operator config is
        // never passed to OpenSSH.
        current.directives.push((key_lower, value.to_owned()));
    }
    if !current.directives.is_empty() || !current_is_global {
        blocks.push(current);
    }
    Ok(blocks)
}

fn parse_proxy_jump(value: &str) -> Result<Vec<String>, McpError> {
    if value.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for raw in value.split(',') {
        let alias = raw.trim();
        validate_alias(alias)?;
        result.push(alias.to_owned());
    }
    if result.is_empty() || result.len() > MAX_PROXY_HOPS {
        return Err(policy_error("SSH ProxyJump chain exceeds allowed bounds"));
    }
    Ok(result)
}

fn load_config_lines(
    root: &Path,
    path: &Path,
    depth: usize,
    included_files: &mut usize,
) -> Result<Vec<String>, McpError> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(policy_error("SSH Include nesting exceeds allowed bounds"));
    }
    *included_files += 1;
    if *included_files > MAX_INCLUDE_FILES {
        return Err(policy_error(
            "SSH Include file count exceeds allowed bounds",
        ));
    }
    let canonical = canonical_file_within(root, path, "SSH config")?;
    let text =
        fs::read_to_string(&canonical).map_err(|_| policy_error("SSH config is unavailable"))?;
    if text.len() > 512 * 1024 {
        return Err(policy_error("SSH config exceeds allowed bounds"));
    }
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let line = strip_config_comment(raw_line).trim();
        if line.is_empty() {
            lines.push(raw_line.to_owned());
            continue;
        }
        let Ok((key, value)) = split_directive(line) else {
            lines.push(raw_line.to_owned());
            continue;
        };
        if !key.eq_ignore_ascii_case("include") {
            lines.push(raw_line.to_owned());
            continue;
        }
        for include in shell_words::split(value)
            .map_err(|_| policy_error("SSH Include could not be parsed"))?
        {
            for include_path in expand_include_pattern(root, canonical.parent(), &include)? {
                lines.extend(load_config_lines(
                    root,
                    &include_path,
                    depth + 1,
                    included_files,
                )?);
            }
        }
    }
    Ok(lines)
}

fn expand_include_pattern(
    root: &Path,
    base: Option<&Path>,
    value: &str,
) -> Result<Vec<PathBuf>, McpError> {
    if value.contains('$') || value.contains('`') || value.contains('%') {
        return Err(policy_error("SSH Include path expansion is unsupported"));
    }
    let raw = if let Some(relative) = value.strip_prefix("~/.ssh/") {
        root.join(relative)
    } else if Path::new(value).is_absolute() {
        PathBuf::from(value)
    } else {
        base.unwrap_or(root).join(value)
    };
    let raw_text = raw.to_string_lossy();
    if !raw_text.contains('*') && !raw_text.contains('?') {
        return Ok(vec![canonical_file_within(root, &raw, "SSH Include")?]);
    }
    let parent = raw
        .parent()
        .ok_or_else(|| policy_error("SSH Include pattern is invalid"))?;
    let file_pattern = raw
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| policy_error("SSH Include pattern is invalid"))?;
    if parent.to_string_lossy().contains('*') || parent.to_string_lossy().contains('?') {
        return Err(policy_error(
            "SSH Include wildcards are supported only in the final path component",
        ));
    }
    let parent = fs::canonicalize(parent)
        .map_err(|_| policy_error("SSH Include directory is unavailable"))?;
    if !parent.starts_with(root) || !parent.is_dir() {
        return Err(policy_error(
            "SSH Include escapes the approved credential root",
        ));
    }
    let mut matches = fs::read_dir(parent)
        .map_err(|_| policy_error("SSH Include directory is unavailable"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| glob_matches(file_pattern, name))
                && path.is_file()
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn split_directive(line: &str) -> Result<(&str, &str), McpError> {
    let index = line
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '=')
        .ok_or_else(|| policy_error("SSH config directive is malformed"))?;
    let key = &line[..index];
    let value = line[index..]
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == '=')
        .trim();
    if key.is_empty() || value.is_empty() {
        return Err(policy_error("SSH config directive is malformed"));
    }
    Ok((key, value))
}

fn strip_config_comment(line: &str) -> &str {
    let mut escaped = false;
    let mut quoted = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn host_block_matches(patterns: &[String], alias: &str) -> Result<bool, McpError> {
    let mut positive = false;
    for pattern in patterns {
        let (negated, pattern) = pattern
            .strip_prefix('!')
            .map_or((false, pattern.as_str()), |value| (true, value));
        if pattern.is_empty() {
            return Err(policy_error("SSH Host pattern is invalid"));
        }
        if glob_matches(pattern, alias) {
            if negated {
                return Ok(false);
            }
            positive = true;
        }
    }
    Ok(positive)
}

fn glob_matches(pattern: &str, text: &str) -> bool {
    fn recurse(pattern: &[u8], text: &[u8]) -> bool {
        match pattern.first() {
            None => text.is_empty(),
            Some(b'*') => {
                recurse(&pattern[1..], text) || (!text.is_empty() && recurse(pattern, &text[1..]))
            }
            Some(b'?') => !text.is_empty() && recurse(&pattern[1..], &text[1..]),
            Some(value) => {
                !text.is_empty()
                    && value.eq_ignore_ascii_case(&text[0])
                    && recurse(&pattern[1..], &text[1..])
            }
        }
    }
    recurse(pattern.as_bytes(), text.as_bytes())
}

fn validate_hostname(value: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.starts_with('-')
        || value
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_ascii_whitespace())
        || value.contains(['/', '\\', '@', '%'])
    {
        return Err(policy_error("SSH hostname is invalid"));
    }
    Ok(())
}

fn validate_user(value: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.len() > 128
        || value.chars().any(|ch| {
            ch.is_ascii_control()
                || ch.is_ascii_whitespace()
                || !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-')
        })
    {
        return Err(policy_error("SSH user is invalid"));
    }
    Ok(())
}

fn resolve_ssh_path(root: &Path, value: &str, label: &str) -> Result<PathBuf, McpError> {
    if value.contains('%') || value.contains('$') || value.contains('`') {
        return Err(policy_error("SSH credential path expansion is unsupported"));
    }
    let candidate = if value == "~/.ssh" {
        root.to_path_buf()
    } else if let Some(relative) = value.strip_prefix("~/.ssh/") {
        root.join(relative)
    } else if Path::new(value).is_absolute() {
        PathBuf::from(value)
    } else {
        root.join(value)
    };
    canonical_file_within(root, &candidate, label)
}

fn canonical_file_within(root: &Path, path: &Path, label: &str) -> Result<PathBuf, McpError> {
    let canonical =
        fs::canonicalize(path).map_err(|_| policy_error(&format!("{label} is unavailable")))?;
    if !canonical.starts_with(root) {
        return Err(policy_error(&format!(
            "{label} escapes the approved SSH credential root"
        )));
    }
    if !canonical.is_file() {
        return Err(policy_error(&format!("{label} must be a regular file")));
    }
    Ok(canonical)
}
