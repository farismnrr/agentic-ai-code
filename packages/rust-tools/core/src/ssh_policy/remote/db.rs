use super::super::policy_error;
use super::common::{path_looks_sensitive, validate_identifier};
use crate::error::McpError;

pub(super) fn psql(
    tokens: &[String],
    readonly_db_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    let user = readonly_db_user.ok_or_else(|| {
        policy_error("PostgreSQL diagnostics require a configured read-only database principal")
    })?;
    validate_db_user(user)?;
    let mut database: Option<String> = None;
    let mut query: Option<String> = None;
    let mut index = 1;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-d" | "--dbname" => {
                index += 1;
                database = Some(
                    tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| policy_error("psql database is missing"))?,
                );
            }
            "-c" | "--command" => {
                index += 1;
                query = Some(
                    tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| policy_error("psql query is missing"))?,
                );
            }
            value if value.starts_with('-') => {
                return Err(policy_error(
                    "psql option is not allowed in diagnostic mode",
                ))
            }
            _ => {
                return Err(policy_error(
                    "psql positional arguments are not allowed in diagnostic mode",
                ))
            }
        }
        index += 1;
    }
    let query = query.ok_or_else(|| policy_error("psql requires exactly one query"))?;
    validate_sql_readonly(&query)?;
    let mut result = vec![
        "psql".into(),
        "-X".into(),
        "-w".into(),
        "-v".into(),
        "ON_ERROR_STOP=1".into(),
        "-U".into(),
        user.into(),
    ];
    if let Some(database) = database {
        validate_identifier(&database, "database")?;
        result.extend(["-d".into(), database]);
    }
    result.extend([
        "-c".into(),
        format!("SET statement_timeout='10s'; BEGIN READ ONLY; {query}; ROLLBACK"),
    ]);
    Ok(result)
}

pub(super) fn mysql(
    tokens: &[String],
    readonly_db_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    let user = readonly_db_user.ok_or_else(|| {
        policy_error("MySQL diagnostics require a configured read-only database principal")
    })?;
    validate_db_user(user)?;
    let mut database: Option<String> = None;
    let mut query: Option<String> = None;
    let mut index = 1;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-D" | "--database" => {
                index += 1;
                database = Some(
                    tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| policy_error("MySQL database is missing"))?,
                );
            }
            "-e" | "--execute" => {
                index += 1;
                query = Some(
                    tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| policy_error("MySQL query is missing"))?,
                );
            }
            value if value.starts_with('-') => {
                return Err(policy_error(
                    "MySQL option is not allowed in diagnostic mode",
                ))
            }
            _ => {
                return Err(policy_error(
                    "MySQL positional arguments are not allowed in diagnostic mode",
                ))
            }
        }
        index += 1;
    }
    let query = query.ok_or_else(|| policy_error("MySQL requires exactly one query"))?;
    validate_sql_readonly(&query)?;
    let mut result = vec![
        tokens[0].clone(),
        "--batch".into(),
        "--raw".into(),
        "--skip-column-names".into(),
        "--skip-password".into(),
        "-u".into(),
        user.into(),
    ];
    if let Some(database) = database {
        validate_identifier(&database, "database")?;
        result.extend(["-D".into(), database]);
    }
    result.extend([
        "-e".into(),
        format!("START TRANSACTION READ ONLY; {query}; ROLLBACK"),
    ]);
    Ok(result)
}

pub(super) fn sqlite(tokens: &[String]) -> Result<Vec<String>, McpError> {
    if tokens.len() != 3 {
        return Err(policy_error(
            "sqlite3 diagnostics require a database path and one query",
        ));
    }
    if path_looks_sensitive(&tokens[1]) {
        return Err(policy_error(
            "SQLite diagnostic path is confidentiality-sensitive",
        ));
    }
    validate_sql_readonly(&tokens[2])?;
    Ok(vec![
        "sqlite3".into(),
        "-readonly".into(),
        tokens[1].clone(),
        tokens[2].clone(),
    ])
}

pub(super) fn redis(
    tokens: &[String],
    readonly_redis_user: Option<&str>,
) -> Result<Vec<String>, McpError> {
    let user = readonly_redis_user
        .ok_or_else(|| policy_error("Redis diagnostics require a configured read-only ACL user"))?;
    validate_db_user(user)?;
    if tokens.len() < 2 {
        return Err(policy_error("redis-cli requires a read-only command"));
    }
    if tokens[1].starts_with('-') {
        return Err(policy_error(
            "redis-cli connection/credential options are relay-owned",
        ));
    }
    let command = tokens[1].to_ascii_uppercase();
    if !matches!(
        command.as_str(),
        "GET"
            | "MGET"
            | "HGET"
            | "HMGET"
            | "HGETALL"
            | "TTL"
            | "PTTL"
            | "TYPE"
            | "EXISTS"
            | "LLEN"
            | "LRANGE"
            | "SCARD"
            | "SMEMBERS"
            | "ZRANGE"
            | "ZCARD"
            | "INFO"
            | "PING"
    ) {
        return Err(policy_error(
            "Redis command is not in the bounded read-only allowlist",
        ));
    }
    if tokens.len() > 20 {
        return Err(policy_error(
            "Redis diagnostic arguments exceed allowed bounds",
        ));
    }
    let mut result = vec![
        "redis-cli".into(),
        "--user".into(),
        user.into(),
        "--no-auth-warning".into(),
    ];
    result.extend(tokens[1..].iter().cloned());
    Ok(result)
}

fn validate_db_user(value: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|ch| !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.'))
    {
        return Err(policy_error(
            "configured read-only database principal is invalid",
        ));
    }
    Ok(())
}

fn validate_sql_readonly(query: &str) -> Result<(), McpError> {
    let words = sql_words(query)?;
    if words.is_empty() {
        return Err(policy_error("database query is empty"));
    }
    if !matches!(
        words[0].as_str(),
        "SELECT" | "WITH" | "EXPLAIN" | "SHOW" | "DESCRIBE" | "DESC"
    ) {
        return Err(policy_error("database query is not read-only"));
    }
    const FORBIDDEN: &[&str] = &[
        "INSERT", "UPDATE", "DELETE", "MERGE", "UPSERT", "REPLACE", "CREATE", "ALTER", "DROP",
        "TRUNCATE", "GRANT", "REVOKE", "COPY", "CALL", "DO", "EXEC", "EXECUTE", "VACUUM",
        "ANALYZE", "REINDEX", "CLUSTER", "REFRESH", "LOCK", "UNLOCK", "SET", "RESET", "DISCARD",
        "LISTEN", "NOTIFY", "UNLISTEN", "LOAD", "ATTACH", "DETACH", "PRAGMA", "INTO",
    ];
    if words.iter().any(|word| FORBIDDEN.contains(&word.as_str())) {
        return Err(policy_error(
            "database query contains a mutating or ambiguous statement",
        ));
    }
    const DANGEROUS_FUNCTIONS: &[&str] = &[
        "PG_SLEEP",
        "PG_TERMINATE_BACKEND",
        "PG_CANCEL_BACKEND",
        "PG_RELOAD_CONF",
        "PG_ROTATE_LOGFILE",
        "PG_CREATE_PHYSICAL_REPLICATION_SLOT",
        "PG_CREATE_LOGICAL_REPLICATION_SLOT",
        "PG_DROP_REPLICATION_SLOT",
        "PG_WRITE_FILE",
        "PG_FILE_WRITE",
        "LO_EXPORT",
        "LO_IMPORT",
        "DBLINK_EXEC",
        "SLEEP",
        "BENCHMARK",
        "LOAD_FILE",
    ];
    if words
        .iter()
        .any(|word| DANGEROUS_FUNCTIONS.contains(&word.as_str()))
    {
        return Err(policy_error("database query contains a dangerous function"));
    }
    Ok(())
}

fn sql_words(query: &str) -> Result<Vec<String>, McpError> {
    if query.len() > 16 * 1024 || query.contains('\0') {
        return Err(policy_error("database query exceeds allowed bounds"));
    }
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = query.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        if let Some(active) = quote {
            if ch == active {
                if chars.peek() == Some(&active) {
                    chars.next();
                } else {
                    quote = None;
                }
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            flush_word(&mut words, &mut current);
            quote = Some(ch);
            continue;
        }
        if ch == '-' && chars.peek() == Some(&'-') {
            flush_word(&mut words, &mut current);
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            flush_word(&mut words, &mut current);
            chars.next();
            let mut closed = false;
            let mut previous = '\0';
            for next in chars.by_ref() {
                if previous == '*' && next == '/' {
                    closed = true;
                    break;
                }
                previous = next;
            }
            if !closed {
                return Err(policy_error(
                    "database query contains an unterminated comment",
                ));
            }
            continue;
        }
        if ch == ';' {
            flush_word(&mut words, &mut current);
            if chars.clone().any(|next| !next.is_ascii_whitespace()) {
                return Err(policy_error("multiple database statements are forbidden"));
            }
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_uppercase());
        } else {
            flush_word(&mut words, &mut current);
        }
    }
    if quote.is_some() {
        return Err(policy_error("database query quoting is malformed"));
    }
    flush_word(&mut words, &mut current);
    Ok(words)
}

fn flush_word(words: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}
