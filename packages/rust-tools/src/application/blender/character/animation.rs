use super::*;

pub(super) async fn action(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    parameters: &Value,
) -> Result<Value, McpError> {
    let source_asset_id = required_str(parameters, "source_asset_id")?;
    let action_name = required_str(parameters, "action_name")?;
    let start = bounded_i64(parameters, "start_frame", 1, 100_000)?;
    let end = bounded_i64(parameters, "end_frame", start + 1, 100_000)?;
    let imported = assets::import_asset(
        cwd,
        config,
        project_id,
        source_asset_id,
        "asset",
        None,
        Some("CharacterAction"),
    )
    .await?;
    let names = imported_names(&imported)?;
    let names_literal = json_literal(&names)?;
    let action_literal = py_string(action_name)?;
    let code = format!(
        r#"# MASIHAWAM_CHARACTER_ACTION
import bpy
_names = {names_literal}
_action_name = {action_literal}
_start = {start}
_end = {end}
_arm = next((bpy.data.objects.get(n) for n in _names if bpy.data.objects.get(n) and bpy.data.objects.get(n).type == 'ARMATURE'), None)
if _arm is None:
    _arm = next((o for o in bpy.context.scene.objects if o.type == 'ARMATURE'), None)
if _arm is None:
    raise ValueError('character action requires an armature')
_action = bpy.data.actions.get(_action_name) or bpy.data.actions.new(_action_name)
if _arm.animation_data is None:
    _arm.animation_data_create()
_arm.animation_data.action = _action
_arm.rotation_mode = 'XYZ'
_arm.rotation_euler[2] = 0.0
_arm.keyframe_insert(data_path='rotation_euler', frame=_start, group='MasihAwamBody')
_mid = (_start + _end) // 2
_arm.rotation_euler[2] = 0.18
_arm.keyframe_insert(data_path='rotation_euler', frame=_mid, group='MasihAwamBody')
_arm.rotation_euler[2] = 0.0
_arm.keyframe_insert(data_path='rotation_euler', frame=_end, group='MasihAwamBody')
bpy.context.scene.frame_start = min(bpy.context.scene.frame_start, _start)
bpy.context.scene.frame_end = max(bpy.context.scene.frame_end, _end)
_fcurve_count = 0
if hasattr(_action, 'fcurves'):
    _fcurve_count = len(_action.fcurves)
else:
    for _layer in getattr(_action, 'layers', []):
        for _strip in getattr(_layer, 'strips', []):
            for _bag in getattr(_strip, 'channelbags', []):
                _fcurve_count += len(getattr(_bag, 'fcurves', []))
result = {{'armature':_arm.name,'action':_action.name,'frame_start':_start,'frame_end':_end,'fcurves':_fcurve_count}}
"#
    );
    let authored = bridge::execute(config, &code, true).await?;
    let armature = authored.result.get("armature").and_then(Value::as_str);
    let animation = reads::inspect(config, "animation", armature, "production").await?;
    let checkpoint =
        checkpoints::create(cwd, config, owner, project_id, Some("character-action")).await?;
    let selection = armature.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let export = assets::export_asset(
        cwd,
        config,
        project_id,
        &selection,
        "glb",
        "animation",
        &format!("{}_action.glb", safe_leaf(action_name)?),
    )
    .await?;
    Ok(
        json!({"workflow":"character_action","source_asset_id":source_asset_id,"authored":authored.result,"inspection":animation,"checkpoint":checkpoint,"export":export,"temporal_inspection":"not_inspected"}),
    )
}

pub(super) async fn secondary_motion(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    parameters: &Value,
) -> Result<Value, McpError> {
    let source_asset_id = required_str(parameters, "source_asset_id")?;
    let mode = required_str(parameters, "mode")?;
    let reason = required_str(parameters, "reason")?;
    let imported = assets::import_asset(
        cwd,
        config,
        project_id,
        source_asset_id,
        "asset",
        None,
        Some("CharacterSecondaryMotion"),
    )
    .await?;
    let names = imported_names(&imported)?;
    let names_literal = json_literal(&names)?;
    let mode_literal = py_string(mode)?;
    let reason_literal = py_string(reason)?;
    let code = format!(
        r#"# MASIHAWAM_CHARACTER_SECONDARY_MOTION
import bpy
_names = {names_literal}
_mode = {mode_literal}
_reason = {reason_literal}
_targets = [bpy.data.objects.get(n) for n in _names]
_targets = [o for o in _targets if o is not None and o.type == 'MESH']
for _obj in _targets:
    _obj['masihawam_secondary_motion_mode'] = _mode
    _obj['masihawam_secondary_motion_reason'] = _reason
result = {{'targets':[o.name for o in _targets],'mode':_mode,'reason':_reason,'physics_state':'inspectable' if _mode == 'authored' else 'deliberately_omitted'}}
"#
    );
    let authored = bridge::execute(config, &code, true).await?;
    let target = authored
        .result
        .get("targets")
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .and_then(Value::as_str);
    let physics = reads::inspect(config, "physics", target, "production").await?;
    let checkpoint =
        checkpoints::create(cwd, config, owner, project_id, Some("secondary-motion")).await?;
    Ok(
        json!({"workflow":"character_secondary_motion","source_asset_id":source_asset_id,"authored":authored.result,"inspection":physics,"checkpoint":checkpoint,"clipping_visual_inspection":"not_inspected"}),
    )
}
