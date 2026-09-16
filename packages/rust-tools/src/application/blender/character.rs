use super::{assets, bridge, checkpoints, reads};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};

mod animation;

pub async fn run_character_workflow(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    workflow_id: &str,
    parameters: &Value,
) -> Result<Value, McpError> {
    match workflow_id {
        "character_mesh_production" => {
            mesh_production(cwd, config, owner, project_id, parameters).await
        }
        "character_rig_production" => {
            rig_production(cwd, config, owner, project_id, parameters).await
        }
        "character_action" => animation::action(cwd, config, owner, project_id, parameters).await,
        "character_facial_performance" => {
            animation::facial_performance(cwd, config, owner, project_id, parameters).await
        }
        "character_secondary_motion" => {
            animation::secondary_motion(cwd, config, owner, project_id, parameters).await
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported Blender character workflow".into(),
        )),
    }
}

async fn mesh_production(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    parameters: &Value,
) -> Result<Value, McpError> {
    let source_asset_id = required_str(parameters, "source_asset_id")?;
    let character_element_id = required_str(parameters, "element_id")?;
    let style_element_id = required_str(parameters, "style_element_id")?;
    let strategy = required_str(parameters, "strategy")?;
    let imported = assets::import_asset(
        cwd,
        config,
        project_id,
        source_asset_id,
        "asset",
        None,
        Some("CharacterProduction"),
    )
    .await?;
    let imported_names = imported_names(&imported)?;
    let names_literal = json_literal(&imported_names)?;
    let strategy_literal = py_string(strategy)?;
    let code = format!(
        r#"# MASIHAWAM_CHARACTER_MESH_PRODUCTION
import bpy
_names = {names_literal}
_strategy = {strategy_literal}
_imported_meshes = [bpy.data.objects.get(n) for n in _names]
_imported_meshes = [o for o in _imported_meshes if o is not None and o.type == 'MESH']
_tagged_meshes = [o for o in _imported_meshes if bool(o.get('masihawam_character_bootstrap', False))]
_meshes = _tagged_meshes or _imported_meshes
if not _meshes:
    _meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH' and bool(o.get('masihawam_character_bootstrap', False))][:16]
if not _meshes:
    raise ValueError('character mesh production requires at least one mesh')
for _obj in _meshes:
    bpy.context.view_layer.objects.active = _obj
    _obj.select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    if _obj.data.uv_layers.active is None:
        _obj.data.uv_layers.new(name='UVMap')
    _obj['masihawam_topology_strategy'] = _strategy
    _obj['masihawam_deformation_ready_candidate'] = True
    for _poly in _obj.data.polygons:
        _poly.use_smooth = True
    _obj.select_set(False)
_mat = bpy.data.materials.get('MasihAwam_Toon') or bpy.data.materials.new('MasihAwam_Toon')
_mat.use_nodes = True
_nodes = _mat.node_tree.nodes
_bsdf = _nodes.get('Principled BSDF')
if _bsdf:
    _bsdf.inputs['Roughness'].default_value = 0.82
    if 'Base Color' in _bsdf.inputs:
        _bsdf.inputs['Base Color'].default_value = (0.72, 0.78, 0.88, 1.0)
for _obj in _meshes:
    if len(_obj.data.materials) == 0:
        _obj.data.materials.append(_mat)
result = {{
    'meshes':[o.name for o in _meshes],
    'uv_ready':all(o.data.uv_layers.active is not None for o in _meshes),
    'material':'MasihAwam_Toon',
    'strategy':_strategy,
    'vertex_counts':{{o.name:len(o.data.vertices) for o in _meshes}},
    'polygon_counts':{{o.name:len(o.data.polygons) for o in _meshes}},
}}
"#
    );
    let authored = bridge::execute(config, &code, true).await?;
    let root = authored
        .result
        .get("meshes")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str);
    let mesh = reads::inspect(config, "mesh", root, "production").await?;
    let uv = reads::inspect(config, "uv", root, "production").await?;
    let material = reads::inspect(config, "material", root, "production").await?;
    let character = reads::inspect(config, "character", root, "production").await?;
    let checkpoint =
        checkpoints::create(cwd, config, owner, project_id, Some("mesh-production")).await?;
    let selection = authored
        .result
        .get("meshes")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("character mesh evidence is missing".into()))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let file_name = format!("{character_element_id}_production.glb");
    let exported = assets::export_asset(
        cwd, config, project_id, &selection, "glb", "export", &file_name,
    )
    .await?;
    Ok(json!({
        "workflow":"character_mesh_production",
        "element_id":character_element_id,
        "style_element_id":style_element_id,
        "source_asset_id":source_asset_id,
        "authored":authored.result,
        "inspection":{"mesh":mesh,"uv":uv,"material":material,"character":character},
        "checkpoint":checkpoint,
        "export":exported,
        "visual_inspection":"not_inspected"
    }))
}

async fn rig_production(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    parameters: &Value,
) -> Result<Value, McpError> {
    let source_asset_id = required_str(parameters, "source_asset_id")?;
    let character_element_id = required_str(parameters, "element_id")?;
    let imported = assets::import_asset(
        cwd,
        config,
        project_id,
        source_asset_id,
        "asset",
        None,
        Some("CharacterRig"),
    )
    .await?;
    let names = imported_names(&imported)?;
    let names_literal = json_literal(&names)?;
    let code = format!(
        r#"# MASIHAWAM_CHARACTER_RIG_PRODUCTION
import bpy
from mathutils import Vector
_names = {names_literal}
_meshes = [bpy.data.objects.get(n) for n in _names]
_meshes = [o for o in _meshes if o is not None and o.type == 'MESH']
if not _meshes:
    _meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH'][:16]
if not _meshes:
    raise ValueError('character rig requires a mesh')
_arm_data = bpy.data.armatures.get('MasihAwam_Rig') or bpy.data.armatures.new('MasihAwam_Rig')
_arm = bpy.data.objects.get('MasihAwam_Rig') or bpy.data.objects.new('MasihAwam_Rig', _arm_data)
if not _arm.users_collection:
    bpy.context.scene.collection.objects.link(_arm)
bpy.context.view_layer.objects.active = _arm
_arm.select_set(True)
bpy.ops.object.mode_set(mode='EDIT')
if len(_arm_data.edit_bones) == 0:
    _defs = [
        ('root',(0,0,0),(0,0,0.3),None),
        ('spine',(0,0,0.3),(0,0,1.1),'root'),
        ('head',(0,0,1.1),(0,0,1.55),'spine'),
        ('arm.L',(0,0,1.05),(0.65,0,1.0),'spine'),
        ('arm.R',(0,0,1.05),(-0.65,0,1.0),'spine'),
        ('leg.L',(0.18,0,0.35),(0.18,0,-0.75),'root'),
        ('leg.R',(-0.18,0,0.35),(-0.18,0,-0.75),'root'),
    ]
    _created = {{}}
    for _name,_head,_tail,_parent in _defs:
        _b = _arm_data.edit_bones.new(_name)
        _b.head = _head
        _b.tail = _tail
        if _parent:
            _b.parent = _created[_parent]
        _created[_name] = _b
bpy.ops.object.mode_set(mode='OBJECT')
for _obj in bpy.context.selected_objects:
    _obj.select_set(False)
for _mesh in _meshes:
    _mesh.select_set(True)
_arm.select_set(True)
bpy.context.view_layer.objects.active = _arm
_weighting_strategy = 'automatic_weights'
try:
    bpy.ops.object.parent_set(type='ARMATURE_AUTO')
except RuntimeError:
    _weighting_strategy = 'bounded_object_groups_fallback'
    for _mesh in _meshes:
        for _group in list(_mesh.vertex_groups):
            _mesh.vertex_groups.remove(_group)
        _bone_name = 'head' if 'head' in _mesh.name.lower() else ('spine' if 'torso' in _mesh.name.lower() else 'root')
        _group = _mesh.vertex_groups.new(name=_bone_name)
        _group.add(list(range(len(_mesh.data.vertices))), 1.0, 'REPLACE')
        _mesh.parent = _arm
for _mesh in _meshes:
    _mod = next((m for m in _mesh.modifiers if m.type == 'ARMATURE'), None)
    if _mod is None:
        _mod = _mesh.modifiers.new('MasihAwam_Armature','ARMATURE')
    _mod.object = _arm
    if _mesh.data.shape_keys is None:
        _mesh.shape_key_add(name='Basis')
    for _shape in ['blink_L','blink_R','jaw_open','viseme_A','viseme_I','viseme_U','viseme_E','viseme_O']:
        if _mesh.data.shape_keys.key_blocks.get(_shape) is None:
            _mesh.shape_key_add(name=_shape)
_arm['masihawam_rig_strategy'] = 'reviewed_humanoid_v1'
_arm['masihawam_weighting_strategy'] = _weighting_strategy
_arm['masihawam_ik_fk_review'] = 'caller_editable'
result = {{
    'armature':_arm.name,
    'bones':[b.name for b in _arm.data.bones],
    'meshes':[o.name for o in _meshes],
    'shape_keys':{{o.name:[k.name for k in o.data.shape_keys.key_blocks] for o in _meshes}},
    'vertex_groups':{{o.name:[g.name for g in o.vertex_groups] for o in _meshes}},
    'weighting_strategy':_weighting_strategy,
    'deformation_inspection':'not_inspected'
}}
"#
    );
    let authored = bridge::execute(config, &code, true).await?;
    let armature = authored.result.get("armature").and_then(Value::as_str);
    let rig = reads::inspect(config, "rig", armature, "production").await?;
    let character = reads::inspect(config, "character", armature, "production").await?;
    let checkpoint =
        checkpoints::create(cwd, config, owner, project_id, Some("rig-production")).await?;
    let mut selection = authored
        .result
        .get("meshes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if let Some(armature) = armature {
        selection.push(armature.to_owned());
    }
    let exported = assets::export_asset(
        cwd,
        config,
        project_id,
        &selection,
        "glb",
        "export",
        &format!("{character_element_id}_rigged.glb"),
    )
    .await?;
    Ok(json!({
        "workflow":"character_rig_production",
        "element_id":character_element_id,
        "source_asset_id":source_asset_id,
        "authored":authored.result,
        "inspection":{"rig":rig,"character":character},
        "checkpoint":checkpoint,
        "export":exported,
        "deformation_visual_inspection":"not_inspected"
    }))
}

fn imported_names(value: &Value) -> Result<Vec<String>, McpError> {
    value
        .get("bridge_result")
        .and_then(|value| value.get("imported"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            McpError::InvalidRequest("Blender import did not report imported objects".into())
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                McpError::InvalidRequest("Blender import returned an invalid object name".into())
            })
        })
        .collect()
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, McpError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| {
            !value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| McpError::InvalidRequest(format!("character workflow {field} is invalid")))
}

fn bounded_i64(value: &Value, field: &str, min: i64, max: i64) -> Result<i64, McpError> {
    let number = value.get(field).and_then(Value::as_i64).ok_or_else(|| {
        McpError::InvalidRequest(format!("character workflow {field} is required"))
    })?;
    if number < min || number > max {
        return Err(McpError::InvalidRequest(format!(
            "character workflow {field} is outside allowed bounds"
        )));
    }
    Ok(number)
}

fn py_string(value: &str) -> Result<String, McpError> {
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("character workflow string encoding failed".into()))
}

fn json_literal<T: serde::Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("character workflow JSON encoding failed".into()))
}

fn safe_leaf(value: &str) -> Result<String, McpError> {
    let sanitized = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() || sanitized.len() > 96 {
        return Err(McpError::InvalidRequest(
            "character action name is invalid".into(),
        ));
    }
    Ok(sanitized)
}
