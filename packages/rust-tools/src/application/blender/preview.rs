use super::{
    artifact_relative_path, artifacts, bridge, resolve_project_root, session, BlenderArtifactScope,
};
use crate::application::resources;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use image::GenericImageView;
use ring::digest::{Context, SHA256};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

const MAX_PREVIEW_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ANIMATION_PREVIEW_PIXELS: u64 = 128 * 1024 * 1024;

pub async fn screenshot(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    source: &str,
    width: u32,
    height: u32,
    save_name: Option<&str>,
) -> Result<Value, McpError> {
    if !matches!(source, "viewport" | "render_result") {
        return Err(McpError::InvalidRequest(
            "unsupported Blender screenshot source".into(),
        ));
    }
    validate_dimensions(width, height)?;
    session::ensure_project_layout(cwd, config)?;
    let root = resolve_project_root(cwd, config)?;
    let file_name = screenshot_name(save_name)?;
    let relative_path = artifact_relative_path(BlenderArtifactScope::RenderPreview, &file_name)?;
    let absolute_path = root.join(&relative_path);
    preflight_output(&root, &absolute_path)?;

    let path_literal = py_string_path(&absolute_path)?;
    let source_literal = serde_json::to_string(source)
        .map_err(|_| McpError::Internal("Blender screenshot source encoding failed".into()))?;
    let code = format!(
        r#"# MASIHAWAM_SCREENSHOT
import bpy
_scene = bpy.context.scene
_path = {path_literal}
_source = {source_literal}
_old_path = _scene.render.filepath
_old_x = _scene.render.resolution_x
_old_y = _scene.render.resolution_y
_old_pct = _scene.render.resolution_percentage
_old_format = _scene.render.image_settings.file_format
try:
    _scene.render.resolution_x = {width}
    _scene.render.resolution_y = {height}
    _scene.render.resolution_percentage = 100
    _scene.render.image_settings.file_format = "PNG"
    if _source == "render_result":
        _scene.render.filepath = _path
        _op_result = bpy.ops.render.render(write_still=True)
        if "FINISHED" not in _op_result:
            raise RuntimeError("Blender render did not finish")
    else:
        _scene.render.filepath = _path
        try:
            _op_result = bpy.ops.render.opengl(write_still=True, view_context=False)
        except RuntimeError:
            _op_result = {{"CANCELLED"}}
        if "FINISHED" not in _op_result:
            bpy.ops.render.render(write_still=True)
    result = {{"written": True, "source": _source}}
finally:
    _scene.render.filepath = _old_path
    _scene.render.resolution_x = _old_x
    _scene.render.resolution_y = _old_y
    _scene.render.resolution_percentage = _old_pct
    _scene.render.image_settings.file_format = _old_format
"#
    );
    let reply = bridge::execute(config, &code, true).await?;
    let mut metadata = inspect_preview_file(&root, &absolute_path, &relative_path)?;
    let asset_id = artifacts::register_generated_output(
        cwd,
        config,
        project_id,
        artifacts::GeneratedOutputRegistration {
            relative_path: &relative_path,
            media_type: "image/png",
            role: "blender_screenshot_preview",
            artifact_kind: "blender_screenshot",
            parent_asset_id: None,
        },
    )?;
    let resource_uri = resources::creative_asset_resource_uri(cwd, config, project_id, &asset_id)?;
    if let Some(object) = metadata.as_object_mut() {
        object.insert("asset_id".into(), json!(asset_id));
        object.insert("resource_uri".into(), json!(resource_uri));
    }
    Ok(json!({
        "kind":"screenshot",
        "source":source,
        "preview":metadata,
        "bridge_result":reply.result
    }))
}

pub struct AnimationPreviewRequest<'a> {
    pub start_frame: i32,
    pub end_frame: i32,
    pub step: u32,
    pub max_frames: usize,
    pub width: u32,
    pub height: u32,
    pub save_name: Option<&'a str>,
}

pub async fn animation_preview(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    request: AnimationPreviewRequest<'_>,
) -> Result<Value, McpError> {
    validate_dimensions(request.width, request.height)?;
    if request.start_frame > request.end_frame || request.step == 0 || request.max_frames == 0 {
        return Err(McpError::InvalidRequest(
            "Blender animation preview frame range is invalid".into(),
        ));
    }
    let frames = sampled_frames(
        request.start_frame,
        request.end_frame,
        request.step,
        request.max_frames,
    );
    let total_pixels = u64::from(request.width)
        .saturating_mul(u64::from(request.height))
        .saturating_mul(frames.len() as u64);
    if total_pixels > MAX_ANIMATION_PREVIEW_PIXELS {
        return Err(McpError::InvalidRequest(
            "Blender animation preview exceeds the total pixel budget".into(),
        ));
    }

    session::ensure_project_layout(cwd, config)?;
    let root = resolve_project_root(cwd, config)?;
    let stem = animation_stem(request.save_name)?;
    let mut relative_paths = Vec::with_capacity(frames.len());
    let mut absolute_paths = Vec::with_capacity(frames.len());
    for frame in &frames {
        let file_name = format!("{stem}-{frame}.png");
        let relative_path =
            artifact_relative_path(BlenderArtifactScope::RenderPreview, &file_name)?;
        let absolute_path = root.join(&relative_path);
        preflight_output(&root, &absolute_path)?;
        relative_paths.push(relative_path);
        absolute_paths.push(absolute_path);
    }

    let paths = absolute_paths
        .iter()
        .map(|path| {
            path.to_str()
                .ok_or_else(|| McpError::InvalidRequest("Blender preview path is not UTF-8".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let paths_literal = serde_json::to_string(&paths)
        .map_err(|_| McpError::Internal("Blender animation paths could not be encoded".into()))?;
    let frames_literal = serde_json::to_string(&frames)
        .map_err(|_| McpError::Internal("Blender animation frames could not be encoded".into()))?;
    let code = format!(
        r#"# MASIHAWAM_ANIMATION_PREVIEW
import bpy
_scene = bpy.context.scene
_frames = {frames_literal}
_paths = {paths_literal}
_old_frame = _scene.frame_current
_old_path = _scene.render.filepath
_old_x = _scene.render.resolution_x
_old_y = _scene.render.resolution_y
_old_pct = _scene.render.resolution_percentage
_old_format = _scene.render.image_settings.file_format
_written = []
try:
    _scene.render.resolution_x = {width}
    _scene.render.resolution_y = {height}
    _scene.render.resolution_percentage = 100
    _scene.render.image_settings.file_format = "PNG"
    for _frame, _path in zip(_frames, _paths):
        _scene.frame_set(_frame)
        _scene.render.filepath = _path
        if bpy.app.background:
            bpy.ops.render.render(write_still=True)
        else:
            _op_result = bpy.ops.render.opengl(write_still=True, view_context=False)
            if "FINISHED" not in _op_result:
                bpy.ops.render.render(write_still=True)
        _written.append({{"frame": _frame, "path": _path}})
    result = {{"written": _written}}
finally:
    _scene.frame_set(_old_frame)
    _scene.render.filepath = _old_path
    _scene.render.resolution_x = _old_x
    _scene.render.resolution_y = _old_y
    _scene.render.resolution_percentage = _old_pct
    _scene.render.image_settings.file_format = _old_format
"#,
        width = request.width,
        height = request.height,
    );
    let reply = bridge::execute(config, &code, true).await?;
    let mut previews = Vec::with_capacity(frames.len());
    for ((frame, absolute_path), relative_path) in frames
        .iter()
        .zip(absolute_paths.iter())
        .zip(relative_paths.iter())
    {
        let mut metadata = inspect_preview_file(&root, absolute_path, relative_path)?;
        let asset_id = artifacts::register_generated_output(
            cwd,
            config,
            project_id,
            artifacts::GeneratedOutputRegistration {
                relative_path,
                media_type: "image/png",
                role: "blender_animation_preview_frame",
                artifact_kind: "blender_animation_preview_frame",
                parent_asset_id: None,
            },
        )?;
        let resource_uri =
            resources::creative_asset_resource_uri(cwd, config, project_id, &asset_id)?;
        metadata["frame"] = json!(frame);
        metadata["asset_id"] = json!(asset_id);
        metadata["resource_uri"] = json!(resource_uri);
        previews.push(metadata);
    }
    Ok(json!({
        "kind":"animation_preview",
        "sampled_frames":frames,
        "truncated":last_requested_frame(request.start_frame, request.end_frame, request.step, request.max_frames)
            .is_some_and(|last| last < request.end_frame),
        "previews":previews,
        "bridge_result":reply.result
    }))
}

fn sampled_frames(start: i32, end: i32, step: u32, max_frames: usize) -> Vec<i32> {
    let mut frames = Vec::with_capacity(max_frames.min(240));
    let mut frame = i64::from(start);
    let end = i64::from(end);
    let step = i64::from(step);
    while frame <= end && frames.len() < max_frames {
        frames.push(frame as i32);
        frame = frame.saturating_add(step);
    }
    frames
}

fn last_requested_frame(start: i32, end: i32, step: u32, max_frames: usize) -> Option<i32> {
    sampled_frames(start, end, step, max_frames).last().copied()
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), McpError> {
    if !(64..=4096).contains(&width) || !(64..=4096).contains(&height) {
        return Err(McpError::InvalidRequest(
            "Blender preview dimensions are outside allowed bounds".into(),
        ));
    }
    Ok(())
}

fn screenshot_name(value: Option<&str>) -> Result<String, McpError> {
    let name = value
        .map(str::to_owned)
        .unwrap_or_else(|| format!("screenshot-{}.png", Uuid::new_v4().simple()));
    if !name.to_ascii_lowercase().ends_with(".png") {
        return Err(McpError::InvalidRequest(
            "Blender screenshot save_name must use a .png extension".into(),
        ));
    }
    Ok(name)
}

fn animation_stem(value: Option<&str>) -> Result<String, McpError> {
    let name = value
        .map(str::to_owned)
        .unwrap_or_else(|| format!("animation-preview-{}", Uuid::new_v4().simple()));
    let stem = name.strip_suffix(".png").unwrap_or(&name);
    if stem.is_empty() || stem.len() > 140 || stem.starts_with('.') || stem.contains(['/', '\\']) {
        return Err(McpError::InvalidRequest(
            "Blender animation preview save_name is invalid".into(),
        ));
    }
    Ok(stem.to_owned())
}

fn preflight_output(project_root: &Path, path: &Path) -> Result<(), McpError> {
    let blender_root = fs::canonicalize(project_root.join("blender"))
        .map_err(|_| McpError::InvalidRequest("Blender project layout is unavailable".into()))?;
    let parent = path
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("Blender preview target has no parent".into()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|_| McpError::InvalidRequest("Blender preview parent is unavailable".into()))?;
    if !canonical_parent.starts_with(&blender_root) {
        return Err(McpError::InvalidRequest(
            "Blender preview target escapes the project Blender subtree".into(),
        ));
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Err(McpError::InvalidRequest(
            "Blender preview target already exists".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(McpError::InvalidRequest(
            "Blender preview target cannot be inspected".into(),
        )),
    }
}

fn inspect_preview_file(
    project_root: &Path,
    path: &Path,
    relative_path: &str,
) -> Result<Value, McpError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| McpError::InvalidRequest("Blender preview output is missing".into()))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_PREVIEW_FILE_BYTES
    {
        return Err(McpError::InvalidRequest(
            "Blender preview output is not a bounded regular file".into(),
        ));
    }
    let blender_root = fs::canonicalize(project_root.join("blender"))
        .map_err(|_| McpError::InvalidRequest("Blender project layout is unavailable".into()))?;
    let canonical = fs::canonicalize(path)
        .map_err(|_| McpError::InvalidRequest("Blender preview output is inaccessible".into()))?;
    if !canonical.starts_with(&blender_root) {
        return Err(McpError::InvalidRequest(
            "Blender preview output escaped the project Blender subtree".into(),
        ));
    }
    let image = image::ImageReader::open(&canonical)
        .map_err(|_| McpError::InvalidRequest("Blender preview image cannot be opened".into()))?
        .with_guessed_format()
        .map_err(|_| McpError::InvalidRequest("Blender preview image format is invalid".into()))?
        .decode()
        .map_err(|_| {
            McpError::InvalidRequest("Blender preview output is not a valid image".into())
        })?;
    let (width, height) = image.dimensions();
    if width > 4096 || height > 4096 || width == 0 || height == 0 {
        return Err(McpError::InvalidRequest(
            "Blender preview output dimensions exceed allowed bounds".into(),
        ));
    }
    let checksum_sha256 = sha256_file(&canonical)?;
    Ok(json!({
        "relative_path":relative_path,
        "media_type":"image/png",
        "bytes":metadata.len(),
        "width":width,
        "height":height,
        "checksum_sha256":checksum_sha256
    }))
}

fn sha256_file(path: &Path) -> Result<String, McpError> {
    let mut file = File::open(path)
        .map_err(|_| McpError::InvalidRequest("Blender preview output is inaccessible".into()))?;
    let mut context = Context::new(&SHA256);
    let mut remaining = MAX_PREVIEW_FILE_BYTES + 1;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let to_read = remaining.min(buffer.len() as u64) as usize;
        if to_read == 0 {
            return Err(McpError::InvalidRequest(
                "Blender preview output exceeds allowed bounds".into(),
            ));
        }
        let read = file.read(&mut buffer[..to_read]).map_err(|_| {
            McpError::InvalidRequest("Blender preview output cannot be read".into())
        })?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn py_string_path(path: &Path) -> Result<String, McpError> {
    let value = path
        .to_str()
        .ok_or_else(|| McpError::InvalidRequest("Blender preview path is not UTF-8".into()))?;
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("Blender preview path could not be encoded".into()))
}
