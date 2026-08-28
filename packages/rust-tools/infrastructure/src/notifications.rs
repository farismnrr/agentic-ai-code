//! Outbound task-completion notifications.
//!
//! Telegram is deliberately an implementation detail of the relay. This
//! module exposes one task-completion enqueue operation, a fixed configured
//! recipient, and a durable deduplication ledger; it does not expose a
//! generic Telegram or HTTP tool to MCP callers.

use relay_core::config::ServerConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

mod ledger;
mod telegram;
pub use ledger::{ClaimedNotification, NotificationLedger};
use telegram::ReqwestTelegramSender;
pub use telegram::{DeliveryError, TelegramSender};

pub const CONTRACT_VERSION: &str = "1";
pub const MAX_TASK_ID_BYTES: usize = 128;
pub const MAX_TITLE_BYTES: usize = 160;
pub const MAX_SUMMARY_BYTES: usize = 2_000;
pub const MAX_RESULT_URL_BYTES: usize = 2_048;
pub const MAX_MESSAGE_BYTES: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskCompletionPayload {
    pub task_id: String,
    pub title: String,
    pub summary: String,
    pub completed_at: String,
    pub result_url: Option<String>,
    pub source: String,
}

#[derive(Debug)]
pub enum NotificationError {
    Io,
    Database,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueResult {
    Queued,
    AlreadySent,
    Disabled,
}

impl QueueResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::AlreadySent => "already_sent",
            Self::Disabled => "disabled",
        }
    }
}

/// Validates, normalizes, and redacts an event before it can reach the
/// durable ledger. `source` is assigned by the authenticated entry point and
/// is never taken from arbitrary MCP tool arguments.
pub fn sanitize_task_completion(
    mut payload: TaskCompletionPayload,
) -> Result<TaskCompletionPayload, NotificationError> {
    if payload.source != "nuxt" && payload.source != "chatgpt" {
        return Err(NotificationError::Invalid);
    }
    payload.task_id = bounded_text(payload.task_id, MAX_TASK_ID_BYTES)?;
    payload.title = bounded_text(payload.title, MAX_TITLE_BYTES)?;
    payload.summary = bounded_text(payload.summary, MAX_SUMMARY_BYTES)?;
    payload.completed_at = bounded_timestamp(payload.completed_at)?;
    payload.result_url = payload.result_url.map(bounded_result_url).transpose()?;
    Ok(payload)
}

pub fn format_task_completion_message(payload: &TaskCompletionPayload) -> String {
    let result_line = payload
        .result_url
        .as_deref()
        .map(|url| format!("\nResult: {url}"))
        .unwrap_or_default();
    let prefix = format!("✅ {}\n", payload.title);
    let prefix_and_result = format!("{prefix}{result_line}");
    let available = MAX_MESSAGE_BYTES.saturating_sub(prefix_and_result.len());
    let summary = truncate_utf8(&payload.summary, available);
    let message = format!("{prefix}{summary}{result_line}");
    if message.len() <= MAX_MESSAGE_BYTES {
        return message;
    }
    truncate_utf8(
        &format!(
            "{prefix}{}",
            truncate_utf8(
                &payload.summary,
                MAX_MESSAGE_BYTES.saturating_sub(prefix.len())
            )
        ),
        MAX_MESSAGE_BYTES,
    )
}

/// Parse the bounded public tool/private method argument shape. The source is
/// supplied by the caller of this function, not accepted from the payload.
pub fn payload_from_arguments(
    arguments: &Value,
    source: &'static str,
) -> Result<TaskCompletionPayload, NotificationError> {
    let object = arguments.as_object().ok_or(NotificationError::Invalid)?;
    let task_id = object
        .get("taskId")
        .and_then(Value::as_str)
        .ok_or(NotificationError::Invalid)?;
    let title = object
        .get("title")
        .and_then(Value::as_str)
        .ok_or(NotificationError::Invalid)?;
    let summary = object
        .get("summary")
        .and_then(Value::as_str)
        .ok_or(NotificationError::Invalid)?;
    let result_url = object
        .get("resultUrl")
        .map(|value| value.as_str().ok_or(NotificationError::Invalid))
        .transpose()?;
    let completed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| NotificationError::Invalid)?;
    sanitize_task_completion(TaskCompletionPayload {
        task_id: task_id.to_owned(),
        title: title.to_owned(),
        summary: summary.to_owned(),
        completed_at,
        result_url: result_url.map(str::to_owned),
        source: source.to_owned(),
    })
}

pub struct TaskNotificationService {
    ledger: Option<Arc<NotificationLedger>>,
    sender: Option<Arc<dyn TelegramSender>>,
}

impl TaskNotificationService {
    pub fn open(config: &ServerConfig) -> Result<Arc<Self>, NotificationError> {
        if !config.telegram_enabled {
            return Ok(Arc::new(Self {
                ledger: None,
                sender: None,
            }));
        }
        let token = config
            .telegram_bot_token
            .as_deref()
            .ok_or(NotificationError::Invalid)?;
        let chat_id = config
            .telegram_chat_id
            .as_deref()
            .ok_or(NotificationError::Invalid)?;
        let ledger = Arc::new(NotificationLedger::open(config)?);
        let sender = Arc::new(ReqwestTelegramSender::new(token, chat_id)?);
        Ok(Arc::new(Self {
            ledger: Some(ledger),
            sender: Some(sender),
        }))
    }

    pub async fn enqueue(
        &self,
        payload: TaskCompletionPayload,
    ) -> Result<QueueResult, NotificationError> {
        let Some(ledger) = &self.ledger else {
            return Ok(QueueResult::Disabled);
        };
        let payload = sanitize_task_completion(payload)?;
        let inserted = ledger.enqueue(&payload)?;
        if !inserted {
            return Ok(QueueResult::AlreadySent);
        }
        let _ = self.process_once().await;
        Ok(QueueResult::Queued)
    }

    pub async fn process_once(&self) -> Result<bool, NotificationError> {
        let (Some(ledger), Some(sender)) = (&self.ledger, &self.sender) else {
            return Ok(false);
        };
        let Some(claimed) = ledger.claim_next(now_ms())? else {
            return Ok(false);
        };
        match sender.send(&claimed.message).await {
            Ok(()) => ledger.mark_sent(&claimed.task_id)?,
            Err(DeliveryError::Retryable) => {
                let delay = backoff(claimed.attempts);
                ledger.mark_retry(&claimed.task_id, "transient", delay)?;
            }
            Err(DeliveryError::RateLimited(delay)) => {
                ledger.mark_retry(&claimed.task_id, "rate_limited", delay)?;
            }
            Err(DeliveryError::Permanent) => ledger.mark_failed(&claimed.task_id, "permanent")?,
        }
        Ok(true)
    }

    pub fn spawn_worker(self: &Arc<Self>) {
        if self.ledger.is_none() {
            return;
        }
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let service = Arc::clone(self);
        handle.spawn(async move {
            loop {
                let _ = service.process_once().await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

fn bounded_text(value: String, max: usize) -> Result<String, NotificationError> {
    let value = redact_text(&value);
    if value.is_empty() || value.len() > max {
        return Err(NotificationError::Invalid);
    }
    Ok(value)
}

fn bounded_timestamp(value: String) -> Result<String, NotificationError> {
    if value.is_empty() || value.len() > 64 || value.chars().any(char::is_control) {
        return Err(NotificationError::Invalid);
    }
    Ok(value)
}

fn bounded_result_url(value: String) -> Result<String, NotificationError> {
    if value.is_empty() || value.len() > MAX_RESULT_URL_BYTES {
        return Err(NotificationError::Invalid);
    }
    let parsed = url::Url::parse(&value).map_err(|_| NotificationError::Invalid)?;
    if parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.as_str() != value
    {
        return Err(NotificationError::Invalid);
    }
    Ok(value)
}

fn redact_text(value: &str) -> String {
    let mut result = value
        .replace('\u{1b}', "")
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>();
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");
    redact_bearer(&mut result);
    for keyword in [
        "authorization",
        "cookie",
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "api-key",
    ] {
        redact_assignment(&mut result, keyword);
    }
    result
}

fn redact_bearer(value: &mut String) {
    loop {
        let lower = value.to_ascii_lowercase();
        let Some(start) = lower.find("bearer") else {
            return;
        };
        let after = start + "bearer".len();
        if lower
            .as_bytes()
            .get(after)
            .is_none_or(|byte| !byte.is_ascii_whitespace())
        {
            return;
        }
        let secret_start = lower[after..]
            .find(|ch: char| !ch.is_ascii_whitespace())
            .map(|offset| after + offset)
            .unwrap_or(value.len());
        if lower[secret_start..].starts_with("[redacted]") {
            return;
        }
        let secret_end = value[secret_start..]
            .find(|ch: char| ch.is_ascii_whitespace() || [',', ';'].contains(&ch))
            .map(|offset| secret_start + offset)
            .unwrap_or(value.len());
        if secret_start == secret_end {
            return;
        }
        value.replace_range(secret_start..secret_end, "[REDACTED]");
    }
}

fn redact_assignment(value: &mut String, keyword: &str) {
    loop {
        let lower = value.to_ascii_lowercase();
        let Some(start) = lower.find(keyword) else {
            return;
        };
        let after = start + keyword.len();
        let Some(delimiter) = lower[after..].find(['=', ':']).map(|offset| after + offset) else {
            return;
        };
        if lower[after..delimiter]
            .chars()
            .any(|ch| !ch.is_ascii_whitespace())
        {
            return;
        }
        let secret_start = lower[delimiter + 1..]
            .find(|ch: char| !ch.is_ascii_whitespace())
            .map(|offset| delimiter + 1 + offset)
            .unwrap_or(value.len());
        if lower[secret_start..].starts_with("bearer ") {
            return;
        }
        if lower[secret_start..].starts_with("[redacted]") {
            return;
        }
        let secret_end = value[secret_start..]
            .find(|ch: char| ch.is_ascii_whitespace() || [',', ';'].contains(&ch))
            .map(|offset| secret_start + offset)
            .unwrap_or(value.len());
        if secret_start == secret_end {
            return;
        }
        value.replace_range(secret_start..secret_end, "[REDACTED]");
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].trim_end().to_owned()
}

fn backoff(attempts: u32) -> Duration {
    Duration::from_secs(5_u64.saturating_mul(1_u64 << attempts.min(6)))
        .min(Duration::from_secs(300))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
