use ai_tools::infrastructure::observability::{redact_secrets, safe_log_field};

const CANARY: &str = "canary-secret-fake-token-DO-NOT-LEAK-12345";

#[test]
fn safe_log_field_redacts_bearer_tokens() {
    let output = safe_log_field(&format!("Authorization: Bearer {CANARY}"));
    assert!(!output.contains(CANARY));
    assert!(output.contains("Bearer [REDACTED]"));
}

#[test]
fn redact_secrets_masks_database_userinfo() {
    let output = redact_secrets(&format!("postgres://user:{CANARY}@localhost/db"));
    assert!(!output.contains(CANARY));
    assert!(output.contains("postgres://[REDACTED]@localhost/db"));
}

#[test]
fn redact_secrets_masks_api_key_assignments() {
    let output = redact_secrets(&format!("x-api-key={CANARY}"));
    assert!(!output.contains(CANARY));
    assert!(output.contains("x-api-key=[REDACTED]"));
}
