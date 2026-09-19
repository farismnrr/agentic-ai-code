use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenePlanningMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SceneGlobalDirection {
    #[serde(default)]
    pub genre_look: Option<String>,
    #[serde(default)]
    pub lighting: Option<String>,
    #[serde(default)]
    pub color_palette: Vec<String>,
    #[serde(default)]
    pub atmosphere: Option<String>,
    #[serde(default)]
    pub era_time: Option<String>,
    #[serde(default)]
    pub spatial_constraints: Value,
    #[serde(default)]
    pub hero_frame_first: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DirectorShotSpec {
    #[serde(default)]
    pub camera_profile: Option<String>,
    #[serde(default)]
    pub depth_of_field_intent: Option<String>,
    #[serde(default)]
    pub movement_speed: Option<String>,
    #[serde(default)]
    pub stabilization: Option<String>,
    #[serde(default)]
    pub tempo_edit_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ShotManifest {
    pub shot_id: String,
    pub order: u32,
    pub duration_ms: u64,
    #[serde(default)]
    pub element_ids: Vec<String>,
    #[serde(default)]
    pub location_variant_id: Option<String>,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default)]
    pub director: DirectorShotSpec,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
    #[serde(default)]
    pub hero_frame_asset_id: Option<String>,
    #[serde(default)]
    pub carry_forward_anchor_asset_id: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub execution_binding_id: Option<String>,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub state_in: Value,
    #[serde(default)]
    pub state_out: Value,
    #[serde(default)]
    pub continuity: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SceneBoardFrame {
    pub frame_id: String,
    pub order: u32,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub shot_id: Option<String>,
    #[serde(default)]
    pub element_ids: Vec<String>,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
    #[serde(default)]
    pub hero_candidate: bool,
    #[serde(default)]
    pub direction: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SceneBoard {
    pub board_id: String,
    pub scene_id: String,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub planning_mode: Option<ScenePlanningMode>,
    #[serde(default)]
    pub frames: Vec<SceneBoardFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SceneManifest {
    pub scene_id: String,
    pub title: String,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub planning_mode: Option<ScenePlanningMode>,
    #[serde(default)]
    pub global_direction: SceneGlobalDirection,
    #[serde(default)]
    pub cast_element_ids: Vec<String>,
    #[serde(default)]
    pub location_element_id: Option<String>,
    #[serde(default)]
    pub style_element_id: Option<String>,
    #[serde(default)]
    pub target_duration_ms: Option<u64>,
    #[serde(default)]
    pub selected_hero_frame_asset_id: Option<String>,
    #[serde(default)]
    pub shots: Vec<ShotManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlayerMode {
    #[default]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameProductionIntent {
    DesignOnly,
    AssetsOnly,
    Build,
    Deploy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameRuntimeBudget {
    #[serde(default)]
    pub target_fps: Option<u32>,
    #[serde(default)]
    pub max_asset_bytes: Option<u64>,
    #[serde(default)]
    pub max_initial_load_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameBuildState {
    #[serde(default)]
    pub source_revision: Option<String>,
    #[serde(default)]
    pub build_revision: Option<String>,
    #[serde(default)]
    pub accepted_build_revision: Option<String>,
    #[serde(default)]
    pub deployment_id: Option<String>,
    #[serde(default)]
    pub deployment_url: Option<String>,
    #[serde(default)]
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameManifest {
    pub game_id: String,
    pub title: String,
    #[serde(default)]
    pub revision_id: Option<String>,
    #[serde(default)]
    pub production_intent: Option<GameProductionIntent>,
    pub genre: String,
    pub perspective: String,
    pub core_loop: String,
    pub win_condition: String,
    pub lose_condition: String,
    pub restart_behavior: String,
    #[serde(default)]
    pub progression: Option<String>,
    #[serde(default)]
    pub camera: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub physics_timing: Value,
    #[serde(default)]
    pub style_formula: Value,
    #[serde(default)]
    pub placeholder_policy: Option<String>,
    #[serde(default)]
    pub runtime_budget: GameRuntimeBudget,
    #[serde(default)]
    pub build_state: GameBuildState,
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
pub struct MultiplayerRoomState {
    pub room_id: String,
    pub project_id: String,
    pub game_id: String,
    pub owner: String,
    #[serde(default)]
    pub member_ids: Vec<String>,
    #[serde(default)]
    pub shared_state: Value,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub closed: bool,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
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
    #[serde(default)]
    pub source_revision_id: Option<String>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub evaluator_binding_id: Option<String>,
    #[serde(default)]
    pub evidence: Value,
    pub created_at_ms: u128,
}
