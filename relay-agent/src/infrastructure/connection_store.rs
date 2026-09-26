use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::{
    application::{AuthError, ConnectionRepository},
    domain::{Connection, ConnectionStatus, VerifiedPrincipal},
};

struct StoredConnection {
    connection: Connection,
    expires_at: Instant,
}

pub struct InMemoryConnectionRepository {
    ttl: Duration,
    entries: Mutex<HashMap<String, StoredConnection>>,
}

impl InMemoryConnectionRepository {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_seconds),
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl ConnectionRepository for InMemoryConnectionRepository {
    fn create(&self, connection: Connection) -> Result<(), AuthError> {
        let mut entries = self.entries.lock().map_err(|_| AuthError::StorageUnavailable)?;
        if entries.contains_key(&connection.id) {
            return Err(AuthError::ConnectionConflict);
        }

        entries.insert(
            connection.id.clone(),
            StoredConnection {
                connection,
                expires_at: Instant::now() + self.ttl,
            },
        );
        Ok(())
    }

    fn find(&self, id: &str) -> Result<Connection, AuthError> {
        let mut entries = self.entries.lock().map_err(|_| AuthError::StorageUnavailable)?;
        let expired = entries
            .get(id)
            .is_some_and(|stored| stored.expires_at <= Instant::now());
        if expired {
            entries.remove(id);
        }

        entries
            .get(id)
            .map(|stored| stored.connection.clone())
            .ok_or(AuthError::ConnectionNotFound)
    }

    fn complete_by_state(
        &self,
        state: &str,
        principal: VerifiedPrincipal,
    ) -> Result<Connection, AuthError> {
        let now = Instant::now();
        let mut entries = self.entries.lock().map_err(|_| AuthError::StorageUnavailable)?;
        entries.retain(|_, stored| stored.expires_at > now);

        let stored = entries
            .values_mut()
            .find(|stored| {
                stored.connection.state() == state
                    && stored.connection.status == ConnectionStatus::Pending
            })
            .ok_or(AuthError::ConnectionStateMismatch)?;

        stored.connection.connect(principal);
        Ok(stored.connection.clone())
    }
}
