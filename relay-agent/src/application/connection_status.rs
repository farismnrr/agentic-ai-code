use std::sync::Arc;

use crate::domain::Connection;

use super::{AuthError, ConnectionRepository};

pub struct ConnectionStatusUseCase {
    connections: Arc<dyn ConnectionRepository>,
}

impl ConnectionStatusUseCase {
    pub fn new(connections: Arc<dyn ConnectionRepository>) -> Self {
        Self { connections }
    }

    pub fn execute(&self, id: &str) -> Result<Connection, AuthError> {
        self.connections.find(id)
    }
}
