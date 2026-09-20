use super::{apply_reference_tint, enforce_dimensions, id_array, load_asset_image};
use crate::application::creative::contracts::{
    AssetMetadata, AssetSource, AssetState, AssetSurface, ElementKind,
};
use crate::application::creative::graph::{CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use crate::application::creative::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use image::{DynamicImage, ImageFormat};
use ring::digest::{Context, SHA256};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Cursor;

const MAX_VARIANTS: usize = 32;
const MAX_VARIANT_LABEL_BYTES: usize = 96;

pub(super) fn execute_reference_workflow(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Option<CreativeJobRecord>, McpError> {
    let Some(workflow_id) = job.workflow_id.as_deref() else {
        return Ok(None);
    };
    let (field, role, variant_kind) = match workflow_id {
        "character_turnaround" => ("views", "character_reference_interpreted", "view"),
        "expression_sheet" => (
            "expressions",
            "character_expression_interpreted",
            "expression",
        ),
        _ => return Ok(None),
    };
    let labels = variant_labels(&job.execution_parameters, field)?;
    let reference_ids = id_array(&job.execution_parameters, "reference_asset_ids", 1, 16)?;
    let parent_asset_id = reference_ids[0].clone();
    let element_id = job
        .execution_parameters
        .get("element_id")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("creative element_id is required".into()))?;
    crate::application::creative::validate_id(element_id, "element_id")?;
    let style_element_id = job
        .execution_parameters
        .get("style_element_id")
        .and_then(Value::as_str);
    if let Some(style_element_id) = style_element_id {
        crate::application::creative::validate_id(style_element_id, "style_element_id")?;
        let project = store::load_project(cwd, config, &job.project_id)?;
        let style = project
            .element(style_element_id)
            .ok_or_else(|| McpError::InvalidRequest("unknown style element".into()))?;
        if style.kind != ElementKind::Style {
            return Err(McpError::InvalidRequest(
                "style_element_id must reference a Style Element".into(),
            ));
        }
    }
    let dependency_element_ids = style_element_id
        .map(str::to_owned)
        .into_iter()
        .collect::<Vec<_>>();
    let source = load_asset_image(cwd, config, &job.project_id, &parent_asset_id)?;

    let mut rendered = Vec::with_capacity(labels.len());
    let mut total_bytes = 0u64;
    for (index, label) in labels.iter().enumerate() {
        let variant = render_variant(
            &source,
            &job.execution_parameters,
            workflow_id,
            label,
            index,
        )?;
        total_bytes = total_bytes.saturating_add(variant.bytes.len() as u64);
        if total_bytes > config.creative_max_job_output_bytes {
            let mut failed = job.clone();
            failed.status = CreativeJobStatus::Failed;
            failed.failure_code = Some("output_too_large".into());
            failed.actual_output_bytes = Some(total_bytes);
            failed.updated_at_ms = store::now_ms();
            return Ok(Some(failed));
        }
        rendered.push((label.clone(), variant));
    }

    let mut completed = job.clone();
    completed.output_asset_ids.clear();
    completed.node_runs.clear();
    for (index, (label, variant)) in rendered.into_iter().enumerate() {
        let node_id = format!("variant_{index:03}");
        let relative_path = format!(
            "creative/{}/assets/generated/{}/{}.png",
            job.project_id, job.job_id, node_id
        );
        crate::application::workspace::write_contained_bytes(
            &relative_path,
            cwd,
            &variant.bytes,
            true,
            false,
            config,
        )?;
        let (project, asset_id) = store::register_asset(
            cwd,
            config,
            &job.project_id,
            AssetRegistrationInput {
                path: relative_path,
                media_type: "image/png".into(),
                role: role.into(),
                source: AssetSource::GeneratedAsset,
                source_surface: AssetSurface::Anime,
                state: AssetState::Candidate,
                job_id: Some(job.job_id.clone()),
                parent_asset_id: Some(parent_asset_id.clone()),
                element_id: Some(element_id.to_owned()),
                dependency_element_ids: dependency_element_ids.clone(),
                metadata: AssetMetadata {
                    width: Some(variant.width),
                    height: Some(variant.height),
                    ..AssetMetadata::default()
                },
            },
        )?;
        let asset = project.asset(&asset_id).ok_or_else(|| {
            McpError::Internal("workflow output asset disappeared after registration".into())
        })?;
        completed.output_asset_ids.push(asset_id.clone());
        completed.node_runs.push(NodeRunRecord {
            node_id,
            status: CreativeJobStatus::Completed,
            output: Some(json!({
                "asset_id": asset_id,
                "asset_role": role,
                "variant_kind": variant_kind,
                "variant_label": label,
                "reference_authority": "interpreted",
                "inspection": "not_inspected",
                "parent_asset_id": parent_asset_id,
                "element_id": element_id,
                "style_element_id": style_element_id,
                "checksum_sha256": asset.checksum_sha256,
                "width": variant.width,
                "height": variant.height
            })),
            failure_code: None,
            reused: false,
            execution_batch: 0,
        });
    }
    completed.status = CreativeJobStatus::Completed;
    completed.failure_code = None;
    completed.actual_output_bytes = Some(total_bytes);
    completed.updated_at_ms = store::now_ms();
    Ok(Some(completed))
}

struct RenderedVariant {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

fn render_variant(
    source: &DynamicImage,
    params: &Value,
    workflow_id: &str,
    label: &str,
    index: usize,
) -> Result<RenderedVariant, McpError> {
    let width = source.width();
    let height = source.height();
    enforce_dimensions(width, height)?;
    let mut image = source.to_rgba8();
    apply_reference_tint(&mut image, variant_seed(params, workflow_id, label, index));
    let image = DynamicImage::ImageRgba8(image);
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|_| McpError::InvalidRequest("image encoding failed".into()))?;
    Ok(RenderedVariant {
        bytes: cursor.into_inner(),
        width,
        height,
    })
}

fn variant_labels(params: &Value, field: &str) -> Result<Vec<String>, McpError> {
    let values = params
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    if values.is_empty() || values.len() > MAX_VARIANTS {
        return Err(McpError::InvalidRequest(format!(
            "creative {field} count exceeds allowed bounds"
        )));
    }
    let mut seen = HashSet::new();
    values
        .iter()
        .map(|value| {
            let label = value
                .as_str()
                .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is invalid")))?;
            if label.is_empty()
                || label.len() > MAX_VARIANT_LABEL_BYTES
                || label.chars().any(char::is_control)
                || !seen.insert(label)
            {
                return Err(McpError::InvalidRequest(format!(
                    "creative {field} label is invalid or duplicated"
                )));
            }
            Ok(label.to_owned())
        })
        .collect()
}

fn variant_seed(params: &Value, workflow_id: &str, label: &str, index: usize) -> [u8; 32] {
    let mut context = Context::new(&SHA256);
    if let Some(prompt) = params.get("prompt").and_then(Value::as_str) {
        context.update(prompt.as_bytes());
    }
    context.update(&[0]);
    context.update(workflow_id.as_bytes());
    context.update(&[0]);
    context.update(label.as_bytes());
    context.update(&[0]);
    context.update(index.to_string().as_bytes());
    let hash = context.finish();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(hash.as_ref());
    seed
}
