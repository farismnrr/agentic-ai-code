use ai_tools::application::creative::{
    AssetMetadata, AssetRecord, AssetSource, AssetState, AssetSurface, AudioCue, AudioPlan,
    CameraSpec, CreativeProject, CreativeTrack, ElementKind, ElementRecord, ElementRevision,
    GameAssetRole, GameManifest, PlayerMode, ProductionTarget, QaFinding, QaSeverity,
    ReferenceAuthority, RevisionState, SceneManifest, ShotManifest, CREATIVE_SCHEMA_VERSION,
};
use serde_json::json;

fn revision(id: &str) -> ElementRevision {
    ElementRevision {
        revision_id: id.into(),
        state: RevisionState::Accepted,
        authority: ReferenceAuthority::Authoritative,
        reference_asset_ids: vec![],
        spec: json!({"locked": true}),
        created_at_ms: 1,
    }
}

fn element(id: &str, kind: ElementKind, name: &str) -> ElementRecord {
    let revision_id = format!("{id}_r1");
    ElementRecord {
        element_id: id.into(),
        kind,
        name: name.into(),
        selected_revision_id: Some(revision_id.clone()),
        revisions: vec![revision(&revision_id)],
    }
}

#[test]
fn project_contract_expresses_scene_anime_game_audio_and_lineage_without_engine_ids() {
    let project = CreativeProject {
        schema_version: CREATIVE_SCHEMA_VERSION,
        project_id: "project_contract".into(),
        title: "Cross-track fixture".into(),
        intent: "Prove one provider-neutral source of truth\nfor scene, anime, and game work."
            .into(),
        tracks: vec![
            CreativeTrack::Scene,
            CreativeTrack::Anime,
            CreativeTrack::Game,
        ],
        target: Some(ProductionTarget {
            aspect_ratio: Some("16:9".into()),
            fps: Some(24.0),
            width: Some(1920),
            height: Some(1080),
            duration_ms: Some(12_000),
        }),
        elements: vec![
            element("hero", ElementKind::Character, "Hero"),
            element("alley", ElementKind::Location, "Neon Alley"),
            element("sword", ElementKind::Prop, "Sword"),
            element("look", ElementKind::Style, "Anime Look"),
            element("voice", ElementKind::AudioVoice, "Hero Voice"),
            element("rig", ElementKind::Asset3d, "Hero Rig"),
        ],
        assets: vec![AssetRecord {
            asset_id: "asset_voice".into(),
            media_type: "audio/wav".into(),
            role: "dialogue_take".into(),
            relative_path: "assets/dialogue.wav".into(),
            checksum_sha256: "a".repeat(64),
            bytes: 1024,
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Anime,
            state: AssetState::Accepted,
            job_id: Some("job_voice".into()),
            parent_asset_id: None,
            element_id: Some("voice".into()),
            metadata: AssetMetadata {
                duration_ms: Some(2_000),
                language: Some("en".into()),
                sample_rate_hz: Some(48_000),
                channels: Some(1),
                ..AssetMetadata::default()
            },
            created_at_ms: 2,
        }],
        scenes: vec![SceneManifest {
            scene_id: "scene_intro".into(),
            title: "Opening beat".into(),
            cast_element_ids: vec!["hero".into()],
            location_element_id: Some("alley".into()),
            style_element_id: Some("look".into()),
            target_duration_ms: Some(12_000),
            shots: vec![
                ShotManifest {
                    shot_id: "shot_1".into(),
                    order: 1,
                    duration_ms: 4_000,
                    element_ids: vec!["hero".into(), "sword".into()],
                    camera: CameraSpec {
                        shot_size: Some("medium".into()),
                        focal_length_mm: Some(50.0),
                        aperture_f: Some(2.8),
                        movement: Some("dolly_in".into()),
                        framing: Some("rule_of_thirds".into()),
                    },
                    action: "Hero enters frame.".into(),
                    continuity: json!({"screen_direction": "left_to_right"}),
                },
                ShotManifest {
                    shot_id: "shot_2".into(),
                    order: 2,
                    duration_ms: 4_000,
                    element_ids: vec!["hero".into()],
                    camera: CameraSpec::default(),
                    action: "Hero reacts.".into(),
                    continuity: json!({"eye_line": "camera_right"}),
                },
                ShotManifest {
                    shot_id: "shot_3".into(),
                    order: 3,
                    duration_ms: 4_000,
                    element_ids: vec!["hero".into(), "sword".into()],
                    camera: CameraSpec::default(),
                    action: "Hero draws sword.".into(),
                    continuity: json!({"prop_hand": "right"}),
                },
            ],
        }],
        games: vec![GameManifest {
            game_id: "game_duel".into(),
            title: "Alley Duel".into(),
            genre: "action".into(),
            perspective: "third_person".into(),
            core_loop: "Dodge, counter, and build meter.".into(),
            win_condition: "Defeat the rival.".into(),
            lose_condition: "Health reaches zero.".into(),
            restart_behavior: "Restart from checkpoint.".into(),
            player_mode: PlayerMode::Solo,
            target_devices: vec!["desktop".into(), "mobile".into()],
            verbs: vec!["move".into(), "dodge".into(), "attack".into()],
            inputs: vec!["keyboard".into(), "touch".into()],
            style_element_id: Some("look".into()),
            asset_roles: vec![GameAssetRole {
                role: "player_character".into(),
                runtime_path: "characters/hero.glb".into(),
                asset_id: None,
            }],
        }],
        audio_plans: vec![AudioPlan {
            audio_plan_id: "audio_intro".into(),
            voice_element_id: Some("voice".into()),
            language: Some("en".into()),
            cues: vec![AudioCue {
                cue_id: "cue_1".into(),
                start_ms: 1_000,
                duration_ms: Some(2_000),
                asset_id: Some("asset_voice".into()),
                text: Some("We finish this here.".into()),
            }],
        }],
        graph_ids: vec!["graph_intro".into()],
        job_ids: vec!["job_voice".into()],
        qa_findings: vec![QaFinding {
            finding_id: "qa_1".into(),
            domain: "continuity".into(),
            severity: QaSeverity::NotInspected,
            subject_id: "scene_intro".into(),
            message: "Awaiting visual evidence.".into(),
            created_at_ms: 3,
        }],
        created_at_ms: 1,
        updated_at_ms: 3,
    };

    project
        .validate()
        .expect("cross-track project contract must validate");
    let serialized = serde_json::to_value(&project).expect("serialize project contract");
    let text = serialized.to_string();
    for forbidden in ["provider_id", "model_id", "agent_id", "endpoint", "api_key"] {
        assert!(
            !text.contains(forbidden),
            "engine-specific field leaked: {forbidden}"
        );
    }

    let mut unaccepted_selection = project.clone();
    unaccepted_selection.elements[0].revisions[0].state = RevisionState::Candidate;
    assert!(unaccepted_selection.validate().is_err());

    let mut forged_generation = project.clone();
    forged_generation.elements[0].revisions[0].authority = ReferenceAuthority::Generated;
    assert!(forged_generation.validate().is_err());

    let mut path_escape = project.clone();
    path_escape.games[0].asset_roles[0].runtime_path = "../escape.glb".into();
    assert!(path_escape.validate().is_err());

    let mut cyclic_lineage = project.clone();
    let mut child = cyclic_lineage.assets[0].clone();
    child.asset_id = "asset_child".into();
    child.relative_path = "assets/dialogue-child.wav".into();
    child.checksum_sha256 = "b".repeat(64);
    child.parent_asset_id = Some("asset_voice".into());
    cyclic_lineage.assets[0].parent_asset_id = Some("asset_child".into());
    cyclic_lineage.assets.push(child);
    assert!(cyclic_lineage.validate().is_err());

    let mut forged_surface = project;
    forged_surface.assets[0].source = AssetSource::ManualImport;
    assert!(forged_surface.validate().is_err());
}
