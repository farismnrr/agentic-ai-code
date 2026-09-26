use serde::Serialize;

use super::VerifiedPrincipal;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Pending,
    Connected,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    #[serde(skip_serializing)]
    state: String,
    pub status: ConnectionStatus,
    pub principal: Option<VerifiedPrincipal>,
}

impl Connection {
    pub fn pending(id: String, state: String) -> Self {
        Self {
            id,
            state,
            status: ConnectionStatus::Pending,
            principal: None,
        }
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn connect(&mut self, principal: VerifiedPrincipal) {
        self.status = ConnectionStatus::Connected;
        self.principal = Some(principal);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedConnectionAssertion {
    pub principal: VerifiedPrincipal,
    pub state: String,
}
