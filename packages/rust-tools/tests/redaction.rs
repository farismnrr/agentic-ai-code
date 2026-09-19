use ai_tools::infrastructure::observability::{redact_secrets, safe_log_field};
use std::time::{Duration, Instant};

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

#[test]
fn redact_secrets_preserves_large_base64ish_input_with_bounded_runtime() {
    let input = "A".repeat(2 * 1024 * 1024);
    let started = Instant::now();
    let output = redact_secrets(&input);

    assert_eq!(output, input);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "large credential-free input should not trigger quadratic scanning"
    );
}
