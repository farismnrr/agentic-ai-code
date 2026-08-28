use super::{validate_bot_token, validate_channel_target, NotificationError};
use std::fs;
use std::path::Path;

pub struct HermesTelegramCredentials {
    pub bot_token: String,
    pub home_channel: String,
}

pub fn load_credentials(path: &Path) -> Result<HermesTelegramCredentials, NotificationError> {
    let contents = fs::read_to_string(path).map_err(|_| NotificationError::Io)?;
    let mut bot_token = None;
    let mut home_channel = None;
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
            "TELEGRAM_HOME_CHANNEL" => home_channel = Some(value),
            _ => {}
        }
    }
    let bot_token = bot_token.ok_or(NotificationError::Invalid)?;
    let home_channel = home_channel.ok_or(NotificationError::Invalid)?;
    validate_bot_token(&bot_token)?;
    validate_channel_target(&home_channel)?;
    Ok(HermesTelegramCredentials {
        bot_token,
        home_channel,
    })
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
