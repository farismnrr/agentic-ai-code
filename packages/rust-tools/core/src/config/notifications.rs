use super::ServerConfig;
use crate::error::RelayError;

pub(super) fn validate(config: &ServerConfig) -> Result<(), RelayError> {
    if !config.telegram_enabled {
        return Ok(());
    }
    if let Some(path) = config.telegram_hermes_env.as_deref() {
        if path.is_empty() || path.chars().any(char::is_control) {
            return Err(RelayError::InvalidConfig(
                "RELAY_TELEGRAM_HERMES_ENV must be a non-empty path without control characters"
                    .into(),
            ));
        }
    }
    Ok(())
}
