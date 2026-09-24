use std::{
    env,
    error::Error,
    net::SocketAddr,
};

#[derive(Clone)]
pub struct AppConfig {
    port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()?;

        Ok(Self { port })
    }

    pub fn listen_address(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }
}
