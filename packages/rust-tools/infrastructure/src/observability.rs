use axum::extract::Request;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use serde_json::json;
use std::time::Instant;

const MAX_LOG_FIELD: usize = 128;
const HDR_CORRELATION_ID: &str = "x-correlation-id";
const HDR_REQUEST_ID: &str = "x-request-id";

/// P1 value-level secret redaction (Plan 035 remediation round 2), kept
/// policy-aligned with `server/infrastructure/observability/sanitize.ts`'s
/// `redactSecrets`: same category list, same "mask the credential-shaped
/// substring, keep surrounding context" approach. Deterministic regex-free
/// scanning only (no `regex` crate dependency needed) — each category is a
/// small hand-rolled scan so this stays a focused function, not a generic
/// scanner framework. Applied BEFORE truncation in `safe_log_field` so a
/// credential isn't accidentally left half-truncated-but-unmasked.
pub fn redact_secrets(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0usize;

    // Case-insensitive prefix match at position i.
    let starts_with_ci = |s: &str, at: usize, needle: &str| -> bool {
        let end = at + needle.len();
        end <= s.len() && s.as_bytes()[at..end].eq_ignore_ascii_case(needle.as_bytes())
    };

    // Consume a "token" of URL-safe / base64-ish characters starting at i.
    let token_end = |s: &str, start: usize| -> usize {
        let mut j = start;
        for c in s[start..].chars() {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=' | '~') {
                j += c.len_utf8();
            } else {
                break;
            }
        }
        j
    };

    while i < bytes.len() {
        // Bearer <token>
        if starts_with_ci(value, i, "Bearer ") {
            out.push_str("Bearer [REDACTED]");
            let mut j = i + 7;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            i = token_end(value, j);
            continue;
        }
        // Basic <token>
        if starts_with_ci(value, i, "Basic ") {
            out.push_str("Basic [REDACTED]");
            let mut j = i + 6;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            i = token_end(value, j);
            continue;
        }
        // URL userinfo credentials: scheme://user:pass@host (incl. DB
        // connection strings: postgres/postgresql/mysql/mongodb/redis).
        if let Some(scheme_end) = value[i..].find("://") {
            let scheme_end = i + scheme_end;
            let is_scheme_start = i == 0 || !value.as_bytes()[i - 1].is_ascii_alphanumeric();
            let scheme_ok = value[i..scheme_end]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
                && !value[i..scheme_end].is_empty()
                && is_scheme_start;
            if scheme_ok {
                let rest = &value[scheme_end + 3..];
                if let Some(at_pos) = rest.find('@') {
                    let userinfo = &rest[..at_pos];
                    let no_slash = !userinfo.contains('/');
                    let has_colon = userinfo.contains(':');
                    if no_slash && has_colon && !userinfo.is_empty() {
                        out.push_str(&value[i..scheme_end]);
                        out.push_str("://[REDACTED]@");
                        i = scheme_end + 3 + at_pos + 1;
                        continue;
                    }
                }
            }
        }
        // key=value / key: value style credentials.
        const KEYS: &[&str] = &[
            "x-api-key",
            "api-key",
            "api_key",
            "apikey",
            "cookie",
            "session",
            "password",
            "passwd",
            "token",
            "secret",
            "access-key",
            "access_key",
            "client-secret",
            "client_secret",
            "key",
        ];
        let mut matched_key = false;
        for k in KEYS {
            if starts_with_ci(value, i, k) {
                let after = i + k.len();
                let mut j = after;
                while j < bytes.len() && bytes[j] == b' ' {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b':') {
                    j += 1;
                    while j < bytes.len() && bytes[j] == b' ' {
                        j += 1;
                    }
                    let mut end = j;
                    for c in value[j..].chars() {
                        if c.is_whitespace() || c == '\'' || c == '"' || c == ',' || c == ';' {
                            break;
                        }
                        end += c.len_utf8();
                    }
                    if end > j {
                        out.push_str(k);
                        out.push_str("=[REDACTED]");
                        i = end;
                        matched_key = true;
                        break;
                    }
                }
            }
        }
        if matched_key {
            continue;
        }
        // JWT-like values: eyJ....X.Y (header.payload.signature)
        if starts_with_ci(value, i, "eyJ") {
            let end = token_end_jwt(value, i);
            if end > i {
                out.push_str("[REDACTED-JWT]");
                i = end;
                continue;
            }
        }

        // No pattern matched at this position: copy one char forward.
        let ch = value[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Consumes a JWT-shaped `header.payload.signature` token (three
/// dot-separated base64url segments) starting at `start`, if one is
/// actually present; returns `start` (no match) otherwise.
fn token_end_jwt(value: &str, start: usize) -> usize {
    let is_b64url = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let mut j = start;
    let mut dots = 0u8;
    for c in value[start..].chars() {
        if c == '.' {
            dots += 1;
            if dots > 2 {
                break;
            }
            j += 1;
        } else if is_b64url(c) {
            j += c.len_utf8();
        } else {
            break;
        }
    }
    if dots == 2 {
        j
    } else {
        start
    }
}

pub fn safe_log_field(value: &str) -> String {
    redact_secrets(value)
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_LOG_FIELD)
        .collect()
}

pub fn privacy_id(value: Option<&str>) -> &'static str {
    if value.is_some() {
        "present"
    } else {
        "absent"
    }
}

/// A client-supplied `x-correlation-id` header value, retained ONLY as
/// untrusted optional metadata (Plan 035 Phase 9). It is bounded/sanitized on
/// extraction, may be attached to private telemetry for support correlation,
/// and is NEVER treated as the authoritative request identity and NEVER set
/// on the response — see [`RequestId`] for the server-generated authoritative
/// identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(Option<String>);

impl CorrelationId {
    pub fn from_request(req: &Request) -> Self {
        Self::from_headers(req.headers())
    }
    fn from_headers(headers: &HeaderMap) -> Self {
        let id = headers
            .get(HDR_CORRELATION_ID)
            .and_then(|value| value.to_str().ok())
            .filter(|value| {
                value.len() <= MAX_LOG_FIELD
                    && value.chars().all(|c| c.is_ascii_graphic() && c != '"')
            })
            .map(str::to_owned);
        Self(id)
    }
    /// The sanitized client hint, if the caller supplied a well-formed one.
    /// Untrusted metadata only — never use as an authoritative identity.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// Server-generated request identity (Plan 035 Phase 9). This is the
/// authoritative correlation value for one inbound request: it is always
/// generated by this process (never taken from client input), returned to
/// the client on every response via the `x-request-id` header, and safe to
/// echo back in a generic 5xx body so a client can hand it to support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestId(String);

impl RequestId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn insert_response_header(&self, response: &mut axum::response::Response) {
        if let Ok(value) = self.as_str().parse() {
            response
                .headers_mut()
                .insert(HeaderName::from_static(HDR_REQUEST_ID), value);
        }
    }
}

/// Emits the `relay.request` audit event to stderr (and, when the OTel
/// bridge from `telemetry.rs` is active, into the current tracing span as a
/// structured event). `request_id` is the server-generated authoritative
/// identity; `client_hint` is the untrusted, optional client-supplied
/// correlation metadata (never authoritative, never returned to the client).
#[allow(clippy::too_many_arguments)]
pub fn audit(
    request_id: &str,
    client_hint: Option<&str>,
    method: &str,
    tool: Option<&str>,
    outcome: &str,
    status: StatusCode,
    started: Instant,
    subject: Option<&str>,
) {
    let latency_ms = started.elapsed().as_millis();
    tracing::info!(
        event = "relay.request",
        request_id = safe_log_field(request_id),
        method = safe_log_field(method),
        tool = privacy_id(tool),
        outcome = safe_log_field(outcome),
        status = status.as_u16(),
        latency_ms = latency_ms as u64,
        subject = privacy_id(subject),
    );
    eprintln!(
        "{}",
        json!({"event":"relay.request","request_id":safe_log_field(request_id),
        "client_hint":client_hint.map(safe_log_field),
        "method":safe_log_field(method),"tool":privacy_id(tool),"outcome":safe_log_field(outcome),
        "status":status.as_u16(),"latency_ms":latency_ms,"subject":privacy_id(subject)})
    );
}

#[cfg(test)]
mod redact_tests {
    use super::*;

    #[test]
    fn redacts_bearer_token() {
        let out =
            safe_log_field("Authorization: Bearer canary-secret-fake-token-DO-NOT-LEAK-12345");
        assert!(!out.contains("canary-secret-fake-token-DO-NOT-LEAK-12345"));
        assert!(out.contains("Bearer [REDACTED]"));
    }

    #[test]
    fn redacts_db_url_userinfo() {
        let out = redact_secrets(
            "postgres://user:canary-secret-fake-token-DO-NOT-LEAK-12345@localhost/db",
        );
        assert!(!out.contains("canary-secret-fake-token-DO-NOT-LEAK-12345"));
        assert!(out.contains("postgres://[REDACTED]@localhost/db"));
    }

    #[test]
    fn redacts_api_key_assignment() {
        let out = redact_secrets("x-api-key=canary-secret-fake-token-DO-NOT-LEAK-12345");
        assert!(!out.contains("canary-secret-fake-token-DO-NOT-LEAK-12345"));
        assert!(out.contains("[REDACTED]"));
    }
}
