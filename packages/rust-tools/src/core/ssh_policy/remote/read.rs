use super::super::policy_error;
use super::common::{looks_like_path, path_looks_sensitive, reject_shellish, validate_identifier};
use crate::core::error::McpError;

pub(super) fn command_discovery(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() != 3 || tokens[1] != "-v" {
        return Err(policy_error("only 'command -v <executable>' is allowed"));
    }
    validate_identifier(&tokens[2], "executable")?;
    Ok(tokens.to_vec())
}

pub(super) fn read_transform(tokens: &[String]) -> Result<Vec<String>, McpError> {
    match tokens[0].as_str() {
        "cat" => cat(tokens),
        "head" => head_or_tail(tokens, false),
        "tail" => head_or_tail(tokens, true),
        "grep" => grep(tokens),
        "wc" => wc(tokens),
        "stat" => stat(tokens),
        _ => Err(policy_error("read transform is not supported")),
    }
}

fn cat(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 8 {
        return Err(policy_error("cat requires a bounded file list"));
    }
    let mut paths = 0usize;
    for token in &tokens[1..] {
        if token.starts_with('-') {
            if !matches!(
                token.as_str(),
                "-n" | "--number" | "-b" | "--number-nonblank" | "-s" | "--squeeze-blank"
            ) {
                return Err(policy_error("cat option is not allowed"));
            }
        } else {
            paths += 1;
            validate_read_path(token)?;
        }
    }
    if paths == 0 {
        return Err(policy_error("cat requires at least one file"));
    }
    Ok(tokens.to_vec())
}

fn head_or_tail(tokens: &[String], tail_mode: bool) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 12 {
        return Err(policy_error("head/tail arguments exceed allowed bounds"));
    }
    let mut index = 1usize;
    while index < tokens.len() {
        let token = &tokens[index];
        if matches!(token.as_str(), "-f" | "--follow" | "--retry" | "-F") {
            return Err(policy_error("streaming/follow diagnostics are forbidden"));
        }
        if matches!(
            token.as_str(),
            "-q" | "--quiet" | "--silent" | "-v" | "--verbose"
        ) {
            index += 1;
            continue;
        }
        if matches!(token.as_str(), "-n" | "--lines" | "-c" | "--bytes") {
            let bytes_mode = matches!(token.as_str(), "-c" | "--bytes");
            index += 1;
            let value = tokens
                .get(index)
                .ok_or_else(|| policy_error("head/tail count is missing"))?;
            validate_count(value, bytes_mode)?;
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--lines=") {
            validate_count(value, false)?;
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--bytes=") {
            validate_count(value, true)?;
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix('-').filter(|value| !value.is_empty()) {
            if value.chars().all(|character| character.is_ascii_digit()) {
                validate_count(value, false)?;
                index += 1;
                continue;
            }
            return Err(policy_error("head/tail option is not allowed"));
        }
        validate_read_path(token)?;
        index += 1;
    }
    if tail_mode && tokens.iter().any(|value| value.starts_with("--pid")) {
        return Err(policy_error("tail process-follow mode is forbidden"));
    }
    Ok(tokens.to_vec())
}

fn grep(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 24 {
        return Err(policy_error("grep arguments exceed allowed bounds"));
    }
    let mut positional = 0usize;
    let mut index = 1usize;
    while index < tokens.len() {
        let token = &tokens[index];
        if matches!(
            token.as_str(),
            "-r" | "-R" | "--recursive" | "--dereference-recursive"
        ) || token.starts_with("--include")
            || token.starts_with("--exclude")
            || token.starts_with("--exclude-dir")
        {
            return Err(policy_error("recursive grep is forbidden"));
        }
        if matches!(
            token.as_str(),
            "-m" | "--max-count"
                | "-A"
                | "--after-context"
                | "-B"
                | "--before-context"
                | "-C"
                | "--context"
        ) {
            index += 1;
            let value = tokens
                .get(index)
                .ok_or_else(|| policy_error("grep bound is missing"))?;
            let max = if matches!(token.as_str(), "-m" | "--max-count") {
                1000
            } else {
                100
            };
            validate_numeric_bound(value, max, "grep bound")?;
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--max-count=") {
            validate_numeric_bound(value, 1000, "grep max-count")?;
            index += 1;
            continue;
        }
        if token.starts_with('-') {
            if !matches!(
                token.as_str(),
                "-n" | "--line-number"
                    | "-i"
                    | "--ignore-case"
                    | "-E"
                    | "--extended-regexp"
                    | "-F"
                    | "--fixed-strings"
                    | "-v"
                    | "--invert-match"
                    | "-H"
                    | "--with-filename"
                    | "-h"
                    | "--no-filename"
                    | "--color=never"
            ) {
                return Err(policy_error("grep option is not allowed"));
            }
        } else {
            positional += 1;
            if positional > 1 && looks_like_path(token) {
                validate_read_path(token)?;
            }
        }
        index += 1;
    }
    if positional == 0 {
        return Err(policy_error("grep pattern is required"));
    }
    Ok(tokens.to_vec())
}

fn wc(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 12 {
        return Err(policy_error("wc arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        if token.starts_with('-') {
            if !matches!(
                token.as_str(),
                "-l" | "--lines" | "-w" | "--words" | "-c" | "--bytes" | "-m" | "--chars"
            ) {
                return Err(policy_error("wc option is not allowed"));
            }
        } else {
            validate_read_path(token)?;
        }
    }
    Ok(tokens.to_vec())
}

fn stat(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 12 {
        return Err(policy_error("stat arguments exceed allowed bounds"));
    }
    let mut paths = 0usize;
    for token in &tokens[1..] {
        if token.starts_with('-') {
            if !matches!(
                token.as_str(),
                "-L" | "--dereference" | "-f" | "--file-system" | "-t" | "--terse"
            ) && !token.starts_with("--format=")
                && !token.starts_with("--printf=")
            {
                return Err(policy_error("stat option is not allowed"));
            }
        } else {
            paths += 1;
            validate_read_path(token)?;
        }
    }
    if paths == 0 {
        return Err(policy_error("stat requires at least one path"));
    }
    Ok(tokens.to_vec())
}
fn validate_count(value: &str, bytes_mode: bool) -> Result<(), McpError> {
    validate_numeric_bound(
        value,
        if bytes_mode { 64 * 1024 } else { 1000 },
        "read bound",
    )
}

fn validate_numeric_bound(value: &str, max: u64, label: &str) -> Result<(), McpError> {
    let parsed = value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0 && *value <= max)
        .ok_or_else(|| policy_error(&format!("{label} exceeds allowed bounds")))?;
    let _ = parsed;
    Ok(())
}

fn validate_read_path(value: &str) -> Result<(), McpError> {
    reject_shellish(value)?;
    if path_looks_sensitive(value) {
        return Err(policy_error(
            "diagnostic read targets a protected credential-like path",
        ));
    }
    if value == "-" {
        return Err(policy_error("stdin-backed remote reads are forbidden"));
    }
    Ok(())
}
