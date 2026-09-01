use ai_tools::application::continuation::{decode_claim, encode_claim, paginate};
use ai_tools::core::config::ServerConfig;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{json, Value};

fn config() -> ServerConfig {
    ServerConfig {
        oauth_secret: Some("test-continuation-key".into()),
        ..ServerConfig::default()
    }
}

fn current_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[test]
fn continuation_works_without_oauth_secret() {
    let config = ServerConfig::default();
    let first = json!({"query":"needle","cwd":"/repo","max_results":1});
    let (page, token) = paginate(
        &first,
        vec![1, 2],
        1,
        &config,
        "text_search",
        "/repo",
        Some("snapshot"),
    )
    .unwrap();
    assert_eq!(page, vec![1]);
    let mut next = first;
    next["continuation"] = Value::String(token.unwrap());
    let (page, token) = paginate(
        &next,
        vec![1, 2],
        1,
        &config,
        "text_search",
        "/repo",
        Some("snapshot"),
    )
    .unwrap();
    assert_eq!(page, vec![2]);
    assert!(token.is_none());
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
    let reordered = json!({"max_results":1,"cwd":"/repo","query":"needle","continuation":token});
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
    let now = current_ms();
    let token = encode_claim(
        &config,
        "tool",
        "{}",
        "/repo",
        1,
        1,
        1,
        now.saturating_sub(1),
        Some("v1"),
    )
    .unwrap();
    assert!(decode_claim(&token, &config, "tool", "{}", "/repo", 1, Some("v1"), now).is_err());
    let token = encode_claim(
        &config,
        "tool",
        "{}",
        "/repo",
        1,
        1,
        1,
        now + 1000,
        Some("v1"),
    )
    .unwrap();
    assert!(decode_claim(&token, &config, "tool", "{}", "/other", 1, Some("v1"), now).is_err());
}
