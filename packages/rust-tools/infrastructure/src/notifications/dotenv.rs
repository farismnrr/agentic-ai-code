use super::{
    validate_bot_token, validate_channel_target, validate_message_thread_id, NotificationError,
};
use std::fs;
use std::path::Path;

pub struct EnvTelegramCredentials {
    pub bot_token: String,
    pub channel: String,
    pub message_thread_id: Option<i64>,
}

pub fn load_credentials(path: &Path) -> Result<EnvTelegramCredentials, NotificationError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| NotificationError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NotificationError::Io);
    }
    let contents = fs::read_to_string(path).map_err(|_| NotificationError::Io)?;
    let mut bot_token = None;
    let mut channel = None;
    let mut message_thread_id = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = parse_value(value);
        match key.trim() {
            "TELEGRAM_BOT_TOKEN" => bot_token = Some(value),
            "TELEGRAM_HOME_CHANNEL" => channel = Some(value),
            "TELEGRAM_HOME_CHANNEL_THREAD_ID" => {
                message_thread_id = parse_message_thread_id(&value)?
            }
            _ => {}
        }
    }
    let bot_token = bot_token.ok_or(NotificationError::Invalid)?;
    let channel = channel.ok_or(NotificationError::Invalid)?;
    validate_bot_token(&bot_token)?;
    validate_channel_target(&channel)?;
    validate_message_thread_id(message_thread_id)?;
    Ok(EnvTelegramCredentials {
        bot_token,
        channel,
        message_thread_id,
    })
}

fn parse_message_thread_id(value: &str) -> Result<Option<i64>, NotificationError> {
    if value.is_empty() {
        return Ok(None);
    }
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(NotificationError::Invalid);
    }
    let message_thread_id = value
        .parse::<i64>()
        .map_err(|_| NotificationError::Invalid)?;
    validate_message_thread_id(Some(message_thread_id))?;
    Ok(Some(message_thread_id))
}

fn parse_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return value[1..value.len() - 1].to_owned();
    }
    value.to_owned()
}
