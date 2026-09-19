use super::bridge;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

const MAX_STRUCTURED_RESULT_BYTES: usize = 1024 * 1024;

pub async fn inspect(
    config: &ServerConfig,
    scope: &str,
    target: Option<&str>,
    detail: &str,
) -> Result<Value, McpError> {
    let scope_literal = py_string(scope)?;
    let target_literal = match target {
        Some(value) => py_string(value)?,
        None => "None".into(),
    };
    let detail_literal = py_string(detail)?;
    let code = format!(
        r#"# MASIHAWAM_INSPECT:{scope}
import bpy
_scope = {scope_literal}
_target = {target_literal}
_detail = {detail_literal}

def _vec(value):
    return [float(v) for v in value]

def _object(name, scope):
    if name:
        return bpy.data.objects.get(name)
    active = bpy.context.active_object
    if scope not in {{"rig", "animation", "character"}}:
        return active
    armatures = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
    if scope == "animation":
        animated = [
            o for o in armatures
            if o.animation_data is not None and o.animation_data.action is not None
        ]
        if animated:
            return animated[0]
    reviewed = [
        o for o in armatures
        if bool(o.get("masihawam_rig_strategy", False))
        or o.name.startswith("Ari_Rig")
        or o.name.startswith("MasihAwam_Rig")
    ]
    if reviewed:
        return reviewed[0]
    if armatures:
        return armatures[0]
    return active

def _limit(values, limit=128):
    return list(values)[:limit]

def _action_fcurve_count(action):
    if action is None:
        return 0
    legacy = getattr(action, "fcurves", None)
    if legacy is not None:
        try:
            return len(legacy)
        except (AttributeError, TypeError):
            pass
    total = 0
    for layer in getattr(action, "layers", []):
        for strip in getattr(layer, "strips", []):
            for channelbag in getattr(strip, "channelbags", []):
                fcurves = getattr(channelbag, "fcurves", None)
                if fcurves is not None:
                    total += len(fcurves)
    return total

obj = _object(_target, _scope)
scene = bpy.context.scene
if _scope == "scene":
    result = {{
        "scope": _scope,
        "detail": _detail,
        "scene": scene.name if scene else None,
        "frame_start": scene.frame_start if scene else None,
        "frame_end": scene.frame_end if scene else None,
        "frame_current": scene.frame_current if scene else None,
        "camera": scene.camera.name if scene and scene.camera else None,
        "objects": [{{"name": o.name, "type": o.type}} for o in _limit(scene.objects if scene else [])],
    }}
elif _scope == "object":
    result = {{
        "scope": _scope,
        "object": None if obj is None else {{
            "name": obj.name,
            "type": obj.type,
            "location": _vec(obj.location),
            "rotation_euler": _vec(obj.rotation_euler),
            "scale": _vec(obj.scale),
            "visible_viewport": not obj.hide_viewport,
            "visible_render": not obj.hide_render,
        }}
    }}
elif _scope == "mesh":
    mesh = obj.data if obj is not None and obj.type == "MESH" else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "mesh": None if mesh is None else {{
            "name": mesh.name,
            "vertices": len(mesh.vertices),
            "edges": len(mesh.edges),
            "polygons": len(mesh.polygons),
            "materials": [m.name if m else None for m in _limit(mesh.materials)],
        }}
    }}
elif _scope == "uv":
    mesh = obj.data if obj is not None and obj.type == "MESH" else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "layers": [] if mesh is None else [layer.name for layer in _limit(mesh.uv_layers)],
        "active": None if mesh is None or mesh.uv_layers.active is None else mesh.uv_layers.active.name,
    }}
elif _scope == "rig":
    armature = obj.data if obj is not None and obj.type == "ARMATURE" else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "bones": [] if armature is None else [
            {{"name": b.name, "parent": b.parent.name if b.parent else None}}
            for b in _limit(armature.bones)
        ],
    }}
elif _scope == "animation":
    animation = obj.animation_data if obj is not None else None
    action = animation.action if animation else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "action": action.name if action else None,
        "frame_range": None if action is None else [float(action.frame_range[0]), float(action.frame_range[1])],
        "fcurves": _action_fcurve_count(action),
        "nla_tracks": 0 if animation is None else len(animation.nla_tracks),
    }}
elif _scope == "material":
    material = bpy.data.materials.get(_target) if _target else None
    if material is None and obj is not None and len(obj.material_slots) > 0:
        material = obj.material_slots[0].material
    result = {{
        "scope": _scope,
        "material": None if material is None else {{
            "name": material.name,
            "use_nodes": bool(material.use_nodes),
            "surface_render_method": getattr(material, "surface_render_method", None),
        }}
    }}
elif _scope == "nodes":
    material = bpy.data.materials.get(_target) if _target else None
    if material is None and obj is not None and len(obj.material_slots) > 0:
        material = obj.material_slots[0].material
    tree = material.node_tree if material is not None and material.use_nodes else None
    result = {{
        "scope": _scope,
        "material": material.name if material else None,
        "nodes": [] if tree is None else [{{"name": n.name, "type": n.bl_idname}} for n in _limit(tree.nodes)],
        "links": 0 if tree is None else len(tree.links),
    }}
elif _scope == "physics":
    rigid = obj.rigid_body if obj is not None else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "modifiers": [] if obj is None else [{{"name": m.name, "type": m.type}} for m in _limit(obj.modifiers)],
        "rigid_body": None if rigid is None else {{
            "type": rigid.type,
            "mass": float(rigid.mass),
            "kinematic": bool(rigid.kinematic),
        }},
    }}
elif _scope == "asset":
    asset = getattr(obj, "asset_data", None) if obj is not None else None
    result = {{
        "scope": _scope,
        "object": obj.name if obj else None,
        "is_asset": asset is not None,
        "description": None if asset is None else getattr(asset, "description", None),
        "tags": [] if asset is None else [tag.name for tag in _limit(asset.tags)],
    }}
elif _scope == "render":
    render = scene.render if scene else None
    result = {{
        "scope": _scope,
        "engine": scene.render.engine if scene else None,
        "resolution": None if render is None else [render.resolution_x, render.resolution_y, render.resolution_percentage],
        "fps": None if render is None else float(render.fps) / float(render.fps_base),
        "frame_start": scene.frame_start if scene else None,
        "frame_end": scene.frame_end if scene else None,
    }}
elif _scope == "character":
    root = obj
    meshes = []
    armature = None
    if root is not None:
        if root.type == "ARMATURE":
            armature = root
            meshes = [
                candidate
                for candidate in _limit(scene.objects if scene else [])
                if candidate.type == "MESH" and candidate.find_armature() == root
            ]
        elif root.type == "MESH":
            meshes = [root]
            armature = root.find_armature()
    result = {{
        "scope": _scope,
        "root": root.name if root else None,
        "armature": armature.name if armature else None,
        "meshes": [m.name for m in meshes],
        "shape_keys": {{
            m.name: [] if m.data.shape_keys is None else [k.name for k in _limit(m.data.shape_keys.key_blocks)]
            for m in meshes
        }},
        "materials": {{m.name: [slot.material.name if slot.material else None for slot in _limit(m.material_slots)] for m in meshes}},
    }}
else:
    raise ValueError("unsupported inspect scope")
"#
    );
    let reply = bridge::execute(config, &code, true).await?;
    bounded_result(reply.result)
}

fn bounded_result(value: Value) -> Result<Value, McpError> {
    let encoded = serde_json::to_vec(&value)
        .map_err(|_| McpError::Internal("Blender inspection result could not be encoded".into()))?;
    if encoded.len() > MAX_STRUCTURED_RESULT_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender inspection result exceeds allowed bounds".into(),
        ));
    }
    Ok(value)
}

fn py_string(value: &str) -> Result<String, McpError> {
    if value.len() > 512 || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "Blender structured read argument exceeds allowed bounds".into(),
        ));
    }
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("Blender argument could not be encoded".into()))
}
