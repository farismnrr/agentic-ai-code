use super::super::policy_error;
use super::common::validate_identifier;
use super::db;
use crate::core::error::McpError;

const MAX_LOG_TAIL: u64 = 1000;

pub(super) fn validate(
    tokens: &[String],
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    if tokens.len() < 2 {
        return Err(policy_error("Docker diagnostic subcommand is required"));
    }
    match tokens[1].as_str() {
        "ps" | "images" => listing(tokens),
        "container" if tokens.get(2).map(String::as_str) == Some("ls") => listing(tokens),
        "logs" => logs(tokens),
        "stats" => stats(tokens),
        "top" => top(tokens),
        "inspect" => inspect(tokens),
        "exec" => exec(tokens, readonly_db_user, readonly_redis_user),
        "compose" => compose(tokens, readonly_db_user, readonly_redis_user),
        "run" | "start" | "stop" | "restart" | "kill" | "rm" | "rmi" | "build" | "pull"
        | "push" | "commit" | "update" | "cp" | "create" | "rename" | "attach" | "import"
        | "load" | "save" | "tag" | "login" | "logout" | "system" => Err(policy_error(
            "Docker mutation is forbidden in SSH diagnostic mode",
        )),
        _ => Err(policy_error(
            "Docker subcommand is not in the diagnostic allowlist",
        )),
    }
}

fn listing(tokens: &[String]) -> Result<Vec<String>, McpError> {
    let container_listing = tokens.get(1).map(String::as_str) == Some("container");
    let option_start = if container_listing { 3 } else { 2 };
    let image_listing = tokens.get(1).map(String::as_str) == Some("images");
    let mut filters = Vec::new();
    let mut format: Option<String> = None;
    let mut all = false;
    let mut quiet = false;
    let mut index = option_start;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-a" | "--all" => all = true,
            "-q" | "--quiet" => quiet = true,
            "--filter" => {
                index += 1;
                let value = tokens
                    .get(index)
                    .ok_or_else(|| policy_error("Docker listing filter value is missing"))?;
                super::common::reject_shellish(value)?;
                filters.push(format!("--filter={value}"));
            }
            "--format" => {
                index += 1;
                let value = tokens
                    .get(index)
                    .ok_or_else(|| policy_error("Docker listing format value is missing"))?;
                if value.is_empty()
                    || value.len() > 4096
                    || value.chars().any(|ch| ch.is_control() && ch != '\t')
                {
                    return Err(policy_error("Docker listing format is invalid"));
                }
                super::common::reject_shellish(value)?;
                format = Some(value.clone());
            }
            value if value.starts_with("--filter=") => {
                super::common::reject_shellish(value)?;
                filters.push(value.to_owned());
            }
            value if value.starts_with("--format=") => {
                let value = value.trim_start_matches("--format=");
                if value.is_empty()
                    || value.len() > 4096
                    || value.chars().any(|ch| ch.is_control() && ch != '\t')
                {
                    return Err(policy_error("Docker listing format is invalid"));
                }
                super::common::reject_shellish(value)?;
                format = Some(value.to_owned());
            }
            _ => return Err(policy_error("Docker listing option is not allowed")),
        }
        index += 1;
    }
    let mut result = if image_listing {
        vec!["docker".into(), "images".into()]
    } else {
        vec!["docker".into(), "ps".into()]
    };
    if all {
        result.push("--all".into());
    }
    result.extend(filters);
    if quiet {
        result.push("--quiet".into());
    } else if let Some(format) = format {
        result.extend(["--format".into(), format]);
    } else if image_listing {
        result.extend([
            "--format".into(),
            "table {{.ID}}\t{{.Repository}}\t{{.Tag}}\t{{.CreatedSince}}\t{{.Size}}".into(),
        ]);
    } else {
        result.extend([
            "--format".into(),
            "table {{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}".into(),
        ]);
    }
    Ok(result)
}

fn logs(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() < 3 {
        return Err(policy_error("Docker logs requires a container"));
    }
    let mut tail: u64 = 200;
    let mut container: Option<String> = None;
    let mut index = 2;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-f" | "--follow" => return Err(policy_error("Docker log streaming is forbidden")),
            "--details" | "-t" | "--timestamps" => {}
            "--tail" => {
                index += 1;
                tail = parse_tail(tokens.get(index))?;
            }
            value if value.starts_with("--tail=") => {
                tail = parse_tail(Some(&value[7..].to_owned()))?;
            }
            value if value.starts_with('-') => {
                return Err(policy_error("Docker logs option is not allowed"))
            }
            value => {
                if container.replace(value.to_owned()).is_some() {
                    return Err(policy_error("Docker logs accepts exactly one container"));
                }
            }
        }
        index += 1;
    }
    let container = container.ok_or_else(|| policy_error("Docker logs requires a container"))?;
    validate_identifier(&container, "container")?;
    Ok(vec![
        "docker".into(),
        "logs".into(),
        "--tail".into(),
        tail.to_string(),
        container,
    ])
}

fn stats(tokens: &[String]) -> Result<Vec<String>, McpError> {
    let mut result = vec!["docker".into(), "stats".into(), "--no-stream".into()];
    for token in &tokens[2..] {
        if matches!(token.as_str(), "--no-stream" | "--no-trunc") {
            if token == "--no-trunc" {
                result.push(token.clone());
            }
        } else if token.starts_with('-') {
            return Err(policy_error(
                "Docker stats streaming/options are not allowed",
            ));
        } else {
            validate_identifier(token, "container")?;
            result.push(token.clone());
        }
    }
    Ok(result)
}

fn top(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() != 3 {
        return Err(policy_error(
            "Docker top accepts exactly one container; process format is relay-owned",
        ));
    }
    validate_identifier(&tokens[2], "container")?;
    Ok(vec![
        "docker".into(),
        "top".into(),
        tokens[2].clone(),
        "-eo".into(),
        "pid,ppid,user,stat,etime,comm".into(),
    ])
}

fn inspect(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() != 3 {
        return Err(policy_error(
            "Docker inspect accepts exactly one container in diagnostic mode",
        ));
    }
    validate_identifier(&tokens[2], "container")?;
    Ok(vec![
        "docker".into(),
        "inspect".into(),
        "--format".into(),
        "name={{.Name}} status={{.State.Status}} health={{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}} image={{.Image}} restart_count={{.RestartCount}}".into(),
        tokens[2].clone(),
    ])
}

fn exec(
    tokens: &[String],
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    if tokens.len() < 4 {
        return Err(policy_error(
            "Docker exec requires a container and nested command",
        ));
    }
    if tokens[2].starts_with('-') {
        return Err(policy_error(
            "Docker exec flags are forbidden in diagnostic mode",
        ));
    }
    validate_identifier(&tokens[2], "container")?;
    let nested = &tokens[3..];
    let normalized = match nested[0].as_str() {
        "psql" => db::psql(nested, readonly_db_user)?,
        "mysql" | "mariadb" => db::mysql(nested, readonly_db_user)?,
        "sqlite3" => db::sqlite(nested)?,
        "redis-cli" => db::redis(nested, readonly_redis_user)?,
        _ => super::validate_command_node(nested, readonly_db_user, readonly_redis_user)?,
    };
    let mut result = vec!["docker".into(), "exec".into()];
    if normalized.iter().any(|token| token == "redis-cli") {
        result.push("-i".into());
    }
    result.push(tokens[2].clone());
    result.extend(normalized);
    Ok(result)
}

fn compose(
    tokens: &[String],
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    if tokens.len() < 3 {
        return Err(policy_error(
            "Docker Compose diagnostic subcommand is required",
        ));
    }
    match tokens[2].as_str() {
        "ps" => compose_ps(tokens),
        "logs" => compose_logs(tokens),
        "config" => compose_config(tokens),
        "exec" => compose_exec(tokens, readonly_db_user, readonly_redis_user),
        "up" | "down" | "restart" | "start" | "stop" | "kill" | "rm" | "run" | "build" | "pull"
        | "push" | "create" => Err(policy_error(
            "Docker Compose mutation is forbidden in diagnostic mode",
        )),
        _ => Err(policy_error(
            "Docker Compose subcommand is not in the diagnostic allowlist",
        )),
    }
}

fn compose_exec(
    tokens: &[String],
    readonly_db_user: Option<&str>,
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    if tokens.len() < 5 {
        return Err(policy_error(
            "Docker Compose exec requires a service and nested command",
        ));
    }
    let mut index = 3usize;
    let mut result = vec![
        "docker".into(),
        "compose".into(),
        "exec".into(),
        "-T".into(),
    ];
    while index < tokens.len() && tokens[index].starts_with('-') {
        match tokens[index].as_str() {
            "-T" | "--no-TTY" => {}
            _ => {
                return Err(policy_error(
                    "Docker Compose exec option is not allowed in diagnostic mode",
                ))
            }
        }
        index += 1;
    }
    let service = tokens
        .get(index)
        .ok_or_else(|| policy_error("Docker Compose exec service is missing"))?;
    validate_identifier(service, "service")?;
    result.push(service.clone());
    index += 1;
    let nested = tokens
        .get(index..)
        .filter(|nested| !nested.is_empty())
        .ok_or_else(|| policy_error("Docker Compose exec nested command is missing"))?;
    let normalized = match nested[0].as_str() {
        "psql" => db::psql(nested, readonly_db_user)?,
        "mysql" | "mariadb" => db::mysql(nested, readonly_db_user)?,
        "sqlite3" => db::sqlite(nested)?,
        "redis-cli" => db::redis(nested, readonly_redis_user)?,
        _ => super::validate_command_node(nested, readonly_db_user, readonly_redis_user)?,
    };
    result.extend(normalized);
    Ok(result)
}

fn compose_ps(tokens: &[String]) -> Result<Vec<String>, McpError> {
    for token in &tokens[3..] {
        if token.starts_with('-')
            && !matches!(
                token.as_str(),
                "-a" | "--all" | "--services" | "-q" | "--quiet"
            )
        {
            return Err(policy_error("Docker Compose ps option is not allowed"));
        }
    }
    Ok(tokens.to_vec())
}

fn compose_logs(tokens: &[String]) -> Result<Vec<String>, McpError> {
    let mut tail: u64 = 200;
    let mut services = Vec::new();
    let mut index = 3;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-f" | "--follow" => {
                return Err(policy_error("Docker Compose log streaming is forbidden"))
            }
            "-t" | "--timestamps" | "--no-color" => {}
            "--tail" => {
                index += 1;
                tail = parse_tail(tokens.get(index))?;
            }
            value if value.starts_with("--tail=") => {
                let owned = value[7..].to_owned();
                tail = parse_tail(Some(&owned))?;
            }
            value if value.starts_with('-') => {
                return Err(policy_error("Docker Compose logs option is not allowed"))
            }
            value => {
                validate_identifier(value, "service")?;
                services.push(value.to_owned());
            }
        }
        index += 1;
    }
    let mut result = vec![
        "docker".into(),
        "compose".into(),
        "logs".into(),
        "--tail".into(),
        tail.to_string(),
        "--no-color".into(),
    ];
    result.extend(services);
    Ok(result)
}

fn compose_config(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() != 4
        || !matches!(
            tokens[3].as_str(),
            "--services" | "--images" | "--profiles" | "--volumes" | "--hash" | "--quiet" | "-q"
        )
    {
        return Err(policy_error(
            "full Docker Compose config output is not confidentiality-safe",
        ));
    }
    Ok(tokens.to_vec())
}

fn parse_tail(value: Option<&String>) -> Result<u64, McpError> {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0 && *value <= MAX_LOG_TAIL)
        .ok_or_else(|| policy_error("Docker log tail exceeds allowed bounds"))
}
