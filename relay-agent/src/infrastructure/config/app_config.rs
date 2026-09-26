use std::{env, error::Error, net::SocketAddr};

use url::Url;

const RELAY_CLIENT_ID: &str = "relay-agent";
const CONNECTION_ATTEMPT_TTL_SECONDS: u64 = 300;

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
    sso_base_url: Url,
    relay_public_url: Url,
    relay_assertion_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3100".to_string())
            .parse::<u16>()?;

        Ok(Self {
            port,
            sso_base_url: required_http_url("SSO_BASE_URL")?,
            relay_public_url: required_http_url("RELAY_PUBLIC_URL")?,
            relay_assertion_secret: required("RELAY_ASSERTION_SECRET")?,
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

    pub fn relay_client_id(&self) -> &'static str {
        RELAY_CLIENT_ID
    }

    pub fn relay_assertion_secret(&self) -> String {
        self.relay_assertion_secret.clone()
    }

    pub fn connection_attempt_ttl_seconds(&self) -> u64 {
        CONNECTION_ATTEMPT_TTL_SECONDS
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
