use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use url::Url;

use crate::{
    application::{AuthError, ConnectedAppRepository},
    domain::ConnectedApp,
};

pub struct FileConnectedAppRepository {
    path: PathBuf,
    apps: Mutex<Vec<ConnectedApp>>,
}

impl FileConnectedAppRepository {
    pub fn new(path: PathBuf) -> Result<Self, AuthError> {
        let apps = load_apps(&path)?;
        for app in &apps {
            validate_app(app)?;
        }

        Ok(Self {
            path,
            apps: Mutex::new(apps),
        })
    }

    fn persist(&self, apps: &[ConnectedApp]) -> Result<(), AuthError> {
        let parent = self.path.parent().ok_or(AuthError::StorageUnavailable)?;
        fs::create_dir_all(parent).map_err(|_| AuthError::StorageUnavailable)?;

        let payload = serde_json::to_vec_pretty(apps).map_err(|_| AuthError::StorageUnavailable)?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, payload).map_err(|_| AuthError::StorageUnavailable)?;
        fs::rename(temporary, &self.path).map_err(|_| AuthError::StorageUnavailable)
    }
}

impl ConnectedAppRepository for FileConnectedAppRepository {
    fn list(&self) -> Result<Vec<ConnectedApp>, AuthError> {
        let mut apps = self
            .apps
            .lock()
            .map_err(|_| AuthError::StorageUnavailable)?
            .clone();
        apps.sort_by(|left, right| left.client_id.cmp(&right.client_id));
        Ok(apps)
    }

    fn find(&self, client_id: &str) -> Result<Option<ConnectedApp>, AuthError> {
        let apps = self
            .apps
            .lock()
            .map_err(|_| AuthError::StorageUnavailable)?;
        Ok(apps.iter().find(|app| app.client_id == client_id).cloned())
    }

    fn save(&self, app: ConnectedApp) -> Result<ConnectedApp, AuthError> {
        validate_app(&app)?;
        let mut apps = self
            .apps
            .lock()
            .map_err(|_| AuthError::StorageUnavailable)?;

        match apps
            .iter_mut()
            .find(|stored| stored.client_id == app.client_id)
        {
            Some(stored) => *stored = app.clone(),
            None => apps.push(app.clone()),
        }
        self.persist(&apps)?;
        Ok(app)
    }
}

fn load_apps(path: &Path) -> Result<Vec<ConnectedApp>, AuthError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let payload = fs::read(path).map_err(|_| AuthError::StorageUnavailable)?;
    serde_json::from_slice(&payload)
        .map_err(|_| AuthError::InvalidConnectedApp("connected app registry contains invalid JSON"))
}

fn validate_app(app: &ConnectedApp) -> Result<(), AuthError> {
    let valid_client_id = (3..=64).contains(&app.client_id.len())
        && app
            .client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !valid_client_id {
        return Err(AuthError::InvalidConnectedApp(
            "client ID must use 3-64 URL-safe characters",
        ));
    }

    if app.name.trim().is_empty() || app.name.len() > 80 {
        return Err(AuthError::InvalidConnectedApp(
            "app name must contain 1-80 characters",
        ));
    }
    if app.description.len() > 240 {
        return Err(AuthError::InvalidConnectedApp(
            "app description must be at most 240 characters",
        ));
    }
    if !(30..=300).contains(&app.assertion_ttl_seconds) {
        return Err(AuthError::InvalidConnectedApp(
            "assertion TTL must be between 30 and 300 seconds",
        ));
    }

    let callback = Url::parse(&app.callback_url)
        .map_err(|_| AuthError::InvalidConnectedApp("callback URL must be valid"))?;
    if !matches!(callback.scheme(), "http" | "https")
        || callback.cannot_be_a_base()
        || callback.query().is_some()
        || callback.fragment().is_some()
    {
        return Err(AuthError::InvalidConnectedApp(
            "callback URL must be an absolute HTTP(S) URL without query or fragment",
        ));
    }
    Ok(())
}
