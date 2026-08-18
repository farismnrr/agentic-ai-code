//! Opaque, signed continuation claims shared by deterministic relay tools.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use relay_core::{config::ServerConfig, error::McpError};
use ring::hmac;
use serde_json::{Map, Value};

const MAX_TOTAL: usize = 1000;
pub(crate) const MAX_TOTAL_ENTRIES: usize = MAX_TOTAL;
const MAX_TOKEN_BYTES: usize = 4096;
const TTL_MS: u64 = 5 * 60 * 1000;

pub(crate) fn paginate<T>(
    arguments: &Value,
    items: Vec<T>,
    default_limit: usize,
    config: &ServerConfig,
    tool: &str,
    scope: &str,
    snapshot: Option<&str>,
) -> Result<(Vec<T>, Option<String>), McpError> {
    let limit = bounded_limit(arguments, default_limit)?;
    let query = canonical_query(arguments)?;
    let now = now_ms();
    let (offset, retrieved, expiry) = match arguments.get("continuation").and_then(Value::as_str) {
        Some(token) => decode_claim(token, config, tool, &query, scope, limit, snapshot, now)?,
        None => (0, 0, now.saturating_add(TTL_MS)),
    };
    if offset > items.len() || retrieved.saturating_add(limit) > MAX_TOTAL {
        return Err(invalid_token());
    }
    let total = items.len();
    let page = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_offset = offset.saturating_add(page.len());
    let next = if next_offset < total {
        Some(encode_claim(
            config,
            tool,
            &query,
            scope,
            limit,
            next_offset,
            retrieved.saturating_add(page.len()),
            expiry,
            snapshot,
        )?)
    } else {
        None
    };
    Ok((page, next))
}

fn bounded_limit(arguments: &Value, default_limit: usize) -> Result<usize, McpError> {
    match arguments
        .get("max_results")
        .or_else(|| arguments.get("max_entries"))
    {
        None => Ok(default_limit.min(128)),
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| (1..=128).contains(v))
            .ok_or_else(invalid_token),
        _ => Err(invalid_token()),
    }
}

fn canonical_query(arguments: &Value) -> Result<String, McpError> {
    let mut value = arguments.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("continuation");
    }
    serde_json::to_string(&canonical_value(&value)).map_err(|_| invalid_token())
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = Map::new();
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in entries {
                sorted.insert(key.clone(), canonical_value(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_claim(
    config: &ServerConfig,
    tool: &str,
    query: &str,
    scope: &str,
    limit: usize,
    offset: usize,
    retrieved: usize,
    expires_at: u64,
    snapshot: Option<&str>,
) -> Result<String, McpError> {
    let mut claim = Map::new();
    claim.insert("v".into(), Value::from(1));
    claim.insert("tool".into(), Value::from(tool));
    claim.insert("query".into(), Value::from(query));
    claim.insert("scope".into(), Value::from(scope));
    claim.insert("limit".into(), Value::from(limit as u64));
    claim.insert("offset".into(), Value::from(offset as u64));
    claim.insert("retrieved".into(), Value::from(retrieved as u64));
    claim.insert("expires_at".into(), Value::from(expires_at));
    if let Some(snapshot) = snapshot {
        claim.insert("snapshot".into(), Value::from(snapshot));
    }
    let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claim).map_err(|_| invalid_token())?);
    let tag = URL_SAFE_NO_PAD.encode(sign(config, body.as_bytes())?);
    Ok(format!("{body}.{tag}"))
}

#[allow(clippy::too_many_arguments)]
fn decode_claim(
    token: &str,
    config: &ServerConfig,
    tool: &str,
    query: &str,
    scope: &str,
    limit: usize,
    snapshot: Option<&str>,
    now: u64,
) -> Result<(usize, usize, u64), McpError> {
    if token.len() > MAX_TOKEN_BYTES || token.split('.').count() != 2 {
        return Err(invalid_token());
    }
    let mut parts = token.split('.');
    let body = parts.next().unwrap_or("");
    let tag = parts.next().unwrap_or("");
    let expected = sign(config, body.as_bytes())?;
    let actual = URL_SAFE_NO_PAD.decode(tag).map_err(|_| invalid_token())?;
    if !constant_time_equal(&expected, &actual) {
        return Err(invalid_token());
    }
    let object = serde_json::from_slice::<Value>(
        &URL_SAFE_NO_PAD.decode(body).map_err(|_| invalid_token())?,
    )
    .map_err(|_| invalid_token())?
    .as_object()
    .cloned()
    .ok_or_else(invalid_token)?;
    let allowed = [
        "v",
        "tool",
        "query",
        "scope",
        "limit",
        "offset",
        "retrieved",
        "expires_at",
        "snapshot",
    ];
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || object.get("v") != Some(&Value::from(1))
    {
        return Err(invalid_token());
    }
    let claim_snapshot = object.get("snapshot").and_then(Value::as_str);
    let expiry = object
        .get("expires_at")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_token)?;
    let token_limit = object
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(invalid_token)?;
    let offset = object
        .get("offset")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(invalid_token)?;
    let retrieved = object
        .get("retrieved")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(invalid_token)?;
    if object.get("tool").and_then(Value::as_str) != Some(tool)
        || object.get("query").and_then(Value::as_str) != Some(query)
        || object.get("scope").and_then(Value::as_str) != Some(scope)
        || token_limit != limit
        || claim_snapshot != snapshot
        || expiry <= now
        || retrieved.saturating_add(token_limit) > MAX_TOTAL
    {
        return Err(invalid_token());
    }
    Ok((offset, retrieved, expiry))
}

fn sign(config: &ServerConfig, body: &[u8]) -> Result<Vec<u8>, McpError> {
    let configured = config
        .oauth_secret
        .as_deref()
        .filter(|value| !value.is_empty());
    let secret = match configured {
        Some(value) => value,
        None if cfg!(debug_assertions) => "039h-debug-only-continuation-key",
        None => {
            return Err(McpError::Internal(
                "continuation signing key is not configured".into(),
            ))
        }
    };
    Ok(
        hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes()), body)
            .as_ref()
            .to_vec(),
    )
}
fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .fold(0u8, |result, (left, right)| result | (left ^ right))
            == 0
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
fn invalid_token() -> McpError {
    McpError::InvalidRequest("invalid continuation token".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::config::ServerConfig;
    use serde_json::json;

    fn config() -> ServerConfig {
        ServerConfig {
            oauth_secret: Some("test-continuation-key".into()),
            ..Default::default()
        }
    }

    #[test]
    fn signed_cursor_rejects_mutation_and_accepts_reordered_query() {
        let config = config();
        let first = json!({"query":"needle","cwd":"/repo","max_results":1});
        let (_, token) = paginate(
            &first,
            vec![1, 2],
            1,
            &config,
            "text_search",
            "/repo",
            Some("snapshot"),
        )
        .unwrap();
        let token = token.unwrap();
        let reordered =
            json!({"max_results":1,"cwd":"/repo","query":"needle","continuation":token});
        assert!(paginate(
            &reordered,
            vec![1, 2],
            1,
            &config,
            "text_search",
            "/repo",
            Some("snapshot")
        )
        .is_ok());
        let mut parts = token.split('.');
        let body = URL_SAFE_NO_PAD.decode(parts.next().unwrap()).unwrap();
        let mut value: Value = serde_json::from_slice(&body).unwrap();
        value["offset"] = json!(0);
        let mutated_body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap());
        let mutated = format!("{mutated_body}.{}", parts.next().unwrap());
        let mut args = first;
        args["continuation"] = Value::String(mutated);
        assert!(paginate(
            &args,
            vec![1, 2],
            1,
            &config,
            "text_search",
            "/repo",
            Some("snapshot")
        )
        .is_err());
    }

    #[test]
    fn cursor_expiry_and_scope_are_bound() {
        let config = config();
        let token = encode_claim(
            &config,
            "tool",
            "{}",
            "/repo",
            1,
            1,
            1,
            now_ms().saturating_sub(1),
            Some("v1"),
        )
        .unwrap();
        assert!(decode_claim(
            &token,
            &config,
            "tool",
            "{}",
            "/repo",
            1,
            Some("v1"),
            now_ms()
        )
        .is_err());
        let token = encode_claim(
            &config,
            "tool",
            "{}",
            "/repo",
            1,
            1,
            1,
            now_ms() + 1000,
            Some("v1"),
        )
        .unwrap();
        assert!(decode_claim(
            &token,
            &config,
            "tool",
            "{}",
            "/other",
            1,
            Some("v1"),
            now_ms()
        )
        .is_err());
    }
}
