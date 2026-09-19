use super::contracts::{
    validate_id, validate_spec, AssetSource, CreativeProject, CreativeProjectLayout, CreativeTrack,
    ElementKind, ElementRecord, ElementRevision, GameManifest, MultiplayerRoomState,
    ProductionTarget, QaFinding, ReferenceAuthority, RevisionState, SceneBoard, SceneManifest,
    CREATIVE_SCHEMA_VERSION, MAX_PROJECT_ELEMENTS, MAX_PROJECT_GAMES, MAX_PROJECT_SCENES,
    MAX_PROJECT_SCENE_BOARDS, MAX_QA_FINDINGS, MAX_REFERENCES_PER_REVISION,
    MAX_REVISIONS_PER_ELEMENT,
};
use super::graph::{
    validate_graph, validate_job_record, CreativeGraph, CreativeJobKind, CreativeJobRecord,
};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

mod assets;
mod dependencies;
mod io;
mod production;
mod support;
mod templates;
mod uploads;
pub use assets::{
    promote_asset, register_asset, reject_asset, search_assets, AssetRegistrationInput, AssetSearch,
};
use io::*;
pub use production::{
    add_qa_finding, list_projects, load_multiplayer_room, load_project, project_layout,
    store_multiplayer_room, upsert_game, upsert_scene, upsert_scene_board,
};
use support::new_id;
pub use support::now_ms;
pub use templates::{list_templates, load_template, store_template};
pub use uploads::{
    list_upload_tickets, load_upload_receipt, load_upload_ticket, store_upload_receipt,
    store_upload_ticket,
};

const STATE_PREFIX: &str = ".masihawam/creative";
const MAX_REGISTER_ASSET_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_PROJECTS_PER_WORKSPACE: usize = 256;
const MAX_JOBS_PER_PROJECT: usize = 4_096;
const MAX_GRAPHS_PER_PROJECT: usize = 1_024;

pub struct NewProject {
    pub project_id: Option<String>,
    pub title: String,
    pub intent: String,
    pub tracks: Vec<CreativeTrack>,
    pub target: Option<ProductionTarget>,
}

pub struct ElementRevisionInput {
    pub element_id: Option<String>,
    pub kind: Option<ElementKind>,
    pub name: Option<String>,
    pub authority: ReferenceAuthority,
    pub reference_asset_ids: Vec<String>,
    pub spec: Value,
}

pub fn resolve_registered_asset_path(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
) -> Result<std::path::PathBuf, McpError> {
    validate_id(asset_id, "asset_id")?;
    let project = load_project(cwd, config, project_id)?;
    let asset = project
        .asset(asset_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
    Ok(io::resolve_asset_path(cwd, config, &asset.relative_path)?.absolute)
}

pub(crate) fn store_blender_checkpoint_state(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    checkpoint_id: &str,
    value: &Value,
) -> Result<(), McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(checkpoint_id, "checkpoint_id")?;
    let path =
        format!("{STATE_PREFIX}/projects/{project_id}/blender/checkpoints/{checkpoint_id}.json");
    write_json(cwd, config, &path, value, false)
}

pub(crate) fn load_blender_checkpoint_state(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    checkpoint_id: &str,
) -> Result<Value, McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(checkpoint_id, "checkpoint_id")?;
    let path =
        format!("{STATE_PREFIX}/projects/{project_id}/blender/checkpoints/{checkpoint_id}.json");
    read_json(cwd, config, &path)
}

pub fn create_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    input: NewProject,
) -> Result<CreativeProject, McpError> {
    io::ensure_creative_project_root(cwd, config)?;
    let project_id = input.project_id.unwrap_or_else(|| new_id("project"));
    validate_id(&project_id, "project_id")?;
    let project_path = project_path(&project_id);
    if state_exists(cwd, config, &project_path)? {
        return Err(McpError::InvalidRequest(
            "creative project already exists".into(),
        ));
    }
    let mut index = read_project_index(cwd, config)?;
    if index.project_ids.len() >= MAX_PROJECTS_PER_WORKSPACE {
        return Err(McpError::InvalidRequest(
            "creative project index capacity reached".into(),
        ));
    }
    if index.project_ids.iter().any(|value| value == &project_id) {
        return Err(McpError::InvalidRequest(
            "creative project identity already exists in the index".into(),
        ));
    }
    let timestamp = now_ms();
    let project = CreativeProject {
        schema_version: CREATIVE_SCHEMA_VERSION,
        project_id: project_id.clone(),
        title: input.title,
        intent: input.intent,
        tracks: input.tracks,
        target: input.target,
        elements: Vec::new(),
        assets: Vec::new(),
        scene_boards: Vec::new(),
        scenes: Vec::new(),
        games: Vec::new(),
        graph_ids: Vec::new(),
        job_ids: Vec::new(),
        qa_findings: Vec::new(),
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
    };
    project.validate()?;
    write_json(cwd, config, &project_path, &project, false)?;

    index.project_ids.push(project_id);
    index.project_ids.sort();
    index.project_ids.dedup();
    write_json(cwd, config, index_path(), &index, true)?;
    Ok(project)
}

pub fn add_element_revision(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    input: ElementRevisionInput,
) -> Result<(CreativeProject, String, String), McpError> {
    if input.reference_asset_ids.len() > MAX_REFERENCES_PER_REVISION {
        return Err(McpError::InvalidRequest(
            "element reference count exceeds maximum".into(),
        ));
    }
    validate_spec(&input.spec)?;
    let mut project = load_project(cwd, config, project_id)?;
    for asset_id in &input.reference_asset_ids {
        if project.asset(asset_id).is_none() {
            return Err(McpError::InvalidRequest(
                "element revision references an unknown asset".into(),
            ));
        }
    }
    if input.authority == ReferenceAuthority::Generated
        && !input.reference_asset_ids.iter().any(|asset_id| {
            project
                .asset(asset_id)
                .is_some_and(|asset| asset.source == AssetSource::GeneratedAsset)
        })
    {
        return Err(McpError::InvalidRequest(
            "generated element authority requires machine-produced asset provenance".into(),
        ));
    }

    let revision_id = new_id("revision");
    let revision = ElementRevision {
        revision_id: revision_id.clone(),
        state: RevisionState::Candidate,
        authority: input.authority,
        reference_asset_ids: input.reference_asset_ids,
        spec: input.spec,
        created_at_ms: now_ms(),
    };
    let element_id = match input.element_id {
        Some(element_id) => {
            validate_id(&element_id, "element_id")?;
            let element = project
                .elements
                .iter_mut()
                .find(|value| value.element_id == element_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative element".into()))?;
            if element.revisions.len() >= MAX_REVISIONS_PER_ELEMENT {
                return Err(McpError::InvalidRequest(
                    "element revision capacity reached".into(),
                ));
            }
            if let Some(kind) = input.kind {
                if kind != element.kind {
                    return Err(McpError::InvalidRequest(
                        "element kind cannot change across revisions".into(),
                    ));
                }
            }
            if let Some(name) = input.name {
                if name != element.name {
                    return Err(McpError::InvalidRequest(
                        "element name change requires a separate project-state revision".into(),
                    ));
                }
            }
            element.revisions.push(revision);
            element_id
        }
        None => {
            if project.elements.len() >= MAX_PROJECT_ELEMENTS {
                return Err(McpError::InvalidRequest(
                    "creative element capacity reached".into(),
                ));
            }
            let kind = input
                .kind
                .ok_or_else(|| McpError::InvalidRequest("element kind is required".into()))?;
            let name = input
                .name
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| McpError::InvalidRequest("element name is required".into()))?;
            let element_id = new_id("element");
            project.elements.push(ElementRecord {
                element_id: element_id.clone(),
                kind,
                name,
                selected_revision_id: None,
                revisions: vec![revision],
            });
            element_id
        }
    };
    project.updated_at_ms = now_ms();
    save_project(cwd, config, &project)?;
    Ok((project, element_id, revision_id))
}

pub fn promote_element_revision(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    element_id: &str,
    revision_id: &str,
) -> Result<CreativeProject, McpError> {
    validate_id(element_id, "element_id")?;
    validate_id(revision_id, "revision_id")?;
    let mut project = load_project(cwd, config, project_id)?;
    let (is_style, previous_selected) = {
        let element = project
            .elements
            .iter_mut()
            .find(|value| value.element_id == element_id)
            .ok_or_else(|| McpError::InvalidRequest("unknown creative element".into()))?;
        if !element
            .revisions
            .iter()
            .any(|value| value.revision_id == revision_id)
        {
            return Err(McpError::InvalidRequest("unknown element revision".into()));
        }
        let previous_selected = element.selected_revision_id.clone();
        for revision in &mut element.revisions {
            revision.state = if revision.revision_id == revision_id {
                RevisionState::Accepted
            } else if revision.state == RevisionState::Accepted {
                RevisionState::Candidate
            } else {
                revision.state.clone()
            };
        }
        element.selected_revision_id = Some(revision_id.to_owned());
        (element.kind == ElementKind::Style, previous_selected)
    };
    let timestamp = now_ms();
    if is_style
        && previous_selected
            .as_deref()
            .is_some_and(|value| value != revision_id)
    {
        dependencies::mark_style_dependents_for_review(
            &mut project,
            element_id,
            revision_id,
            timestamp,
        )?;
    }
    project.updated_at_ms = timestamp;
    save_project(cwd, config, &project)?;
    Ok(project)
}

pub fn reject_element_revision(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    element_id: &str,
    revision_id: &str,
) -> Result<CreativeProject, McpError> {
    validate_id(element_id, "element_id")?;
    validate_id(revision_id, "revision_id")?;
    let mut project = load_project(cwd, config, project_id)?;
    let element = project
        .elements
        .iter_mut()
        .find(|value| value.element_id == element_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative element".into()))?;
    let revision = element
        .revisions
        .iter_mut()
        .find(|value| value.revision_id == revision_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown element revision".into()))?;
    revision.state = RevisionState::Rejected;
    if element.selected_revision_id.as_deref() == Some(revision_id) {
        element.selected_revision_id = None;
    }
    project.updated_at_ms = now_ms();
    save_project(cwd, config, &project)?;
    Ok(project)
}

pub fn store_graph(
    cwd: Option<&str>,
    config: &ServerConfig,
    graph: &CreativeGraph,
) -> Result<(), McpError> {
    validate_id(&graph.project_id, "project_id")?;
    validate_id(&graph.graph_id, "graph_id")?;
    let mut project = load_project(cwd, config, &graph.project_id)?;
    let validation = validate_graph(graph, &project, config)?;
    if !validation.valid {
        return Err(McpError::InvalidRequest(
            "creative graph cannot be stored before validation succeeds".into(),
        ));
    }
    let is_new = !project
        .graph_ids
        .iter()
        .any(|value| value == &graph.graph_id);
    if is_new && project.graph_ids.len() >= MAX_GRAPHS_PER_PROJECT {
        return Err(McpError::InvalidRequest(
            "creative graph capacity reached".into(),
        ));
    }
    write_json(
        cwd,
        config,
        &graph_path(&graph.project_id, &graph.graph_id),
        graph,
        true,
    )?;
    if is_new {
        project.graph_ids.push(graph.graph_id.clone());
        project.updated_at_ms = now_ms();
        save_project(cwd, config, &project)?;
    }
    Ok(())
}

pub fn load_graph(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    graph_id: &str,
) -> Result<CreativeGraph, McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(graph_id, "graph_id")?;
    let project = load_project(cwd, config, project_id)?;
    if !project.graph_ids.iter().any(|value| value == graph_id) {
        return Err(McpError::InvalidRequest("unknown creative graph".into()));
    }
    let graph: CreativeGraph = read_json(cwd, config, &graph_path(project_id, graph_id))?;
    let validation = validate_graph(&graph, &project, config)?;
    if !validation.valid {
        return Err(McpError::InvalidRequest(
            "stored creative graph is invalid".into(),
        ));
    }
    Ok(graph)
}

pub fn store_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<(), McpError> {
    validate_job_record(job)?;
    let mut project = load_project(cwd, config, &job.project_id)?;
    if job.kind == CreativeJobKind::Graph {
        let graph_id = job.graph_id.as_deref().ok_or_else(|| {
            McpError::InvalidRequest("graph creative job requires graph_id".into())
        })?;
        if !project.graph_ids.iter().any(|value| value == graph_id) {
            return Err(McpError::InvalidRequest(
                "creative job references an unknown stored graph".into(),
            ));
        }
    }
    let is_new = !project.job_ids.iter().any(|value| value == &job.job_id);
    if is_new && project.job_ids.len() >= MAX_JOBS_PER_PROJECT {
        return Err(McpError::InvalidRequest(
            "creative job capacity reached".into(),
        ));
    }
    write_json(
        cwd,
        config,
        &job_path(&job.project_id, &job.job_id),
        job,
        true,
    )?;
    if is_new {
        project.job_ids.push(job.job_id.clone());
        project.updated_at_ms = now_ms();
        save_project(cwd, config, &project)?;
    }
    Ok(())
}

pub fn load_job(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    job_id: &str,
) -> Result<CreativeJobRecord, McpError> {
    validate_id(project_id, "project_id")?;
    validate_id(job_id, "job_id")?;
    let project = load_project(cwd, config, project_id)?;
    if !project.job_ids.iter().any(|value| value == job_id) {
        return Err(McpError::InvalidRequest("unknown creative job".into()));
    }
    let job: CreativeJobRecord = read_json(cwd, config, &job_path(project_id, job_id))?;
    validate_job_record(&job)?;
    if job.project_id != project.project_id {
        return Err(McpError::InvalidRequest(
            "creative job belongs to a different project".into(),
        ));
    }
    Ok(job)
}

pub fn list_jobs(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<Vec<CreativeJobRecord>, McpError> {
    let project = load_project(cwd, config, project_id)?;
    project
        .job_ids
        .iter()
        .rev()
        .take(200)
        .map(|job_id| load_job(cwd, config, project_id, job_id))
        .collect()
}
