use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedApp {
    pub client_id: String,
    pub name: String,
    pub description: String,
    pub callback_url: String,
    pub enabled: bool,
    pub assertion_ttl_seconds: u64,
}
