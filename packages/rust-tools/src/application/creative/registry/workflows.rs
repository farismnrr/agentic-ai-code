use super::WorkflowDescriptor;
use serde_json::{json, Value};

pub fn workflows() -> Vec<WorkflowDescriptor> {
    vec![
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
            "Bootstrap a 3D asset from approved image references.",
            &["3d.image_to_mesh"],
            json!({
                "type":"object",
                "properties":{
                    "reference_asset_ids": id_array(1, 16),
                    "element_id": id_schema(),
                    "target_role": short_string(64)
                },
                "required":["reference_asset_ids"],
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
        workflow(
            "voice_clone",
            "Create an authorized reusable voice artifact from bounded voice references.",
            &["audio.voice_clone"],
            json!({
                "type":"object",
                "properties":{
                    "reference_asset_ids":id_array(1,16),
                    "consent_asserted":{"const":true},
                    "language":short_string(32)
                },
                "required":["reference_asset_ids","consent_asserted"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "voice_change",
            "Create a lineaged voice-changed audio child asset.",
            &["audio.voice_change"],
            json!({
                "type":"object",
                "properties":{"asset_id":id_schema(),"voice_element_id":id_schema()},
                "required":["asset_id","voice_element_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "video_dub",
            "Create synchronized dubbed media from a contained source video and voice plan.",
            &["audio.video_dub"],
            json!({
                "type":"object",
                "properties":{
                    "video_asset_id":id_schema(),
                    "voice_element_id":id_schema(),
                    "language":short_string(32)
                },
                "required":["video_asset_id","voice_element_id","language"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "shot_render_preview",
            "Execute one caller-authored shot through a selected backend.",
            &["video.reference_generate"],
            json!({
                "type":"object",
                "properties":{"scene_id":id_schema(),"shot_id":id_schema()},
                "required":["scene_id","shot_id"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "game_asset_batch",
            "Run caller-declared independent game asset jobs.",
            &["image.generate", "audio.sfx"],
            json!({
                "type":"object",
                "properties":{
                    "game_id":id_schema(),
                    "roles":{"type":"array","minItems":1,"maxItems":128,"uniqueItems":true,"items":short_string(128)}
                },
                "required":["game_id","roles"],
                "additionalProperties":false
            }),
            asset_output(),
        ),
        workflow(
            "game_build_playtest",
            "Build then verify a browser game through reviewed runtime primitives.",
            &["game.build", "game.playtest"],
            json!({
                "type":"object",
                "properties":{"game_id":id_schema(),"build_revision":id_schema()},
                "required":["game_id"],
                "additionalProperties":false
            }),
            json!({
                "type":"object",
                "properties":{
                    "build_id":id_schema(),
                    "playtest_evidence_id":id_schema()
                },
                "required":["build_id","playtest_evidence_id"],
                "additionalProperties":false
            }),
        ),
    ]
}

fn workflow(
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

fn id_schema() -> Value {
    json!({"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$"})
}

fn id_array(min: usize, max: usize) -> Value {
    json!({
        "type":"array",
        "minItems":min,
        "maxItems":max,
        "uniqueItems":true,
        "items":id_schema()
    })
}

fn short_string(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}

fn free_text(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}

fn asset_output() -> Value {
    json!({
        "type":"object",
        "properties":{"asset_ids":id_array(1,128)},
        "required":["asset_ids"],
        "additionalProperties":false
    })
}

fn storyboard_output() -> Value {
    json!({
        "type":"object",
        "properties":{"scene_id":id_schema(),"frame_asset_ids":id_array(1,256)},
        "required":["scene_id","frame_asset_ids"],
        "additionalProperties":false
    })
}

fn storyboard_schema(mode: &str) -> Value {
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

fn single_asset_schema(extra_optional_ids: &[&str]) -> Value {
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

fn resize_schema() -> Value {
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
