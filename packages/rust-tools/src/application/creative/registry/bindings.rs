use super::{capability, BindingAvailability, ExecutionBindingDescriptor};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

pub fn execution_bindings(
    config: &ServerConfig,
) -> Result<Vec<ExecutionBindingDescriptor>, McpError> {
    let mut bindings = Vec::with_capacity(config.creative_binding_descriptors.len());
    for raw in &config.creative_binding_descriptors {
        let descriptor: ExecutionBindingDescriptor = serde_json::from_str(raw).map_err(|_| {
            McpError::InvalidRequest("creative execution binding descriptor is invalid".into())
        })?;
        validate_binding_descriptor(&descriptor)?;
        if bindings
            .iter()
            .any(|existing: &ExecutionBindingDescriptor| {
                existing.binding_id == descriptor.binding_id
            })
        {
            return Err(McpError::InvalidRequest(
                "duplicate creative execution binding identity".into(),
            ));
        }
        bindings.push(descriptor);
    }
    bindings.sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
    Ok(bindings)
}

pub fn binding(
    config: &ServerConfig,
    id: &str,
) -> Result<Option<ExecutionBindingDescriptor>, McpError> {
    Ok(execution_bindings(config)?
        .into_iter()
        .find(|descriptor| descriptor.binding_id == id))
}

pub fn compatible_binding_ids(
    config: &ServerConfig,
    capability_id: &str,
) -> Result<Vec<String>, McpError> {
    Ok(execution_bindings(config)?
        .into_iter()
        .filter(|descriptor| {
            descriptor.availability == BindingAvailability::Available
                && descriptor
                    .capabilities
                    .iter()
                    .any(|value| value == capability_id)
        })
        .map(|descriptor| descriptor.binding_id)
        .collect())
}

pub fn validate_binding_selection(
    config: &ServerConfig,
    capability_id: &str,
    selected: Option<&str>,
) -> Result<Option<ExecutionBindingDescriptor>, McpError> {
    let descriptor = capability(capability_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative capability".into()))?;
    if !descriptor.requires_execution_binding {
        return Ok(None);
    }
    let compatible = compatible_binding_ids(config, capability_id)?;
    let Some(selected) = selected else {
        return Err(McpError::InvalidRequest(format!(
            "execution_binding_required:{capability_id}:{}",
            compatible.join(",")
        )));
    };
    let binding = binding(config, selected)?
        .ok_or_else(|| McpError::InvalidRequest("execution binding is unavailable".into()))?;
    if binding.availability != BindingAvailability::Available {
        return Err(McpError::InvalidRequest(
            "execution binding is unavailable".into(),
        ));
    }
    if !binding
        .capabilities
        .iter()
        .any(|value| value == capability_id)
    {
        return Err(McpError::InvalidRequest(
            "execution binding is incompatible with capability".into(),
        ));
    }
    Ok(Some(binding))
}

fn validate_binding_descriptor(descriptor: &ExecutionBindingDescriptor) -> Result<(), McpError> {
    crate::application::creative::contracts::validate_id(
        &descriptor.binding_id,
        "execution_binding_id",
    )?;
    if descriptor.binding_version.is_empty()
        || descriptor.binding_version.len() > 64
        || descriptor.binding_version.chars().any(char::is_control)
        || descriptor.capabilities.is_empty()
        || descriptor.capabilities.len() > 64
        || descriptor.media_roles.len() > 64
    {
        return Err(McpError::InvalidRequest(
            "creative execution binding descriptor exceeds allowed bounds".into(),
        ));
    }
    for capability_id in &descriptor.capabilities {
        if capability(capability_id).is_none() {
            return Err(McpError::InvalidRequest(
                "creative execution binding references an unknown capability".into(),
            ));
        }
    }
    for role in &descriptor.media_roles {
        if role.is_empty() || role.len() > 128 || role.chars().any(char::is_control) {
            return Err(McpError::InvalidRequest(
                "creative execution binding media role is invalid".into(),
            ));
        }
    }
    validate_public_descriptor_value(&descriptor.extension_schema, 0)?;
    validate_public_descriptor_value(&descriptor.constraints, 0)?;
    if descriptor
        .license_notes
        .as_deref()
        .is_some_and(|value| value.len() > 1024 || value.chars().any(char::is_control))
    {
        return Err(McpError::InvalidRequest(
            "creative execution binding license notes exceed allowed bounds".into(),
        ));
    }
    Ok(())
}

fn validate_public_descriptor_value(value: &Value, depth: usize) -> Result<(), McpError> {
    if depth > 8 {
        return Err(McpError::InvalidRequest(
            "creative execution binding descriptor nesting exceeds maximum".into(),
        ));
    }
    match value {
        Value::Object(object) => {
            if object.len() > 128 {
                return Err(McpError::InvalidRequest(
                    "creative execution binding descriptor object is too large".into(),
                ));
            }
            for (key, child) in object {
                let normalized = key.to_ascii_lowercase().replace(['-', '.'], "_");
                if [
                    "endpoint",
                    "url",
                    "api_key",
                    "apikey",
                    "token",
                    "secret",
                    "password",
                    "authorization",
                    "credential",
                    "credentials",
                    "headers",
                    "command",
                    "code",
                    "script",
                    "executable",
                    "environment",
                    "env",
                    "socket",
                ]
                .iter()
                .any(|forbidden| {
                    normalized == *forbidden
                        || normalized.starts_with(&format!("{forbidden}_"))
                        || normalized.ends_with(&format!("_{forbidden}"))
                }) {
                    return Err(McpError::InvalidRequest(
                        "creative execution binding descriptors cannot expose endpoints, credentials, or executable payloads".into(),
                    ));
                }
                validate_public_descriptor_value(child, depth + 1)?;
            }
        }
        Value::Array(values) => {
            if values.len() > 128 {
                return Err(McpError::InvalidRequest(
                    "creative execution binding descriptor array is too large".into(),
                ));
            }
            for child in values {
                validate_public_descriptor_value(child, depth + 1)?;
            }
        }
        Value::String(value) if value.len() > 4096 || value.chars().any(char::is_control) => {
            return Err(McpError::InvalidRequest(
                "creative execution binding descriptor string exceeds allowed bounds".into(),
            ));
        }
        _ => {}
    }
    Ok(())
}
