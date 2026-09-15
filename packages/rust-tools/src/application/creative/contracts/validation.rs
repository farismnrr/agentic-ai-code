use super::*;
use crate::core::error::McpError;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path};

mod assets;
mod location;
use assets::{validate_asset, validate_asset_lineage};

impl CreativeProject {
    pub fn validate(&self) -> Result<(), McpError> {
        if self.schema_version != CREATIVE_SCHEMA_VERSION {
            return Err(McpError::InvalidRequest(
                "creative project schema version is unsupported".into(),
            ));
        }
        validate_id(&self.project_id, "project_id")?;
        validate_text(&self.title, 1, MAX_PROJECT_TITLE_BYTES, "project title")?;
        validate_freeform_text(&self.intent, 1, MAX_PROJECT_INTENT_BYTES, "project intent")?;
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
            || self.scene_boards.len() > MAX_PROJECT_SCENE_BOARDS
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
            self.scene_boards
                .iter()
                .map(|value| value.board_id.as_str()),
            "scene board",
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
        validate_asset_lineage(self)?;
        for board in &self.scene_boards {
            validate_scene_board(board, self)?;
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
            validate_qa_finding(finding, self)?;
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
        if element.kind == ElementKind::Location {
            location::validate_location_pack_spec(&revision.spec, project)?;
        }
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
        if revision.authority == ReferenceAuthority::Generated
            && !revision.reference_asset_ids.iter().any(|asset_id| {
                project
                    .asset(asset_id)
                    .is_some_and(|asset| asset.source == AssetSource::GeneratedAsset)
            })
        {
            return Err(McpError::InvalidRequest(
                "generated element authority requires machine-produced asset provenance".into(),
            ));
        }
    }
    if let Some(selected) = element.selected_revision_id.as_deref() {
        let selected_revision = element
            .revisions
            .iter()
            .find(|revision| revision.revision_id == selected)
            .ok_or_else(|| {
                McpError::InvalidRequest("selected element revision does not exist".into())
            })?;
        if selected_revision.state != RevisionState::Accepted {
            return Err(McpError::InvalidRequest(
                "selected element revision must be accepted".into(),
            ));
        }
    }
    if element
        .revisions
        .iter()
        .filter(|revision| revision.state == RevisionState::Accepted)
        .count()
        > 1
    {
        return Err(McpError::InvalidRequest(
            "element cannot contain multiple accepted revisions".into(),
        ));
    }
    Ok(())
}

mod production;
use production::{validate_audio, validate_game, validate_scene, validate_scene_board};

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

fn validate_qa_finding(finding: &QaFinding, project: &CreativeProject) -> Result<(), McpError> {
    validate_id(&finding.finding_id, "QA finding id")?;
    validate_id(&finding.subject_id, "QA subject id")?;
    validate_text(&finding.domain, 1, 128, "QA domain")?;
    validate_freeform_text(&finding.message, 1, 4_096, "QA message")?;
    if let Some(revision_id) = finding.source_revision_id.as_deref() {
        validate_id(revision_id, "QA source revision id")?;
    }
    if let Some(asset_id) = finding.asset_id.as_deref() {
        validate_id(asset_id, "QA asset id")?;
        if project.asset(asset_id).is_none() {
            return Err(McpError::InvalidRequest(
                "QA finding references an unknown asset".into(),
            ));
        }
    }
    if let Some(binding_id) = finding.evaluator_binding_id.as_deref() {
        validate_id(binding_id, "QA evaluator binding id")?;
    }
    validate_spec(&finding.evidence)
}

fn validate_text(value: &str, min: usize, max: usize, field: &str) -> Result<(), McpError> {
    if value.len() < min || value.len() > max || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(format!(
            "{field} exceeds allowed bounds"
        )));
    }
    Ok(())
}

fn validate_freeform_text(
    value: &str,
    min: usize,
    max: usize,
    field: &str,
) -> Result<(), McpError> {
    if value.len() < min || value.len() > max || value.chars().any(is_disallowed_control) {
        return Err(McpError::InvalidRequest(format!(
            "{field} exceeds allowed bounds"
        )));
    }
    Ok(())
}

fn validate_relative_path(value: &str, field: &str) -> Result<(), McpError> {
    validate_text(value, 1, 4_096, field)?;
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(McpError::InvalidRequest(format!(
            "{field} must contain only normal relative components"
        )));
    }
    Ok(())
}

fn is_disallowed_control(value: char) -> bool {
    value.is_control() && !matches!(value, '\n' | '\r' | '\t')
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
