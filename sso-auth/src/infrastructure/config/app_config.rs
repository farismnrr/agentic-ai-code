use std::{env, error::Error, net::SocketAddr};

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
    github_client_id: String,
    github_client_secret: String,
    github_callback_url: String,
    github_authorize_url: String,
    github_token_url: String,
    github_api_url: String,
    session_secret: String,
    session_ttl_seconds: u64,
    cookie_secure: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse::<u16>()?;
        let base_url = url::Url::parse(&required("SSO_BASE_URL")?)?;

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
            session_secret: required("SESSION_SECRET")?,
            session_ttl_seconds: env::var("SESSION_TTL_SECONDS")
                .unwrap_or_else(|_| "604800".to_string())
                .parse::<u64>()?,
            cookie_secure: base_url.scheme() == "https",
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }

    pub fn github_client_id(&self) -> String { self.github_client_id.clone() }
    pub fn github_client_secret(&self) -> String { self.github_client_secret.clone() }
    pub fn github_callback_url(&self) -> String { self.github_callback_url.clone() }
    pub fn github_authorize_url(&self) -> String { self.github_authorize_url.clone() }
    pub fn github_token_url(&self) -> String { self.github_token_url.clone() }
    pub fn github_api_url(&self) -> String { self.github_api_url.clone() }
    pub fn session_secret(&self) -> String { self.session_secret.clone() }
    pub fn session_ttl_seconds(&self) -> u64 { self.session_ttl_seconds }
    pub fn cookie_secure(&self) -> bool { self.cookie_secure }
}

fn required(name: &'static str) -> Result<String, Box<dyn Error>> {
    let value = env::var(name)?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty").into());
    }
    Ok(value)
}
