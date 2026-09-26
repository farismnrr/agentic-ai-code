use url::{ParseError, Url};

use crate::application::SsoConnectUrlProvider;

pub struct SsoConnectUrlBuilder {
    connect_url: Url,
    client_id: String,
}

impl SsoConnectUrlBuilder {
    pub fn new(sso_base_url: Url, client_id: String) -> Result<Self, ParseError> {
        Ok(Self {
            connect_url: sso_base_url.join("/connect")?,
            client_id,
        })
    }
}

impl SsoConnectUrlProvider for SsoConnectUrlBuilder {
    fn connect_url(&self, state: &str) -> String {
        let mut url = self.connect_url.clone();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("connection_state", state);
        url.to_string()
    }
}
