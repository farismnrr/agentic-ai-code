use std::{env, error::Error, net::SocketAddr};

use url::Url;

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
    sso_base_url: Url,
    relay_public_url: Url,
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
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }

    pub fn sso_base_url(&self) -> Url {
        self.sso_base_url.clone()
    }

    pub fn relay_public_url(&self) -> Url {
        self.relay_public_url.clone()
    }
}

fn required_http_url(name: &'static str) -> Result<Url, Box<dyn Error>> {
    let value = env::var(name)?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty").into());
    }

    let url = Url::parse(&value)?;
    if !matches!(url.scheme(), "http" | "https") || url.cannot_be_a_base() {
        return Err(format!("{name} must be an absolute HTTP(S) URL").into());
    }

    Ok(url)
}
