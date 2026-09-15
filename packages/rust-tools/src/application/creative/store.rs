use super::contracts::{
    validate_id, validate_spec, AssetSource, CreativeProject, CreativeProjectLayout, CreativeTrack,
    ElementKind, ElementRecord, ElementRevision, ProductionTarget, ReferenceAuthority,
    RevisionState, CREATIVE_SCHEMA_VERSION, MAX_PROJECT_ELEMENTS, MAX_REFERENCES_PER_REVISION,
    MAX_REVISIONS_PER_ELEMENT,
};
use super::graph::{
    validate_graph, validate_job_record, CreativeGraph, CreativeJobKind, CreativeJobRecord,
};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

mod assets;
mod io;
mod support;
mod templates;
mod uploads;
pub use assets::{
    promote_asset, register_asset, search_assets, AssetRegistrationInput, AssetSearch,
};
use io::*;
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

pub fn project_layout(project_id: &str) -> Result<CreativeProjectLayout, McpError> {
    validate_id(project_id, "project_id")?;
    let state_root = format!("{STATE_PREFIX}/projects/{project_id}");
    let production_root = format!("creative/{project_id}");
    Ok(CreativeProjectLayout {
        state_root: state_root.clone(),
        production_root: production_root.clone(),
        assets_root: format!("{production_root}/assets"),
        scene_boards_root: format!("{production_root}/scene-boards"),
        graphs_root: format!("{state_root}/graphs"),
        templates_root: format!("{state_root}/templates"),
        games_root: format!("{production_root}/games"),
        qa_root: format!("{state_root}/qa"),
        exports_root: format!("{production_root}/exports"),
    })
}

pub fn create_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    input: NewProject,
) -> Result<CreativeProject, McpError> {
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
        scenes: Vec::new(),
        games: Vec::new(),
        audio_plans: Vec::new(),
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

pub fn load_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<CreativeProject, McpError> {
    validate_id(project_id, "project_id")?;
    let project: CreativeProject = read_json(cwd, config, &project_path(project_id))?;
    project.validate()?;
    Ok(project)
}

pub fn list_projects(cwd: Option<&str>, config: &ServerConfig) -> Result<Vec<String>, McpError> {
    Ok(read_project_index(cwd, config)?.project_ids)
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
    let element = project
        .elements
        .iter_mut()
        .find(|value| value.element_id == element_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative element".into()))?;
    for revision in &mut element.revisions {
        revision.state = if revision.revision_id == revision_id {
            RevisionState::Accepted
        } else if revision.state == RevisionState::Accepted {
            RevisionState::Candidate
        } else {
            revision.state.clone()
        };
    }
    if !element
        .revisions
        .iter()
        .any(|value| value.revision_id == revision_id)
    {
        return Err(McpError::InvalidRequest("unknown element revision".into()));
    }
    element.selected_revision_id = Some(revision_id.to_owned());
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
