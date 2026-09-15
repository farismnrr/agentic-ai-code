use super::contracts::{
    AssetMetadata, AssetSource, AssetState, AssetSurface, CREATIVE_SCHEMA_VERSION,
};
use super::store::{self, AssetRegistrationInput};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::network::{safe_http_client, validate_public_http_url};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

mod support;
use support::*;

const DEFAULT_UPLOAD_MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_UPLOAD_MAX_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_UPLOAD_TTL_MS: u64 = 10 * 60 * 1000;
const MAX_UPLOAD_TTL_MS: u64 = 30 * 60 * 1000;
const URL_IMPORT_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadTicket {
    pub schema_version: u32,
    pub source: AssetSource,
    pub ticket_id: String,
    pub project_id: String,
    pub owner: String,
    pub media_type: String,
    pub role: String,
    pub filename: String,
    pub max_bytes: u64,
    pub expires_at_ms: u128,
    #[serde(default)]
    pub used_at_ms: Option<u128>,
    pub token_sha256: String,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadRequestResult {
    pub ticket_id: String,
    pub upload_path: String,
    pub upload_token: String,
    pub expires_at_ms: u128,
    pub max_bytes: u64,
    pub method: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadReceipt {
    pub ticket_id: String,
    pub project_id: String,
    pub relative_path: String,
    pub bytes: u64,
    pub media_type: String,
    pub completed_at_ms: u128,
    #[serde(default)]
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub source: AssetSource,
    pub media_type: String,
    pub role: String,
    pub filename: String,
    pub max_bytes: Option<u64>,
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UrlImportRequest {
    pub url: String,
    pub media_type: Option<String>,
    pub role: String,
    pub filename: Option<String>,
    pub max_bytes: Option<u64>,
    pub parent_asset_id: Option<String>,
    pub element_id: Option<String>,
}

pub fn request_upload(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    request: UploadRequest,
) -> Result<UploadRequestResult, McpError> {
    store::load_project(cwd, config, project_id)?;
    validate_owner(owner)?;
    validate_upload_source(&request.source)?;
    validate_media_type(&request.media_type)?;
    validate_role(&request.role)?;
    let filename = sanitize_filename(&request.filename)?;
    let max_bytes = request.max_bytes.unwrap_or(DEFAULT_UPLOAD_MAX_BYTES);
    if max_bytes == 0 || max_bytes > MAX_UPLOAD_MAX_BYTES {
        return Err(McpError::InvalidRequest(
            "creative upload max_bytes exceeds allowed bounds".into(),
        ));
    }
    let ttl_ms = request.ttl_ms.unwrap_or(DEFAULT_UPLOAD_TTL_MS);
    if ttl_ms == 0 || ttl_ms > MAX_UPLOAD_TTL_MS {
        return Err(McpError::InvalidRequest(
            "creative upload ttl exceeds allowed bounds".into(),
        ));
    }
    let now = store::now_ms();
    let ticket_id = format!("upload_{}", Uuid::new_v4().simple());
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let ticket = UploadTicket {
        schema_version: CREATIVE_SCHEMA_VERSION,
        source: request.source,
        ticket_id: ticket_id.clone(),
        project_id: project_id.to_owned(),
        owner: owner.to_owned(),
        media_type: request.media_type.clone(),
        role: request.role,
        filename,
        max_bytes,
        expires_at_ms: now.saturating_add(u128::from(ttl_ms)),
        used_at_ms: None,
        token_sha256: sha256_text(&token),
        created_at_ms: now,
    };
    store::store_upload_ticket(cwd, config, &ticket)?;
    let encoded_cwd =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(cwd.unwrap_or_default().as_bytes());
    Ok(UploadRequestResult {
        ticket_id: ticket_id.clone(),
        upload_path: format!("/creative-upload/{ticket_id}?cwd={encoded_cwd}"),
        upload_token: token,
        expires_at_ms: ticket.expires_at_ms,
        max_bytes,
        method: "PUT".into(),
        content_type: request.media_type,
    })
}

pub fn list_uploads(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<Vec<UploadTicket>, McpError> {
    validate_owner(owner)?;
    store::load_project(cwd, config, project_id)?;
    let mut tickets = store::list_upload_tickets(cwd, config, project_id)?;
    tickets.retain(|ticket| ticket.owner == owner);
    tickets.sort_by_key(|ticket| std::cmp::Reverse(ticket.created_at_ms));
    tickets.truncate(200);
    for ticket in &mut tickets {
        ticket.token_sha256.clear();
    }
    Ok(tickets)
}

pub fn accept_upload_bytes(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    ticket_id: &str,
    token: &str,
    content_type: Option<&str>,
    bytes: &[u8],
) -> Result<UploadReceipt, McpError> {
    validate_owner(owner)?;
    let mut ticket = store::load_upload_ticket(cwd, config, ticket_id)?;
    if ticket.owner != owner {
        return Err(McpError::InvalidRequest(
            "creative upload ticket owner mismatch".into(),
        ));
    }
    if ticket.used_at_ms.is_some() {
        return Err(McpError::InvalidRequest(
            "creative upload ticket was already used".into(),
        ));
    }
    let now = store::now_ms();
    if now > ticket.expires_at_ms {
        return Err(McpError::InvalidRequest(
            "creative upload ticket expired".into(),
        ));
    }
    if token.is_empty() || sha256_text(token) != ticket.token_sha256 {
        return Err(McpError::InvalidRequest(
            "creative upload ticket token is invalid".into(),
        ));
    }
    if bytes.is_empty() || bytes.len() as u64 > ticket.max_bytes {
        return Err(McpError::InvalidRequest(
            "creative upload body exceeds ticket bounds".into(),
        ));
    }
    if content_type.is_some_and(|value| normalize_content_type(value) != ticket.media_type) {
        return Err(McpError::InvalidRequest(
            "creative upload content type does not match ticket".into(),
        ));
    }
    let relative_path = format!(
        "creative/{}/assets/uploads/{}-{}",
        ticket.project_id, ticket.ticket_id, ticket.filename
    );
    crate::application::workspace::write_contained_bytes(
        &relative_path,
        cwd,
        bytes,
        true,
        false,
        config,
    )?;
    ticket.used_at_ms = Some(now);
    store::store_upload_ticket(cwd, config, &ticket)?;
    store::store_upload_receipt(
        cwd,
        config,
        &UploadReceipt {
            ticket_id: ticket.ticket_id.clone(),
            project_id: ticket.project_id.clone(),
            relative_path: relative_path.clone(),
            bytes: bytes.len() as u64,
            media_type: ticket.media_type.clone(),
            completed_at_ms: now,
            asset_id: None,
        },
    )?;
    Ok(UploadReceipt {
        ticket_id: ticket.ticket_id,
        project_id: ticket.project_id,
        relative_path,
        bytes: bytes.len() as u64,
        media_type: ticket.media_type,
        completed_at_ms: now,
        asset_id: None,
    })
}

pub fn complete_upload(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    ticket_id: &str,
) -> Result<(String, super::contracts::AssetRecord), McpError> {
    validate_owner(owner)?;
    let ticket = store::load_upload_ticket(cwd, config, ticket_id)?;
    if ticket.project_id != project_id || ticket.owner != owner {
        return Err(McpError::InvalidRequest(
            "creative upload ticket project/owner mismatch".into(),
        ));
    }
    if ticket.used_at_ms.is_none() {
        return Err(McpError::InvalidRequest(
            "creative upload has not received bytes".into(),
        ));
    }
    let mut receipt = store::load_upload_receipt(cwd, config, ticket_id)?;
    if let Some(asset_id) = receipt.asset_id.as_deref() {
        let project = store::load_project(cwd, config, project_id)?;
        let asset = project.asset(asset_id).cloned().ok_or_else(|| {
            McpError::InvalidRequest("creative upload completion asset is missing".into())
        })?;
        return Ok((asset_id.to_owned(), asset));
    }
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        project_id,
        AssetRegistrationInput {
            path: receipt.relative_path.clone(),
            media_type: ticket.media_type,
            role: ticket.role,
            source: ticket.source,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: None,
            parent_asset_id: None,
            element_id: None,
            metadata: AssetMetadata::default(),
        },
    )?;
    receipt.asset_id = Some(asset_id.clone());
    store::store_upload_receipt(cwd, config, &receipt)?;
    let project = store::load_project(cwd, config, project_id)?;
    let asset = project
        .asset(&asset_id)
        .cloned()
        .ok_or_else(|| McpError::Internal("registered creative upload asset disappeared".into()))?;
    Ok((asset_id, asset))
}

pub async fn import_url(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    request: UrlImportRequest,
) -> Result<(String, super::contracts::AssetRecord), McpError> {
    store::load_project(cwd, config, project_id)?;
    validate_role(&request.role)?;
    let url = validate_public_http_url(&request.url)?;
    let max_bytes = request.max_bytes.unwrap_or(DEFAULT_UPLOAD_MAX_BYTES);
    if max_bytes == 0 || max_bytes > MAX_UPLOAD_MAX_BYTES {
        return Err(McpError::InvalidRequest(
            "creative URL import max_bytes exceeds allowed bounds".into(),
        ));
    }
    let client = safe_http_client(Some(Duration::from_millis(URL_IMPORT_TIMEOUT_MS)), 5)?;
    let mut response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|_| McpError::InvalidRequest("creative URL import fetch failed".into()))?;
    if !response.status().is_success() {
        return Err(McpError::InvalidRequest(
            "creative URL import returned a non-success status".into(),
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length == 0 || length > max_bytes)
    {
        return Err(McpError::InvalidRequest(
            "creative URL import content length exceeds allowed bounds".into(),
        ));
    }
    let response_media_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(normalize_content_type)
        .ok_or_else(|| {
            McpError::InvalidRequest("creative URL import content type is missing".into())
        })?;
    validate_media_type(&response_media_type)?;
    if request
        .media_type
        .as_deref()
        .is_some_and(|expected| normalize_content_type(expected) != response_media_type)
    {
        return Err(McpError::InvalidRequest(
            "creative URL import content type does not match request".into(),
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| McpError::InvalidRequest("creative URL import body read failed".into()))?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes as usize {
            return Err(McpError::InvalidRequest(
                "creative URL import body exceeds allowed bounds".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    if body.is_empty() {
        return Err(McpError::InvalidRequest(
            "creative URL import body is empty".into(),
        ));
    }
    let filename = match request.filename.as_deref() {
        Some(value) => sanitize_filename(value)?,
        None => filename_from_url(&url, &response_media_type),
    };
    let import_id = format!("import_{}", Uuid::new_v4().simple());
    let relative_path = format!("creative/{project_id}/assets/imports/{import_id}-{filename}");
    crate::application::workspace::write_contained_bytes(
        &relative_path,
        cwd,
        &body,
        true,
        false,
        config,
    )?;
    let (_project, asset_id) = store::register_asset(
        cwd,
        config,
        project_id,
        AssetRegistrationInput {
            path: relative_path,
            media_type: response_media_type,
            role: request.role,
            source: AssetSource::UrlImport,
            source_surface: AssetSurface::Mcp,
            state: AssetState::Candidate,
            job_id: None,
            parent_asset_id: request.parent_asset_id,
            element_id: request.element_id,
            metadata: AssetMetadata::default(),
        },
    )?;
    let project = store::load_project(cwd, config, project_id)?;
    let asset = project
        .asset(&asset_id)
        .cloned()
        .ok_or_else(|| McpError::Internal("registered URL import asset disappeared".into()))?;
    Ok((asset_id, asset))
}

pub fn asset_preview(asset: &super::contracts::AssetRecord) -> serde_json::Value {
    serde_json::json!({
        "asset_id": asset.asset_id,
        "media_type": asset.media_type,
        "resource_uri": format!("creative-asset://{}/{}", asset.asset_id, asset.relative_path),
        "relative_path": asset.relative_path,
        "bytes": asset.bytes,
        "checksum_sha256": asset.checksum_sha256,
    })
}
