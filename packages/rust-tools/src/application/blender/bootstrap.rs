use super::{artifacts, assets, bridge, reads, BlenderArtifactScope};
use crate::application::creative::{
    self, AssetMetadata, AssetRegistrationInput, AssetSource, AssetState, AssetSurface,
};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::path::Path;

pub(crate) struct CharacterBootstrapRequest<'a> {
    pub project_id: &'a str,
    pub job_id: &'a str,
    pub element_id: &'a str,
    pub route: &'a str,
    pub front_asset_id: &'a str,
    pub side_asset_id: &'a str,
    pub back_asset_id: &'a str,
    pub reference_scale: f64,
    pub mesh_asset_id: Option<&'a str>,
}

pub(crate) async fn run_character_bootstrap(
    cwd: Option<&str>,
    config: &ServerConfig,
    request: CharacterBootstrapRequest<'_>,
) -> Result<Value, McpError> {
    if !matches!(request.route, "manual" | "binding" | "hybrid") {
        return Err(McpError::InvalidRequest(
            "character bootstrap route is unsupported".into(),
        ));
    }
    if !(0.1..=100.0).contains(&request.reference_scale) {
        return Err(McpError::InvalidRequest(
            "character bootstrap reference scale is outside allowed bounds".into(),
        ));
    }
    if request.route == "manual" && request.mesh_asset_id.is_some() {
        return Err(McpError::InvalidRequest(
            "manual character bootstrap cannot consume a generated mesh".into(),
        ));
    }
    if matches!(request.route, "binding" | "hybrid") && request.mesh_asset_id.is_none() {
        return Err(McpError::InvalidRequest(
            "binding-backed character bootstrap requires a generated mesh asset".into(),
        ));
    }

    let collection_name = format!("MA_Bootstrap_{}", short_id(request.job_id));
    let mut materialized_reference_asset_ids = Vec::with_capacity(3);
    let mut reference_objects = Vec::with_capacity(3);
    for (role, asset_id) in [
        ("front", request.front_asset_id),
        ("side", request.side_asset_id),
        ("back", request.back_asset_id),
    ] {
        let source = creative::require_asset(cwd, config, request.project_id, asset_id)?;
        let extension = source_extension(&source.relative_path)?;
        let target_name = format!(
            "bootstrap_{}_{}.{}",
            short_id(request.job_id),
            role,
            extension
        );
        let imported = assets::import_asset(
            cwd,
            config,
            request.project_id,
            asset_id,
            "reference",
            Some(&target_name),
            Some(&collection_name),
        )
        .await?;
        let materialized_id = imported
            .get("materialized_asset_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                McpError::Internal("Blender reference materialization returned no Asset ID".into())
            })?;
        let object_name = imported
            .pointer("/bridge_result/imported/0")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                McpError::InvalidRequest("Blender reference import returned no object".into())
            })?;
        materialized_reference_asset_ids.push(materialized_id.to_owned());
        reference_objects.push((role, object_name.to_owned()));
    }

    let mut materialized_mesh_asset_id = None;
    let mut imported_mesh_objects = Vec::new();
    if let Some(mesh_asset_id) = request.mesh_asset_id {
        let source = creative::require_asset(cwd, config, request.project_id, mesh_asset_id)?;
        let extension = source_extension(&source.relative_path)?;
        let target_name = format!("bootstrap_{}_mesh.{}", short_id(request.job_id), extension);
        let imported = assets::import_asset(
            cwd,
            config,
            request.project_id,
            mesh_asset_id,
            "asset",
            Some(&target_name),
            Some(&collection_name),
        )
        .await?;
        materialized_mesh_asset_id = imported
            .get("materialized_asset_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        imported_mesh_objects = imported
            .pointer("/bridge_result/imported")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .take(256)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if imported_mesh_objects.is_empty() {
            return Err(McpError::InvalidRequest(
                "Blender generated-mesh import returned no objects".into(),
            ));
        }
    }

    let scene_name = format!("character_bootstrap_{}.blend", short_id(request.job_id));
    let scene_relative_path =
        super::artifact_relative_path(BlenderArtifactScope::Scene, &scene_name)?;
    let scene_absolute_path = artifacts::preflight_output(cwd, config, &scene_relative_path)?;
    let scene_path_literal = artifacts::path_literal(&scene_absolute_path)?;
    let refs_literal = serde_json::to_string(
        &reference_objects
            .iter()
            .map(|(role, name)| json!({"role":role,"name":name}))
            .collect::<Vec<_>>(),
    )
    .map_err(|_| McpError::Internal("Blender reference alignment encoding failed".into()))?;
    let imported_literal = serde_json::to_string(&imported_mesh_objects)
        .map_err(|_| McpError::Internal("Blender mesh-object encoding failed".into()))?;
    let route_literal = serde_json::to_string(request.route)
        .map_err(|_| McpError::Internal("Blender bootstrap route encoding failed".into()))?;
    let collection_literal = serde_json::to_string(&collection_name)
        .map_err(|_| McpError::Internal("Blender collection encoding failed".into()))?;

    let code = format!(
        r#"# MASIHAWAM_CHARACTER_BOOTSTRAP
import bpy
import math
_refs = {refs_literal}
_imported = {imported_literal}
_route = {route_literal}
_reference_scale = {reference_scale}
_collection_name = {collection_literal}
_scene_path = {scene_path_literal}

_transforms = {{
    "front": ((0.0, -_reference_scale, 0.0), (math.pi / 2.0, 0.0, 0.0)),
    "side": ((_reference_scale, 0.0, 0.0), (math.pi / 2.0, 0.0, math.pi / 2.0)),
    "back": ((0.0, _reference_scale, 0.0), (math.pi / 2.0, 0.0, math.pi)),
}}
for _entry in _refs:
    _obj = bpy.data.objects.get(_entry["name"])
    if _obj is None:
        raise ValueError("bootstrap reference object is missing")
    _location, _rotation = _transforms[_entry["role"]]
    _obj.location = _location
    _obj.rotation_euler = _rotation
    if hasattr(_obj, "empty_display_size"):
        _obj.empty_display_size = _reference_scale

_bootstrap_objects = []
if _route == "manual":
    bpy.ops.mesh.primitive_cube_add(location=(0.0, 0.0, 1.0), scale=(0.55, 0.32, 1.0))
    _torso = bpy.context.object
    _torso.name = "MA_Blockout_Torso"
    bpy.ops.mesh.primitive_uv_sphere_add(location=(0.0, 0.0, 2.25), scale=(0.48, 0.48, 0.58))
    _head = bpy.context.object
    _head.name = "MA_Blockout_Head"
    _bootstrap_objects = [_torso, _head]
else:
    _bootstrap_objects = [bpy.data.objects.get(name) for name in _imported]
    _bootstrap_objects = [obj for obj in _bootstrap_objects if obj is not None]
    if not _bootstrap_objects:
        raise ValueError("bootstrap mesh objects are missing")

_meshes = [obj for obj in _bootstrap_objects if obj.type == 'MESH']
if not _meshes:
    raise ValueError("bootstrap contains no mesh objects")
for _obj in bpy.context.selected_objects:
    _obj.select_set(False)
for _obj in _bootstrap_objects:
    _obj.select_set(True)
bpy.context.view_layer.objects.active = _meshes[0]

_points = []
for _mesh in _meshes:
    for _corner in _mesh.bound_box:
        _points.append(_mesh.matrix_world @ __import__('mathutils').Vector(_corner))
if not _points:
    raise ValueError("bootstrap mesh bounds are unavailable")
_min_x = min(p.x for p in _points)
_max_x = max(p.x for p in _points)
_min_y = min(p.y for p in _points)
_max_y = max(p.y for p in _points)
_min_z = min(p.z for p in _points)
_max_z = max(p.z for p in _points)
_extent_x = float(_max_x - _min_x)
_extent_y = float(_max_y - _min_y)
_extent_z = float(_max_z - _min_z)

bpy.ops.wm.save_as_mainfile(filepath=_scene_path)
result = {{
    "route": _route,
    "collection": _collection_name,
    "reference_objects": {{entry["role"]: entry["name"] for entry in _refs}},
    "bootstrap_objects": [obj.name for obj in _bootstrap_objects],
    "silhouette_extent_evidence": {{
        "front": [_extent_x, _extent_z],
        "side": [_extent_y, _extent_z],
        "back": [_extent_x, _extent_z],
    }},
    "visual_inspection": "not_inspected",
}}
"#,
        reference_scale = request.reference_scale
    );
    let reply = bridge::execute(config, &code, true).await?;
    artifacts::verify_output(cwd, config, &scene_relative_path)?;

    let parent_asset_id = request
        .mesh_asset_id
        .unwrap_or(request.front_asset_id)
        .to_owned();
    let scene_asset_id = creative::register_internal_asset(
        cwd,
        config,
        request.project_id,
        AssetRegistrationInput {
            path: scene_relative_path.clone(),
            media_type: "application/x-blender".into(),
            role: "blender_character_bootstrap".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Blender,
            state: AssetState::Candidate,
            job_id: Some(request.job_id.to_owned()),
            parent_asset_id: Some(parent_asset_id),
            element_id: Some(request.element_id.to_owned()),
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some("blender_character_bootstrap_scene".into()),
                artifact_version: Some("character-bootstrap-v1".into()),
                ..AssetMetadata::default()
            },
        },
    )?;

    let active_target = reply
        .result
        .get("bootstrap_objects")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str);
    let inspection = reads::inspect(config, "character", active_target, "standard").await?;

    Ok(json!({
        "route":request.route,
        "scene_asset_id":scene_asset_id,
        "scene_relative_path":scene_relative_path,
        "materialized_reference_asset_ids":materialized_reference_asset_ids,
        "materialized_mesh_asset_id":materialized_mesh_asset_id,
        "bridge_result":reply.result,
        "character_inspection":inspection,
        "reference_alignment":{
            "front_asset_id":request.front_asset_id,
            "side_asset_id":request.side_asset_id,
            "back_asset_id":request.back_asset_id,
            "reference_scale":request.reference_scale
        }
    }))
}

fn short_id(value: &str) -> &str {
    value.rsplit('_').next().unwrap_or(value)
}

fn source_extension(relative_path: &str) -> Result<String, McpError> {
    Path::new(relative_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 16
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .ok_or_else(|| McpError::InvalidRequest("creative asset extension is invalid".into()))
}
