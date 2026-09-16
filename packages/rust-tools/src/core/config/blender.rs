use super::ServerConfig;
use crate::core::error::RelayError;

pub(super) const DEFAULT_BLENDER_BRIDGE_PORT: u16 = 9876;
pub(super) const MAX_BLENDER_BRIDGE_TIMEOUT_MS: u64 = 120_000;

pub(super) fn validate(config: &ServerConfig) -> Result<(), RelayError> {
    if config.blender_bridge_port < 1024 {
        return Err(RelayError::InvalidConfig(
            "Blender bridge port must be an unprivileged TCP port".into(),
        ));
    }
    if !(100..=MAX_BLENDER_BRIDGE_TIMEOUT_MS).contains(&config.blender_bridge_timeout_ms) {
        return Err(RelayError::InvalidConfig(
            "Blender bridge timeout is outside allowed bounds".into(),
        ));
    }
    if let Some(executable) = config.blender_executable.as_deref() {
        if executable.is_empty()
            || executable.len() > 4_096
            || executable.chars().any(char::is_control)
        {
            return Err(RelayError::InvalidConfig(
                "Blender executable configuration is invalid".into(),
            ));
        }
    }
    Ok(())
}
