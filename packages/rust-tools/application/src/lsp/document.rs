use super::{
    LspError, LspSession, TextDocumentSyncKind, MAX_LSP_DOCUMENTS_PER_SESSION,
    MAX_LSP_DOCUMENT_BYTES,
};
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use serde_json::json;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) struct OpenDocument {
    uri: String,
    version: u64,
    content_hash: u64,
    content: String,
    opened: bool,
}

#[derive(Default)]
pub(super) struct DocumentStore {
    documents: HashMap<String, OpenDocument>,
}

pub(super) async fn sync_document(session: &LspSession, path: &str) -> Result<u64, LspError> {
    if path.len() > 4096 {
        return Err(LspError::ContainedLocationRejected);
    }
    let target = resolve_existing_path(&session.identity().root, None, path, EntryKind::File)
        .map_err(|_| LspError::ContainedLocationRejected)?;
    let metadata = std::fs::metadata(&target).map_err(|_| LspError::ContainedLocationRejected)?;
    if metadata.len() as usize > MAX_LSP_DOCUMENT_BYTES {
        return Err(LspError::OversizedResponse);
    }
    let content = std::fs::read_to_string(&target).map_err(|_| LspError::MalformedResponse)?;
    if content.len() > MAX_LSP_DOCUMENT_BYTES {
        return Err(LspError::OversizedResponse);
    }
    let uri = url::Url::from_file_path(&target)
        .map_err(|_| LspError::ContainedLocationRejected)?
        .to_string();
    let hash = content_hash(&content);
    let capabilities = session.capabilities().clone();
    let mut store = session.documents().lock().await;
    if !store.documents.contains_key(&uri) && store.documents.len() >= MAX_LSP_DOCUMENTS_PER_SESSION
    {
        return Err(LspError::CapacityReached);
    }

    if let Some(document) = store.documents.get_mut(&uri) {
        if document.content_hash == hash {
            return Ok(document.version);
        }
        document.version = document.version.saturating_add(1);
        let version = document.version;
        if document.opened {
            match capabilities.text_sync {
                TextDocumentSyncKind::Full => {
                    session
                        .notify(
                            "textDocument/didChange",
                            json!({
                                "textDocument":{"uri":document.uri,"version":version},
                                "contentChanges":[{"text":content}]
                            }),
                        )
                        .await?;
                }
                TextDocumentSyncKind::Incremental => {
                    let (line, character) = end_position_utf16(&document.content);
                    session
                        .notify(
                            "textDocument/didChange",
                            json!({
                                "textDocument":{"uri":document.uri,"version":version},
                                "contentChanges":[{
                                    "range":{
                                        "start":{"line":0,"character":0},
                                        "end":{"line":line,"character":character}
                                    },
                                    "text":content
                                }]
                            }),
                        )
                        .await?;
                }
                TextDocumentSyncKind::None => {}
            }
        }
        document.content_hash = hash;
        document.content = content;
        return Ok(version);
    }

    let opened = capabilities.open_close;
    if opened {
        session
            .notify(
                "textDocument/didOpen",
                json!({
                    "textDocument":{
                        "uri":uri,
                        "languageId":session.language(),
                        "version":1,
                        "text":content
                    }
                }),
            )
            .await?;
    }
    store.documents.insert(
        uri.clone(),
        OpenDocument {
            uri,
            version: 1,
            content_hash: hash,
            content,
            opened,
        },
    );
    Ok(1)
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn end_position_utf16(content: &str) -> (usize, usize) {
    let mut line = 0usize;
    let mut character = 0usize;
    for value in content.chars() {
        if value == '\n' {
            line += 1;
            character = 0;
        } else {
            character += value.len_utf16();
        }
    }
    (line, character)
}
