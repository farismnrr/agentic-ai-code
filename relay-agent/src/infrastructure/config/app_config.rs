use std::{env, error::Error, net::SocketAddr};

use url::Url;

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
    sso_base_url: Url,
    relay_public_url: Url,
    relay_client_id: String,
    relay_assertion_secret: String,
    connection_attempt_ttl_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3100".to_string())
            .parse::<u16>()?;
        let relay_client_id =
            env::var("RELAY_CLIENT_ID").unwrap_or_else(|_| "relay-agent".to_string());

        if relay_client_id.trim().is_empty() {
            return Err("RELAY_CLIENT_ID must not be empty".into());
        }

        Ok(Self {
            port,
            sso_base_url: required_http_url("SSO_BASE_URL")?,
            relay_public_url: required_http_url("RELAY_PUBLIC_URL")?,
            relay_client_id,
            relay_assertion_secret: required("RELAY_ASSERTION_SECRET")?,
            connection_attempt_ttl_seconds: env::var("CONNECTION_ATTEMPT_TTL_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse::<u64>()?,
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }

    pub fn sso_base_url(&self) -> Url {
        self.sso_base_url.clone()
    }

    pub fn sso_issuer(&self) -> String {
        self.sso_base_url.as_str().trim_end_matches('/').to_string()
    }

    pub fn relay_public_url(&self) -> Url {
        self.relay_public_url.clone()
    }

    pub fn relay_client_id(&self) -> String {
        self.relay_client_id.clone()
    }

    pub fn relay_assertion_secret(&self) -> String {
        self.relay_assertion_secret.clone()
    }

    pub fn connection_attempt_ttl_seconds(&self) -> u64 {
        self.connection_attempt_ttl_seconds
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
