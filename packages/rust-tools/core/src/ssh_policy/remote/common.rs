use super::super::policy_error;
use crate::error::McpError;

pub(super) fn simple(tokens: &[String], max_args: usize) -> Result<Vec<String>, McpError> {
    if tokens.len() > max_args {
        return Err(policy_error(
            "diagnostic command arguments exceed allowed bounds",
        ));
    }
    for arg in tokens.iter().skip(1) {
        reject_shellish(arg)?;
    }
    Ok(tokens.to_vec())
}

pub(super) fn validate_identifier(value: &str, label: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.len() > 255
        || value.chars().any(|ch| {
            ch.is_ascii_control()
                || ch.is_ascii_whitespace()
                || matches!(ch, ';' | '|' | '&' | '>' | '<' | '`' | '$')
        })
    {
        return Err(policy_error(&format!("{label} identifier is invalid")));
    }
    Ok(())
}

pub(super) fn reject_shellish(value: &str) -> Result<(), McpError> {
    if value.chars().any(|ch| {
        matches!(
            ch,
            ';' | '|' | '&' | '>' | '<' | '`' | '$' | '\n' | '\r' | '\0'
        )
    }) {
        return Err(policy_error(
            "diagnostic argument contains forbidden shell syntax",
        ));
    }
    Ok(())
}

pub(super) fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.starts_with('.')
}

pub(super) fn path_looks_sensitive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "/etc/shadow",
        "/etc/gshadow",
        "/etc/ssh",
        "/root/",
        "/.ssh/",
        "/run/secrets",
        "/var/run/secrets",
        "/proc/self/environ",
        "/proc/1/environ",
        "/proc/self/fd",
        "/proc/1/fd",
        "/proc/self/root",
        "/proc/1/root",
        ".env",
        "id_rsa",
        "id_ed25519",
        "credentials",
        "credential",
        "secret",
        "password",
        "passwd",
        "token",
        "private_key",
        "private-key",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || (lower.starts_with("/proc/")
            && (lower.contains("/environ")
                || lower.contains("/fd/")
                || lower.ends_with("/fd")
                || lower.contains("/root/")))
}
