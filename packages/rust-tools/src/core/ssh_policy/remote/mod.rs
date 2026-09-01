use super::policy_error;
use crate::core::error::McpError;

mod common;
mod db;
mod docker;
mod git;
mod host;
mod network;
mod read;

const MAX_REMOTE_COMMAND_BYTES: usize = 16 * 1024;
const MAX_PIPELINE_NODES: usize = 16;
const MAX_NODE_ARGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRemoteCommand {
    pub rendered: String,
    pub summary: String,
}

pub fn validate_remote_command(
    raw: &str,
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<ValidatedRemoteCommand, McpError> {
    if raw.is_empty() || raw.len() > MAX_REMOTE_COMMAND_BYTES || raw.contains(['\n', '\r', '\0']) {
        return Err(policy_error(
            "remote diagnostic command exceeds allowed bounds",
        ));
    }
    let nodes = split_composition(raw)?;
    if nodes.is_empty() || nodes.len() > MAX_PIPELINE_NODES {
        return Err(policy_error(
            "remote diagnostic composition exceeds allowed bounds",
        ));
    }

    let mut rendered = String::new();
    let mut summary = String::new();
    for (index, (operator, node)) in nodes.iter().enumerate() {
        let tokens = shell_words::split(node)
            .map_err(|_| policy_error("remote diagnostic command could not be parsed"))?;
        if tokens.is_empty() || tokens.len() > MAX_NODE_ARGS {
            return Err(policy_error("remote diagnostic command node is invalid"));
        }
        let normalized = validate_command_node(&tokens, readonly_db_user, readonly_redis_user)?;
        if index > 0 {
            let op = operator
                .as_deref()
                .ok_or_else(|| policy_error("invalid command composition"))?;
            rendered.push(' ');
            rendered.push_str(op);
            rendered.push(' ');
            summary.push(' ');
            summary.push_str(op);
            summary.push(' ');
        }
        rendered.push_str(&render_tokens(&normalized));
        summary.push_str(&summarize_tokens(&normalized));
    }

    Ok(ValidatedRemoteCommand { rendered, summary })
}

pub(super) fn validate_command_node(
    tokens: &[String],
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    match tokens[0].as_str() {
        "docker" => docker::validate(tokens, readonly_db_user, readonly_redis_user),
        "uname" | "uptime" | "hostname" | "whoami" => common::simple(tokens, 8),
        "id" | "df" | "free" | "ps" | "ss" | "ip" => host::bounded_observation(tokens),
        "command" => read::command_discovery(tokens),
        "cat" | "head" | "tail" | "grep" | "wc" | "stat" => read::read_transform(tokens),
        "git" => git::validate(tokens),
        "curl" => network::curl(tokens),
        "sudo" | "su" | "doas" | "pkexec" | "sh" | "bash" | "dash" | "zsh" | "fish" | "python"
        | "python3" | "node" | "ruby" | "perl" | "php" | "lua" => Err(policy_error(
            "remote privilege escalation or arbitrary interpreter execution is forbidden",
        )),
        _ => Err(policy_error(
            "remote executable is not in the diagnostic allowlist",
        )),
    }
}

fn split_composition(raw: &str) -> Result<Vec<(Option<String>, String)>, McpError> {
    if raw.contains([';', '>', '<', '`']) || raw.contains("$(") || raw.contains("${") {
        return Err(policy_error(
            "remote shell mutation/expansion syntax is forbidden",
        ));
    }
    let mut result = Vec::new();
    let mut current = String::new();
    let mut operator: Option<String> = None;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let chars = raw.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            index += 1;
            continue;
        }
        if let Some(active) = quote {
            current.push(ch);
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            current.push(ch);
            index += 1;
            continue;
        }
        let found = if ch == '|' && chars.get(index + 1) == Some(&'|') {
            Some(("||", 2))
        } else if ch == '&' && chars.get(index + 1) == Some(&'&') {
            Some(("&&", 2))
        } else if ch == '|' {
            Some(("|", 1))
        } else if ch == '&' {
            return Err(policy_error("background remote execution is forbidden"));
        } else {
            None
        };
        if let Some((op, width)) = found {
            let node = current.trim();
            if node.is_empty() {
                return Err(policy_error("remote diagnostic composition is malformed"));
            }
            result.push((operator.take(), node.to_owned()));
            operator = Some(op.to_owned());
            current.clear();
            index += width;
            continue;
        }
        current.push(ch);
        index += 1;
    }
    if quote.is_some() || escaped {
        return Err(policy_error(
            "remote diagnostic command quoting is malformed",
        ));
    }
    let node = current.trim();
    if node.is_empty() {
        return Err(policy_error("remote diagnostic composition is malformed"));
    }
    result.push((operator, node.to_owned()));
    Ok(result)
}

fn render_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| shell_words::quote(token).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn summarize_tokens(tokens: &[String]) -> String {
    let mut summary = tokens
        .iter()
        .take(8)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    if summary.len() > 240 {
        summary.truncate(240);
    }
    summary
}
