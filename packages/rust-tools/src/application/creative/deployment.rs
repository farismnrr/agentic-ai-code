use super::contracts::{validate_id, AssetMetadata, AssetSource, AssetState, AssetSurface};
use super::graph::{CreativeJobKind, CreativeJobRecord, CreativeJobStatus, NodeRunRecord};
use super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::EntryKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path};
use uuid::Uuid;

const MAX_DEPLOYMENT_FILES: usize = 512;
const MAX_DEPLOYMENT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_DEPLOYED_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DEPLOYMENT_SCHEMA_VERSION: u32 = 1;
const DEPLOYMENT_ROOT: &str = ".masihawam/creative/deployments";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalDeploymentRecord {
    schema_version: u32,
    deployment_id: String,
    owner: String,
    project_id: String,
    game_id: String,
    build_revision: String,
    site_root: String,
    created_at_ms: u128,
    published: bool,
}

pub(crate) struct DeployedFile {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

pub(super) fn execute_local_static_game(
    cwd: Option<&str>,
    config: &ServerConfig,
    job: &CreativeJobRecord,
) -> Result<Option<CreativeJobRecord>, McpError> {
    if !is_game_deploy(job) {
        return Ok(None);
    }
    let Some(binding_id) = job.execution_binding_id.as_deref() else {
        return Ok(None);
    };
    if backend_kind(config, binding_id)? != Some("local_static_game") {
        return Ok(None);
    }

    let game_id = required_str(&job.execution_parameters, "game_id")?;
    let build_revision = required_str(&job.execution_parameters, "build_revision")?;
    let project = store::load_project(cwd, config, &job.project_id)?;
    let mut game = project
        .games
        .iter()
        .find(|game| game.game_id == game_id)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest("unknown game manifest".into()))?;
    if game.build_state.accepted_build_revision.as_deref() != Some(build_revision) {
        return Err(McpError::InvalidRequest(
            "game deploy requires the accepted build revision".into(),
        ));
    }

    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
    let build_relative = format!("creative/{}/games/{game_id}/build", job.project_id);
    let build_root = crate::core::workspace_path::resolve_existing_path(
        &execution_root,
        cwd,
        &build_relative,
        EntryKind::Directory,
    )?;

    let deployment_id = format!("deployment_{}", Uuid::new_v4().simple());
    let site_root = format!("{DEPLOYMENT_ROOT}/{deployment_id}/site");
    let (file_count, total_bytes) = copy_tree(config, &build_root, &build_root, &site_root)?;
    if file_count == 0 {
        return Err(McpError::InvalidRequest(
            "game deployment build directory is empty".into(),
        ));
    }

    let record = LocalDeploymentRecord {
        schema_version: DEPLOYMENT_SCHEMA_VERSION,
        deployment_id: deployment_id.clone(),
        owner: job.owner.clone(),
        project_id: job.project_id.clone(),
        game_id: game_id.to_owned(),
        build_revision: build_revision.to_owned(),
        site_root: site_root.clone(),
        created_at_ms: store::now_ms(),
        published: false,
    };
    write_record(config, &record)?;

    let deployment_url = format!("/creative-deploy/{deployment_id}/index.html");
    game.build_state.deployment_id = Some(deployment_id.clone());
    game.build_state.deployment_url = Some(deployment_url.clone());
    game.build_state.published = false;
    store::upsert_game(cwd, config, &job.project_id, game)?;

    let receipt = json!({
        "schema":"game-local-deployment-v1",
        "deployment_id":deployment_id,
        "deployment_url":deployment_url,
        "project_id":job.project_id,
        "game_id":game_id,
        "build_revision":build_revision,
        "file_count":file_count,
        "total_bytes":total_bytes,
        "visibility":"authenticated_relay_local",
        "published":false
    });
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| McpError::Internal("game deployment receipt encoding failed".into()))?;
    let receipt_path = format!(
        "creative/{}/games/{game_id}/deployments/{deployment_id}.json",
        job.project_id
    );
    crate::application::workspace::write_contained_bytes(
        &receipt_path,
        cwd,
        &receipt_bytes,
        true,
        false,
        config,
    )?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        &job.project_id,
        AssetRegistrationInput {
            path: receipt_path,
            media_type: "application/json".into(),
            role: "game_deployment_receipt".into(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Game,
            state: AssetState::Candidate,
            job_id: Some(job.job_id.clone()),
            parent_asset_id: None,
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some("game_local_deployment".into()),
                artifact_version: Some("v1".into()),
                ..AssetMetadata::default()
            },
        },
    )?;

    let mut completed = job.clone();
    completed.status = CreativeJobStatus::Completed;
    completed.failure_code = None;
    completed.output_asset_ids = vec![asset_id.clone()];
    completed.actual_output_bytes = Some(receipt_bytes.len() as u64);
    completed.node_runs = vec![NodeRunRecord {
        node_id: "game_deploy".into(),
        status: CreativeJobStatus::Completed,
        output: Some(json!({
            "asset_id":asset_id,
            "deployment_id":record.deployment_id,
            "deployment_url":format!("/creative-deploy/{}/index.html", record.deployment_id),
            "published":false
        })),
        failure_code: None,
        reused: false,
        execution_batch: 0,
    }];
    completed.updated_at_ms = store::now_ms();
    Ok(Some(completed))
}

pub(crate) fn read_deployed_file(
    config: &ServerConfig,
    owner: &str,
    deployment_id: &str,
    request_path: &str,
) -> Result<DeployedFile, McpError> {
    validate_id(deployment_id, "deployment_id")?;
    let record = read_record(config, deployment_id)?;
    if record.owner != owner || record.deployment_id != deployment_id {
        return Err(McpError::InvalidRequest("unknown game deployment".into()));
    }
    validate_request_path(request_path)?;
    let relative = format!("{}/{}", record.site_root, request_path);
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
    let path = crate::core::workspace_path::resolve_existing_path(
        &execution_root,
        None,
        &relative,
        EntryKind::File,
    )?;
    let metadata = fs::metadata(&path)
        .map_err(|_| McpError::InvalidRequest("game deployment file is inaccessible".into()))?;
    if metadata.len() > MAX_DEPLOYED_FILE_BYTES {
        return Err(McpError::InvalidRequest(
            "game deployment file exceeds response bound".into(),
        ));
    }
    let bytes = fs::read(&path)
        .map_err(|_| McpError::InvalidRequest("game deployment file is inaccessible".into()))?;
    Ok(DeployedFile {
        bytes,
        content_type: content_type(request_path),
    })
}

fn copy_tree(
    config: &ServerConfig,
    root: &Path,
    current: &Path,
    site_root: &str,
) -> Result<(usize, u64), McpError> {
    fn visit(
        config: &ServerConfig,
        root: &Path,
        current: &Path,
        site_root: &str,
        file_count: &mut usize,
        total_bytes: &mut u64,
    ) -> Result<(), McpError> {
        for entry in fs::read_dir(current)
            .map_err(|_| McpError::InvalidRequest("game build directory is inaccessible".into()))?
        {
            let entry = entry.map_err(|_| {
                McpError::InvalidRequest("game build directory is inaccessible".into())
            })?;
            let file_type = entry
                .file_type()
                .map_err(|_| McpError::InvalidRequest("game build entry is inaccessible".into()))?;
            if file_type.is_symlink() {
                return Err(McpError::InvalidRequest(
                    "game deployment refuses symlinked build entries".into(),
                ));
            }
            let path = entry.path();
            if file_type.is_dir() {
                visit(config, root, &path, site_root, file_count, total_bytes)?;
                continue;
            }
            if !file_type.is_file() {
                return Err(McpError::InvalidRequest(
                    "game deployment build contains unsupported entry type".into(),
                ));
            }
            *file_count = file_count.saturating_add(1);
            if *file_count > MAX_DEPLOYMENT_FILES {
                return Err(McpError::InvalidRequest(
                    "game deployment exceeds file-count bound".into(),
                ));
            }
            let metadata = entry
                .metadata()
                .map_err(|_| McpError::InvalidRequest("game build file is inaccessible".into()))?;
            *total_bytes = total_bytes.saturating_add(metadata.len());
            if *total_bytes > MAX_DEPLOYMENT_BYTES || metadata.len() > MAX_DEPLOYED_FILE_BYTES {
                return Err(McpError::InvalidRequest(
                    "game deployment exceeds byte bounds".into(),
                ));
            }
            let rel = path
                .strip_prefix(root)
                .map_err(|_| McpError::Internal("game build path escaped root".into()))?;
            let rel_text = rel
                .to_str()
                .ok_or_else(|| McpError::InvalidRequest("game build path is not UTF-8".into()))?;
            validate_request_path(rel_text)?;
            let bytes = fs::read(&path)
                .map_err(|_| McpError::InvalidRequest("game build file is inaccessible".into()))?;
            crate::application::workspace::write_contained_bytes(
                &format!("{site_root}/{rel_text}"),
                None,
                &bytes,
                true,
                false,
                config,
            )?;
        }
        Ok(())
    }

    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    visit(
        config,
        root,
        current,
        site_root,
        &mut file_count,
        &mut total_bytes,
    )?;
    Ok((file_count, total_bytes))
}

fn write_record(config: &ServerConfig, record: &LocalDeploymentRecord) -> Result<(), McpError> {
    let content = serde_json::to_string_pretty(record)
        .map_err(|_| McpError::Internal("game deployment record encoding failed".into()))?;
    let arguments = json!({
        "path":record_path(&record.deployment_id),
        "content":content,
        "create_parents":true,
        "overwrite":false
    });
    crate::application::workspace::file_write(&arguments, config)?;
    Ok(())
}

fn read_record(
    config: &ServerConfig,
    deployment_id: &str,
) -> Result<LocalDeploymentRecord, McpError> {
    let text = crate::application::workspace::read_contained_text(
        &record_path(deployment_id),
        None,
        config,
        64 * 1024,
    )?;
    let record: LocalDeploymentRecord = serde_json::from_str(&text)
        .map_err(|_| McpError::InvalidRequest("game deployment record is invalid".into()))?;
    if record.schema_version != DEPLOYMENT_SCHEMA_VERSION
        || record.published
        || record.deployment_id != deployment_id
    {
        return Err(McpError::InvalidRequest(
            "game deployment record is invalid".into(),
        ));
    }
    Ok(record)
}

fn record_path(deployment_id: &str) -> String {
    format!("{DEPLOYMENT_ROOT}/{deployment_id}/record.json")
}

fn backend_kind<'a>(
    config: &'a ServerConfig,
    binding_id: &str,
) -> Result<Option<&'a str>, McpError> {
    let mut result = None;
    for raw in &config.creative_binding_backends {
        let (id, kind) = raw.split_once('=').ok_or_else(|| {
            McpError::InvalidRequest("creative binding backend mapping is invalid".into())
        })?;
        validate_id(id, "execution_binding_id")?;
        if !matches!(kind, "local_raster" | "local_static_game") {
            return Err(McpError::InvalidRequest(
                "creative binding backend kind is unsupported".into(),
            ));
        }
        if id == binding_id && result.replace(kind).is_some() {
            return Err(McpError::InvalidRequest(
                "duplicate creative binding backend mapping".into(),
            ));
        }
    }
    Ok(result)
}

fn is_game_deploy(job: &CreativeJobRecord) -> bool {
    job.kind == CreativeJobKind::Capability && job.capability_id.as_deref() == Some("game.deploy")
        || job.kind == CreativeJobKind::Workflow
            && job.workflow_id.as_deref() == Some("game_deploy")
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, McpError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))
}

fn validate_request_path(path: &str) -> Result<(), McpError> {
    if path.is_empty() || path.len() > 1024 || Path::new(path).is_absolute() {
        return Err(McpError::InvalidRequest(
            "game deployment path is invalid".into(),
        ));
    }
    if Path::new(path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(McpError::InvalidRequest(
            "game deployment path is invalid".into(),
        ));
    }
    Ok(())
}

fn content_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
