use super::{validate_freeform_text, validate_id, validate_text};
use crate::application::creative::contracts::{CreativeProject, ElementKind, LocationPackSpec};
use crate::core::error::McpError;
use serde_json::Value;
use std::collections::HashSet;

const MAX_LOCATION_LANDMARKS: usize = 64;
const MAX_LOCATION_VARIANTS: usize = 32;
const MAX_LOCATION_CAMERA_LANDMARKS: usize = 64;
const MAX_LOCATION_3D_BINDINGS: usize = 32;

pub(super) fn validate_location_pack_spec(
    value: &Value,
    project: &CreativeProject,
) -> Result<(), McpError> {
    let spec: LocationPackSpec = serde_json::from_value(value.clone()).map_err(|_| {
        McpError::InvalidRequest("location element requires a valid world_location_v1 spec".into())
    })?;
    if spec.spec_type != "world_location_v1" {
        return Err(McpError::InvalidRequest(
            "location element spec_type must be world_location_v1".into(),
        ));
    }
    validate_freeform_text(&spec.concept, 1, 4096, "location concept")?;
    validate_text(&spec.scale, 1, 256, "location scale")?;
    validate_freeform_text(
        &spec.architecture_language,
        1,
        4096,
        "location architecture language",
    )?;
    validate_freeform_text(
        &spec.set_dressing_language,
        1,
        4096,
        "location set dressing language",
    )?;
    validate_freeform_text(&spec.continuity_notes, 0, 4096, "location continuity notes")?;
    if spec.canonical_landmarks.len() > MAX_LOCATION_LANDMARKS
        || spec.variants.len() > MAX_LOCATION_VARIANTS
        || spec.camera_landmarks.len() > MAX_LOCATION_CAMERA_LANDMARKS
        || spec.asset3d_element_ids.len() > MAX_LOCATION_3D_BINDINGS
    {
        return Err(McpError::InvalidRequest(
            "location pack collection exceeds allowed bounds".into(),
        ));
    }
    validate_unique_texts(&spec.canonical_landmarks, 256, "location landmark")?;
    validate_unique_texts(&spec.camera_landmarks, 256, "location camera landmark")?;

    let mut variant_ids = HashSet::new();
    for variant in &spec.variants {
        validate_id(&variant.variant_id, "location variant id")?;
        if !variant_ids.insert(variant.variant_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "location variant identity must be unique".into(),
            ));
        }
        for (value, label) in [
            (variant.time_of_day.as_deref(), "location time of day"),
            (variant.weather.as_deref(), "location weather"),
            (variant.lighting.as_deref(), "location lighting"),
        ] {
            if let Some(value) = value {
                validate_text(value, 1, 256, label)?;
            }
        }
    }

    if let Some(style_element_id) = spec.style_element_id.as_deref() {
        validate_id(style_element_id, "location style element id")?;
        let element = project.element(style_element_id).ok_or_else(|| {
            McpError::InvalidRequest("location style references an unknown element".into())
        })?;
        if element.kind != ElementKind::Style {
            return Err(McpError::InvalidRequest(
                "location style dependency must reference a Style Element".into(),
            ));
        }
    }

    let mut asset3d_ids = HashSet::new();
    for element_id in &spec.asset3d_element_ids {
        validate_id(element_id, "location 3D element id")?;
        if !asset3d_ids.insert(element_id.as_str()) {
            return Err(McpError::InvalidRequest(
                "location 3D element identities must be unique".into(),
            ));
        }
        let element = project.element(element_id).ok_or_else(|| {
            McpError::InvalidRequest("location 3D binding references an unknown element".into())
        })?;
        if element.kind != ElementKind::Asset3d {
            return Err(McpError::InvalidRequest(
                "location 3D binding must reference an Asset3d Element".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn selected_location_pack(
    project: &CreativeProject,
    element_id: &str,
) -> Result<LocationPackSpec, McpError> {
    validate_id(element_id, "location element id")?;
    let element = project.element(element_id).ok_or_else(|| {
        McpError::InvalidRequest("scene location references an unknown element".into())
    })?;
    if element.kind != ElementKind::Location {
        return Err(McpError::InvalidRequest(
            "scene location must reference a Location Element".into(),
        ));
    }
    let selected_id = element.selected_revision_id.as_deref().ok_or_else(|| {
        McpError::InvalidRequest("scene location requires a selected Location revision".into())
    })?;
    let revision = element
        .revisions
        .iter()
        .find(|revision| revision.revision_id == selected_id)
        .ok_or_else(|| McpError::InvalidRequest("selected Location revision is missing".into()))?;
    validate_location_pack_spec(&revision.spec, project)?;
    serde_json::from_value(revision.spec.clone()).map_err(|_| {
        McpError::InvalidRequest("selected Location revision is not world_location_v1".into())
    })
}

fn validate_unique_texts(values: &[String], max: usize, label: &str) -> Result<(), McpError> {
    let mut seen = HashSet::new();
    for value in values {
        validate_text(value, 1, max, label)?;
        if !seen.insert(value.as_str()) {
            return Err(McpError::InvalidRequest(format!(
                "{label} values must be unique"
            )));
        }
    }
    Ok(())
}
