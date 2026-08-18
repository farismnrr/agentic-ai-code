use relay_core::error::McpError;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const MAX_TOTAL: usize = 1000;

pub(crate) fn paginate<T>(
    arguments: &Value,
    mut items: Vec<T>,
    default_limit: usize,
) -> Result<(Vec<T>, Option<String>), McpError> {
    let max_results = arguments
        .get("max_results")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .filter(|v| *v > 0)
        .unwrap_or(default_limit)
        .min(128);
    let query_hash = continuation_hash(arguments);
    let offset = match arguments.get("continuation").and_then(Value::as_str) {
        Some(token) => {
            let mut fields = token.split('.');
            if fields.next() != Some("v1") {
                return Err(McpError::InvalidRequest(
                    "invalid continuation token".into(),
                ));
            }
            let offset = fields.next().and_then(|v| v.parse::<usize>().ok());
            let token_limit = fields.next().and_then(|v| v.parse::<usize>().ok());
            let token_hash = fields.next();
            if fields.next().is_some()
                || token_limit != Some(max_results)
                || token_hash != Some(query_hash.as_str())
            {
                return Err(McpError::InvalidRequest(
                    "invalid continuation token".into(),
                ));
            }
            offset.ok_or_else(|| McpError::InvalidRequest("invalid continuation token".into()))?
        }
        None => 0,
    };
    if offset > items.len() || offset.saturating_add(max_results) > MAX_TOTAL {
        return Err(McpError::InvalidRequest(
            "invalid continuation token".into(),
        ));
    }
    let total = items.len();
    let page: Vec<T> = items.drain(offset..).take(max_results).collect();
    let next_offset = offset + page.len();
    let next =
        (next_offset < total).then(|| format!("v1.{next_offset}.{max_results}.{query_hash}"));
    Ok((page, next))
}

fn continuation_hash(arguments: &Value) -> String {
    let mut stable = arguments.clone();
    if let Some(object) = stable.as_object_mut() {
        object.remove("continuation");
    }
    let encoded = serde_json::to_string(&stable).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
