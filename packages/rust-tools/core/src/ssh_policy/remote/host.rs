use super::super::policy_error;
use super::common::{looks_like_path, path_looks_sensitive, reject_shellish};
use crate::error::McpError;

pub(super) fn bounded_observation(tokens: &[String]) -> Result<Vec<String>, McpError> {
    match tokens[0].as_str() {
        "id" => id(tokens),
        "df" => df(tokens),
        "free" => free(tokens),
        "ps" => ps(tokens),
        "ss" => ss(tokens),
        "ip" => ip(tokens),
        _ => Err(policy_error("host diagnostic command is not supported")),
    }
}

fn id(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 8 {
        return Err(policy_error("id arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        if token.starts_with('-') && !matches!(token.as_str(), "-u" | "-g" | "-G" | "-n" | "-r") {
            return Err(policy_error("id option is not allowed"));
        }
        reject_shellish(token)?;
    }
    Ok(tokens.to_vec())
}

fn df(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 16 {
        return Err(policy_error("df arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        reject_shellish(token)?;
        if looks_like_path(token) && path_looks_sensitive(token) {
            return Err(policy_error("df path is confidentiality-sensitive"));
        }
        if token.starts_with("--output") {
            // GNU df --output prints selected fields to stdout; it is safe but
            // allowing arbitrary field syntax adds no diagnostic value here.
            return Err(policy_error("df custom output is unsupported"));
        }
    }
    Ok(tokens.to_vec())
}

fn free(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 8 {
        return Err(policy_error("free arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        if token.starts_with('-')
            && !matches!(
                token.as_str(),
                "-b" | "--bytes"
                    | "-k"
                    | "--kibi"
                    | "-m"
                    | "--mebi"
                    | "-g"
                    | "--gibi"
                    | "-h"
                    | "--human"
                    | "-t"
                    | "--total"
                    | "-w"
                    | "--wide"
            )
        {
            return Err(policy_error(
                "free streaming/unsupported option is forbidden",
            ));
        }
    }
    Ok(tokens.to_vec())
}

fn ps(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 24 {
        return Err(policy_error("ps arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        reject_shellish(token)?;
        if token.starts_with("--sort=") || token.starts_with("--format=") {
            continue;
        }
        if token.starts_with("--")
            && !matches!(
                token.as_str(),
                "--all"
                    | "--everyone"
                    | "--forest"
                    | "--headers"
                    | "--no-headers"
                    | "--cols"
                    | "--columns"
                    | "--rows"
                    | "--width"
            )
        {
            return Err(policy_error("ps long option is not allowed"));
        }
    }
    Ok(tokens.to_vec())
}

fn ss(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 20 {
        return Err(policy_error("ss arguments exceed allowed bounds"));
    }
    for token in &tokens[1..] {
        reject_shellish(token)?;
        let lower = token.to_ascii_lowercase();
        if lower == "--kill"
            || (token.starts_with('-') && !token.starts_with("--") && token[1..].contains('K'))
        {
            return Err(policy_error("ss socket-kill mode is forbidden"));
        }
        if lower.starts_with("--events") {
            return Err(policy_error("ss event streaming is forbidden"));
        }
    }
    Ok(tokens.to_vec())
}

fn ip(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() > 24 {
        return Err(policy_error("ip arguments exceed allowed bounds"));
    }
    let mut index = 1usize;
    while index < tokens.len() && tokens[index].starts_with('-') {
        if !matches!(
            tokens[index].as_str(),
            "-4" | "-6"
                | "-j"
                | "-json"
                | "-p"
                | "-pretty"
                | "-d"
                | "-details"
                | "-s"
                | "-stats"
                | "-br"
                | "-brief"
                | "-o"
                | "-oneline"
                | "-h"
                | "-human"
                | "-iec"
        ) {
            return Err(policy_error("ip global option is not allowed"));
        }
        index += 1;
    }
    let object = tokens
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| policy_error("ip diagnostic object is required"))?;
    index += 1;
    let command = tokens.get(index).map(String::as_str);
    let command_is = |allowed: &[&str]| command.is_none_or(|value| allowed.contains(&value));
    match object {
        "addr" | "address" if command_is(&["show", "list"]) => {}
        "link" if command_is(&["show", "list"]) => {}
        "route" if command_is(&["show", "list", "get"]) => {}
        "neigh" | "neighbor" if command_is(&["show", "get"]) => {}
        "rule" if command_is(&["show", "list"]) => {}
        _ => {
            return Err(policy_error(
                "ip mutation or unsupported diagnostic is forbidden",
            ))
        }
    }
    const MUTATORS: &[&str] = &[
        "add", "delete", "del", "set", "change", "replace", "flush", "append", "prepend", "save",
        "restore", "exec", "monitor",
    ];
    if tokens[index..]
        .iter()
        .any(|value| MUTATORS.contains(&value.as_str()))
    {
        return Err(policy_error(
            "ip mutation/streaming arguments are forbidden",
        ));
    }
    for token in &tokens[index..] {
        reject_shellish(token)?;
    }
    Ok(tokens.to_vec())
}
