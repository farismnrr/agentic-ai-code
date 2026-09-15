use super::{artifacts, bridge};
use crate::application::creative;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::path::Path;

pub async fn import_asset(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
    purpose: &str,
    target_name: Option<&str>,
    collection: Option<&str>,
) -> Result<Value, McpError> {
    let source = creative::require_asset(cwd, config, project_id, asset_id)?;
    let source_extension = extension(Path::new(&source.relative_path))?;
    let target_extension = target_name
        .map(Path::new)
        .map(extension)
        .transpose()?
        .unwrap_or_else(|| source_extension.clone());
    if target_extension != source_extension {
        return Err(McpError::InvalidRequest(
            "Blender materialization cannot change the source file format".into(),
        ));
    }
    match purpose {
        "reference"
            if !matches!(
                source_extension.as_str(),
                "png" | "jpg" | "jpeg" | "exr" | "webp" | "tif" | "tiff" | "bmp"
            ) =>
        {
            return Err(McpError::InvalidRequest(
                "Blender reference import supports reviewed image formats only".into(),
            ));
        }
        "asset"
            if !matches!(
                source_extension.as_str(),
                "fbx" | "obj" | "gltf" | "glb" | "usd" | "usda" | "usdc" | "usdz" | "blend"
            ) =>
        {
            return Err(McpError::InvalidRequest(
                "Blender asset import format is unsupported".into(),
            ));
        }
        "reference" | "asset" => {}
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender asset purpose must be reference or asset".into(),
            ))
        }
    }
    let materialized =
        artifacts::materialize_asset(cwd, config, project_id, asset_id, purpose, target_name)?;
    let extension = source_extension;
    let path_literal = artifacts::path_literal(&materialized.absolute_path)?;
    let collection_literal = match collection {
        Some(value) => py_string(value, 128)?,
        None => "None".into(),
    };

    let code = if purpose == "reference" {
        format!(
            r#"# MASIHAWAM_ASSET_IMPORT_REFERENCE
import bpy
_path = {path_literal}
_collection_name = {collection_literal}
_image = bpy.data.images.load(_path, check_existing=True)
_empty = bpy.data.objects.new("Reference_" + _image.name, None)
_empty.empty_display_type = 'IMAGE'
_empty.data = _image
_collection = bpy.context.scene.collection
if _collection_name:
    _collection = bpy.data.collections.get(_collection_name)
    if _collection is None:
        _collection = bpy.data.collections.new(_collection_name)
        bpy.context.scene.collection.children.link(_collection)
_collection.objects.link(_empty)
result = {{"imported":[_empty.name], "kind":"reference_image"}}
"#
        )
    } else {
        asset_import_code(&extension, &path_literal, &collection_literal)?
    };
    let reply = bridge::execute(config, &code, true).await?;
    Ok(json!({
        "source_asset_id":materialized.source.asset_id,
        "materialized_asset_id":materialized.asset_id,
        "relative_path":materialized.relative_path,
        "bridge_result":reply.result
    }))
}

pub async fn export_asset(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    selection: &[String],
    format: &str,
    output_scope: &str,
    file_name: &str,
) -> Result<Value, McpError> {
    if selection.is_empty() || selection.len() > 128 {
        return Err(McpError::InvalidRequest(
            "Blender export selection is outside allowed bounds".into(),
        ));
    }
    let expected_extension = match format {
        "glb" => "glb",
        "gltf" => "gltf",
        "fbx" => "fbx",
        "obj" => "obj",
        "usd" => "usd",
        "usdz" => "usdz",
        _ => {
            return Err(McpError::InvalidRequest(
                "unsupported Blender export format".into(),
            ))
        }
    };
    if !file_name
        .to_ascii_lowercase()
        .ends_with(&format!(".{expected_extension}"))
    {
        return Err(McpError::InvalidRequest(
            "Blender export file extension does not match the selected format".into(),
        ));
    }
    let scope = match output_scope {
        "export" => super::BlenderArtifactScope::Export,
        "animation" if matches!(format, "glb" | "gltf" | "fbx" | "usd" | "usdz") => {
            super::BlenderArtifactScope::Animation
        }
        "animation" => {
            return Err(McpError::InvalidRequest(
                "Blender animation export requires an animation-capable reviewed format".into(),
            ))
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender export output_scope must be export or animation".into(),
            ))
        }
    };
    let relative_path = super::artifact_relative_path(scope, file_name)?;
    let absolute_path = artifacts::preflight_output(cwd, config, &relative_path)?;
    let path_literal = artifacts::path_literal(&absolute_path)?;
    let selection_literal = serde_json::to_string(selection)
        .map_err(|_| McpError::Internal("Blender export selection could not be encoded".into()))?;
    let operator = export_operator(format)?;
    let code = format!(
        r#"# MASIHAWAM_ASSET_EXPORT
import bpy
_names = {selection_literal}
_path = {path_literal}
_missing = [name for name in _names if bpy.data.objects.get(name) is None]
if _missing:
    raise ValueError("export selection contains unknown objects")
_old_active = bpy.context.view_layer.objects.active
_old_selected = [obj for obj in bpy.context.selected_objects]
try:
    bpy.ops.object.select_all(action='DESELECT')
    for _name in _names:
        bpy.data.objects[_name].select_set(True)
    bpy.context.view_layer.objects.active = bpy.data.objects[_names[0]]
    {operator}
    result = {{"exported": _names, "format": {format_literal}}}
finally:
    bpy.ops.object.select_all(action='DESELECT')
    for _obj in _old_selected:
        if _obj.name in bpy.data.objects:
            _obj.select_set(True)
    if _old_active is not None and _old_active.name in bpy.data.objects:
        bpy.context.view_layer.objects.active = _old_active
"#,
        format_literal = serde_json::to_string(format)
            .map_err(|_| McpError::Internal("Blender export format encoding failed".into()))?
    );
    let reply = bridge::execute(config, &code, true).await?;
    artifacts::verify_output(cwd, config, &relative_path)?;
    let media_type = match format {
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "fbx" => "application/vnd.autodesk.fbx",
        "obj" => "model/obj",
        "usd" => "model/vnd.usd",
        "usdz" => "model/vnd.usdz+zip",
        _ => unreachable!(),
    };
    let output_role = if output_scope == "animation" {
        "blender_animation_export"
    } else {
        "blender_export"
    };
    let artifact_kind = format!("blender_{output_scope}_{format}");
    let output_asset_id = artifacts::register_generated_output(
        cwd,
        config,
        project_id,
        artifacts::GeneratedOutputRegistration {
            relative_path: &relative_path,
            media_type,
            role: output_role,
            artifact_kind: &artifact_kind,
            parent_asset_id: None,
        },
    )?;
    Ok(json!({
        "asset_id":output_asset_id,
        "relative_path":relative_path,
        "format":format,
        "output_scope":output_scope,
        "bridge_result":reply.result
    }))
}

fn asset_import_code(
    extension: &str,
    path_literal: &str,
    collection_literal: &str,
) -> Result<String, McpError> {
    let operator = match extension {
        "fbx" => format!("bpy.ops.import_scene.fbx(filepath={path_literal})"),
        "obj" => format!("bpy.ops.wm.obj_import(filepath={path_literal})"),
        "gltf" | "glb" => format!("bpy.ops.import_scene.gltf(filepath={path_literal})"),
        "usd" | "usda" | "usdc" | "usdz" => {
            format!("bpy.ops.wm.usd_import(filepath={path_literal})")
        }
        "blend" => {
            return Ok(format!(
                r#"# MASIHAWAM_ASSET_IMPORT_BLEND
import bpy
_path = {path_literal}
_collection_name = {collection_literal}
_before = set(bpy.data.objects.keys())
with bpy.data.libraries.load(_path, link=False) as (_data_from, _data_to):
    _data_to.objects = list(_data_from.objects)[:256]
_imported = [obj for obj in _data_to.objects if obj is not None]
_collection = bpy.context.scene.collection
if _collection_name:
    _collection = bpy.data.collections.get(_collection_name)
    if _collection is None:
        _collection = bpy.data.collections.new(_collection_name)
        bpy.context.scene.collection.children.link(_collection)
for _obj in _imported:
    if len(_obj.users_collection) == 0:
        _collection.objects.link(_obj)
result = {{"imported":[obj.name for obj in _imported], "kind":"blend"}}
"#
            ));
        }
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender asset import format is unsupported".into(),
            ))
        }
    };
    Ok(format!(
        r#"# MASIHAWAM_ASSET_IMPORT
import bpy
_collection_name = {collection_literal}
_before = set(bpy.data.objects.keys())
{operator}
_imported = [bpy.data.objects[name] for name in bpy.data.objects.keys() if name not in _before]
if _collection_name:
    _collection = bpy.data.collections.get(_collection_name)
    if _collection is None:
        _collection = bpy.data.collections.new(_collection_name)
        bpy.context.scene.collection.children.link(_collection)
    for _obj in _imported:
        for _existing in list(_obj.users_collection):
            _existing.objects.unlink(_obj)
        _collection.objects.link(_obj)
result = {{"imported":[obj.name for obj in _imported], "kind":"asset"}}
"#
    ))
}

fn export_operator(format: &str) -> Result<String, McpError> {
    match format {
        "glb" => Ok("bpy.ops.export_scene.gltf(filepath=_path, export_format='GLB', use_selection=True)".into()),
        "gltf" => Ok("bpy.ops.export_scene.gltf(filepath=_path, export_format='GLTF_EMBEDDED', use_selection=True)".into()),
        "fbx" => Ok("bpy.ops.export_scene.fbx(filepath=_path, use_selection=True)".into()),
        "obj" => Ok("bpy.ops.wm.obj_export(filepath=_path, export_selected_objects=True, export_materials=False)".into()),
        "usd" | "usdz" => Ok("bpy.ops.wm.usd_export(filepath=_path, selected_objects_only=True)".into()),
        _ => Err(McpError::InvalidRequest(
            "unsupported Blender export format".into(),
        )),
    }
}

fn extension(path: &Path) -> Result<String, McpError> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            McpError::InvalidRequest("Blender asset has no reviewed file extension".into())
        })
}

fn py_string(value: &str, max: usize) -> Result<String, McpError> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "Blender structured string exceeds allowed bounds".into(),
        ));
    }
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("Blender structured string could not be encoded".into()))
}
