use super::WorkflowDescriptor;
use serde_json::{json, Value};

mod extended;

pub fn workflows() -> Vec<WorkflowDescriptor> {
    let mut items = vec![
        workflow(
            "character_turnaround",
            "Create a reusable multi-view character reference set.",
            &["image.reference_generate"],
            json!({
                "type":"object",
                "properties":{
                    "element_id": id_schema(),
                    "style_element_id": id_schema(),
                    "reference_asset_ids": id_array(1, 16),
                    "views":{"type":"array","minItems":2,"maxItems":12,"uniqueItems":true,"items":short_string(64)},
                    "prompt": free_text(4096)
                },
                "required":["element_id","reference_asset_ids","views"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "expression_sheet",
            "Create controlled expression references for one Character Element.",
            &["image.reference_generate"],
            json!({
                "type":"object",
                "properties":{
                    "element_id": id_schema(),
                    "style_element_id": id_schema(),
                    "reference_asset_ids": id_array(1, 16),
                    "expressions":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,"items":short_string(96)}
                },
                "required":["element_id","reference_asset_ids","expressions"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "pose_sheet",
            "Create controlled pose references for one Character Element.",
            &["image.reference_generate"],
            json!({
                "type":"object",
                "properties":{
                    "element_id": id_schema(),
                    "reference_asset_ids": id_array(1, 16),
                    "poses":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,"items":free_text(256)}
                },
                "required":["element_id","reference_asset_ids","poses"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "storyboard_auto",
            "Store an upper-layer-generated storyboard with auto planning provenance; MCP does not plan it.",
            &["storyboard.store"],
            storyboard_schema("auto"),
            storyboard_output(),
        ),
        workflow(
            "storyboard_manual",
            "Store and revise a caller-authored connected storyboard.",
            &["storyboard.store"],
            storyboard_schema("manual"),
            storyboard_output(),
        ),
        workflow(
            "hero_frame",
            "Produce or register a selected hero frame for a shot.",
            &["image.reference_generate"],
            json!({
                "type":"object",
                "properties":{
                    "scene_id": id_schema(),
                    "shot_id": id_schema(),
                    "reference_asset_ids": id_array(1, 32),
                    "element_ids": id_array(0, 32),
                    "prompt": free_text(4096)
                },
                "required":["scene_id","shot_id","reference_asset_ids"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "image_to_3d_bootstrap",
            "Bootstrap a Character Element into a contained Blender production setup using the caller-selected manual, binding, or hybrid route.",
            &["3d.image_to_mesh"],
            json!({
                "type":"object",
                "properties":{
                    "route":{"type":"string","enum":["manual","binding","hybrid"]},
                    "reference_asset_ids": id_array(3, 16),
                    "element_id": id_schema(),
                    "target_role": short_string(64),
                    "alignment":{
                        "type":"object",
                        "properties":{
                            "front_asset_id":id_schema(),
                            "side_asset_id":id_schema(),
                            "back_asset_id":id_schema(),
                            "reference_scale":{"type":"number","minimum":0.1,"maximum":100.0,"default":2.0}
                        },
                        "required":["front_asset_id","side_asset_id","back_asset_id"],
                        "additionalProperties":false
                    }
                },
                "required":["route","reference_asset_ids","element_id","alignment"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_mesh_production",
            "Turn a contained Blender bootstrap into a deformation-ready mesh/UV/toon-material character candidate while preserving Character/Style authority.",
            &[],
            json!({
                "type":"object",
                "properties":{
                    "source_asset_id":id_schema(),
                    "element_id":id_schema(),
                    "style_element_id":id_schema(),
                    "strategy":{"type":"string","enum":["repair","retopo","manual_cleanup"]}
                },
                "required":["source_asset_id","element_id","style_element_id","strategy"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_rig_production",
            "Create a reusable reviewed humanoid rig candidate with armature binding and initial facial/viseme controls.",
            &[],
            json!({
                "type":"object",
                "properties":{"source_asset_id":id_schema(),"element_id":id_schema()},
                "required":["source_asset_id","element_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_action",
            "Author one bounded editable Blender action on a contained rigged character asset.",
            &[],
            json!({
                "type":"object",
                "properties":{
                    "source_asset_id":id_schema(),
                    "action_name":short_string(96),
                    "start_frame":{"type":"integer","minimum":1,"maximum":100000},
                    "end_frame":{"type":"integer","minimum":2,"maximum":100000}
                },
                "required":["source_asset_id","action_name","start_frame","end_frame"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_facial_performance",
            "Author bounded editable viseme/facial animation from a contained audio Asset without rebuilding body animation.",
            &[],
            json!({
                "type":"object",
                "properties":{
                    "source_asset_id":id_schema(),
                    "audio_asset_id":id_schema(),
                    "start_frame":{"type":"integer","minimum":1,"maximum":100000},
                    "end_frame":{"type":"integer","minimum":2,"maximum":100000}
                },
                "required":["source_asset_id","audio_asset_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_secondary_motion",
            "Record and inspect a bounded secondary-motion choice for hair/clothing, including deliberate omission.",
            &[],
            json!({
                "type":"object",
                "properties":{
                    "source_asset_id":id_schema(),
                    "mode":{"type":"string","enum":["authored","omitted"]},
                    "reason":free_text(1024)
                },
                "required":["source_asset_id","mode","reason"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "character_rig_bootstrap",
            "Bootstrap a riggable or rigged character asset.",
            &["3d.rig_bootstrap"],
            single_asset_schema(&["element_id"]),
            asset_output(),
        ),
        workflow(
            "image_to_video_preview",
            "Create a bounded motion preview from an approved image.",
            &["video.image_to_video"],
            json!({
                "type":"object",
                "properties":{
                    "asset_id": id_schema(),
                    "duration_ms":{"type":"integer","minimum":250,"maximum":30000},
                    "prompt":free_text(4096)
                },
                "required":["asset_id","duration_ms"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "lipsync_preview",
            "Create a bounded facial/lipsync preview from contained audio timing and an animation target.",
            &["audio.voice"],
            json!({
                "type":"object",
                "properties":{
                    "audio_asset_id":id_schema(),
                    "animation_element_id":id_schema(),
                    "language":short_string(32)
                },
                "required":["audio_asset_id","animation_element_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "image_upscale",
            "Create a lineaged upscaled image child asset.",
            &["image.upscale"],
            resize_schema(),
            asset_output(),
        ),
        workflow(
            "video_upscale",
            "Create a lineaged upscaled video child asset.",
            &["video.upscale"],
            resize_schema(),
            asset_output(),
        ),
        workflow(
            "image_remove_background",
            "Create a lineaged image with background removed.",
            &["image.remove_background"],
            single_asset_schema(&[]),
            asset_output(),
        ),
        workflow(
            "video_remove_background",
            "Create a lineaged video with background removed.",
            &["video.remove_background"],
            single_asset_schema(&[]),
            asset_output(),
        ),
        workflow(
            "image_outpaint",
            "Extend an image canvas while preserving parent lineage.",
            &["image.outpaint"],
            resize_schema(),
            asset_output(),
        ),
        workflow(
            "video_reframe",
            "Create a lineaged reframed video child asset.",
            &["video.reframe"],
            json!({
                "type":"object",
                "properties":{
                    "asset_id":id_schema(),
                    "aspect_ratio":{"type":"string","minLength":3,"maxLength":16,"pattern":"^[0-9]+:[0-9]+$"}
                },
                "required":["asset_id","aspect_ratio"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "video_motion_control",
            "Apply a distinct motion-reference video to a character/reference image.",
            &["video.motion_control"],
            json!({
                "type":"object",
                "properties":{
                    "reference_asset_id":id_schema(),
                    "motion_asset_id":id_schema(),
                    "duration_ms":{"type":"integer","minimum":250,"maximum":30000}
                },
                "required":["reference_asset_id","motion_asset_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "video_clip_extract",
            "Extract timestamp-lineaged clips from a contained video asset.",
            &["video.clip_extract"],
            json!({
                "type":"object",
                "properties":{
                    "asset_id":id_schema(),
                    "start_ms":{"type":"integer","minimum":0,"maximum":86400000},
                    "end_ms":{"type":"integer","minimum":1,"maximum":86400000}
                },
                "required":["asset_id","start_ms","end_ms"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
    ];
    items.extend(extended::workflows());
    items
}

pub(super) fn workflow(
    id: &str,
    description: &str,
    required: &[&str],
    input_schema: Value,
    output_schema: Value,
) -> WorkflowDescriptor {
    WorkflowDescriptor {
        workflow_id: id.to_owned(),
        version: 1,
        description: description.to_owned(),
        required_capabilities: required.iter().map(|value| (*value).to_owned()).collect(),
        input_schema,
        output_schema,
    }
}

pub(super) fn id_schema() -> Value {
    json!({"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$"})
}

pub(super) fn id_array(min: usize, max: usize) -> Value {
    json!({
        "type":"array",
        "minItems":min,
        "maxItems":max,
        "uniqueItems":true,
        "items":id_schema()
    })
}

pub(super) fn short_string(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}

pub(super) fn free_text(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}

pub(super) fn asset_output() -> Value {
    json!({
        "type":"object",
        "properties":{"asset_ids":id_array(1,128)},
        "required":["asset_ids"],
        "additionalProperties":false
    })
}

pub(super) fn storyboard_output() -> Value {
    json!({
        "type":"object",
        "properties":{"scene_id":id_schema(),"frame_asset_ids":id_array(1,256)},
        "required":["scene_id","frame_asset_ids"],
        "additionalProperties":false
    })
}

pub(super) fn storyboard_schema(mode: &str) -> Value {
    json!({
        "type":"object",
        "properties":{
            "scene_id":id_schema(),
            "planning_mode":{"const":mode},
            "frames":{
                "type":"array",
                "minItems":1,
                "maxItems":256,
                "items":{
                    "type":"object",
                    "properties":{
                        "frame_id":id_schema(),
                        "direction":free_text(2048),
                        "element_ids":id_array(0,32)
                    },
                    "required":["frame_id","direction"],
                    "additionalProperties":false
                }
            }
        },
        "required":["scene_id","planning_mode","frames"],
        "additionalProperties":false
    })
}

pub(super) fn single_asset_schema(extra_optional_ids: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert("asset_id".into(), id_schema());
    for key in extra_optional_ids {
        properties.insert((*key).into(), id_schema());
    }
    json!({
        "type":"object",
        "properties":Value::Object(properties),
        "required":["asset_id"],
        "additionalProperties":false
    })
}

pub(super) fn resize_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "asset_id":id_schema(),
            "width":{"type":"integer","minimum":1,"maximum":16384},
            "height":{"type":"integer","minimum":1,"maximum":16384}
        },
        "required":["asset_id","width","height"],
        "additionalProperties":false
    })
}
