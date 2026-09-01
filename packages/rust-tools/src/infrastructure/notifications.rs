//! Outbound Telegram message delivery.
//!
//! Telegram is a first-class MCP capability, but credentials, destination,
//! topic, and endpoint remain relay-owned. Callers provide only an authorized
//! absolute working directory plus bounded message text.

use crate::core::config::ServerConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod dotenv;
mod ledger;
mod telegram;
pub use ledger::{ClaimedTelegramMessage, TelegramCredentials, TelegramMessageLedger};
use telegram::ReqwestTelegramSender;
pub use telegram::{telegram_send_message_body, DeliveryError, TelegramSender};

pub const CONTRACT_VERSION: &str = "2";
pub const MAX_WORKING_DIRECTORY_BYTES: usize = 4_096;
pub const MAX_USER_MESSAGE_BYTES: usize = 4_000;
pub const MAX_MESSAGE_BYTES: usize = 4_096;
pub const MAX_MESSAGE_THREAD_ID: i64 = i32::MAX as i64;
const TELEGRAM_MESSAGE_PREFIX: &str = "Working directory: ";
const TELEGRAM_MESSAGE_SEPARATOR: &str = "\n\n";

pub fn validate_channel_target(chat_id: &str) -> Result<(), NotificationError> {
    let valid_numeric_channel = chat_id.strip_prefix("-100").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.len() <= 16 && suffix.chars().all(|ch| ch.is_ascii_digit())
    });
    let valid_username = chat_id.strip_prefix('@').is_some_and(|username| {
        (5..=32).contains(&username.len())
            && username
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    });
    if valid_numeric_channel || valid_username {
        Ok(())
    } else {
        Err(NotificationError::Invalid)
    }
}

pub fn validate_message_thread_id(message_thread_id: Option<i64>) -> Result<(), NotificationError> {
    match message_thread_id {
        None => Ok(()),
        Some(id) if (1..=MAX_MESSAGE_THREAD_ID).contains(&id) => Ok(()),
        Some(_) => Err(NotificationError::Invalid),
    }
}

pub(crate) fn validate_bot_token(token: &str) -> Result<(), NotificationError> {
    let Some((bot_id, secret)) = token.split_once(':') else {
        return Err(NotificationError::Invalid);
    };
    if bot_id.is_empty()
        || secret.is_empty()
        || token.len() > 512
        || !bot_id.chars().all(|ch| ch.is_ascii_digit())
        || !secret
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        return Err(NotificationError::Invalid);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramMessagePayload {
    pub working_directory: String,
    pub message: String,
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
    Disabled,
}

impl QueueResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Disabled => "disabled",
        }
    }
}

pub fn sanitize_telegram_message(
    mut payload: TelegramMessagePayload,
) -> Result<TelegramMessagePayload, NotificationError> {
    payload.working_directory = bounded_working_directory(payload.working_directory)?;
    payload.message = bounded_text(payload.message, MAX_USER_MESSAGE_BYTES, true)?;
    if formatted_message_len(&payload) > MAX_MESSAGE_BYTES {
        return Err(NotificationError::Invalid);
    }
    Ok(payload)
}

pub fn format_telegram_message(payload: &TelegramMessagePayload) -> String {
    format!(
        "{TELEGRAM_MESSAGE_PREFIX}{}{TELEGRAM_MESSAGE_SEPARATOR}{}",
        payload.working_directory, payload.message
    )
}

fn formatted_message_len(payload: &TelegramMessagePayload) -> usize {
    TELEGRAM_MESSAGE_PREFIX.len()
        + payload.working_directory.len()
        + TELEGRAM_MESSAGE_SEPARATOR.len()
        + payload.message.len()
}

/// Parse and validate the public tool arguments. Workspace authorization is
/// enforced against the relay's live allowlist; relative paths are rejected
/// even though other workspace tools may resolve them against the primary root.
pub fn payload_from_arguments(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<TelegramMessagePayload, NotificationError> {
    let object = arguments.as_object().ok_or(NotificationError::Invalid)?;
    if object.len() != 2
        || object
            .keys()
            .any(|key| key != "working_directory" && key != "message")
    {
        return Err(NotificationError::Invalid);
    }
    let working_directory = object
        .get("working_directory")
        .and_then(Value::as_str)
        .ok_or(NotificationError::Invalid)?;
    if working_directory.len() > MAX_WORKING_DIRECTORY_BYTES
        || !Path::new(working_directory).is_absolute()
    {
        return Err(NotificationError::Invalid);
    }
    config
        .ensure_workspaces_initialized()
        .map_err(|_| NotificationError::Invalid)?;
    let guard = config
        .workspaces
        .read()
        .map_err(|_| NotificationError::Invalid)?;
    let canonical = crate::core::workspace_path::resolve_contained_cwd_in_allowlist(
        &guard,
        Some(working_directory),
    )
    .map_err(|_| NotificationError::Invalid)?;
    let canonical = canonical
        .to_str()
        .ok_or(NotificationError::Invalid)?
        .to_owned();
    drop(guard);
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .ok_or(NotificationError::Invalid)?;
    sanitize_telegram_message(TelegramMessagePayload {
        working_directory: canonical,
        message: message.to_owned(),
    })
}

pub struct TelegramMessageService {
    ledger: Option<Arc<TelegramMessageLedger>>,
    sender: Option<Arc<dyn TelegramSender>>,
}

impl TelegramMessageService {
    pub fn open(config: &ServerConfig) -> Result<Arc<Self>, NotificationError> {
        if !config.telegram_enabled {
            return Ok(Arc::new(Self {
                ledger: None,
                sender: None,
            }));
        }
        let ledger = Arc::new(TelegramMessageLedger::open(config)?);
        let credentials = ledger.load_telegram_credentials()?;
        let sender = credentials.as_ref().and_then(|credentials| {
            ReqwestTelegramSender::new(
                &credentials.bot_token,
                &credentials.chat_id,
                credentials.message_thread_id,
            )
            .ok()
            .map(|sender| Arc::new(sender) as Arc<dyn TelegramSender>)
        });
        if sender.is_none() {
            tracing::warn!("Telegram messaging is enabled but relay credentials are unavailable or invalid; delivery is disabled");
        }
        Ok(Arc::new(Self {
            ledger: Some(ledger),
            sender,
        }))
    }

    pub async fn enqueue(
        &self,
        payload: TelegramMessagePayload,
    ) -> Result<QueueResult, NotificationError> {
        let Some(ledger) = &self.ledger else {
            return Ok(QueueResult::Disabled);
        };
        if self.sender.is_none() {
            return Ok(QueueResult::Disabled);
        }
        let payload = sanitize_telegram_message(payload)?;
        let _message_id = ledger.enqueue(&payload)?;
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
            Ok(()) => ledger.mark_sent(&claimed.message_id)?,
            Err(DeliveryError::Retryable) => {
                let delay = backoff(claimed.attempts);
                ledger.mark_retry(&claimed.message_id, "transient", delay)?;
            }
            Err(DeliveryError::RateLimited(delay)) => {
                ledger.mark_retry(&claimed.message_id, "rate_limited", delay)?;
            }
            Err(DeliveryError::Permanent) => {
                ledger.mark_failed(&claimed.message_id, "permanent")?
            }
        }
        Ok(true)
    }

    pub fn spawn_worker(self: &Arc<Self>) {
        if self.ledger.is_none() || self.sender.is_none() {
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

fn bounded_working_directory(value: String) -> Result<String, NotificationError> {
    if value.is_empty()
        || value.len() > MAX_WORKING_DIRECTORY_BYTES
        || value.chars().any(char::is_control)
        || !Path::new(&value).is_absolute()
    {
        return Err(NotificationError::Invalid);
    }
    Ok(value)
}

fn bounded_text(
    value: String,
    max: usize,
    preserve_newlines: bool,
) -> Result<String, NotificationError> {
    let value = redact_text(&value, preserve_newlines);
    if value.is_empty() || value.len() > max {
        return Err(NotificationError::Invalid);
    }
    Ok(value)
}

fn redact_text(value: &str, preserve_newlines: bool) -> String {
    let mut result = value
        .replace('\u{1b}', "")
        .chars()
        .map(|ch| {
            if ch.is_control() && !(preserve_newlines && ch == '\n') {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();
    result = if preserve_newlines {
        result
            .split('\n')
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    };
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
