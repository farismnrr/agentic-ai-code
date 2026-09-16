use super::ServerConfig;
use crate::core::error::RelayError;

pub(super) fn validate(config: &ServerConfig) -> Result<(), RelayError> {
    if config.creative_job_hard_compute_units == 0
        || config.creative_project_hard_compute_units < config.creative_job_hard_compute_units
        || config.creative_max_job_output_bytes == 0
        || config.creative_max_concurrent_jobs == 0
        || config.creative_max_retries > 16
        || config.creative_approval_compute_units > config.creative_job_hard_compute_units
    {
        return Err(RelayError::InvalidConfig(
            "creative job/budget limits are invalid".into(),
        ));
    }
    if config.creative_binding_descriptors.len() > 32
        || config.creative_binding_descriptors.iter().any(|value| {
            value.is_empty() || value.len() > 16 * 1024 || value.chars().any(char::is_control)
        })
    {
        return Err(RelayError::InvalidConfig(
            "creative binding descriptors exceed allowed bounds".into(),
        ));
    }
    if config.creative_binding_backends.len() > 32
        || config.creative_binding_backends.iter().any(|value| {
            value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
        })
    {
        return Err(RelayError::InvalidConfig(
            "creative binding backends exceed allowed bounds".into(),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for mapping in &config.creative_binding_backends {
        let Some((binding_id, backend_kind)) = mapping.split_once('=') else {
            return Err(RelayError::InvalidConfig(
                "creative binding backend mapping must be binding_id=backend_kind".into(),
            ));
        };
        if binding_id.is_empty()
            || binding_id.len() > 128
            || !binding_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
            || !matches!(backend_kind, "local_raster" | "local_static_game")
            || !ids.insert(binding_id)
        {
            return Err(RelayError::InvalidConfig(
                "creative binding backend mapping is invalid or duplicated".into(),
            ));
        }
    }
    Ok(())
}
