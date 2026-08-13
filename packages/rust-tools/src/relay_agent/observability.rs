use axum::extract::Request;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use serde_json::json;
use std::time::Instant;

const MAX_LOG_FIELD: usize = 128;
const HDR_CORRELATION_ID: &str = "x-correlation-id";

pub fn safe_log_field(value: &str) -> String {
    value
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(String);

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
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Self(id)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn insert_response_header(&self, response: &mut axum::response::Response) {
        if let Ok(value) = self.as_str().parse() {
            response
                .headers_mut()
                .insert(HeaderName::from_static(HDR_CORRELATION_ID), value);
        }
    }
}

pub fn audit(
    correlation_id: &str,
    method: &str,
    tool: Option<&str>,
    outcome: &str,
    status: StatusCode,
    started: Instant,
    subject: Option<&str>,
) {
    eprintln!(
        "{}",
        json!({"event":"relay_request","correlation_id":safe_log_field(correlation_id),
        "method":safe_log_field(method),"tool":privacy_id(tool),"outcome":safe_log_field(outcome),
        "status":status.as_u16(),"latency_ms":started.elapsed().as_millis(),"subject":privacy_id(subject)})
    );
}
