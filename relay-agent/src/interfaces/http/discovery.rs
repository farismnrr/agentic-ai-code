use axum::{extract::State, Json};
use serde::Serialize;

use super::RelayHttpState;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDocument {
    schema_version: u8,
    id: String,
    name: &'static str,
    description: &'static str,
    connection: ConnectionDiscovery,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionDiscovery {
    #[serde(rename = "type")]
    kind: &'static str,
    connect_url: String,
    status_url_template: String,
}

impl DiscoveryDocument {
    pub fn new(id: String, connect_url: String, status_url_template: String) -> Self {
        Self {
            schema_version: 1,
            id,
            name: "Masih Awam Relay",
            description: "Connect an authenticated Masih Awam identity to the Relay service.",
            connection: ConnectionDiscovery {
                kind: "sso",
                connect_url,
                status_url_template,
            },
        }
    }
}

pub async fn get(State(state): State<RelayHttpState>) -> Json<DiscoveryDocument> {
    Json(state.discovery.clone())
}
