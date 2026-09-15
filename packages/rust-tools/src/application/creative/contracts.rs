use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CREATIVE_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_TITLE_BYTES: usize = 200;
pub const MAX_PROJECT_INTENT_BYTES: usize = 4_096;
pub const MAX_SPEC_BYTES: usize = 32 * 1024;
pub const MAX_PROJECT_ELEMENTS: usize = 256;
pub const MAX_PROJECT_ASSETS: usize = 2_048;
pub const MAX_PROJECT_SCENES: usize = 128;
pub const MAX_PROJECT_GAMES: usize = 32;
pub const MAX_PROJECT_AUDIO_PLANS: usize = 128;
pub const MAX_QA_FINDINGS: usize = 2_048;
pub const MAX_PROJECT_GRAPHS: usize = 1_024;
pub const MAX_PROJECT_JOBS: usize = 4_096;
pub const MAX_REVISIONS_PER_ELEMENT: usize = 128;
pub const MAX_REFERENCES_PER_REVISION: usize = 64;
pub const MAX_SCENE_SHOTS: usize = 256;
pub const MAX_GAME_LIST_ITEMS: usize = 128;
pub const MAX_GAME_ASSET_ROLES: usize = 512;
pub const MAX_AUDIO_CUES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreativeTrack {
    Scene,
    Anime,
    Game,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductionTarget {
    #[serde(default)]
    pub aspect_ratio: Option<String>,
    #[serde(default)]
    pub fps: Option<f32>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Character,
    Location,
    Prop,
    Style,
    AudioVoice,
    Media,
    Asset3d,
    AnimationClip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevisionState {
    Candidate,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceAuthority {
    Authoritative,
    Interpreted,
    Generated,
    Imported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementRevision {
    pub revision_id: String,
    pub state: RevisionState,
    pub authority: ReferenceAuthority,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
    #[serde(default)]
    pub spec: Value,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementRecord {
    pub element_id: String,
    pub kind: ElementKind,
    pub name: String,
    #[serde(default)]
    pub selected_revision_id: Option<String>,
    #[serde(default)]
    pub revisions: Vec<ElementRevision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetState {
    Candidate,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    ConversationUpload,
    McpUpload,
    UrlImport,
    GeneratedAsset,
    ManualImport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSurface {
    Mcp,
    Canvas,
    Scene,
    Anime,
    Game,
    Blender,
    ManualImport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AssetMetadata {
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub frame_rate: Option<f32>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub sample_rate_hz: Option<u32>,
    #[serde(default)]
    pub channels: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetRecord {
    pub asset_id: String,
    pub media_type: String,
    pub role: String,
    pub relative_path: String,
    pub checksum_sha256: String,
    pub bytes: u64,
    pub source: AssetSource,
    pub source_surface: AssetSurface,
    pub state: AssetState,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub parent_asset_id: Option<String>,
    #[serde(default)]
    pub element_id: Option<String>,
    #[serde(default)]
    pub metadata: AssetMetadata,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CameraSpec {
    #[serde(default)]
    pub shot_size: Option<String>,
    #[serde(default)]
    pub focal_length_mm: Option<f32>,
    #[serde(default)]
    pub aperture_f: Option<f32>,
    #[serde(default)]
    pub movement: Option<String>,
    #[serde(default)]
    pub framing: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShotManifest {
    pub shot_id: String,
    pub order: u32,
    pub duration_ms: u64,
    #[serde(default)]
    pub element_ids: Vec<String>,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub continuity: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneManifest {
    pub scene_id: String,
    pub title: String,
    #[serde(default)]
    pub cast_element_ids: Vec<String>,
    #[serde(default)]
    pub location_element_id: Option<String>,
    #[serde(default)]
    pub style_element_id: Option<String>,
    #[serde(default)]
    pub target_duration_ms: Option<u64>,
    #[serde(default)]
    pub shots: Vec<ShotManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerMode {
    Solo,
    LocalMultiplayer,
    OnlineMultiplayer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameAssetRole {
    pub role: String,
    pub runtime_path: String,
    #[serde(default)]
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameManifest {
    pub game_id: String,
    pub title: String,
    pub genre: String,
    pub perspective: String,
    pub core_loop: String,
    pub win_condition: String,
    pub lose_condition: String,
    pub restart_behavior: String,
    pub player_mode: PlayerMode,
    #[serde(default)]
    pub target_devices: Vec<String>,
    #[serde(default)]
    pub verbs: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub style_element_id: Option<String>,
    #[serde(default)]
    pub asset_roles: Vec<GameAssetRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioCue {
    pub cue_id: String,
    pub start_ms: u64,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioPlan {
    pub audio_plan_id: String,
    #[serde(default)]
    pub voice_element_id: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub cues: Vec<AudioCue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QaSeverity {
    HardFail,
    SoftFinding,
    NotInspected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QaFinding {
    pub finding_id: String,
    pub domain: String,
    pub severity: QaSeverity,
    pub subject_id: String,
    pub message: String,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreativeProject {
    pub schema_version: u32,
    pub project_id: String,
    pub title: String,
    pub intent: String,
    #[serde(default)]
    pub tracks: Vec<CreativeTrack>,
    #[serde(default)]
    pub target: Option<ProductionTarget>,
    #[serde(default)]
    pub elements: Vec<ElementRecord>,
    #[serde(default)]
    pub assets: Vec<AssetRecord>,
    #[serde(default)]
    pub scenes: Vec<SceneManifest>,
    #[serde(default)]
    pub games: Vec<GameManifest>,
    #[serde(default)]
    pub audio_plans: Vec<AudioPlan>,
    #[serde(default)]
    pub graph_ids: Vec<String>,
    #[serde(default)]
    pub job_ids: Vec<String>,
    #[serde(default)]
    pub qa_findings: Vec<QaFinding>,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
}

mod validation;
pub use validation::{validate_id, validate_spec};
