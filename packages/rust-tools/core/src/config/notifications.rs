use super::ServerConfig;
use crate::error::RelayError;

pub(super) fn validate(config: &ServerConfig) -> Result<(), RelayError> {
    if !config.telegram_enabled {
        return Ok(());
    }
    let token_valid = config.telegram_bot_token.as_deref().is_some_and(|token| {
        !token.is_empty() && token.len() <= 512 && !token.chars().any(char::is_control)
    });
    let chat_id_valid = config.telegram_chat_id.as_deref().is_some_and(|chat_id| {
        !chat_id.is_empty()
            && chat_id.len() <= 128
            && chat_id.chars().all(|ch| ch.is_ascii_graphic())
    });
    if token_valid && chat_id_valid {
        return Ok(());
    }
    Err(RelayError::InvalidConfig(
        "Telegram notifier is enabled but its server-side credentials are incomplete or invalid"
            .into(),
    ))
}
