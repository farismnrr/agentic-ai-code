use super::{artifact_relative_path, artifacts, bridge, BlenderArtifactScope};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::path::Path;

const MAX_RENDER_FRAMES: usize = 120;
const MAX_RENDER_DIMENSION: u32 = 4096;

pub struct RenderRequest<'a> {
    pub mode: &'a str,
    pub output_scope: &'a str,
    pub file_name: &'a str,
    pub start_frame: Option<i32>,
    pub end_frame: Option<i32>,
}

pub async fn render(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    request: RenderRequest<'_>,
) -> Result<Value, McpError> {
    let scope = match request.output_scope {
        "preview" => BlenderArtifactScope::RenderPreview,
        "final" => BlenderArtifactScope::RenderFinal,
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender render output_scope must be preview or final".into(),
            ))
        }
    };
    match request.mode {
        "still" => render_still(cwd, config, project_id, scope, request.file_name).await,
        "animation" => {
            let start = request.start_frame.ok_or_else(|| {
                McpError::InvalidRequest("animation render requires start_frame".into())
            })?;
            let end = request.end_frame.ok_or_else(|| {
                McpError::InvalidRequest("animation render requires end_frame".into())
            })?;
            render_animation(
                cwd,
                config,
                project_id,
                scope,
                request.file_name,
                start,
                end,
            )
            .await
        }
        _ => Err(McpError::InvalidRequest(
            "Blender render mode must be still or animation".into(),
        )),
    }
}

async fn render_still(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    scope: BlenderArtifactScope,
    file_name: &str,
) -> Result<Value, McpError> {
    require_png_name(file_name)?;
    let relative_path = artifact_relative_path(scope, file_name)?;
    let absolute_path = artifacts::preflight_output(cwd, config, &relative_path)?;
    let path_literal = artifacts::path_literal(&absolute_path)?;
    let code = format!(
        r#"# MASIHAWAM_RENDER_STILL
import bpy
_scene = bpy.context.scene
if _scene.render.resolution_x > {max_dim} or _scene.render.resolution_y > {max_dim}:
    raise ValueError("render resolution exceeds relay bound")
_path = {path_literal}
_old_path = _scene.render.filepath
_old_format = _scene.render.image_settings.file_format
try:
    _scene.render.filepath = _path
    _scene.render.image_settings.file_format = "PNG"
    bpy.ops.render.render(write_still=True)
    result = {{"written": True, "frame": _scene.frame_current}}
finally:
    _scene.render.filepath = _old_path
    _scene.render.image_settings.file_format = _old_format
"#,
        max_dim = MAX_RENDER_DIMENSION,
    );
    let reply = bridge::execute(config, &code, true).await?;
    let output_asset_id = artifacts::register_generated_output(
        cwd,
        config,
        project_id,
        artifacts::GeneratedOutputRegistration {
            relative_path: &relative_path,
            media_type: "image/png",
            role: "blender_render",
            artifact_kind: "blender_render_still",
            parent_asset_id: None,
        },
    )?;
    Ok(json!({
        "mode":"still",
        "asset_id":output_asset_id,
        "relative_path":relative_path,
        "bridge_result":reply.result
    }))
}

async fn render_animation(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    scope: BlenderArtifactScope,
    file_name: &str,
    start: i32,
    end: i32,
) -> Result<Value, McpError> {
    if start > end {
        return Err(McpError::InvalidRequest(
            "Blender render frame range is invalid".into(),
        ));
    }
    let frame_count = i64::from(end)
        .saturating_sub(i64::from(start))
        .saturating_add(1) as usize;
    if frame_count == 0 || frame_count > MAX_RENDER_FRAMES {
        return Err(McpError::InvalidRequest(
            "Blender animation render exceeds the frame bound".into(),
        ));
    }
    require_png_name(file_name)?;
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| McpError::InvalidRequest("Blender render file name is invalid".into()))?;
    if stem.len() > 140 {
        return Err(McpError::InvalidRequest(
            "Blender render file name is too long for frame suffixes".into(),
        ));
    }
    let mut relative_paths = Vec::with_capacity(frame_count);
    let mut absolute_paths = Vec::with_capacity(frame_count);
    for frame in start..=end {
        let frame_name = format!("{stem}-{frame:06}.png");
        let relative = artifact_relative_path(scope, &frame_name)?;
        let absolute = artifacts::preflight_output(cwd, config, &relative)?;
        relative_paths.push(relative);
        absolute_paths.push(absolute);
    }
    let path_strings = absolute_paths
        .iter()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| McpError::InvalidRequest("Blender render path is not UTF-8".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let paths_literal = serde_json::to_string(&path_strings)
        .map_err(|_| McpError::Internal("Blender render paths could not be encoded".into()))?;
    let code = format!(
        r#"# MASIHAWAM_RENDER_ANIMATION
import bpy
_scene = bpy.context.scene
if _scene.render.resolution_x > {max_dim} or _scene.render.resolution_y > {max_dim}:
    raise ValueError("render resolution exceeds relay bound")
_paths = {paths_literal}
_frames = list(range({start}, {end_plus_one}))
_old_frame = _scene.frame_current
_old_path = _scene.render.filepath
_old_format = _scene.render.image_settings.file_format
_written = []
try:
    _scene.render.image_settings.file_format = "PNG"
    for _frame, _path in zip(_frames, _paths):
        _scene.frame_set(_frame)
        _scene.render.filepath = _path
        bpy.ops.render.render(write_still=True)
        _written.append(_frame)
    result = {{"written_frames": _written}}
finally:
    _scene.frame_set(_old_frame)
    _scene.render.filepath = _old_path
    _scene.render.image_settings.file_format = _old_format
"#,
        max_dim = MAX_RENDER_DIMENSION,
        end_plus_one = i64::from(end) + 1,
    );
    let reply = bridge::execute(config, &code, true).await?;
    let mut assets = Vec::with_capacity(frame_count);
    for relative_path in &relative_paths {
        let asset_id = artifacts::register_generated_output(
            cwd,
            config,
            project_id,
            artifacts::GeneratedOutputRegistration {
                relative_path,
                media_type: "image/png",
                role: "blender_render_frame",
                artifact_kind: "blender_render_animation_frame",
                parent_asset_id: None,
            },
        )?;
        assets.push(json!({"asset_id":asset_id,"relative_path":relative_path}));
    }
    Ok(json!({
        "mode":"animation",
        "start_frame":start,
        "end_frame":end,
        "outputs":assets,
        "bridge_result":reply.result
    }))
}

fn require_png_name(file_name: &str) -> Result<(), McpError> {
    if !file_name.to_ascii_lowercase().ends_with(".png") {
        return Err(McpError::InvalidRequest(
            "Blender render file_name must use a .png extension".into(),
        ));
    }
    Ok(())
}
