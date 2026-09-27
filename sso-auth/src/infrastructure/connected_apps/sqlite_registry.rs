use std::{fs, path::PathBuf};

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
    application::{AuthError, ConnectedAppRepository},
    domain::ConnectedApp,
};

use super::validation::validate_app;

pub struct SqliteConnectedAppRepository {
    path: PathBuf,
}

impl SqliteConnectedAppRepository {
    pub fn new(path: PathBuf) -> Result<Self, AuthError> {
        let parent = path.parent().ok_or(AuthError::StorageUnavailable)?;
        fs::create_dir_all(parent).map_err(|_| AuthError::StorageUnavailable)?;

        let repository = Self { path };
        repository.initialize()?;
        Ok(repository)
    }

    fn connect(&self) -> Result<Connection, AuthError> {
        Connection::open(&self.path).map_err(|_| AuthError::StorageUnavailable)
    }

    fn initialize(&self) -> Result<(), AuthError> {
        let connection = self.connect()?;
        connection
            .execute_batch(
                "
                PRAGMA journal_mode = WAL;
                CREATE TABLE IF NOT EXISTS connected_apps (
                    client_id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL,
                    callback_url TEXT NOT NULL,
                    enabled INTEGER NOT NULL CHECK(enabled IN (0, 1)),
                    assertion_ttl_seconds INTEGER NOT NULL
                        CHECK(assertion_ttl_seconds BETWEEN 30 AND 300)
                );
                ",
            )
            .map_err(|_| AuthError::StorageUnavailable)?;
        Ok(())
    }
}

impl ConnectedAppRepository for SqliteConnectedAppRepository {
    fn list(&self) -> Result<Vec<ConnectedApp>, AuthError> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "SELECT client_id, name, description, callback_url, enabled,
                        assertion_ttl_seconds
                 FROM connected_apps
                 ORDER BY client_id",
            )
            .map_err(|_| AuthError::StorageUnavailable)?;

        let rows = statement
            .query_map([], map_app)
            .map_err(|_| AuthError::StorageUnavailable)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| AuthError::StorageUnavailable)
    }

    fn find(&self, client_id: &str) -> Result<Option<ConnectedApp>, AuthError> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT client_id, name, description, callback_url, enabled,
                        assertion_ttl_seconds
                 FROM connected_apps
                 WHERE client_id = ?1",
                [client_id],
                map_app,
            )
            .optional()
            .map_err(|_| AuthError::StorageUnavailable)
    }

    fn save(&self, app: ConnectedApp) -> Result<ConnectedApp, AuthError> {
        validate_app(&app)?;
        let connection = self.connect()?;
        connection
            .execute(
                "INSERT INTO connected_apps (
                    client_id, name, description, callback_url, enabled,
                    assertion_ttl_seconds
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(client_id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    callback_url = excluded.callback_url,
                    enabled = excluded.enabled,
                    assertion_ttl_seconds = excluded.assertion_ttl_seconds",
                params![
                    app.client_id,
                    app.name,
                    app.description,
                    app.callback_url,
                    if app.enabled { 1_i64 } else { 0_i64 },
                    app.assertion_ttl_seconds as i64,
                ],
            )
            .map_err(|_| AuthError::StorageUnavailable)?;
        Ok(app)
    }

    fn delete(&self, client_id: &str) -> Result<bool, AuthError> {
        let connection = self.connect()?;
        let changed = connection
            .execute(
                "DELETE FROM connected_apps WHERE client_id = ?1",
                [client_id],
            )
            .map_err(|_| AuthError::StorageUnavailable)?;
        Ok(changed > 0)
    }
}

fn map_app(row: &Row<'_>) -> rusqlite::Result<ConnectedApp> {
    Ok(ConnectedApp {
        client_id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        callback_url: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        assertion_ttl_seconds: row.get::<_, i64>(5)? as u64,
    })
}

#[cfg(test)]
#[path = "../../../test/infrastructure/connected_apps/sqlite_registry.rs"]
mod tests;
