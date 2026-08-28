use super::NotificationError;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug)]
pub enum DeliveryError {
    Retryable,
    RateLimited(Duration),
    Permanent,
}

pub trait TelegramSender: Send + Sync {
    fn send<'a>(
        &'a self,
        message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), DeliveryError>> + Send + 'a>>;
}

pub(super) struct ReqwestTelegramSender {
    client: reqwest::Client,
    endpoint: String,
    chat_id: String,
}

impl ReqwestTelegramSender {
    pub(super) fn new(token: &str, chat_id: &str) -> Result<Self, NotificationError> {
        if token.is_empty() || chat_id.is_empty() {
            return Err(NotificationError::Invalid);
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| NotificationError::Io)?,
            endpoint: format!("https://api.telegram.org/bot{token}/sendMessage"),
            chat_id: chat_id.to_owned(),
        })
    }
}

impl TelegramSender for ReqwestTelegramSender {
    fn send<'a>(
        &'a self,
        message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), DeliveryError>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .client
                .post(&self.endpoint)
                .json(&json!({ "chat_id": self.chat_id, "text": message }))
                .send()
                .await
                .map_err(|_| DeliveryError::Retryable)?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(DeliveryError::RateLimited(Duration::from_secs(60)));
            }
            if status.is_server_error() {
                return Err(DeliveryError::Retryable);
            }
            if !status.is_success() {
                return Err(DeliveryError::Permanent);
            }
            let body = response
                .json::<TelegramApiResponse>()
                .await
                .map_err(|_| DeliveryError::Retryable)?;
            if body.ok {
                Ok(())
            } else {
                Err(DeliveryError::Permanent)
            }
        })
    }
}

#[derive(Deserialize)]
struct TelegramApiResponse {
    ok: bool,
}
