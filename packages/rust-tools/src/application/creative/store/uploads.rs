use super::io::{read_json, write_json};
use super::{load_project, now_ms};
use crate::application::creative::contracts::{validate_id, AssetSource, CREATIVE_SCHEMA_VERSION};
use crate::application::creative::ingest::{UploadReceipt, UploadTicket};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};

const MAX_UPLOAD_TICKETS_PER_PROJECT: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadIndex {
    schema_version: u32,
    ticket_ids: Vec<String>,
}

pub fn store_upload_ticket(
    cwd: Option<&str>,
    config: &ServerConfig,
    ticket: &UploadTicket,
) -> Result<(), McpError> {
    validate_ticket(ticket)?;
    load_project(cwd, config, &ticket.project_id)?;
    let mut index = read_index(cwd, config, &ticket.project_id)?;
    let is_new = !index
        .ticket_ids
        .iter()
        .any(|value| value == &ticket.ticket_id);
    if is_new && index.ticket_ids.len() >= MAX_UPLOAD_TICKETS_PER_PROJECT {
        return Err(McpError::InvalidRequest(
            "creative upload ticket capacity reached".into(),
        ));
    }
    write_json(
        cwd,
        config,
        &ticket_path(&ticket.project_id, &ticket.ticket_id),
        ticket,
        true,
    )?;
    if is_new {
        index.ticket_ids.push(ticket.ticket_id.clone());
        index.ticket_ids.sort();
        write_json(cwd, config, &index_path(&ticket.project_id), &index, true)?;
    }
    Ok(())
}

pub fn load_upload_ticket(
    cwd: Option<&str>,
    config: &ServerConfig,
    ticket_id: &str,
) -> Result<UploadTicket, McpError> {
    validate_id(ticket_id, "upload ticket id")?;
    let project_ids = super::list_projects(cwd, config)?;
    for project_id in project_ids {
        let index = read_index(cwd, config, &project_id)?;
        if index.ticket_ids.iter().any(|value| value == ticket_id) {
            let ticket: UploadTicket =
                read_json(cwd, config, &ticket_path(&project_id, ticket_id))?;
            validate_ticket(&ticket)?;
            return Ok(ticket);
        }
    }
    Err(McpError::InvalidRequest(
        "unknown creative upload ticket".into(),
    ))
}

pub fn list_upload_tickets(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<Vec<UploadTicket>, McpError> {
    load_project(cwd, config, project_id)?;
    let index = read_index(cwd, config, project_id)?;
    index
        .ticket_ids
        .iter()
        .map(|ticket_id| {
            let ticket: UploadTicket = read_json(cwd, config, &ticket_path(project_id, ticket_id))?;
            validate_ticket(&ticket)?;
            Ok(ticket)
        })
        .collect()
}

pub fn store_upload_receipt(
    cwd: Option<&str>,
    config: &ServerConfig,
    receipt: &UploadReceipt,
) -> Result<(), McpError> {
    validate_id(&receipt.ticket_id, "upload ticket id")?;
    validate_id(&receipt.project_id, "project_id")?;
    if receipt.bytes == 0 || receipt.media_type.is_empty() || receipt.relative_path.is_empty() {
        return Err(McpError::InvalidRequest(
            "creative upload receipt is invalid".into(),
        ));
    }
    write_json(
        cwd,
        config,
        &receipt_path(&receipt.project_id, &receipt.ticket_id),
        receipt,
        true,
    )
}

pub fn load_upload_receipt(
    cwd: Option<&str>,
    config: &ServerConfig,
    ticket_id: &str,
) -> Result<UploadReceipt, McpError> {
    let ticket = load_upload_ticket(cwd, config, ticket_id)?;
    let receipt: UploadReceipt = read_json(
        cwd,
        config,
        &receipt_path(&ticket.project_id, &ticket.ticket_id),
    )?;
    if receipt.ticket_id != ticket.ticket_id || receipt.project_id != ticket.project_id {
        return Err(McpError::InvalidRequest(
            "creative upload receipt identity mismatch".into(),
        ));
    }
    Ok(receipt)
}

fn read_index(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
) -> Result<UploadIndex, McpError> {
    match read_json::<UploadIndex>(cwd, config, &index_path(project_id)) {
        Ok(index) => {
            if index.schema_version != CREATIVE_SCHEMA_VERSION
                || index.ticket_ids.len() > MAX_UPLOAD_TICKETS_PER_PROJECT
            {
                return Err(McpError::InvalidRequest(
                    "creative upload index is invalid".into(),
                ));
            }
            for ticket_id in &index.ticket_ids {
                validate_id(ticket_id, "upload ticket id")?;
            }
            Ok(index)
        }
        Err(McpError::InvalidRequest(message))
            if message == "path does not exist or is inaccessible" =>
        {
            Ok(UploadIndex {
                schema_version: CREATIVE_SCHEMA_VERSION,
                ticket_ids: Vec::new(),
            })
        }
        Err(error) => Err(error),
    }
}

fn validate_ticket(ticket: &UploadTicket) -> Result<(), McpError> {
    if ticket.schema_version != CREATIVE_SCHEMA_VERSION {
        return Err(McpError::InvalidRequest(
            "creative upload ticket schema version is unsupported".into(),
        ));
    }
    validate_id(&ticket.ticket_id, "upload ticket id")?;
    validate_id(&ticket.project_id, "project_id")?;
    if !matches!(
        ticket.source,
        AssetSource::McpUpload | AssetSource::ConversationUpload
    ) || ticket.owner.is_empty()
        || ticket.owner.len() > 512
        || ticket.media_type.is_empty()
        || ticket.role.is_empty()
        || ticket.filename.is_empty()
        || ticket.max_bytes == 0
        || ticket.expires_at_ms < ticket.created_at_ms
        || ticket.token_sha256.len() != 64
        || ticket.created_at_ms > now_ms().saturating_add(60_000)
    {
        return Err(McpError::InvalidRequest(
            "creative upload ticket is invalid".into(),
        ));
    }
    Ok(())
}

fn index_path(project_id: &str) -> String {
    format!(".masihawam/creative/projects/{project_id}/uploads/index.json")
}

fn ticket_path(project_id: &str, ticket_id: &str) -> String {
    format!(".masihawam/creative/projects/{project_id}/uploads/{ticket_id}.json")
}

fn receipt_path(project_id: &str, ticket_id: &str) -> String {
    format!(".masihawam/creative/projects/{project_id}/uploads/{ticket_id}.receipt.json")
}
