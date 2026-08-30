use super::policy_error;
use crate::error::McpError;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_ALIAS_BYTES: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConnectionSpec {
    pub alias: String,
    pub hostname: String,
    pub user: Option<String>,
    pub port: u16,
    pub identity_file: PathBuf,
    pub known_hosts_file: PathBuf,
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
    let text =
        fs::read_to_string(&config).map_err(|_| policy_error("SSH config is unavailable"))?;
    let blocks = parse_config(&text)?;
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
    })
}

pub fn openssh_args(
    spec: &SshConnectionSpec,
    remote: &super::ValidatedRemoteCommand,
) -> Vec<String> {
    let mut args = vec![
        "-F".into(),
        "/dev/null".into(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "PasswordAuthentication=no".into(),
        "-o".into(),
        "KbdInteractiveAuthentication=no".into(),
        "-o".into(),
        "PreferredAuthentications=publickey".into(),
        "-o".into(),
        "NumberOfPasswordPrompts=0".into(),
        "-o".into(),
        "IdentitiesOnly=yes".into(),
        "-o".into(),
        "IdentityAgent=none".into(),
        "-o".into(),
        "ClearAllForwardings=yes".into(),
        "-o".into(),
        "ForwardAgent=no".into(),
        "-o".into(),
        "ForwardX11=no".into(),
        "-o".into(),
        "PermitLocalCommand=no".into(),
        "-o".into(),
        "ControlMaster=no".into(),
        "-o".into(),
        "ControlPersist=no".into(),
        "-o".into(),
        "RequestTTY=no".into(),
        "-o".into(),
        "StdinNull=yes".into(),
        "-o".into(),
        "EscapeChar=none".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        "UpdateHostKeys=no".into(),
        "-o".into(),
        "ConnectionAttempts=1".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        format!("UserKnownHostsFile={}", spec.known_hosts_file.display()),
        "-i".into(),
        spec.identity_file.to_string_lossy().into_owned(),
        "-p".into(),
        spec.port.to_string(),
    ];
    if let Some(user) = &spec.user {
        args.extend(["-l".into(), user.clone()]);
    }
    args.push(spec.hostname.clone());
    args.push(remote.rendered.clone());
    args
}

fn parse_config(text: &str) -> Result<Vec<HostBlock>, McpError> {
    let mut blocks = Vec::<HostBlock>::new();
    let mut current: Option<HostBlock> = None;
    for raw_line in text.lines() {
        let line = strip_config_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = split_directive(line)?;
        let key_lower = key.to_ascii_lowercase();
        if matches!(
            key_lower.as_str(),
            "include"
                | "match"
                | "proxycommand"
                | "proxyjump"
                | "localcommand"
                | "knownhostscommand"
                | "remotecommand"
                | "identityagent"
                | "pkcs11provider"
                | "securitykeyprovider"
                | "controlmaster"
                | "controlpath"
                | "controlpersist"
                | "localforward"
                | "remoteforward"
                | "dynamicforward"
        ) {
            return Err(policy_error(
                "SSH config contains an unsupported capability directive",
            ));
        }
        if matches!(
            key_lower.as_str(),
            "forwardagent" | "forwardx11" | "forwardx11trusted" | "permitlocalcommand"
        ) && !matches!(value.to_ascii_lowercase().as_str(), "no" | "false")
        {
            return Err(policy_error(
                "SSH config attempts to enable a forbidden capability",
            ));
        }
        if key_lower == "host" {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            let patterns = shell_words::split(value)
                .map_err(|_| policy_error("SSH Host pattern could not be parsed"))?;
            if patterns.is_empty() {
                return Err(policy_error("SSH Host pattern must not be empty"));
            }
            current = Some(HostBlock {
                patterns,
                directives: Vec::new(),
            });
            continue;
        }
        let block = current.as_mut().ok_or_else(|| {
            policy_error("SSH config directives before the first Host block are unsupported")
        })?;
        block.directives.push((key_lower, value.to_owned()));
    }
    if let Some(block) = current {
        blocks.push(block);
    }
    Ok(blocks)
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
