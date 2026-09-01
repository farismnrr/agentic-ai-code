use super::cli::ActivityMode;
use crate::core::error::RelayError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivityConfig {
    pub mode: ActivityMode,
    pub state_dir: Option<String>,
    pub sink_url: Option<String>,
    pub source_token: Option<String>,
    pub spool_quota_bytes: u64,
    pub acknowledged_retention_ms: u64,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            mode: ActivityMode::Off,
            state_dir: None,
            sink_url: None,
            source_token: None,
            spool_quota_bytes: 64 * 1024 * 1024,
            acknowledged_retention_ms: 86_400_000,
        }
    }
}

pub(super) fn validate(config: &ActivityConfig) -> Result<(), RelayError> {
    if config.spool_quota_bytes < 64 * 1024 {
        return Err(RelayError::InvalidConfig(
            "activity spool quota must be at least 65536 bytes".into(),
        ));
    }
    if config.acknowledged_retention_ms == 0 {
        return Err(RelayError::InvalidConfig(
            "activity acknowledged retention must be non-zero".into(),
        ));
    }
    if let Some(token) = &config.source_token {
        if token.len() < 32 || token.chars().any(char::is_control) {
            return Err(RelayError::InvalidConfig(
                "activity source token must be a bounded high-entropy credential".into(),
            ));
        }
    }
    if let Some(sink) = &config.sink_url {
        let parsed = url::Url::parse(sink).map_err(|_| {
            RelayError::InvalidConfig("activity sink URL must be absolute HTTPS".into())
        })?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || parsed.username() != ""
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(RelayError::InvalidConfig(
                "activity sink URL must be absolute HTTPS without credentials or query".into(),
            ));
        }
    }
    Ok(())
}
