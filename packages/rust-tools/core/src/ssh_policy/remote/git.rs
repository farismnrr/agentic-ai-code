use super::super::policy_error;
use super::common::path_looks_sensitive;
use crate::error::McpError;

pub(super) fn validate(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 32 {
        return Err(policy_error("Git diagnostic subcommand is invalid"));
    }
    if tokens.iter().any(|arg| {
        arg == "-c"
            || arg.starts_with("--config-env")
            || arg.starts_with("--exec-path")
            || arg == "--paginate"
            || arg == "-p"
            || arg == "--ext-diff"
            || arg == "--textconv"
            || arg.starts_with("--output")
            || path_looks_sensitive(arg)
    }) {
        return Err(policy_error("Git diagnostic option/path is not allowed"));
    }
    match tokens[1].as_str() {
        "status" => {
            let mut result = vec!["git".into(), "--no-pager".into(), "status".into()];
            result.extend(tokens[2..].iter().cloned());
            Ok(result)
        }
        "rev-parse" => {
            let mut result = vec!["git".into(), "--no-pager".into(), "rev-parse".into()];
            result.extend(tokens[2..].iter().cloned());
            Ok(result)
        }
        "branch" => git_branch(tokens),
        "log" => git_log(tokens),
        _ => Err(policy_error(
            "Git mutation or unsupported subcommand is forbidden",
        )),
    }
}

fn git_branch(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if !tokens[2..].iter().any(|arg| arg == "--list") {
        return Err(policy_error("Git branch diagnostics require --list"));
    }
    for token in &tokens[2..] {
        if token.starts_with('-')
            && !matches!(
                token.as_str(),
                "--list"
                    | "--all"
                    | "-a"
                    | "--remotes"
                    | "-r"
                    | "--merged"
                    | "--no-merged"
                    | "--contains"
                    | "--no-contains"
                    | "--column"
            )
            && !token.starts_with("--sort=")
            && !token.starts_with("--format=")
        {
            return Err(policy_error("Git branch option is not allowed"));
        }
    }
    let mut result = vec!["git".into(), "--no-pager".into(), "branch".into()];
    result.extend(tokens[2..].iter().cloned());
    Ok(result)
}

fn git_log(tokens: &[String]) -> Result<Vec<String>, McpError> {
    for token in &tokens[2..] {
        if matches!(
            token.as_str(),
            "--patch" | "--raw" | "--stat" | "--numstat" | "--name-only" | "--name-status"
        ) {
            return Err(policy_error("Git log content-diff modes are forbidden"));
        }
    }
    let mut result = vec![
        "git".into(),
        "--no-pager".into(),
        "log".into(),
        "--no-patch".into(),
        "--max-count=100".into(),
        "--format=%h %ad %s".into(),
        "--date=iso-strict".into(),
    ];
    // Only revision/path selectors are retained; user formatting/count options
    // are intentionally ignored to keep output bounded and confidentiality-safe.
    for token in &tokens[2..] {
        if token.starts_with('-') {
            if matches!(token.as_str(), "--all" | "--branches" | "--remotes")
                || token.starts_with("--since=")
                || token.starts_with("--until=")
            {
                result.push(token.clone());
            } else {
                return Err(policy_error("Git log option is not allowed"));
            }
        } else {
            result.push(token.clone());
        }
    }
    Ok(result)
}
