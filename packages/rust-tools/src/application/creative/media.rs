use super::contracts::{AssetMetadata, AssetSource, AssetState, AssetSurface};
use super::graph::{CreativeJobKind, CreativeJobRecord, CreativeJobStatus};
use super::registry;
use super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use image::{DynamicImage, GenericImage, ImageBuffer, ImageFormat, Rgba};
use ring::digest::{digest, SHA256};
use serde_json::Value;
use std::io::Cursor;

const MAX_RASTER_DIMENSION: u32 = 4096;
const MAX_RASTER_PIXELS: u64 = 16_777_216;
const DEFAULT_GENERATED_DIMENSION: u32 = 512;

pub fn execute_media_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Option<CreativeJobRecord>, McpError> {
    let Some(binding_id) = job.execution_binding_id.as_deref() else {
        return Ok(None);
    };
    if backend_kind(config, binding_id)? != Some("local_raster") {
        return Ok(None);
    }
    let workflow_capability = job
        .workflow_id
        .as_deref()
        .and_then(registry::workflow)
        .and_then(|workflow| {
            (workflow.required_capabilities.len() == 1)
                .then(|| workflow.required_capabilities[0].clone())
        });
    let capability_id = match job.kind {
        CreativeJobKind::Capability => job.capability_id.as_deref(),
        CreativeJobKind::Workflow => workflow_capability.as_deref(),
        CreativeJobKind::Graph => None,
    };
    let Some(capability_id) = capability_id else {
        return Ok(None);
    };
    if !matches!(
        capability_id,
        "image.generate"
            | "image.reference_generate"
            | "image.edit"
            | "image.inpaint"
            | "image.upscale"
            | "image.remove_background"
            | "image.outpaint"
    ) {
        return Ok(None);
    }

    let result = render(capability_id, cwd, config, job)?;
    let mut completed = job.clone();
    if result.bytes.len() as u64 > config.creative_max_job_output_bytes {
        completed.status = CreativeJobStatus::Failed;
        completed.failure_code = Some("output_too_large".into());
        completed.actual_output_bytes = Some(result.bytes.len() as u64);
        completed.updated_at_ms = store::now_ms();
        return Ok(Some(completed));
    }

    let relative_path = format!(
        "creative/{}/assets/generated/{}.png",
        job.project_id, job.job_id
    );
    crate::application::workspace::write_contained_bytes(
        &relative_path,
        cwd,
        &result.bytes,
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
            role: result.role,
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: result.parent_asset_id,
            element_id: result.element_id,
            metadata: AssetMetadata {
                width: Some(result.width),
                height: Some(result.height),
                ..AssetMetadata::default()
            },
        },
    )?;
    if project.asset(&asset_id).is_none() {
        return Err(McpError::Internal(
            "generated creative asset disappeared after registration".into(),
        ));
    }
    completed.status = CreativeJobStatus::Completed;
    completed.output_asset_ids = vec![asset_id];
    completed.failure_code = None;
    completed.actual_output_bytes = Some(result.bytes.len() as u64);
    completed.updated_at_ms = store::now_ms();
    Ok(Some(completed))
}

struct RenderResult {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    role: String,
    parent_asset_id: Option<String>,
    element_id: Option<String>,
}

fn render(
    capability_id: &str,
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<RenderResult, McpError> {
    let params = &job.execution_parameters;
    let element_id = optional_id(params, "element_id")?;
    let (image, parent_asset_id) = match capability_id {
        "image.generate" => {
            let (width, height) = requested_dimensions(
                params,
                DEFAULT_GENERATED_DIMENSION,
                DEFAULT_GENERATED_DIMENSION,
            )?;
            (procedural_image(width, height, prompt_seed(params)), None)
        }
        "image.reference_generate" => {
            let reference_ids = id_array(params, "reference_asset_ids", 1, 16)?;
            let parent = reference_ids[0].clone();
            let source = load_asset_image(cwd, config, &job.project_id, &parent)?;
            let (width, height) = requested_dimensions(params, source.width(), source.height())?;
            let mut rendered = source
                .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            apply_reference_tint(&mut rendered, prompt_seed(params));
            (DynamicImage::ImageRgba8(rendered), Some(parent))
        }
        "image.edit" => {
            let parent = required_id(params, "asset_id")?;
            let mut image = load_asset_image(cwd, config, &job.project_id, &parent)?.to_rgba8();
            image::imageops::colorops::invert(&mut image);
            (DynamicImage::ImageRgba8(image), Some(parent))
        }
        "image.inpaint" => {
            let parent = required_id(params, "asset_id")?;
            let mask_id = required_id(params, "mask_asset_id")?;
            let mut source = load_asset_image(cwd, config, &job.project_id, &parent)?.to_rgba8();
            let mask = load_asset_image(cwd, config, &job.project_id, &mask_id)?
                .resize_exact(
                    source.width(),
                    source.height(),
                    image::imageops::FilterType::Nearest,
                )
                .to_luma8();
            let seed = prompt_seed(params);
            for (x, y, pixel) in source.enumerate_pixels_mut() {
                if mask.get_pixel(x, y).0[0] >= 128 {
                    *pixel = procedural_pixel(x, y, &seed);
                }
            }
            (DynamicImage::ImageRgba8(source), Some(parent))
        }
        "image.upscale" => {
            let parent = required_id(params, "asset_id")?;
            let source = load_asset_image(cwd, config, &job.project_id, &parent)?;
            let (width, height) = requested_dimensions(params, source.width(), source.height())?;
            if width < source.width() || height < source.height() {
                return Err(McpError::InvalidRequest(
                    "image upscale dimensions cannot shrink the source".into(),
                ));
            }
            (
                source.resize_exact(width, height, image::imageops::FilterType::Lanczos3),
                Some(parent),
            )
        }
        "image.remove_background" => {
            let parent = required_id(params, "asset_id")?;
            let mut image = load_asset_image(cwd, config, &job.project_id, &parent)?.to_rgba8();
            for pixel in image.pixels_mut() {
                let [r, g, b, _] = pixel.0;
                if r >= 240 && g >= 240 && b >= 240 {
                    pixel.0[3] = 0;
                }
            }
            (DynamicImage::ImageRgba8(image), Some(parent))
        }
        "image.outpaint" => {
            let parent = required_id(params, "asset_id")?;
            let source = load_asset_image(cwd, config, &job.project_id, &parent)?;
            let (width, height) = requested_dimensions(params, source.width(), source.height())?;
            if width < source.width() || height < source.height() {
                return Err(McpError::InvalidRequest(
                    "image outpaint dimensions cannot shrink the source".into(),
                ));
            }
            let seed = prompt_seed(params);
            let mut canvas = procedural_image(width, height, seed).to_rgba8();
            let x = (width - source.width()) / 2;
            let y = (height - source.height()) / 2;
            canvas
                .copy_from(&source.to_rgba8(), x, y)
                .map_err(|_| McpError::Internal("image outpaint composition failed".into()))?;
            (DynamicImage::ImageRgba8(canvas), Some(parent))
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "unsupported local raster capability".into(),
            ))
        }
    };
    enforce_dimensions(image.width(), image.height())?;
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|_| McpError::InvalidRequest("image encoding failed".into()))?;
    Ok(RenderResult {
        bytes: cursor.into_inner(),
        width: image.width(),
        height: image.height(),
        role: capability_id.replace('.', "_"),
        parent_asset_id,
        element_id,
    })
}

fn backend_kind<'a>(
    config: &'a ServerConfig,
    binding_id: &str,
) -> Result<Option<&'a str>, McpError> {
    let mut result = None;
    for raw in &config.creative_binding_backends {
        let (id, kind) = raw.split_once('=').ok_or_else(|| {
            McpError::InvalidRequest("creative binding backend mapping is invalid".into())
        })?;
        super::contracts::validate_id(id, "execution_binding_id")?;
        if !matches!(kind, "local_raster") {
            return Err(McpError::InvalidRequest(
                "creative binding backend kind is unsupported".into(),
            ));
        }
        if id == binding_id && result.replace(kind).is_some() {
            return Err(McpError::InvalidRequest(
                "duplicate creative binding backend mapping".into(),
            ));
        }
    }
    Ok(result)
}

fn load_asset_image(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
) -> Result<DynamicImage, McpError> {
    let path = store::resolve_registered_asset_path(cwd, config, project_id, asset_id)?;
    let reader = image::ImageReader::open(&path)
        .map_err(|_| McpError::InvalidRequest("image asset is inaccessible".into()))?
        .with_guessed_format()
        .map_err(|_| McpError::InvalidRequest("image asset format is invalid".into()))?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| McpError::InvalidRequest("image dimensions are invalid".into()))?;
    enforce_dimensions(width, height)?;
    image::ImageReader::open(&path)
        .map_err(|_| McpError::InvalidRequest("image asset is inaccessible".into()))?
        .with_guessed_format()
        .map_err(|_| McpError::InvalidRequest("image asset format is invalid".into()))?
        .decode()
        .map_err(|_| McpError::InvalidRequest("image asset could not be decoded".into()))
}

fn requested_dimensions(
    params: &Value,
    default_width: u32,
    default_height: u32,
) -> Result<(u32, u32), McpError> {
    let width = dimension(params, "width")?.unwrap_or(default_width);
    let height = dimension(params, "height")?.unwrap_or(default_height);
    enforce_dimensions(width, height)?;
    Ok((width, height))
}

fn dimension(params: &Value, field: &str) -> Result<Option<u32>, McpError> {
    params
        .get(field)
        .map(|value| {
            let value = value.as_u64().ok_or_else(|| {
                McpError::InvalidRequest(format!("creative {field} must be an integer"))
            })?;
            u32::try_from(value)
                .map_err(|_| McpError::InvalidRequest(format!("creative {field} exceeds maximum")))
        })
        .transpose()
}

fn enforce_dimensions(width: u32, height: u32) -> Result<(), McpError> {
    if width == 0
        || height == 0
        || width > MAX_RASTER_DIMENSION
        || height > MAX_RASTER_DIMENSION
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_RASTER_PIXELS
    {
        return Err(McpError::InvalidRequest(
            "creative raster dimensions exceed allowed bounds".into(),
        ));
    }
    Ok(())
}

fn required_id(params: &Value, field: &str) -> Result<String, McpError> {
    let value = params
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    super::contracts::validate_id(value, field)?;
    Ok(value.to_owned())
}

fn optional_id(params: &Value, field: &str) -> Result<Option<String>, McpError> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(|value| {
            super::contracts::validate_id(value, field)?;
            Ok(value.to_owned())
        })
        .transpose()
}

fn id_array(params: &Value, field: &str, min: usize, max: usize) -> Result<Vec<String>, McpError> {
    let values = params
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    if values.len() < min || values.len() > max {
        return Err(McpError::InvalidRequest(format!(
            "creative {field} count exceeds allowed bounds"
        )));
    }
    values
        .iter()
        .map(|value| {
            let value = value
                .as_str()
                .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is invalid")))?;
            super::contracts::validate_id(value, field)?;
            Ok(value.to_owned())
        })
        .collect()
}

fn prompt_seed(params: &Value) -> [u8; 32] {
    let prompt = params
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let hash = digest(&SHA256, prompt.as_bytes());
    let mut seed = [0u8; 32];
    seed.copy_from_slice(hash.as_ref());
    seed
}

fn procedural_image(width: u32, height: u32, seed: [u8; 32]) -> DynamicImage {
    DynamicImage::ImageRgba8(ImageBuffer::from_fn(width, height, |x, y| {
        procedural_pixel(x, y, &seed)
    }))
}

fn procedural_pixel(x: u32, y: u32, seed: &[u8; 32]) -> Rgba<u8> {
    let a = seed[((x as usize).wrapping_add(y as usize)) % seed.len()];
    let b = seed[((x as usize).wrapping_mul(3).wrapping_add(y as usize * 5)) % seed.len()];
    let c = seed[((x as usize).wrapping_mul(7).wrapping_add(y as usize * 11)) % seed.len()];
    Rgba([a, b, c, 255])
}

fn apply_reference_tint(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, seed: [u8; 32]) {
    for pixel in image.pixels_mut() {
        pixel.0[0] = ((u16::from(pixel.0[0]) * 3 + u16::from(seed[0])) / 4) as u8;
        pixel.0[1] = ((u16::from(pixel.0[1]) * 3 + u16::from(seed[1])) / 4) as u8;
        pixel.0[2] = ((u16::from(pixel.0[2]) * 3 + u16::from(seed[2])) / 4) as u8;
    }
}
