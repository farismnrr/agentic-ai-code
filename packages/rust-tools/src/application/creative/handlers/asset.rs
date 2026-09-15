use super::super::contracts::{AssetSource, AssetState, AssetSurface};
use super::super::{ingest, store};
use super::{complete, optional_string, parse_optional, required_str};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::ToolCallResult;
use serde_json::{json, Value};

pub(in crate::application::creative) async fn asset(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "register" => {
            for field in [
                "source",
                "source_surface",
                "state",
                "job_id",
                "created_after_ms",
                "created_before_ms",
                "limit",
            ] {
                if arguments.get(field).is_some() {
                    return Err(McpError::InvalidRequest(format!(
                        "creative asset register does not accept {field}"
                    )));
                }
            }
            let (project, asset_id) = store::register_asset(
                cwd,
                config,
                project_id,
                store::AssetRegistrationInput {
                    path: required_str(arguments, "path")?.to_owned(),
                    media_type: required_str(arguments, "media_type")?.to_owned(),
                    role: required_str(arguments, "role")?.to_owned(),
                    source: AssetSource::ManualImport,
                    source_surface: AssetSurface::ManualImport,
                    state: AssetState::Candidate,
                    job_id: None,
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    element_id: optional_string(arguments, "element_id"),
                    metadata: parse_optional(arguments, "metadata")?.unwrap_or_default(),
                },
            )?;
            complete(json!({
                "asset_id": asset_id,
                "source": "manual_import",
                "source_surface": "manual_import",
                "state": "candidate",
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "promote" => {
            let asset_id = required_str(arguments, "asset_id")?;
            let project = store::promote_asset(cwd, config, project_id, asset_id)?;
            complete(json!({
                "asset_id": asset_id,
                "state": "accepted",
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "reject" => {
            let asset_id = required_str(arguments, "asset_id")?;
            let project = store::reject_asset(cwd, config, project_id, asset_id)?;
            complete(json!({
                "asset_id": asset_id,
                "state": "rejected",
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "get" => {
            let project = store::load_project(cwd, config, project_id)?;
            let asset_id = required_str(arguments, "asset_id")?;
            let asset = project
                .asset(asset_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
            complete(json!({"asset": asset}))
        }
        "list" => {
            let project = store::load_project(cwd, config, project_id)?;
            complete(json!({"assets": project.assets}))
        }
        "search" => {
            let assets = store::search_assets(
                cwd,
                config,
                project_id,
                store::AssetSearch {
                    media_type: optional_string(arguments, "media_type"),
                    role: optional_string(arguments, "role"),
                    source: parse_optional(arguments, "source")?,
                    source_surface: parse_optional(arguments, "source_surface")?,
                    state: parse_optional(arguments, "state")?,
                    job_id: optional_string(arguments, "job_id"),
                    element_id: optional_string(arguments, "element_id"),
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    created_after_ms: arguments
                        .get("created_after_ms")
                        .and_then(Value::as_u64)
                        .map(u128::from),
                    created_before_ms: arguments
                        .get("created_before_ms")
                        .and_then(Value::as_u64)
                        .map(u128::from),
                    limit: arguments
                        .get("limit")
                        .and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or(50),
                },
            )?;
            complete(json!({"assets": assets}))
        }
        "upload_request" => {
            let request = ingest::request_upload(
                cwd,
                config,
                owner,
                project_id,
                ingest::UploadRequest {
                    source: parse_optional(arguments, "source")?.unwrap_or(AssetSource::McpUpload),
                    media_type: required_str(arguments, "media_type")?.to_owned(),
                    role: required_str(arguments, "role")?.to_owned(),
                    filename: required_str(arguments, "filename")?.to_owned(),
                    max_bytes: arguments.get("max_bytes").and_then(Value::as_u64),
                    ttl_ms: arguments.get("ttl_ms").and_then(Value::as_u64),
                },
            )?;
            complete(json!({"upload": request}))
        }
        "upload_complete" => {
            let (asset_id, asset) = ingest::complete_upload(
                cwd,
                config,
                owner,
                project_id,
                required_str(arguments, "ticket_id")?,
            )?;
            complete(json!({
                "asset_id": asset_id,
                "asset": asset,
                "preview": ingest::asset_preview(&asset)
            }))
        }
        "upload_list" => complete(json!({
            "uploads": ingest::list_uploads(cwd, config, owner, project_id)?
        })),
        "import_url" => {
            let (asset_id, asset) = ingest::import_url(
                cwd,
                config,
                project_id,
                ingest::UrlImportRequest {
                    url: required_str(arguments, "url")?.to_owned(),
                    media_type: optional_string(arguments, "media_type"),
                    role: required_str(arguments, "role")?.to_owned(),
                    filename: optional_string(arguments, "filename"),
                    max_bytes: arguments.get("max_bytes").and_then(Value::as_u64),
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    element_id: optional_string(arguments, "element_id"),
                },
            )
            .await?;
            complete(json!({
                "asset_id": asset_id,
                "asset": asset,
                "preview": ingest::asset_preview(&asset)
            }))
        }
        "preview" => {
            let project = store::load_project(cwd, config, project_id)?;
            let asset = project
                .asset(required_str(arguments, "asset_id")?)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
            complete(json!({"preview": ingest::asset_preview(asset)}))
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative asset action".into(),
        )),
    }
}
