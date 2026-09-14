use super::WorkflowDescriptor;
use serde_json::json;

pub fn workflows() -> Vec<WorkflowDescriptor> {
    vec![
        workflow_descriptor(
            "character_turnaround",
            "Create a reusable multi-view character reference set.",
            &["image.reference_generate"],
        ),
        workflow_descriptor(
            "expression_sheet",
            "Create controlled expression references for one Character Element.",
            &["image.reference_generate"],
        ),
        workflow_descriptor(
            "pose_sheet",
            "Create controlled pose references for one Character Element.",
            &["image.reference_generate"],
        ),
        workflow_descriptor(
            "storyboard_manual",
            "Store and revise a caller-authored connected storyboard.",
            &["storyboard.store"],
        ),
        workflow_descriptor(
            "hero_frame",
            "Produce or register a selected hero frame for a shot.",
            &["image.reference_generate"],
        ),
        workflow_descriptor(
            "image_to_3d_bootstrap",
            "Bootstrap a 3D asset from approved image references.",
            &["3d.image_to_mesh"],
        ),
        workflow_descriptor(
            "character_rig_bootstrap",
            "Bootstrap a riggable or rigged character asset.",
            &["3d.rig_bootstrap"],
        ),
        workflow_descriptor(
            "image_to_video_preview",
            "Create a bounded motion preview from an approved image.",
            &["video.image_to_video"],
        ),
        workflow_descriptor(
            "image_upscale",
            "Create a lineaged upscaled image child asset.",
            &["image.upscale"],
        ),
        workflow_descriptor(
            "video_reframe",
            "Create a lineaged reframed video child asset.",
            &["video.reframe"],
        ),
        workflow_descriptor(
            "video_motion_control",
            "Apply a distinct motion reference to a character/reference image.",
            &["video.motion_control"],
        ),
        workflow_descriptor(
            "video_clip_extract",
            "Extract timestamp-lineaged clips from a contained video asset.",
            &["video.clip_extract"],
        ),
        workflow_descriptor(
            "voice_clone",
            "Create an authorized reusable voice artifact from a voice reference.",
            &["audio.voice_clone"],
        ),
        workflow_descriptor(
            "video_dub",
            "Create synchronized dubbed media from a source video and voice plan.",
            &["audio.video_dub"],
        ),
        workflow_descriptor(
            "shot_render_preview",
            "Execute one caller-authored shot through a selected backend.",
            &["video.reference_generate"],
        ),
        workflow_descriptor(
            "game_asset_batch",
            "Run caller-declared independent game asset jobs.",
            &["image.generate", "audio.sfx"],
        ),
        workflow_descriptor(
            "game_build_playtest",
            "Build then verify a browser game through reviewed runtime primitives.",
            &["game.build", "game.playtest"],
        ),
    ]
}

fn workflow_descriptor(id: &str, description: &str, required: &[&str]) -> WorkflowDescriptor {
    WorkflowDescriptor {
        workflow_id: id.to_owned(),
        version: 1,
        description: description.to_owned(),
        required_capabilities: required.iter().map(|value| (*value).to_owned()).collect(),
        input_schema: json!({"type":"object","maxProperties":64}),
        output_schema: json!({"type":"object","maxProperties":64}),
    }
}
