use std::{env, error::Error, net::SocketAddr};

use url::Url;

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
    github_client_id: String,
    github_client_secret: String,
    github_callback_url: String,
    github_authorize_url: String,
    github_token_url: String,
    github_api_url: String,
    allowed_github_user_ids: Vec<u64>,
    session_secret: String,
    session_ttl_seconds: u64,
    cookie_secure: bool,
    sso_issuer: String,
    relay_callback_url: Url,
    relay_audience: String,
    relay_assertion_secret: String,
    relay_assertion_ttl_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()?;
        let base_url = required_http_url("SSO_BASE_URL")?;
        let relay_callback_url = required_http_url("RELAY_CALLBACK_URL")?;
        if relay_callback_url.query().is_some() || relay_callback_url.fragment().is_some() {
            return Err("RELAY_CALLBACK_URL must not contain a query or fragment".into());
        }

        Ok(Self {
            port,
            github_client_id: required("GITHUB_CLIENT_ID")?,
            github_client_secret: required("GITHUB_CLIENT_SECRET")?,
            github_callback_url: required("GITHUB_CALLBACK_URL")?,
            github_authorize_url: env::var("GITHUB_AUTHORIZE_URL")
                .unwrap_or_else(|_| "https://github.com/login/oauth/authorize".to_string()),
            github_token_url: env::var("GITHUB_TOKEN_URL")
                .unwrap_or_else(|_| "https://github.com/login/oauth/access_token".to_string()),
            github_api_url: env::var("GITHUB_API_URL")
                .unwrap_or_else(|_| "https://api.github.com/".to_string()),
            allowed_github_user_ids: parse_allowed_github_user_ids(&required(
                "ALLOWED_GITHUB_USER_IDS",
            )?)?,
            session_secret: required("SESSION_SECRET")?,
            session_ttl_seconds: env::var("SESSION_TTL_SECONDS")
                .unwrap_or_else(|_| "604800".to_string())
                .parse::<u64>()?,
            cookie_secure: base_url.scheme() == "https",
            sso_issuer: base_url.as_str().trim_end_matches('/').to_string(),
            relay_callback_url,
            relay_audience: required("RELAY_AUDIENCE")?,
            relay_assertion_secret: required("RELAY_ASSERTION_SECRET")?,
            relay_assertion_ttl_seconds: env::var("RELAY_ASSERTION_TTL_SECONDS")
                .unwrap_or_else(|_| "90".to_string())
                .parse::<u64>()?,
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }

    pub fn github_client_id(&self) -> String {
        self.github_client_id.clone()
    }

    pub fn github_client_secret(&self) -> String {
        self.github_client_secret.clone()
    }

    pub fn github_callback_url(&self) -> String {
        self.github_callback_url.clone()
    }

    pub fn github_authorize_url(&self) -> String {
        self.github_authorize_url.clone()
    }

    pub fn github_token_url(&self) -> String {
        self.github_token_url.clone()
    }

    pub fn github_api_url(&self) -> String {
        self.github_api_url.clone()
    }

    pub fn allowed_github_user_ids(&self) -> Vec<u64> {
        self.allowed_github_user_ids.clone()
    }

    pub fn session_secret(&self) -> String {
        self.session_secret.clone()
    }

    pub fn session_ttl_seconds(&self) -> u64 {
        self.session_ttl_seconds
    }

    pub fn cookie_secure(&self) -> bool {
        self.cookie_secure
    }

    pub fn sso_issuer(&self) -> String {
        self.sso_issuer.clone()
    }

    pub fn relay_callback_url(&self) -> Url {
        self.relay_callback_url.clone()
    }

    pub fn relay_audience(&self) -> String {
        self.relay_audience.clone()
    }

    pub fn relay_assertion_secret(&self) -> String {
        self.relay_assertion_secret.clone()
    }

    pub fn relay_assertion_ttl_seconds(&self) -> u64 {
        self.relay_assertion_ttl_seconds
    }
}

fn required(name: &'static str) -> Result<String, Box<dyn Error>> {
    let value = env::var(name)?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty").into());
    }
    Ok(value)
}

fn required_http_url(name: &'static str) -> Result<Url, Box<dyn Error>> {
    let url = Url::parse(&required(name)?)?;
    if !matches!(url.scheme(), "http" | "https") || url.cannot_be_a_base() {
        return Err(format!("{name} must be an absolute HTTP(S) URL").into());
    }
    Ok(url)
}

fn parse_allowed_github_user_ids(value: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    let mut ids = Vec::new();

    for raw in value.split(',') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("ALLOWED_GITHUB_USER_IDS contains an empty entry".into());
        }
        ids.push(trimmed.parse::<u64>().map_err(|_| {
            format!("ALLOWED_GITHUB_USER_IDS contains invalid GitHub user ID: {trimmed}")
        })?);
    }

    if ids.is_empty() {
        return Err("ALLOWED_GITHUB_USER_IDS must contain at least one GitHub user ID".into());
    }

    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}
