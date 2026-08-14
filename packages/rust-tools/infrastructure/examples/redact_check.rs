// Deterministic black-box acceptance helper (not a unit test — this repo has
// no unit-test-suite policy). Prints the result of `redact_secrets` /
// `safe_log_field` applied to a CLI argument so shell-level acceptance
// scripts (scripts/verify-value-level-secret-redaction.mjs) can assert
// against real, compiled redaction behavior via `cargo run --example`.
//
// Usage: redact_check <safe_log_field|redact_secrets> <input>
use relay_infrastructure::observability::{redact_secrets, safe_log_field};

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    let input = args.next().unwrap_or_default();

    let output = match mode.as_str() {
        "safe_log_field" => safe_log_field(&input),
        "redact_secrets" => redact_secrets(&input),
        _ => {
            eprintln!("usage: redact_check <safe_log_field|redact_secrets> <input>");
            std::process::exit(2);
        }
    };

    println!("{output}");
}
