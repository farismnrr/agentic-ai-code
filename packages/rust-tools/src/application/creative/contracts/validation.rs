use super::*;
use crate::core::error::McpError;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path};

impl CreativeProject {
    pub fn validate(&self) -> Result<(), McpError> {
        if self.schema_version != CREATIVE_SCHEMA_VERSION {
            return Err(McpError::InvalidRequest(
                "creative project schema version is unsupported".into(),
            ));
        }
        validate_id(&self.project_id, "project_id")?;
        validate_text(&self.title, 1, MAX_PROJECT_TITLE_BYTES, "project title")?;
        validate_text(&self.intent, 1, MAX_PROJECT_INTENT_BYTES, "project intent")?;
        if self.tracks.is_empty() || self.tracks.len() > 3 {
            return Err(McpError::InvalidRequest(
                "creative project tracks must contain one to three entries".into(),
            ));
        }
        for (index, track) in self.tracks.iter().enumerate() {
            if self.tracks[..index].contains(track) {
                return Err(McpError::InvalidRequest(
                    "creative project tracks must be unique".into(),
                ));
            }
        }
        if let Some(target) = &self.target {
            validate_target(target)?;
        }
        if self.elements.len() > MAX_PROJECT_ELEMENTS
            || self.assets.len() > MAX_PROJECT_ASSETS
            || self.scenes.len() > MAX_PROJECT_SCENES
            || self.games.len() > MAX_PROJECT_GAMES
            || self.audio_plans.len() > MAX_PROJECT_AUDIO_PLANS
            || self.graph_ids.len() > MAX_PROJECT_GRAPHS
            || self.job_ids.len() > MAX_PROJECT_JOBS
            || self.qa_findings.len() > MAX_QA_FINDINGS
        {
            return Err(McpError::InvalidRequest(
                "creative project collection exceeds allowed bounds".into(),
            ));
        }
        ensure_unique(
            self.elements.iter().map(|value| value.element_id.as_str()),
            "element",
        )?;
        ensure_unique(
            self.assets.iter().map(|value| value.asset_id.as_str()),
            "asset",
        )?;
        ensure_unique(
            self.scenes.iter().map(|value| value.scene_id.as_str()),
            "scene",
        )?;
        ensure_unique(
            self.games.iter().map(|value| value.game_id.as_str()),
            "game",
        )?;
        ensure_unique(
            self.audio_plans
                .iter()
                .map(|value| value.audio_plan_id.as_str()),
            "audio plan",
        )?;
        ensure_unique(self.graph_ids.iter().map(String::as_str), "graph")?;
        ensure_unique(self.job_ids.iter().map(String::as_str), "job")?;
        ensure_unique(
            self.qa_findings
                .iter()
                .map(|value| value.finding_id.as_str()),
            "QA finding",
        )?;
        for graph_id in &self.graph_ids {
            validate_id(graph_id, "graph_id")?;
        }
        for job_id in &self.job_ids {
            validate_id(job_id, "job_id")?;
        }
        for element in &self.elements {
            validate_element(element, self)?;
        }
        for asset in &self.assets {
            validate_asset(asset, self)?;
        }
        for scene in &self.scenes {
            validate_scene(scene, self)?;
        }
        for game in &self.games {
            validate_game(game, self)?;
        }
        for audio in &self.audio_plans {
            validate_audio(audio, self)?;
        }
        for finding in &self.qa_findings {
            validate_qa_finding(finding)?;
        }
        Ok(())
    }

    pub fn element(&self, id: &str) -> Option<&ElementRecord> {
        self.elements.iter().find(|value| value.element_id == id)
    }

    pub fn asset(&self, id: &str) -> Option<&AssetRecord> {
        self.assets.iter().find(|value| value.asset_id == id)
    }
}

pub fn validate_id(value: &str, field: &str) -> Result<(), McpError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(McpError::InvalidRequest(format!(
            "{field} must be 1-64 ASCII letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

pub fn validate_spec(value: &Value) -> Result<(), McpError> {
    if !value.is_object() && !value.is_null() {
        return Err(McpError::InvalidRequest(
            "creative specification must be an object".into(),
        ));
    }
    let bytes = serde_json::to_vec(value)
        .map_err(|_| McpError::InvalidRequest("creative specification is invalid".into()))?;
    if bytes.len() > MAX_SPEC_BYTES {
        return Err(McpError::InvalidRequest(
            "creative specification exceeds maximum".into(),
        ));
    }
    Ok(())
}

fn validate_element(element: &ElementRecord, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&element.element_id, "element_id")?;
    validate_text(&element.name, 1, 200, "element name")?;
    if element.revisions.len() > MAX_REVISIONS_PER_ELEMENT {
        return Err(McpError::InvalidRequest(
            "element revision count exceeds maximum".into(),
        ));
    }
    ensure_unique(
        element
            .revisions
            .iter()
            .map(|value| value.revision_id.as_str()),
        "element revision",
    )?;
    for revision in &element.revisions {
        validate_id(&revision.revision_id, "revision_id")?;
        validate_spec(&revision.spec)?;
        if revision.reference_asset_ids.len() > MAX_REFERENCES_PER_REVISION {
            return Err(McpError::InvalidRequest(
                "element reference count exceeds maximum".into(),
            ));
        }
        for asset_id in &revision.reference_asset_ids {
            validate_id(asset_id, "reference asset id")?;
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "element revision references an unknown asset".into(),
                ));
            }
        }
    }
    if let Some(selected) = element.selected_revision_id.as_deref() {
        if !element
            .revisions
            .iter()
            .any(|revision| revision.revision_id == selected)
        {
            return Err(McpError::InvalidRequest(
                "selected element revision does not exist".into(),
            ));
        }
    }
    Ok(())
}

fn validate_asset(asset: &AssetRecord, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&asset.asset_id, "asset_id")?;
    validate_text(&asset.media_type, 1, 128, "asset media type")?;
    validate_text(&asset.role, 1, 128, "asset role")?;
    validate_text(&asset.relative_path, 1, 4_096, "asset relative path")?;
    let relative_path = Path::new(&asset.relative_path);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(McpError::InvalidRequest(
            "asset relative path must contain only normal relative components".into(),
        ));
    }
    if asset.checksum_sha256.len() != 64
        || !asset
            .checksum_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(McpError::InvalidRequest(
            "asset checksum must be a SHA-256 hex digest".into(),
        ));
    }
    validate_asset_metadata(&asset.metadata)?;
    if let Some(job_id) = asset.job_id.as_deref() {
        validate_id(job_id, "asset job id")?;
        if !project.job_ids.iter().any(|value| value == job_id) {
            return Err(McpError::InvalidRequest(
                "asset job lineage references an unknown project job".into(),
            ));
        }
    }
    if let Some(parent_id) = asset.parent_asset_id.as_deref() {
        if parent_id == asset.asset_id || project.asset(parent_id).is_none() {
            return Err(McpError::InvalidRequest(
                "asset parent must reference another project asset".into(),
            ));
        }
    }
    if let Some(element_id) = asset.element_id.as_deref() {
        if project.element(element_id).is_none() {
            return Err(McpError::InvalidRequest(
                "asset element binding references an unknown element".into(),
            ));
        }
    }
    Ok(())
}

fn validate_asset_metadata(metadata: &AssetMetadata) -> Result<(), McpError> {
    if metadata
        .width
        .is_some_and(|value| value == 0 || value > 16_384)
        || metadata
            .height
            .is_some_and(|value| value == 0 || value > 16_384)
        || metadata
            .duration_ms
            .is_some_and(|value| value == 0 || value > 86_400_000)
        || metadata
            .frame_rate
            .is_some_and(|value| !value.is_finite() || !(1.0..=240.0).contains(&value))
        || metadata
            .sample_rate_hz
            .is_some_and(|value| !(8_000..=384_000).contains(&value))
        || metadata
            .channels
            .is_some_and(|value| value == 0 || value > 32)
    {
        return Err(McpError::InvalidRequest(
            "creative asset media metadata is outside allowed bounds".into(),
        ));
    }
    if let Some(language) = metadata.language.as_deref() {
        validate_text(language, 1, 32, "asset language")?;
    }
    Ok(())
}

fn validate_scene(scene: &SceneManifest, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&scene.scene_id, "scene_id")?;
    validate_text(&scene.title, 1, 200, "scene title")?;
    let mut shot_ids = HashSet::new();
    for element_id in scene
        .cast_element_ids
        .iter()
        .chain(scene.location_element_id.iter())
        .chain(scene.style_element_id.iter())
    {
        if project.element(element_id).is_none() {
            return Err(McpError::InvalidRequest(
                "scene references an unknown element".into(),
            ));
        }
    }
    for shot in &scene.shots {
        validate_id(&shot.shot_id, "shot_id")?;
        if !shot_ids.insert(shot.shot_id.as_str()) || shot.duration_ms == 0 {
            return Err(McpError::InvalidRequest(
                "scene shot identity or duration is invalid".into(),
            ));
        }
        validate_spec(&shot.continuity)?;
        for element_id in &shot.element_ids {
            if project.element(element_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "shot references an unknown element".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_game(game: &GameManifest, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&game.game_id, "game_id")?;
    for (value, label) in [
        (&game.title, "game title"),
        (&game.genre, "game genre"),
        (&game.perspective, "game perspective"),
        (&game.core_loop, "game core loop"),
        (&game.win_condition, "game win condition"),
        (&game.lose_condition, "game lose condition"),
        (&game.restart_behavior, "game restart behavior"),
    ] {
        validate_text(value, 1, 4_096, label)?;
    }
    if let Some(style_id) = game.style_element_id.as_deref() {
        if project.element(style_id).is_none() {
            return Err(McpError::InvalidRequest(
                "game style references an unknown element".into(),
            ));
        }
    }
    Ok(())
}

fn validate_audio(audio: &AudioPlan, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&audio.audio_plan_id, "audio_plan_id")?;
    if let Some(voice_id) = audio.voice_element_id.as_deref() {
        if project.element(voice_id).is_none() {
            return Err(McpError::InvalidRequest(
                "audio plan voice references an unknown element".into(),
            ));
        }
    }
    for cue in &audio.cues {
        validate_id(&cue.cue_id, "audio cue id")?;
        if let Some(asset_id) = cue.asset_id.as_deref() {
            if project.asset(asset_id).is_none() {
                return Err(McpError::InvalidRequest(
                    "audio cue references an unknown asset".into(),
                ));
            }
        }
        if let Some(text) = cue.text.as_deref() {
            validate_text(text, 1, 4_096, "audio cue text")?;
        }
    }
    Ok(())
}

fn validate_target(target: &ProductionTarget) -> Result<(), McpError> {
    if let Some(aspect_ratio) = target.aspect_ratio.as_deref() {
        validate_text(aspect_ratio, 1, 32, "production aspect ratio")?;
    }
    if target
        .fps
        .is_some_and(|fps| !fps.is_finite() || !(1.0..=240.0).contains(&fps))
    {
        return Err(McpError::InvalidRequest(
            "production target fps is outside allowed bounds".into(),
        ));
    }
    if target
        .width
        .is_some_and(|value| value == 0 || value > 16_384)
        || target
            .height
            .is_some_and(|value| value == 0 || value > 16_384)
        || target
            .duration_ms
            .is_some_and(|value| value == 0 || value > 86_400_000)
    {
        return Err(McpError::InvalidRequest(
            "production target dimensions or duration are outside allowed bounds".into(),
        ));
    }
    Ok(())
}

fn validate_qa_finding(finding: &QaFinding) -> Result<(), McpError> {
    validate_id(&finding.finding_id, "QA finding id")?;
    validate_id(&finding.subject_id, "QA subject id")?;
    validate_text(&finding.domain, 1, 128, "QA domain")?;
    validate_text(&finding.message, 1, 4_096, "QA message")
}

fn validate_text(value: &str, min: usize, max: usize, field: &str) -> Result<(), McpError> {
    if value.len() < min || value.len() > max || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(format!(
            "{field} exceeds allowed bounds"
        )));
    }
    Ok(())
}

fn ensure_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<(), McpError> {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(McpError::InvalidRequest(format!(
                "duplicate {label} identity"
            )));
        }
    }
    Ok(())
}
