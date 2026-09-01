use super::super::policy_error;
use crate::core::error::McpError;

pub(super) fn curl(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 || tokens.len() > 20 {
        return Err(policy_error("curl diagnostic arguments are invalid"));
    }
    let mut head = false;
    for token in &tokens[1..] {
        if matches!(
            token.as_str(),
            "-X" | "--request"
                | "-d"
                | "-F"
                | "--form"
                | "-T"
                | "--upload-file"
                | "-o"
                | "--output"
                | "-O"
                | "--remote-name"
                | "-H"
                | "--header"
                | "-u"
                | "--user"
                | "--cert"
                | "--key"
                | "--proxy"
                | "-x"
        ) || token.starts_with("--data")
        {
            return Err(policy_error(
                "curl mutation/credential/output options are forbidden",
            ));
        }
        if matches!(token.as_str(), "-I" | "--head") {
            head = true;
        } else if matches!(token.as_str(), "-L" | "--location") {
            return Err(policy_error(
                "curl redirects are forbidden in diagnostic mode",
            ));
        } else if token.starts_with('-') && token != "--compressed" {
            return Err(policy_error(
                "curl option is not allowed in diagnostic mode",
            ));
        }
    }
    let raw_url = tokens
        .last()
        .ok_or_else(|| policy_error("curl URL is missing"))?;
    let url = url::Url::parse(raw_url).map_err(|_| policy_error("curl URL is invalid"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err(policy_error(
            "curl diagnostics require a credential-free HTTP(S) URL",
        ));
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if is_metadata_host(&host) {
        return Err(policy_error(
            "cloud/container metadata endpoints are forbidden",
        ));
    }
    let mut result = vec![
        "curl".into(),
        "--fail".into(),
        "--silent".into(),
        "--show-error".into(),
        "--max-time".into(),
        "10".into(),
        "--proto".into(),
        "=http,https".into(),
        "--proto-redir".into(),
        "=http,https".into(),
    ];
    if head {
        result.push("--head".into());
    }
    if tokens.iter().any(|value| value == "--compressed") {
        result.push("--compressed".into());
    }
    result.push(raw_url.clone());
    Ok(result)
}

fn is_metadata_host(host: &str) -> bool {
    matches!(
        host,
        "169.254.169.254"
            | "100.100.100.200"
            | "metadata.google.internal"
            | "metadata.goog"
            | "instance-data.ec2.internal"
            | "fd00:ec2::254"
    ) || host.starts_with("169.254.")
}
